use crate::ast::{ConstDef, Expr, Function, Stmt, StructDef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

thread_local! {
    pub static VFS: RefCell<Option<HashMap<PathBuf, String>>> = RefCell::new(None);
}

pub fn get_vfs_file(path: &Path) -> Option<String> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    VFS.with(|vfs| {
        vfs.borrow()
            .as_ref()
            .and_then(|map| map.get(&canonical).cloned())
    })
}

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl Backend {
    fn log(&self, msg: String) {
        let client = self.client.clone();
        tokio::spawn(async move {
            client.log_message(MessageType::LOG, msg).await;
        });
    }

    async fn validate_document(&self, uri: Url) {
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Populate the VFS
        let docs = self.documents.read().unwrap().clone();
        let mut vfs_map = HashMap::new();
        for (u, content) in docs {
            if let Ok(p) = u.to_file_path() {
                let canon = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                vfs_map.insert(canon, content);
            }
        }

        // Run the compiler diagnostics pipeline on a blocking thread so the
        // tokio async runtime is not starved. The transport layer needs the
        // runtime to flush framed messages through stdout, and a synchronous
        // blocking call would prevent that.
        let path_clone = path.clone();
        let (had_panic, compiler_diags) = tokio::task::spawn_blocking(move || {
            VFS.with(|vfs| {
                *vfs.borrow_mut() = Some(vfs_map);
            });

            let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::compile_only_for_diagnostics(&path_clone);
                crate::parse_for_lsp(&path_clone);
            }));

            if let Err(payload) = &panic_result {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown parser error".to_string()
                };
                crate::diagnostics::report_error(msg, None);
            }

            let had_panic = panic_result.is_err();
            let diags = crate::diagnostics::DIAGNOSTICS.with(|d| d.borrow().clone());

            crate::diagnostics::clear_diagnostics();
            VFS.with(|vfs| {
                *vfs.borrow_mut() = None;
            });

            (had_panic, diags)
        })
        .await
        .unwrap_or((false, Vec::new()));

        // Log if parsing panicked
        if had_panic {
            self.log("Parser encountered an error (see diagnostics for details)".to_string());
        }

        // Map compiler diagnostics to LSP diagnostics
        let mut lsp_diags = Vec::new();
        for diag in compiler_diags {
            let is_same_file = match (&diag.file_path, path.to_str()) {
                (Some(df), Some(pf)) => {
                    let df_canon = std::fs::canonicalize(df).unwrap_or_else(|_| PathBuf::from(df));
                    let pf_canon = std::fs::canonicalize(pf).unwrap_or_else(|_| PathBuf::from(pf));
                    df_canon == pf_canon
                }
                _ => false,
            };

            if is_same_file {
                let range = if let Some(span) = diag.span {
                    let docs_lock = self.documents.read().unwrap();
                    let content = docs_lock.get(&uri).cloned().unwrap_or_default();
                    let converter = PositionConverter::new(&content);
                    Range::new(
                        converter.offset_to_lsp(span.start),
                        converter.offset_to_lsp(span.end),
                    )
                } else {
                    Range::new(Position::new(0, 0), Position::new(0, 0))
                };

                lsp_diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("fox".to_string()),
                    message: diag.message,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        // Publish to client
        self.client.publish_diagnostics(uri, lsp_diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.log("Fox language server initialized!".to_string());
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents.write().unwrap().insert(uri.clone(), text);
        self.validate_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents
                .write()
                .unwrap()
                .insert(uri.clone(), change.text);
        }
        self.validate_document(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.write().unwrap().remove(&uri);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        self.log(format!(
            "goto_definition: {}:{}:{}",
            uri,
            pos.line + 1,
            pos.character + 1
        ));

        let path = match uri.to_file_path() {
            Ok(p) => std::fs::canonicalize(&p).unwrap_or(p),
            Err(_) => {
                self.log("goto_definition: failed to convert URI to path".to_string());
                return Ok(None);
            }
        };

        let content = {
            let docs = self.documents.read().unwrap();
            docs.get(&uri).cloned()
        };
        let content = match content {
            Some(c) => c,
            None => {
                self.log("goto_definition: document not found".to_string());
                return Ok(None);
            }
        };

        let converter = PositionConverter::new(&content);
        let offset = match converter.lsp_to_offset(&pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let location = {
            let cache = crate::get_ast_cache().read().unwrap();
            let mut result = None;
            let mut matched_func: Option<&Function> = None;

            'outer: for func in &cache.funcs {
                if let (Some(span), Some(file)) =
                    (crate::ast::get_span(func), crate::ast::get_file(func))
                {
                    let file_matches = if let Ok(c_file) = std::fs::canonicalize(&file) {
                        c_file == path
                    } else {
                        file == path.to_string_lossy()
                    };
                    if file_matches && span.start <= offset && offset <= span.end {
                        matched_func = Some(func);
                        let file_str = path.to_string_lossy();
                        for stmt in &func.body {
                            if let Some(expr) = walk_stmt(stmt, offset, &path, &file_str) {
                                let loc = match expr {
                                    Expr::Identifier(ref name) => {
                                        if let Some(local_span) =
                                            find_local_decl(func, name, offset)
                                        {
                                            Some(Location::new(
                                                uri.clone(),
                                                Range::new(
                                                    converter.offset_to_lsp(local_span.start),
                                                    converter.offset_to_lsp(local_span.end),
                                                ),
                                            ))
                                        } else if let Some((const_span, const_file)) =
                                            find_global_const(&cache.consts, name)
                                        {
                                            Url::from_file_path(&const_file).ok().and_then(
                                                |target_url| {
                                                    let target_content =
                                                        std::fs::read_to_string(&const_file)
                                                            .unwrap_or_default();
                                                    let target_conv =
                                                        PositionConverter::new(&target_content);
                                                    Some(Location::new(
                                                        target_url,
                                                        Range::new(
                                                            target_conv
                                                                .offset_to_lsp(const_span.start),
                                                            target_conv
                                                                .offset_to_lsp(const_span.end),
                                                        ),
                                                    ))
                                                },
                                            )
                                        } else if let Some((struct_span, struct_file)) =
                                            find_struct(&cache.structs, name)
                                        {
                                            Url::from_file_path(&struct_file).ok().and_then(
                                                |target_url| {
                                                    let target_content =
                                                        std::fs::read_to_string(&struct_file)
                                                            .unwrap_or_default();
                                                    let target_conv =
                                                        PositionConverter::new(&target_content);
                                                    Some(Location::new(
                                                        target_url,
                                                        Range::new(
                                                            target_conv
                                                                .offset_to_lsp(struct_span.start),
                                                            target_conv
                                                                .offset_to_lsp(struct_span.end),
                                                        ),
                                                    ))
                                                },
                                            )
                                        } else {
                                            None
                                        }
                                    }
                                    Expr::Call(ref name, _) => find_function(&cache.funcs, name)
                                        .and_then(|(fn_span, fn_file)| {
                                            Url::from_file_path(&fn_file).ok().map(|target_url| {
                                                let target_content =
                                                    std::fs::read_to_string(&fn_file)
                                                        .unwrap_or_default();
                                                let target_conv =
                                                    PositionConverter::new(&target_content);
                                                Location::new(
                                                    target_url,
                                                    Range::new(
                                                        target_conv.offset_to_lsp(fn_span.start),
                                                        target_conv.offset_to_lsp(fn_span.end),
                                                    ),
                                                )
                                            })
                                        }),
                                    Expr::MethodCall(_, ref method_name, _) => find_function(
                                        &cache.funcs,
                                        method_name,
                                    )
                                    .and_then(|(fn_span, fn_file)| {
                                        Url::from_file_path(&fn_file).ok().map(|target_url| {
                                            let target_content = std::fs::read_to_string(&fn_file)
                                                .unwrap_or_default();
                                            let target_conv =
                                                PositionConverter::new(&target_content);
                                            Location::new(
                                                target_url,
                                                Range::new(
                                                    target_conv.offset_to_lsp(fn_span.start),
                                                    target_conv.offset_to_lsp(fn_span.end),
                                                ),
                                            )
                                        })
                                    }),
                                    Expr::StructInit(ref name, _) => find_struct(
                                        &cache.structs,
                                        name,
                                    )
                                    .and_then(|(struct_span, struct_file)| {
                                        Url::from_file_path(&struct_file).ok().map(|target_url| {
                                            let target_content =
                                                std::fs::read_to_string(&struct_file)
                                                    .unwrap_or_default();
                                            let target_conv =
                                                PositionConverter::new(&target_content);
                                            Location::new(
                                                target_url,
                                                Range::new(
                                                    target_conv.offset_to_lsp(struct_span.start),
                                                    target_conv.offset_to_lsp(struct_span.end),
                                                ),
                                            )
                                        })
                                    }),
                                    _ => None,
                                };
                                if let Some(loc) = loc {
                                    result = Some(loc);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }

            if result.is_none() {
                if let Some((name, _start, _end)) = symbol_at_offset(&content, offset) {
                    if let Some(func) = matched_func {
                        if let Some(local_span) = find_local_decl(func, &name, offset) {
                            result = Some(Location::new(
                                uri.clone(),
                                Range::new(
                                    converter.offset_to_lsp(local_span.start),
                                    converter.offset_to_lsp(local_span.end),
                                ),
                            ));
                        }
                    }
                    if result.is_none() {
                        result = find_global_const(&cache.consts, &name).and_then(
                            |(const_span, const_file)| {
                                Url::from_file_path(&const_file).ok().map(|target_url| {
                                    let target_content =
                                        std::fs::read_to_string(&const_file).unwrap_or_default();
                                    let target_conv = PositionConverter::new(&target_content);
                                    Location::new(
                                        target_url,
                                        Range::new(
                                            target_conv.offset_to_lsp(const_span.start),
                                            target_conv.offset_to_lsp(const_span.end),
                                        ),
                                    )
                                })
                            },
                        );
                    }
                    if result.is_none() {
                        result = find_struct(&cache.structs, &name).and_then(
                            |(struct_span, struct_file)| {
                                Url::from_file_path(&struct_file).ok().map(|target_url| {
                                    let target_content =
                                        std::fs::read_to_string(&struct_file).unwrap_or_default();
                                    let target_conv = PositionConverter::new(&target_content);
                                    Location::new(
                                        target_url,
                                        Range::new(
                                            target_conv.offset_to_lsp(struct_span.start),
                                            target_conv.offset_to_lsp(struct_span.end),
                                        ),
                                    )
                                })
                            },
                        );
                    }
                    if result.is_none() {
                        result =
                            find_function(&cache.funcs, &name).and_then(|(fn_span, fn_file)| {
                                Url::from_file_path(&fn_file).ok().map(|target_url| {
                                    let target_content =
                                        std::fs::read_to_string(&fn_file).unwrap_or_default();
                                    let target_conv = PositionConverter::new(&target_content);
                                    Location::new(
                                        target_url,
                                        Range::new(
                                            target_conv.offset_to_lsp(fn_span.start),
                                            target_conv.offset_to_lsp(fn_span.end),
                                        ),
                                    )
                                })
                            });
                    }
                }
            }

            result
        };

        if let Some(loc) = location {
            return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
        }

        self.log("goto_definition: no definition found".to_string());
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        self.log(format!(
            "hover: {}:{}:{}",
            uri,
            pos.line + 1,
            pos.character + 1
        ));

        let path = match uri.to_file_path() {
            Ok(p) => std::fs::canonicalize(&p).unwrap_or(p),
            Err(_) => return Ok(None),
        };

        let content = {
            let docs = self.documents.read().unwrap();
            docs.get(&uri).cloned()
        };
        let content = match content {
            Some(c) => c,
            None => return Ok(None),
        };

        let converter = PositionConverter::new(&content);
        let offset = match converter.lsp_to_offset(&pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let cache = crate::get_ast_cache().read().unwrap();

        let funcs_map: HashMap<_, _> = cache
            .funcs
            .iter()
            .map(|f| (f.name.clone(), f.clone()))
            .collect();
        let structs_map: HashMap<_, _> = cache
            .structs
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect();

        let mut matched_func: Option<&Function> = None;
        for func in &cache.funcs {
            if let (Some(span), Some(file)) =
                (crate::ast::get_span(func), crate::ast::get_file(func))
            {
                let file_matches = if let Ok(c_file) = std::fs::canonicalize(&file) {
                    c_file == path
                } else {
                    file == path.to_string_lossy()
                };
                if file_matches && span.start <= offset && offset <= span.end {
                    matched_func = Some(func);
                    let file_str = path.to_string_lossy();
                    for stmt in &func.body {
                        if let Some(expr) = walk_stmt(stmt, offset, &path, &file_str) {
                            let mut hover_text = String::new();
                            match expr {
                                Expr::Identifier(ref name) => {
                                    let sym =
                                        get_local_symbols(func, offset, &funcs_map, &structs_map);
                                    if let Some(ty_str) = sym.get(name) {
                                        hover_text =
                                            format!("```fox\nlet {}: {}\n```", name, ty_str);
                                    } else if let Some(c) = cache.consts.iter().find(|c| {
                                        c.name == *name || c.name.ends_with(&format!("::{}", name))
                                    }) {
                                        hover_text =
                                            format!("```fox\nconst {}: {}\n```", c.name, c.ty);
                                    } else if let Some(s) = cache.structs.iter().find(|s| {
                                        s.name == *name || s.name.ends_with(&format!("::{}", name))
                                    }) {
                                        let fields_str = s
                                            .fields
                                            .iter()
                                            .map(|f| format!("    {}: {}", f.name, f.ty))
                                            .collect::<Vec<_>>()
                                            .join(",\n");
                                        hover_text = format!(
                                            "```fox\nstruct {} {{\n{}\n}}\n```",
                                            s.name, fields_str
                                        );
                                    }
                                }
                                Expr::Call(ref name, _) => {
                                    if let Some(f) = cache.funcs.iter().find(|f| {
                                        f.name == *name || f.name.ends_with(&format!("::{}", name))
                                    }) {
                                        let params_str = f
                                            .params
                                            .iter()
                                            .map(|p| format!("{}: {}", p.name, p.ty))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        hover_text = format!(
                                            "```fox\nfn {}({}) -> {}\n```",
                                            f.name, params_str, f.return_ty
                                        );
                                    }
                                }
                                Expr::MethodCall(_, ref method_name, _) => {
                                    if let Some(f) = cache.funcs.iter().find(|f| {
                                        f.name == *method_name
                                            || f.name.ends_with(&format!("::{}", method_name))
                                    }) {
                                        let params_str = f
                                            .params
                                            .iter()
                                            .map(|p| format!("{}: {}", p.name, p.ty))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        hover_text = format!(
                                            "```fox\nfn {}({}) -> {}\n```",
                                            f.name, params_str, f.return_ty
                                        );
                                    }
                                }
                                Expr::StructInit(ref name, _) => {
                                    if let Some(s) = cache.structs.iter().find(|s| {
                                        s.name == *name || s.name.ends_with(&format!("::{}", name))
                                    }) {
                                        let fields_str = s
                                            .fields
                                            .iter()
                                            .map(|f| format!("    {}: {}", f.name, f.ty))
                                            .collect::<Vec<_>>()
                                            .join(",\n");
                                        hover_text = format!(
                                            "```fox\nstruct {} {{\n{}\n}}\n```",
                                            s.name, fields_str
                                        );
                                    }
                                }
                                _ => {
                                    let sym =
                                        get_local_symbols(func, offset, &funcs_map, &structs_map);
                                    let ty_str = crate::type_checker::get_expr_type(
                                        &expr,
                                        &sym,
                                        &funcs_map,
                                        &structs_map,
                                    );
                                    hover_text = format!("```fox\ntype: {}\n```", ty_str);
                                }
                            }

                            if !hover_text.is_empty() {
                                return Ok(Some(Hover {
                                    contents: HoverContents::Scalar(MarkedString::String(
                                        hover_text,
                                    )),
                                    range: None,
                                }));
                            }
                        }
                    }
                }
            }
        }

        // Token-based fallback for symbols the AST walker missed (e.g. method calls
        // without expression spans or imported functions).
        if let Some((name, _start, _end)) = symbol_at_offset(&content, offset) {
            // Don't treat string literal contents as symbols - e.g. hovering on
            // "class" in set_attribute("class", ...) should not resolve to a
            // struct method named `class`.
            if is_offset_in_string_literal(&content, offset) {
                return Ok(None);
            }

            let mut hover_text = String::new();
            let enclosing_func = matched_func.or_else(|| {
                cache.funcs.iter().find(|f| {
                    let (Some(span), Some(file)) =
                        (crate::ast::get_span(f), crate::ast::get_file(f))
                    else {
                        return false;
                    };
                    let file_matches = if let Ok(c_file) = std::fs::canonicalize(&file) {
                        c_file == path
                    } else {
                        file == path.to_string_lossy()
                    };
                    file_matches && span.start <= offset && offset <= span.end
                })
            });
            if let Some(func) = enclosing_func {
                if find_local_decl(func, &name, offset).is_some() {
                    let sym = get_local_symbols(func, offset, &funcs_map, &structs_map);
                    if let Some(ty_str) = sym.get(&name) {
                        hover_text = format!("```fox\nlet {}: {}\n```", name, ty_str);
                    } else {
                        hover_text = format!("```fox\nlet {}\n```", name);
                    }
                }
            }
            if hover_text.is_empty() {
                if let Some(c) = cache
                    .consts
                    .iter()
                    .find(|c| c.name == name || c.name.ends_with(&format!("::{}", name)))
                {
                    hover_text = format!("```fox\nconst {}: {}\n```", c.name, c.ty);
                }
            }
            if hover_text.is_empty() {
                if let Some(s) = cache
                    .structs
                    .iter()
                    .find(|s| s.name == name || s.name.ends_with(&format!("::{}", name)))
                {
                    let fields_str = s
                        .fields
                        .iter()
                        .map(|f| format!("    {}: {}", f.name, f.ty))
                        .collect::<Vec<_>>()
                        .join(",\n");
                    hover_text = format!("```fox\nstruct {} {{\n{}\n}}\n```", s.name, fields_str);
                }
            }
            if hover_text.is_empty() {
                if let Some(f) = cache
                    .funcs
                    .iter()
                    .find(|f| f.name == name || f.name.ends_with(&format!("::{}", name)))
                {
                    let params_str = f
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.ty))
                        .collect::<Vec<_>>()
                        .join(", ");
                    hover_text = format!(
                        "```fox\nfn {}({}) -> {}\n```",
                        f.name, params_str, f.return_ty
                    );
                }
            }
            if !hover_text.is_empty() {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(hover_text)),
                    range: None,
                }));
            }
        }

        self.log("hover: no result found".to_string());
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let path = match uri.to_file_path() {
            Ok(p) => std::fs::canonicalize(&p).unwrap_or(p),
            Err(_) => return Ok(None),
        };

        let docs = self.documents.read().unwrap();
        let content = match docs.get(&uri) {
            Some(c) => c,
            None => return Ok(None),
        };

        let converter = PositionConverter::new(content);

        let cache = crate::get_ast_cache().read().unwrap();

        let mut symbols = Vec::new();

        // 1. Process Structs
        for s in &cache.structs {
            if let (Some(span), Some(file)) = (crate::ast::get_span(s), crate::ast::get_file(s)) {
                if let Ok(c_file) = std::fs::canonicalize(&file) {
                    if c_file == path {
                        let range = Range::new(
                            converter.offset_to_lsp(span.start),
                            converter.offset_to_lsp(span.end),
                        );
                        let selection_range = range;

                        #[allow(deprecated)]
                        let doc_symbol = DocumentSymbol {
                            name: s.name.clone(),
                            detail: None,
                            kind: SymbolKind::STRUCT,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range,
                            children: None,
                        };
                        symbols.push(doc_symbol);
                    }
                }
            }
        }

        // 2. Process Functions
        for f in &cache.funcs {
            if let (Some(span), Some(file)) = (crate::ast::get_span(f), crate::ast::get_file(f)) {
                if let Ok(c_file) = std::fs::canonicalize(&file) {
                    if c_file == path {
                        let range = Range::new(
                            converter.offset_to_lsp(span.start),
                            converter.offset_to_lsp(span.end),
                        );
                        let selection_range = range;

                        #[allow(deprecated)]
                        let doc_symbol = DocumentSymbol {
                            name: f.name.clone(),
                            detail: None,
                            kind: SymbolKind::FUNCTION,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range,
                            children: None,
                        };
                        symbols.push(doc_symbol);
                    }
                }
            }
        }

        // 3. Process Constants
        for c in &cache.consts {
            if let (Some(span), Some(file)) = (crate::ast::get_span(c), crate::ast::get_file(c)) {
                if let Ok(c_file) = std::fs::canonicalize(&file) {
                    if c_file == path {
                        let range = Range::new(
                            converter.offset_to_lsp(span.start),
                            converter.offset_to_lsp(span.end),
                        );
                        let selection_range = range;

                        #[allow(deprecated)]
                        let doc_symbol = DocumentSymbol {
                            name: c.name.clone(),
                            detail: Some(c.ty.to_string()),
                            kind: SymbolKind::CONSTANT,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range,
                            children: None,
                        };
                        symbols.push(doc_symbol);
                    }
                }
            }
        }

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        }
    }
}

