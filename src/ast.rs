#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Span { start, end, line, column }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Spanned { node, span }
    }
}

thread_local! {
    pub static SPAN_TABLE: std::cell::RefCell<std::collections::HashMap<usize, Span>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub fn register_span<T>(node: &T, span: Span) {
    let addr = node as *const T as usize;
    SPAN_TABLE.with(|table| {
        table.borrow_mut().insert(addr, span);
    });
}

pub fn get_span<T>(node: &T) -> Option<Span> {
    let addr = node as *const T as usize;
    SPAN_TABLE.with(|table| {
        table.borrow_mut().get(&addr).copied()
    })
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Use,
    As,
    Extern,
    Compiler,
    Fn,
    True,
    False,
    Return,
    Let,
    For,
    In,
    If,
    While,
    Struct,
    Enum,
    Pub,
    Static,
    New,
    Map,
    Trait,
    Impl,
    Const,
    Match,
    Default,
    FatArrow,
    Identifier(String),
    Type(String),
    Integer(String),
    Float(f64),
    StringLit(String),
    DoubleColon,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    Dot,
    DotDot,
    DotDotDot,
    Plus,
    Minus,
    Star,
    Slash,
    Ampersand,
    Percent,
    Assign,
    PlusAssign,
    Less,
    LessEqual,
    Greater,
    Pipe,
    Semicolon,
    Eof,
    Else,
    ShiftRight,
    ShiftLeft,
    EqualEqual,
    NotEqual,
    GreaterEqual,
    Caret,
    Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Str,
    Byte,
    Anyref,
    Externref,
    Void,
    Array(Box<Type>),
    Struct(String, Vec<Type>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>),
    GenericParam(String),
}

impl Type {
    pub fn substitute(&self, generic_name: &str, replacement: &Type) -> Type {
        match self {
            Type::GenericParam(n) if n == generic_name => replacement.clone(),
            Type::Struct(name, args) => {
                if name == generic_name && args.is_empty() {
                    replacement.clone()
                } else {
                    let subbed_args = args.iter().map(|arg| arg.substitute(generic_name, replacement)).collect();
                    Type::Struct(name.clone(), subbed_args)
                }
            }
            Type::Array(inner) => Type::Array(Box::new(inner.substitute(generic_name, replacement))),
            Type::Tuple(elems) => {
                let subbed_elems = elems.iter().map(|el| el.substitute(generic_name, replacement)).collect();
                Type::Tuple(subbed_elems)
            }
            Type::Function(params, ret) => {
                let subbed_params = params.iter().map(|p| p.substitute(generic_name, replacement)).collect();
                let subbed_ret = ret.substitute(generic_name, replacement);
                Type::Function(subbed_params, Box::new(subbed_ret))
            }
            _ => self.clone(),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "str"),
            Type::Byte => write!(f, "byte"),
            Type::Anyref => write!(f, "anyref"),
            Type::Externref => write!(f, "externref"),
            Type::Void => write!(f, "void"),
            Type::Array(inner) => write!(f, "[]{}", inner),
            Type::Struct(name, args) => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    write!(f, "{}<{}>", name, args_str.join(","))
                }
            }
            Type::Tuple(elems) => {
                let elems_str: Vec<String> = elems.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", elems_str.join(","))
            }
            Type::Function(params, ret) => {
                let params_str: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}):{}", params_str.join(","), ret)
            }
            Type::GenericParam(name) => write!(f, "{}", name),
        }
    }
}

