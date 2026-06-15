use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod codegen;
pub mod optimizer;
mod closure;
pub mod macro_runner;
pub mod type_checker;

use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::codegen::{generate_wat, collect_types_from_stmt, collect_string_literals, dead_code_eliminate, sanitize_name, extract_tuple_types, make_tuple_struct_def, generate_js_bindings};
use crate::optimizer::*;

fn type_contains_placeholder(ty: &str, placeholders: &HashSet<String>) -> bool {
    let chars: Vec<char> = ty.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }
            if placeholders.contains(&ident) || (ident.len() == 1 && ident.chars().next().unwrap().is_ascii_uppercase()) {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

fn extract_generic_instantiations_with_placeholders(
    ty: &str,
    instantiations: &mut HashMap<String, HashSet<String>>,
    placeholders: &HashSet<String>,
) {
    let chars: Vec<char> = ty.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == ':' {
            let mut name = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == ':' || chars[i] == '/') {
                name.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '<' {
                i += 1;
                let mut depth = 1;
                let mut args_str = String::new();
                while i < chars.len() && depth > 0 {
                    if chars[i] == '<' {
                        depth += 1;
                    } else if chars[i] == '>' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        args_str.push(chars[i]);
                    }
                    i += 1;
                }
                let mut current_arg = String::new();
                let mut arg_depth = 0;
                let mut args = Vec::new();
                for c in args_str.chars() {
                    if c == '<' {
                        arg_depth += 1;
                        current_arg.push(c);
                    } else if c == '>' {
                        arg_depth -= 1;
                        current_arg.push(c);
                    } else if c == ',' && arg_depth == 0 {
                        args.push(current_arg.trim().to_string());
                        current_arg.clear();
                    } else {
                        current_arg.push(c);
                    }
                }
                if !current_arg.is_empty() {
                    args.push(current_arg.trim().to_string());
                }
                for arg in &args {
                    extract_generic_instantiations_with_placeholders(arg, instantiations, placeholders);
                }
                let has_placeholder = type_contains_placeholder(&args_str, placeholders);
                if !has_placeholder {
                    instantiations.entry(name).or_default().insert(args.join(","));
                }
            }
        } else {
            i += 1;
        }
    }
}

