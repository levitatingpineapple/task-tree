mod commands;
mod context;
mod file;
mod group;
mod session;
mod task;
mod tree;

use crate::commands::{export_ics, extract_completed};
use context::{Config, Context};
use std::fs;
use tokio::io::{stdin, stdout};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::InitializeParams;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const EXPORT_ICS: &str = "tasktree.export";
const EXTRACT_COMPLETED: &str = "tasktree.cleanup";

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        context: Mutex::new(None),
    });
    Server::new(stdin(), stdout(), socket).serve(service).await;
}

#[derive(Debug)]
struct Backend {
    client: Client,
    context: Mutex<Option<Context>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Find provided workspace root
        let workspace = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .ok_or_else(|| Error::invalid_params("No workspace folder found"))?;
        // Find and load .task-tree.toml config file
        let config: Config = fs::read_to_string(&workspace.join(".task-tree.toml"))
            .map_err(|_| Error::invalid_params("Failed to find .task-tree.toml file"))
            .and_then(|content| {
                toml::from_str(&content).map_err(|e| {
                    Error::invalid_params(format!("Failed to parse .task-tree.toml: {}", e))
                })
            })?;
        *self.context.lock().await = Some(Context::new(config, workspace));
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: [EXPORT_ICS, EXTRACT_COMPLETED]
                        .map(|str| str.to_string())
                        .to_vec(),
                    work_done_progress_options: Default::default(),
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        let lock = self.context.lock().await;

        if let Some(ref context) = *lock {
            match params.command.as_str() {
                EXPORT_ICS => {
                    if let Err(err) = export_ics(context).await {
                        self.client
                            .show_message(MessageType::ERROR, format!("🔴 {}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🟢 {}", "Exported!"))
                            .await;
                    }
                }
                EXTRACT_COMPLETED => {
                    if let Err(err) = extract_completed(&context.todo(), &context.done()) {
                        self.client
                            .show_message(MessageType::ERROR, format!("{}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🟢 {}", "Extracted!"))
                            .await;
                    }
                }
                _ => { /* ignore unknown commands */ }
            }
        } else {
            self.client
                .show_message(MessageType::ERROR, "🔴 Missing opened file")
                .await;
        }

        Ok(None)
    }
}
