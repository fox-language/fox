use std::collections::{HashMap, HashSet};
use crate::ast::*;


pub mod intrinsics;
pub mod js;
pub mod wat;

pub use wat::*;
pub use js::*;
pub use intrinsics::*;
pub use crate::type_checker::{get_expr_type, validate_match_patterns};

thread_local! {
    pub static GLOBAL_CONSTS: std::cell::RefCell<HashMap<String, String>> = std::cell::RefCell::new(HashMap::new());
    pub static GLOBAL_CONST_VALUES: std::cell::RefCell<HashMap<String, f64>> = std::cell::RefCell::new(HashMap::new());
    pub static CURRENT_NAMESPACE: std::cell::RefCell<String> = std::cell::RefCell::new("".to_string());
    pub static IMPORTS_REGISTRY: std::cell::RefCell<HashMap<String, HashSet<String>>> = std::cell::RefCell::new(HashMap::new());
    pub static RENAME_ALIASES: std::cell::RefCell<HashMap<String, String>> = std::cell::RefCell::new(HashMap::new());
    pub static CURRENT_EXPECTED_TYPE: std::cell::RefCell<String> = std::cell::RefCell::new("".to_string());
}

pub fn set_rename_aliases(aliases: HashMap<String, String>) {
    RENAME_ALIASES.with(|r| {
        *r.borrow_mut() = aliases;
    });
}

pub fn set_current_namespace(ns: String) {
    CURRENT_NAMESPACE.with(|c| {
        *c.borrow_mut() = ns;
    });
}


pub fn init_codegen_env(
    imports_registry: HashMap<String, HashSet<String>>,
    rename_aliases: HashMap<String, String>,
) {
    IMPORTS_REGISTRY.with(|r| {
        *r.borrow_mut() = imports_registry;
    });
    RENAME_ALIASES.with(|r| {
        *r.borrow_mut() = rename_aliases;
    });
    CURRENT_NAMESPACE.with(|c| {
        *c.borrow_mut() = "".to_string();
    });
}

pub fn get_namespace(name: &str) -> String {
    if let Some(idx) = name.find("::") {
        name[..idx].to_string()
    } else {
        "".to_string()
    }
}

pub fn resolve_const_name(name: &str) -> Option<String> {
    GLOBAL_CONSTS.with(|gc| {
        let map = gc.borrow();
        if map.contains_key(name) {
            return Some(name.to_string());
        }
        for key in map.keys() {
            if key.ends_with(&format!("::{}", name)) {
                return Some(key.clone());
            }
        }
        None
    })
}

pub fn clean_type(ty: &str, structs: &HashMap<String, StructDef>) -> String {
    if ty.starts_with("[]") {
        return format!("[]{}", clean_type(&ty[2..], structs));
    }
    resolve_struct_name(ty, structs)
}

pub fn canonical_type(ty: &Type) -> Type {
    match ty {
        Type::Struct(name, args) => {
            let stripped_name = name.split("::").last().unwrap_or(name).to_string();
            let canonical_args = args.iter().map(|arg| canonical_type(arg)).collect();
            Type::Struct(stripped_name, canonical_args)
        }
        Type::Array(inner) => Type::Array(Box::new(canonical_type(inner))),
        Type::Tuple(elems) => Type::Tuple(elems.iter().map(|el| canonical_type(el)).collect()),
        Type::Function(params, ret) => {
            Type::Function(
                params.iter().map(|p| canonical_type(p)).collect(),
                Box::new(canonical_type(ret)),
            )
        }
        _ => ty.clone(),
    }
}

pub fn canonical_type_string(ty_str: &str) -> String {
    use std::str::FromStr;
    if let Ok(ty) = Type::from_str(ty_str) {
        let canonical_ty = canonical_type(&ty);
        let s = canonical_ty.to_string();
        s.replace('<', "_").replace('>', "").replace(',', "_").replace(" ", "")
    } else {
        ty_str.to_string()
    }
}

pub fn is_compatible(actual: &str, expected: &str, structs: &HashMap<String, StructDef>) -> bool {
    if expected == "unknown" || actual == "unknown" {
        return true;
    }
    if expected == "anyref" || actual == "anyref" {
        return true;
    }
    if expected == actual {
        return true;
    }
    if actual == "fn(...)" && expected.starts_with("fn(") {
        return true;
    }

    let is_actual_64 = actual == "i64" || actual == "u64";
    let is_expected_64 = expected == "i64" || expected == "u64";
    if is_actual_64 && is_expected_64 {
        return true;
    }

    let is_actual_32 = actual == "i32" || actual == "u32" || actual == "byte" || actual == "bool";
    let is_expected_32 = expected == "i32" || expected == "u32" || expected == "byte" || expected == "bool";
    if is_actual_32 && is_expected_32 {
        return true;
    }

    // Float widening/narrowing is always allowed
    if (actual == "f32" || actual == "f64") && (expected == "f32" || expected == "f64") {
        return true;
    }
    // Integer widening/narrowing is NOT allowed (original emit_widening behavior)
    if (is_expected_64 && is_actual_32) || (is_expected_32 && is_actual_64) {
        return false;
    }

    let canon_expected = canonical_type_string(expected);
    let canon_actual = canonical_type_string(actual);
    if canon_expected == canon_actual {
        return true;
    }

    let resolved_expected = clean_type(expected, structs);
    let resolved_actual = clean_type(actual, structs);
    if clean_type(&resolved_expected, structs) == clean_type(&resolved_actual, structs) {
        return true;
    }
    if canonical_type_string(&resolved_expected) == canonical_type_string(&resolved_actual) {
        return true;
    }

    false
}

pub fn emit_widening(_wat: &mut String, actual_ty: &str, expected_ty: &str, structs: &HashMap<String, StructDef>) {
    if !is_compatible(actual_ty, expected_ty, structs) {
        crate::diagnostics::report_error(format!("Type mismatch: expected '{}', found '{}'", expected_ty, actual_ty), None);
    }
}

