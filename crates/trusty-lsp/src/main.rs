use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const TRUSTY_MODULES: &[&str] = &["trusty:time", "trusty:math", "trusty:rand"];

fn trusty_module_exports(module_path: &str) -> &'static [&'static str] {
    match module_path {
        "trusty:math" => &[
            "PI", "E", "sqrt", "pow", "log", "abs", "min", "max", "clamp", "sin", "cos", "tan", "asin", "acos", "atan",
        ],
        "trusty:time" => &[
            "Instant", "Duration", "sleep", "Date", "Time", "DateTime", "SystemTime", "compare", "addSeconds", "addMinutes", "addDays",
            "addMonths", "addYears", "subSeconds", "subMinutes", "subDays", "subMonths", "subYears",
        ],
        "trusty:rand" => &["random", "randomInt", "randomFloat", "bernoulli", "weightedIndex", "chooseOne", "shuffle"],
        _ => &[],
    }
}

struct Backend {
    client: Client,
    docs: Arc<RwLock<HashMap<Url, String>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let diagnostics = match trusty_compiler::compile(text) {
            Ok(_) => Vec::new(),
            Err(err) => {
                let message = err.to_string();
                let range = Self::range_from_error_message(text, &message).unwrap_or(Range {
                    start: Position::new(0, 0),
                    end: Position::new(0, 1),
                });
                vec![Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("trusty-compiler".to_string()),
                    message,
                    related_information: None,
                    tags: None,
                    data: None,
                }]
            }
        };

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    fn range_from_error_message(text: &str, message: &str) -> Option<Range> {
        let (start, end) = Self::extract_byte_span(message)?;
        let text_len = text.len();
        let start = start.min(text_len);
        let end = end.max(start.saturating_add(1)).min(text_len);
        let start_pos = Self::byte_offset_to_position(text, start);
        let mut end_pos = Self::byte_offset_to_position(text, end);
        if end_pos == start_pos {
            end_pos.character = end_pos.character.saturating_add(1);
        }
        Some(Range {
            start: start_pos,
            end: end_pos,
        })
    }

    fn extract_byte_span(message: &str) -> Option<(usize, usize)> {
        let bytes = message.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if !bytes[i].is_ascii_digit() {
                i += 1;
                continue;
            }
            let start_i = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 >= bytes.len() || bytes[i] != b'.' || bytes[i + 1] != b'.' {
                continue;
            }
            let start = message[start_i..i].parse::<usize>().ok()?;
            i += 2;
            let end_i = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if end_i == i {
                continue;
            }
            let end = message[end_i..i].parse::<usize>().ok()?;
            return Some((start, end));
        }
        None
    }

    fn byte_offset_to_position(text: &str, offset: usize) -> Position {
        let mut line = 0u32;
        let mut character = 0u32;
        let mut seen = 0usize;
        let limit = offset.min(text.len());

        for ch in text.chars() {
            let ch_len = ch.len_utf8();
            if seen + ch_len > limit {
                break;
            }
            seen += ch_len;
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += ch.len_utf16() as u32;
            }
            if seen == limit {
                break;
            }
        }

        Position::new(line, character)
    }

    fn completion_items() -> Vec<CompletionItem> {
        let keywords = [
            "function", "struct", "enum", "implements", "import", "export", "from", "val", "var", "const",
            "if", "else", "match", "default", "try", "catch", "finally", "for", "in", "of", "loop",
            "break", "continue", "return", "throw", "and", "or", "async", "await",
        ];
        let types = [
            "int", "int8", "int16", "int32", "int64", "float", "float32", "float64", "string", "boolean",
            "Pointer", "Threaded", "Map", "Set", "Result",
        ];
        let builtins = ["string", "boolean", "int32", "float64", "console.write"];

        let mut out = Vec::new();
        for kw in keywords {
            out.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..CompletionItem::default()
            });
        }
        for ty in types {
            out.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..CompletionItem::default()
            });
        }
        for b in builtins {
            out.push(CompletionItem {
                label: b.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                ..CompletionItem::default()
            });
        }
        out
    }

    fn line_prefix(line: &str, col: usize) -> &str {
        let end = col.min(line.len());
        &line[..end]
    }

    fn completion_for_import_path(line: &str, col: usize) -> Option<Vec<CompletionItem>> {
        let prefix = Self::line_prefix(line, col);
        let from_idx = prefix.find("from \"")?;
        let module_prefix = &prefix[from_idx + "from \"".len()..];
        if module_prefix.contains('"') {
            return None;
        }
        if !("trusty:".starts_with(module_prefix) || module_prefix.starts_with("trusty:")) {
            return None;
        }

        let mut out = Vec::new();
        for m in TRUSTY_MODULES {
            out.push(CompletionItem {
                label: (*m).to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("TRUST stdlib module".to_string()),
                ..CompletionItem::default()
            });
        }
        Some(out)
    }

    fn parse_trusty_import_symbols_line(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if !trimmed.starts_with("import {") {
            return None;
        }
        let from_idx = trimmed.find(" from ")?;
        let after_from = trimmed[from_idx + " from ".len()..].trim();
        let quote = after_from.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let rest = &after_from[1..];
        let end = rest.find(quote)?;
        let path = &rest[..end];
        if path.starts_with("trusty:") {
            Some(path.to_string())
        } else {
            None
        }
    }

    fn completion_for_import_symbols(line: &str, col: usize) -> Option<Vec<CompletionItem>> {
        let prefix = Self::line_prefix(line, col);
        if !prefix.contains("import {") {
            return None;
        }
        let open = prefix.find('{')?;
        let close = prefix.find('}').unwrap_or(prefix.len());
        if col < open + 1 || col > close {
            return None;
        }
        let module = Self::parse_trusty_import_symbols_line(line)?;
        let exports = trusty_module_exports(&module);
        if exports.is_empty() {
            return None;
        }

        let mut out = Vec::new();
        for sym in exports {
            out.push(CompletionItem {
                label: (*sym).to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("export from {}", module)),
                ..CompletionItem::default()
            });
        }
        Some(out)
    }

    fn collect_struct_fields(text: &str) -> HashMap<String, Vec<String>> {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0usize;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let mut rest = trimmed.strip_prefix("struct ");
            if rest.is_none() {
                rest = trimmed.strip_prefix("export struct ");
            }
            let Some(rest) = rest else {
                i += 1;
                continue;
            };
            let Some(name) = rest.split_whitespace().next() else {
                i += 1;
                continue;
            };
            let type_name = name.trim_end_matches('{').trim().to_string();
            let mut fields = Vec::new();
            i += 1;
            while i < lines.len() {
                let f = lines[i].trim();
                if f.starts_with('}') {
                    break;
                }
                if let Some(colon) = f.find(':') {
                    let field = f[..colon].trim().trim_end_matches(';').to_string();
                    if !field.is_empty() {
                        fields.push(field);
                    }
                }
                i += 1;
            }
            if !type_name.is_empty() && !fields.is_empty() {
                out.insert(type_name, fields);
            }
            i += 1;
        }
        out
    }

    fn is_ident(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    fn parse_var_decl_type(trimmed: &str) -> Option<(String, String)> {
        let start = if let Some(r) = trimmed.strip_prefix("val ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("var ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("const ") {
            r
        } else {
            return None;
        };

        let mut chars = start.chars().peekable();
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if Self::is_ident(c) {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        let rest_owned = chars.collect::<String>();
        let rest = rest_owned.trim_start();
        if name.is_empty() {
            return None;
        }

        if let Some(after_colon) = rest.strip_prefix(':') {
            let after_colon = after_colon.trim_start();
            let ty: String = after_colon
                .chars()
                .take_while(|c| Self::is_ident(*c))
                .collect();
            if !ty.is_empty() {
                return Some((name, ty));
            }
        }

        if let Some(eq_idx) = rest.find('=') {
            let rhs = rest[eq_idx + 1..].trim_start();
            let ctor: String = rhs.chars().take_while(|c| Self::is_ident(*c)).collect();
            if ctor
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false)
            {
                return Some((name, ctor));
            }
        }
        None
    }

    fn collect_var_types_until(text: &str, max_line_inclusive: usize) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for (i, line) in text.lines().enumerate() {
            if i > max_line_inclusive {
                break;
            }
            if let Some((name, ty)) = Self::parse_var_decl_type(line.trim()) {
                out.insert(name, ty);
            }
        }
        out
    }

    fn member_target_before_cursor(line: &str, col: usize) -> Option<String> {
        let prefix = Self::line_prefix(line, col);
        let dot_idx = prefix.rfind('.')?;
        let before_dot = &prefix[..dot_idx];
        let mut name = String::new();
        for c in before_dot.chars().rev() {
            if Self::is_ident(c) {
                name.push(c);
            } else {
                break;
            }
        }
        if name.is_empty() {
            return None;
        }
        Some(name.chars().rev().collect())
    }

    fn completion_for_member_access(text: &str, line: usize, col: usize) -> Option<Vec<CompletionItem>> {
        let lines: Vec<&str> = text.lines().collect();
        let current = lines.get(line)?;
        let target = Self::member_target_before_cursor(current, col)?;

        let struct_fields = Self::collect_struct_fields(text);
        let var_types = Self::collect_var_types_until(text, line);
        let ty = var_types.get(&target)?;
        let fields = struct_fields.get(ty)?;

        let mut out = Vec::new();
        for f in fields {
            out.push(CompletionItem {
                label: f.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("field of {}", ty)),
                ..CompletionItem::default()
            });
        }
        Some(out)
    }

    fn hover_doc(word: &str) -> Option<&'static str> {
        match word {
            "val" => Some("`val`: immutable local variable."),
            "var" => Some("`var`: mutable local variable."),
            "const" => Some("`const`: global constant."),
            "match" => Some("`match (x) { pat => expr, default => expr }`: expression match."),
            "loop" => Some("`loop (cond) { ... }`: conditional loop."),
            "async" => Some("`async function`: runs the function body in a thread and returns a handle."),
            "await" => Some("`await handle`: waits for a spawned async handle (`join().unwrap()`)."),
            "string" => Some("`string(...)`: cast value to TRUST string."),
            "boolean" => Some("`boolean(...)`: cast value to TRUST boolean."),
            "int32" => Some("`int32`: 32-bit signed integer."),
            "float64" => Some("`float64`: 64-bit floating point."),
            "Point" => Some("Struct constructor style: `Point({ x: 1, y: 2 })`."),
            _ => None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "trusty-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), "\"".to_string(), "{".to_string(), ",".to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "trusty-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.write().await.insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.first() {
            let text = change.text.clone();
            self.docs.write().await.insert(uri.clone(), text.clone());
            self.publish_diagnostics(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.write().await.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let text_doc = &params.text_document_position.text_document;
        let position = params.text_document_position.position;

        let docs = self.docs.read().await;
        let Some(text) = docs.get(&text_doc.uri) else {
            return Ok(Some(CompletionResponse::Array(Self::completion_items())));
        };

        let lines: Vec<&str> = text.lines().collect();
        let Some(line) = lines.get(position.line as usize) else {
            return Ok(Some(CompletionResponse::Array(Self::completion_items())));
        };
        let col = position.character as usize;

        if let Some(items) = Self::completion_for_member_access(text, position.line as usize, col) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = Self::completion_for_import_symbols(line, col) {
            return Ok(Some(CompletionResponse::Array(items)));
        }
        if let Some(items) = Self::completion_for_import_path(line, col) {
            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(Some(CompletionResponse::Array(Self::completion_items())))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let text_doc = params.text_document_position_params.text_document;
        let position = params.text_document_position_params.position;

        let docs = self.docs.read().await;
        let Some(text) = docs.get(&text_doc.uri) else {
            return Ok(None);
        };

        let lines: Vec<&str> = text.lines().collect();
        let Some(line) = lines.get(position.line as usize) else {
            return Ok(None);
        };
        let col = position.character as usize;
        if col > line.len() {
            return Ok(None);
        }

        let bytes = line.as_bytes();
        let mut start = col;
        while start > 0 {
            let c = bytes[start - 1] as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                start -= 1;
            } else {
                break;
            }
        }
        let mut end = col;
        while end < bytes.len() {
            let c = bytes[end] as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                end += 1;
            } else {
                break;
            }
        }
        if start >= end {
            return Ok(None);
        }

        let word = &line[start..end];
        let Some(doc) = Self::hover_doc(word) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(doc.to_string())),
            range: Some(Range {
                start: Position::new(position.line, start as u32),
                end: Position::new(position.line, end as u32),
            }),
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
