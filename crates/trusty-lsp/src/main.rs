use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

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
            Err(err) => vec![Diagnostic {
                range: Range {
                    start: Position::new(0, 0),
                    end: Position::new(0, 1),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("trusty-compiler".to_string()),
                message: err.to_string(),
                related_information: None,
                tags: None,
                data: None,
            }],
        };

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    fn completion_items() -> Vec<CompletionItem> {
        let keywords = [
            "function", "struct", "enum", "implements", "import", "export", "from", "val", "var", "const",
            "if", "else", "match", "default", "try", "catch", "finally", "for", "in", "of", "loop",
            "break", "continue", "return", "throw", "and", "or",
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

    fn hover_doc(word: &str) -> Option<&'static str> {
        match word {
            "val" => Some("`val`: immutable local variable."),
            "var" => Some("`var`: mutable local variable."),
            "const" => Some("`const`: global constant."),
            "match" => Some("`match (x) { pat => expr, default => expr }`: expression match."),
            "loop" => Some("`loop (cond) { ... }`: conditional loop."),
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
                completion_provider: Some(CompletionOptions::default()),
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

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
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
