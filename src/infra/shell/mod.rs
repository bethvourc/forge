use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::Stdio;
use std::sync::mpsc::{self as std_mpsc, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::app::AppEvent;
use crate::domain::{ExecutionMode, ExecutionRequest, OutputStream};
use crate::shared::error::{AppResult, InfraError, PtyPhase};
use crate::shared::ids::CommandId;

pub type CancellationRegistry = Arc<Mutex<HashMap<CommandId, Option<oneshot::Sender<()>>>>>;

const INTERACTIVE_COMMANDS: &[&str] = &[
    "bash", "bpython", "fish", "htop", "ipython", "irb", "less", "man", "more", "nano", "node",
    "nvim", "python", "screen", "sh", "sqlite3", "ssh", "sudo", "tmux", "top", "vim", "watch",
    "zsh",
];
const MANAGED_CANCEL_GRACE: Duration = Duration::from_millis(750);

pub fn execution_mode_for(raw: &str, background: bool) -> ExecutionMode {
    if background {
        ExecutionMode::Managed
    } else if is_likely_interactive_command(raw) {
        ExecutionMode::Pty
    } else {
        ExecutionMode::Managed
    }
}

pub fn spawn_managed_command(
    request: ExecutionRequest,
    events_tx: mpsc::Sender<AppEvent>,
    cancellations: CancellationRegistry,
) -> AppResult<()> {
    if !matches!(request.mode, ExecutionMode::Managed) {
        return Err(
            InfraError::Runtime("managed executor requires managed mode".to_string()).into(),
        );
    }

    let mut command = build_shell_command(&request);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|source| InfraError::Spawn {
        command: request.raw.clone(),
        source,
    })?;

    let pid = child.id();
    let command_id = request.command_id;
    let job_id = request.job_id;
    let service_id = request.service_id;
    let raw = request.raw.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    let events_tx_for_task = events_tx.clone();
    let cancellations_for_task = cancellations.clone();

    tokio::spawn(async move {
        {
            let mut registry = cancellations_for_task.lock().await;
            registry.insert(command_id, Some(cancel_tx));
        }

        let _ = events_tx_for_task
            .send(AppEvent::CommandStarted {
                command_id,
                job_id,
                service_id,
                pid,
            })
            .await;

        if let Some(stdout) = stdout {
            spawn_stream_reader(
                stdout,
                events_tx_for_task.clone(),
                command_id,
                job_id,
                service_id,
                OutputStream::Stdout,
            );
        }
        if let Some(stderr) = stderr {
            spawn_stream_reader(
                stderr,
                events_tx_for_task.clone(),
                command_id,
                job_id,
                service_id,
                OutputStream::Stderr,
            );
        }

        tokio::select! {
            _ = &mut cancel_rx => {
                match cancel_managed_child(&mut child, pid, &raw).await {
                    Ok(()) => {
                        let _ = events_tx_for_task.send(AppEvent::CommandCancelled {
                            command_id,
                            job_id,
                            service_id,
                        }).await;
                    }
                    Err(error) => {
                        let _ = events_tx_for_task.send(AppEvent::Error(format!(
                            "failed to cancel command `{raw}`: {error}"
                        ))).await;
                    }
                }
            }
            status = child.wait() => {
                match status {
                    Ok(exit_status) => {
                        let _ = events_tx_for_task.send(AppEvent::CommandExited {
                            command_id,
                            job_id,
                            service_id,
                            exit_code: exit_status.code().unwrap_or(-1),
                        }).await;
                    }
                    Err(error) => {
                        let _ = events_tx_for_task.send(AppEvent::Error(format!(
                            "failed waiting for command `{raw}`: {error}"
                        ))).await;
                    }
                }
            }
        }

        let mut registry = cancellations_for_task.lock().await;
        registry.remove(&command_id);
    });

    Ok(())
}

pub struct AttachedPtySession {
    command_id: CommandId,
    control_tx: std_mpsc::Sender<PtyControl>,
}