pub fn eval_const_val(expr: &Expr) -> f64 {
    match expr {
        Expr::Float(f) => *f,
        Expr::Integer(s) => s.parse::<f64>().unwrap_or(0.0),
        Expr::Bool(b) => if *b { 1.0 } else { 0.0 },
        Expr::Binary(left, op, right) => {
            let l = eval_const_val(left);
            let r = eval_const_val(right);
            match op {
                Op::Add => l + r,
                Op::Sub => l - r,
                Op::Mul => l * r,
                Op::Div => l / r,
                Op::Rem => l % r,
                _ => panic!("Unsupported operator in constant expression: {:?}", op),
            }
        }
        Expr::Identifier(n) => {
            if let Some(resolved) = resolve_const_name(n) {
                GLOBAL_CONST_VALUES.with(|gcv| {
                    gcv.borrow().get(&resolved).cloned().unwrap_or(0.0)
                })
            } else {
                panic!("Unknown constant identifier: {}", n);
            }
        }
        _ => panic!("Unsupported expression in constant expression: {:?}", expr),
    }
}

pub fn eval_const_expr(expr: &Expr, ty: &str) -> String {
    if ty == "i64" {
        if let Expr::Integer(s) = expr {
            if let Ok(u) = s.parse::<u64>() {
                return format!("i64.const {}", u as i64);
            }
            if let Ok(i) = s.parse::<i64>() {
                return format!("i64.const {}", i);
            }
        }
    } else if ty == "i32" || ty == "u32" || ty == "byte" || ty == "bool" {
        if let Expr::Integer(s) = expr {
            if let Ok(u) = s.parse::<u32>() {
                return format!("i32.const {}", u as i32);
            }
            if let Ok(i) = s.parse::<i32>() {
                return format!("i32.const {}", i);
            }
        }
    }

    let val = eval_const_val(expr);
    if ty == "f64" {
        format!("f64.const {}", val)
    } else if ty == "f32" {
        format!("f32.const {}", val)
    } else if ty == "i64" {
        format!("i64.const {}", val as i64)
    } else {
        format!("i32.const {}", val as i32)
    }
}

pub fn is_const_expr(expr: &Expr, consts_map: &HashMap<String, ConstDef>) -> bool {
    match expr {
        Expr::Integer(_) | Expr::Float(_) | Expr::Bool(_) => true,
        Expr::Binary(l, op, r) => {
            matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div)
                && is_const_expr(l, consts_map)
                && is_const_expr(r, consts_map)
        }
        Expr::Identifier(n) => {
            if let Some(resolved) = resolve_const_name(n) {
                if let Some(c) = consts_map.get(&resolved) {
                    return !c.is_mutable;
                }
            }
            false
        }
        _ => false,
    }
}

pub fn get_wasm_default_const(wasm_ty: &str) -> String {
    if wasm_ty == "i32" {
        "i32.const 0".to_string()
    } else if wasm_ty == "i64" {
        "i64.const 0".to_string()
    } else if wasm_ty == "f32" {
        "f32.const 0.0".to_string()
    } else if wasm_ty == "f64" {
        "f64.const 0.0".to_string()
    } else if wasm_ty == "externref" {
        "ref.null extern".to_string()
    } else if wasm_ty == "(ref null any)" {
        "ref.null any".to_string()
    } else if wasm_ty.starts_with("(ref null $") && wasm_ty.ends_with(')') {
        let heap_ty = &wasm_ty[11..wasm_ty.len() - 1];
        format!("ref.null ${}", heap_ty)
    } else {
        panic!("Unknown wasm type for default: {}", wasm_ty);
    }
}




pub fn sanitize_name(s: &str) -> String {
    s.replace("<", "_")
     .replace(">", "_")
     .replace(",", "_")
     .replace("[", "Slice_")
     .replace("]", "_")
     .replace("(", "_")
     .replace(")", "_")
     .replace(" ", "")
}

