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
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams,
        GotoDefinitionParams, GotoDefinitionResponse, InitializeParams, InitializeResult,
        InitializedParams, Location, OneOf, Position, Range, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
    },
};
use trask::{ParseError, TASK_FILE, TASKS_DIR, Task, TaskId, TaskStore};

const TASK_ID_LEN: usize = 15;

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

    fn is_task_document(uri: &Url) -> bool {
        uri.to_file_path()
            .ok()
            .and_then(|path| path.file_name().map(|name| name == TASK_FILE))
            .unwrap_or(false)
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

    async fn clear_diagnostics(&self, uri: Url) {
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
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
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
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
        let version = params.text_document.version;
        let text = params.text_document.text;

        self.documents
            .write()
            .unwrap()
            .insert(uri.clone(), text.clone());

        if !Self::is_task_document(&uri) {
            self.clear_diagnostics(uri).await;
            return;
        }

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

        if !Self::is_task_document(&uri) {
            self.clear_diagnostics(uri).await;
            return;
        }

        self.publish_diagnostics(uri, &change.text, Some(version))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        self.documents.write().unwrap().remove(&uri);
        self.clear_diagnostics(uri).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let text_document_position = params.text_document_position_params;
        let uri = text_document_position.text_document.uri;
        let position = text_document_position.position;

        let Some(text) = self.document(&uri) else {
            return Ok(None);
        };

        let Some(line) = text.lines().nth(position.line as usize) else {
            return Ok(None);
        };

        let Some(id) = task_id_at(line, position.character) else {
            return Ok(None);
        };

        let path = self.root().join(TASKS_DIR).join(id).join(TASK_FILE);

        if !path.is_file() {
            return Ok(None);
        }

        let Ok(task_uri) = Url::from_file_path(path) else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            task_uri,
            Range::new(Position::new(0, 0), Position::new(0, 0)),
        ))))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;

        if !Self::is_task_document(uri) {
            return Ok(None);
        }

        let position = params.text_document_position.position;

        let Some(text) = self.document(uri) else {
            return Ok(None);
        };

        let Some(line) = text.lines().nth(position.line as usize) else {
            return Ok(None);
        };

        let cursor = utf16_to_byte_index(line, position.character);

        if let Some(value) = field_value(line, cursor, "STATUS") {
            let prefix = value.trim().to_ascii_lowercase();

            let statuses = [
                ("OPEN", &["open", "opened", "o"][..]),
                ("CLOSED", &["closed", "close", "c"][..]),
            ];

            let items = statuses
                .into_iter()
                .filter(|(_, aliases)| {
                    prefix.is_empty() || aliases.iter().any(|alias| alias.starts_with(&prefix))
                })
                .map(|(status, aliases)| CompletionItem {
                    label: status.to_string(),
                    filter_text: Some(aliases.join(" ")),
                    insert_text: Some(status.to_string()),
                    kind: Some(CompletionItemKind::ENUM),
                    ..Default::default()
                })
                .collect();

            return Ok(Some(CompletionResponse::Array(items)));
        }

        if let Some(value) = field_value(line, cursor, "TAGS") {
            let prefix = value
                .rsplit(',')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();

            let items = self
                .tags()
                .into_iter()
                .filter(|tag| tag.to_ascii_lowercase().starts_with(&prefix))
                .map(|tag| CompletionItem {
                    label: tag.clone(),
                    filter_text: Some(tag.clone()),
                    insert_text: Some(tag),
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                })
                .collect();

            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        if !Self::is_task_document(uri) {
            return Ok(None);
        }

        let Some(text) = self.document(uri) else {
            return Ok(None);
        };

        let Ok(task) = Task::from_markdown(&text) else {
            return Ok(None);
        };

        let formatted = task.to_markdown();

        if formatted == text {
            return Ok(None);
        }

        let range = Range::new(Position::new(0, 0), document_end(&text));

        Ok(Some(vec![TextEdit::new(range, formatted)]))
    }
}

/// Returns a Trask task ID when the cursor is positioned on one.
///
/// Expected format: YYYYMMDD-HHMMSS.
fn task_id_at(line: &str, cursor: u32) -> Option<&str> {
    let bytes = line.as_bytes();

    for (start, _) in line.char_indices() {
        let end = start + TASK_ID_LEN;

        let Some(candidate) = line.get(start..end) else {
            continue;
        };

        let candidate_bytes = candidate.as_bytes();

        let valid = candidate_bytes[..8].iter().all(u8::is_ascii_digit)
            && candidate_bytes[8] == b'-'
            && candidate_bytes[9..].iter().all(u8::is_ascii_digit);

        if !valid {
            continue;
        }

        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }

        if end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            continue;
        }

        let start_utf16 = line[..start].encode_utf16().count() as u32;
        let end_utf16 = start_utf16 + TASK_ID_LEN as u32;

        if (start_utf16..end_utf16).contains(&cursor) {
            return Some(candidate);
        }
    }

    None
}

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let Err(error) = Task::from_markdown(text) else {
        return Vec::new();
    };

    let line_number = match &error {
        ParseError::InvalidStatus(_) => find_metadata_line(text, "STATUS"),
        ParseError::InvalidPriority => find_metadata_line(text, "PRIORITY"),
        ParseError::InvalidFormat(message) if message.contains("STATUS") => {
            find_metadata_line(text, "STATUS")
        }
        ParseError::InvalidFormat(message) if message.contains("PRIORITY") => {
            find_metadata_line(text, "PRIORITY")
        }
        _ => None,
    }
    .unwrap_or(0);

    let line = text.lines().nth(line_number).unwrap_or("");
    let end = line.encode_utf16().count().max(1) as u32;

    vec![Diagnostic {
        range: Range::new(
            Position::new(line_number as u32, 0),
            Position::new(line_number as u32, end),
        ),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("trask".to_string()),
        message: error.to_string(),
        ..Default::default()
    }]
}

fn find_metadata_line(text: &str, field: &str) -> Option<usize> {
    text.lines().position(|line| {
        let line = line.trim().strip_prefix('-').unwrap_or(line).trim();

        line.split_once(':')
            .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case(field))
    })
}

/// Returns a metadata value up to the cursor, accepting flexible spacing.
fn field_value<'a>(line: &'a str, cursor: usize, field: &str) -> Option<&'a str> {
    let mut start = line.len() - line.trim_start().len();

    if let Some(after_marker) = line[start..].strip_prefix('-') {
        start += 1 + after_marker.len() - after_marker.trim_start().len();
    }

    let metadata = &line[start..];
    let (key, _) = metadata.split_once(':')?;

    if !key.trim().eq_ignore_ascii_case(field) {
        return None;
    }

    let value_start = start + key.len() + 1;
    let remaining = &line[value_start..];
    let value_start = value_start + remaining.len() - remaining.trim_start().len();

    if cursor < value_start {
        return None;
    }

    line.get(value_start..cursor.min(line.len()))
}

fn utf16_to_byte_index(text: &str, target: u32) -> usize {
    let mut utf16_offset = 0;

    for (byte_index, character) in text.char_indices() {
        if utf16_offset >= target {
            return byte_index;
        }

        let width = character.len_utf16() as u32;

        if utf16_offset + width > target {
            return byte_index;
        }

        utf16_offset += width;
    }

    text.len()
}

fn document_end(text: &str) -> Position {
    let line = text.bytes().filter(|byte| *byte == b'\n').count() as u32;

    let character = text
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r')
        .encode_utf16()
        .count() as u32;

    Position::new(line, character)
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin(), stdout(), socket).serve(service).await;
}