struct TypeParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> TypeParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<Type, String> {
        self.skip_whitespace();
        let first = self.chars.peek().cloned();
        match first {
            None => Err("Empty input".to_string()),
            Some('[') => {
                self.chars.next(); // Consume '['
                self.skip_whitespace();
                if self.chars.next() != Some(']') {
                    return Err("Expected ']' in array type".to_string());
                }
                let inner = self.parse()?;
                Ok(Type::Array(Box::new(inner)))
            }
            Some('(') => {
                self.chars.next(); // Consume '('
                let mut params = Vec::new();
                self.skip_whitespace();
                while self.chars.peek() != Some(&')') {
                    params.push(self.parse()?);
                    self.skip_whitespace();
                    if self.chars.peek() == Some(&',') {
                        self.chars.next();
                        self.skip_whitespace();
                    } else if self.chars.peek() != Some(&')') {
                        return Err("Expected ',' or ')' in tuple type".to_string());
                    }
                }
                self.chars.next(); // Consume ')'
                Ok(Type::Tuple(params))
            }
            Some('f') => {
                let mut is_fn = false;
                {
                    let mut probe = self.chars.clone();
                    probe.next(); // 'f'
                    if probe.peek() == Some(&'n') {
                        probe.next();
                        while let Some(&c) = probe.peek() {
                            if c.is_whitespace() { probe.next(); } else { break; }
                        }
                        if probe.peek() == Some(&'(') {
                            is_fn = true;
                        }
                    }
                }
                if is_fn {
                    self.chars.next(); // 'f'
                    self.chars.next(); // 'n'
                    self.skip_whitespace();
                    self.chars.next(); // '('
                    let mut params = Vec::new();
                    self.skip_whitespace();
                    while self.chars.peek() != Some(&')') {
                        params.push(self.parse()?);
                        self.skip_whitespace();
                        if self.chars.peek() == Some(&',') {
                            self.chars.next();
                            self.skip_whitespace();
                        } else if self.chars.peek() != Some(&')') {
                            return Err("Expected ',' or ')' in fn parameters".to_string());
                        }
                    }
                    self.chars.next(); // Consume ')'
                    self.skip_whitespace();
                    let mut ret_ty = Type::Void;
                    if self.chars.peek() == Some(&':') {
                        self.chars.next(); // Consume ':'
                        ret_ty = self.parse()?;
                    }
                    Ok(Type::Function(params, Box::new(ret_ty)))
                } else {
                    self.parse_identifier_or_struct()
                }
            }
            _ => self.parse_identifier_or_struct(),
        }
    }

    fn parse_identifier_or_struct(&mut self) -> Result<Type, String> {
        let mut name = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' || c == ':' {
                name.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(format!("Unexpected character in type parsing: {:?}", self.chars.peek()));
        }
        self.skip_whitespace();
        if self.chars.peek() == Some(&'<') {
            self.chars.next(); // Consume '<'
            let mut args = Vec::new();
            self.skip_whitespace();
            while self.chars.peek() != Some(&'>') {
                args.push(self.parse()?);
                self.skip_whitespace();
                if self.chars.peek() == Some(&',') {
                    self.chars.next();
                    self.skip_whitespace();
                } else if self.chars.peek() != Some(&'>') {
                    return Err("Expected ',' or '>' in generic type arguments".to_string());
                }
            }
            self.chars.next(); // Consume '>'
            Ok(Type::Struct(name, args))
        } else {
            let ty = match name.as_str() {
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
                _ => Type::GenericParam(name),
            };
            Ok(ty)
        }
    }
}

impl std::str::FromStr for Type {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = TypeParser::new(s);
        let parsed = parser.parse()?;
        parser.skip_whitespace();
        if parser.chars.peek().is_some() {
            return Err(format!("Extraneous input starting at {:?}", parser.chars.peek()));
        }
        Ok(parsed)
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug)]
pub struct StructDef {
    pub is_pub: bool,
    pub name: String,
    pub generic: GenericParams,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
    pub is_enum: bool,
    pub variants: Vec<String>,
    pub attributes: Vec<Attribute>,
}

