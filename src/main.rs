mod commands;
mod file;
mod group;
mod session;
mod task;
mod tree;

use std::fs;
use tokio::io::{stdin, stdout};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::commands::{export_ics, extract_completed};
use crate::config::{Config, Workspace};

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

#[derive(Debug)]
struct Context {
    config: Config,
    workspace: Workspace,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Find and load .task-tree.toml config file
        let workspace = Workspace::from(params)
            .ok_or_else(|| Error::invalid_params("No workspace folder found"))?;
        let config_content = fs::read_to_string(&workspace.path(config::Path::Config))
            .map_err(|_| Error::invalid_params("Failed to find .task-tree.toml file"))?;
        let config: Config = toml::from_str(&config_content).map_err(|e| {
            Error::invalid_params(format!("Failed to parse .task-tree.toml: {}", e))
        })?;

        // Log the decoded config
        self.client
            .log_message(MessageType::INFO, format!("Loaded config: {:?}", config))
            .await;
        *self.context.lock().await = Some(Context { config, workspace });
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
                    let from = context.workspace.path(config::Path::Todo);
                    let to = context.workspace.path(config::Path::Done);
                    if let Err(err) = extract_completed(&from, &to) {
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

mod config {
    use serde::Deserialize;
    use std::path::PathBuf;
    use tower_lsp::lsp_types::InitializeParams;

    #[derive(Deserialize, Debug)]
    pub struct Config {
        pub caldav: CalDAV,
    }

    #[derive(Debug)]
    pub struct Workspace {
        root: PathBuf,
    }

    impl Workspace {
        pub fn from(params: InitializeParams) -> Option<Workspace> {
            params
                .workspace_folders
                .as_ref()
                .and_then(|folders| folders.first())
                .and_then(|folder| folder.uri.to_file_path().ok())
                .map(|uri| Workspace { root: uri })
        }

        pub fn path(&self, file: Path) -> PathBuf {
            self.root.join(match file {
                // Path::Idea => "idea.md",
                Path::Todo => "todo.md",
                Path::Done => "done.md",
                Path::Config => ".task-tree.toml",
            })
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct CalDAV {
        pub url: String,
        pub user: String,
        pub pass: String,
    }

    pub enum Path {
        // Idea,
        Todo,
        Done,
        Config,
    }
}