impl AttachedPtySession {
    pub fn command_id(&self) -> CommandId {
        self.command_id
    }

    pub fn send_input(&self, bytes: Vec<u8>) -> AppResult<()> {
        self.control_tx.send(PtyControl::Input(bytes)).map_err(|_| {
            InfraError::Pty {
                phase: PtyPhase::Shutdown,
                message: "pty session is no longer active".to_string(),
            }
            .into()
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> AppResult<()> {
        self.control_tx
            .send(PtyControl::Resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            }))
            .map_err(|_| {
                InfraError::Pty {
                    phase: PtyPhase::Shutdown,
                    message: "pty session is no longer active".to_string(),
                }
                .into()
            })
    }

    pub fn interrupt(&self) -> AppResult<()> {
        self.control_tx.send(PtyControl::Interrupt).map_err(|_| {
            InfraError::Pty {
                phase: PtyPhase::Shutdown,
                message: "pty session is no longer active".to_string(),
            }
            .into()
        })
    }

    pub fn cancel(&self) -> AppResult<()> {
        self.control_tx.send(PtyControl::Cancel).map_err(|_| {
            InfraError::Pty {
                phase: PtyPhase::Shutdown,
                message: "pty session is no longer active".to_string(),
            }
            .into()
        })
    }
}

enum PtyControl {
    Input(Vec<u8>),
    Resize(PtySize),
    Interrupt,
    Cancel,
}

pub fn spawn_attached_pty_command(
    request: ExecutionRequest,
    events_tx: mpsc::Sender<AppEvent>,
) -> AppResult<AttachedPtySession> {
    if !matches!(request.mode, ExecutionMode::Pty) {
        return Err(InfraError::Runtime("pty executor requires pty mode".to_string()).into());
    }

    #[cfg(unix)]
    {
        spawn_attached_pty_command_unix(request, events_tx)
    }

    #[cfg(not(unix))]
    {
        let _ = request;
        let _ = events_tx;
        Err(InfraError::Pty {
            phase: PtyPhase::Setup,
            message: "PTY execution is only implemented on Unix-like systems".to_string(),
        }
        .into())
    }
}

fn is_likely_interactive_command(raw: &str) -> bool {
    if contains_shell_operators(raw) {
        return false;
    }

    let Some(token) = first_command_token(raw) else {
        return false;
    };
    let command = token.rsplit(['/', '\\']).next().unwrap_or(token);
    INTERACTIVE_COMMANDS.contains(&command)
}

fn contains_shell_operators(raw: &str) -> bool {
    ["&&", "||", ";", "|", ">", "<"]
        .iter()
        .any(|operator| raw.contains(operator))
}

fn first_command_token(raw: &str) -> Option<&str> {
    raw.split_whitespace().next()
}

async fn cancel_managed_child(child: &mut Child, pid: Option<u32>, raw: &str) -> AppResult<()> {
    terminate_managed_child(child, pid).await?;
    match tokio::time::timeout(MANAGED_CANCEL_GRACE, child.wait()).await {
        Ok(Ok(_status)) => Ok(()),
        Ok(Err(error)) => Err(InfraError::Runtime(format!(
            "failed waiting for cancelled command `{raw}`: {error}"
        ))
        .into()),
        Err(_elapsed) => {
            kill_managed_child(child, pid).await?;
            match tokio::time::timeout(MANAGED_CANCEL_GRACE, child.wait()).await {
                Ok(Ok(_status)) => Ok(()),
                Ok(Err(error)) => Err(InfraError::Runtime(format!(
                    "failed waiting for force-killed command `{raw}`: {error}"
                ))
                .into()),
                Err(_elapsed) => Err(InfraError::Runtime(format!(
                    "timed out waiting for force-killed command `{raw}`"
                ))
                .into()),
            }
        }
    }
}

#[cfg(unix)]
async fn terminate_managed_child(_child: &mut Child, pid: Option<u32>) -> AppResult<()> {
    signal_managed_process(pid, libc::SIGTERM)
}

#[cfg(not(unix))]
async fn terminate_managed_child(child: &mut Child, _pid: Option<u32>) -> AppResult<()> {
    child.start_kill()?;
    Ok(())
}

#[cfg(unix)]
async fn kill_managed_child(_child: &mut Child, pid: Option<u32>) -> AppResult<()> {
    signal_managed_process(pid, libc::SIGKILL)
}

#[cfg(not(unix))]
async fn kill_managed_child(child: &mut Child, _pid: Option<u32>) -> AppResult<()> {
    child.kill().await?;
    Ok(())
}

fn build_shell_command(request: &ExecutionRequest) -> Command {
    let mut command = if cfg!(windows) {
        let shell = effective_shell(&request.shell, true);
        let mut command = Command::new(shell);
        command.arg("/C").arg(&request.raw);
        command
    } else {
        let shell = effective_shell(&request.shell, false);
        let mut command = Command::new(shell);
        command.arg("-lc").arg(&request.raw);
        command
    };

    command.current_dir(&request.cwd);
    for (key, value) in &request.env {
        command.env(key, value);
    }
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    command.kill_on_drop(true);
    command
}

fn build_pty_command(request: &ExecutionRequest) -> CommandBuilder {
    let mut command = if cfg!(windows) {
        let mut builder = CommandBuilder::new(effective_shell(&request.shell, true));
        builder.arg("/C");
        builder.arg(&request.raw);
        builder
    } else {
        let mut builder = CommandBuilder::new(effective_shell(&request.shell, false));
        builder.arg("-lc");
        builder.arg(&request.raw);
        builder
    };

    command.cwd(&request.cwd);
    for (key, value) in &request.env {
        command.env(key, value);
    }
    command
}

fn effective_shell(shell: &str, windows: bool) -> String {
    if shell.trim().is_empty() {
        if windows {
            "cmd".to_string()
        } else {
            "/bin/sh".to_string()
        }
    } else {
        shell.to_string()
    }
}

fn spawn_stream_reader<R>(
    reader: R,
    events_tx: mpsc::Sender<AppEvent>,
    command_id: CommandId,
    job_id: crate::shared::ids::JobId,
    service_id: Option<crate::shared::ids::ServiceId>,
    stream: OutputStream,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = events_tx
                        .send(AppEvent::CommandOutput {
                            command_id,
                            job_id,
                            service_id,
                            stream,
                            chunk: line,
                        })
                        .await;
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = events_tx
                        .send(AppEvent::Error(format!(
                            "failed reading command output: {error}"
                        )))
                        .await;
                    break;
                }
            }
        }
    });
}