impl Clone for StructDef {
    fn clone(&self) -> Self {
        let cloned = StructDef {
            is_pub: self.is_pub,
            name: self.name.clone(),
            generic: self.generic.clone(),
            fields: self.fields.clone(),
            methods: self.methods.clone(),
            is_enum: self.is_enum,
            variants: self.variants.clone(),
            attributes: self.attributes.clone(),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub constraints: Vec<Type>,
}

#[derive(Debug, Clone, Default)]
pub struct GenericParams {
    pub params: Vec<GenericParam>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub is_variadic: bool,
}

#[derive(Debug)]
pub struct Function {
    pub is_pub: bool,
    pub is_extern: bool,
    pub is_compiler: bool,
    pub _is_pub: bool,
    pub _is_static: bool,
    pub parent_struct: Option<String>,
    pub name: String,
    pub generic: GenericParams,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub body: Vec<Stmt>,
    pub attributes: Vec<Attribute>,
}

impl Clone for Function {
    fn clone(&self) -> Self {
        let cloned = Function {
            is_pub: self.is_pub,
            is_extern: self.is_extern,
            is_compiler: self.is_compiler,
            _is_pub: self._is_pub,
            _is_static: self._is_static,
            parent_struct: self.parent_struct.clone(),
            name: self.name.clone(),
            generic: self.generic.clone(),
            params: self.params.clone(),
            return_ty: self.return_ty.clone(),
            body: self.body.clone(),
            attributes: self.attributes.clone(),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

impl Function {
    pub fn is_variadic(&self) -> bool {
        self.params.last().map(|p| p.is_variadic).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Stmt>,
    pub val: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Some(String),
    None,
    Ok(String),
    Err(String),
    Variant(String, Vec<String>),
    CatchAll,
}

#[derive(Debug)]
pub enum Stmt {
    Let(String, Option<Type>, Expr),
    LetTuple(Vec<(String, Type)>, Expr),
    ExprStmt(Expr),
    Return(Option<Expr>),
    Assign(String, Expr),
    AssignPlus(String, Expr),
    AssignIndex(Box<Expr>, Box<Expr>, Expr),
    AssignField(Box<Expr>, String, Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    For(String, String, Vec<Stmt>),
}

impl Clone for Stmt {
    fn clone(&self) -> Self {
        let cloned = match self {
            Stmt::Let(s, t, e) => Stmt::Let(s.clone(), t.clone(), e.clone()),
            Stmt::LetTuple(v, e) => Stmt::LetTuple(v.clone(), e.clone()),
            Stmt::ExprStmt(e) => Stmt::ExprStmt(e.clone()),
            Stmt::Return(e) => Stmt::Return(e.clone()),
            Stmt::Assign(s, e) => Stmt::Assign(s.clone(), e.clone()),
            Stmt::AssignPlus(s, e) => Stmt::AssignPlus(s.clone(), e.clone()),
            Stmt::AssignIndex(e1, e2, e3) => Stmt::AssignIndex(e1.clone(), e2.clone(), e3.clone()),
            Stmt::AssignField(e1, s, e2) => Stmt::AssignField(e1.clone(), s.clone(), e2.clone()),
            Stmt::If(e, s1, s2) => Stmt::If(e.clone(), s1.clone(), s2.clone()),
            Stmt::While(e, s) => Stmt::While(e.clone(), s.clone()),
            Stmt::For(s1, s2, s) => Stmt::For(s1.clone(), s2.clone(), s.clone()),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug)]
pub enum Expr {
    Identifier(String),
    Integer(String),
    Float(f64),
    Binary(Box<Expr>, Op, Box<Expr>),
    Call(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    FieldAccess(Box<Expr>, String),
    StructInit(String, Vec<(String, Expr)>),
    IndexAccess(Box<Expr>, Box<Expr>),
    New(Type, Vec<Expr>),
    StringLit(String),
    Bool(bool),
    Match(Box<Expr>, Vec<MatchArm>),
    If(Box<Expr>, Box<(Vec<Stmt>, Option<Expr>)>, Option<Box<(Vec<Stmt>, Option<Expr>)>>),
    Default,
    InvokeFuncPtr(Box<Expr>, Vec<Expr>),
    Closure(Box<Function>),
    ClosureInstantiate(String, String, Vec<Expr>), // (func_name, env_struct_name, captured_vars)
    Cast(Box<Expr>, Type),
    Spread(Box<Expr>),
    Tuple(Vec<Expr>),
    MapLit(Vec<(Expr, Expr)>),
}

impl Clone for Expr {
    fn clone(&self) -> Self {
        let cloned = match self {
            Expr::Identifier(s) => Expr::Identifier(s.clone()),
            Expr::Integer(s) => Expr::Integer(s.clone()),
            Expr::Float(f) => Expr::Float(*f),
            Expr::Binary(l, op, r) => Expr::Binary(l.clone(), *op, r.clone()),
            Expr::Call(s, v) => Expr::Call(s.clone(), v.clone()),
            Expr::MethodCall(e, s, v) => Expr::MethodCall(e.clone(), s.clone(), v.clone()),
            Expr::FieldAccess(e, s) => Expr::FieldAccess(e.clone(), s.clone()),
            Expr::StructInit(s, v) => Expr::StructInit(s.clone(), v.clone()),
            Expr::IndexAccess(e1, e2) => Expr::IndexAccess(e1.clone(), e2.clone()),
            Expr::New(t, v) => Expr::New(t.clone(), v.clone()),
            Expr::StringLit(s) => Expr::StringLit(s.clone()),
            Expr::Bool(b) => Expr::Bool(*b),
            Expr::Match(e, v) => Expr::Match(e.clone(), v.clone()),
            Expr::If(e, b1, b2) => Expr::If(e.clone(), b1.clone(), b2.clone()),
            Expr::Default => Expr::Default,
            Expr::InvokeFuncPtr(e, v) => Expr::InvokeFuncPtr(e.clone(), v.clone()),
            Expr::Closure(f) => Expr::Closure(f.clone()),
            Expr::ClosureInstantiate(s1, s2, v) => Expr::ClosureInstantiate(s1.clone(), s2.clone(), v.clone()),
            Expr::Cast(e, t) => Expr::Cast(e.clone(), t.clone()),
            Expr::Spread(e) => Expr::Spread(e.clone()),
            Expr::Tuple(v) => Expr::Tuple(v.clone()),
            Expr::MapLit(v) => Expr::MapLit(v.clone()),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    Rem,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    NotEqual,
    ShiftRight,
    ShiftLeft,
    BitXor,
}

#[derive(Debug)]
pub struct TraitDef {
    pub is_pub: bool,
    pub name: String,
    pub generic: GenericParams,
    pub methods: Vec<Function>,
    pub attributes: Vec<Attribute>,
}

impl Clone for TraitDef {
    fn clone(&self) -> Self {
        let cloned = TraitDef {
            is_pub: self.is_pub,
            name: self.name.clone(),
            generic: self.generic.clone(),
            methods: self.methods.clone(),
            attributes: self.attributes.clone(),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug)]
pub struct ImplDef {
    pub is_pub: bool,
    /// `None` indicates an inherent impl: `impl <Type> { ... }`.
    /// `Some(name)` indicates a trait impl: `impl <Trait> for <Type> { ... }`.
    pub trait_name: Option<Type>,
    pub generic: GenericParams,
    pub target_ty: Type,
    pub methods: Vec<Function>,
    pub attributes: Vec<Attribute>,
}

impl Clone for ImplDef {
    fn clone(&self) -> Self {
        let cloned = ImplDef {
            is_pub: self.is_pub,
            trait_name: self.trait_name.clone(),
            generic: self.generic.clone(),
            target_ty: self.target_ty.clone(),
            methods: self.methods.clone(),
            attributes: self.attributes.clone(),
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug)]
pub struct ConstDef {
    pub is_pub: bool,
    pub name: String,
    pub ty: Type,
    pub value: Expr,
    pub attributes: Vec<Attribute>,
    pub is_mutable: bool,
}

impl Clone for ConstDef {
    fn clone(&self) -> Self {
        let cloned = ConstDef {
            is_pub: self.is_pub,
            name: self.name.clone(),
            ty: self.ty.clone(),
            value: self.value.clone(),
            attributes: self.attributes.clone(),
            is_mutable: self.is_mutable,
        };
        if let Some(span) = get_span(self) {
            register_span(&cloned, span);
        }
        cloned
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Use {
        path: Vec<String>,
        symbols: Vec<(String, Option<String>)>,
    },
    Function(Function),
    Struct(StructDef),
    Trait(TraitDef),
    Impl(ImplDef),
    Const(ConstDef),
}
