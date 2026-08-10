// ── mlog-lsp: LSP server for METALOGOS (.mlog) files ──────────────────
//
// Features:
//   - diagnostics (semantic analysis errors/warnings in real-time)
//   - go-to-definition (entity, pattern, flow declarations)
//   - hover (type + confidence info)
//   - textDocument/didOpen, textDocument/didChange support

use dashmap::DashMap;
use metalogos::ast::{Declaration, Span};
use metalogos::parser;
use metalogos::semantic::AnalysisResult;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// Document state: parsed declarations + diagnostics cache.
#[derive(Debug, Clone)]
struct DocumentState {
    #[allow(dead_code)]
    uri: Url,
    text: String,
    declarations: Vec<Declaration>,
}

/// Symbol entry: name → (definition span, declaration index).
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub span: Span,
    pub decl_index: usize,
}

/// The LSP backend state.
pub struct Backend {
    client: Client,
    documents: Arc<DashMap<String, DocumentState>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DashMap::new()),
        }
    }

    /// Parse source, run semantic analysis, publish diagnostics.
    async fn analyze_and_publish(&self, uri: &Url, text: &str, version: i32) {
        let (declarations, diagnostics) = Self::parse_and_analyze(text);

        // Store declarations
        let key = uri.as_str().to_string();
        let state = DocumentState {
            uri: uri.clone(),
            text: text.to_string(),
            declarations,
        };
        self.documents.insert(key, state);

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;
    }

    /// Parse source and run semantic analysis, returning (declarations, LSP diagnostics).
    /// Public for integration tests.
    pub fn parse_and_analyze(text: &str) -> (Vec<Declaration>, Vec<Diagnostic>) {
        match parser::parse(text) {
            Ok(declarations) => {
                let analysis = metalogos::semantic::check_program(&declarations);
                let lsp_diagnostics = to_lsp_diagnostics(&analysis);
                (declarations, lsp_diagnostics)
            }
            Err(e) => {
                let lsp_diags = vec![Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 100)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("mlog".to_string()),
                    message: format!("parse error: {}", e),
                    related_information: None,
                    tags: None,
                    data: None,
                }];
                (Vec::new(), lsp_diags)
            }
        }
    }

    /// Build symbol table from declarations.
    /// Positions are resolved via text-based search (Variant B).
    /// Public for integration tests.
    pub fn build_symbols(declarations: &[Declaration]) -> Vec<SymbolEntry> {
        Self::build_symbols_with_text(declarations, "")
    }

    /// Build symbol table with text-based position resolution.
    /// When `source_text` is non-empty, searches for each declaration's
    /// keyword + name pattern to determine accurate positions.
    pub fn build_symbols_with_text(
        declarations: &[Declaration],
        source_text: &str,
    ) -> Vec<SymbolEntry> {
        let mut symbols = Vec::new();
        for (i, decl) in declarations.iter().enumerate() {
            if let Some(name) = decl.name() {
                let span = if source_text.is_empty() {
                    Span::unknown()
                } else {
                    find_declaration_span(source_text, decl.kind_str(), name)
                };
                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    span,
                    decl_index: i,
                });
            }
        }
        symbols
    }

    /// Convert a URL to a DashMap key.
    fn uri_key(uri: &Url) -> String {
        uri.as_str().to_string()
    }
}

/// Convert semantic AnalysisResult to LSP Diagnostic vec.
fn to_lsp_diagnostics(analysis: &AnalysisResult) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for err in &analysis.errors {
        diagnostics.push(Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 100)),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("mlog".to_string()),
            message: err.clone(),
            related_information: None,
            tags: None,
            data: None,
        });
    }
    for warn in &analysis.warnings {
        diagnostics.push(Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 100)),
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("mlog".to_string()),
            message: warn.clone(),
            related_information: None,
            tags: None,
            data: None,
        });
    }
    diagnostics
}

/// Convert an AST Span to an LSP Range.
/// Public for integration tests.
pub fn span_to_range(span: &Span) -> Range {
    Range::new(
        Position::new(span.start_line, span.start_col),
        Position::new(span.end_line, span.end_col),
    )
}

/// Find identifier at a given position by scanning the source text.
fn find_word_at_position(text: &str, position: &Position) -> Option<String> {
    let line_index = position.line as usize;
    let col_index = position.character as usize;

    let lines: Vec<&str> = text.lines().collect();
    if line_index >= lines.len() {
        return None;
    }

    let line = lines[line_index];

    let mut start = col_index;
    while start > 0 && is_identifier_char(line.chars().nth(start - 1)?) {
        start -= 1;
    }
    let mut end = col_index;
    while end < line.len() && is_identifier_char(line.chars().nth(end)?) {
        end += 1;
    }

    if start == end {
        None
    } else {
        Some(line[start..end].to_string())
    }
}