pub fn run_lsp() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

pub struct PositionConverter {
    line_offsets: Vec<usize>,
    source: String,
}

impl PositionConverter {
    pub fn new(source: &str) -> Self {
        let mut line_offsets = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_offsets.push(i + 1);
            }
        }
        Self {
            line_offsets,
            source: source.to_string(),
        }
    }

    pub fn lsp_to_offset(&self, pos: &Position) -> Option<usize> {
        let line = pos.line as usize;
        if line >= self.line_offsets.len() {
            return None;
        }
        let line_start_offset = self.line_offsets[line];
        let line_end_offset = if line + 1 < self.line_offsets.len() {
            self.line_offsets[line + 1]
        } else {
            self.source.len()
        };

        let line_str = &self.source[line_start_offset..line_end_offset];
        let mut utf16_count = 0;
        let mut byte_offset = 0;

        for c in line_str.chars() {
            if utf16_count >= pos.character as usize {
                break;
            }
            utf16_count += c.len_utf16();
            byte_offset += c.len_utf8();
        }

        Some(line_start_offset + byte_offset)
    }

    pub fn offset_to_lsp(&self, offset: usize) -> Position {
        let mut line = 0;
        for (i, &line_start) in self.line_offsets.iter().enumerate() {
            if offset >= line_start {
                line = i;
            } else {
                break;
            }
        }

        let line_start_offset = self.line_offsets[line];
        let line_str = &self.source[line_start_offset..offset.min(self.source.len())];
        let character = line_str.chars().map(|c| c.len_utf16()).sum::<usize>();

        Position::new(line as u32, character as u32)
    }
}

