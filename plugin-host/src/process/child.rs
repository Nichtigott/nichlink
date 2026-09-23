//! Child-process plumbing for the process adapter: spawn, drain, frame, reap.
//! 进程适配器的子进程底层机制：派生、排空、分帧、回收。
//!
//! This is the half of `process` that talks to the operating system. It is
//! separate so the adapter's contract — limits, the program, the operation
//! framing the host promises — stays readable in one page, and so the pipe and
//! spawn hazards below get room for the explanation they need.
//! 这是 `process` 中与操作系统对话的那一半。它独立出来，是为了让适配器的契约——限制、
//! 程序、宿主承诺的操作分帧——保持在一页内可读，也让下面的管道与派生陷阱有地方把话说明白。

use std::{
    io::Read,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use crate::HostError;

use super::ProcessProgram;

/// How often the child's exit status is polled while it runs.
/// 子进程运行期间轮询其退出状态的间隔。
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Attempts made when `exec` reports the staged file as busy.
/// `exec` 报告暂存文件忙时的尝试次数。
const SPAWN_ATTEMPTS: usize = 10;

/// Spawn the staged plugin, retrying the one transient failure `exec` reports.
/// 派生已暂存的插件，并对 `exec` 唯一会报的瞬时失败进行重试。
///
/// `ETXTBSY` ("Text file busy") means the file is open for writing somewhere.
/// The staged copy is private and was just written by this host, so the
/// condition is a race with a concurrent fork/exec rather than a real conflict:
/// running several plugin calls at once hit it here, reported as
/// `ExecutableFileBusy` by `stderr_larger_than_the_pipe_buffer_does_not_block`.
/// A host that spawns plugins from several threads can hit the same race, so it
/// is retried a bounded number of times and then reported as the I/O error it
/// is; nothing else is retried.
/// `ETXTBSY`（"Text file busy"）表示该文件在某处被以可写方式打开。这份暂存副本是私有的、
/// 刚由本宿主写入，因此该状况是与并发的 fork/exec 竞争，而不是真实冲突：并发执行多个插件
/// 调用时在这里撞上了它，被 `stderr_larger_than_the_pipe_buffer_does_not_block` 报成
/// `ExecutableFileBusy`。从多个线程派生插件的宿主会撞上同一种竞争，因此这里做有界重试，
/// 之后仍作为 I/O 错误上报；其他错误一律不重试。
pub(super) fn spawn_staged(program: &ProcessProgram, operation: &str) -> Result<Child, HostError> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match Command::new(&program.executable)
            .args(&program.arguments)
            .arg(operation)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < SPAWN_ATTEMPTS
                    && error.kind() == std::io::ErrorKind::ExecutableFileBusy =>
            {
                thread::sleep(POLL_INTERVAL);
            }
            Err(error) => return Err(error.into()),
        }
    }
}

/// Bytes of stderr kept for the failure message.
/// 失败消息保留的 stderr 字节数。
const STDERR_BYTES: usize = 4096;

/// Read a child's stderr to EOF, keeping at most [`STDERR_BYTES`].
/// 读到子进程 stderr 的 EOF，最多保留 [`STDERR_BYTES`] 字节。
///
/// Reading to EOF even after the cap is reached is deliberate: stopping early
/// would let a chatty child block on a full pipe, which is the same failure this
/// adapter removed from stdout.
/// 达到上限后仍读到 EOF 是有意的：提前停止会让话多的子进程阻塞在满管道上，那正是本适配器
/// 从 stdout 上移除掉的同一种故障。
pub(super) fn read_stderr(reader: &mut impl Read) -> String {
    let mut kept = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let room = STDERR_BYTES.saturating_sub(kept.len());
                kept.extend_from_slice(&buffer[..read.min(room)]);
            }
        }
    }
    String::from_utf8_lossy(&kept).into_owned()
}

/// Kill the child and reap it, even when the kill itself fails.
/// 杀掉子进程并回收它，即使 kill 本身失败。
///
/// `Child::kill` fails with `InvalidInput` when the child has already been
/// reaped; propagating that with `?` would skip `Child::wait` and leave a
/// zombie, so both steps are attempted and neither is reported.
/// 子进程已被回收时 `Child::kill` 会以 `InvalidInput` 失败；用 `?` 传播它会跳过
/// `Child::wait` 并留下僵尸进程，因此两步都尝试、都不上报。
pub(super) fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read one response frame: the declared length, then that many bytes.
/// 读取一个响应帧：先是声明的长度，然后是相应字节数。
///
/// The declared length is checked against the cap *before* the buffer is
/// allocated, so an over-limit declaration costs nothing.
/// 声明的长度在分配缓冲区**之前**就与上限比较，因此超限的声明不花任何代价。
pub(super) fn read_frame(reader: &mut impl Read, max_output: usize) -> Result<Vec<u8>, HostError> {
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