/// Check if a character can be part of an identifier.
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\''
}

/// Keyword prefix used for text-based declaration search.
/// Maps kind_str() values to the keyword that appears in source text.
fn declaration_keyword(kind: &str) -> &str {
    match kind {
        "mlogserver" => "mlogserver",
        "template" => "template",
        "db" => "db",
        "schema" => "schema",
        "skill_index" => "skill_index",
        "memory" => "memory",
        "import" => "import",
        "entity_type" => "entity",
        "entity_record" => "entity",
        "entity" => "entity",
        "rule" => "rule",
        "memorize" => "memorize",
        "forget" => "forget",
        "fluid" => "fluid",
        "adapt" => "adapt",
        "relate" => "relate",
        "sandbox" => "sandbox",
        "hook" => "hook",
        "mutate" => "mutate",
        "eval" => "eval",
        "pattern" => "pattern",
        "learnable_pattern" => "learnable",
        "flow" => "flow",
        "conversation" => "conversation",
        "tool" => "tool",
        "llm_config" => "llm",
        "context_budget" => "context_budget",
        other => other,
    }
}

/// Find the source span of a declaration by text-based search (Variant B).
/// Searches for `keyword <name>` pattern in the source text, line by line.
/// Returns the first match where the keyword is at the start of a line (after whitespace).
fn find_declaration_span(source_text: &str, kind: &str, name: &str) -> Span {
    let keyword = declaration_keyword(kind);
    // For keyword + name declarations, search for "keyword name" pattern
    let pattern = format!("{} {}", keyword, name);

    for (line_idx, line) in source_text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&pattern) || trimmed.starts_with(&format!("{}{{", keyword)) {
            let col = line.len() - trimmed.len();
            let name_start = if trimmed.starts_with(&format!("{}{{", keyword)) {
                // e.g. "db { ... }" — no name, span the keyword
                0
            } else {
                // keyword + space = offset to name
                keyword.len() + 1
            };
            let name_end = name_start + name.len();
            return Span::new(
                line_idx as u32,
                (col + name_start) as u32,
                line_idx as u32,
                (col + name_end) as u32,
            );
        }
    }

    // Fallback: search for just the name on its own (e.g. for entity record "entity name: Type")
    // This handles cases where keyword and name have different syntax patterns.
    for (line_idx, line) in source_text.lines().enumerate() {
        if let Some(col) = line.find(name) {
            // Verify it's a whole-word match
            let before_ok =
                col == 0 || !is_identifier_char(line.chars().nth(col - 1).unwrap_or(' '));
            let after_end = col + name.len();
            let after_ok = after_end >= line.len()
                || !is_identifier_char(line.chars().nth(after_end).unwrap_or(' '));
            if before_ok && after_ok {
                return Span::new(
                    line_idx as u32,
                    col as u32,
                    line_idx as u32,
                    after_end as u32,
                );
            }
        }
    }

    Span::unknown()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "mlog-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "mlog-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        self.analyze_and_publish(&uri, &text, version).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // TextDocumentSyncKind::FULL means we get the full text
        let text = params
            .content_changes
            .drain(..)
            .next()
            .map(|c| c.text)
            .unwrap_or_default();

        self.analyze_and_publish(&uri, &text, version).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let key = Self::uri_key(&uri);
        self.documents.remove(&key);

        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let key = Self::uri_key(&uri);
        let doc = match self.documents.get(&key) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let word = match find_word_at_position(&doc.text, &position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let symbols = Self::build_symbols_with_text(&doc.declarations, &doc.text);
        for symbol in &symbols {
            if symbol.name == word || symbol.name.ends_with(&format!("/{}", word)) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range: span_to_range(&symbol.span),
                })));
            }
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let key = Self::uri_key(&uri);
        let doc = match self.documents.get(&key) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let word = match find_word_at_position(&doc.text, &position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let symbols = Self::build_symbols_with_text(&doc.declarations, &doc.text);
        for symbol in &symbols {
            if symbol.name == word {
                let decl = &doc.declarations[symbol.decl_index];
                let type_info = decl.type_info();
                let kind_str = decl.kind_str();
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```mlog\n{}\n```\n\n**{}**", type_info, kind_str),
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;

        let key = Self::uri_key(&uri);
        let doc = match self.documents.get(&key) {
            Some(d) => d.clone(),
            None => return Ok(None),
        };

        let symbols = Self::build_symbols_with_text(&doc.declarations, &doc.text);
        let items: Vec<CompletionItem> = symbols
            .iter()
            .map(|s| CompletionItem {
                label: s.name.clone(),
                kind: Some(symbol_to_completion_kind(&doc.declarations[s.decl_index])),
                detail: None,
                documentation: None,
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
        })))
    }
}