#[cfg(unix)]
fn spawn_attached_pty_command_unix(
    request: ExecutionRequest,
    events_tx: mpsc::Sender<AppEvent>,
) -> AppResult<AttachedPtySession> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(current_pty_size()?)
        .map_err(|error| InfraError::Pty {
            phase: PtyPhase::Setup,
            message: format!("failed to open pty: {error}"),
        })?;
    let command = build_pty_command(&request);
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| InfraError::Pty {
            phase: PtyPhase::Setup,
            message: format!("failed to spawn pty command: {error}"),
        })?;
    drop(pair.slave);

    let pid = child.process_id();
    let process_group = pair
        .master
        .process_group_leader()
        .and_then(|leader| u32::try_from(leader).ok())
        .or(pid);
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| InfraError::Pty {
            phase: PtyPhase::Setup,
            message: format!("failed to clone pty reader: {error}"),
        })?;
    let mut writer = pair.master.take_writer().map_err(|error| InfraError::Pty {
        phase: PtyPhase::Setup,
        message: format!("failed to open pty writer: {error}"),
    })?;
    let (control_tx, control_rx) = std_mpsc::channel();
    let command_id = request.command_id;
    let job_id = request.job_id;
    let service_id = request.service_id;
    let raw = request.raw.clone();
    let output_events_tx = events_tx.clone();

    std::thread::spawn(move || {
        let _ = events_tx.blocking_send(AppEvent::CommandStarted {
            command_id,
            job_id,
            service_id,
            pid,
        });

        let output_thread = std::thread::spawn(move || {
            stream_pty_output(reader, output_events_tx, command_id, job_id, service_id)
        });

        let result = drive_pty_session(
            &mut child,
            &*pair.master,
            &mut writer,
            process_group,
            &raw,
            control_rx,
        );
        drop(writer);
        let output_result = output_thread
            .join()
            .map_err(|_| {
                InfraError::Pty {
                    phase: PtyPhase::Read,
                    message: "pty output thread panicked".to_string(),
                }
                .into()
            })
            .and_then(|result| result);

        match result {
            Ok(completion) => {
                if let Err(error) = output_result {
                    let _ = events_tx.blocking_send(AppEvent::Error(error.to_string()));
                }
                if completion.cancelled {
                    let _ = events_tx.blocking_send(AppEvent::CommandCancelled {
                        command_id,
                        job_id,
                        service_id,
                    });
                } else {
                    let _ = events_tx.blocking_send(AppEvent::CommandExited {
                        command_id,
                        job_id,
                        service_id,
                        exit_code: completion.exit_code,
                    });
                }
            }
            Err(error) => {
                let _ = terminate_process_group(process_group);
                let _ = events_tx.blocking_send(AppEvent::Error(error.to_string()));
                let _ = events_tx.blocking_send(AppEvent::CommandCancelled {
                    command_id,
                    job_id,
                    service_id,
                });
            }
        }
    });

    Ok(AttachedPtySession {
        command_id,
        control_tx,
    })
}

