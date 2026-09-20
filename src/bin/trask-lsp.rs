use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::io::{stdin, stdout};
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
        CompletionResponse, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        InitializedParams, Position, Range, ServerCapabilities, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
};
use trask::{Task, TaskId, TaskStore};

const STATUS_PREFIX: &str = "- STATUS: ";
const TAGS_PREFIX: &str = "- TAGS: ";

struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    root: Arc<RwLock<Option<PathBuf>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            root: Arc::new(RwLock::new(None)),
        }
    }

    fn root(&self) -> PathBuf {
        self.root
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn document(&self, uri: &Url) -> Option<String> {
        self.documents.read().unwrap().get(uri).cloned()
    }

    fn tags(&self) -> Vec<String> {
        let store = TaskStore::new(self.root());
        let mut counts = HashMap::new();

        let Ok(entries) = fs::read_dir(store.tasks_dir()) else {
            return Vec::new();
        };

        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }

            let file_name = entry.file_name();

            let Some(id) = file_name.to_str() else {
                continue;
            };

            let task_id = TaskId::new(id);

            let Ok(task) = store.load(&task_id) else {
                continue;
            };

            for tag in task.task.tags {
                *counts.entry(tag).or_insert(0usize) += 1;
            }
        }

        let mut tags = counts.into_iter().collect::<Vec<_>>();

        tags.sort_by(|(tag_a, count_a), (tag_b, count_b)| {
            count_b.cmp(count_a).then_with(|| tag_a.cmp(tag_b))
        });

        tags.into_iter().map(|(tag, _)| tag).collect()
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str, version: Option<i32>) {
        self.client
            .publish_diagnostics(uri, diagnostics(text), version)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params
            .workspace_folders
            .and_then(|folders| folders.into_iter().next())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .or_else(|| params.root_uri.and_then(|uri| uri.to_file_path().ok()));

        *self.root.write().unwrap() = root;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), ",".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                "trask-lsp initialized",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        self.documents
            .write()
            .unwrap()
            .insert(uri.clone(), text.clone());

        self.publish_diagnostics(uri, &text, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };

        self.documents
            .write()
            .unwrap()
            .insert(uri.clone(), change.text.clone());

        self.publish_diagnostics(uri, &change.text, Some(version))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        self.documents.write().unwrap().remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let Some(text) = self.document(uri) else {
            return Ok(None);
        };

        let Some(line) = text.lines().nth(position.line as usize) else {
            return Ok(None);
        };

        let character = position.character as usize;

        if status_value(line, character).is_some() {
            let items = ["OPEN", "CLOSED"]
                .into_iter()
                .map(|status| CompletionItem {
                    label: status.to_string(),
                    kind: Some(CompletionItemKind::ENUM),
                    ..Default::default()
                })
                .collect();

            return Ok(Some(CompletionResponse::Array(items)));
        }

        if let Some(value) = tag_value(line, character) {
            let items = self
                .tags()
                .into_iter()
                .filter(|tag| tag.starts_with(value))
                .map(|tag| CompletionItem {
                    label: tag,
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                })
                .collect();

            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(None)
    }
}

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    match Task::from_markdown(text) {
        Ok(_) => Vec::new(),
        Err(error) => vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("trask".to_string()),
            message: error.to_string(),
            ..Default::default()
        }],
    }
}

fn status_value(line: &str, cursor: usize) -> Option<&str> {
    let value = line.strip_prefix(STATUS_PREFIX)?;
    let cursor = cursor.saturating_sub(STATUS_PREFIX.len());

    Some(&value[..cursor.min(value.len())])
}

fn tag_value(line: &str, cursor: usize) -> Option<&str> {
    let value = line.strip_prefix(TAGS_PREFIX)?;
    let cursor = cursor.saturating_sub(TAGS_PREFIX.len());
    let value = &value[..cursor.min(value.len())];

    value.rsplit(',').next().map(str::trim)
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin(), stdout(), socket).serve(service).await;
}
