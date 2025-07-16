mod export;
mod file;
mod group;
mod session;
mod task;
mod tree;

use std::path::{Path, PathBuf};
use tokio::io::{stdin, stdout};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::export::{export_from, extract_completed};

const EXPORT_ICS: &str = "tasktree.export";
const EXTRACT_COMPLETED: &str = "tasktree.cleanup";

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        path: Mutex::new(None),
    });
    Server::new(stdin(), stdout(), socket).serve(service).await;
}

#[derive(Debug)]
struct Backend {
    client: Client,
    path: Mutex<Option<PathBuf>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![EXPORT_ICS.to_string()],
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut p = self.path.lock().await;
        *p = params.text_document.uri.to_file_path().ok();
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            EXPORT_ICS => {
                let p = self.path.lock().await;
                if let Some(ref path) = *p {
                    if let Err(err) = export_from(path) {
                        self.client
                            .show_message(MessageType::ERROR, format!("🔴 {}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🟢 {}", "Exported!"))
                            .await;
                    }
                } else {
                    self.client
                        .show_message(MessageType::ERROR, "🔴 Missing opened file")
                        .await;
                }
            }
            EXTRACT_COMPLETED => {
                let p = self.path.lock().await;
                if let Some(ref path) = *p {
                    // TODO: Implement finding `plan` root
                    if let Err(err) =
                        extract_completed(path, &Path::new("/Users/user/notes/plan/done.md"))
                    {
                        self.client
                            .show_message(MessageType::ERROR, format!("{}", err))
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::INFO, format!("🟢 {}", "Extracted!"))
                            .await;
                    }
                }
            }
            _ => { /* ignore unknown commands */ }
        }
        Ok(None)
    }
}