#[cfg(unix)]
fn current_pty_size() -> AppResult<PtySize> {
    let (cols, rows) = crossterm::terminal::size().map_err(|error| InfraError::Pty {
        phase: PtyPhase::Setup,
        message: format!("failed to determine terminal size for pty: {error}"),
    })?;
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

#[cfg(unix)]
fn stream_pty_output(
    mut reader: Box<dyn Read + Send>,
    events_tx: mpsc::Sender<AppEvent>,
    command_id: CommandId,
    job_id: crate::shared::ids::JobId,
    service_id: Option<crate::shared::ids::ServiceId>,
) -> AppResult<()> {
    let mut stdout = std::io::stdout();
    let mut buffer = [0_u8; 4096];
    let mut pending = Vec::new();

    loop {
        let bytes = reader.read(&mut buffer).map_err(|error| InfraError::Pty {
            phase: PtyPhase::Read,
            message: format!("failed reading pty output: {error}"),
        })?;
        if bytes == 0 {
            break;
        }

        stdout
            .write_all(&buffer[..bytes])
            .map_err(|error| InfraError::Pty {
                phase: PtyPhase::Read,
                message: format!("failed writing pty output to terminal: {error}"),
            })?;
        stdout.flush().map_err(|error| InfraError::Pty {
            phase: PtyPhase::Read,
            message: format!("failed flushing pty output to terminal: {error}"),
        })?;
        pending.extend_from_slice(&buffer[..bytes]);

        while let Some(index) = pending.iter().position(|byte| *byte == b'\n') {
            let mut line = pending.drain(..=index).collect::<Vec<_>>();
            if matches!(line.last(), Some(b'\n')) {
                line.pop();
            }
            if matches!(line.last(), Some(b'\r')) {
                line.pop();
            }
            emit_pty_line(
                &events_tx,
                command_id,
                job_id,
                service_id,
                String::from_utf8_lossy(&line).to_string(),
            )?;
        }
    }

    if !pending.is_empty() {
        emit_pty_line(
            &events_tx,
            command_id,
            job_id,
            service_id,
            String::from_utf8_lossy(&pending).to_string(),
        )?;
    }

    Ok(())
}

#[cfg(unix)]
fn emit_pty_line(
    events_tx: &mpsc::Sender<AppEvent>,
    command_id: CommandId,
    job_id: crate::shared::ids::JobId,
    service_id: Option<crate::shared::ids::ServiceId>,
    line: String,
) -> AppResult<()> {
    if line.is_empty() {
        return Ok(());
    }
    events_tx
        .blocking_send(AppEvent::CommandOutput {
            command_id,
            job_id,
            service_id,
            stream: OutputStream::Stdout,
            chunk: line,
        })
        .map_err(|_| {
            InfraError::Pty {
                phase: PtyPhase::Read,
                message: "failed to publish pty output".to_string(),
            }
            .into()
        })
}

#[cfg(unix)]
struct PtyCompletion {
    exit_code: i32,
    cancelled: bool,
}

#[cfg(unix)]
fn drive_pty_session(
    child: &mut Box<dyn portable_pty::Child + Send + Sync>,
    master: &(dyn portable_pty::MasterPty + Send),
    writer: &mut Box<dyn Write + Send>,
    pid: Option<u32>,
    raw: &str,
    control_rx: std_mpsc::Receiver<PtyControl>,
) -> AppResult<PtyCompletion> {
    let mut cancelled = false;
    let mut interrupted = false;
    let mut kill_deadline = None;

    loop {
        if let Some(status) = child.try_wait().map_err(|error| InfraError::Pty {
            phase: PtyPhase::Shutdown,
            message: format!("failed waiting for pty command `{raw}`: {error}"),
        })? {
            let exit_code = status.exit_code() as i32;
            return Ok(PtyCompletion {
                exit_code,
                cancelled: cancelled || (interrupted && exit_code == 130),
            });
        }

        if let Some(deadline) = kill_deadline {
            if Instant::now() >= deadline {
                signal_process(pid, libc::SIGKILL)?;
                kill_deadline = None;
            }
        }

        match control_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(PtyControl::Input(bytes)) => {
                writer.write_all(&bytes).map_err(|error| InfraError::Pty {
                    phase: PtyPhase::Write,
                    message: format!("failed forwarding stdin to pty: {error}"),
                })?;
                writer.flush().map_err(|error| InfraError::Pty {
                    phase: PtyPhase::Write,
                    message: format!("failed flushing pty stdin: {error}"),
                })?;
            }
            Ok(PtyControl::Resize(size)) => {
                master.resize(size).map_err(|error| InfraError::Pty {
                    phase: PtyPhase::Resize,
                    message: format!("failed resizing pty: {error}"),
                })?;
            }
            Ok(PtyControl::Interrupt) => {
                interrupted = true;
                signal_process(pid, libc::SIGINT)?;
            }
            Ok(PtyControl::Cancel) => {
                cancelled = true;
                signal_process(pid, libc::SIGTERM)?;
                kill_deadline = Some(Instant::now() + Duration::from_millis(250));
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                cancelled = true;
                signal_process(pid, libc::SIGTERM)?;
                kill_deadline = Some(Instant::now() + Duration::from_millis(250));
            }
        }
    }
}

