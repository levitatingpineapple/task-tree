use crate::{
    commands::{export_ics, extract_completed},
    context::{self},
    session::Session,
};
use std::str::FromStr;
use tower_lsp::{Client, LanguageServer, jsonrpc, lsp_types::*};

const CHART: &str = "tasktree.chart";
const EXPORT_ICS: &str = "tasktree.export";
const EXTRACT_COMPLETED: &str = "tasktree.cleanup";

#[derive(Debug)]
pub struct TaskTreeServer {
    client: Client,
}

impl TaskTreeServer {
    pub fn new(client: Client) -> Self {
        TaskTreeServer { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TaskTreeServer {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        let workspace = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .ok_or_else(|| jsonrpc::Error::invalid_params("No workspace folder found"))?;
        // Find and load .task-tree.toml config file
        context::set(&workspace).map_err(|e| jsonrpc::Error::invalid_params(e.to_string()))?;
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
                    commands: [CHART, EXPORT_ICS, EXTRACT_COMPLETED]
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
        if !context::get().enabled(&params.text_document.uri) {
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
        let ctx = context::get();
        if !ctx.enabled(&params.text_document.uri) {
            return;
        };
        if let Ok(file_path) = params.text_document.uri.to_file_path() {
            if file_path == ctx.todo() {
                if let Err(err) = export_ics(ctx).await {
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
        let ctx = context::get();
        match params.command.as_str() {
            CHART => {
                crate::chart::run_chart(&ctx.todo());
            }
            EXPORT_ICS => {
                if let Err(err) = export_ics(ctx).await {
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
                if let Err(err) = extract_completed(ctx) {
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
        if !context::get().enabled(&params.text_document_position.text_document.uri) {
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
                            label: Session::next_hour(context::get().config().timezone, i)
                                .to_string(),
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