/// Map declaration type to CompletionItemKind.
fn symbol_to_completion_kind(decl: &Declaration) -> CompletionItemKind {
    match decl {
        Declaration::EntityType(_) => CompletionItemKind::STRUCT,
        Declaration::Pattern(_) => CompletionItemKind::FUNCTION,
        Declaration::LearnablePattern(_) => CompletionItemKind::FUNCTION,
        Declaration::Flow(_) => CompletionItemKind::CLASS,
        Declaration::EntityRecord(_) | Declaration::EntitySimple(_) => CompletionItemKind::VARIABLE,
        Declaration::Fluid(_) => CompletionItemKind::VARIABLE,
        Declaration::Import(_) => CompletionItemKind::MODULE,
        Declaration::Sandbox(_) => CompletionItemKind::CLASS,
        _ => CompletionItemKind::TEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_word_at_position() {
        let text = "entity greeting: String = \"Hello\"";
        let pos = Position::new(0, 7);
        let word = find_word_at_position(text, &pos);
        assert_eq!(word, Some("greeting".to_string()));

        let pos = Position::new(0, 2);
        let word = find_word_at_position(text, &pos);
        assert_eq!(word, Some("entity".to_string()));
    }

    #[test]
    fn test_parse_and_analyze_errors() {
        let source = "entity m: UnknownType = { text: \"hi\" }";
        let (declarations, diagnostics) = Backend::parse_and_analyze(source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert!(diagnostics[0].message.contains("unknown type"));
        assert!(!declarations.is_empty());
    }

    #[test]
    fn test_parse_and_analyze_clean() {
        let source = r#"
            entity greeting: String = "Hello, Metalogos!"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let (declarations, diagnostics) = Backend::parse_and_analyze(source);
        assert!(
            diagnostics.is_empty(),
            "clean program should have no diagnostics"
        );
        assert_eq!(declarations.len(), 3);
    }

    #[test]
    fn test_span_to_range() {
        let span = Span::new(2, 5, 2, 15);
        let range = span_to_range(&span);
        assert_eq!(range.start, Position::new(2, 5));
        assert_eq!(range.end, Position::new(2, 15));
    }

    #[test]
    fn test_build_symbols() {
        let source = r#"
            entity greeting: String = "Hello"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let (declarations, _) = Backend::parse_and_analyze(source);
        let symbols = Backend::build_symbols(&declarations);
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "greeting");
        assert_eq!(symbols[1].name, "SayHello");
        assert_eq!(symbols[2].name, "Main");
    }

    #[test]
    fn test_find_word_at_position_multi_line() {
        let text =
            "entity greeting: String = \"Hello\"\npattern Foo(x: String) -> String { return x }";
        // Line 1, on "Foo"
        let pos = Position::new(1, 8);
        let word = find_word_at_position(text, &pos);
        assert_eq!(word, Some("Foo".to_string()));
    }

    #[test]
    fn test_find_word_out_of_bounds() {
        let text = "entity greeting: String = \"Hello\"";
        let pos = Position::new(5, 0);
        let word = find_word_at_position(text, &pos);
        assert!(word.is_none());
    }

    #[test]
    fn test_go_to_definition_logic() {
        let source = r#"
            entity greeting: String = "Hello"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let (declarations, _) = Backend::parse_and_analyze(source);
        let symbols = Backend::build_symbols(&declarations);

        // Find "greeting" → should match the entity declaration
        let found = symbols.iter().find(|s| s.name == "greeting");
        assert!(found.is_some());
        assert_eq!(found.unwrap().decl_index, 0);

        // Find "SayHello" → should match the pattern declaration
        let found = symbols.iter().find(|s| s.name == "SayHello");
        assert!(found.is_some());
        assert_eq!(found.unwrap().decl_index, 1);
    }

    #[test]
    fn test_diagnostics_for_parse_error() {
        let source = "entity { this is not valid syntax";
        let (declarations, diagnostics) = Backend::parse_and_analyze(source);
        assert!(declarations.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("parse error"));
    }

    #[test]
    fn test_diagnostics_for_duplicate() {
        let source = r#"
            pattern Foo(x: String) -> String { return x }
            pattern Foo(y: String) -> String { return y }
        "#;
        let (_declarations, diagnostics) = Backend::parse_and_analyze(source);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate pattern")));
    }
}