pub fn strip_mangled_namespaces(s: &str) -> String {
    let segments: Vec<&str> = s.split(|c| c == ':' || c == '_').filter(|seg| !seg.is_empty()).collect();
    let mut kept = Vec::new();
    for seg in segments {
        let first_char = seg.chars().next();
        let is_uppercase = first_char.map(|c| c.is_ascii_uppercase()).unwrap_or(false);
        let is_primitive = matches!(seg, "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "str" | "void" | "byte" | "bool" | "anyref" | "externref" | "tuple" | "Slice");
        if is_uppercase || is_primitive {
            kept.push(seg);
        }
    }
    kept.join("_")
}

pub fn resolve_struct_name(name: &str, structs: &HashMap<String, StructDef>) -> String {
    let normalized_name = if let Some(start) = name.find('<') {
        if let Some(end) = name.rfind('>') {
            let base = &name[..start];
            let args_str = &name[start + 1..end];
            let args = split_types(args_str);
            let mut mono = base.to_string();
            for arg in &args {
                mono.push('_');
                let resolved_arg = resolve_struct_name(arg, structs);
                mono.push_str(&resolved_arg.replace("::", "_"));
            }
            sanitize_name(&mono)
        } else {
            name.to_string()
        }
    } else {
        name.to_string()
    };

    if structs.contains_key(&normalized_name) {
        return normalized_name;
    }

    for key in structs.keys() {
        let stripped_key = strip_mangled_namespaces(key);
        let stripped_norm = strip_mangled_namespaces(&normalized_name);
        if stripped_key == stripped_norm {
            return key.clone();
        }
    }

    let base_name_str = if let Some(start) = name.find('<') {
        let end = name.rfind('>').unwrap_or(start);
        let mut b = name[..start].to_string();
        if end + 1 < name.len() {
            b.push_str(&name[end + 1..]);
        }
        b
    } else {
        name.to_string()
    };
    let base_name = &base_name_str;
    let current_ns = CURRENT_NAMESPACE.with(|c| c.borrow().clone());
    
    let mut possible_matches = Vec::new();
    for key in structs.keys() {
        if key.ends_with(&format!("::{}", normalized_name)) || normalized_name.ends_with(&format!("::{}", key)) {
            let target_ns = get_namespace(key);
            if target_ns == current_ns && current_ns != "" {
                return key.clone();
            }
            let is_imported = IMPORTS_REGISTRY.with(|reg| {
                let r = reg.borrow();
                if let Some(imported) = r.get(&current_ns) {
                    let root = base_name.split("::").next().unwrap_or(base_name);
                    imported.contains(root)
                } else {
                    false
                }
            });
            if is_imported {
                return key.clone();
            }
            possible_matches.push(key.clone());
        }
    }
    if possible_matches.len() == 1 {
        return possible_matches[0].clone();
    }

    // Fallback: when generic params make the mangled name unmatchable,
    // try matching by base name (without generic args).
    // e.g. "vec::Vec<fn():void>" should resolve to "vec::Vec"
    if name.contains('<') {
        let mut base_possible = Vec::new();
        for key in structs.keys() {
            if key == base_name || key.ends_with(&format!("::{}", base_name)) || base_name.ends_with(&format!("::{}", key)) {
                base_possible.push(key.clone());
            }
        }
        if base_possible.len() == 1 {
            return base_possible[0].clone();
        }
        if !base_possible.is_empty() {
            let target_ns = get_namespace(&base_possible[0]);
            if target_ns == current_ns && current_ns != "" {
                return base_possible[0].clone();
            }
            let is_imported = IMPORTS_REGISTRY.with(|reg| {
                let r = reg.borrow();
                if let Some(imported) = r.get(&current_ns) {
                    let root = base_name.split("::").next().unwrap_or(base_name);
                    imported.contains(root)
                } else {
                    false
                }
            });
            if is_imported {
                return base_possible[0].clone();
            }
        }
    }

    normalized_name
}

pub fn resolve_func_name(name: &str, arg_types: &[String], funcs: &HashMap<String, Function>, structs: &HashMap<String, StructDef>) -> String {
    resolve_func_name_impl(name, arg_types, "unknown", funcs, structs)
}

pub fn resolve_func_name_with_expected(
    name: &str,
    arg_types: &[String],
    expected_ret_ty: &str,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> String {
    resolve_func_name_impl(name, arg_types, expected_ret_ty, funcs, structs)
}

fn func_args_compatible(func: &Function, arg_types: &[String]) -> bool {
    if arg_types.is_empty() {
        return true;
    }
    let params = if !func.params.is_empty() && func.params[0].name == "self" {
        &func.params[1..]
    } else {
        &func.params
    };
    let is_variadic = func.is_variadic();
    let fixed_params = if is_variadic {
        &params[..params.len().saturating_sub(1)]
    } else {
        params
    };
    if arg_types.len() < fixed_params.len() {
        return false;
    }
    fixed_params.iter().zip(arg_types.iter()).all(|(p, a)| p.ty.to_string() == *a || p.ty.to_string() == "unknown" || a == "unknown")
}

fn pick_best_candidate<'a>(
    candidates: Vec<&'a String>,
    arg_types: &[String],
    expected_ret_ty: &str,
    funcs: &'a HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> Option<&'a String> {
    if candidates.is_empty() {
        return None;
    }
    let mut compatible: Vec<&String> = candidates.iter().cloned().filter(|k| {
        if let Some(f) = funcs.get(*k) {
            if !func_args_compatible(f, arg_types) {
                return false;
            }
            if expected_ret_ty != "" && expected_ret_ty != "unknown" {
                let f_ret_resolved = resolve_struct_name(&f.return_ty.to_string(), structs);
                let expected_resolved = resolve_struct_name(expected_ret_ty, structs);
                if f_ret_resolved != expected_resolved {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }).collect();
    
    if compatible.is_empty() {
        compatible = candidates.iter().cloned().filter(|k| {
            funcs.get(*k).map(|f| func_args_compatible(f, arg_types)).unwrap_or(false)
        }).collect();
    }
    
    if compatible.is_empty() {
        compatible = candidates;
    }
    
    compatible.sort_by_key(|k| k.split("::").count());
    compatible.first().cloned()
}

pub fn resolve_func_name_impl(
    name: &str,
    arg_types: &[String],
    expected_ret_ty: &str,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> String {
    let mut resolved_name_str = name.to_string();
    let mut last_colon_idx = None;
    let mut colon_depth = 0;
    let colon_chars: Vec<char> = name.chars().collect();
    let mut colon_i = 0;
    while colon_i < colon_chars.len() {
        if colon_chars[colon_i] == '<' {
            colon_depth += 1;
        } else if colon_chars[colon_i] == '>' {
            colon_depth -= 1;
        } else if colon_chars[colon_i] == ':' && colon_i + 1 < colon_chars.len() && colon_chars[colon_i+1] == ':' && colon_depth == 0 {
            last_colon_idx = Some(colon_i);
            colon_i += 1;
        }
        colon_i += 1;
    }
    if let Some(idx) = last_colon_idx {
        let struct_part = &name[..idx];
        let method_part = &name[idx + 2..];
        let resolved_struct = resolve_struct_name(struct_part, structs);
        resolved_name_str = format!("{}::{}", resolved_struct, method_part);
        if funcs.contains_key(&resolved_name_str) {
            return resolved_name_str;
        }
        // Fallback: match by parent_struct + method name
        // e.g. "vec::Vec::new" should find "vec::Vec<T>::new"
        for (k, f) in funcs {
            if let Some(ref ps) = f.parent_struct {
                if ps == &resolved_struct && k.ends_with(&format!("::{}", method_part)) {
                    return k.clone();
                }
            }
        }
    }

    let mut explicit_args = Vec::new();
    let base_name_str = if let Some(start) = resolved_name_str.find('<') {
        let end = resolved_name_str.rfind('>').unwrap_or(start);
        
        let args_str = &resolved_name_str[start + 1..end];
        let mut depth = 0;
        let mut current = String::new();
        for c in args_str.chars() {
            if c == '<' { depth += 1; current.push(c); }
            else if c == '>' { depth -= 1; current.push(c); }
            else if c == ',' && depth == 0 { explicit_args.push(current.trim().to_string()); current.clear(); }
            else { current.push(c); }
        }
        if !current.is_empty() { explicit_args.push(current.trim().to_string()); }

        let mut b = resolved_name_str[..start].to_string();
        if end + 1 < resolved_name_str.len() {
            b.push_str(&resolved_name_str[end + 1..]);
        }
        b
    } else {
        resolved_name_str.clone()
    };
    let base_name = &base_name_str;
    let current_ns = CURRENT_NAMESPACE.with(|c| c.borrow().clone());

    let mut template_func = funcs.get(base_name);
    if template_func.is_none() {
        let mut candidates = Vec::new();
        for (k, _v) in funcs {
            if k.ends_with(&format!("::{}", base_name)) || base_name.ends_with(&format!("::{}", k)) {
                let target_ns = get_namespace(k);
                if target_ns == current_ns || current_ns == "" {
                    candidates.push(k);
                } else {
                    let is_imported = IMPORTS_REGISTRY.with(|reg| {
                        let r = reg.borrow();
                        if let Some(imported) = r.get(&current_ns) {
                            let root = base_name.split("::").next().unwrap_or(base_name);
                            imported.contains(root)
                        } else {
                            false
                        }
                    });
                    if is_imported {
                        candidates.push(k);
                    }
                }
            }
        }
        if !candidates.is_empty() {
            if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
                template_func = funcs.get(best);
            }
        }
    }

    // Try progressively stripping leading namespace segments from base_name.
    // e.g. "std::encoding::json::JsonScanner::new" -> "encoding::json::JsonScanner::new" -> "json::JsonScanner::new"
    // This resolves fully-qualified import paths (std::encoding::json) that map to
    // file-stem-based internal names (json::JsonScanner::new).
    // No namespace filtering: the remaining prefix (e.g. "json::") already disambiguates.
    if template_func.is_none() && base_name.contains("::") {
        let mut shortened = base_name.to_string();
        while let Some(pos) = shortened.find("::") {
            shortened = shortened[pos + 2..].to_string();
            if shortened.is_empty() { break; }
            let mut candidates: Vec<&String> = Vec::new();
            for k in funcs.keys() {
                if k.ends_with(&format!("::{}", shortened)) || shortened == k.as_str() {
                    candidates.push(k);
                }
            }
            if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
                template_func = funcs.get(best);
                break;
            }
        }
    }

    // Fallback: when the qualified name (e.g. fmt::sprintf) doesn't match any key,
    // try matching by just the function name (last segment). This handles cases like
    // `use std::fmt;` + `fmt::sprintf()` where the actual function is `sprintf::sprintf`
    // (namespaced by file stem, not directory name).
    if template_func.is_none() && base_name.contains("::") {
        let func_name = base_name.rsplit("::").next().unwrap_or(base_name);
        let mut candidates: Vec<&String> = Vec::new();
        for k in funcs.keys() {
            if k.ends_with(&format!("::{}", func_name)) {
                candidates.push(k);
            }
        }
        if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
            template_func = funcs.get(best);
        }
    }

    // Check rename aliases: if the name was imported as `original as alias`,
    // look up the original function name and retry resolution.
    if template_func.is_none() {
        let original_name = RENAME_ALIASES.with(|r| r.borrow().get(base_name).cloned());
        if let Some(orig) = original_name {
            template_func = funcs.get(&orig);
            if template_func.is_none() {
                for (k, v) in funcs {
                    if k.ends_with(&format!("::{}", orig)) {
                        template_func = Some(v);
                        break;
                    }
                }
            }
            if let Some(f) = template_func {
                if f.generic.params.is_empty() {
                    if funcs.contains_key(&orig) {
                        return orig;
                    }
                    for key in funcs.keys() {
                        if key.ends_with(&format!("::{}", orig)) {
                            return key.clone();
                        }
                    }
                }
            }
        }
    }

    if let Some(f) = template_func {
        if f.generic.params.is_empty() {
            // It's a non-generic function, so return its full name immediately
            if funcs.contains_key(base_name) {
                return base_name.to_string();
            }
            let mut candidates: Vec<&String> = Vec::new();
            for key in funcs.keys() {
                if key.ends_with(&format!("::{}", base_name)) {
                    candidates.push(key);
                }
            }
            if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
                return best.clone();
            }
            // Try progressively stripping leading namespace segments
            // e.g. "std::encoding::json::JsonScanner::new" -> "json::JsonScanner::new"
            {
                let mut shortened = base_name.to_string();
                while let Some(pos) = shortened.find("::") {
                    shortened = shortened[pos + 2..].to_string();
                    if shortened.is_empty() { break; }
                    if funcs.contains_key(&shortened) {
                        return shortened;
                    }
                    let mut candidates: Vec<&String> = Vec::new();
                    for key in funcs.keys() {
                        if key.ends_with(&format!("::{}", shortened)) {
                            candidates.push(key);
                        }
                    }
                    if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
                        return best.clone();
                    }
                }
            }
            // Fallback: if the qualified name didn't match, try by function name only
            if base_name.contains("::") {
                let func_name = base_name.rsplit("::").next().unwrap_or(base_name);
                let mut candidates: Vec<&String> = Vec::new();
                for key in funcs.keys() {
                    if key.ends_with(&format!("::{}", func_name)) {
                        candidates.push(key);
                    }
                }
                if let Some(best) = pick_best_candidate(candidates, arg_types, expected_ret_ty, funcs, structs) {
                    return best.clone();
                }
            }
        }
    }

    let mono_args = if !explicit_args.is_empty() {
        explicit_args
    } else {
        arg_types.to_vec()
    };

    if !mono_args.is_empty() {
        for i in (1..=mono_args.len()).rev() {
            let partial_args = &mono_args[..i];
            let suffix = partial_args.iter().map(|s| sanitize_wat_name(s)).collect::<Vec<_>>().join("_");
            let mono_names = if let Some(ref tf) = template_func {
                if tf.parent_struct.is_some() {
                    let mut parts: Vec<&str> = tf.name.split("::").collect();
                    let last = parts.pop().unwrap();
                    let struct_part = parts.join("::");
                    vec![format!("{}_{}::{}", struct_part, suffix, last)]
                } else {
                    vec![format!("{}_{}", tf.name, suffix)]
                }
            } else {
                if base_name.contains("::") {
                    let mut parts: Vec<&str> = base_name.split("::").collect();
                    let last = parts.pop().unwrap();
                    let struct_part = parts.join("::");
                    vec![
                        format!("{}_{}::{}", struct_part, suffix, last),
                        format!("{}_{}", base_name, suffix),
                    ]
                } else {
                    vec![format!("{}_{}", base_name, suffix)]
                }
            };
            
            for mono_name in &mono_names {
                if funcs.contains_key(mono_name) {
                    return mono_name.clone();
                }
                for key in funcs.keys() {
                    if key.ends_with(&format!("::{}", mono_name)) {
                        let target_ns = get_namespace(key);
                        if target_ns == current_ns || current_ns == "" {
                            return key.clone();
                        }
                        let is_imported = IMPORTS_REGISTRY.with(|reg| {
                            let r = reg.borrow();
                            if let Some(imported) = r.get(&current_ns) {
                                let root = base_name.split("::").next().unwrap_or(base_name);
                                imported.contains(root)
                            } else {
                                false
                            }
                        });
                        if is_imported {
                            return key.clone();
                        }
                    }
                }
                
                // Try progressive namespace stripping from mono_name
                let mut shortened = mono_name.clone();
                while let Some(pos) = shortened.find("::") {
                    shortened = shortened[pos + 2..].to_string();
                    if shortened.is_empty() { break; }
                    for key in funcs.keys() {
                        if key.ends_with(&format!("::{}", shortened)) || key == &shortened {
                            let target_ns = get_namespace(key);
                            if target_ns == current_ns || current_ns == "" {
                                return key.clone();
                            }
                            let is_imported = IMPORTS_REGISTRY.with(|reg| {
                                let r = reg.borrow();
                                if let Some(imported) = r.get(&current_ns) {
                                    let root = base_name.split("::").next().unwrap_or(base_name);
                                    imported.contains(root)
                                } else {
                                    false
                                }
                            });
                            if is_imported {
                                return key.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    let mut candidates: Vec<&String> = Vec::new();
    let mut candidate_ns_match: Vec<&String> = Vec::new();
    let mut candidate_imported: Vec<&String> = Vec::new();
    for key in funcs.keys() {
        if key.ends_with(&format!("::{}", base_name)) {
            let target_ns = get_namespace(key);
            if target_ns == current_ns || current_ns == "" {
                candidate_ns_match.push(key);
            } else {
                let is_imported = IMPORTS_REGISTRY.with(|reg| {
                    let r = reg.borrow();
                    if let Some(imported) = r.get(&current_ns) {
                        let root = base_name.split("::").next().unwrap_or(base_name);
                        imported.contains(root)
                    } else {
                        false
                    }
                });
                if is_imported {
                    candidate_imported.push(key);
                }
            }
            candidates.push(key);
        }
    }
    let all_candidates = candidate_ns_match.into_iter().chain(candidate_imported.into_iter()).collect::<Vec<_>>();
    if let Some(best) = pick_best_candidate(all_candidates, arg_types, expected_ret_ty, funcs, structs) {
        return best.clone();
    }
    name.to_string()
}

pub fn map_wasm_ty(ty: &str, structs: &HashMap<String, StructDef>) -> String {
    if ty == "str" {
        "externref".to_string()
    } else if ty == "anyref" {
        "(ref null any)".to_string()
    } else if ty.starts_with("fn(") {
        format!("(ref null ${})", fn_type_to_wasm_name(ty))
    } else if ty.starts_with("[]") {
        let inner = &ty[2..];
        let resolved_inner = resolve_struct_name(inner, structs);
        format!("(ref null $array_{})", sanitize_wat_name(&resolved_inner))
    } else {
        let resolved = resolve_struct_name(ty, structs);
        if structs.contains_key(&resolved) {
            format!("(ref null ${})", sanitize_wat_name(&resolved))
        } else if ty == "u32" || ty == "byte" || ty == "bool" {
            "i32".to_string()
        } else if ty == "u64" {
            "i64".to_string()
        } else {
            ty.to_string()
        }
    }
}
pub fn sanitize_wat_name(name: &str) -> String {
    if name.starts_with('(') && name.ends_with(')') {
        let content = &name[1..name.len() - 1];
        let sanitized = sanitize_wat_name(content);
        return format!("tuple_{}", sanitized);
    }
    let step1 = name.replace("::", "_");
    let mut safe_name = String::new();
    for c in step1.chars() {
        if c.is_alphanumeric() || c == '_' {
            safe_name.push(c);
        } else {
            safe_name.push('_');
        }
    }
    safe_name
}

pub fn shorten_import_name(name: &str) -> String {
    if name.starts_with("__fox_") {
        match name {
            "__fox_panic" => "f_p",
            "__fox_str_starts_with" => "f_ss",
            "__fox_str_ends_with" => "f_se",
            "__fox_str_contains" => "f_sc",
            "__fox_str_index_of" => "f_si",
            "__fox_str_last_index_of" => "f_sl",
            "__fox_str_is_empty" => "f_semp",
            "__fox_str_eq" => "f_seq",
            "__fox_str_join" => "f_sjn",
            "__fox_str_compare" => "f_scmp",
            "__fox_dom_is_null" => "f_dn",
            "__fox_dom_is_null_str" => "f_dns",
            "__fox_dom_element_append_child" => "f_dac",
            "__fox_dom_element_set_attribute" => "f_dsa",
            "__fox_dom_element_get_attribute" => "f_dga",
            "__fox_dom_element_remove_attribute" => "f_dra",
            "__fox_dom_element_set_text_content" => "f_dst",
            "__fox_dom_element_get_text_content" => "f_dgt",
            "__fox_dom_element_add_click_listener" => "f_dcl",
            "__fox_dom_document_query_selector" => "f_dqs",
            "__fox_dom_document_create_element" => "f_dce",
            "__fox_dom_console" => "f_con",
            "__fox_dom_performance_now" => "f_dpn",
            "__fox_time_now" => "f_tn",
            "__fox_time_local_offset" => "f_tlo",
            "__fox_f64_to_str" => "f_f2s",
            "__fox_i32_to_str" => "f_i2s",
            "__fox_i64_to_str" => "f_l2s",
            "__fox_json_parse_int" => "f_jpi",
            "__fox_json_parse_float" => "f_jpf",
            "__fox_json_parse_string" => "f_jps",
            "__fox_json_encode_string" => "f_jes",
            "__fox_str_substring" => "f_sub",
            "__fox_http_send" => "f_hs",
            "__fox_http_get_status" => "f_hgs",
            "__fox_http_get_status_text" => "f_hgt",
            "__fox_http_get_body" => "f_hgb",
            "__fox_async_yield" => "f_ay",
            "__fox_async_sleep" => "f_as",
            "__fox_queue_task" => "f_qt",
            _ => name,
        }
        .to_string()
    } else {
        name.to_string()
    }
}

/// Strip the rightmost `namespace::` prefix from a type name, if present.
/// This normalizes `option::Option_fn():void` → `Option_fn():void`.
fn strip_namespace(name: &str) -> String {
    if let Some(idx) = name.rfind("::") {
        name[idx + 2..].to_string()
    } else {
        name.to_string()
    }
}

/// Normalize a function type string `fn(X):Y` by stripping namespace prefixes
/// and replacing `<>` with `_` in params and return type. This ensures that
/// equivalent function types expressed with different conventions
/// (e.g. raw `Option<fn():void>` vs resolved `option::Option_fn():void`)
/// produce the same canonical key.
fn normalize_fn_ty(fn_ty: &str) -> String {
    if !fn_ty.starts_with("fn(") {
        return fn_ty.to_string();
    }
    let inner = &fn_ty[3..];
    // Find the "):" separator at paren depth 0
    let mut depth = 1;
    let mut colon_pos = None;
    for (i, c) in inner.char_indices() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                if inner.chars().nth(i + 1) == Some(':') {
                    colon_pos = Some(i);
                    break;
                }
            }
        }
    }
    if let Some(colon) = colon_pos {
        let params_str = &inner[..colon];
        let ret_ty = &inner[colon + 2..];
        let normalized_params: Vec<String> = if params_str.is_empty() {
            vec![]
        } else {
            params_str.split(',').map(|p| normalize_type_name(p.trim())).collect()
        };
        let normalized_ret = normalize_type_name(ret_ty);
        format!("fn({}):{}", normalized_params.join(","), normalized_ret)
    } else {
        fn_ty.to_string()
    }
}

/// Strip namespace prefix and replace `<` with `_` (removing `>` without adding
/// a trailing underscore), matching `resolve_struct_name`'s convention.
fn normalize_type_name(name: &str) -> String {
    let stripped = strip_namespace(name);
    stripped.replace('<', "_").replace('>', "")
}

pub fn fn_type_to_wasm_name(fn_ty: &str) -> String {
    let normalized = normalize_fn_ty(fn_ty);
    format!("fat_{}", sanitize_wat_name(&normalized))
}

/// Extract the return type from a function type string like `fn(params):ret`.
/// Correctly handles nested function types (e.g. `fn():Option<fn():void>`).
pub fn extract_fn_return_type(fn_ty: &str) -> &str {
    if !fn_ty.starts_with("fn(") {
        return "void";
    }
    let mut depth = 0;
    for (i, c) in fn_ty.chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    if fn_ty.get(i+1..i+2) == Some(":") {
                        return &fn_ty[i+2..];
                    }
                    break;
                }
            }
            _ => {}
        }
    }
    "void"
}

/// Description of an inherent method on a builtin type (`str`, `f64`, `f32`,
/// `i32`, `i64`, `[]T`) declared in `std::builtin`. The body is provided
/// either by emitting a single Wasm opcode or by calling a Wasm import.








/// Emit code to box a value to anyref. The value to be boxed must already be
/// on the stack; this just appends the boxing instructions.

/// Emit a runtime helper that converts an i32 to its decimal string
/// representation. Handles negative numbers and zero.

/// Emit a runtime helper that builds a 4-char string from 4 char codes.

/// Emit the WAT prelude of helper functions used by sprintf. This is
/// emitted at the top of sprintf's body.

/// Emit the helper functions as top-level module functions. Each
/// function is a complete `(func ...)` definition.

/// Emit the body of sprintf: a Wasm implementation that parses the format
/// string, dispatches on arg type via ref.test, and builds the result using
/// wasm:js-string builtins.










fn infer_expr_type(expr: &Expr, env: &HashMap<String, String>) -> String {
    match expr {
        Expr::Identifier(n) => {
            if let Some(t) = env.get(n) {
                t.clone()
            } else if let Some(resolved) = resolve_const_name(n) {
                GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                })
            } else {
                "i32".to_string()
            }
        }
        Expr::Integer(_) => "i32".to_string(),
        Expr::Float(_) => "f32".to_string(),
        Expr::StringLit(_) => "str".to_string(),
        Expr::Bool(_) => "bool".to_string(),
        Expr::New(ty, _) => ty.to_string(),
        Expr::StructInit(ty, _) => ty.clone(),
        Expr::MapLit(pairs) => {
            if pairs.is_empty() {
                "Map<str, anyref>".to_string()
            } else {
                let k_ty = infer_expr_type(&pairs[0].0, env);
                let first_v_ty = infer_expr_type(&pairs[0].1, env);
                let mut v_ty = first_v_ty;
                for (_, v) in pairs.iter().skip(1) {
                    let cur_v_ty = infer_expr_type(v, env);
                    if cur_v_ty != v_ty {
                        v_ty = "anyref".to_string();
                        break;
                    }
                }
                format!("Map<{}, {}>", k_ty, v_ty)
            }
        }
        Expr::MethodCall(obj, method, _) => {
            let obj_ty = infer_expr_type(obj, env);
            if obj_ty.starts_with("Map<") {
                if method == "get" {
                    if let Some(start) = obj_ty.find(',') {
                        if let Some(end) = obj_ty.find('>') {
                            let v_ty = obj_ty[start+1..end].trim().to_string();
                            return format!("Option<{}>", v_ty);
                        }
                    }
                }
                if method == "iter" {
                    if let Some(start) = obj_ty.find('<') {
                        if let Some(end) = obj_ty.rfind('>') {
                            let inner = &obj_ty[start+1..end];
                            return format!("MapIterator<{}>", inner);
                        }
                    }
                }
            }
            if obj_ty.starts_with("Vec<") {
                if method == "iter" {
                    if let Some(start) = obj_ty.find('<') {
                        if let Some(end) = obj_ty.rfind('>') {
                            let inner = &obj_ty[start+1..end];
                            return format!("VecIterator<{}>", inner);
                        }
                    }
                }
            }
            if obj_ty.starts_with("Set<") {
                if method == "iter" {
                    if let Some(start) = obj_ty.find('<') {
                        if let Some(end) = obj_ty.rfind('>') {
                            let inner = &obj_ty[start+1..end];
                            return format!("SetIterator<{}>", inner);
                        }
                    }
                }
            }
            if obj_ty.starts_with("MapIterator<") {
                if method == "next" {
                    if let Some(start) = obj_ty.find('<') {
                        if let Some(end) = obj_ty.rfind('>') {
                            let inner = &obj_ty[start+1..end];
                            return format!("Option<({})>", inner);
                        }
                    }
                }
            }
            if obj_ty.starts_with("VecIterator<") || obj_ty.starts_with("SetIterator<") {
                if method == "next" {
                    if let Some(start) = obj_ty.find('<') {
                        if let Some(end) = obj_ty.rfind('>') {
                            let inner = &obj_ty[start+1..end];
                            return format!("Option<{}>", inner);
                        }
                    }
                }
            }
            "unknown".to_string()
        }
        Expr::Tuple(exprs) => {
            let item_tys: Vec<String> = exprs.iter().map(|e| infer_expr_type(e, env)).collect();
            format!("({})", item_tys.join(","))
        }
        Expr::Call(name, _) => {
            if name.ends_with("::new") {
                name[..name.len() - 5].to_string()
            } else if name.ends_with("::Some") {
                name[..name.len() - 6].to_string()
            } else if name.ends_with("::Ok") {
                name[..name.len() - 4].to_string()
            } else if name.ends_with("::Err") {
                name[..name.len() - 5].to_string()
            } else {
                "unknown".to_string()
            }
        }
        Expr::Cast(_, ty) => ty.to_string(),
        _ => "unknown".to_string(),
    }
}



