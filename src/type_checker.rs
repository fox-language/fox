use std::collections::{HashMap, HashSet};
use crate::ast::{Expr, Function, StructDef, MatchArm, MatchPattern, Op};
use crate::codegen::{
    resolve_struct_name, resolve_const_name, resolve_method_name,
    resolve_func_name, extract_fn_return_type,
};
use crate::codegen::intrinsics::lookup_builtin_intrinsic;

pub fn get_expr_type(
    expr: &Expr,
    sym: &HashMap<String, String>,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> String {
    crate::diagnostics::set_current_span(crate::ast::get_span(expr));
    match expr {
        Expr::Identifier(n) => {
            if sym.contains_key(n) {
                sym.get(n).unwrap().clone()
            } else if let Some(resolved) = resolve_const_name(n) {
                crate::codegen::GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                })
            } else {
                "i32".to_string()
            }
        }
        Expr::IndexAccess(arr, _) => {
            let arr_ty = get_expr_type(arr, sym, funcs, structs);
            if arr_ty.starts_with("[]") {
                arr_ty[2..].to_string()
            } else {
                panic!("Indexing non-array type: {}", arr_ty);
            }
        }
        Expr::Binary(l, op, r) => {
            if matches!(op, Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual | Op::EqualEqual | Op::NotEqual) {
                return "bool".to_string();
            }
            let l_ty = get_expr_type(l, sym, funcs, structs);
            let r_ty = get_expr_type(r, sym, funcs, structs);
            if l_ty == "f64" || r_ty == "f64" {
                "f64".to_string()
            } else if l_ty == "f32" || r_ty == "f32" {
                "f32".to_string()
            } else if l_ty == "i64" || r_ty == "i64" {
                "i64".to_string()
            } else if l_ty == "i32" || l_ty == "unknown" {
                r_ty
            } else {
                l_ty
            }
        }
        Expr::Integer(s) => {
            if s.parse::<i32>().is_ok() {
                "i32".to_string()
            } else {
                "i64".to_string()
            }
        }
        Expr::Float(_) => "f32".to_string(),
        Expr::StringLit(_) => "str".to_string(),
        Expr::Bool(_) => "bool".to_string(),
        Expr::FieldAccess(obj, f_name) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            if obj_ty.starts_with("[]") {
                panic!("Field access not supported on array type: {}.{}", obj_ty, f_name);
            }
            let resolved_ty = resolve_struct_name(&obj_ty, structs);
            if let Some(s) = structs.get(&resolved_ty) {
                if let Some(f) = s.fields.iter().find(|f| f.name == *f_name) {
                    return f.ty.to_string();
                }
            }
            "i32".to_string()
        }
        Expr::StructInit(n, fields) => {
            let resolved_name = resolve_struct_name(n, structs);
            if let Some(s) = structs.get(&resolved_name) {
                for s_field in &s.fields {
                    if !fields.iter().any(|(fname, _)| fname == &s_field.name) {
                        crate::diagnostics::report_error(
                            format!("Missing field '{}' in instantiation of struct '{}'", s_field.name, n),
                            crate::ast::get_span(expr),
                        );
                    }
                }
                for (fname, fexpr) in fields {
                    if let Some(s_field) = s.fields.iter().find(|sf| &sf.name == fname) {
                        let actual_ty = get_expr_type(fexpr, sym, funcs, structs);
                        let expected_ty = s_field.ty.to_string();
                        if expected_ty != actual_ty {
                            let is_actual_64 = actual_ty == "i64" || actual_ty == "u64";
                            let is_expected_64 = expected_ty == "i64" || expected_ty == "u64";
                            let is_actual_32 = actual_ty == "i32" || actual_ty == "u32" || actual_ty == "byte" || actual_ty == "bool";
                            let is_expected_32 = expected_ty == "i32" || expected_ty == "u32" || expected_ty == "byte" || expected_ty == "bool";
                            if (is_expected_64 && is_actual_32) || (is_expected_32 && is_actual_64) || (!is_actual_64 && !is_expected_64 && actual_ty != expected_ty) {
                                crate::diagnostics::report_error(
                                    format!("Type mismatch for field '{}': expected '{}', found '{}'", fname, expected_ty, actual_ty),
                                    crate::ast::get_span(fexpr),
                                );
                            }
                        }
                    } else {
                        crate::diagnostics::report_error(
                            format!("Unknown field '{}' in instantiation of struct '{}'", fname, n),
                            crate::ast::get_span(expr),
                        );
                    }
                }
            } else {
                crate::diagnostics::report_error(
                    format!("Struct '{}' not found", n),
                    crate::ast::get_span(expr),
                );
            }
            n.clone()
        }
        Expr::MethodCall(obj, m, _) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            if let Some(intr) = lookup_builtin_intrinsic(&obj_ty, m) {
                intr.return_ty.to_string()
            } else {
                let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
                let actual_name = resolve_method_name(&resolved_obj_ty, m, &[], funcs);
                if let Some(f) = funcs.get(&actual_name) {
                    f.return_ty.to_string()
                } else if let Some(s_def) = structs.get(&resolved_obj_ty) {
                    if let Some(field) = s_def.fields.iter().find(|f| f.name == *m) {
                        let field_ty_str = field.ty.to_string();
                        if field_ty_str.starts_with("fn(") {
                            extract_fn_return_type(&field_ty_str).to_string()
                        } else {
                            "void".to_string()
                        }
                    } else {
                        "void".to_string()
                    }
                } else {
                    "void".to_string()
                }
            }
        }
        Expr::New(ty, _) => ty.to_string(),
        Expr::Call(n, args) => {
            if let Some(ty) = sym.get(n) {
                if ty.starts_with("fn(") {
                    if let Some(idx) = ty.rfind("):") {
                        return ty[idx+2..].to_string();
                    }
                }
            }
            let mut arg_types = Vec::new();
            for arg in args {
                arg_types.push(get_expr_type(arg, sym, funcs, structs));
            }
            let actual_name = resolve_func_name(n, &arg_types, funcs, structs);
            if let Some(f) = funcs.get(&actual_name) {
                return f.return_ty.to_string();
            }
            "i32".to_string()
        }
        Expr::If(_, then_b, _) => {
            let (_, t_val) = &**then_b;
            if let Some(v) = t_val {
                get_expr_type(v, sym, funcs, structs)
            } else {
                "void".to_string()
            }
        }
        Expr::Match(_, arms) => {
            if !arms.is_empty() && arms.iter().all(|a| a.val.is_some()) {
                if let Some(arm) = arms.iter().find(|a| a.val.is_some()) {
                    return get_expr_type(arm.val.as_ref().unwrap(), sym, funcs, structs);
                }
            }
            "void".to_string()
        }
        Expr::Default => "unknown".to_string(),
        Expr::Closure(cls) => panic!("Closures should be lifted before type generation: {:?}", cls),
        Expr::ClosureInstantiate(func_name, _, _) => {
            if let Some(f) = funcs.get(func_name) {
                let mut params_str = Vec::new();
                for p in &f.params {
                    if p.name != "__env" {
                        params_str.push(p.ty.to_string());
                    }
                }
                format!("fn({}):{}", params_str.join(","), f.return_ty)
            } else {
                "unknown".to_string()
            }
        }
        Expr::InvokeFuncPtr(func_expr, _) => {
            let func_ty = get_expr_type(func_expr, sym, funcs, structs);
            if func_ty.starts_with("fn(") {
                let inner = &func_ty[3..];
                if let Some(idx) = inner.find("):") {
                    return inner[idx+2..].to_string();
                }
            }
            "unknown".to_string()
        }
        Expr::Cast(_, target_ty) => target_ty.to_string(),
        Expr::Spread(e) => get_expr_type(e, sym, funcs, structs),
        Expr::Tuple(exprs) => {
            let mut tys = Vec::new();
            for e in exprs {
                tys.push(get_expr_type(e, sym, funcs, structs));
            }
            format!("({})", tys.join(","))
        }
        Expr::MapLit(pairs) => {
            if pairs.is_empty() {
                "Map<str, anyref>".to_string()
            } else {
                let first_k_ty = get_expr_type(&pairs[0].0, sym, funcs, structs);
                for (k, _) in pairs {
                    let k_ty = get_expr_type(k, sym, funcs, structs);
                    if k_ty != first_k_ty {
                        panic!("Map keys must all have the same type: found '{}' and '{}'", first_k_ty, k_ty);
                    }
                }
                
                let first_v_ty = get_expr_type(&pairs[0].1, sym, funcs, structs);
                let mut vals_same = true;
                for (_, v) in pairs {
                    let v_ty = get_expr_type(v, sym, funcs, structs);
                    if v_ty != first_v_ty {
                        vals_same = false;
                        break;
                    }
                }
                let v_ty = if vals_same { first_v_ty } else { "anyref".to_string() };
                format!("Map<{}, {}>", first_k_ty, v_ty)
            }
        }
    }
}

