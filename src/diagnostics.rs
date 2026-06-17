use crate::ast::Span;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub file_path: Option<String>,
}

thread_local! {
    pub static DIAGNOSTICS: RefCell<Vec<Diagnostic>> = RefCell::new(Vec::new());
    pub static CURRENT_FILE: RefCell<Option<String>> = RefCell::new(None);
    pub static CURRENT_SPAN: RefCell<Option<Span>> = RefCell::new(None);
}

pub fn set_current_file(file: Option<String>) {
    CURRENT_FILE.with(|f| {
        *f.borrow_mut() = file;
    });
}

pub fn set_current_span(span: Option<Span>) {
    CURRENT_SPAN.with(|s| {
        *s.borrow_mut() = span;
    });
}

pub fn get_current_span() -> Option<Span> {
    CURRENT_SPAN.with(|s| *s.borrow())
}

pub fn report_error(msg: String, span: Option<Span>) {
    let file_path = CURRENT_FILE.with(|f| f.borrow().clone());
    let resolved_span = span.or_else(get_current_span);
    DIAGNOSTICS.with(|d| {
        d.borrow_mut().push(Diagnostic {
            message: msg,
            span: resolved_span,
            file_path,
        });
    });
}

pub fn has_errors() -> bool {
    DIAGNOSTICS.with(|d| !d.borrow().is_empty())
}

pub fn clear_diagnostics() {
    DIAGNOSTICS.with(|d| d.borrow_mut().clear());
}

pub fn print_diagnostics() {
    DIAGNOSTICS.with(|diagnostics_cell| {
        let diagnostics = diagnostics_cell.borrow();
        for diag in diagnostics.iter() {
            eprintln!("\x1b[1;31merror\x1b[0m: {}", diag.message);
            if let Some(span) = diag.span {
                if let Some(ref file_path) = diag.file_path {
                    eprintln!(
                        "  \x1b[1;34m-->\x1b[0m {}:{}:{}",
                        file_path, span.line, span.column
                    );

                    // Try to read the file and show the source line with visual indicators
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        let lines: Vec<&str> = content.lines().collect();
                        if span.line > 0 && span.line <= lines.len() {
                            let line_content = lines[span.line - 1];
                            let line_num_str = format!("{}", span.line);
                            let pad = " ".repeat(line_num_str.len());
                            eprintln!("   \x1b[1;34m|\x1b[0m");
                            eprintln!("{} \x1b[1;34m|\x1b[0m {}", line_num_str, line_content);

                            // Visual indicator (carets)
                            let caret_pad = " ".repeat(span.column - 1);
                            let caret_len = if span.end > span.start {
                                let len = span.end - span.start;
                                if len > 0 { len } else { 1 }
                            } else {
                                1
                            };
                            let carets = "^".repeat(caret_len);
                            eprintln!("{} \x1b[1;34m|\x1b[0m {}{}", pad, caret_pad, carets);
                            eprintln!("   \x1b[1;34m|\x1b[0m");
                        }
                    }
                } else {
                    eprintln!(
                        "  \x1b[1;34m-->\x1b[0m Line {}, Column {}",
                        span.line, span.column
                    );
                }
            }
            eprintln!();
        }
    });
}
