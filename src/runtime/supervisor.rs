use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};

use crate::ai::AiRuntime;
use crate::app::AppEvent;
use crate::domain::{AiRequest, ExecutionRequest};
use crate::infra::{git, project, shell};
use crate::shared::error::AppResult;
use crate::shared::ids::CommandId;

#[derive(Clone)]
pub struct RuntimeSupervisor {
    events_tx: mpsc::Sender<AppEvent>,
    ai_runtime: AiRuntime,
    cancellations: Arc<Mutex<HashMap<CommandId, oneshot::Sender<()>>>>,
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
        if let Some(cancel_tx) = registry.remove(&command_id) {
            let _ = cancel_tx.send(());
        }
        Ok(())
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
