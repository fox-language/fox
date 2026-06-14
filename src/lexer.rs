use std::iter::Peekable;
use std::str::Chars;
use crate::ast::{Token, Span, Spanned};

#[derive(Clone)]
pub struct Lexer<'a> {
    pub chars: Peekable<Chars<'a>>,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl<'a> Lexer<'a> {
    pub fn clone_peek(&self) -> Token {
        let mut cloned = Lexer {
            chars: self.chars.clone(),
            offset: self.offset,
            line: self.line,
            column: self.column,
        };
        cloned.next_token().node
    }

    pub fn new(input: &'a str) -> Self {
        Lexer {
            chars: input.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn consume(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    pub fn next_token(&mut self) -> Spanned<Token> {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.consume();
            } else {
                break;
            }
        }
        let start_offset = self.offset;
        let start_line = self.line;
        let start_column = self.column;

        let Some(&c) = self.chars.peek() else {
            let span = Span::new(start_offset, start_offset, start_line, start_column);
            return Spanned::new(Token::Eof, span);
        };
        let token = match c {
            '(' => {
                self.consume();
                Token::LParen
            }
            ')' => {
                self.consume();
                Token::RParen
            }
            '{' => {
                self.consume();
                Token::LBrace
            }
            '}' => {
                self.consume();
                Token::RBrace
            }
            '[' => {
                self.consume();
                Token::LBracket
            }
            ']' => {
                self.consume();
                Token::RBracket
            }
            ':' => {
                self.consume();
                if let Some(&':') = self.chars.peek() {
                    self.consume();
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }
            ',' => {
                self.consume();
                Token::Comma
            }
            '.' => {
                self.consume();
                if let Some(&'.') = self.chars.peek() {
                    self.consume();
                    if let Some(&'.') = self.chars.peek() {
                        self.consume();
                        Token::DotDotDot
                    } else {
                        Token::DotDot
                    }
                } else {
                    Token::Dot
                }
            }
            '-' => {
                self.consume();
                Token::Minus
            }
            '*' => {
                self.consume();
                Token::Star
            }
            '/' => {
                self.consume();
                if let Some(&'/') = self.chars.peek() {
                    self.consume();
                    while let Some(&nc) = self.chars.peek() {
                        if nc == '\n' {
                            break;
                        }
                        self.consume();
                    }
                    return self.next_token();
                } else {
                    Token::Slash
                }
            }
            ';' => {
                self.consume();
                Token::Semicolon
            }
            '&' => {
                self.consume();
                Token::Ampersand
            }
            '%' => {
                self.consume();
                Token::Percent
            }
            '+' => {
                self.consume();
                if let Some(&'=') = self.chars.peek() {
                    self.consume();
                    Token::PlusAssign
                } else {
                    Token::Plus
                }
            }
            '<' => {
                self.consume();
                if let Some(&'=') = self.chars.peek() {
                    self.consume();
                    Token::LessEqual
                } else if let Some(&'<') = self.chars.peek() {
                    self.consume();
                    Token::ShiftLeft
                } else {
                    Token::Less
                }
            }
            '>' => {
                self.consume();
                if let Some(&'=') = self.chars.peek() {
                    self.consume();
                    Token::GreaterEqual
                } else if let Some(&'>') = self.chars.peek() {
                    self.consume();
                    Token::ShiftRight
                } else {
                    Token::Greater
                }
            }
            '|' => {
                self.consume();
                Token::Pipe
            }
            '^' => {
                self.consume();
                Token::Caret
            }
            '#' => {
                self.consume();
                Token::Hash
            }
            '=' => {
                self.consume();
                if let Some(&'=') = self.chars.peek() {
                    self.consume();
                    Token::EqualEqual
                } else if let Some(&'>') = self.chars.peek() {
                    self.consume();
                    Token::FatArrow
                } else {
                    Token::Assign
                }
            }
            '!' => {
                self.consume();
                if let Some(&'=') = self.chars.peek() {
                    self.consume();
                    Token::NotEqual
                } else {
                    panic!("Unexpected !");
                }
            }
            c if c.is_ascii_digit() => {
                let mut num = String::new();
                let mut is_float = false;
                // Peek ahead to disambiguate `1.2` (float) from `1.method()`
                // by counting how many digits follow any `.`. A single `.`
                // followed by a non-digit is a method access, not part of the
                // number.
                let mut saw_dot = false;
                while let Some(&ch) = self.chars.peek() {
                    if ch.is_ascii_digit() {
                        num.push(ch);
                        self.consume();
                        if saw_dot {
                            saw_dot = false;
                        }
                    } else if ch == '.' {
                        if saw_dot {
                            // Second `.` in a row (e.g. `1..`) — stop.
                            break;
                        }
                        // Look at the next-next char without consuming.
                        let mut probe = self.chars.clone();
                        probe.next();
                        match probe.peek().copied() {
                            Some(c) if c.is_ascii_digit() => {
                                is_float = true;
                                saw_dot = true;
                                num.push(ch);
                                self.consume();
                            }
                            _ => {
                                // `.` followed by a non-digit (e.g. `1.abs()`):
                                // don't consume the dot, it's a method access.
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                if is_float {
                    Token::Float(num.parse().unwrap())
                } else {
                    Token::Integer(num)
                }
            }
            '"' => {
                self.consume();
                let mut s = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch == '\\' {
                        self.consume();
                        if let Some(&next) = self.chars.peek() {
                            match next {
                                'n' => { s.push('\n'); self.consume(); }
                                't' => { s.push('\t'); self.consume(); }
                                'r' => { s.push('\r'); self.consume(); }
                                '\\' => { s.push('\\'); self.consume(); }
                                '"' => { s.push('"'); self.consume(); }
                                '0' => { s.push('\0'); self.consume(); }
                                _ => { s.push(ch); s.push(next); self.consume(); }
                            }
                        }
                    } else if ch != '"' {
                        s.push(ch);
                        self.consume();
                    } else {
                        break;
                    }
                }
                self.consume();
                Token::StringLit(s)
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        self.consume();
                    } else {
                        break;
                    }
                }
                match ident.as_str() {
                    "use" => Token::Use,
                    "as" => Token::As,
                    "extern" => Token::Extern,
                    "compiler" => Token::Compiler,
                    "fn" => Token::Fn,
                    "true" => Token::True,
                    "false" => Token::False,
                    "return" => Token::Return,
                    "let" => Token::Let,
                    "const" => Token::Const,
                    "for" => Token::For,
                    "in" => Token::In,
                    "if" => Token::If,
                    "while" => Token::While,
                    "struct" => Token::Struct,
                    "enum" => Token::Enum,
                    "pub" => Token::Pub,
                    "static" => Token::Static,
                    "new" => Token::New,
                    "map" => Token::Map,
                    "trait" => Token::Trait,
                    "impl" => Token::Impl,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "default" => Token::Default,
                    "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "str" | "byte" | "bool" | "anyref" => {
                        Token::Type(ident)
                    }
                    _ => Token::Identifier(ident),
                }
            }
            _ => panic!("Unexpected character: {}", c),
        };
        let span = Span::new(start_offset, self.offset, start_line, start_column);
        Spanned::new(token, span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spans_simple() {
        let source = "let x = 42;";
        let mut lexer = Lexer::new(source);

        let t1 = lexer.next_token();
        assert_eq!(t1.node, Token::Let);
        assert_eq!(t1.span.start, 0);
        assert_eq!(t1.span.end, 3);
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.column, 1);

        let t2 = lexer.next_token();
        assert_eq!(t2.node, Token::Identifier("x".to_string()));
        assert_eq!(t2.span.start, 4);
        assert_eq!(t2.span.end, 5);
        assert_eq!(t2.span.line, 1);
        assert_eq!(t2.span.column, 5);

        let t3 = lexer.next_token();
        assert_eq!(t3.node, Token::Assign);
        assert_eq!(t3.span.start, 6);
        assert_eq!(t3.span.end, 7);
        assert_eq!(t3.span.line, 1);
        assert_eq!(t3.span.column, 7);

        let t4 = lexer.next_token();
        assert_eq!(t4.node, Token::Integer("42".to_string()));
        assert_eq!(t4.span.start, 8);
        assert_eq!(t4.span.end, 10);
        assert_eq!(t4.span.line, 1);
        assert_eq!(t4.span.column, 9);

        let t5 = lexer.next_token();
        assert_eq!(t5.node, Token::Semicolon);
        assert_eq!(t5.span.start, 10);
        assert_eq!(t5.span.end, 11);
        assert_eq!(t5.span.line, 1);
        assert_eq!(t5.span.column, 11);

        let t6 = lexer.next_token();
        assert_eq!(t6.node, Token::Eof);
    }

    #[test]
    fn test_spans_multiline() {
        let source = "let a =\n  12;\nlet b = 3;";
        let mut lexer = Lexer::new(source);

        // "let"
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Let);
        assert_eq!(t.span.start, 0);
        assert_eq!(t.span.end, 3);
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.column, 1);

        // "a"
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Identifier("a".to_string()));
        assert_eq!(t.span.start, 4);
        assert_eq!(t.span.end, 5);
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.column, 5);

        // "="
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Assign);
        assert_eq!(t.span.start, 6);
        assert_eq!(t.span.end, 7);
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.column, 7);

        // "12" on line 2, column 3 (after "\n  ")
        // "let a =\n  12"
        // line 1 has 7 chars + '\n' (byte 7).
        // line 2 starts at byte 8.
        // spaces are at 8 and 9.
        // "12" is at byte 10.
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Integer("12".to_string()));
        assert_eq!(t.span.start, 10);
        assert_eq!(t.span.end, 12);
        assert_eq!(t.span.line, 2);
        assert_eq!(t.span.column, 3);

        // ";"
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Semicolon);
        assert_eq!(t.span.start, 12);
        assert_eq!(t.span.end, 13);
        assert_eq!(t.span.line, 2);
        assert_eq!(t.span.column, 5);

        // "let" on line 3, column 1
        // "let a =\n  12;\nlet b = 3;"
        // line 1: "let a =\n" (8 bytes: 0 to 7)
        // line 2: "  12;\n" (6 bytes: 8 to 13)
        // line 3 starts at byte 14.
        let t = lexer.next_token();
        assert_eq!(t.node, Token::Let);
        assert_eq!(t.span.start, 14);
        assert_eq!(t.span.end, 17);
        assert_eq!(t.span.line, 3);
        assert_eq!(t.span.column, 1);
    }

    #[test]
    fn test_spans_utf8() {
        // "let α = 1;"
        // "α" is a 2-byte char.
        let source = "let α = 1;";
        let mut lexer = Lexer::new(source);

        let t = lexer.next_token(); // let
        assert_eq!(t.node, Token::Let);

        let t = lexer.next_token(); // α
        assert_eq!(t.node, Token::Identifier("α".to_string()));
        assert_eq!(t.span.start, 4);
        assert_eq!(t.span.end, 6); // 4 + 2 bytes
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.column, 5);

        let t = lexer.next_token(); // =
        assert_eq!(t.node, Token::Assign);
        assert_eq!(t.span.start, 7); // 6 + 1 space
        assert_eq!(t.span.end, 8);
        assert_eq!(t.span.line, 1);
        assert_eq!(t.span.column, 7); // column count is in chars: "let " is 4, "α" is 1 char -> next col is 6, space is 6, "=" is 7
    }
}
