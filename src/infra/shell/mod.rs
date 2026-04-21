use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::app::AppEvent;
use crate::domain::{ExecutionMode, ExecutionRequest, OutputStream};
use crate::shared::error::{AppResult, InfraError};
use crate::shared::ids::CommandId;

pub type CancellationRegistry = Arc<Mutex<HashMap<CommandId, oneshot::Sender<()>>>>;

pub fn spawn_managed_command(
    request: ExecutionRequest,
    events_tx: mpsc::Sender<AppEvent>,
    cancellations: CancellationRegistry,
) -> AppResult<()> {
    if !matches!(request.mode, ExecutionMode::Managed) {
        return Err(InfraError::Runtime("PTY execution is not implemented yet".to_string()).into());
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
            registry.insert(command_id, cancel_tx);
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
                if let Err(error) = child.kill().await {
                    let _ = events_tx_for_task.send(AppEvent::Error(format!(
                        "failed to cancel command `{raw}`: {error}"
                    ))).await;
                } else {
                    let _ = events_tx_for_task.send(AppEvent::CommandCancelled {
                        command_id,
                        job_id,
                        service_id,
                    }).await;
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

fn build_shell_command(request: &ExecutionRequest) -> Command {
    let mut command = if cfg!(windows) {
        let shell = if request.shell.trim().is_empty() {
            "cmd".to_string()
        } else {
            request.shell.clone()
        };
        let mut command = Command::new(shell);
        command.arg("/C").arg(&request.raw);
        command
    } else {
        let shell = if request.shell.trim().is_empty() {
            "/bin/sh".to_string()
        } else {
            request.shell.clone()
        };
        let mut command = Command::new(shell);
        command.arg("-lc").arg(&request.raw);
        command
    };

    command.current_dir(&request.cwd);
    for (key, value) in &request.env {
        command.env(key, value);
    }
    command.kill_on_drop(true);
    command
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
