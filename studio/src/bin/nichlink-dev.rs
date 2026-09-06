//! Resident rebuild supervisor for NichLink Studio.
//! NichLink Studio 的常驻重建监督器。

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use notify::Watcher as _;

fn main() -> ExitCode {
    match supervise(std::env::args().nth(1).as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("NichLink Studio: {error}");
            ExitCode::FAILURE
        }
    }
}

fn supervise(query: Option<&str>) -> Result<(), String> {
    let studio_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = studio_root
        .parent()
        .ok_or_else(|| "Studio manifest has no package parent".to_owned())?;
    let package_root = std::env::var_os("NICH_LINK_PACKAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.to_owned());
    let executable = workspace_root.join("target/debug/nichlink-studio");
    let mut watcher = SourceWatcher::new(&package_root).map_err(|error| error.to_string())?;

    rebuild(&studio_root)?;
    watcher.drain();
    let mut child = spawn(&executable, query).map_err(|error| error.to_string())?;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {}
            Err(error) => return Err(format!("cannot inspect Studio process: {error}")),
        }
        match watcher.changed().map_err(|error| error.to_string())? {
            false => thread::sleep(Duration::from_millis(80)),
            true => {
                while watcher.changed().unwrap_or(false) {}
                match rebuild(&studio_root) {
                    Ok(()) => {
                        stop(&mut child).map_err(|error| error.to_string())?;
                        child = spawn(&executable, query).map_err(|error| error.to_string())?;
                    }
                    Err(error) => eprintln!(
                        "NichLink Studio: build failed; keeping the last good process\n{error}"
                    ),
                }
            }
        }
    }
}

fn rebuild(studio_root: &Path) -> Result<(), String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(studio_root.join("Cargo.toml"))
        .args(["--bin", "nichlink-studio"])
        .output()
        .map_err(|error| format!("cannot invoke Cargo: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn spawn(executable: &Path, query: Option<&str>) -> io::Result<Child> {
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(query) = query.filter(|query| !query.is_empty() && *query != "watch") {
        command.arg(query);
    }
    command.spawn()
}

fn stop(child: &mut Child) -> io::Result<()> {
    request_stop(child)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill()?;
    let _ = child.wait();
    Ok(())
}

#[cfg(unix)]
fn request_stop(child: &Child) -> io::Result<()> {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    if unsafe { kill(child.id() as i32, 15) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn request_stop(child: &Child) -> io::Result<()> {
    let _ = child.id();
    Ok(())
}

struct SourceWatcher {
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl SourceWatcher {
    fn new(package_root: &Path) -> io::Result<Self> {
        let (sender, events) = std::sync::mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(sender, notify::Config::default())
            .map_err(io::Error::other)?;
        watcher
            .watch(package_root, notify::RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
        for root in [
            package_root.join("src"),
            package_root.join("studio/src"),
            // Plugin selection is source too. Watching only the extension
            // crate leaves catalog changes invisible until restart.
            // 插件选择文件同样属于源码。只监听 extension crate 会让目录修改
            // 在重启前不可见。
            package_root.join(".nichlink/plugins"),
        ] {
            if root.is_dir() {
                watcher
                    .watch(&root, notify::RecursiveMode::Recursive)
                    .map_err(io::Error::other)?;
            }
        }
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    fn changed(&mut self) -> io::Result<bool> {
        use std::sync::mpsc::RecvTimeoutError;

        let first = match self.events.recv_timeout(Duration::from_millis(120)) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return Ok(false),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "watch channel closed",
                ));
            }
        };
        let mut relevant = first.is_ok_and(|event| relevant_event(&event));
        while let Ok(event) = self.events.try_recv() {
            relevant |= event.is_ok_and(|event| relevant_event(&event));
        }
        Ok(relevant)
    }

    fn drain(&mut self) {
        while self.events.try_recv().is_ok() {}
    }
}

fn relevant_event(event: &notify::Event) -> bool {
    use notify::event::{EventKind, ModifyKind};

    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Name(_))
    ) && event.paths.iter().any(|path| {
        !path
            .components()
            .any(|part| matches!(part.as_os_str().to_str(), Some("target" | ".git")))
            && (path.extension().and_then(|extension| extension.to_str()) == Some("rs")
                || matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("Cargo.toml" | "Cargo.lock" | "build.rs" | "official.lock" | "user.lock")
                ))
    })
}

#[cfg(test)]
mod tests {
    use notify::{event::CreateKind, Event, EventKind};

    use super::relevant_event;

    #[test]
    fn filters_generated_files_and_accepts_sources() {
        let source =
            Event::new(EventKind::Create(CreateKind::File)).add_path("src/control.rs".into());
        let target = Event::new(EventKind::Create(CreateKind::File))
            .add_path("target/debug/generated.rs".into());
        assert!(relevant_event(&source));
        assert!(!relevant_event(&target));
    }

    #[test]
    fn accepts_plugin_catalog_changes() {
        let catalog = Event::new(EventKind::Modify(notify::event::ModifyKind::Any))
            .add_path(".nichlink/plugins/official.rs".into());
        assert!(relevant_event(&catalog));
    }

    #[test]
    fn accepts_remove_rename_and_cargo_changes() {
        let removed = Event::new(EventKind::Remove(notify::event::RemoveKind::File))
            .add_path("studio/src/old.rs".into());
        let renamed = Event::new(EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(".nichlink/plugins/user.lock".into());
        let cargo = Event::new(EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Content,
        )))
        .add_path("Cargo.toml".into());
        assert!(relevant_event(&removed));
        assert!(relevant_event(&renamed));
        assert!(relevant_event(&cargo));
    }
}
