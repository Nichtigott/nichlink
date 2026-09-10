use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use nichlink_run_method::{PluginAdapter, VerifiedPluginArtifact};
use tempfile::{Builder, TempPath};

use crate::{HostError, PluginInstance};

/// Limits for one isolated process call.
/// 单次隔离进程调用的限制。
#[derive(Clone, Copy, Debug)]
pub struct ProcessLimits {
    pub timeout: Duration,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
            max_input_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// Executable and fixed arguments for an isolated plugin.
/// 隔离插件使用的程序和固定参数。
#[derive(Clone, Debug)]
pub struct ProcessProgram {
    executable: PathBuf,
    arguments: Vec<String>,
}

impl ProcessProgram {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            arguments: Vec::new(),
        }
    }

    pub fn argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

/// Process-isolated plugin loader.
/// 进程隔离插件加载器。
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessBackend {
    limits: ProcessLimits,
}

impl ProcessBackend {
    pub const fn new(limits: ProcessLimits) -> Self {
        Self { limits }
    }

    pub fn load(
        &self,
        artifact: VerifiedPluginArtifact,
        program: ProcessProgram,
    ) -> Result<ProcessInstance, HostError> {
        let metadata = std::fs::metadata(program.executable())?;
        if !metadata.is_file() {
            return Err(HostError::InvalidArtifact(format!(
                "{} is not a file",
                program.executable().display()
            )));
        }
        let (_, verified_bytes) = artifact.into_parts();
        if fs::read(program.executable())? != verified_bytes {
            return Err(HostError::InvalidArtifact(
                "process executable differs from the verified bytes".to_owned(),
            ));
        }
        let (program, staged_artifact) = stage_program(program, &verified_bytes)?;
        Ok(ProcessInstance {
            program,
            _staged_artifact: staged_artifact,
            limits: self.limits,
        })
    }
}

/// A process plugin. Each call gets a fresh child and a hard deadline.
/// 进程插件。每次调用使用独立子进程和硬超时。
pub struct ProcessInstance {
    program: ProcessProgram,
    _staged_artifact: TempPath,
    limits: ProcessLimits,
}

fn stage_program(
    program: ProcessProgram,
    bytes: &[u8],
) -> Result<(ProcessProgram, TempPath), HostError> {
    let suffix = program
        .executable
        .extension()
        .and_then(|extension| extension.to_str())
        .map_or_else(String::new, |extension| format!(".{extension}"));
    let mut staged = Builder::new()
        .prefix("nichlink-plugin-")
        .suffix(&suffix)
        .tempfile()?;
    staged.write_all(bytes)?;
    staged.flush()?;
    set_executable(staged.path())?;
    let path = staged.into_temp_path();
    Ok((
        ProcessProgram {
            executable: path.to_path_buf(),
            arguments: program.arguments,
        },
        path,
    ))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), HostError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o500))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_: &Path) -> Result<(), HostError> {
    Ok(())
}

impl PluginInstance for ProcessInstance {
    fn adapter(&self) -> PluginAdapter {
        PluginAdapter::Process
    }

    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        validate_operation(operation)?;
        if input.len() > self.limits.max_input_bytes || input.len() > u32::MAX as usize {
            return Err(HostError::Limit(format!(
                "input is {} bytes; limit is {}",
                input.len(),
                self.limits.max_input_bytes
            )));
        }
        let mut child = Command::new(&self.program.executable)
            .args(&self.program.arguments)
            .arg(operation)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| HostError::Process("child stdin was unavailable".to_owned()))?;
        stdin.write_all(&(input.len() as u32).to_le_bytes())?;
        stdin.write_all(input)?;
        drop(stdin);

        let max_output = self.limits.max_output_bytes;
        let deadline = Instant::now() + self.limits.timeout;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                let _ = child.wait();
                return Err(HostError::Timeout);
            }
            thread::sleep(Duration::from_millis(2));
        };
        if !status.success() {
            let mut error = String::new();
            if let Some(stderr) = child.stderr.take() {
                let _ = stderr.take(4096).read_to_string(&mut error);
            }
            return Err(HostError::Process(if error.trim().is_empty() {
                format!("child exited with {status}")
            } else {
                error.trim().to_owned()
            }));
        }
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| HostError::Process("child stdout was unavailable".to_owned()))?;
        read_frame(&mut stdout, max_output)
    }
}

fn validate_operation(operation: &str) -> Result<(), HostError> {
    if nichlink_run_method::validate_operation_name(operation).is_err() {
        return Err(HostError::InvalidOperation(operation.to_owned()));
    }
    Ok(())
}

fn read_frame(reader: &mut impl Read, max_output: usize) -> Result<Vec<u8>, HostError> {
    let mut encoded_length = [0; 4];
    reader.read_exact(&mut encoded_length)?;
    let length = u32::from_le_bytes(encoded_length) as usize;
    if length > max_output {
        return Err(HostError::Limit(format!(
            "output is {length} bytes; limit is {max_output}"
        )));
    }
    let mut output = vec![0; length];
    reader.read_exact(&mut output)?;
    Ok(output)
}
