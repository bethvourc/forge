use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Mutex};

use crate::ai::AiRuntime;
use crate::app::AppEvent;
use crate::domain::{AiRequest, ExecutionRequest};
use crate::infra::{git, project, shell};
use crate::shared::error::{AppResult, InfraError};
use crate::shared::ids::CommandId;

#[derive(Clone)]
pub struct RuntimeSupervisor {
    events_tx: mpsc::Sender<AppEvent>,
    ai_runtime: AiRuntime,
    cancellations: shell::CancellationRegistry,
}

impl RuntimeSupervisor {
    pub fn new(events_tx: mpsc::Sender<AppEvent>, ai_runtime: AiRuntime) -> Self {
        Self {
            events_tx,
            ai_runtime,
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn events_tx(&self) -> mpsc::Sender<AppEvent> {
        self.events_tx.clone()
    }

    pub async fn execute_command(&self, request: ExecutionRequest) -> AppResult<()> {
        shell::spawn_managed_command(request, self.events_tx.clone(), self.cancellations.clone())
    }

    pub async fn run_ai_request(&self, request: AiRequest) -> AppResult<()> {
        let events_tx = self.events_tx.clone();
        let ai_runtime = self.ai_runtime.clone();
        tokio::spawn(async move {
            let request_id = request.id;
            let provider = ai_runtime.provider_name().to_string();

            let _ = events_tx
                .send(AppEvent::AiStarted {
                    request_id,
                    provider: provider.clone(),
                })
                .await;

            match ai_runtime.execute(request).await {
                Ok(response) => {
                    let _ = events_tx.send(AppEvent::AiCompleted(response)).await;
                }
                Err(error) => {
                    let _ = events_tx
                        .send(AppEvent::AiFailed {
                            request_id,
                            provider,
                            message: error.to_string(),
                        })
                        .await;
                }
            }
        });
        Ok(())
    }

    pub async fn cancel_command(&self, command_id: CommandId) -> AppResult<()> {
        let mut registry = self.cancellations.lock().await;
        if let Some(cancel_tx) = registry.get_mut(&command_id).and_then(Option::take) {
            let _ = cancel_tx.send(());
        }
        Ok(())
    }

    pub async fn shutdown_commands(&self, timeout: Duration) -> AppResult<()> {
        {
            let mut registry = self.cancellations.lock().await;
            for cancel_tx in registry.values_mut().filter_map(Option::take) {
                let _ = cancel_tx.send(());
            }
        }

        let deadline = Instant::now() + timeout;
        loop {
            if self.cancellations.lock().await.is_empty() {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(InfraError::Runtime(
                    "timed out waiting for managed commands to shut down".to_string(),
                )
                .into());
            }

            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    pub fn refresh_project(&self, start_dir: PathBuf) {
        let events_tx = self.events_tx.clone();
        tokio::spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || project::scan_project(&start_dir, None, None))
                    .await;
            match result {
                Ok(Ok(project)) => {
                    let _ = events_tx.send(AppEvent::ProjectScanned(project)).await;
                }
                Ok(Err(error)) => {
                    let _ = events_tx.send(AppEvent::Error(error.to_string())).await;
                }
                Err(error) => {
                    let _ = events_tx.send(AppEvent::Error(error.to_string())).await;
                }
            }
        });
    }

    pub fn refresh_git(&self, root: PathBuf) {
        let events_tx = self.events_tx.clone();
        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || git::snapshot_git(&root)).await;
            match result {
                Ok(Ok(snapshot)) => {
                    let _ = events_tx.send(AppEvent::GitRefreshed(snapshot)).await;
                }
                Ok(Err(error)) => {
                    let _ = events_tx.send(AppEvent::Error(error.to_string())).await;
                }
                Err(error) => {
                    let _ = events_tx.send(AppEvent::Error(error.to_string())).await;
                }
            }
        });
    }
}