fn get_all_monomorphized_variants(
    actual: &str,
    funcs: &HashMap<String, Function>,
) -> Vec<String> {
    let mut variants = Vec::new();
    if let Some(f) = funcs.get(actual) {
        if let Some(ref parent) = f.parent_struct {
            let parts: Vec<&str> = parent.split("::").collect();
            if let Some(last_segment) = parts.last() {
                if let Some(idx) = last_segment.find('_') {
                    let struct_base = &last_segment[..idx];
                    let mut prefix_parts = parts.clone();
                    prefix_parts.pop();
                    let prefix = if prefix_parts.is_empty() {
                        format!("{}_", struct_base)
                    } else {
                        format!("{}::{}_", prefix_parts.join("::"), struct_base)
                    };
                    
                    let method_name = actual.split("::").last().unwrap_or("");
                    let suffix = format!("::{}", method_name);
                    
                    for k in funcs.keys() {
                        if k.starts_with(&prefix) && k.ends_with(&suffix) {
                            variants.push(k.clone());
                        }
                    }
                }
            }
        } else {
            // Free function monomorphization
            // e.g. "fw::signal_i32" -> prefix is "fw::signal_"
            let parts: Vec<&str> = actual.split("::").collect();
            if let Some(last_segment) = parts.last() {
                if let Some(idx) = last_segment.rfind('_') {
                    let func_base = &last_segment[..idx];
                    let mut prefix_parts = parts.clone();
                    prefix_parts.pop();
                    let prefix = if prefix_parts.is_empty() {
                        format!("{}_", func_base)
                    } else {
                        format!("{}::{}_", prefix_parts.join("::"), func_base)
                    };
                    for k in funcs.keys() {
                        if k.starts_with(&prefix) {
                            variants.push(k.clone());
                        }
                    }
                }
            }
        }
    }
    if variants.is_empty() {
        variants.push(actual.to_string());
    }
    variants
}

