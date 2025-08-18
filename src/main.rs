mod commands;
mod context;
mod file;
mod group;
mod session;
mod task;
mod tree;

use commands::{export_ics, extract_completed};
use context::{Config, Context};
use std::fs;
use tokio::io::{stdin, stdout};
use tokio::sync::OnceCell;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::InitializeParams;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const EXPORT_ICS: &str = "tasktree.export";
const EXTRACT_COMPLETED: &str = "tasktree.cleanup";

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin(), stdout(), socket).serve(service).await;
}

#[derive(Debug)]
struct Backend {
    client: Client,
}

static CONTEXT: OnceCell<Context> = OnceCell::const_new();

pub fn context() -> &'static Context {
    CONTEXT.get().expect("initialised")
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        // Find provided workspace root
        let workspace = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .ok_or_else(|| jsonrpc::Error::invalid_params("No workspace folder found"))?;
        // Find and load .task-tree.toml config file
        let config: Config = fs::read_to_string(&workspace.join(".task-tree.toml"))
            .map_err(|_| jsonrpc::Error::invalid_params("Failed to find .task-tree.toml file"))
            .and_then(|content| {
                toml::from_str(&content).map_err(|e| {
                    jsonrpc::Error::invalid_params(format!(
                        "Failed to parse .task-tree.toml: {}",
                        e
                    ))
                })
            })?;
        CONTEXT
            .set(Context::new(config, workspace))
            .expect("Init should be called only once");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
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

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Ok(file_path) = params.text_document.uri.to_file_path() {
            if let Some(context) = CONTEXT.get() {
                if file_path == context.todo() {
                    if let Err(err) = export_ics(context).await {
                        self.client
                            .show_message(MessageType::ERROR, format!("🌳 {}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🌳 {}", "Calendar exported."))
                            .await;
                    }
                }
            }
        }
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        if let Some(context) = CONTEXT.get() {
            match params.command.as_str() {
                EXPORT_ICS => {
                    if let Err(err) = export_ics(context).await {
                        self.client
                            .show_message(MessageType::ERROR, format!("🌳 {}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🌳 {}", "Calendar exported."))
                            .await;
                    }
                }
                EXTRACT_COMPLETED => {
                    if let Err(err) = extract_completed(context) {
                        self.client
                            .show_message(MessageType::ERROR, format!("{}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🌳 {}", "Moved completed"))
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