pub fn validate_match_patterns(opt_ty: &str, arms: &[MatchArm], structs: &HashMap<String, StructDef>) {
    let resolved_obj_ty = resolve_struct_name(opt_ty, structs);
    if let Some(s_def) = structs.get(&resolved_obj_ty) {
        if !s_def.is_enum {
            crate::diagnostics::report_error(format!("Cannot match on non-enum type '{}'", opt_ty), None);
            return;
        }
        let mut matched_variants = HashSet::new();
        let mut has_catch_all = false;
        for arm in arms {
            if let MatchPattern::CatchAll = arm.pattern {
                has_catch_all = true;
                continue;
            }
            let (variant_name, bindings_len) = match &arm.pattern {
                MatchPattern::Some(_) => ("Some".to_string(), 1),
                MatchPattern::None => ("None".to_string(), 0),
                MatchPattern::Ok(_) => ("Ok".to_string(), 1),
                MatchPattern::Err(_) => ("Err".to_string(), 1),
                MatchPattern::Variant(name, binds) => (name.rsplit("::").next().unwrap().to_string(), binds.len()),
                MatchPattern::CatchAll => unreachable!(),
            };
            matched_variants.insert(variant_name.clone());
            if !s_def.variants.contains(&variant_name) {
                if variant_name == "Some" || variant_name == "None" {
                    crate::diagnostics::report_error(format!("Cannot match Option patterns (Some/None) on non-Option type '{}'", opt_ty), None);
                } else if variant_name == "Ok" || variant_name == "Err" {
                    crate::diagnostics::report_error(format!("Cannot match Result patterns (Ok/Err) on non-Result type '{}'", opt_ty), None);
                } else {
                    crate::diagnostics::report_error(format!("Enum '{}' does not have variant '{}'", resolved_obj_ty, variant_name), None);
                }
                continue;
            }
            let expected_payload_len = s_def.fields.iter()
                .filter(|f| f.name.starts_with(&format!("{}_", variant_name)))
                .count();
            if bindings_len != expected_payload_len {
                crate::diagnostics::report_error(
                    format!(
                        "Pattern for variant '{}' expected {} bindings, found {}",
                        variant_name, expected_payload_len, bindings_len
                    ),
                    None
                );
            }
        }
        if !has_catch_all {
            let mut missing_variants = Vec::new();
            for variant in &s_def.variants {
                if !matched_variants.contains(variant) {
                    missing_variants.push(variant.clone());
                }
            }
            if !missing_variants.is_empty() {
                crate::diagnostics::report_error(
                    format!(
                        "Match expression must be exhaustive: missing {}",
                        missing_variants.join(", ")
                    ),
                    None
                );
            }
        }
    } else {
        crate::diagnostics::report_error(format!("Matched target type '{}' not found", resolved_obj_ty), None);
    }
}