fn collect_callees_expr(
    expr: &Expr,
    sym: &mut HashMap<String, String>,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    callees: &mut HashSet<String>,
) {
    match expr {
        Expr::Call(name, args) => {
            if let Some(ty) = sym.get(name) {
                if ty.starts_with("fn(") {
                    for arg in args {
                        collect_callees_expr(arg, sym, funcs, structs, callees);
                    }
                    return;
                }
            }
            let mut arg_types = Vec::new();
            for arg in args {
                arg_types.push(get_expr_type(arg, sym, funcs, structs));
            }
            let actual = resolve_func_name(name, &arg_types, funcs, structs);
            for variant in get_all_monomorphized_variants(&actual, funcs) {
                callees.insert(variant);
            }
            for arg in args {
                collect_callees_expr(arg, sym, funcs, structs, callees);
            }
        }
        Expr::MethodCall(obj, method, args) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            if lookup_builtin_intrinsic(&obj_ty, method).is_none() {
                let resolved = resolve_struct_name(&obj_ty, structs);
                let actual = resolve_method_name(&resolved, method, &[], funcs);
                if funcs.contains_key(&actual) {
                    callees.insert(actual);
                }
            }
            collect_callees_expr(obj, sym, funcs, structs, callees);
            for arg in args {
                collect_callees_expr(arg, sym, funcs, structs, callees);
            }
        }
        Expr::Binary(l, _, r) => {
            collect_callees_expr(l, sym, funcs, structs, callees);
            collect_callees_expr(r, sym, funcs, structs, callees);
        }
        Expr::FieldAccess(obj, _) => {
            collect_callees_expr(obj, sym, funcs, structs, callees);
        }
        Expr::IndexAccess(arr, idx) => {
            collect_callees_expr(arr, sym, funcs, structs, callees);
            collect_callees_expr(idx, sym, funcs, structs, callees);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields {
                collect_callees_expr(e, sym, funcs, structs, callees);
            }
        }
        Expr::New(_, args) => {
            for arg in args {
                collect_callees_expr(arg, sym, funcs, structs, callees);
            }
        }
        Expr::If(cond, then_b, else_b) => {
            collect_callees_expr(cond, sym, funcs, structs, callees);
            let (t_stmts, t_val) = &**then_b;
            for s in t_stmts { collect_callees_stmt(s, sym, funcs, structs, callees); }
            if let Some(v) = t_val { collect_callees_expr(v, sym, funcs, structs, callees); }
            if let Some(eb) = else_b {
                let (e_stmts, e_val) = &**eb;
                for s in e_stmts { collect_callees_stmt(s, sym, funcs, structs, callees); }
                if let Some(v) = e_val { collect_callees_expr(v, sym, funcs, structs, callees); }
            }
        }
        Expr::Match(target, arms) => {
            let opt_ty = get_expr_type(target, sym, funcs, structs);
            validate_match_patterns(&opt_ty, arms, structs);
            if crate::diagnostics::has_errors() {
                return;
            }
            collect_callees_expr(target, sym, funcs, structs, callees);
            let resolved_ty = resolve_struct_name(&opt_ty, structs);

            for arm in arms {
                if let MatchPattern::CatchAll = arm.pattern {
                    for s in &arm.body {
                        collect_callees_stmt(s, sym, funcs, structs, callees);
                    }
                    if let Some(v) = &arm.val {
                        collect_callees_expr(v, sym, funcs, structs, callees);
                    }
                    continue;
                }

                let (variant_name, bindings) = match &arm.pattern {
                    MatchPattern::Some(v) => ("Some".to_string(), vec![v.clone()]),
                    MatchPattern::None => ("None".to_string(), vec![]),
                    MatchPattern::Ok(v) => ("Ok".to_string(), vec![v.clone()]),
                    MatchPattern::Err(v) => ("Err".to_string(), vec![v.clone()]),
                    MatchPattern::Variant(name, binds) => (name.rsplit("::").next().unwrap().to_string(), binds.clone()),
                    MatchPattern::CatchAll => unreachable!(),
                };

                let mut prev_types = Vec::new();
                if let Some(s_def) = structs.get(&resolved_ty) {
                    for (j, binding_name) in bindings.iter().enumerate() {
                        let field_name = format!("{}_{}", variant_name, j);
                        let ty = s_def.fields.iter().find(|f| f.name == field_name).map(|f| f.ty.to_string()).unwrap_or_else(|| "i32".to_string());
                        let prev = sym.insert(binding_name.clone(), ty);
                        prev_types.push((binding_name.clone(), prev));
                    }
                }

                for s in &arm.body {
                    collect_callees_stmt(s, sym, funcs, structs, callees);
                }
                if let Some(v) = &arm.val {
                    collect_callees_expr(v, sym, funcs, structs, callees);
                }

                for (binding_name, prev) in prev_types {
                    if let Some(prev_ty) = prev {
                        sym.insert(binding_name, prev_ty);
                    } else {
                        sym.remove(&binding_name);
                    }
                }
            }
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            collect_callees_expr(func_expr, sym, funcs, structs, callees);
            for arg in args {
                collect_callees_expr(arg, sym, funcs, structs, callees);
            }
        }
        Expr::Closure(_) => {}
        Expr::ClosureInstantiate(func_name, _, captured) => {
            if funcs.contains_key(func_name) {
                callees.insert(func_name.clone());
            }
            for arg in captured {
                collect_callees_expr(arg, sym, funcs, structs, callees);
            }
        }
        Expr::Cast(e, _) => collect_callees_expr(e, sym, funcs, structs, callees),
        Expr::Spread(e) => {
            collect_callees_expr(e, sym, funcs, structs, callees);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                collect_callees_expr(e, sym, funcs, structs, callees);
            }
        }
        Expr::MapLit(pairs) => {
            let map_ty = infer_expr_type(expr, sym);
            let mono_map_struct = resolve_struct_name(&map_ty, structs);
            callees.insert(format!("{}::new", mono_map_struct));
            callees.insert(format!("{}::set", mono_map_struct));
            for (k, v) in pairs {
                collect_callees_expr(k, sym, funcs, structs, callees);
                collect_callees_expr(v, sym, funcs, structs, callees);
            }
        }
        _ => {}
    }
}