fn parse_file(
    path: &Path,
    namespace: Option<&str>,
    visited: &mut HashSet<PathBuf>,
    cache: &mut HashMap<PathBuf, (Vec<StructDef>, Vec<Function>, Vec<ImplDef>, Vec<TraitDef>, Vec<ConstDef>)>,
    imports_registry: &mut HashMap<String, HashSet<String>>,
    namespace_aliases: &mut HashMap<String, Vec<String>>,
    rename_aliases: &mut HashMap<String, String>,
) -> (Vec<StructDef>, Vec<Function>, Vec<ImplDef>, Vec<TraitDef>, Vec<ConstDef>) {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        if let Some(cached) = cache.get(&canonical) {
            return cached.clone();
        }
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    }
    visited.insert(canonical.clone());

    let is_std_file = {
        let is_std_by_env = if let Ok(fox_path) = std::env::var("FOX_PATH") {
            if let (Ok(c_path), Ok(mut c_fox)) = (std::fs::canonicalize(path), std::fs::canonicalize(Path::new(&fox_path))) {
                if c_fox.join("std").exists() && c_fox.join("src").exists() {
                    c_fox = c_fox.join("std");
                }
                c_path.starts_with(&c_fox)
            } else {
                false
            }
        } else {
            false
        };
        let is_std_by_path = {
            let path_str = path.to_string_lossy();
            path_str.contains("/std/") || path_str.contains("\\std\\") || path_str.starts_with("std/") || path_str.starts_with("std\\")
        };
        is_std_by_env || is_std_by_path
    };

    let source =
        std::fs::read_to_string(path).expect(&format!("Failed to read file: {:?}", path));
    crate::diagnostics::set_current_file(Some(path.to_string_lossy().to_string()));
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let items = parser.parse_module();
    let mut structs = Vec::new();
    let mut funcs = Vec::new();
    let mut impls = Vec::new();
    let mut traits = Vec::new();
    let mut consts = Vec::new();

    for item in items {
        match item {
            Item::Use { path: import_path, symbols } => {
                    // Resolve namespace aliases: use fmt::{sprintf}; where fmt was imported via use std::fmt;
                    let resolved_path = if namespace_aliases.contains_key(&import_path[0]) && import_path.len() == 1 {
                        namespace_aliases[&import_path[0]].clone()
                    } else {
                        import_path.clone()
                    };

                    let dir_path = if resolved_path[0] == "self" {
                        let parent = path.parent().unwrap_or(Path::new("."));
                        let relative = resolved_path[1..].join("/");
                        parent.join(relative).to_string_lossy().to_string()
                    } else if resolved_path[0] == "std" {
                        let mut fox_path = std::env::var("FOX_PATH").unwrap_or_else(|_| String::new());
                         if fox_path.is_empty() {
                            fox_path = String::from("/usr/local/fox");
                        }
                        format!("{}/{}", fox_path, resolved_path.join("/"))
                    } else {
                        panic!("Unknown import root: {}", resolved_path[0]);
                    };

                    let base_path = Path::new(&dir_path);
                    if !base_path.is_dir() {
                        panic!("Import path is not a directory: {:?}", base_path);
                    }

                    // Load all files from the directory
                    let mut imported_structs = Vec::new();
                    let mut imported_funcs = Vec::new();
                    let mut imported_impls = Vec::new();
                    let mut imported_traits = Vec::new();
                    let mut imported_consts = Vec::new();

                    for entry in std::fs::read_dir(base_path).expect(&format!("Failed to read directory {:?}", base_path)) {
                        let entry = entry.unwrap();
                        let file_path = entry.path();
                        if !file_path.is_file() {
                            continue;
                        }
                        let fname = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                        if !fname.ends_with(".fox") {
                            continue;
                        }
                        // .test.fox and .bench.fox are top-level compile units; never pull them
                        // in transitively from a directory import.
                        if fname.ends_with(".test.fox") || fname.ends_with(".bench.fox") {
                            continue;
                        }
                        let mod_name = file_path.file_stem().unwrap().to_str().unwrap();
                        let (imp_structs, imp_funcs, imp_impls, imp_traits, imp_consts) = parse_file(&file_path, Some(mod_name), visited, cache, imports_registry, namespace_aliases, rename_aliases);
                        imported_structs.extend(imp_structs);
                        imported_funcs.extend(imp_funcs);
                        imported_impls.extend(imp_impls);
                        imported_traits.extend(imp_traits);
                        imported_consts.extend(imp_consts);
                    }

                    // Verify that all requested symbols are available (they'll be namespaced in the struct/func/trait lists)
                    for (original, _alias) in &symbols {
                        let mut found = false;
                        for s in &imported_structs {
                            if s.name.ends_with(&format!("::{}", original)) {
                                if !s.is_pub { panic!("Imported struct '{}' is not pub", original); }
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            for f in &imported_funcs {
                                if f.name.ends_with(&format!("::{}", original)) || (f.is_extern && f.name == *original) {
                                    if !f.is_pub && !f._is_pub {
                                        panic!("Imported function '{}' is not pub", original);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            for t in &imported_traits {
                                if t.name.ends_with(&format!("::{}", original)) {
                                    if !t.is_pub { panic!("Imported trait '{}' is not pub", original); }
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            for c in &imported_consts {
                                if c.name.ends_with(&format!("::{}", original)) {
                                    if !c.is_pub { panic!("Imported const '{}' is not pub", original); }
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            panic!("Imported symbol '{}' not found in path {:?}", original, import_path);
                        }
                    }

                    let current_ns = namespace.unwrap_or("").to_string();
                    let registry_entry = imports_registry.entry(current_ns).or_default();

                    if symbols.is_empty() {
                        // Bare namespace import: use std::fmt;
                        // Register the last path segment as a namespace alias
                        let alias = resolved_path.last().unwrap().clone();
                        namespace_aliases.insert(alias.clone(), resolved_path.clone());
                        // Also register the alias in the imports registry so codegen can find it
                        registry_entry.insert(alias);
                    } else {
                        for (original, alias) in symbols {
                            if let Some(alias_name) = alias {
                                // Renamed import: use fmt::{sprintf as sprintf_alias};
                                rename_aliases.insert(alias_name.clone(), original.clone());
                                registry_entry.insert(alias_name);
                            } else {
                                registry_entry.insert(original);
                            }
                        }
                    }

                    structs.extend(imported_structs);
                    funcs.extend(imported_funcs);
                    impls.extend(imported_impls);
                    traits.extend(imported_traits);
                    consts.extend(imported_consts);
            }
            Item::Struct(mut s) => {
                for f in &s.methods {
                    if f.is_extern && f.name.starts_with("__fox_") && !is_std_file {
                        panic!("User-defined extern function '{}' cannot use the protected standard library prefix '__fox_'", f.name);
                    }
                }
                let struct_funcs = std::mem::take(&mut s.methods);
                if let Some(ns) = namespace {
                    s.name = format!("{}::{}", ns, s.name);
                }
                for mut f in struct_funcs {
                    if let Some(ns) = namespace {
                        f.name = format!("{}::{}", ns, f.name);
                        f.parent_struct = Some(s.name.clone());
                        if !f.params.is_empty() && f.params[0].name == "self" {
                            f.params[0].ty = s.name.parse::<Type>().unwrap();
                        }
                    }
                    funcs.push(f);
                }
                structs.push(s);
            }
            Item::Function(mut f) => {
                if f.is_extern && f.name.starts_with("__fox_") && !is_std_file {
                    panic!("User-defined extern function '{}' cannot use the protected standard library prefix '__fox_'", f.name);
                }
                if let Some(ns) = namespace {
                    if !f.is_extern {
                        f.name = format!("{}::{}", ns, f.name);
                    }
                }
                funcs.push(f);
            }
            Item::Trait(mut t) => {
                if let Some(ns) = namespace {
                    t.name = format!("{}::{}", ns, t.name);
                }
                traits.push(t);
            }
            Item::Impl(mut imp) => {
                for f in &imp.methods {
                    if f.is_extern && f.name.starts_with("__fox_") && !is_std_file {
                        panic!("User-defined extern function '{}' cannot use the protected standard library prefix '__fox_'", f.name);
                    }
                }
                let target_ty_str = imp.target_ty.to_string();
                let base_name = target_ty_str.split('<').next().unwrap().to_string();
                let is_primitive = matches!(
                    target_ty_str.as_str(),
                    "i32" | "i64" | "f32" | "f64" | "str" | "void"
                        | "byte" | "anyref" | "externref" | "bool"
                );
                let is_array = target_ty_str.starts_with("[]");
                if imp.trait_name.is_none() {
                    if !is_std_file && (is_primitive || is_array) {
                        panic!(
                            "Inherent impl `impl {} {{ ... }}` is only allowed inside the standard library (FOX_PATH); only the std lib may attach methods to builtins",
                            base_name
                        );
                    }
                }
                let namespaced_target_ty = if let Some(ns) = namespace {
                    if !is_primitive {
                        format!("{}::{}", ns, target_ty_str)
                    } else {
                        target_ty_str.clone()
                    }
                } else {
                    target_ty_str.clone()
                };
                let base_name = namespaced_target_ty.split('<').next().unwrap().to_string();
                for mut f in imp.methods.clone() {
                    if let Some(ns) = namespace {
                        f.name = format!("{}::{}", ns, f.name);
                    }
                    f.parent_struct = Some(base_name.clone());
                    funcs.push(f);
                }
                if let Some(ns) = namespace {
                    if let Some(ref mut tn) = imp.trait_name {
                        *tn = format!("{}::{}", ns, tn).parse::<Type>().unwrap();
                    }
                    if !is_primitive {
                        imp.target_ty = format!("{}::{}", ns, target_ty_str).parse::<Type>().unwrap();
                    }
                }
                impls.push(imp);
            }
            Item::Const(mut c) => {
                if c.name.starts_with("__fox_") && !is_std_file {
                    let kind = if c.is_mutable { "variable" } else { "constant" };
                    panic!("User-defined {} '{}' cannot use the protected standard library prefix '__fox_'", kind, c.name);
                }
                if let Some(ns) = namespace {
                    c.name = format!("{}::{}", ns, c.name);
                }
                consts.push(c);
            }
        }
    }
    let result = (structs, funcs, impls, traits, consts);
    cache.insert(canonical, result.clone());
    result
}

fn strip_unused_runtime_array_helpers(wat: &str) -> String {
    // The codegen always emits, for every array element type X it sees:
    //   (func $fox_alloc_array_X ...)         -- helper definition
    //   (export "fox_alloc_array_X" (func ..)) -- helper export
    //   (func $fox_set_array_X ...)
    //   (export "fox_set_array_X" (func ..))
    // JS only ever calls the f32 variants (see generate_js_bindings), so the
    // others are dead. Strip both the function definition and the export for
    // any helper that has no `call` reference in the WAT, UNLESS the array
    // type appears in a function parameter of an exported function (needed by
    // the macro runner to pass arrays from JS).
    let mut out = String::with_capacity(wat.len());
    let bytes = wat.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((replacement, consumed)) = try_strip_one(wat, i) {
            out.push_str(&replacement);
            i = consumed;
        } else {
            // Copy a char (handle UTF-8 safely).
            let ch = wat[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn array_type_used_in_export_params(wat: &str, type_name: &str) -> bool {
    // Check if this array type appears as a parameter of any non-helper function.
    // The macro runner needs array alloc/set helpers to pass arrays from JS to WASM
    // for compiler functions like `json` that take array params.
    let marker = format!("(param (ref null $array_{})", type_name);
    wat.contains(&marker)
}

fn try_strip_one(wat: &str, i: usize) -> Option<(String, usize)> {
    // Try matching a function definition first.
    if wat[i..].starts_with("(func $fox_alloc_array_")
        || wat[i..].starts_with("(func $fox_set_array_")
    {
        let kind = if wat[i..].starts_with("(func $fox_alloc_array_") {
            "alloc"
        } else {
            "set"
        };
        let prefix = format!("(func $fox_{}_array_", kind);
        let after = &wat[i + prefix.len()..];
        let name_end = after
            .find(|c: char| c == ' ' || c == '\n' || c == ')')
            .unwrap_or(after.len());
        let inner = &after[..name_end];
        let helper_name = format!("$fox_{}_array_{}", kind, inner);
        if wat.contains(&format!("call {}", helper_name)) {
            return None;
        }
        // Keep the helper if the array type is used in a function parameter
        // (needed by the macro runner to pass arrays from JS)
        if array_type_used_in_export_params(wat, inner) {
            return None;
        }
        let consumed = consume_balanced(wat, i)?;
        return Some((String::new(), consumed));
    }
    // Then try matching an export whose inner func is one of the helpers.
    if wat[i..].starts_with("(export \"fox_alloc_array_")
        || wat[i..].starts_with("(export \"fox_set_array_")
    {
        let kind = if wat[i..].starts_with("(export \"fox_alloc_array_") {
            "alloc"
        } else {
            "set"
        };
        let prefix = format!("(export \"fox_{}_array_", kind);
        let after = &wat[i + prefix.len()..];
        // The type name runs until the next `"` (end of string literal).
        let name_end = after.find('"').unwrap_or(after.len());
        let inner = &after[..name_end];
        let helper_name = format!("$fox_{}_array_{}", kind, inner);
        if wat.contains(&format!("call {}", helper_name)) {
            return None;
        }
        // Keep the helper export if the array type is used in a function parameter
        if array_type_used_in_export_params(wat, inner) {
            return None;
        }
        let consumed = consume_balanced(wat, i)?;
        return Some((String::new(), consumed));
    }
    None
}

fn consume_balanced(wat: &str, start: usize) -> Option<usize> {
    let bytes = wat.as_bytes();
    if bytes.get(start) != Some(&b'(') {
        return None;
    }
    let mut depth: usize = 1;
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

const REQUIRED_WASM_OPT_FLAGS: &[&str] = &[
    "--enable-gc",
    "--enable-reference-types",
    "--enable-exception-handling",
    "--enable-multimemory",
    "--enable-bulk-memory",
    "--enable-sign-ext",
    "--enable-nontrapping-float-to-int",
    "--enable-mutable-globals",
    "--enable-tail-call",
    "--enable-multivalue",
];

fn run_wasm_opt(wasm_path: &Path, user_flags: &[String]) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new("wasm-opt");
    cmd.args(REQUIRED_WASM_OPT_FLAGS)
        .args(user_flags)
        .arg(wasm_path)
        .arg("-o")
        .arg(wasm_path);
    let status = cmd.status()?;
    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("wasm-opt exited with status {:?}", status.code()),
        ));
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_path: Option<String> = None;
    let mut output_dir: Option<String> = None;
    let mut opt_flags: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-o" {
            if i + 1 >= args.len() {
                eprintln!("Usage: fox <input.fox> -o <output_dir> [-O* | <wasm-opt flags>...]");
                std::process::exit(1);
            }
            output_dir = Some(args[i + 1].clone());
            i += 2;
        } else if arg.starts_with('-') {
            opt_flags.push(arg.clone());
            i += 1;
        } else if input_path.is_none() {
            input_path = Some(arg.clone());
            i += 1;
        } else {
            eprintln!("Unexpected argument: {}", arg);
            eprintln!("Usage: fox <input.fox> -o <output_dir> [-O* | <wasm-opt flags>...]");
            std::process::exit(1);
        }
    }
    let input_path = match input_path.as_deref() {
        Some(p) => p,
        None => {
            eprintln!("Usage: fox <input.fox> -o <output_dir> [-O* | <wasm-opt flags>...]");
            std::process::exit(1);
        }
    };
    let output_dir = match output_dir.as_deref() {
        Some(p) => p,
        None => {
            eprintln!("Usage: fox <input.fox> -o <output_dir> [-O* | <wasm-opt flags>...]");
            std::process::exit(1);
        }
    };

    // ==========================================
    // Phase 1: Parse source code & build AST
    // ==========================================
    let mut visited = HashSet::new();
    let mut cache = HashMap::new();
    let mut imports_registry = HashMap::new();
    let mut namespace_aliases = HashMap::new();
    let mut rename_aliases = HashMap::new();
    let (mut parsed_structs, mut parsed_funcs, mut parsed_impls, _parsed_traits, mut parsed_consts) =
        parse_file(Path::new(input_path), None, &mut visited, &mut cache, &mut imports_registry, &mut namespace_aliases, &mut rename_aliases);

    if crate::diagnostics::has_errors() {
        crate::diagnostics::print_diagnostics();
        std::process::exit(1);
    }

    // ==========================================
    // Phase 2: Expand macros
    // ==========================================
    crate::macro_runner::run_macros(
        &mut parsed_structs,
        &mut parsed_funcs,
        &mut parsed_impls,
        &mut parsed_consts,
        &imports_registry,
    );

    // ==========================================
    // Phase 3: Type check & run semantic analysis (reporting diagnostics)
    // ==========================================
    // Qualify all types in the AST to their fully-qualified names
    crate::codegen::init_codegen_env(imports_registry.clone(), rename_aliases.clone());
    let mut structs_map = HashMap::new();
    for s in &parsed_structs {
        structs_map.insert(s.name.clone(), s.clone());
    }
    
    for s in &mut parsed_structs {
        let ns = crate::codegen::get_namespace(&s.name);
        crate::codegen::set_current_namespace(ns.clone());
        for field in &mut s.fields {
            field.ty = qualify_type(&field.ty, &ns, &structs_map);
        }
    }
    for f in &mut parsed_funcs {
        let ns = crate::codegen::get_namespace(&f.name);
        crate::codegen::set_current_namespace(ns.clone());
        f.return_ty = qualify_type(&f.return_ty, &ns, &structs_map);
        for param in &mut f.params {
            param.ty = qualify_type(&param.ty, &ns, &structs_map);
        }
        f.body = f.body.iter().map(|s| qualify_stmt(s, &ns, &structs_map)).collect();
    }
    for c in &mut parsed_consts {
        let ns = crate::codegen::get_namespace(&c.name);
        crate::codegen::set_current_namespace(ns.clone());
        c.ty = qualify_type(&c.ty, &ns, &structs_map);
        c.value = qualify_expr(&c.value, &ns, &structs_map);
    }
    for imp in &mut parsed_impls {
        let ns = imp.trait_name.as_ref().map(|t| crate::codegen::get_namespace(&t.to_string())).unwrap_or_else(|| crate::codegen::get_namespace(&imp.target_ty.to_string()));
        crate::codegen::set_current_namespace(ns.clone());
        if let Some(ref mut trait_name) = imp.trait_name {
            *trait_name = qualify_type(trait_name, &ns, &structs_map);
        }
        imp.target_ty = qualify_type(&imp.target_ty, &ns, &structs_map);
        for f in &mut imp.methods {
            let f_ns = crate::codegen::get_namespace(&f.name);
            crate::codegen::set_current_namespace(f_ns.clone());
            f.return_ty = qualify_type(&f.return_ty, &f_ns, &structs_map);
            for p in &mut f.params {
                p.ty = qualify_type(&p.ty, &f_ns, &structs_map);
            }
            f.body = f.body.iter().map(|s| qualify_stmt(s, &f_ns, &structs_map)).collect();
        }
    }
    crate::codegen::set_current_namespace("".to_string());

    // ==========================================
    // Phase 4: Perform generic monomorphization
    // ==========================================
    // Deduplicate consts
    let mut unique_consts = Vec::new();
    let mut const_names = HashSet::new();
    for c in parsed_consts {
        if const_names.insert(c.name.clone()) {
            unique_consts.push(c);
        }
    }
    parsed_consts = unique_consts;

    // Deduplicate structs
    let mut unique_structs = Vec::new();
    let mut struct_names = HashSet::new();
    for s in parsed_structs {
        if struct_names.insert(s.name.clone()) {
            unique_structs.push(s);
        }
    }
    parsed_structs = unique_structs;

    // Deduplicate impls
    let mut unique_impls = Vec::new();
    let mut impl_keys = HashSet::new();
    for imp in parsed_impls {
        let key = (imp.trait_name.clone(), imp.target_ty.clone());
        if impl_keys.insert(key) {
            unique_impls.push(imp);
        }
    }
    parsed_impls = unique_impls;

    // Merge impl methods into parsed_structs
    for imp in &parsed_impls {
        for target in &mut parsed_structs {
            if target.name == imp.target_ty.to_string() {
                for method in imp.methods.clone() {
                    target.methods.push(method);
                }
            }
        }
    }

    // ==========================================
    // Phase 3b: Type-check all function bodies (including private functions
    // that will be dead-code-eliminated before codegen)
    // ==========================================
    {
        // Populate GLOBAL_CONSTS so const references resolve correctly
        crate::codegen::GLOBAL_CONSTS.with(|gc| {
            let mut map = gc.borrow_mut();
            map.clear();
            for c in &parsed_consts {
                map.insert(c.name.clone(), c.ty.to_string());
            }
        });
        let mut structs_map = HashMap::new();
        for s in &parsed_structs {
            structs_map.insert(s.name.clone(), s.clone());
        }
        let mut funcs_map = HashMap::new();
        for f in &parsed_funcs {
            funcs_map.insert(f.name.clone(), f.clone());
        }
        for f in &parsed_funcs {
            crate::type_checker::validate_call_types_in_func(f, &funcs_map, &structs_map);
        }
        if crate::diagnostics::has_errors() {
            crate::diagnostics::print_diagnostics();
            std::process::exit(1);
        }
    }

    for f in &mut parsed_funcs {
        optimizer::inline_closures_in_function(f);
    }

    closure::lift_closures_in_funcs(&mut parsed_funcs, &mut parsed_structs);

    // Deduplicate funcs
    let mut unique_funcs = Vec::new();
    let mut func_names = HashSet::new();
    for f in parsed_funcs {
        if func_names.insert(f.name.clone()) {
            unique_funcs.push(f);
        }
    }
    parsed_funcs = unique_funcs;


    // Resolve trait constraints to concrete types implementing those traits
    for s in &mut parsed_structs {
        for param in &mut s.generic.params {
            let mut resolved_constraints = Vec::new();
            for constraint in &param.constraints {
                let mut found_impls = Vec::new();
                for imp in &parsed_impls {
                    if let Some(a) = &imp.trait_name {
                        let a_str = a.to_string();
                        let b_str = constraint.to_string();
                        let matched = a == constraint || a_str.ends_with(&format!("::{}", b_str)) || b_str.ends_with(&format!("::{}", a_str));
                        if matched {
                            found_impls.push(imp.target_ty.clone());
                        }
                    }
                }
                if found_impls.is_empty() {
                    resolved_constraints.push(constraint.clone());
                } else {
                    resolved_constraints.extend(found_impls);
                }
            }
            param.constraints = resolved_constraints;
        }
    }

    for f in &mut parsed_funcs {
        for param in &mut f.generic.params {
            let mut resolved_constraints = Vec::new();
            for constraint in &param.constraints {
                let mut found_impls = Vec::new();
                for imp in &parsed_impls {
                    if let Some(a) = &imp.trait_name {
                        let a_str = a.to_string();
                        let b_str = constraint.to_string();
                        let matched = a == constraint || a_str.ends_with(&format!("::{}", b_str)) || b_str.ends_with(&format!("::{}", a_str));
                        if matched {
                            found_impls.push(imp.target_ty.clone());
                        }
                    }
                }
                if found_impls.is_empty() {
                    resolved_constraints.push(constraint.clone());
                } else {
                    resolved_constraints.extend(found_impls);
                }
            }
            param.constraints = resolved_constraints;
        }
    }

    // Keep track of which generic parameters were originally unconstrained
    let mut originally_unconstrained_structs = HashSet::new();
    for s in &parsed_structs {
        for (idx, param) in s.generic.params.iter().enumerate() {
            if param.constraints.is_empty() {
                originally_unconstrained_structs.insert((s.name.clone(), idx));
            }
        }
    }
    let mut originally_unconstrained_funcs = HashSet::new();
    for f in &parsed_funcs {
        for (idx, param) in f.generic.params.iter().enumerate() {
            if param.constraints.is_empty() {
                originally_unconstrained_funcs.insert((f.name.clone(), idx));
            }
        }
    }

    // Auto-populate constraints for unconstrained generic parameters by scanning AST type instantiations
    let mut instantiations: HashMap<String, HashSet<String>> = HashMap::new();
    let mut processed_types = HashSet::new();

    loop {
        let mut new_types = Vec::new();
        // 1. Scan un-substituted templates (both structs and functions) to find instantiations
        for s in &parsed_structs {
            let placeholders: HashSet<String> = s.generic.params.iter().map(|gp| gp.name.clone()).collect();
            for f in &s.fields {
                if processed_types.insert(f.ty.to_string()) {
                    new_types.push((f.ty.to_string(), placeholders.clone()));
                }
            }
        }
        for f in &parsed_funcs {
            let mut placeholders: HashSet<String> = f.generic.params.iter().map(|gp| gp.name.clone()).collect();
            if let Some(ref parent_name) = f.parent_struct {
                if let Some(parent) = parsed_structs.iter().find(|s| s.name == *parent_name) {
                    placeholders.extend(parent.generic.params.iter().map(|gp| gp.name.clone()));
                }
            }
            if processed_types.insert(f.return_ty.to_string()) {
                new_types.push((f.return_ty.to_string(), placeholders.clone()));
            }
            for p in &f.params {
                if processed_types.insert(p.ty.to_string()) {
                    new_types.push((p.ty.to_string(), placeholders.clone()));
                }
            }
            let mut env = HashMap::new();
            for p in &f.params {
                env.insert(p.name.clone(), p.ty.to_string());
            }
            let mut func_body_types = Vec::new();
            for stmt in &f.body {
                collect_types_from_stmt(stmt, &mut func_body_types, &mut env);
            }
            for ty in func_body_types {
                if processed_types.insert(ty.clone()) {
                    new_types.push((ty, placeholders.clone()));
                }
            }
        }

        // 2. Scan substituted templates for all combinations of currently-known constraints
        for s in &parsed_structs {
            if s.generic.params.is_empty() {
                continue;
            }
            let mut combinations = Vec::new();
            let mut current = Vec::new();
            generate_combinations(&s.generic.params, 0, &mut current, &mut combinations);
            for choices in combinations {
                let placeholders = HashSet::new();
                for f in &s.fields {
                    let substituted = apply_multi_substitute_type(&f.ty, &s.generic.params, &choices).to_string();
                    if processed_types.insert(substituted.clone()) {
                        new_types.push((substituted, placeholders.clone()));
                    }
                }
            }
        }

        for f in &parsed_funcs {
            let mut all_params = Vec::new();
            let mut parent_generic = None;
            if let Some(ref parent_name) = f.parent_struct {
                if let Some(parent) = parsed_structs.iter().find(|s| s.name == *parent_name) {
                    all_params.extend(parent.generic.params.clone());
                    parent_generic = Some(parent.generic.clone());
                }
            }
            all_params.extend(f.generic.params.clone());

            if all_params.is_empty() {
                continue;
            }

            let mut combinations = Vec::new();
            let mut current = Vec::new();
            generate_combinations(&all_params, 0, &mut current, &mut combinations);
            for choices in combinations {
                let placeholders = HashSet::new();
                
                let (parent_choices, func_choices) = if let Some(ref pg) = parent_generic {
                    let split_idx = pg.params.len();
                    (&choices[..split_idx], &choices[split_idx..])
                } else {
                    (&choices[..0], &choices[..])
                };

                let mut subbed_return_ty = f.return_ty.clone();
                if let Some(ref pg) = parent_generic {
                    subbed_return_ty = apply_multi_substitute_type(&subbed_return_ty, &pg.params, parent_choices);
                }
                subbed_return_ty = apply_multi_substitute_type(&subbed_return_ty, &f.generic.params, func_choices);

                if processed_types.insert(subbed_return_ty.to_string()) {
                    new_types.push((subbed_return_ty.to_string(), placeholders.clone()));
                }

                for p in &f.params {
                    let mut subbed_p_ty = p.ty.clone();
                    if let Some(ref pg) = parent_generic {
                        subbed_p_ty = apply_multi_substitute_type(&subbed_p_ty, &pg.params, parent_choices);
                    }
                    subbed_p_ty = apply_multi_substitute_type(&subbed_p_ty, &f.generic.params, func_choices);

                    if processed_types.insert(subbed_p_ty.to_string()) {
                        new_types.push((subbed_p_ty.to_string(), placeholders.clone()));
                    }
                }

                let mut env = HashMap::new();
                for p in &f.params {
                    let mut subbed_p_ty = p.ty.clone();
                    if let Some(ref pg) = parent_generic {
                        subbed_p_ty = apply_multi_substitute_type(&subbed_p_ty, &pg.params, parent_choices);
                    }
                    subbed_p_ty = apply_multi_substitute_type(&subbed_p_ty, &f.generic.params, func_choices);
                    env.insert(p.name.clone(), subbed_p_ty.to_string());
                }

                let mut func_body_types = Vec::new();
                for stmt in &f.body {
                    let mut subbed_stmt = stmt.clone();
                    if let Some(ref pg) = parent_generic {
                        subbed_stmt = apply_multi_substitute_stmt(&subbed_stmt, &pg.params, parent_choices);
                    }
                    subbed_stmt = apply_multi_substitute_stmt(&subbed_stmt, &f.generic.params, func_choices);
                    collect_types_from_stmt(&subbed_stmt, &mut func_body_types, &mut env);
                }
                for ty in func_body_types {
                    if processed_types.insert(ty.clone()) {
                        new_types.push((ty, placeholders.clone()));
                    }
                }
            }
        }

        if new_types.is_empty() {
            break;
        }

        let mut added_any = false;
        let mut before_total = 0;
        for s in instantiations.values() {
            before_total += s.len();
        }
        for (ty, placeholders) in new_types {
            extract_generic_instantiations_with_placeholders(&ty, &mut instantiations, &placeholders);
        }
        let mut after_total = 0;
        for s in instantiations.values() {
            after_total += s.len();
        }
        if after_total != before_total {
            added_any = true;
        }

        for s in &mut parsed_structs {
            if !s.generic.params.is_empty() {
                let short_name = s.name.split("::").last().unwrap_or(&s.name);
                let mut choices_for_params = vec![HashSet::new(); s.generic.params.len()];

                for (key, insts) in &instantiations {
                    let key_short = key.split("::").last().unwrap_or(key);
                    if key_short == short_name {
                        for inst in insts {
                            let mut current = String::new();
                            let mut depth_angle = 0;
                            let mut depth_paren = 0;
                            let mut parts = Vec::new();
                            for c in inst.chars() {
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
                            for (idx, part) in parts.iter().enumerate() {
                                if idx < choices_for_params.len() {
                                    let is_placeholder = s.generic.params.iter().any(|gp| gp.name == *part);
                                    if !is_placeholder {
                                        choices_for_params[idx].insert(part.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                for (idx, param) in s.generic.params.iter_mut().enumerate() {
                    if originally_unconstrained_structs.contains(&(s.name.clone(), idx)) {
                        let original_len = param.constraints.len();
                        let mut new_constraints: Vec<String> = choices_for_params[idx].iter().cloned().collect();
                        new_constraints.sort();
                        param.constraints = new_constraints.iter().map(|c| c.parse::<Type>().unwrap()).collect();
                        if param.constraints.len() != original_len {
                            added_any = true;
                        }
                    }
                }
            }
        }

        for f in &mut parsed_funcs {
            if !f.generic.params.is_empty() {
                let short_name = f.name.split("::").last().unwrap_or(&f.name);
                let mut choices_for_params = vec![HashSet::new(); f.generic.params.len()];

                for (key, insts) in &instantiations {
                    let key_short = key.split("::").last().unwrap_or(key);
                    if key_short == short_name {
                        for inst in insts {
                            let mut current = String::new();
                            let mut depth_angle = 0;
                            let mut depth_paren = 0;
                            let mut parts = Vec::new();
                            for c in inst.chars() {
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
                            for (idx, part) in parts.iter().enumerate() {
                                if idx < choices_for_params.len() {
                                    let is_placeholder = f.generic.params.iter().any(|gp| gp.name == *part);
                                    if !is_placeholder {
                                        choices_for_params[idx].insert(part.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                for (idx, param) in f.generic.params.iter_mut().enumerate() {
                    if originally_unconstrained_funcs.contains(&(f.name.clone(), idx)) {
                        let original_len = param.constraints.len();
                        let mut new_constraints: Vec<String> = choices_for_params[idx].iter().cloned().collect();
                        new_constraints.sort();
                        param.constraints = new_constraints.iter().map(|c| c.parse::<Type>().unwrap()).collect();
                        if param.constraints.len() != original_len {
                            added_any = true;
                        }
                    }
                }
            }
        }

        if !added_any {
            break;
        }
    }

    let mut type_remap: HashMap<String, String> = HashMap::new();
    let mut skip_funcs: HashSet<String> = HashSet::new();
    let mut mono_structs: Vec<StructDef> = Vec::new();
    let mut mono_methods: Vec<Function> = Vec::new();

    for s in &parsed_structs {
        if !s.generic.params.is_empty() {
            let mut combinations = Vec::new();
            let mut current = Vec::new();
            generate_combinations(&s.generic.params, 0, &mut current, &mut combinations);

            for choices in &combinations {
                let suffix: String = choices.iter().map(|c| format!("_{}", c.replace("::", "_"))).collect();
                let new_name = sanitize_name(&format!(
                    "{}{}",
                    s.name,
                    suffix
                ));
                let remap_key = format!(
                    "{}<{}>",
                    s.name,
                    choices.join(",")
                );
                type_remap.insert(remap_key, new_name.clone());
                let short_name = s.name.split("::").last().unwrap_or(&s.name);
                let short_remap_key = format!("{}<{}>", short_name, choices.join(","));
                type_remap.insert(short_remap_key, new_name.clone());

                let new_fields: Vec<Field> = s
                    .fields
                    .iter()
                    .map(|f| Field {
                        name: f.name.clone(),
                        ty: apply_multi_substitute_type(&f.ty, &s.generic.params, &choices),
                        attributes: f.attributes.clone(),
                    })
                    .collect();

                for f in &parsed_funcs {
                    if f.parent_struct.as_deref() == Some(s.name.as_str()) {
                        skip_funcs.insert(f.name.clone());
                        let method_name = f.name.split("::").last().unwrap();
                        let mut new_f = f.clone();
                        new_f.name = format!("{}::{}", new_name, method_name);
                        new_f.parent_struct = Some(new_name.clone());
                        new_f.generic = GenericParams::default();
                        new_f.return_ty = apply_multi_substitute_type(
                            &f.return_ty,
                            &s.generic.params,
                            &choices,
                        );
                        for param in &mut new_f.params {
                            param.ty = apply_multi_substitute_type(
                                &param.ty,
                                &s.generic.params,
                                &choices,
                            );
                        }
                        if !new_f.params.is_empty() && new_f.params[0].name == "self" {
                            new_f.params[0].ty = new_name.parse::<Type>().unwrap();
                        }
                        new_f.body = f
                            .body
                            .iter()
                            .map(|st| apply_multi_substitute_stmt(st, &s.generic.params, &choices))
                            .collect();
                        mono_methods.push(new_f);
                    }
                }

                mono_structs.push(StructDef {
                    is_pub: s.is_pub,
                    name: new_name,
                    generic: GenericParams::default(),
                    fields: new_fields,
                    methods: Vec::new(),
                    is_enum: s.is_enum,
                    variants: s.variants.clone(),
                    attributes: s.attributes.clone(),
                });
            }
            
            if combinations.is_empty() {
                for f in &parsed_funcs {
                    if f.parent_struct.as_deref() == Some(s.name.as_str()) {
                        skip_funcs.insert(f.name.clone());
                    }
                }
            }
        } else {
            mono_structs.push(s.clone());
        }
    }

    if !type_remap.is_empty() {
        for f in &mut mono_methods {
            f.return_ty = remap_type(&f.return_ty, &type_remap);
            for p in &mut f.params {
                p.ty = remap_type(&p.ty, &type_remap);
            }
            f.body = f.body.iter().map(|s| remap_stmt(s, &type_remap)).collect();
        }
        for s in &mut mono_structs {
            for field in &mut s.fields {
                field.ty = remap_type(&field.ty, &type_remap);
            }
        }
        for c in &mut parsed_consts {
            c.ty = remap_type(&c.ty, &type_remap);
            c.value = remap_expr(&c.value, &type_remap);
        }
    }

    let mut funcs: Vec<Function> = Vec::new();
    for f in parsed_funcs {
        if skip_funcs.contains(&f.name) {
            continue;
        }
        let mut f = f;
        if !type_remap.is_empty() {
            f.return_ty = remap_type(&f.return_ty, &type_remap);
            for p in &mut f.params {
                p.ty = remap_type(&p.ty, &type_remap);
            }
            f.body = f.body.iter().map(|s| remap_stmt(s, &type_remap)).collect();
        }
        if !f.generic.params.is_empty() {
            let generic = &f.generic.params[0];
            for constraint in &generic.constraints {
                let mut new_f = f.clone();
                new_f.name = sanitize_name(&format!("{}_{}", f.name, constraint.to_string().replace("::", "_")));
                new_f.generic = GenericParams::default();
                new_f.return_ty = f.return_ty.substitute(&generic.name, constraint);
                for param in &mut new_f.params {
                    param.ty = param.ty.substitute(&generic.name, constraint);
                }
                new_f.body = f
                    .body
                    .iter()
                    .map(|s| substitute_stmt(s, &generic.name, &constraint.to_string()))
                    .collect();
                funcs.push(new_f);
            }
        } else {
            funcs.push(f);
        }
    }
    funcs.extend(mono_methods);

    // ==========================================
    // Phase 5: Run optimization passes (e.g. optimizer.rs)
    // ==========================================
    let mut func_map = HashMap::new();
    for f in &funcs {
        func_map.insert(f.name.clone(), f.clone());
    }

    let mut optimized_funcs = Vec::new();
    for mut f in funcs {
        f.body = f.body.iter().map(|s| inline_calls_in_stmt(s, &func_map)).collect();
        f.body = pass_loop_unswitch_block(&f.body);
        f.body = optimize_block(&f.body);
        optimized_funcs.push(f);
    }

    let mut type_strings = std::collections::HashSet::new();
    for s in &mono_structs {
        type_strings.insert(s.name.clone());
        for f in &s.fields {
            type_strings.insert(f.ty.to_string());
        }
    }
    for f in &optimized_funcs {
        type_strings.insert(f.return_ty.to_string());
        let mut env = HashMap::new();
        for p in &f.params {
            type_strings.insert(p.ty.to_string());
            env.insert(p.name.clone(), p.ty.to_string());
            if p.is_variadic {
                type_strings.insert(format!("[]{}", p.ty.to_string()));
            }
        }
        let mut func_types = Vec::new();
        for s in &f.body {
            collect_types_from_stmt(s, &mut func_types, &mut env);
        }
        for ty in func_types {
            type_strings.insert(ty);
        }
    }

    let mut tuple_types = std::collections::HashSet::new();
    for ty in type_strings {
        extract_tuple_types(&ty, &mut tuple_types);
    }

    for tuple_ty in tuple_types {
        let tuple_struct = make_tuple_struct_def(&tuple_ty);
        mono_structs.push(tuple_struct);
    }

    // ==========================================
    // Phase 6: Generate WAT and companion JS bindings
    // ==========================================
    crate::codegen::init_codegen_env(imports_registry.clone(), rename_aliases.clone());
    let optimized_funcs = dead_code_eliminate(optimized_funcs, &mono_structs, &parsed_consts);

    let string_literals = collect_string_literals(&optimized_funcs);

    let mut variadic_funcs: HashMap<String, usize> = HashMap::new();
    for f in &optimized_funcs {
        if f.is_variadic() {
            let safe_name = f.name.replace("::", "_");
            let arity = f.params.len() - 1;
            variadic_funcs.insert(safe_name, arity);
        }
    }

    let (final_wat, filtered_structs) = {
        crate::codegen::set_rename_aliases(rename_aliases);
        generate_wat(&optimized_funcs, &mono_structs, &string_literals, &parsed_consts, &imports_registry)
    };
    if crate::diagnostics::has_errors() {
        crate::diagnostics::print_diagnostics();
        std::process::exit(1);
    }
    let final_wat = strip_unused_runtime_array_helpers(&final_wat);
    let wasm_bytes = match wat::parse_str(&final_wat) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("{}\nFailed to parse WAT: {:?}", final_wat, e);
            std::fs::write("/tmp/fox_failed.wat", &final_wat).ok();
            panic!("Failed to parse WAT");
        }
    };
    let input_stem = Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");

    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");
    let wat_path = Path::new(output_dir).join(format!("{}.wat", input_stem));
    std::fs::write(&wat_path, &final_wat).expect("Failed to write wat file");
    let output_path = Path::new(output_dir).join(format!("{}.wasm", input_stem));
    std::fs::write(&output_path, wasm_bytes).expect("Failed to write wasm file");

    if !opt_flags.is_empty() {
        match run_wasm_opt(&output_path, &opt_flags) {
            Ok(()) => println!(
                "Optimized {} with wasm-opt ({})",
                output_path.display(),
                opt_flags.join(" ")
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!(
                    "warning: wasm-opt not found on PATH; {} was not optimized",
                    output_path.display()
                );
            }
            Err(e) => {
                eprintln!("wasm-opt failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    let js_path = Path::new(output_dir).join(format!("{}.js", input_stem));
    std::fs::write(&js_path, generate_js_bindings(&filtered_structs, &string_literals, &variadic_funcs, Some(&final_wat)))
        .expect("Failed to write JS bindings");

    println!(
        "Successfully compiled {} to {} and {}",
        input_path,
        output_path.display(),
        js_path.display()
    );
}

use std::str::FromStr;

fn qualify_type(ty: &Type, current_ns: &str, structs: &HashMap<String, StructDef>) -> Type {
    crate::codegen::set_current_namespace(current_ns.to_string());
    match ty {
        Type::Struct(name, args) => {
            let resolved_name = crate::codegen::resolve_struct_name(name, structs);
            let resolved_args = args.iter().map(|arg| qualify_type(arg, current_ns, structs)).collect();
            Type::Struct(resolved_name, resolved_args)
        }
        Type::GenericParam(name) => {
            let resolved_name = crate::codegen::resolve_struct_name(name, structs);
            Type::GenericParam(resolved_name)
        }
        Type::Array(inner) => Type::Array(Box::new(qualify_type(inner, current_ns, structs))),
        Type::Tuple(elems) => Type::Tuple(elems.iter().map(|el| qualify_type(el, current_ns, structs)).collect()),
        Type::Function(params, ret) => {
            Type::Function(
                params.iter().map(|p| qualify_type(p, current_ns, structs)).collect(),
                Box::new(qualify_type(ret, current_ns, structs))
            )
        }
        _ => ty.clone(),
    }
}

fn qualify_stmt(stmt: &Stmt, current_ns: &str, structs: &HashMap<String, StructDef>) -> Stmt {
    let result = match stmt {
        Stmt::Let(name, ty_opt, expr) => {
            let new_ty = ty_opt.as_ref().map(|t| qualify_type(t, current_ns, structs));
            Stmt::Let(name.clone(), new_ty, qualify_expr(expr, current_ns, structs))
        }
        Stmt::LetTuple(bindings, expr) => {
            let new_bindings = bindings.iter().map(|(name, ty)| {
                (name.clone(), qualify_type(ty, current_ns, structs))
            }).collect();
            Stmt::LetTuple(new_bindings, qualify_expr(expr, current_ns, structs))
        }
        Stmt::ExprStmt(expr) => Stmt::ExprStmt(qualify_expr(expr, current_ns, structs)),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|e| qualify_expr(e, current_ns, structs))),
        Stmt::Assign(name, expr) => Stmt::Assign(name.clone(), qualify_expr(expr, current_ns, structs)),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(name.clone(), qualify_expr(expr, current_ns, structs)),
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(qualify_expr(arr, current_ns, structs)),
            Box::new(qualify_expr(idx, current_ns, structs)),
            qualify_expr(val, current_ns, structs),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(qualify_expr(obj, current_ns, structs)),
            field.clone(),
            qualify_expr(val, current_ns, structs),
        ),
        Stmt::If(cond, body, else_body) => {
            let new_body = body.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect();
            let new_else = else_body.as_ref().map(|eb| eb.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect());
            Stmt::If(qualify_expr(cond, current_ns, structs), new_body, new_else)
        }
        Stmt::While(cond, body) => {
            let new_body = body.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect();
            Stmt::While(qualify_expr(cond, current_ns, structs), new_body)
        }
        Stmt::For(loop_var, target, body) => {
            let new_body = body.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect();
            Stmt::For(loop_var.clone(), target.clone(), new_body)
        }
    };
    if let Some(span) = get_span(stmt) {
        register_span(&result, span);
    }
    result
}

fn qualify_expr(expr: &Expr, current_ns: &str, structs: &HashMap<String, StructDef>) -> Expr {
    let result = match expr {
        Expr::Binary(l, op, r) => Expr::Binary(
            Box::new(qualify_expr(l, current_ns, structs)),
            *op,
            Box::new(qualify_expr(r, current_ns, structs)),
        ),
        Expr::MethodCall(obj, method, args) => Expr::MethodCall(
            Box::new(qualify_expr(obj, current_ns, structs)),
            method.clone(),
            args.iter().map(|a| qualify_expr(a, current_ns, structs)).collect(),
        ),
        Expr::FieldAccess(obj, field) => Expr::FieldAccess(
            Box::new(qualify_expr(obj, current_ns, structs)),
            field.clone(),
        ),
        Expr::StructInit(name, fields) => {
            let name_ty = Type::from_str(name).unwrap_or(Type::GenericParam(name.clone()));
            let resolved_name = qualify_type(&name_ty, current_ns, structs).to_string();
            Expr::StructInit(
                resolved_name,
                fields.iter().map(|(n, e)| (n.clone(), qualify_expr(e, current_ns, structs))).collect(),
            )
        }
        Expr::Call(name, args) => {
            let mut resolved_name = name.clone();
            if name.contains("::") {
                let mut last_colon_idx = None;
                let mut depth = 0;
                let chars: Vec<char> = name.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '<' { depth += 1; }
                    else if chars[i] == '>' { depth -= 1; }
                    else if chars[i] == ':' && i + 1 < chars.len() && chars[i+1] == ':' && depth == 0 {
                        last_colon_idx = Some(i);
                        i += 1;
                    }
                    i += 1;
                }
                if let Some(idx) = last_colon_idx {
                    let struct_part = &name[..idx];
                    let method_part = &name[idx + 2..];
                    if let Ok(struct_ty) = Type::from_str(struct_part) {
                        let resolved_struct = qualify_type(&struct_ty, current_ns, structs).to_string();
                        resolved_name = format!("{}::{}", resolved_struct, method_part);
                    }
                }
            }
            Expr::Call(
                resolved_name,
                args.iter().map(|a| qualify_expr(a, current_ns, structs)).collect(),
            )
        }
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(qualify_expr(arr, current_ns, structs)),
            Box::new(qualify_expr(idx, current_ns, structs)),
        ),
        Expr::New(ty, args) => Expr::New(
            qualify_type(ty, current_ns, structs),
            args.iter().map(|a| qualify_expr(a, current_ns, structs)).collect(),
        ),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_then = Box::new((
                t_stmts.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect(),
                t_val.as_ref().map(|v| qualify_expr(v, current_ns, structs)),
            ));
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect(),
                    e_val.as_ref().map(|v| qualify_expr(v, current_ns, structs)),
                ))
            });
            Expr::If(
                Box::new(qualify_expr(cond, current_ns, structs)),
                new_then,
                new_else,
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(qualify_expr(cond, current_ns, structs)),
            arms.iter().map(|arm| MatchArm {
                pattern: arm.pattern.clone(),
                body: arm.body.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect(),
                val: arm.val.as_ref().map(|v| qualify_expr(v, current_ns, structs)),
            }).collect(),
        ),
        Expr::InvokeFuncPtr(func, args) => Expr::InvokeFuncPtr(
            Box::new(qualify_expr(func, current_ns, structs)),
            args.iter().map(|a| qualify_expr(a, current_ns, structs)).collect(),
        ),
        Expr::Closure(func) => {
            let mut new_func = func.clone();
            new_func.return_ty = qualify_type(&func.return_ty, current_ns, structs);
            for p in &mut new_func.params {
                p.ty = qualify_type(&p.ty, current_ns, structs);
            }
            new_func.body = func.body.iter().map(|s| qualify_stmt(s, current_ns, structs)).collect();
            Expr::Closure(new_func)
        }
        Expr::ClosureInstantiate(name, env, captured) => Expr::ClosureInstantiate(
            name.clone(),
            env.clone(),
            captured.iter().map(|a| qualify_expr(a, current_ns, structs)).collect(),
        ),
        Expr::Cast(e, ty) => Expr::Cast(
            Box::new(qualify_expr(e, current_ns, structs)),
            qualify_type(ty, current_ns, structs),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(qualify_expr(e, current_ns, structs))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|e| qualify_expr(e, current_ns, structs)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(
            pairs.iter().map(|(k, v)| (qualify_expr(k, current_ns, structs), qualify_expr(v, current_ns, structs))).collect()
        ),
        _ => expr.clone(),
    };
    if let Some(span) = get_span(expr) {
        register_span(&result, span);
    }
    result
}
