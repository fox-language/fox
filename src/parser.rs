use crate::ast::*;
use crate::lexer::Lexer;
use std::sync::atomic::{AtomicUsize, Ordering};

static NESTED_TUPLE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn is_system_file() -> bool {
    let file_opt = crate::diagnostics::CURRENT_FILE.with(|f| f.borrow().clone());
    if let Some(path_str) = file_opt {
        path_str.contains("/std/") || path_str.contains("\\std\\") || path_str.starts_with("std/") || path_str.starts_with("std\\") ||
        path_str.contains("/benchmarks/") || path_str.contains("\\benchmarks\\") || path_str.starts_with("benchmarks/") || path_str.starts_with("benchmarks\\")
    } else {
        false
    }
}

#[derive(Debug, Clone)]
enum ParsePattern {
    Var(String, Type), // name, type
    Tuple(Vec<ParsePattern>),
}

fn get_pattern_type(pattern: &ParsePattern) -> Type {
    match pattern {
        ParsePattern::Var(_, ty) => ty.clone(),
        ParsePattern::Tuple(subs) => {
            let sub_tys: Vec<Type> = subs.iter().map(get_pattern_type).collect();
            Type::Tuple(sub_tys)
        }
    }
}

fn flatten_pattern(
    pattern: &ParsePattern,
    rhs: Expr,
    stmts: &mut Vec<Stmt>,
) {
    match pattern {
        ParsePattern::Var(name, ty) => {
            stmts.push(Stmt::Let(name.clone(), Some(ty.clone()), rhs));
        }
        ParsePattern::Tuple(subs) => {
            let mut flat_bindings = Vec::new();
            let mut nested_to_process = Vec::new();
            for sub in subs {
                match sub {
                    ParsePattern::Var(name, ty) => {
                        flat_bindings.push((name.clone(), ty.clone()));
                    }
                    ParsePattern::Tuple(_) => {
                        let temp_id = NESTED_TUPLE_COUNTER.fetch_add(1, Ordering::SeqCst);
                        let temp_name = format!("_nested_tuple_{}", temp_id);
                        let temp_ty = get_pattern_type(sub);
                        flat_bindings.push((temp_name.clone(), temp_ty));
                        nested_to_process.push((sub, temp_name));
                    }
                }
            }
            stmts.push(Stmt::LetTuple(flat_bindings, rhs));
            for (sub, temp_name) in nested_to_process {
                flatten_pattern(sub, Expr::Identifier(temp_name), stmts);
            }
        }
    }
}

pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub current_token: Token,
    pub current_span: Span,
    pub previous_span: Span,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let spanned = lexer.next_token();
        Parser {
            lexer,
            current_token: spanned.node,
            current_span: spanned.span,
            previous_span: spanned.span,
        }
    }
    pub fn advance(&mut self) {
        self.previous_span = self.current_span;
        let spanned = self.lexer.next_token();
        self.current_token = spanned.node;
        self.current_span = spanned.span;
    }
    pub fn expect(&mut self, expected: Token) {
        if self.current_token == expected {
            self.advance();
        } else if expected == Token::Greater && self.current_token == Token::ShiftRight {
            self.current_token = Token::Greater;
        } else {
            let next_spanned = self.lexer.next_token();
            let msg = format!("Expected {:?}, found {:?} (next: {:?})", expected, self.current_token, next_spanned.node);
            crate::diagnostics::report_error(msg, Some(self.current_span));
            self.advance();
        }
    }
    fn span_from(&self, start: Span) -> Span {
        Span::new(start.start, self.previous_span.end, start.line, start.column)
    }
    fn register_span<T>(&self, node: &T, start: Span) -> Span {
        let span = self.span_from(start);
        register_span(node, span);
        span
    }
    pub fn current_as_identifier(&self) -> Option<String> {
        match &self.current_token {
            Token::Identifier(n) | Token::Type(n) => Some(n.clone()),
            Token::New => Some("new".to_string()),
            Token::Map => Some("map".to_string()),
            _ => None,
        }
    }
    pub fn lexer_peek(&self) -> Token {
        self.lexer.clone_peek()
    }
    pub fn parse_generic_param(&mut self) -> GenericParam {
        let type_name = match &self.current_token {
            Token::Identifier(n) => n.clone(),
            Token::Type(n) => n.clone(),
            _ => panic!("Expected generic type name, found {:?}", self.current_token),
        };
        self.advance();
        let mut constraints = Vec::new();
        if self.current_token == Token::In {
            self.advance();
            loop {
                constraints.push(self.parse_type());
                if self.current_token == Token::Pipe {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        GenericParam {
            name: type_name,
            constraints,
        }
    }
    pub fn parse_param_list(&mut self) -> Vec<Param> {
        self.expect(Token::LParen);
        let mut params = Vec::new();
        while self.current_token != Token::RParen {
            let mut is_variadic = false;
            if self.current_token == Token::DotDotDot {
                is_variadic = true;
                self.advance();
            }
            let param_name = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => panic!("Expected param name"),
            };
            self.advance();
            self.expect(Token::Colon);
            let param_ty = self.parse_type();
            params.push(Param {
                name: param_name,
                ty: param_ty,
                is_variadic,
            });
            if is_variadic {
                if self.current_token == Token::Comma {
                    self.advance();
                }
                if self.current_token != Token::RParen {
                    panic!("Variadic parameter must be the last parameter");
                }
                break;
            }
            if self.current_token == Token::Comma {
                self.advance();
            }
        }
        self.expect(Token::RParen);
        params
    }

    pub fn parse_type(&mut self) -> Type {
        if self.current_token == Token::LParen {
            self.advance();
            let mut params = Vec::new();
            while self.current_token != Token::RParen {
                params.push(self.parse_type());
                if self.current_token == Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen);
            if params.is_empty() || self.current_token == Token::Colon {
                let mut ret_ty = Type::Void;
                if self.current_token == Token::Colon {
                    self.advance();
                    ret_ty = self.parse_type();
                }
                return Type::Function(params, Box::new(ret_ty));
            }
            return Type::Tuple(params);
        }
        if self.current_token == Token::LBracket {
            self.advance();
            self.expect(Token::RBracket);
            if !is_system_file() {
                crate::diagnostics::report_error("Raw array types '[]T' are not allowed outside the standard library and benchmarks".to_string(), Some(self.current_span));
            }
            let inner = self.parse_type();
            Type::Array(Box::new(inner))
        } else {
            let ty_name = match &self.current_token {
                Token::Type(t) => {
                    let t = t.clone();
                    self.advance();
                    t
                }
                Token::Identifier(t) => {
                    let mut ty = t.clone();
                    self.advance();
                    while self.current_token == Token::DoubleColon {
                        self.advance();
                        let next_part = match &self.current_token {
                            Token::Identifier(n2) => n2.clone(),
                            _ => {
                                crate::diagnostics::report_error("Expected identifier after :: in type".to_string(), Some(self.current_span));
                                "err".to_string()
                            }
                        };
                        ty = format!("{}::{}", ty, next_part);
                        self.advance();
                    }
                    ty
                }
                _ => {
                    crate::diagnostics::report_error(format!("Expected type, found {:?}", self.current_token), Some(self.current_span));
                    "void".to_string()
                }
            };
            if self.current_token == Token::Less {
                self.advance();
                let mut args = Vec::new();
                loop {
                    args.push(self.parse_type());
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(Token::Greater);
                Type::Struct(ty_name, args)
            } else {
                match ty_name.as_str() {
                    "i32" => Type::I32,
                    "i64" => Type::I64,
                    "f32" => Type::F32,
                    "f64" => Type::F64,
                    "bool" => Type::Bool,
                    "str" => Type::Str,
                    "byte" => Type::Byte,
                    "anyref" => Type::Anyref,
                    "externref" => Type::Externref,
                    "void" => Type::Void,
                    _ => Type::GenericParam(ty_name),
                }
            }
        }
    }
    fn parse_destructure_pattern(&mut self) -> ParsePattern {
        if self.current_token == Token::LParen {
            self.advance();
            let mut sub_patterns = Vec::new();
            while self.current_token != Token::RParen {
                sub_patterns.push(self.parse_destructure_pattern());
                if self.current_token == Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen);
            ParsePattern::Tuple(sub_patterns)
        } else {
            let var_name = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => panic!("Expected variable name in tuple pattern, got {:?}", self.current_token),
            };
            self.advance();
            let var_ty = if self.current_token == Token::Colon {
                self.advance();
                self.parse_type()
            } else {
                Type::GenericParam("".to_string())
            };
            ParsePattern::Var(var_name, var_ty)
        }
    }
    
    pub fn parse_attributes(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        self.expect(Token::Hash);
        self.expect(Token::LBracket);
        let name = self.current_as_identifier().expect("Expected attribute name");
        self.advance();
        let mut args = Vec::new();
        if self.current_token == Token::LParen {
            self.advance();
            while self.current_token != Token::RParen {
                if let Token::StringLit(s) = &self.current_token {
                    args.push(s.clone());
                    self.advance();
                } else if let Some(n) = self.current_as_identifier() {
                    args.push(n);
                    self.advance();
                }
                if self.current_token == Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen);
        }
        attrs.push(Attribute { name, args });
        if self.current_token == Token::Comma {
            self.advance();
            // Parse additional attributes separated by commas
            loop {
                let next_name = self.current_as_identifier().expect("Expected attribute name after comma");
                self.advance();
                let mut next_args = Vec::new();
                if self.current_token == Token::LParen {
                    self.advance();
                    while self.current_token != Token::RParen {
                        if let Token::StringLit(s) = &self.current_token {
                            next_args.push(s.clone());
                            self.advance();
                        } else if let Some(n) = self.current_as_identifier() {
                            next_args.push(n);
                            self.advance();
                        }
                        if self.current_token == Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen);
                }
                attrs.push(Attribute { name: next_name, args: next_args });
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RBracket);
        attrs
    }

    pub fn parse_module(&mut self) -> Vec<Item> {
        let mut items = Vec::new();
        while self.current_token != Token::Eof {
            let mut attributes = Vec::new();
            while self.current_token == Token::Hash {
                attributes.extend(self.parse_attributes());
            }
            let mut is_pub = false;
            if self.current_token == Token::Pub {
                is_pub = true;
                self.advance();
            }
            let next_token = self.current_token.clone();

            if next_token == Token::Struct {
                let (s, span) = self.parse_struct(is_pub, attributes.clone());
                items.push(Item::Struct(s, span));
            } else if next_token == Token::Enum {
                let (s, span) = self.parse_enum(is_pub, attributes.clone());
                items.push(Item::Struct(s, span));
            } else if next_token == Token::Trait {
                let (t, span) = self.parse_trait(is_pub, attributes.clone());
                items.push(Item::Trait(t, span));
            } else if next_token == Token::Impl {
                let (imp, span) = self.parse_impl(is_pub, attributes.clone());
                items.push(Item::Impl(imp, span));
            } else if next_token == Token::Const || next_token == Token::Let {
                let (c, span) = self.parse_const_or_let(is_pub, attributes.clone());
                items.push(Item::Const(c, span));
            } else if self.current_token == Token::Use {
                self.advance();

                // use path::{Symbol1, Symbol2}; or use path; or use path::symbol as alias;
                let mut path = Vec::new();
                let mut is_bare = false;
                let mut single_import: Option<(String, Option<String>)> = None;
                loop {
                    if let Token::Identifier(id) | Token::Type(id) = &self.current_token {
                            path.push(id.clone());
                            self.advance();
                        } else {
                            panic!("Expected identifier in import path, found {:?}", self.current_token);
                        }
                        if self.current_token == Token::DoubleColon {
                            self.advance();
                            if self.current_token == Token::LBrace {
                                break;
                            }
                            // Check if next token is `as` — means the previous segment was the symbol
                            if self.current_token == Token::As {
                                let symbol = path.pop().unwrap();
                                self.advance();
                                if let Token::Identifier(alias) | Token::Type(alias) = &self.current_token {
                                    single_import = Some((symbol, Some(alias.clone())));
                                    self.advance();
                                } else {
                                    panic!("Expected alias name after 'as'");
                                }
                                self.expect(Token::Semicolon);
                                break;
                            }
                        } else if self.current_token == Token::Semicolon {
                            // Bare namespace import: use std::fmt;
                            self.advance();
                            is_bare = true;
                            break;
                        } else if self.current_token == Token::As {
                            // use path::symbol as alias; (symbol is last path segment)
                            let symbol = path.pop().unwrap();
                            self.advance();
                            if let Token::Identifier(alias) | Token::Type(alias) = &self.current_token {
                                single_import = Some((symbol, Some(alias.clone())));
                                self.advance();
                            } else {
                                panic!("Expected alias name after 'as'");
                            }
                            self.expect(Token::Semicolon);
                            break;
                        } else {
                            panic!("Expected :: or ; in import path, found {:?}", self.current_token);
                        }
                    }

                    if is_bare {
                        items.push(Item::Use { path, symbols: Vec::new() });
                    } else if let Some((symbol, alias)) = single_import {
                        items.push(Item::Use { path, symbols: vec![(symbol, alias)] });
                    } else {
                    self.expect(Token::LBrace);
                    let mut symbols: Vec<(String, Option<String>)> = Vec::new();
                    loop {
                        if let Token::Identifier(s) | Token::Type(s) = &self.current_token {
                            let original = s.clone();
                            self.advance();
                            // Check for `as alias`
                            if self.current_token == Token::As {
                                self.advance();
                                if let Token::Identifier(alias) | Token::Type(alias) = &self.current_token {
                                    symbols.push((original, Some(alias.clone())));
                                    self.advance();
                                } else {
                                    panic!("Expected alias name after 'as'");
                                }
                            } else {
                                symbols.push((original, None));
                            }
                        } else {
                            panic!("Expected identifier in import symbol list");
                        }
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(Token::RBrace);
                    self.expect(Token::Semicolon);
                    items.push(Item::Use { path, symbols });
                    }
            } else {
                let (f, span) = self.parse_function(None, is_pub, attributes.clone());
                items.push(Item::Function(f, span));
            }
        }
        items
    }

    pub fn parse_const_or_let(&mut self, is_pub: bool, attributes: Vec<Attribute>) -> (ConstDef, Span) {
        let start_span = self.current_span;
        let is_mutable = if self.current_token == Token::Let {
            self.advance();
            true
        } else {
            self.expect(Token::Const);
            false
        };
        let name = match &self.current_token {
            Token::Identifier(n) => n.clone(),
            Token::Type(n) => n.clone(),
            _ => panic!("Expected constant or variable name, found {:?}", self.current_token),
        };
        self.advance();
        let ty = if self.current_token == Token::Colon {
            self.advance();
            self.parse_type()
        } else {
            Type::Anyref
        };
        self.expect(Token::Assign);
        let value = self.parse_expr();
        let ty = if matches!(ty, Type::Anyref) {
            match &value {
                Expr::StringLit(_) => Type::Str,
                Expr::Bool(_) => Type::Bool,
                Expr::Integer(_) | Expr::Float(_) => {
                    panic!("Type annotation is required for numeric constant '{}'", name);
                }
                _ => panic!("Type annotation is required for constant '{}'", name),
            }
        } else {
            ty
        };
        self.expect(Token::Semicolon);
        let node = ConstDef {
            is_pub,
            name, ty, value, attributes, is_mutable };
        let span = self.span_from(start_span);
        (node, span)
    }

    pub fn parse_struct(&mut self, is_pub: bool, attributes: Vec<Attribute>) -> (StructDef, Span) {
        let start_span = self.current_span;
        self.expect(Token::Struct);
        let name = match &self.current_token {
            Token::Identifier(n) => n.clone(),
            _ => panic!("Expected struct name"),
        };
        self.advance();
        let mut generic = GenericParams::default();
        if self.current_token == Token::Less {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.parse_generic_param());
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(Token::Greater);
            generic = GenericParams { params };
        }
        self.expect(Token::LBrace);

        let mut fields = Vec::new();

        while self.current_token != Token::RBrace {
            let mut field_attrs = Vec::new();
            while self.current_token == Token::Hash {
                field_attrs.extend(self.parse_attributes());
            }
            if self.current_token == Token::Pub {
                self.advance();
            }
            let field_name = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => panic!("Expected field name inside struct definition"),
            };
            self.advance();
            self.expect(Token::Colon);
            let ty = self.parse_type();
            self.expect(Token::Semicolon);
            fields.push(Field { name: field_name, ty, attributes: field_attrs });
        }
        self.expect(Token::RBrace);

        let node = StructDef {
            is_pub,
            name,
            generic,
            fields,
            methods: Vec::new(),
            is_enum: false, variants: Vec::new(), attributes };
        let span = self.span_from(start_span);
        (node, span)
    }

    pub fn parse_enum(&mut self, is_pub: bool, attributes: Vec<Attribute>) -> (StructDef, Span) {
        let start_span = self.current_span;
        self.expect(Token::Enum);
        let name = match &self.current_token {
            Token::Identifier(n) => n.clone(),
            _ => panic!("Expected enum name"),
        };
        self.advance();
        let mut generic = GenericParams::default();
        if self.current_token == Token::Less {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.parse_generic_param());
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(Token::Greater);
            generic = GenericParams { params };
        }
        self.expect(Token::LBrace);

        let mut variants = Vec::new();
        let mut methods = Vec::new();

        while self.current_token != Token::RBrace {
            let variant_name = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => {
                    crate::diagnostics::report_error("Expected variant name inside enum definition".to_string(), Some(self.current_span));
                    break;
                }
            };
            self.advance();
            let mut payload_tys = Vec::new();
            self.expect(Token::LParen);
            if self.current_token != Token::RParen {
                loop {
                    payload_tys.push(self.parse_type());
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(Token::RParen);
            self.expect(Token::Semicolon);
            variants.push((variant_name, payload_tys));
        }
        self.expect(Token::RBrace);

        let mut fields = Vec::new();
        fields.push(Field { name: "_tag".to_string(), ty: Type::I32, attributes: Vec::new() });
        
        let mut variant_names = Vec::new();
        for (variant_name, payload_tys) in &variants {
            variant_names.push(variant_name.clone());
            for (idx, ty) in payload_tys.iter().enumerate() {
                fields.push(Field { name: format!("{}_{}", variant_name, idx), ty: ty.clone(), attributes: Vec::new() });
            }
        }

        let generic_names: Vec<String> = generic.params.iter().map(|p| p.name.clone()).collect();
        let enum_ty = if generic_names.is_empty() {
            Type::GenericParam(name.clone())
        } else {
            let generic_tys: Vec<Type> = generic.params.iter().map(|p| Type::GenericParam(p.name.clone())).collect();
            Type::Struct(name.clone(), generic_tys)
        };

        for (v_idx, (variant_name, payload_tys)) in variants.iter().enumerate() {
            let mut params = Vec::new();
            let mut struct_init_fields = Vec::new();

            struct_init_fields.push(("_tag".to_string(), Expr::Integer(v_idx.to_string())));

            for (idx, ty) in payload_tys.iter().enumerate() {
                let param_name = format!("payload_{}", idx);
                params.push(Param {
                    name: param_name.clone(),
                    ty: ty.clone(),
                    is_variadic: false,
                });
                struct_init_fields.push((format!("{}_{}", variant_name, idx), Expr::Identifier(param_name)));
            }

            for (other_v_name, other_payload_tys) in &variants {
                if other_v_name != variant_name {
                    for idx in 0..other_payload_tys.len() {
                        struct_init_fields.push((format!("{}_{}", other_v_name, idx), Expr::Default));
                    }
                }
            }

            let body = vec![
                Stmt::Return(Some(Expr::StructInit(
                    enum_ty.to_string(),
                    struct_init_fields,
                )))
            ];

            let constructor_fn = Function {
                is_pub: true,
                is_extern: false,
                is_compiler: false,
                _is_pub: true,
                _is_static: true,
                parent_struct: Some(name.clone()),
                name: format!("{}::{}", name, variant_name),
                generic: generic.clone(),
                params, return_ty: enum_ty.clone(), body, attributes: Vec::new() };

            methods.push(constructor_fn);
        }

        let node = StructDef {
            is_pub,
            name,
            generic,
            fields,
            methods,
            is_enum: true, variants: variant_names, attributes };
        let span = self.span_from(start_span);
        (node, span)
    }

    pub fn parse_trait(&mut self, is_pub: bool, attributes: Vec<Attribute>) -> (TraitDef, Span) {
        let start_span = self.current_span;
        self.expect(Token::Trait);
        let name = match &self.current_token {
            Token::Identifier(n) => n.clone(),
            _ => panic!("Expected trait name"),
        };
        self.advance();
        let mut generic = GenericParams::default();
        if self.current_token == Token::Less {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.parse_generic_param());
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(Token::Greater);
            generic = GenericParams { params };
        }
        self.expect(Token::LBrace);
        let mut methods = Vec::new();
        while self.current_token != Token::RBrace {
            let mut is_pub = false;
            if self.current_token == Token::Pub {
                is_pub = true;
                self.advance();
            }
            let mut is_compiler = false;
            if self.current_token == Token::Compiler {
                is_compiler = true;
                self.advance();
            }
            let mut is_static = false;
            if self.current_token == Token::Static {
                is_static = true;
                self.advance();
            }
            self.expect(Token::Fn);
            let method_name = self.current_as_identifier().expect("Expected identifier");
            self.advance();
            let mut generic_params = GenericParams::default();
            if self.current_token == Token::Less {
                self.advance();
                let mut params = Vec::new();
                loop {
                    params.push(self.parse_generic_param());
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(Token::Greater);
                generic_params = GenericParams { params };
            }
            let params = self.parse_param_list();
            let return_ty = if self.current_token == Token::Colon {
                self.advance();
                self.parse_type()
            } else {
                Type::Void
            };
            self.expect(Token::Semicolon);
            
            methods.push(Function {
                is_pub: false,
                is_extern: false,
                is_compiler,
                _is_pub: is_pub,
                _is_static: is_static,
                parent_struct: Some(name.clone()),
                name: method_name,
                generic: generic_params,
                params,
                return_ty,
                body: vec![], attributes: Vec::new() });
        }
        self.expect(Token::RBrace);
        let node = TraitDef {
            is_pub,
            name, generic, methods, attributes };
        let span = self.span_from(start_span);
        (node, span)
    }

    pub fn parse_impl(&mut self, is_pub: bool, attributes: Vec<Attribute>) -> (ImplDef, Span) {
        let start_span = self.current_span;
        self.expect(Token::Impl);
        let mut generic = GenericParams::default();
        if self.current_token == Token::Less {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.parse_generic_param());
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(Token::Greater);
            generic = GenericParams { params };
        }

        // Parse the first type. After this, two forms are valid:
        //   `impl <Trait> for <Type> { ... }`  -- trait impl
        //   `impl <Type> { ... }`              -- inherent impl
        let first_ty = self.parse_type();

        let (trait_name, target_ty) = if self.current_token == Token::For {
            self.advance(); // consume 'for'
            let target = self.parse_type();
            (Some(first_ty), target)
        } else if self.current_token == Token::LBrace {
            // Inherent impl: `impl <Type> { ... }`
            (None, first_ty)
        } else {
            panic!(
                "Expected 'for' or '{{' in impl block, found {:?}",
                self.current_token
            );
        };

        self.expect(Token::LBrace);
        let mut methods = Vec::new();
        while self.current_token != Token::RBrace {
            let (f, f_span) = self.parse_function(Some(target_ty.to_string()), false, Vec::new());
            crate::ast::register_span(&f, f_span);
            methods.push(f);
        }
        self.expect(Token::RBrace);
        let node = ImplDef {
            is_pub,
            trait_name,
            generic, target_ty, methods, attributes };
        let span = self.span_from(start_span);
        (node, span)
    }
    pub fn parse_function(&mut self, parent_struct: Option<String>, is_pub: bool, attributes: Vec<Attribute>) -> (Function, Span) {
        let start_span = self.current_span;
        let mut is_extern = false;
        let mut is_compiler = false;
        let mut is_method_pub = false;
        let mut is_static = false;
        // Parse modifiers in any order: pub, extern/compiler, static
        if self.current_token == Token::Pub {
            is_method_pub = true;
            self.advance();
        }
        if self.current_token == Token::Extern {
            is_extern = true;
            self.advance();
        } else if self.current_token == Token::Compiler {
            is_compiler = true;
            self.advance();
        }
        // Allow pub after extern/compiler too (pub compiler or compiler pub)
        if self.current_token == Token::Pub {
            is_method_pub = true;
            self.advance();
        }
        if self.current_token == Token::Static {
            is_static = true;
            self.advance();
        }
        self.expect(Token::Fn);
        let name = self.current_as_identifier().expect("Expected identifier");
        self.advance();
        let mut generic = GenericParams::default();
        if self.current_token == Token::Less {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.parse_generic_param());
                if self.current_token == Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(Token::Greater);
            generic = GenericParams { params };
        }
        let mut params = self.parse_param_list();
        let return_ty = if self.current_token == Token::Colon {
            self.advance();
            self.parse_type()
        } else {
            Type::Void
        };
        let mut body = Vec::new();

        if is_extern {
            self.expect(Token::Semicolon);
        } else if is_compiler {
            if self.current_token == Token::Semicolon {
                self.advance();
            } else {
                self.expect(Token::LBrace);
                while self.current_token != Token::RBrace {
                    self.parse_stmt(&mut body);
                }
                self.expect(Token::RBrace);
            }
        } else {
            self.expect(Token::LBrace);
            while self.current_token != Token::RBrace {
                self.parse_stmt(&mut body);
            }
            self.expect(Token::RBrace);
        }

        let mut actual_name = name.clone();
        if let Some(ref parent) = parent_struct {
            if !is_static {
                params.insert(
                    0,
                    Param {
                        name: "self".to_string(),
                        ty: parent.parse::<Type>().unwrap(),
                        is_variadic: false,
                    },
                );
            }
            actual_name = format!("{}::{}", parent, name);
        }
        let node = Function {
            is_pub,
            is_extern,
            is_compiler,
            _is_pub: is_method_pub,
            _is_static: is_static,
            parent_struct,
            name: actual_name,
            generic,
            params, return_ty, body, attributes };
        let span = self.span_from(start_span);
        for param in &node.params {
            register_span(param, span);
        }
        (node, span)
    }
    pub fn parse_match_pattern(&mut self) -> MatchPattern {
        match &self.current_token {
            Token::Identifier(name) => {
                if name == "_" {
                    self.advance();
                    return MatchPattern::CatchAll;
                }
                let variant_name = name.clone();
                self.advance();
                // Parens are now required even for empty payload (e.g., None())
                self.expect(Token::LParen);
                let mut bindings = Vec::new();
                if self.current_token != Token::RParen {
                    loop {
                        if let Token::Identifier(v) = &self.current_token {
                            bindings.push(v.clone());
                            self.advance();
                        } else {
                            panic!("Expected identifier in pattern binding");
                        }
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen);
                if variant_name == "Some" && bindings.len() == 1 {
                    MatchPattern::Some(bindings[0].clone())
                } else if variant_name == "None" && bindings.is_empty() {
                    MatchPattern::None
                } else if variant_name == "Ok" && bindings.len() == 1 {
                    MatchPattern::Ok(bindings[0].clone())
                } else if variant_name == "Err" && bindings.len() == 1 {
                    MatchPattern::Err(bindings[0].clone())
                } else {
                    MatchPattern::Variant(variant_name, bindings)
                }
            }
            _ => panic!("Expected identifier pattern, found {:?}", self.current_token),
        }
    }

    pub fn parse_match_arm_body(&mut self) -> (Vec<Stmt>, Option<Expr>) {
        if self.current_token == Token::LBrace {
            self.advance();
            let mut stmts = Vec::new();
            let mut val = None;
            while self.current_token != Token::RBrace {
                if self.current_token == Token::Let {
                    self.parse_stmt(&mut stmts);
                } else if self.current_token == Token::Return {
                    self.parse_stmt(&mut stmts);
                } else if self.current_token == Token::If {
                    self.parse_stmt(&mut stmts);
                } else if self.current_token == Token::While {
                    self.parse_stmt(&mut stmts);
                } else if self.current_token == Token::For {
                    self.parse_stmt(&mut stmts);
                } else {
                    let expr = self.parse_expr();
                    if self.current_token == Token::PlusAssign {
                        self.advance();
                        let rhs = self.parse_expr();
                        self.expect(Token::Semicolon);
                        if let Expr::Identifier(name) = expr {
                            stmts.push(Stmt::AssignPlus(name, rhs));
                        } else {
                            panic!("+= only supported on identifiers for now");
                        }
                    } else if self.current_token == Token::Assign {
                        self.advance();
                        let rhs = self.parse_expr();
                        self.expect(Token::Semicolon);
                        if let Expr::Identifier(name) = expr {
                            stmts.push(Stmt::Assign(name, rhs));
                        } else if let Expr::IndexAccess(arr, idx) = expr {
                            stmts.push(Stmt::AssignIndex(arr, idx, rhs));
                        } else if let Expr::FieldAccess(obj, field) = expr {
                            stmts.push(Stmt::AssignField(obj, field, rhs));
                        } else {
                            panic!("= only supported on identifiers, array indexes, and struct fields for now");
                        }
                    } else if self.current_token == Token::Semicolon {
                        self.advance();
                        stmts.push(Stmt::ExprStmt(expr));
                    } else if self.current_token == Token::RBrace {
                        val = Some(expr);
                        break;
                    } else {
                        panic!("Expected semicolon or }} after expression, found {:?}", self.current_token);
                    }
                }
            }
            self.expect(Token::RBrace);
            (stmts, val)
        } else {
            let expr = self.parse_expr();
            (vec![], Some(expr))
        }
    }
    pub fn parse_stmt(&mut self, out: &mut Vec<Stmt>) {
        let start_span = self.current_span;
        let stmts = self.parse_stmt_impl();
        for stmt in stmts {
            let span = self.span_from(start_span);
            out.push(stmt);
            crate::ast::register_span(out.last().unwrap(), span);
        }
    }
    fn parse_stmt_impl(&mut self) -> Vec<Stmt> {
        if self.current_token == Token::Let {
            self.advance();
            if self.current_token == Token::LParen {
                let pattern = self.parse_destructure_pattern();
                self.expect(Token::Assign);
                let expr = self.parse_expr();
                self.expect(Token::Semicolon);
                let mut stmts = Vec::new();
                flatten_pattern(&pattern, expr, &mut stmts);
                return stmts;
            }
            let var_name = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => {
                    crate::diagnostics::report_error(format!("Expected var name, got {:?}", self.current_token), Some(self.current_span));
                    "err".to_string()
                }
            };
            self.advance();
            let ty_annot = if self.current_token == Token::Colon {
                self.advance();
                Some(self.parse_type())
            } else {
                None
            };
            self.expect(Token::Assign);
            let expr = self.parse_expr();
            if ty_annot.is_none() {
                match &expr {
                    Expr::Integer(_) | Expr::Float(_) => {
                        crate::diagnostics::report_error(format!("Type annotation is required for numeric literal '{}'", var_name), Some(self.current_span));
                    }
                    _ => {}
                }
            }
            self.expect(Token::Semicolon);
            vec![Stmt::Let(var_name, ty_annot, expr)]
        } else if self.current_token == Token::Return {
            self.advance();
            let expr = if self.current_token == Token::Semicolon {
                None
            } else {
                Some(self.parse_expr())
            };
            self.expect(Token::Semicolon);
            vec![Stmt::Return(expr)]
        } else if self.current_token == Token::If {
            self.advance();
            if self.current_token == Token::Let {
                self.advance();
                
                let mut is_destructure_variant = false;
                if let Token::Identifier(_) = &self.current_token {
                    if self.lexer_peek() == Token::LParen {
                        let mut temp_lexer = self.lexer.clone();
                        let _next_tok = temp_lexer.next_token().node; // LParen
                        let next_next_tok = temp_lexer.next_token().node;
                        if next_next_tok == Token::LParen {
                            is_destructure_variant = true;
                        }
                    }
                }

                let (pattern, expr, body) = if is_destructure_variant {
                    let variant_name = match &self.current_token {
                        Token::Identifier(n) => n.clone(),
                        _ => unreachable!(),
                    };
                    self.advance();
                    self.expect(Token::LParen);
                    let destructure_pattern = self.parse_destructure_pattern();
                    self.expect(Token::RParen);
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    self.expect(Token::LBrace);
                    let mut original_body = Vec::new();
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut original_body);
                    }
                    self.expect(Token::RBrace);

                    let temp_id = NESTED_TUPLE_COUNTER.fetch_add(1, Ordering::SeqCst);
                    let temp_binding = format!("_destruct_temp_{}", temp_id);

                    let pattern = if variant_name == "Some" {
                        MatchPattern::Some(temp_binding.clone())
                    } else if variant_name == "None" {
                        MatchPattern::None
                    } else if variant_name == "Ok" {
                        MatchPattern::Ok(temp_binding.clone())
                    } else if variant_name == "Err" {
                        MatchPattern::Err(temp_binding.clone())
                    } else {
                        MatchPattern::Variant(variant_name, vec![temp_binding.clone()])
                    };

                    let mut prepended_stmts = Vec::new();
                    flatten_pattern(
                        &destructure_pattern,
                        Expr::Identifier(temp_binding),
                        &mut prepended_stmts,
                    );
                    prepended_stmts.extend(original_body);

                    (pattern, expr, prepended_stmts)
                } else {
                    let pattern = self.parse_match_pattern();
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    self.expect(Token::LBrace);
                    let mut body = Vec::new();
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut body);
                    }
                    self.expect(Token::RBrace);

                    (pattern, expr, body)
                };

                let mut else_body = Vec::new();
                if self.current_token == Token::Else {
                    self.advance();
                    self.expect(Token::LBrace);
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut else_body);
                    }
                    self.expect(Token::RBrace);
                }
                let arms = vec![
                    MatchArm { pattern, body, val: None },
                    MatchArm { pattern: MatchPattern::CatchAll, body: else_body, val: None },
                ];
                vec![Stmt::ExprStmt(Expr::Match(Box::new(expr), arms))]
            } else {
                let cond = self.parse_expr();
                self.expect(Token::LBrace);
                let mut body = Vec::new();
                while self.current_token != Token::RBrace {
                    self.parse_stmt(&mut body);
                }
                self.expect(Token::RBrace);
                let mut else_body = None;
                if self.current_token == Token::Else {
                    self.advance();
                    self.expect(Token::LBrace);
                    let mut e_body = Vec::new();
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut e_body);
                    }
                    self.expect(Token::RBrace);
                    else_body = Some(e_body);
                }
                vec![Stmt::If(cond, body, else_body)]
            }
        } else if self.current_token == Token::While {
            self.advance();
            let is_pattern_while = if self.current_token == Token::Let {
                self.advance();
                true
            } else {
                let mut temp_lexer = self.lexer.clone();
                let mut tok = self.current_token.clone();
                let mut found = false;
                loop {
                    match tok {
                        Token::Assign => {
                            found = true;
                            break;
                        }
                        Token::LBrace | Token::Eof => {
                            break;
                        }
                        _ => {
                            tok = temp_lexer.next_token().node;
                        }
                    }
                }
                found
            };

            if is_pattern_while {
                let mut is_destructure_variant = false;
                if let Token::Identifier(_) = &self.current_token {
                    if self.lexer_peek() == Token::LParen {
                        let mut temp_lexer = self.lexer.clone();
                        let _next_tok = temp_lexer.next_token().node; // LParen
                        let next_next_tok = temp_lexer.next_token().node;
                        if next_next_tok == Token::LParen {
                            is_destructure_variant = true;
                        }
                    }
                }

                let (pattern, expr, body) = if is_destructure_variant {
                    let variant_name = match &self.current_token {
                        Token::Identifier(n) => n.clone(),
                        _ => unreachable!(),
                    };
                    self.advance();
                    self.expect(Token::LParen);
                    let destructure_pattern = self.parse_destructure_pattern();
                    self.expect(Token::RParen);
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    self.expect(Token::LBrace);
                    let mut original_body = Vec::new();
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut original_body);
                    }
                    self.expect(Token::RBrace);

                    let temp_id = NESTED_TUPLE_COUNTER.fetch_add(1, Ordering::SeqCst);
                    let temp_binding = format!("_destruct_temp_{}", temp_id);

                    let pattern = if variant_name == "Some" {
                        MatchPattern::Some(temp_binding.clone())
                    } else if variant_name == "None" {
                        MatchPattern::None
                    } else if variant_name == "Ok" {
                        MatchPattern::Ok(temp_binding.clone())
                    } else if variant_name == "Err" {
                        MatchPattern::Err(temp_binding.clone())
                    } else {
                        MatchPattern::Variant(variant_name, vec![temp_binding.clone()])
                    };

                    let mut prepended_stmts = Vec::new();
                    flatten_pattern(
                        &destructure_pattern,
                        Expr::Identifier(temp_binding),
                        &mut prepended_stmts,
                    );
                    prepended_stmts.extend(original_body);

                    (pattern, expr, prepended_stmts)
                } else {
                    let pattern = self.parse_match_pattern();
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    self.expect(Token::LBrace);
                    let mut body = Vec::new();
                    while self.current_token != Token::RBrace {
                        self.parse_stmt(&mut body);
                    }
                    self.expect(Token::RBrace);
                    (pattern, expr, body)
                };

                let cond = Expr::Match(
                    Box::new(expr),
                    vec![
                        MatchArm {
                            pattern,
                            body,
                            val: Some(Expr::Bool(true)),
                        },
                        MatchArm {
                            pattern: MatchPattern::CatchAll,
                            body: vec![],
                            val: Some(Expr::Bool(false)),
                        },
                    ],
                );
                vec![Stmt::While(cond, vec![])]
            } else {
                let cond = self.parse_expr();
                self.expect(Token::LBrace);
                let mut body = Vec::new();
                while self.current_token != Token::RBrace {
                    self.parse_stmt(&mut body);
                }
                self.expect(Token::RBrace);
                vec![Stmt::While(cond, body)]
            }
        } else if self.current_token == Token::For {
            self.advance();
            let loop_var = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => panic!("Expected var name"),
            };
            self.advance();
            self.expect(Token::In);
            let iter_target = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                _ => panic!("Expected identifier"),
            };
            self.advance();
            self.expect(Token::LBrace);
            let mut body = Vec::new();
            while self.current_token != Token::RBrace {
                self.parse_stmt(&mut body);
            }
            self.expect(Token::RBrace);
            vec![Stmt::For(loop_var, iter_target, body)]
        } else {
            let expr = self.parse_expr();
            if self.current_token == Token::PlusAssign {
                self.advance();
                let rhs = self.parse_expr();
                self.expect(Token::Semicolon);
                if let Expr::Identifier(name) = expr {
                    vec![Stmt::AssignPlus(name, rhs)]
                } else {
                    panic!("+= only supported on identifiers for now");
                }
            } else if self.current_token == Token::Assign {
                self.advance();
                let rhs = self.parse_expr();
                self.expect(Token::Semicolon);
                if let Expr::Identifier(name) = expr {
                    vec![Stmt::Assign(name, rhs)]
                } else if let Expr::IndexAccess(arr, idx) = expr {
                    vec![Stmt::AssignIndex(arr, idx, rhs)]
                } else if let Expr::FieldAccess(obj, field) = expr {
                    vec![Stmt::AssignField(obj, field, rhs)]
                } else {
                    panic!("= only supported on identifiers, array indexes, and struct fields for now");
                }
            } else {
                self.expect(Token::Semicolon);
                vec![Stmt::ExprStmt(expr)]
            }
        }
    }
    fn op_precedence(op: &Op) -> u8 {
        match op {
            Op::Mul | Op::Div | Op::Rem => 13,
            Op::Add | Op::Sub => 12,
            Op::ShiftLeft | Op::ShiftRight => 11,
            Op::BitAnd => 10,
            Op::BitXor => 9,
            Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => 6,
            Op::EqualEqual | Op::NotEqual => 5,
        }
    }

    pub fn parse_expr(&mut self) -> Expr {
        let start_span = self.current_span;
        let expr = self.parse_expr_impl(0);
        self.register_span(&expr, start_span);
        expr
    }

    fn parse_expr_impl(&mut self, min_prec: u8) -> Expr {
        let start_span = self.current_span;
        let mut left = self.parse_primary();
        while let Some(op) = self.match_op() {
            let prec = Self::op_precedence(&op);
            if prec < min_prec {
                break;
            }
            self.advance();
            // All these operators are left-associative, so we pass prec + 1
            let right = self.parse_expr_impl(prec + 1);
            left = Expr::Binary(Box::new(left), op, Box::new(right));
            self.register_span(&left, start_span);
        }
        left
    }
    fn is_closure_start(&self) -> bool {
        if self.current_token != Token::LParen {
            return false;
        }
        let mut temp_lexer = self.lexer.clone();
        
        let mut depth = 1;
        let mut has_operators_or_literals = false;
        
        loop {
            let tok = temp_lexer.next_token().node;
            match tok {
                Token::LParen => {
                    depth += 1;
                }
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        if has_operators_or_literals {
                            return false;
                        }
                        let next = temp_lexer.next_token().node;
                        return next == Token::Colon || next == Token::FatArrow;
                    }
                }
                Token::Eof => return false,
                Token::Integer(_) | Token::Float(_) | Token::StringLit(_) |
                Token::True | Token::False |
                Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::Percent |
                Token::EqualEqual | Token::NotEqual | Token::Less | Token::LessEqual |
                Token::Greater | Token::GreaterEqual | Token::Assign | Token::PlusAssign => {
                    has_operators_or_literals = true;
                }
                _ => {}
            }
        }
    }

    pub fn parse_primary(&mut self) -> Expr {
        let start_span = self.current_span;
        // Unary minus at the start of a primary: `-x` -> `0 - x`. This lets
        // negative integer literals appear in any expression context,
        // including const initializers.
        if self.current_token == Token::Minus {
            self.advance();
            let inner = self.parse_primary();
            let zero = Expr::Integer("0".to_string());
            let expr = Expr::Binary(Box::new(zero), Op::Sub, Box::new(inner));
            self.register_span(&expr, start_span);
            return expr;
        }
        let mut expr = match &self.current_token {
            Token::If => {
                self.advance();
                if self.current_token == Token::Let {
                    self.advance();
                    let pattern = self.parse_match_pattern();
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    let (then_body, then_val) = self.parse_match_arm_body();
                    let mut else_body = vec![];
                    let mut else_val = None;
                    if self.current_token == Token::Else {
                        self.advance();
                        let (e_body, e_val) = self.parse_match_arm_body();
                        else_body = e_body;
                        else_val = e_val;
                    }
                    let arms = vec![
                        MatchArm { pattern, body: then_body, val: then_val },
                        MatchArm { pattern: MatchPattern::CatchAll, body: else_body, val: else_val },
                    ];
                    Expr::Match(Box::new(expr), arms)
                } else {
                    let cond = self.parse_expr();
                    let (then_body, then_val) = self.parse_match_arm_body();
                    let mut else_block = None;
                    if self.current_token == Token::Else {
                        self.advance();
                        else_block = Some(Box::new(self.parse_match_arm_body()));
                    }
                    Expr::If(Box::new(cond), Box::new((then_body, then_val)), else_block)
                }
            }
            Token::Match => {
                self.advance();
                let cond = self.parse_expr();
                self.expect(Token::LBrace);
                let mut arms = Vec::new();
                while self.current_token != Token::RBrace {
                    let pattern = self.parse_match_pattern();
                    self.expect(Token::FatArrow);
                    let (body, val) = self.parse_match_arm_body();
                    arms.push(MatchArm { pattern, body, val });
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else if self.current_token != Token::RBrace {
                        panic!("Expected ',' after match arm, found {:?}", self.current_token);
                    }
                }
                self.expect(Token::RBrace);
                Expr::Match(Box::new(cond), arms)
            }
            Token::LParen => {
                if self.is_closure_start() {
                    self.expect(Token::LParen);
                    let mut params = Vec::new();
                    while self.current_token != Token::RParen {
                        let param_name = self.current_as_identifier().unwrap_or_else(|| panic!("Expected param name"));
                        self.advance();
                        let mut param_ty = Type::Anyref;
                        let mut is_variadic = false;
                        if self.current_token == Token::Colon {
                            self.advance();
                            if self.current_token == Token::DotDotDot {
                                is_variadic = true;
                                self.advance();
                            } else {
                                param_ty = self.parse_type();
                            }
                        }
                        params.push(Param {
                            name: param_name,
                            ty: param_ty,
                            is_variadic,
                        });
                        if self.current_token == Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen);
                    
                    let mut ret_ty = Type::Void;
                    if self.current_token == Token::Colon {
                        self.advance();
                        ret_ty = self.parse_type();
                    }
                    
                    self.expect(Token::FatArrow);
                    let body = if self.current_token == Token::LBrace {
                        self.advance();
                        let mut body = Vec::new();
                        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                            self.parse_stmt(&mut body);
                        }
                        self.expect(Token::RBrace);
                        body
                    } else {
                        let expr = self.parse_expr();
                        vec![Stmt::Return(Some(expr))]
                    };
                    
                    let func = Function {
                        name: "anonymous".to_string(),
                        params,
                        return_ty: ret_ty,
                        body,
                        is_extern: false,
                        is_compiler: false,
                        is_pub: false,
                        _is_pub: false,
                        _is_static: false,
                        generic: GenericParams { params: vec![] },
                        parent_struct: None, attributes: Vec::new(), }; Expr::Closure(Box::new(func))
                } else {
                    self.advance();
                    let mut exprs = Vec::new();
                    let mut has_comma = false;
                    while self.current_token != Token::RParen {
                        exprs.push(self.parse_expr());
                        if self.current_token == Token::Comma {
                            has_comma = true;
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen);
                    if exprs.len() == 1 && !has_comma {
                        exprs.into_iter().next().unwrap()
                    } else {
                        Expr::Tuple(exprs)
                    }
                }
            }
            Token::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                while self.current_token != Token::RBrace {
                    let key = self.parse_expr();
                    self.expect(Token::Colon);
                    let val = self.parse_expr();
                    pairs.push((key, val));
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else if self.current_token != Token::RBrace {
                        panic!("Expected ',' or '}}' in map literal, found {:?}", self.current_token);
                    }
                }
                self.expect(Token::RBrace);
                Expr::MapLit(pairs)
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while self.current_token != Token::RBracket {
                    elems.push(self.parse_expr());
                    if self.current_token == Token::Comma {
                        self.advance();
                    } else if self.current_token != Token::RBracket {
                        panic!("Expected ',' or ']' in vec literal, found {:?}", self.current_token);
                    }
                }
                self.expect(Token::RBracket);
                Expr::VecLit(elems)
            }
            Token::New => {
                self.advance();
                if self.current_token == Token::LBracket {
                    if !is_system_file() {
                        crate::diagnostics::report_error("Raw array allocations are not allowed outside the standard library and benchmarks".to_string(), Some(self.current_span));
                    }
                    self.advance();
                    if self.current_token == Token::RBracket {
                        self.advance();
                        let inner = self.parse_type();
                        Expr::New(Type::Array(Box::new(inner)), vec![])
                    } else {
                        let len_expr = self.parse_expr();
                        self.expect(Token::RBracket);
                        let inner = self.parse_type();
                        Expr::New(Type::Array(Box::new(inner)), vec![len_expr])
                    }
                } else {
                    panic!("Expected [ after new, found {:?}", self.current_token);
                }
            }
            Token::Identifier(_) => {
                let mut name = self.current_as_identifier().unwrap();
                self.advance();
                while self.current_token == Token::DoubleColon {
                    self.advance();
                    let next_name = self.current_as_identifier().expect("Expected identifier after ::");
                    name = format!("{}::{}", name, next_name);
                    self.advance();
                }
                let is_generic = if self.current_token == Token::Less {
                    let mut temp_lexer = self.lexer.clone();
                    let mut depth = 1;
                    let mut ok = true;
                    loop {
                        let tok = temp_lexer.next_token().node;
                        match tok {
                            Token::Greater => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            Token::ShiftRight => {
                                depth -= 2;
                                if depth <= 0 {
                                    break;
                                }
                            }
                            Token::Less => {
                                depth += 1;
                            }
                            Token::Identifier(_) | Token::Type(_) | Token::DoubleColon | Token::LBracket | Token::RBracket | Token::Comma | Token::Fn | Token::LParen | Token::RParen => {}
                            Token::Eof => {
                                ok = false;
                                break;
                            }
                            _ => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    ok
                } else {
                    false
                };
                if is_generic {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_type());
                        if self.current_token == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(Token::Greater);
                    let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    name = format!("{}<{}>", name, args_str.join(","));
                }
                while self.current_token == Token::DoubleColon {
                    self.advance();
                    let next_name = self.current_as_identifier().expect("Expected identifier after ::");
                    name = format!("{}::{}", name, next_name);
                    self.advance();
                }

                let is_capitalized = name.split("::").last().and_then(|s| s.chars().next()).map(|c| c.is_ascii_uppercase()).unwrap_or(false);
                let is_struct_init = if self.current_token == Token::LBrace && is_capitalized {
                    let mut temp_lexer = self.lexer.clone();
                    let first_tok = temp_lexer.next_token().node;
                    if first_tok == Token::RBrace {
                        true
                    } else if matches!(first_tok, Token::Identifier(_) | Token::Type(_) | Token::New | Token::Map) {
                        let second_tok = temp_lexer.next_token().node;
                        second_tok == Token::Colon
                    } else {
                        false
                    }
                } else {
                    false
                };
                if is_struct_init {
                    self.advance();
                    let mut fields = Vec::new();
                    while self.current_token != Token::RBrace {
                        if self.current_as_identifier().is_none() {
                            panic!("Expected field name in struct init, found token: {:?}", self.current_token);
                        }
                        let field_name = self.current_as_identifier().unwrap();
                        self.advance();
                        self.expect(Token::Colon);
                        let expr = self.parse_expr();
                        fields.push((field_name, expr));
                        if self.current_token == Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBrace);
                    Expr::StructInit(name, fields)
                } else if self.current_token == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current_token != Token::RParen {
                        if self.current_token == Token::DotDotDot {
                            self.advance();
                            let e = self.parse_expr();
                            args.push(Expr::Spread(Box::new(e)));
                        } else {
                            args.push(self.parse_expr());
                        }
                        if self.current_token == Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen);
                    Expr::Call(name, args)
                } else {
                    Expr::Identifier(name)
                }
            }
            Token::Integer(v) => {
                let e = Expr::Integer(v.clone());
                self.advance();
                e
            }
            Token::Float(f) => {
                let e = Expr::Float(*f);
                self.advance();
                e
            }
            Token::StringLit(s) => {
                let e = Expr::StringLit(s.clone());
                self.advance();
                e
            }
            Token::True => {
                self.advance();
                Expr::Bool(true)
            }
            Token::False => {
                self.advance();
                Expr::Bool(false)
            }
            Token::Default => {
                self.advance();
                Expr::Default
            }
            _ => panic!("Expected expression primary, found {:?}", self.current_token),
        };
        self.register_span(&expr, start_span);
        while self.current_token == Token::Dot || self.current_token == Token::LBracket || self.current_token == Token::LParen || self.current_token == Token::As {
            if self.current_token == Token::As {
                self.advance();
                let target_ty = self.parse_type();
                expr = Expr::Cast(Box::new(expr), target_ty);
                self.register_span(&expr, start_span);
                continue;
            }
            if self.current_token == Token::LParen {
                self.advance();
                let mut args = Vec::new();
                while self.current_token != Token::RParen {
                    if self.current_token == Token::DotDotDot {
                        self.advance();
                        let e = self.parse_expr();
                        args.push(Expr::Spread(Box::new(e)));
                    } else {
                        args.push(self.parse_expr());
                    }
                    if self.current_token == Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RParen);
                expr = Expr::InvokeFuncPtr(Box::new(expr), args);
                self.register_span(&expr, start_span);
                continue;
            }
            if self.current_token == Token::LBracket {
                self.advance();
                let index_expr = self.parse_expr();
                self.expect(Token::RBracket);
                expr = Expr::IndexAccess(Box::new(expr), Box::new(index_expr));
                self.register_span(&expr, start_span);
                continue;
            }
            self.advance();
            let method_or_field = match &self.current_token {
                Token::Identifier(n) => n.clone(),
                Token::Integer(val) => val.clone(),
                _ => panic!("Expected identifier or integer after dot, found {:?}", self.current_token),
            };
            self.advance();
            if self.current_token == Token::LParen {
                self.advance();
                let mut args = Vec::new();
                while self.current_token != Token::RParen {
                    if self.current_token == Token::DotDotDot {
                        self.advance();
                        let e = self.parse_expr();
                        args.push(Expr::Spread(Box::new(e)));
                    } else {
                        args.push(self.parse_expr());
                    }
                    if self.current_token == Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RParen);
                expr = Expr::MethodCall(Box::new(expr), method_or_field, args);
            } else {
                expr = Expr::FieldAccess(Box::new(expr), method_or_field);
            }
            self.register_span(&expr, start_span);
        }
        expr
    }
    pub fn match_op(&self) -> Option<Op> {
        match self.current_token {
            Token::Plus => Some(Op::Add),
            Token::Minus => Some(Op::Sub),
            Token::Star => Some(Op::Mul),
            Token::Slash => Some(Op::Div),
            Token::Ampersand => Some(Op::BitAnd),
            Token::Percent => Some(Op::Rem),
            Token::Less => Some(Op::Less),
            Token::LessEqual => Some(Op::LessEqual),
            Token::Greater => Some(Op::Greater),
            Token::GreaterEqual => Some(Op::GreaterEqual),
            Token::EqualEqual => Some(Op::EqualEqual),
            Token::NotEqual => Some(Op::NotEqual),
            Token::ShiftRight => Some(Op::ShiftRight),
            Token::ShiftLeft => Some(Op::ShiftLeft),
            Token::Caret => Some(Op::BitXor),
            _ => None,
        }
    }
}
