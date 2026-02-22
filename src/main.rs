mod chart;
mod commands;
mod context;
mod group;
mod ranged_err;
mod session;
mod task;
mod tasktree;
mod tree;

use commands::{export_ics, extract_completed};
use context::{Config, Context};
use std::fs;
use std::str::FromStr;
use tokio::io::{stdin, stdout};
use tokio::sync::OnceCell;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::InitializeParams;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::session::Session;

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

#[cfg(not(test))]
pub fn context() -> &'static Context {
    CONTEXT.get().expect("initialised")
}

#[cfg(test)]
pub fn context() -> &'static Context {
    static TEST_CONTEXT: std::sync::OnceLock<Context> = std::sync::OnceLock::new();
    TEST_CONTEXT.get_or_init(|| Context::dummy())
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
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["`".to_string()]),
                    ..Default::default()
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        // Required to trigger `did_change` callback
                        change: Some(TextDocumentSyncKind::FULL),
                        // Required to trigger `did_save` callback
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
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if !context().enabled(&params.text_document.uri) {
            return;
        };
        if let Some(last) = params.content_changes.last() {
            let task_tree = crate::tasktree::TaskTree::from_str(&last.text);
            match task_tree {
                Ok(_) => {
                    self.client
                        .publish_diagnostics(params.text_document.uri, vec![], None)
                        .await;
                }
                Err(ranged) => {
                    if let Some(range) = ranged.range {
                        let diagnostic = Diagnostic {
                            range: range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: ranged.error.to_string(),
                            ..Default::default()
                        };
                        self.client
                            .publish_diagnostics(params.text_document.uri, vec![diagnostic], None)
                            .await;
                    } else {
                        self.client
                            .show_message(MessageType::ERROR, ranged.error)
                            .await
                    }
                }
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if !context().enabled(&params.text_document.uri) {
            return;
        };
        if let Ok(file_path) = params.text_document.uri.to_file_path() {
            if file_path == context().todo() {
                if let Err(err) = export_ics(context()).await {
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

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            EXPORT_ICS => {
                if let Err(err) = export_ics(context()).await {
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
                if let Err(err) = extract_completed(context()) {
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
        Ok(None)
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        if !context().enabled(&params.text_document_position.text_document.uri) {
            return Ok(None);
        };
        Ok(
            if let Some(ctx) = params.context
                && ctx.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER
                && ctx.trigger_character == Some("`".to_string())
            {
                Some(CompletionResponse::Array(
                    (0..3)
                        .map(|i| CompletionItem {
                            label: Session::next_hour(context().config().timezone, i).to_string(),
                            ..Default::default()
                        })
                        .collect(),
                ))
            } else {
                None
            },
        )
    }

    async fn code_action(
        &self,
        _params: CodeActionParams,
    ) -> jsonrpc::Result<Option<CodeActionResponse>> {
        // TODO: Add code action to toggle the task
        Ok(None)
    }
}