fn check_span(
    span: Option<crate::ast::Span>,
    offset: usize,
    file_path: &Path,
    node_file: &Option<String>,
) -> bool {
    if let (Some(s), Some(nf)) = (span, node_file.as_ref()) {
        if let Ok(c_file) = std::fs::canonicalize(nf) {
            if let Ok(c_fp) = std::fs::canonicalize(file_path) {
                c_file == c_fp && s.start <= offset && offset <= s.end
            } else {
                nf == &file_path.to_string_lossy() && s.start <= offset && offset <= s.end
            }
        } else {
            nf == &file_path.to_string_lossy() && s.start <= offset && offset <= s.end
        }
    } else {
        false
    }
}

fn check_expr_span(expr: &Expr, offset: usize, file_path: &Path, _file_str: &str) -> bool {
    if let Some(span) = crate::ast::get_span(expr) {
        let expr_file = crate::ast::get_file(expr);
        check_span(Some(span), offset, file_path, &expr_file)
    } else {
        false
    }
}

fn walk_expr(expr: &Expr, offset: usize, file_path: &Path, file_str: &str) -> Option<Expr> {
    if !check_expr_span(expr, offset, file_path, file_str) {
        return None;
    }

    match expr {
        Expr::Binary(l, _, r) => walk_expr(l, offset, file_path, file_str)
            .or_else(|| walk_expr(r, offset, file_path, file_str)),
        Expr::Call(_, args) => {
            for arg in args {
                if let Some(res) = walk_expr(arg, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::MethodCall(obj, _, args) => {
            if let Some(res) = walk_expr(obj, offset, file_path, file_str) {
                return Some(res);
            }
            for arg in args {
                if let Some(res) = walk_expr(arg, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::FieldAccess(obj, _) => walk_expr(obj, offset, file_path, file_str),
        Expr::StructInit(_, fields) => {
            for (_, fexpr) in fields {
                if let Some(res) = walk_expr(fexpr, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::IndexAccess(arr, idx) => walk_expr(arr, offset, file_path, file_str)
            .or_else(|| walk_expr(idx, offset, file_path, file_str)),
        Expr::New(_, args) => {
            for arg in args {
                if let Some(res) = walk_expr(arg, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::Match(cond, arms) => {
            if let Some(res) = walk_expr(cond, offset, file_path, file_str) {
                return Some(res);
            }
            for arm in arms {
                for stmt in &arm.body {
                    if let Some(res) = walk_stmt(stmt, offset, file_path, file_str) {
                        return Some(res);
                    }
                }
                if let Some(val) = &arm.val {
                    if let Some(res) = walk_expr(val, offset, file_path, file_str) {
                        return Some(res);
                    }
                }
            }
            None
        }
        Expr::If(cond, then_b, else_b) => {
            if let Some(res) = walk_expr(cond, offset, file_path, file_str) {
                return Some(res);
            }
            let (then_stmts, then_val) = &**then_b;
            for stmt in then_stmts {
                if let Some(res) = walk_stmt(stmt, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            if let Some(val) = then_val {
                if let Some(res) = walk_expr(val, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            if let Some(eb) = else_b {
                let (else_stmts, else_val) = &**eb;
                for stmt in else_stmts {
                    if let Some(res) = walk_stmt(stmt, offset, file_path, file_str) {
                        return Some(res);
                    }
                }
                if let Some(val) = else_val {
                    if let Some(res) = walk_expr(val, offset, file_path, file_str) {
                        return Some(res);
                    }
                }
            }
            None
        }
        Expr::InvokeFuncPtr(func, args) => {
            if let Some(res) = walk_expr(func, offset, file_path, file_str) {
                return Some(res);
            }
            for arg in args {
                if let Some(res) = walk_expr(arg, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::Closure(func) => {
            for stmt in &func.body {
                if let Some(res) = walk_stmt(stmt, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::ClosureInstantiate(_, _, args) => {
            for arg in args {
                if let Some(res) = walk_expr(arg, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::Cast(e, _) => walk_expr(e, offset, file_path, file_str),
        Expr::Spread(e) => walk_expr(e, offset, file_path, file_str),
        Expr::Tuple(exprs) => {
            for e in exprs {
                if let Some(res) = walk_expr(e, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                if let Some(res) = walk_expr(k, offset, file_path, file_str) {
                    return Some(res);
                }
                if let Some(res) = walk_expr(v, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Expr::VecLit(elems) => {
            for e in elems {
                if let Some(res) = walk_expr(e, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        _ => None,
    }
    .or(Some(expr.clone()))
}

fn walk_stmt(stmt: &Stmt, offset: usize, file_path: &Path, file_str: &str) -> Option<Expr> {
    let stmt_file = crate::ast::get_file(stmt);
    if !check_span(crate::ast::get_span(stmt), offset, file_path, &stmt_file) {
        return None;
    }

    match stmt {
        Stmt::Let(_, _, expr) => walk_expr(expr, offset, file_path, file_str),
        Stmt::LetTuple(_, expr) => walk_expr(expr, offset, file_path, file_str),
        Stmt::ExprStmt(expr) => walk_expr(expr, offset, file_path, file_str),
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                walk_expr(expr, offset, file_path, file_str)
            } else {
                None
            }
        }
        Stmt::Assign(_, expr) => walk_expr(expr, offset, file_path, file_str),
        Stmt::AssignPlus(_, expr) => walk_expr(expr, offset, file_path, file_str),
        Stmt::AssignIndex(arr, idx, val) => walk_expr(arr, offset, file_path, file_str)
            .or_else(|| walk_expr(idx, offset, file_path, file_str))
            .or_else(|| walk_expr(val, offset, file_path, file_str)),
        Stmt::AssignField(obj, _, val) => walk_expr(obj, offset, file_path, file_str)
            .or_else(|| walk_expr(val, offset, file_path, file_str)),
        Stmt::If(cond, then_b, else_b) => {
            if let Some(res) = walk_expr(cond, offset, file_path, file_str) {
                return Some(res);
            }
            for s in then_b {
                if let Some(res) = walk_stmt(s, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            if let Some(eb) = else_b {
                for s in eb {
                    if let Some(res) = walk_stmt(s, offset, file_path, file_str) {
                        return Some(res);
                    }
                }
            }
            None
        }
        Stmt::While(cond, body) => {
            if let Some(res) = walk_expr(cond, offset, file_path, file_str) {
                return Some(res);
            }
            for s in body {
                if let Some(res) = walk_stmt(s, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
        Stmt::For(_, _, body) => {
            for s in body {
                if let Some(res) = walk_stmt(s, offset, file_path, file_str) {
                    return Some(res);
                }
            }
            None
        }
    }
}

fn find_local_decl(func: &Function, name: &str, ref_offset: usize) -> Option<crate::ast::Span> {
    for param in &func.params {
        if param.name == name {
            return crate::ast::get_span(param);
        }
    }

    let mut found_span = None;
    fn check_stmts(
        stmts: &[Stmt],
        name: &str,
        ref_offset: usize,
        found: &mut Option<crate::ast::Span>,
    ) {
        for stmt in stmts {
            if let Some(span) = crate::ast::get_span(stmt) {
                if span.start >= ref_offset {
                    continue;
                }
            }
            match stmt {
                Stmt::Let(var_name, _, _) => {
                    if var_name == name {
                        *found = crate::ast::get_span(stmt);
                    }
                }
                Stmt::LetTuple(vars, _) => {
                    for (var_name, _) in vars {
                        if var_name == name {
                            *found = crate::ast::get_span(stmt);
                        }
                    }
                }
                Stmt::If(_, then_b, else_b) => {
                    check_stmts(then_b, name, ref_offset, found);
                    if let Some(eb) = else_b {
                        check_stmts(eb, name, ref_offset, found);
                    }
                }
                Stmt::While(_, body) => {
                    check_stmts(body, name, ref_offset, found);
                }
                Stmt::For(var_name, _, body) => {
                    if var_name == name {
                        *found = crate::ast::get_span(stmt);
                    }
                    check_stmts(body, name, ref_offset, found);
                }
                _ => {}
            }
        }
    }
    check_stmts(&func.body, name, ref_offset, &mut found_span);
    found_span
}

fn find_global_const(consts: &[ConstDef], name: &str) -> Option<(crate::ast::Span, String)> {
    for c in consts {
        if c.name == name || c.name.ends_with(&format!("::{}", name)) {
            if let (Some(span), Some(file)) = (crate::ast::get_span(c), crate::ast::get_file(c)) {
                return Some((span, file));
            }
        }
    }
    None
}

fn find_function(funcs: &[Function], name: &str) -> Option<(crate::ast::Span, String)> {
    for f in funcs {
        if f.name == name || f.name.ends_with(&format!("::{}", name)) {
            if let (Some(span), Some(file)) = (crate::ast::get_span(f), crate::ast::get_file(f)) {
                return Some((span, file));
            }
        }
    }
    None
}

fn find_struct(structs: &[StructDef], name: &str) -> Option<(crate::ast::Span, String)> {
    for s in structs {
        if s.name == name || s.name.ends_with(&format!("::{}", name)) {
            if let (Some(span), Some(file)) = (crate::ast::get_span(s), crate::ast::get_file(s)) {
                return Some((span, file));
            }
        }
    }
    None
}

fn get_local_symbols(
    func: &Function,
    target_offset: usize,
    funcs_map: &HashMap<String, Function>,
    structs_map: &HashMap<String, StructDef>,
) -> HashMap<String, String> {
    let mut sym = HashMap::new();
    for param in &func.params {
        sym.insert(param.name.clone(), param.ty.to_string());
    }

    fn collect_from_stmts(
        stmts: &[Stmt],
        target_offset: usize,
        sym: &mut HashMap<String, String>,
        funcs_map: &HashMap<String, Function>,
        structs_map: &HashMap<String, StructDef>,
    ) {
        for stmt in stmts {
            if let Some(span) = crate::ast::get_span(stmt) {
                if span.start >= target_offset {
                    continue;
                }
            }
            match stmt {
                Stmt::Let(var_name, opt_ty, init_expr) => {
                    let ty_str = if let Some(ty) = opt_ty {
                        ty.to_string()
                    } else {
                        crate::type_checker::get_expr_type(init_expr, sym, funcs_map, structs_map)
                    };
                    sym.insert(var_name.clone(), ty_str);
                }
                Stmt::LetTuple(vars, _) => {
                    for (var_name, ty) in vars {
                        sym.insert(var_name.clone(), ty.to_string());
                    }
                }
                Stmt::If(_, then_b, else_b) => {
                    collect_from_stmts(then_b, target_offset, sym, funcs_map, structs_map);
                    if let Some(eb) = else_b {
                        collect_from_stmts(eb, target_offset, sym, funcs_map, structs_map);
                    }
                }
                Stmt::While(_, body) => {
                    collect_from_stmts(body, target_offset, sym, funcs_map, structs_map);
                }
                Stmt::For(var_name, _, body) => {
                    sym.insert(var_name.clone(), "i32".to_string());
                    collect_from_stmts(body, target_offset, sym, funcs_map, structs_map);
                }
                _ => {}
            }
        }
    }

    collect_from_stmts(&func.body, target_offset, &mut sym, funcs_map, structs_map);
    sym
}

fn symbol_at_offset(source: &str, offset: usize) -> Option<(String, usize, usize)> {
    // Find the identifier (or number) that contains the given byte offset.
    let bytes = source.as_bytes();
    if offset >= bytes.len() {
        return None;
    }
    let c = source[offset..].chars().next()?;
    if !c.is_alphanumeric() && c != '_' {
        return None;
    }
    let start = source[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map(|(i, _)| i)
        .unwrap_or(offset);
    let end = source[offset..]
        .char_indices()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map(|(i, c)| offset + i + c.len_utf8())
        .unwrap_or(offset + c.len_utf8());
    if start < end {
        Some((source[start..end].to_string(), start, end))
    } else {
        None
    }
}

fn is_offset_in_string_literal(source: &str, offset: usize) -> bool {
    let mut backslash_count = 0;
    let mut quote_count = 0;
    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\\' {
            backslash_count += 1;
        } else if c == '"' {
            // An escaped quote (\" or \") doesn't toggle in/out.
            if backslash_count % 2 == 0 {
                quote_count += 1;
            }
            backslash_count = 0;
        } else {
            backslash_count = 0;
        }
    }
    quote_count % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    // The span/file tables are global mutable state, so LSP tests must run
    // serially.  Use an async mutex because the tests themselves are async.
    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[test]
    fn test_position_converter() {
        let source = "let α = 1;\nlet b = 2;\n";
        let conv = PositionConverter::new(source);

        // "let α"
        // 'α' is a 2-byte UTF-8 char, but 1 UTF-16 code unit.
        // In the first line "let α = 1;"
        // "let " is 4 chars. "α" starts at char 4.
        let pos_alpha = Position::new(0, 4);
        let offset = conv.lsp_to_offset(&pos_alpha).unwrap();
        assert_eq!(offset, 4); // "let " is 4 bytes.

        let back_pos = conv.offset_to_lsp(offset);
        assert_eq!(back_pos.line, 0);
        assert_eq!(back_pos.character, 4);

        // '=' is at character 6 (let(4) + space(1) + α(1) = 6).
        // In UTF-8, "let α " is 4 + 2 + 1 = 7 bytes.
        let pos_eq = Position::new(0, 6);
        let offset_eq = conv.lsp_to_offset(&pos_eq).unwrap();
        assert_eq!(offset_eq, 7);

        let back_pos_eq = conv.offset_to_lsp(offset_eq);
        assert_eq!(back_pos_eq.line, 0);
        assert_eq!(back_pos_eq.character, 6);

        // Second line: "let b = 2;"
        let pos_line2 = Position::new(1, 4); // 'b'
        let offset_line2 = conv.lsp_to_offset(&pos_line2).unwrap();
        // First line is "let α = 1;\n". Length: "let " (4) + "α" (2) + " = 1;" (5) + "\n" (1) = 12 bytes.
        // Second line: "let " is 4 bytes. So 'b' is at offset 12 + 4 = 16.
        assert_eq!(offset_line2, 16);

        let back_pos_line2 = conv.offset_to_lsp(offset_line2);
        assert_eq!(back_pos_line2.line, 1);
        assert_eq!(back_pos_line2.character, 4);
    }

    #[tokio::test]
    async fn test_hover_local_var() {
        let _guard = TEST_LOCK.lock().await;
        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();

        let test_path = PathBuf::from("/tmp/test_hover.fox");
        let uri = Url::from_file_path(&test_path).unwrap();
        let source = "fn main(x: i32) { x; }".to_string();
        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.clone());

        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = Some("/tmp/test_hover.fox".to_string());
        });

        let mut funcs = Vec::new();

        let f = Function {
            is_pub: false,
            is_extern: false,
            is_compiler: false,
            _is_pub: false,
            _is_static: false,
            parent_struct: None,
            name: "main".to_string(),
            generic: crate::ast::GenericParams { params: vec![] },
            params: vec![crate::ast::Param {
                name: "x".to_string(),
                ty: crate::ast::Type::I32,
                is_variadic: false,
            }],
            return_ty: crate::ast::Type::Void,
            body: vec![Stmt::ExprStmt(Expr::Identifier("x".to_string()))],
            attributes: vec![],
        };
        funcs.push(f);

        let f_span = crate::ast::Span {
            start: 0,
            end: 22,
            line: 1,
            column: 1,
        };
        crate::ast::register_span(&funcs[0], f_span);

        let stmt_span = crate::ast::Span {
            start: 18,
            end: 20,
            line: 1,
            column: 18,
        };
        crate::ast::register_span(&funcs[0].body[0], stmt_span);

        if let Stmt::ExprStmt(expr) = &funcs[0].body[0] {
            let ident_span = crate::ast::Span {
                start: 18,
                end: 19,
                line: 1,
                column: 18,
            };
            crate::ast::register_span(expr, ident_span);
        }

        {
            let mut cache = crate::get_ast_cache().write().unwrap();
            cache.structs = vec![];
            cache.funcs = funcs;
            cache.impls = vec![];
            cache.consts = vec![];
        }

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(0, 18),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let res = backend.hover(params).await.unwrap();
        assert!(res.is_some());
        if let Some(hover) = res {
            if let HoverContents::Scalar(MarkedString::String(text)) = &hover.contents {
                assert!(
                    text.contains("x"),
                    "Hover text should contain variable name, got: {}",
                    text
                );
                assert!(
                    text.contains("i32"),
                    "Hover text should contain type i32, got: {}",
                    text
                );
            } else {
                panic!(
                    "Expected Scalar String hover contents, got: {:?}",
                    hover.contents
                );
            }
        }

        crate::clear_ast_cache();
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_document_symbol() {
        let _guard = TEST_LOCK.lock().await;
        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();

        let cargo_toml_path = std::env::current_dir().unwrap().join("Cargo.toml");
        let uri = Url::from_file_path(&cargo_toml_path).unwrap();
        let source = "struct Point {\n    x: i32,\n    y: i32,\n}\n\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n".to_string();
        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.clone());

        let mut structs = Vec::new();
        let mut funcs = Vec::new();

        let s = StructDef {
            is_pub: true,
            name: "Point".to_string(),
            generic: crate::ast::GenericParams::default(),
            fields: vec![],
            methods: vec![],
            is_enum: false,
            variants: vec![],
            attributes: vec![],
        };
        structs.push(s);

        let f = Function {
            is_pub: true,
            is_extern: false,
            is_compiler: false,
            _is_pub: true,
            _is_static: false,
            parent_struct: None,
            name: "add".to_string(),
            generic: crate::ast::GenericParams::default(),
            params: vec![],
            return_ty: crate::ast::Type::I32,
            body: vec![],
            attributes: vec![],
        };
        funcs.push(f);

        let dummy_path = cargo_toml_path.to_string_lossy().to_string();
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = Some(dummy_path.clone());
        });

        let s_span = crate::ast::Span {
            start: 0,
            end: 42,
            line: 1,
            column: 1,
        };
        crate::ast::register_span(&structs[0], s_span);

        let f_span = crate::ast::Span {
            start: 44,
            end: 85,
            line: 6,
            column: 1,
        };
        crate::ast::register_span(&funcs[0], f_span);

        {
            let mut cache = crate::get_ast_cache().write().unwrap();
            cache.structs = structs;
            cache.funcs = funcs;
            cache.impls = vec![];
            cache.consts = vec![];
        }

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let res = backend.document_symbol(params).await.unwrap();
        assert!(res.is_some());
        if let Some(DocumentSymbolResponse::Nested(symbols)) = res {
            assert_eq!(symbols.len(), 2);
            assert_eq!(symbols[0].name, "Point");
            assert_eq!(symbols[1].name, "add");
        } else {
            panic!("Expected nested document symbols");
        }

        crate::clear_ast_cache();
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_goto_definition_local_var() {
        let _guard = TEST_LOCK.lock().await;
        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();

        let test_path = PathBuf::from("/tmp/test_goto.fox");
        let uri = Url::from_file_path(&test_path).unwrap();
        let source = "fn main(x: i32) { x; }".to_string();
        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.clone());

        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = Some("/tmp/test_goto.fox".to_string());
        });

        let mut funcs = Vec::new();

        let f = Function {
            is_pub: false,
            is_extern: false,
            is_compiler: false,
            _is_pub: false,
            _is_static: false,
            parent_struct: None,
            name: "main".to_string(),
            generic: crate::ast::GenericParams { params: vec![] },
            params: vec![crate::ast::Param {
                name: "x".to_string(),
                ty: crate::ast::Type::I32,
                is_variadic: false,
            }],
            return_ty: crate::ast::Type::Void,
            body: vec![Stmt::ExprStmt(Expr::Identifier("x".to_string()))],
            attributes: vec![],
        };
        funcs.push(f);

        let f_span = crate::ast::Span {
            start: 0,
            end: 22,
            line: 1,
            column: 1,
        };
        crate::ast::register_span(&funcs[0], f_span);

        let param_span = crate::ast::Span {
            start: 12,
            end: 13,
            line: 0,
            column: 12,
        };
        crate::ast::register_span(&funcs[0].params[0], param_span);

        let stmt_span = crate::ast::Span {
            start: 18,
            end: 20,
            line: 1,
            column: 18,
        };
        crate::ast::register_span(&funcs[0].body[0], stmt_span);

        if let Stmt::ExprStmt(expr) = &funcs[0].body[0] {
            let ident_span = crate::ast::Span {
                start: 18,
                end: 19,
                line: 1,
                column: 18,
            };
            crate::ast::register_span(expr, ident_span);
        }

        {
            let mut cache = crate::get_ast_cache().write().unwrap();
            cache.structs = vec![];
            cache.funcs = funcs;
            cache.impls = vec![];
            cache.consts = vec![];
        }

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(0, 18),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        };

        let res = backend.goto_definition(params).await.unwrap();
        assert!(
            res.is_some(),
            "goto_definition should find the parameter 'x'"
        );
        if let Some(GotoDefinitionResponse::Scalar(loc)) = res {
            assert!(
                loc.range.start.character <= 18,
                "Definition should be at or before position 18"
            );
        } else {
            panic!("Expected Scalar goto definition response, got: {:?}", res);
        }

        crate::clear_ast_cache();
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_goto_definition_with_compile() {
        let _guard = TEST_LOCK.lock().await;
        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();

        // Use a temp file path for the test
        let test_dir = std::env::temp_dir().join("fox_lsp_test");
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("test_goto_compile.fox");
        // A simple file with a local var reference
        let source = "fn test() {\n    let y: i32 = 42;\n    y;\n}\n";
        std::fs::write(&test_file, source).unwrap();

        let uri = Url::from_file_path(&test_file).unwrap();

        // Simulate did_open: store document, then validate
        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.to_string());
        backend.validate_document(uri.clone()).await;

        // Check what's in the cache
        {
            let _cache = crate::get_ast_cache().read().unwrap();
        }

        // Now test goto_definition at position of 'y' on the last line
        // Line 2 (0-indexed): "    y;\n" — 'y' is at character 4
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 4),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        };

        let res = backend.goto_definition(params).await.unwrap();
        // We expect to find the definition of 'y' at line 1 (let y: i32 = 42;)
        assert!(
            res.is_some(),
            "goto_definition should find local var 'y' after compile"
        );

        crate::clear_ast_cache();
        crate::diagnostics::clear_diagnostics();
        let _ = std::fs::remove_dir_all(&test_dir);
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_diagnostics_range() {
        let _guard = TEST_LOCK.lock().await;
        let test_dir = std::env::temp_dir().join("fox_lsp_diag_range");
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("test.fox");
        let source = "fn add(a: i32, b: i32): i32 {\n    return a + b;\n}\n\nfn test_func(): i32 {\n    add(5, \"hello\");\n    return 1;\n}\n";
        std::fs::write(&test_file, source).unwrap();

        crate::compile_only_for_diagnostics(&test_file);

        let diags = crate::diagnostics::DIAGNOSTICS.with(|d| d.borrow().clone());
        assert!(
            !diags.is_empty(),
            "Diagnostics should be generated for type mismatch"
        );

        let converter = PositionConverter::new(source);
        let mut found_type_mismatch = false;
        for diag in &diags {
            if diag.message == "Type mismatch: expected 'i32', found 'str'" {
                found_type_mismatch = true;
                let span = diag
                    .span
                    .expect("Type mismatch diagnostic should have a span");
                assert!(
                    span.start > 0,
                    "Diagnostic span start should not be 0, got {:?}",
                    span
                );
                let pos = converter.offset_to_lsp(span.start);
                // The string literal "hello" is on 0-indexed line 5 at some column.
                assert_eq!(
                    pos.line, 5,
                    "Diagnostic should point at line 6 (1-indexed), got {:?}",
                    pos
                );
            }
        }
        assert!(
            found_type_mismatch,
            "Expected a type mismatch diagnostic"
        );

        crate::diagnostics::clear_diagnostics();
        crate::clear_ast_cache();
        let _ = std::fs::remove_dir_all(&test_dir);
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_hover_local_var_after_compile() {
        let _guard = TEST_LOCK.lock().await;
        let test_dir = std::env::temp_dir().join("fox_lsp_hover_local");
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("test.fox");
        let source = "fn test(): i32 {\n    let y: i32 = 42;\n    y;\n}\n";
        std::fs::write(&test_file, source).unwrap();

        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();
        let uri = Url::from_file_path(&test_file).unwrap();

        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.to_string());
        backend.validate_document(uri.clone()).await;

        // Hover over the 'y' reference on line 2 (0-indexed), character 4.
        let hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 4),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        let hover_res = backend.hover(hover_params).await.unwrap();
        assert!(
            hover_res.is_some(),
            "hover should find local variable 'y' at line 3"
        );
        if let Some(hover) = hover_res {
            if let HoverContents::Scalar(MarkedString::String(text)) = &hover.contents {
                assert!(
                    text.contains("y"),
                    "Hover text should contain variable name, got: {}",
                    text
                );
                assert!(
                    text.contains("i32"),
                    "Hover text should contain type i32, got: {}",
                    text
                );
            } else {
                panic!(
                    "Expected Scalar String hover contents, got: {:?}",
                    hover.contents
                );
            }
        }

        crate::clear_ast_cache();
        crate::diagnostics::clear_diagnostics();
        let _ = std::fs::remove_dir_all(&test_dir);
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_hover_local_var_from_static_method_call() {
        let _guard = TEST_LOCK.lock().await;
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap());
        let fox_path = manifest_dir.to_string_lossy().to_string();
        unsafe {
            std::env::set_var("FOX_PATH", &fox_path);
        }

        let test_dir = std::env::temp_dir().join("fox_lsp_hover_nav");
        let _ = std::fs::create_dir_all(&test_dir);
        let test_file = test_dir.join("test.fox");
        let source = "use std::global::{Document, Element};\n\nstruct Div {}\n\nimpl Div {\n    pub fn class(): void {}\n}\n\nfn main(): void {\n    let nav = Document::create_element(\"nav\");\n    nav.set_attribute(\"class\", \"logo\");\n}\n";
        std::fs::write(&test_file, source).unwrap();

        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();
        let uri = Url::from_file_path(&test_file).unwrap();

        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.to_string());
        backend.validate_document(uri.clone()).await;

        // Hover over 'nav' on line 9 (0-indexed), after "let " at char 8.
        let hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(9, 8),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        let hover_res = backend.hover(hover_params).await.unwrap();
        assert!(
            hover_res.is_some(),
            "hover should find local variable 'nav'"
        );
        if let Some(hover) = hover_res {
            if let HoverContents::Scalar(MarkedString::String(text)) = &hover.contents {
                assert!(
                    text.contains("nav"),
                    "Hover text should contain variable name, got: {}",
                    text
                );
            } else {
                panic!(
                    "Expected Scalar String hover contents, got: {:?}",
                    hover.contents
                );
            }
        }

        // Hover over the string literal "class" inside set_attribute (line 10, char 23).
        // It must NOT resolve to Div::class or any other method named class, and
        // it should not resolve to any symbol from inside a string literal.
        let string_hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(10, 23),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        let string_hover_res = backend.hover(string_hover_params).await.unwrap();
        assert!(
            string_hover_res.is_none(),
            "Hover inside a string literal should not return any symbol result"
        );

        crate::clear_ast_cache();
        crate::diagnostics::clear_diagnostics();
        let _ = std::fs::remove_dir_all(&test_dir);
        unsafe {
            std::env::remove_var("FOX_PATH");
        }
        crate::diagnostics::CURRENT_FILE.with(|cf| {
            *cf.borrow_mut() = None;
        });
    }

    #[tokio::test]
    async fn test_goto_definition_docs_main_ext_link() {
        let _guard = TEST_LOCK.lock().await;
        let (service, _socket) = LspService::new(|client| Backend {
            client,
            documents: RwLock::new(HashMap::new()),
        });
        let backend = service.inner();

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap());
        let fox_path = manifest_dir.to_string_lossy().to_string();
        unsafe {
            std::env::set_var("FOX_PATH", &fox_path);
        }

        let test_file = manifest_dir.join("docs").join("main.fox");
        let source = std::fs::read_to_string(&test_file).unwrap();
        let uri = Url::from_file_path(&test_file).unwrap();

        backend
            .documents
            .write()
            .unwrap()
            .insert(uri.clone(), source.clone());
        backend.validate_document(uri.clone()).await;

        {
            let _cache = crate::get_ast_cache().read().unwrap();
        }

        // Line 238 (1-based) -> LSP line 237. 'ext_link' identifier starts around col 23.
        let pos = Position::new(237, 27);
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        };

        let res = backend.goto_definition(params).await.unwrap();
        assert!(
            res.is_some(),
            "goto_definition should find ext_link at line 238"
        );

        let hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: pos,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };
        let hover_res = backend.hover(hover_params).await.unwrap();
        assert!(
            hover_res.is_some(),
            "hover should find ext_link at line 238"
        );

        crate::clear_ast_cache();
        crate::diagnostics::clear_diagnostics();
    }

    #[tokio::test]
    async fn test_diagnostics() {
        let _guard = TEST_LOCK.lock().await;
        let cargo_toml_path = std::env::current_dir().unwrap().join("Cargo.toml");
        let source = "fn add(a: i32, b: i32): i32 {\n    return a + b;\n}\n\nfn test_func(): i32 {\n    add(5, \"hello\");\n    return 1;\n}\n".to_string();

        let mut vfs_map = HashMap::new();
        vfs_map.insert(cargo_toml_path.clone(), source);
        VFS.with(|vfs| {
            *vfs.borrow_mut() = Some(vfs_map);
        });

        crate::compile_only_for_diagnostics(&cargo_toml_path);

        let compiler_diags = crate::diagnostics::DIAGNOSTICS.with(|d| d.borrow().clone());
        assert!(
            !compiler_diags.is_empty(),
            "Diagnostics should be generated"
        );

        let has_correct_path = compiler_diags.iter().any(|d| {
            if let Some(ref fp) = d.file_path {
                let df_canon = std::fs::canonicalize(fp).unwrap_or_else(|_| PathBuf::from(fp));
                df_canon == cargo_toml_path
            } else {
                false
            }
        });
        assert!(
            has_correct_path,
            "Diagnostic should have the correct file path"
        );

        crate::diagnostics::clear_diagnostics();
        VFS.with(|vfs| {
            *vfs.borrow_mut() = None;
        });
        crate::clear_ast_cache();
    }
}