#[cfg(unix)]
fn terminate_process_group(pid: Option<u32>) -> AppResult<()> {
    signal_process(pid, libc::SIGTERM)?;
    std::thread::sleep(Duration::from_millis(50));
    signal_process(pid, libc::SIGKILL)
}

#[cfg(unix)]
fn signal_process(pid: Option<u32>, signal: i32) -> AppResult<()> {
    let Some(pid) = pid else {
        return Ok(());
    };
    let pid = pid as i32;

    if unsafe { libc::killpg(pid, signal) } == 0 {
        return Ok(());
    }
    let process_group_error = std::io::Error::last_os_error();
    if matches!(process_group_error.raw_os_error(), Some(code) if code == libc::ESRCH) {
        return Ok(());
    }

    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    let process_error = std::io::Error::last_os_error();
    if matches!(process_error.raw_os_error(), Some(code) if code == libc::ESRCH) {
        return Ok(());
    }

    Err(InfraError::Pty {
        phase: PtyPhase::Shutdown,
        message: format!("failed signaling pty process {pid}: {process_error}"),
    }
    .into())
}

#[cfg(unix)]
fn signal_managed_process(pid: Option<u32>, signal: i32) -> AppResult<()> {
    let Some(pid) = pid else {
        return Ok(());
    };
    let pid = pid as i32;

    if unsafe { libc::killpg(pid, signal) } == 0 {
        return Ok(());
    }
    let process_group_error = std::io::Error::last_os_error();
    if matches!(process_group_error.raw_os_error(), Some(code) if code == libc::ESRCH) {
        return Ok(());
    }

    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    let process_error = std::io::Error::last_os_error();
    if matches!(process_error.raw_os_error(), Some(code) if code == libc::ESRCH) {
        return Ok(());
    }

    Err(InfraError::Runtime(format!(
        "failed signaling managed process {pid}: {process_error}"
    ))
    .into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use tokio::sync::{mpsc, Mutex};

    use super::{execution_mode_for, spawn_managed_command, CancellationRegistry, ExecutionMode};
    use crate::app::AppEvent;
    use crate::domain::{CommandProvenance, ExecutionRequest, SafetyClass};
    use crate::shared::ids::{CommandId, JobId};

    #[test]
    fn chooses_pty_for_interactive_foreground_commands() {
        assert!(matches!(
            execution_mode_for("top", false),
            ExecutionMode::Pty
        ));
        assert!(matches!(
            execution_mode_for("bash", false),
            ExecutionMode::Pty
        ));
    }

    #[test]
    fn keeps_managed_mode_for_background_and_operator_commands() {
        assert!(matches!(
            execution_mode_for("top", true),
            ExecutionMode::Managed
        ));
        assert!(matches!(
            execution_mode_for("echo hi | less", false),
            ExecutionMode::Managed
        ));
        assert!(matches!(
            execution_mode_for("pwd", false),
            ExecutionMode::Managed
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn managed_command_cancellation_emits_cancelled_event() {
        let (events_tx, mut events_rx) = mpsc::channel(16);
        let cancellations: CancellationRegistry = Arc::new(Mutex::new(Default::default()));
        let request = ExecutionRequest {
            command_id: CommandId(1),
            job_id: JobId(2),
            service_id: None,
            raw: "sleep 30".to_string(),
            cwd: std::env::current_dir().expect("current dir should exist"),
            env: BTreeMap::new(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            mode: ExecutionMode::Managed,
            provenance: CommandProvenance::UserInput,
            background: false,
            safety_class: SafetyClass::Passive,
        };

        spawn_managed_command(request, events_tx, cancellations.clone())
            .expect("managed command should spawn");

        let started = tokio::time::timeout(std::time::Duration::from_secs(2), events_rx.recv())
            .await
            .expect("command should start")
            .expect("start event should be sent");
        assert!(matches!(
            started,
            AppEvent::CommandStarted {
                command_id: CommandId(1),
                ..
            }
        ));

        let cancel_tx = {
            let mut registry = cancellations.lock().await;
            registry
                .get_mut(&CommandId(1))
                .and_then(Option::take)
                .expect("command should be cancellable")
        };
        let _ = cancel_tx.send(());

        loop {
            let event = tokio::time::timeout(std::time::Duration::from_secs(2), events_rx.recv())
                .await
                .expect("cancelled event should arrive")
                .expect("cancelled event should be sent");
            if matches!(
                event,
                AppEvent::CommandCancelled {
                    command_id: CommandId(1),
                    ..
                }
            ) {
                break;
            }
        }

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if cancellations.lock().await.is_empty() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("cancellation registry should be cleaned up");
    }
}