fn collect_callees_stmt(
    stmt: &Stmt,
    sym: &mut HashMap<String, String>,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    callees: &mut HashSet<String>,
) {
    match stmt {
        Stmt::Let(name, ty_annot, expr) => {
            // Determine the variable's type: prefer the annotation, otherwise
            // infer from the initializer expression.
            let ty = if let Some(t) = ty_annot {
                t.to_string()
            } else {
                get_expr_type(expr, sym, funcs, structs)
            };
            collect_callees_expr(expr, sym, funcs, structs, callees);
            let prev = sym.insert(name.clone(), ty);
            // (We intentionally don't restore on scope exit; the function body
            // is a single scope for DCE purposes. This is conservative but
            // safe — an extra reachable callee just keeps a function alive.)
            let _ = prev;
        }
        Stmt::LetTuple(bindings, expr) => {
            collect_callees_expr(expr, sym, funcs, structs, callees);
            for (name, ty) in bindings {
                sym.insert(name.clone(), ty.to_string());
            }
        }
        Stmt::ExprStmt(expr) => collect_callees_expr(expr, sym, funcs, structs, callees),
        Stmt::Return(opt) => {
            if let Some(expr) = opt {
                collect_callees_expr(expr, sym, funcs, structs, callees);
            }
        }
        Stmt::Assign(_, expr) => collect_callees_expr(expr, sym, funcs, structs, callees),
        Stmt::AssignPlus(_, expr) => collect_callees_expr(expr, sym, funcs, structs, callees),
        Stmt::AssignIndex(arr, idx, val) => {
            collect_callees_expr(arr, sym, funcs, structs, callees);
            collect_callees_expr(idx, sym, funcs, structs, callees);
            collect_callees_expr(val, sym, funcs, structs, callees);
        }
        Stmt::AssignField(obj, _, val) => {
            collect_callees_expr(obj, sym, funcs, structs, callees);
            collect_callees_expr(val, sym, funcs, structs, callees);
        }
        Stmt::If(cond, body, else_body) => {
            collect_callees_expr(cond, sym, funcs, structs, callees);
            for s in body {
                collect_callees_stmt(s, sym, funcs, structs, callees);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    collect_callees_stmt(s, sym, funcs, structs, callees);
                }
            }
        }
        Stmt::While(cond, body) => {
            collect_callees_expr(cond, sym, funcs, structs, callees);
            for s in body {
                collect_callees_stmt(s, sym, funcs, structs, callees);
            }
        }
        Stmt::For(_, _, body) => {
            for s in body {
                collect_callees_stmt(s, sym, funcs, structs, callees);
            }
        }
    }
}

fn build_call_graph(
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> HashMap<String, HashSet<String>> {
    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
    for (name, f) in funcs {
        CURRENT_NAMESPACE.with(|c| {
            *c.borrow_mut() = get_namespace(&f.name);
        });
        let mut callees = HashSet::new();
        let mut sym: HashMap<String, String> = HashMap::new();
        for p in &f.params {
            sym.insert(p.name.clone(), p.ty.to_string());
        }
        for s in &f.body {
            collect_callees_stmt(s, &mut sym, funcs, structs, &mut callees);
        }
        graph.insert(name.clone(), callees);
    }
    CURRENT_NAMESPACE.with(|c| {
        *c.borrow_mut() = "".to_string();
    });
    graph
}

pub fn resolve_method_name(parent: &str, method: &str, _arg_types: &[String], funcs: &HashMap<String, Function>) -> String {
    let parent_base = parent.split('<').next().unwrap_or(parent);
    
    // Check if the exact name exists in case it was somehow provided exactly (unlikely but safe)
    let method_name = format!("{}::{}", parent_base, method);
    
    // Match based on parent_struct and method name
    let mut matches = Vec::new();
    for f in funcs.values() {
        if let Some(p) = &f.parent_struct {
            if p == parent || p == parent_base {
                let m = f.name.split("::").last().unwrap();
                if m == method {
                    matches.push(f.name.clone());
                }
            }
        }
    }
    
    // If there is only one match, return it
    if matches.len() == 1 {
        return matches[0].clone();
    }
    
    // Fallback if not found (or ambiguous, which shouldn't happen without overloading)
    if matches.is_empty() {
        return method_name;
    }
    
    matches[0].clone()
}

pub fn split_types(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth_angle = 0;
    let mut depth_paren = 0;
    for c in s.chars() {
        if c == '<' {
            depth_angle += 1;
            current.push(c);
        } else if c == '>' {
            depth_angle -= 1;
            current.push(c);
        } else if c == '(' {
            depth_paren += 1;
            current.push(c);
        } else if c == ')' {
            depth_paren -= 1;
            current.push(c);
        } else if c == ',' && depth_angle == 0 && depth_paren == 0 {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}





