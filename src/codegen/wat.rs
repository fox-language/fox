use std::collections::{HashMap, HashSet};
use super::*;
use super::intrinsics::*;

pub fn generate_default_value_wat(
    expected_ty: &str,
    wat: &mut String,
    structs: &HashMap<String, StructDef>,
) {
    if expected_ty == "i32" || expected_ty == "u32" || expected_ty == "bool" || expected_ty == "byte" {
        wat.push_str("    i32.const 0\n");
    } else if expected_ty == "i64" || expected_ty == "u64" {
        wat.push_str("    i64.const 0\n");
    } else if expected_ty == "f32" {
        wat.push_str("    f32.const 0.0\n");
    } else if expected_ty == "f64" {
        wat.push_str("    f64.const 0.0\n");
    } else if expected_ty == "str" || expected_ty == "externref" {
        wat.push_str("    ref.null extern\n");
    } else if expected_ty == "anyref" {
        wat.push_str("    ref.null any\n");
    } else if expected_ty.starts_with("[]") {
        let inner = &expected_ty[2..];
        let resolved_inner = resolve_struct_name(inner, structs);
        wat.push_str(&format!("    ref.null $array_{}\n", sanitize_wat_name(&resolved_inner)));
    } else if expected_ty.starts_with("fn(") {
        let fat_name = fn_type_to_wasm_name(expected_ty);
        let sig_name = format!("sig_{}", fat_name);
        wat.push_str(&format!("    ref.null ${}\n    ref.null any\n    struct.new ${}\n", sig_name, fat_name));
    } else {
        let resolved = resolve_struct_name(expected_ty, structs);
        if resolved.starts_with("vec::Vec") || resolved.contains("vec::Vec") || resolved.contains("std_collections_vec_Vec") {
            let func_name = format!("{}::new", resolved);
            let safe_func_name = sanitize_wat_name(&func_name);
            wat.push_str(&format!("    call ${}\n", safe_func_name));
        } else if structs.contains_key(&resolved) {
            wat.push_str(&format!("    ref.null ${}\n", sanitize_wat_name(&resolved)));
        } else {
            panic!("Cannot infer default value for type '{}'", expected_ty);
        }
    }
}

pub fn generate_expr(
    expr: &Expr,
    sym: &HashMap<String, String>,
    expected_ty: &str,
    wat: &mut String,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    string_lit_ids: &HashMap<String, String>,
    loop_idx: &mut i32,
    varr_depth: &mut i32,
) {
    crate::diagnostics::set_current_span(crate::ast::get_span(expr));
    match expr {
        Expr::Identifier(n) => {
            if sym.contains_key(n) {
                let actual_ty = sym.get(n).cloned().unwrap_or_default();
                wat.push_str(&format!("    local.get ${}\n", n));
                if expected_ty == "anyref" && actual_ty != "anyref" {
                    emit_box_to_anyref(&actual_ty, wat, funcs, structs);
                } else {
                    emit_widening(wat, &actual_ty, expected_ty, structs);
                }
            } else if let Some(resolved) = resolve_const_name(n) {
                let const_ty = GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                });
                let safe_name = sanitize_wat_name(&resolved);
                wat.push_str(&format!("    global.get ${}\n", safe_name));
                if expected_ty == "anyref" && const_ty != "anyref" {
                    emit_box_to_anyref(&const_ty, wat, funcs, structs);
                } else {
                    emit_widening(wat, &const_ty, expected_ty, structs);
                }
            } else {
                wat.push_str(&format!("    local.get ${}\n", n));
                if expected_ty == "anyref" {
                    emit_box_to_anyref("i32", wat, funcs, structs);
                } else {
                    emit_widening(wat, "i32", expected_ty, structs);
                }
            }
        }
        Expr::Integer(v) => {
            let mut t = expected_ty.to_string();
            if t == "unknown" || t == "anyref" {
                t = "i32".to_string();
            } else {
                t = map_wasm_ty(expected_ty, structs);
            }
            wat.push_str(&format!("    {}.const {}\n", t, v));
            if expected_ty == "anyref" {
                emit_box_to_anyref("i32", wat, funcs, structs);
            }
        }
        Expr::Float(f) => {
            let mut t = expected_ty.to_string();
            if t == "unknown" || t == "anyref" || t == "f32" {
                t = "f32".to_string();
            } else {
                t = map_wasm_ty(expected_ty, structs);
            }
            wat.push_str(&format!("    {}.const {}\n", t, f));
            if expected_ty == "anyref" {
                emit_box_to_anyref("f32", wat, funcs, structs);
            }
        }
        Expr::StringLit(s) => {
            let id = string_lit_ids.get(s).expect("string literal not in map");
            wat.push_str(&format!("    global.get ${}\n", id));
            if expected_ty == "anyref" {
                emit_box_to_anyref("str", wat, funcs, structs);
            }
        }
        Expr::Bool(b) => {
            let val = if *b { 1 } else { 0 };
            wat.push_str(&format!("    i32.const {}\n", val));
            if expected_ty == "anyref" {
                emit_box_to_anyref("bool", wat, funcs, structs);
            }
        }
        Expr::Default => {
            if expected_ty == "unknown" || expected_ty.is_empty() {
                panic!("Cannot infer type of default expression");
            }
            generate_default_value_wat(expected_ty, wat, structs);
        }
        Expr::MethodCall(obj, method, args) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            if let Some(intr) = lookup_builtin_intrinsic(&obj_ty, method) {
                if intr.is_opcode {
                    if obj_ty.starts_with("[]") && method == "copy_from" {
                        let inner_ty = &obj_ty[2..];
                        let resolved_inner = resolve_struct_name(inner_ty, structs);
                        let array_type_name = sanitize_wat_name(&resolved_inner);
                        generate_expr(obj, sym, "unknown", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = if i < intr.param_wasm_tys.len() {
                                intr.param_wasm_tys[i]
                            } else {
                                "unknown"
                            };
                            generate_expr(arg, sym, arg_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        }
                        wat.push_str(&format!("    array.copy $array_{} $array_{}\n", array_type_name, array_type_name));
                    } else {
                        // Push self (the receiver), then any extra args, then emit
                        // the opcode. Self's Wasm type is `unknown` for arrays and
                        // matches `obj_ty` for primitives; the array case ignores it.
                        let self_ty = if obj_ty.starts_with("[]") { "unknown" } else { &obj_ty };
                        generate_expr(obj, sym, self_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = if i < intr.param_wasm_tys.len() {
                                intr.param_wasm_tys[i]
                            } else {
                                "unknown"
                            };
                            generate_expr(arg, sym, arg_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        }
                        if obj_ty.starts_with("[]") && method == "len" {
                            wat.push_str(&format!("    {}\n", intr.result_wasm));
                            if expected_ty == "f32" || expected_ty == "f64" {
                                wat.push_str(&format!("    {}.convert_i32_u\n", expected_ty));
                            }
                        } else {
                            wat.push_str(&format!("    {}\n", intr.result_wasm));
                        }
                    }
                } else {
                    // Import or hand-written Wasm helper. Self is `str`, then
                    // each param gets its declared Wasm type.
                    let self_ty = if intr.uses_wasm_helper { &obj_ty } else { "str" };
                    generate_expr(obj, sym, self_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    for (i, arg) in args.iter().enumerate() {
                        let arg_ty = if i < intr.param_wasm_tys.len() {
                            match intr.param_wasm_tys[i] {
                                "externref" => "str",
                                "i32" => "i32",
                                other => other,
                            }
                        } else {
                            "unknown"
                        };
                        generate_expr(arg, sym, arg_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    }
                    let wasm_fn = intr.wasm_fn.expect("import/helper must have wasm_fn");
                    wat.push_str(&format!("    call {}\n", wasm_fn));
                }
            } else {
                let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
                let actual_name = resolve_method_name(&resolved_obj_ty, method, &[], funcs);

                if funcs.contains_key(&actual_name) {
                    // Real method call
                    generate_expr(obj, sym, &resolved_obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    let param_types: Vec<String> = if let Some(f) = funcs.get(&actual_name) {
                        f.params.iter().skip(1).map(|p| p.ty.to_string()).collect()
                    } else {
                        vec!["unknown".to_string(); args.len()]
                    };
                    for (i, arg) in args.iter().enumerate() {
                        let expected = param_types.get(i).map(|s| s.as_str()).unwrap_or("unknown");
                        generate_expr(arg, sym, expected, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    }
                    let safe_name = sanitize_wat_name(&actual_name);
                    wat.push_str(&format!("    call ${}\n", safe_name));
                } else if let Some(s_def) = structs.get(&resolved_obj_ty) {
                    if let Some(field) = s_def.fields.iter().find(|f| f.name == *method) {
                        let field_ty_str = field.ty.to_string();
                        if field_ty_str.starts_with("fn(") {
                            // Field access + closure call
                            let temp_var = format!("_field_call_{}", *loop_idx);
                            *loop_idx += 1;
                            generate_expr(obj, sym, &resolved_obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                            wat.push_str(&format!("    struct.get ${} ${}\n", sanitize_wat_name(&resolved_obj_ty), method));
                            wat.push_str(&format!("    local.set ${}\n", temp_var));
                            for arg in args {
                                generate_expr(arg, sym, "unknown", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                            }
                            let fat_name = fn_type_to_wasm_name(&field_ty_str);
                            let sig_name = format!("sig_{}", fat_name);
                            wat.push_str(&format!("    local.get ${}\n", temp_var));
                            wat.push_str(&format!("    struct.get ${} 1\n", fat_name));
                            wat.push_str(&format!("    local.get ${}\n", temp_var));
                            wat.push_str(&format!("    struct.get ${} 0\n", fat_name));
                            wat.push_str(&format!("    call_ref ${}\n", sig_name));
                        } else {
                            panic!("Cannot call non-function field '{}' on type '{}'", method, resolved_obj_ty);
                        }
                    } else {
                        panic!("No method or callable field '{}' found on type '{}'", method, resolved_obj_ty);
                    }
                } else {
                    generate_expr(obj, sym, &resolved_obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    for arg in args {
                        generate_expr(arg, sym, "unknown", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    }
                    let safe_name = sanitize_wat_name(&actual_name);
                    wat.push_str(&format!("    call ${}\n", safe_name));
                }
            }
            let actual_ty = get_expr_type(expr, sym, funcs, structs);
            if expected_ty == "anyref" && actual_ty != "anyref" {
                emit_box_to_anyref(&actual_ty, wat, funcs, structs);
            } else {
                emit_widening(wat, &actual_ty, expected_ty, structs);
            }
        }
        Expr::FieldAccess(obj, f_name) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            generate_expr(obj, sym, &obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            if obj_ty.starts_with("[]") {
                panic!("Field access not supported on array type: {}.{}", obj_ty, f_name);
            }
            let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
            wat.push_str(&format!("    struct.get ${} ${}\n", sanitize_wat_name(&resolved_obj_ty), f_name));
            let actual_ty = get_expr_type(expr, sym, funcs, structs);
            if expected_ty == "anyref" && actual_ty != "anyref" {
                emit_box_to_anyref(&actual_ty, wat, funcs, structs);
            }
        }
        Expr::IndexAccess(arr, idx) => {
            let ty = get_expr_type(arr, sym, funcs, structs);
            generate_expr(arr, sym, &ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            generate_expr(idx, sym, "i32", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            if ty.starts_with("[]") {
                let inner = &ty[2..];
                let resolved_inner = resolve_struct_name(inner, structs);
                wat.push_str(&format!("    array.get $array_{}\n", sanitize_wat_name(&resolved_inner)));
                let actual_ty = inner.to_string();
                if expected_ty == "anyref" && actual_ty != "anyref" {
                    emit_box_to_anyref(&actual_ty, wat, funcs, structs);
                } else if inner == "i32" || inner == "byte" || inner == "bool" || inner == "u32" {
                    if expected_ty == "i64" || expected_ty == "u64" {
                        crate::diagnostics::report_error(format!("Type mismatch: expected '{}', found '{}'", expected_ty, inner), None);
                    }
                } else if inner == "i64" || inner == "u64" {
                    if expected_ty == "i32" || expected_ty == "u32" || expected_ty == "byte" || expected_ty == "bool" {
                        crate::diagnostics::report_error(format!("Type mismatch: expected '{}', found '{}'", expected_ty, inner), None);
                    }
                }
            } else {
                crate::diagnostics::report_error(format!("Indexing non-array type: {}", ty), None);
            }
        }
        Expr::New(ty, args) => {
            let ty_str = ty.to_string();
            if ty_str.starts_with("[]") {
                let inner = &ty_str[2..];
                if args.is_empty() {
                    wat.push_str("    i32.const 0\n");
                } else {
                    generate_expr(&args[0], sym, "i32", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                }
                let resolved_inner = resolve_struct_name(inner, structs);
                wat.push_str(&format!("    array.new_default $array_{}\n", sanitize_wat_name(&resolved_inner)));
            } else {
                panic!("new is only supported for arrays and slices");
            }
        }
        Expr::StructInit(s_name, fields) => {
            let resolved_name = resolve_struct_name(s_name, structs);
            if let Some(s) = structs.get(&resolved_name) {
                for s_field in &s.fields {
                    if let Some((_, expr)) = fields.iter().find(|(n, _)| n == &s_field.name) {
                        generate_expr(expr, sym, &s_field.ty.to_string(), wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    } else {
                        generate_default_value_wat(&s_field.ty.to_string(), wat, structs);
                    }
                }
                for (fname, fexpr) in fields {
                    if !s.fields.iter().any(|sf| &sf.name == fname) {
                        crate::diagnostics::report_error(
                            format!("Unknown field '{}' in instantiation of struct '{}'", fname, s_name),
                            crate::ast::get_span(fexpr),
                        );
                    }
                }
                wat.push_str(&format!(
                    "    struct.new ${}\n",
                    sanitize_wat_name(&resolved_name)
                ));
            } else {
                crate::diagnostics::report_error(
                    format!("Struct '{}' not found", s_name),
                    crate::ast::get_span(expr),
                );
            }
        }
        Expr::Call(name, args) => {
            if let Some(ty) = sym.get(name) {
                if ty.starts_with("fn(") {
                    let var_expr = Expr::Identifier(name.clone());
                    let invoke_expr = Expr::InvokeFuncPtr(Box::new(var_expr), args.clone());
                    generate_expr(&invoke_expr, sym, expected_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    return;
                }
            }
            emit_call(name, args, sym, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth, false, expected_ty);
            let mut arg_types = Vec::new();
            for arg in args {
                arg_types.push(get_expr_type(arg, sym, funcs, structs));
            }
            let actual_func_name = resolve_func_name_with_expected(name, &arg_types, expected_ty, funcs, structs);
            let actual_ty = if let Some(f) = funcs.get(&actual_func_name) {
                f.return_ty.to_string()
            } else {
                get_expr_type(expr, sym, funcs, structs)
            };
            emit_widening(wat, &actual_ty, expected_ty, structs);
        }
        Expr::If(cond, then_b, else_b) => {
            let (then_stmts, then_val) = &**then_b;
            let current_loop_idx = *loop_idx;
            *loop_idx += 1;
            
            let temp_var = format!("_if_val{}", current_loop_idx);
            let res_var = format!("_if_res{}", current_loop_idx);
            
            let has_value = then_val.is_some();
            
            let match_end = format!("_if_end{}", current_loop_idx);
            let else_label = format!("_if_else{}", current_loop_idx);

            wat.push_str(&format!("    block ${}\n", match_end));
            wat.push_str(&format!("    block ${}\n", else_label));
            
            let cond_ty = get_expr_type(cond, sym, funcs, structs);
            if cond_ty != "bool" {
                panic!("if condition must be a boolean");
            }
            generate_expr(cond, sym, "bool", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str("    i32.eqz\n");
            wat.push_str(&format!("    br_if ${}\n", else_label));
            
            for s in then_stmts { generate_stmt(s, sym, expected_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth); }
            if let Some(v) = then_val {
                generate_expr(v, sym, expected_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                wat.push_str(&format!("    local.set ${}\n", temp_var));
                wat.push_str(&format!("    i32.const 1\n    local.set ${}\n", res_var));
            }
            wat.push_str(&format!("    br ${}\n", match_end));
            
            wat.push_str("    end\n");
            if let Some(else_block) = else_b {
                let (else_stmts, else_val) = &**else_block;
                for s in else_stmts { generate_stmt(s, sym, expected_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth); }
                if let Some(v) = else_val {
                    generate_expr(v, sym, expected_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    wat.push_str(&format!("    local.set ${}\n", temp_var));
                    wat.push_str(&format!("    i32.const 1\n    local.set ${}\n", res_var));
                }
            }
            wat.push_str("    end\n");
            
            if has_value {
                wat.push_str(&format!("    local.get ${}\n", temp_var));
            }
        }
        Expr::Match(target, arms) => {
            let opt_ty = get_expr_type(target, sym, funcs, structs);
            validate_match_patterns(&opt_ty, arms, structs);
            if crate::diagnostics::has_errors() {
                return;
            }
            let resolved_obj_ty = resolve_struct_name(&opt_ty, structs);
            let temp_var = format!("_match_val{}", *loop_idx);
            let res_var = format!("_match_res{}", *loop_idx);
            let current_loop_idx = *loop_idx;
            *loop_idx += 1;

            generate_expr(target, sym, &opt_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str(&format!("    local.set ${}\n", temp_var));

            let match_end = format!("_match_end{}", current_loop_idx);
            let has_value = !arms.is_empty() && arms.iter().all(|a| a.val.is_some());

            wat.push_str(&format!("    block ${}\n", match_end));
            for i in (0..arms.len()).rev() {
                wat.push_str(&format!("    block $_match_arm_{}_{}\n", current_loop_idx, i));
            }

            for (i, arm) in arms.iter().enumerate() {
                if let MatchPattern::CatchAll = arm.pattern {
                    for bstmt in &arm.body {
                        generate_stmt(bstmt, sym, expected_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth);
                    }
                    if let Some(v) = &arm.val {
                        let vt = get_expr_type(v, sym, funcs, structs);
                        generate_expr(v, sym, &vt, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        if has_value {
                            wat.push_str(&format!("    local.set ${}\n", res_var));
                        } else if vt != "void" {
                            wat.push_str("    drop\n");
                        }
                    }
                    wat.push_str(&format!("    br ${}\n", match_end));
                    wat.push_str("    end\n");
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

                let s_def = structs.get(&resolved_obj_ty).unwrap_or_else(|| {
                    panic!("Enum '{}' not found in structs map", resolved_obj_ty);
                });

                let tag_val = s_def.variants.iter().position(|v| v == &variant_name).unwrap_or_else(|| {
                    panic!("Variant '{}' not found in enum '{}'", variant_name, resolved_obj_ty);
                });

                wat.push_str(&format!("    local.get ${}\n", temp_var));
                wat.push_str(&format!("    struct.get ${} $_tag\n", sanitize_wat_name(&resolved_obj_ty)));
                wat.push_str(&format!("    i32.const {}\n", tag_val));
                wat.push_str("    i32.ne\n");
                wat.push_str(&format!("    br_if $_match_arm_{}_{}\n", current_loop_idx, i));

                for (j, binding_name) in bindings.iter().enumerate() {
                    wat.push_str(&format!("    local.get ${}\n", temp_var));
                    wat.push_str(&format!("    struct.get ${} ${}_{}\n", sanitize_wat_name(&resolved_obj_ty), variant_name, j));
                    wat.push_str(&format!("    local.set ${}\n", binding_name));
                }

                for bstmt in &arm.body {
                    generate_stmt(bstmt, sym, expected_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth);
                }

                if let Some(v) = &arm.val {
                    let vt = get_expr_type(v, sym, funcs, structs);
                    generate_expr(v, sym, &vt, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    if has_value {
                        wat.push_str(&format!("    local.set ${}\n", res_var));
                    } else if vt != "void" {
                        wat.push_str("    drop\n");
                    }
                }

                wat.push_str(&format!("    br ${}\n", match_end));
                wat.push_str("    end\n");
            }

            wat.push_str("    end\n");

            if has_value {
                wat.push_str(&format!("    local.get ${}\n", res_var));
            }
        }
        Expr::Binary(l, op, r) => {
            let l_ty = get_expr_type(l, sym, funcs, structs);
            let r_ty = get_expr_type(r, sym, funcs, structs);
            let ty = if l_ty == "f64" || r_ty == "f64" {
                "f64".to_string()
            } else if l_ty == "f32" || r_ty == "f32" {
                "f32".to_string()
            } else if l_ty == "i64" || r_ty == "i64" {
                "i64".to_string()
            } else if l_ty == "i32" || l_ty == "unknown" {
                r_ty
            } else {
                l_ty
            };
            generate_expr(l, sym, &ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            generate_expr(r, sym, &ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);

            let is_ref = ty == "str" || ty.starts_with("[]") || structs.contains_key(&ty);
            let result_ty = if is_ref && (*op == Op::EqualEqual || *op == Op::NotEqual) {
                if ty == "str" {
                    wat.push_str("    call $fox_js_string_equals\n");
                } else {
                    wat.push_str("    ref.eq\n");
                }
                if *op == Op::NotEqual {
                    wat.push_str("    i32.eqz\n");
                }
                "bool".to_string()
            } else if ty == "str" {
                if matches!(op, Op::Add) {
                    wat.push_str("    call $fox_js_string_concat\n");
                } else {
                    panic!("Unsupported operator on str: {:?}", op);
                }
                "str".to_string()
            } else {
                let op_str = match op {
                    Op::Add => "add",
                    Op::Sub => "sub",
                    Op::Mul => "mul",
                    Op::Div => {
                        if ty == "f32" || ty == "f64" {
                            "div"
                        } else if ty == "u32" || ty == "u64" || ty == "byte" {
                            "div_u"
                        } else {
                            "div_s"
                        }
                    }
                    Op::Less => {
                        if ty == "f32" || ty == "f64" {
                            "lt"
                        } else if ty == "u32" || ty == "u64" || ty == "byte" {
                            "lt_u"
                        } else {
                            "lt_s"
                        }
                    }
                    Op::LessEqual => {
                        if ty == "f32" || ty == "f64" {
                            "le"
                        } else if ty == "u32" || ty == "u64" || ty == "byte" {
                            "le_u"
                        } else {
                            "le_s"
                        }
                    }
                    Op::Greater => {
                        if ty == "f32" || ty == "f64" {
                            "gt"
                        } else if ty == "u32" || ty == "u64" || ty == "byte" {
                            "gt_u"
                        } else {
                            "gt_s"
                        }
                    }
                    Op::GreaterEqual => {
                        if ty == "f32" || ty == "f64" {
                            "ge"
                        } else if ty == "u32" || ty == "u64" || ty == "byte" {
                            "ge_u"
                        } else {
                            "ge_s"
                        }
                    }
                    Op::EqualEqual => "eq",
                    Op::NotEqual => "ne",
                    Op::ShiftRight => {
                        if ty == "u32" || ty == "u64" || ty == "byte" {
                            "shr_u"
                        } else {
                            "shr_s"
                        }
                    }
                    Op::ShiftLeft => "shl",
                    Op::BitAnd => "and",
                    Op::BitXor => "xor",
                    Op::Rem => {
                        if ty == "u32" || ty == "u64" || ty == "byte" {
                            "rem_u"
                        } else {
                            "rem_s"
                        }
                    }
                };
                wat.push_str(&format!("    {}.{}\n", map_wasm_ty(&ty, structs), op_str));
                if matches!(op, Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual | Op::EqualEqual | Op::NotEqual) {
                    "bool".to_string()
                } else {
                    ty
                }
            };

            if expected_ty == "anyref" && result_ty != "anyref" {
                emit_box_to_anyref(&result_ty, wat, funcs, structs);
            } else {
                emit_widening(wat, &result_ty, expected_ty, structs);
            }
        }
        Expr::Closure(_) => panic!("Closures should be lifted before code generation"),
        Expr::ClosureInstantiate(func_name, env_name, captured_exprs) => {
            let mut new_wat = String::new();
            new_wat.push_str(&format!("    ref.func ${}\n", func_name));
            let resolved_env_name = resolve_struct_name(env_name, structs);
            let env_struct = structs.get(&resolved_env_name);
            for (i, e) in captured_exprs.iter().enumerate() {
                let temp_str;
                let expected_field_ty = if let Some(s) = env_struct {
                    if let Some(f) = s.fields.get(i) {
                        temp_str = f.ty.to_string();
                        &temp_str
                    } else {
                        "anyref"
                    }
                } else {
                    "anyref"
                };
                generate_expr(e, sym, expected_field_ty, &mut new_wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            }
            new_wat.push_str(&format!("    struct.new ${}\n", env_name));
            
            let func_def = funcs.get(func_name).unwrap_or_else(|| {
                eprintln!("Could not find closure func: {}", func_name);
                eprintln!("Available functions: {:?}", funcs.keys().collect::<Vec<_>>());
                panic!("Could not find closure func: {}", func_name);
            });
            let mut params_str = Vec::new();
            for p in &func_def.params {
                if p.name != "__env" {
                    params_str.push(p.ty.to_string());
                }
            }
            let fn_ty = format!("fn({}):{}", params_str.join(","), func_def.return_ty);
            let fat_name = fn_type_to_wasm_name(&fn_ty);
            new_wat.push_str(&format!("    struct.new ${}\n", fat_name));
            wat.push_str(&new_wat);
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            let temp_var = format!("_invoke_ptr_{}", *loop_idx);
            *loop_idx += 1;
            
            let mut invoke_wat = String::new();
            let func_ty = get_expr_type(func_expr, sym, funcs, structs);
            generate_expr(func_expr, sym, &func_ty, &mut invoke_wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            invoke_wat.push_str(&format!("    local.set ${}\n", temp_var));
            
            for a in args {
                generate_expr(a, sym, "unknown", &mut invoke_wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            }
            
            let fat_name = fn_type_to_wasm_name(&func_ty);
            let sig_name = format!("sig_{}", fat_name);
            
            invoke_wat.push_str(&format!("    local.get ${}\n", temp_var));
            invoke_wat.push_str(&format!("    struct.get ${} 1\n", fat_name));
            
            invoke_wat.push_str(&format!("    local.get ${}\n", temp_var));
            invoke_wat.push_str(&format!("    struct.get ${} 0\n", fat_name));
            invoke_wat.push_str(&format!("    call_ref ${}\n", sig_name));
            
            wat.push_str(&invoke_wat);
            
            let mut return_ty = "void".to_string();
            if func_ty.starts_with("fn(") {
                let inner = &func_ty[3..];
                if let Some(idx) = inner.find("):") {
                    return_ty = inner[idx+2..].to_string();
                }
            }
            emit_widening(wat, &return_ty, expected_ty, structs);
        }
        Expr::Cast(e, target_ty) => {
            let actual_ty = get_expr_type(e, sym, funcs, structs);
            generate_expr(e, sym, &actual_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            
            let target_ty_str = target_ty.to_string();
            let is_actual_64 = actual_ty == "i64" || actual_ty == "u64";
            let is_target_64 = target_ty_str == "i64" || target_ty_str == "u64";
            let is_actual_32 = actual_ty == "i32" || actual_ty == "u32" || actual_ty == "byte" || actual_ty == "bool";
            let is_target_32 = target_ty_str == "i32" || target_ty_str == "u32" || target_ty_str == "byte" || target_ty_str == "bool";

            if is_target_64 && is_actual_32 {
                wat.push_str("    i64.extend_i32_s\n");
            } else if is_target_32 && is_actual_64 {
                wat.push_str("    i32.wrap_i64\n");
            } else if target_ty_str == "anyref" {
                if actual_ty != "anyref" {
                    emit_box_to_anyref(&actual_ty, wat, funcs, structs);
                }
            } else if structs.contains_key(&target_ty_str) || target_ty_str.starts_with("[]") || target_ty_str.starts_with("fn(") {
                let wasm_ty = map_wasm_ty(&target_ty_str, structs);
                wat.push_str(&format!("    ref.cast {}\n", wasm_ty));
            }
        }
        Expr::Spread(e) => {
            generate_expr(e, sym, expected_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
        }
        Expr::Tuple(exprs) => {
            let tuple_ty = get_expr_type(expr, sym, funcs, structs);
            let resolved_name = resolve_struct_name(&tuple_ty, structs);
            let s = structs.get(&resolved_name).unwrap_or_else(|| panic!("Tuple struct definition not found for: {}", tuple_ty));
            for (idx, e) in exprs.iter().enumerate() {
                let s_field = &s.fields[idx];
                generate_expr(e, sym, &s_field.ty.to_string(), wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            }
            wat.push_str(&format!(
                "    struct.new ${}\n",
                sanitize_wat_name(&resolved_name)
            ));
        }
        Expr::MapLit(pairs) => {
            let mut map_ty = get_expr_type(expr, sym, funcs, structs);
            if expected_ty.starts_with("Map<") || expected_ty.starts_with("Map_") || expected_ty.contains("Map") {
                map_ty = expected_ty.to_string();
            }
            let (k_ty, v_ty) = if map_ty.starts_with("Map<") {
                if let Some(start) = map_ty.find('<') {
                    if let Some(comma) = map_ty.find(',') {
                        if let Some(end) = map_ty.find('>') {
                            (map_ty[start+1..comma].trim().to_string(), map_ty[comma+1..end].trim().to_string())
                        } else {
                            ("str".to_string(), "anyref".to_string())
                        }
                    } else {
                        ("str".to_string(), "anyref".to_string())
                    }
                } else {
                    ("str".to_string(), "anyref".to_string())
                }
            } else if map_ty.starts_with("Map_") {
                let parts: Vec<&str> = map_ty.split('_').collect();
                if parts.len() >= 3 {
                    (parts[1].to_string(), parts[2..].join("_"))
                } else {
                    ("str".to_string(), "anyref".to_string())
                }
            } else {
                ("str".to_string(), "anyref".to_string())
            };

            let mono_map_struct = resolve_struct_name(&map_ty, structs);
            let temp_var = format!("_map_lit_tmp{}", *loop_idx);
            *loop_idx += 1;

            let new_func_name = format!("{}::new", mono_map_struct);
            let sanitized_new_fn = sanitize_wat_name(&new_func_name);
            wat.push_str(&format!("    call ${}\n", sanitized_new_fn));
            wat.push_str(&format!("    local.set ${}\n", temp_var));

            let set_func_name = format!("{}::set", mono_map_struct);
            let sanitized_set_fn = sanitize_wat_name(&set_func_name);

            for (k, v) in pairs {
                wat.push_str(&format!("    local.get ${}\n", temp_var));
                generate_expr(k, sym, &k_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                generate_expr(v, sym, &v_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                wat.push_str(&format!("    call ${}\n", sanitized_set_fn));
            }

            wat.push_str(&format!("    local.get ${}\n", temp_var));
        }
        Expr::VecLit(elems) => {
            let mut vec_ty = get_expr_type(expr, sym, funcs, structs);
            if expected_ty.starts_with("Vec<") || expected_ty.starts_with("Vec_") || expected_ty.contains("Vec") {
                vec_ty = expected_ty.to_string();
            }
            let el_ty = if vec_ty.starts_with("Vec<") {
                if let Some(start) = vec_ty.find('<') {
                    if let Some(end) = vec_ty.find('>') {
                        vec_ty[start+1..end].trim().to_string()
                    } else {
                        "anyref".to_string()
                    }
                } else {
                    "anyref".to_string()
                }
            } else if vec_ty.starts_with("Vec_") {
                let parts: Vec<&str> = vec_ty.split('_').collect();
                if parts.len() >= 2 {
                    parts[1..].join("_")
                } else {
                    "anyref".to_string()
                }
            } else {
                "anyref".to_string()
            };

            let mono_vec_struct = resolve_struct_name(&vec_ty, structs);
            let temp_var = format!("_vec_lit_tmp{}", *loop_idx);
            *loop_idx += 1;

            if elems.is_empty() {
                let new_func_name = format!("{}::new", mono_vec_struct);
                let sanitized_new_fn = sanitize_wat_name(&new_func_name);
                wat.push_str(&format!("    call ${}\n", sanitized_new_fn));
            } else {
                let with_cap_func_name = format!("{}::with_cap", mono_vec_struct);
                let sanitized_with_cap_fn = sanitize_wat_name(&with_cap_func_name);
                wat.push_str(&format!("    i32.const {}\n", elems.len()));
                wat.push_str(&format!("    call ${}\n", sanitized_with_cap_fn));
            }
            wat.push_str(&format!("    local.set ${}\n", temp_var));

            if !elems.is_empty() {
                let push_func_name = format!("{}::push", mono_vec_struct);
                let sanitized_push_fn = sanitize_wat_name(&push_func_name);

                for el in elems {
                    wat.push_str(&format!("    local.get ${}\n", temp_var));
                    generate_expr(el, sym, &el_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    wat.push_str(&format!("    call ${}\n", sanitized_push_fn));
                }
            }

            wat.push_str(&format!("    local.get ${}\n", temp_var));
        }
    }
}

pub fn emit_variadic_packing(
    variadic_args: &[Expr],
    varr_depth: &mut i32,
    sym: &HashMap<String, String>,
    wat: &mut String,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    string_lit_ids: &HashMap<String, String>,
    loop_idx: &mut i32,
    target_ty: &str,
) {
    let n_variadic = variadic_args.len();
    let my_temp = *varr_depth;
    *varr_depth += 1;
    let tmp_name = format!("_varr_{}", my_temp);

    // Build array of size n_variadic, populated by side-effecting array.set
    // (stack grows downward: we emit val, idx, arr, then array.set).
    let resolved_target = resolve_struct_name(target_ty, structs);
    let sanitized_target = sanitize_wat_name(&resolved_target);
    wat.push_str(&format!("    i32.const {}\n", n_variadic));
    wat.push_str(&format!("    array.new_default $array_{}\n", sanitized_target));
    wat.push_str(&format!("    local.set ${}\n", tmp_name));

    for (i, arg) in variadic_args.iter().enumerate() {
        let arg_ty = get_expr_type(arg, sym, funcs, structs);
        // Push the array, then idx, then the boxed value, then array.set
        wat.push_str(&format!("    local.get ${}\n", tmp_name));
        wat.push_str(&format!("    i32.const {}\n", i));
        generate_expr(arg, sym, &arg_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
        if target_ty == "anyref" {
            emit_box_to_anyref(&arg_ty, wat, funcs, structs);
        }
        wat.push_str(&format!("    array.set $array_{}\n", sanitized_target));
    }

    // Leave the array on the stack
    wat.push_str(&format!("    local.get ${}\n", tmp_name));
    *varr_depth -= 1;
}

pub fn emit_box_to_anyref(
    arg_ty: &str,
    wat: &mut String,
    _funcs: &HashMap<String, Function>,
    _structs: &HashMap<String, StructDef>,
) {
    match arg_ty {
        "str" => {
            wat.push_str("    any.convert_extern\n");
        }
        "anyref" => {
            // Already anyref, no conversion needed.
        }
        "f32" => {
            wat.push_str("    i32.reinterpret_f32\n");
            wat.push_str("    ref.i31\n");
        }
        "f64" => {
            wat.push_str("    call $__fox_f64_to_str\n");
            wat.push_str("    any.convert_extern\n");
        }
        "byte" | "u32" | "bool" => {
            wat.push_str("    ref.i31\n");
        }
        "i32" => {
            wat.push_str("    ref.i31\n");
        }
        "i64" | "u64" => {
            wat.push_str("    i32.wrap_i64\n");
            wat.push_str("    ref.i31\n");
        }
        ty if ty.starts_with("[]")
            || ty.starts_with("fn(")
            || ty.contains('<')
            || ty.chars().next().map_or(false, |c| c.is_ascii_uppercase())
            || _structs.contains_key(&resolve_struct_name(ty, _structs)) =>
        {
            // GC reference types (structs, arrays, functions) are already subtypes of anyref in Wasm GC.
            // No boxing instructions are needed.
        }
        _ => {
            // Fallback: i31 for everything else (get truncated/clamped).
            // For unsupported types the runtime will see a malformed i31 and skip the
            // spec, which is better than a Wasm type error.
            wat.push_str("    ref.i31\n");
        }
    }
}

pub fn emit_call(
    name: &str,
    args: &[Expr],
    sym: &HashMap<String, String>,
    wat: &mut String,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    string_lit_ids: &HashMap<String, String>,
    loop_idx: &mut i32,
    varr_depth: &mut i32,
    is_return_call: bool,
    expected_ret_ty: &str,
) {
    let mut arg_types = Vec::new();
    for arg in args {
        arg_types.push(get_expr_type(arg, sym, funcs, structs));
    }
    let mut actual_name = resolve_func_name_with_expected(name, &arg_types, expected_ret_ty, funcs, structs);
    // If the resolved name isn't in funcs, search for a matching key by suffix
    if !funcs.contains_key(&actual_name) {
        let base = if let Some(start) = actual_name.find('<') {
            let end = actual_name.rfind('>').unwrap_or(start);
            format!("{}{}", &actual_name[..start], &actual_name[end+1..])
        } else {
            actual_name.clone()
        };
        let candidates: Vec<&String> = funcs.keys().filter(|k| k.ends_with(&format!("::{}", base)) || **k == base).collect();
        if let Some(best) = pick_best_candidate(candidates, &arg_types, expected_ret_ty, funcs, structs) {
            actual_name = best.clone();
        }
    }
    let target_func = funcs.get(&actual_name);
    let is_variadic = target_func.map(|f| f.is_variadic()).unwrap_or(false);
    let fixed_arity = if is_variadic {
        target_func.unwrap().params.len() - 1
    } else {
        args.len()
    };

    let param_types: Vec<String> = if let Some(f) = target_func {
        f.params.iter().map(|p| p.ty.to_string()).collect()
    } else {
        vec!["unknown".to_string(); args.len()]
    };

    for (i, arg) in args.iter().take(fixed_arity).enumerate() {
        let expected = param_types.get(i).map(|s| s.as_str()).unwrap_or("unknown");
        generate_expr(arg, sym, expected, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
    }

    if is_variadic {
        let variadic_args: Vec<Expr> = args.iter().skip(fixed_arity).cloned().collect();
        if variadic_args.len() == 1 {
            if let Expr::Spread(ref e) = variadic_args[0] {
                let target_ty = target_func.unwrap().params.last().unwrap().ty.to_string();
                generate_expr(e, sym, &format!("[]{}", target_ty), wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            } else {
                let target_ty = target_func.unwrap().params.last().unwrap().ty.to_string();
                emit_variadic_packing(&variadic_args, varr_depth, sym, wat, funcs, structs, string_lit_ids, loop_idx, &target_ty);
            }
        } else {
            let target_ty = target_func.unwrap().params.last().unwrap().ty.to_string();
            emit_variadic_packing(&variadic_args, varr_depth, sym, wat, funcs, structs, string_lit_ids, loop_idx, &target_ty);
        }
    }

    if let Some(f) = target_func {
        if f.is_compiler && !actual_name.ends_with("::sprintf") && actual_name != "sprintf" {
            // A free function declared as `pub compiler fn` that isn't a
            // known builtin method. The `str`/`f64`/etc. intrinsics live on
            // builtin types as methods and are dispatched through
            // `lookup_builtin_intrinsic` in `generate_expr`; this branch
            // only fires for top-level compiler fns that future stdlib
            // additions might introduce.
            panic!(
                "Unsupported top-level compiler intrinsic: {} (intrinsics on builtin types must be declared as `impl <Type> {{ pub compiler fn ... }}` and live in std::builtin)",
                actual_name
            );
        }
    }

    let safe_name = sanitize_wat_name(&actual_name);
    let call_instr = if is_return_call { "return_call" } else { "call" };
    wat.push_str(&format!("    {} ${}\n", call_instr, safe_name));
}

pub fn has_variadic_call_stmt(
    stmt: &Stmt,
    sym: &HashMap<String, String>,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> bool {
    match stmt {
        Stmt::Let(_, _, e) => has_variadic_call_expr(e, sym, funcs, structs),
        Stmt::LetTuple(_, e) => has_variadic_call_expr(e, sym, funcs, structs),
        Stmt::ExprStmt(e) => has_variadic_call_expr(e, sym, funcs, structs),
        Stmt::Return(Some(e)) => has_variadic_call_expr(e, sym, funcs, structs),
        Stmt::Return(None) => false,
        Stmt::Assign(_, e) | Stmt::AssignPlus(_, e) => has_variadic_call_expr(e, sym, funcs, structs),
        Stmt::AssignIndex(a, i, v) => {
            has_variadic_call_expr(a, sym, funcs, structs)
                || has_variadic_call_expr(i, sym, funcs, structs)
                || has_variadic_call_expr(v, sym, funcs, structs)
        }
        Stmt::AssignField(o, _, v) => {
            has_variadic_call_expr(o, sym, funcs, structs) || has_variadic_call_expr(v, sym, funcs, structs)
        }
        Stmt::If(c, t, e) => {
            has_variadic_call_expr(c, sym, funcs, structs)
                || t.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs))
                || e.as_ref().map(|eb| eb.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs))).unwrap_or(false)
        }
        Stmt::While(c, b) => {
            has_variadic_call_expr(c, sym, funcs, structs)
                || b.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs))
        }
        Stmt::For(_, _, b) => b.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs)),
    }
}

pub fn has_variadic_call_expr(
    expr: &Expr,
    sym: &HashMap<String, String>,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) -> bool {
    match expr {
        Expr::Call(name, args) => {
            let mut arg_types = Vec::new();
            for arg in args {
                arg_types.push(get_expr_type(arg, sym, funcs, structs));
            }
            let actual = resolve_func_name(name, &arg_types, funcs, structs);
            if let Some(f) = funcs.get(&actual) {
                if f.is_variadic() {
                    return true;
                }
            }
            args.iter().any(|a| has_variadic_call_expr(a, sym, funcs, structs))
        }
        Expr::Binary(l, _, r) => has_variadic_call_expr(l, sym, funcs, structs) || has_variadic_call_expr(r, sym, funcs, structs),
        Expr::MethodCall(o, _, args) => {
            has_variadic_call_expr(o, sym, funcs, structs)
                || args.iter().any(|a| has_variadic_call_expr(a, sym, funcs, structs))
        }
        Expr::FieldAccess(o, _) => has_variadic_call_expr(o, sym, funcs, structs),
        Expr::IndexAccess(a, i) => {
            has_variadic_call_expr(a, sym, funcs, structs) || has_variadic_call_expr(i, sym, funcs, structs)
        }
        Expr::StructInit(_, fs) => fs.iter().any(|(_, e)| has_variadic_call_expr(e, sym, funcs, structs)),
        Expr::New(_, args) => args.iter().any(|a| has_variadic_call_expr(a, sym, funcs, structs)),
        Expr::If(cond, then_b, else_b) => {
            has_variadic_call_expr(cond, sym, funcs, structs) || {
                let (t_stmts, t_val) = &**then_b;
                t_stmts.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs)) ||
                t_val.as_ref().map(|v| has_variadic_call_expr(v, sym, funcs, structs)).unwrap_or(false)
            } || else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                e_stmts.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs)) ||
                e_val.as_ref().map(|v| has_variadic_call_expr(v, sym, funcs, structs)).unwrap_or(false)
            }).unwrap_or(false)
        }
        Expr::Match(target, arms) => {
            has_variadic_call_expr(target, sym, funcs, structs)
                || arms.iter().any(|arm| {
                    arm.body.iter().any(|s| has_variadic_call_stmt(s, sym, funcs, structs))
                        || arm.val.as_ref().map(|v| has_variadic_call_expr(v, sym, funcs, structs)).unwrap_or(false)
                })
        }
        Expr::Spread(e) => has_variadic_call_expr(e, sym, funcs, structs),
        Expr::Tuple(exprs) => exprs.iter().any(|e| has_variadic_call_expr(e, sym, funcs, structs)),
        Expr::MapLit(pairs) => pairs.iter().any(|(k, v)| has_variadic_call_expr(k, sym, funcs, structs) || has_variadic_call_expr(v, sym, funcs, structs)),
        Expr::VecLit(elems) => elems.iter().any(|e| has_variadic_call_expr(e, sym, funcs, structs)),
        _ => false,
    }
}

pub fn scan_locals_expr(
    expr: &Expr,
    sym: &mut HashMap<String, String>,
    locals: &mut Vec<(String, String)>,
    loop_idx: &mut i32,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) {
    match expr {
        Expr::If(cond, then_b, else_b) => {
            let temp_var = format!("_if_val{}", *loop_idx);
            let res_var = format!("_if_res{}", *loop_idx);
            *loop_idx += 1;

            let (then_stmts, then_val) = &**then_b;
            scan_locals_expr(cond, sym, locals, loop_idx, funcs, structs);
            for s in then_stmts { scan_locals(s, sym, locals, loop_idx, funcs, structs); }
            if let Some(v) = then_val {
                scan_locals_expr(v, sym, locals, loop_idx, funcs, structs);
                let ty = get_expr_type(v, sym, funcs, structs);
                if !sym.contains_key(&temp_var) {
                    sym.insert(temp_var.clone(), ty.clone());
                    locals.push((temp_var, ty));
                }
                if !sym.contains_key(&res_var) {
                    sym.insert(res_var.clone(), "i32".to_string());
                    locals.push((res_var, "i32".to_string()));
                }
            }
            if let Some(eb) = else_b {
                let (else_stmts, else_val) = &**eb;
                for s in else_stmts { scan_locals(s, sym, locals, loop_idx, funcs, structs); }
                if let Some(v) = else_val { scan_locals_expr(v, sym, locals, loop_idx, funcs, structs); }
            }
        }
        Expr::Match(target, arms) => {
            let opt_ty = get_expr_type(target, sym, funcs, structs);
            validate_match_patterns(&opt_ty, arms, structs);
            if crate::diagnostics::has_errors() {
                return;
            }
            let temp_var = format!("_match_val{}", *loop_idx);
            let res_var = format!("_match_res{}", *loop_idx);
            let current_loop_idx = *loop_idx;
            *loop_idx += 1;

            if !sym.contains_key(&temp_var) {
                sym.insert(temp_var.clone(), opt_ty.clone());
                locals.push((temp_var, opt_ty.clone()));
            }

            let has_value = !arms.is_empty() && arms.iter().all(|a| a.val.is_some());
            if has_value {
                if let Some(vt) = arms.iter().find_map(|a| a.val.as_ref().map(|v| get_expr_type(v, sym, funcs, structs))) {
                    if !sym.contains_key(&res_var) {
                        sym.insert(res_var.clone(), vt.clone());
                        locals.push((res_var, vt));
                    }
                }
            }
            let _ = current_loop_idx;

            scan_locals_expr(target, sym, locals, loop_idx, funcs, structs);

            let resolved_ty = resolve_struct_name(&opt_ty, structs);

            for arm in arms {
                if let MatchPattern::CatchAll = arm.pattern {
                    for bstmt in &arm.body {
                        scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
                    }
                    if let Some(v) = &arm.val {
                        scan_locals_expr(v, sym, locals, loop_idx, funcs, structs);
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

                if let Some(s_def) = structs.get(&resolved_ty) {
                    for (j, binding_name) in bindings.iter().enumerate() {
                        let field_name = format!("{}_{}", variant_name, j);
                        if let Some(f) = s_def.fields.iter().find(|f| f.name == field_name) {
                            if !sym.contains_key(binding_name) {
                                sym.insert(binding_name.clone(), f.ty.to_string());
                                locals.push((binding_name.clone(), f.ty.to_string()));
                            }
                        } else {
                            panic!("Variant field '{}' not found in struct '{}'", field_name, resolved_ty);
                        }
                    }
                }

                for bstmt in &arm.body {
                    scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
                }
                if let Some(v) = &arm.val {
                    scan_locals_expr(v, sym, locals, loop_idx, funcs, structs);
                }
            }
        }
        Expr::Binary(l, _, r) => {
            scan_locals_expr(l, sym, locals, loop_idx, funcs, structs);
            scan_locals_expr(r, sym, locals, loop_idx, funcs, structs);
        }
        Expr::MethodCall(obj, method, args) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
            let actual_name = resolve_method_name(&resolved_obj_ty, method, &[], funcs);
            if !funcs.contains_key(&actual_name) {
                if let Some(s_def) = structs.get(&resolved_obj_ty) {
                    if let Some(field) = s_def.fields.iter().find(|f| f.name == *method) {
                        let field_ty_str = field.ty.to_string();
                        if field_ty_str.starts_with("fn(") {
                            let temp_var = format!("_field_call_{}", *loop_idx);
                            if !sym.contains_key(&temp_var) {
                                sym.insert(temp_var.clone(), field_ty_str.clone());
                                locals.push((temp_var, field_ty_str));
                            }
                            *loop_idx += 1;
                        }
                    }
                }
            }
            scan_locals_expr(obj, sym, locals, loop_idx, funcs, structs);
            for a in args {
                scan_locals_expr(a, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::FieldAccess(obj, _) => {
            scan_locals_expr(obj, sym, locals, loop_idx, funcs, structs);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields {
                scan_locals_expr(e, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::Call(name, args) => {
            let ty_opt = sym.get(name).cloned();
            if let Some(ty) = ty_opt {
                if ty.starts_with("fn(") {
                    let temp_var = format!("_invoke_ptr_{}", *loop_idx);
                    if !sym.contains_key(&temp_var) {
                        sym.insert(temp_var.clone(), ty.clone());
                        locals.push((temp_var, ty.clone()));
                    }
                    *loop_idx += 1;
                    for a in args {
                        scan_locals_expr(a, sym, locals, loop_idx, funcs, structs);
                    }
                    return;
                }
            }
            for a in args {
                scan_locals_expr(a, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::IndexAccess(arr, idx) => {
            scan_locals_expr(arr, sym, locals, loop_idx, funcs, structs);
            scan_locals_expr(idx, sym, locals, loop_idx, funcs, structs);
        }
        Expr::New(_, args) => {
            for a in args {
                scan_locals_expr(a, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            let temp_var = format!("_invoke_ptr_{}", *loop_idx);
            let ty = get_expr_type(func_expr, sym, funcs, structs);
            if !sym.contains_key(&temp_var) {
                sym.insert(temp_var.clone(), ty.clone());
                locals.push((temp_var, ty));
            }
            *loop_idx += 1;
            scan_locals_expr(func_expr, sym, locals, loop_idx, funcs, structs);
            for a in args {
                scan_locals_expr(a, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::ClosureInstantiate(_, _, captured) => {
            for c in captured {
                scan_locals_expr(c, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::Spread(e) => {
            scan_locals_expr(e, sym, locals, loop_idx, funcs, structs);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                scan_locals_expr(e, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::MapLit(pairs) => {
            let mut map_ty = get_expr_type(expr, sym, funcs, structs);
            let expected = CURRENT_EXPECTED_TYPE.with(|c| c.borrow().clone());
            if expected.starts_with("Map<") || expected.starts_with("Map_") || expected.contains("Map") {
                map_ty = expected;
            }
            let temp_var = format!("_map_lit_tmp{}", *loop_idx);
            *loop_idx += 1;

            if !sym.contains_key(&temp_var) {
                sym.insert(temp_var.clone(), map_ty.clone());
                locals.push((temp_var, map_ty));
            }

            for (k, v) in pairs {
                scan_locals_expr(k, sym, locals, loop_idx, funcs, structs);
                scan_locals_expr(v, sym, locals, loop_idx, funcs, structs);
            }
        }
        Expr::VecLit(elems) => {
            let mut vec_ty = get_expr_type(expr, sym, funcs, structs);
            let expected = CURRENT_EXPECTED_TYPE.with(|c| c.borrow().clone());
            if expected.starts_with("Vec<") || expected.starts_with("Vec_") || expected.contains("Vec") {
                vec_ty = expected;
            }
            let temp_var = format!("_vec_lit_tmp{}", *loop_idx);
            *loop_idx += 1;

            if !sym.contains_key(&temp_var) {
                sym.insert(temp_var.clone(), vec_ty.clone());
                locals.push((temp_var, vec_ty));
            }

            for el in elems {
                scan_locals_expr(el, sym, locals, loop_idx, funcs, structs);
            }
        }
        _ => {}
    }
}

pub fn scan_locals(
    stmt: &Stmt,
    sym: &mut HashMap<String, String>,
    locals: &mut Vec<(String, String)>,
    loop_idx: &mut i32,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
) {
    match stmt {
        Stmt::Let(name, ty_opt, expr) => {
            let ty = ty_opt
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| get_expr_type(expr, sym, funcs, structs));
            if !sym.contains_key(name) {
                sym.insert(name.clone(), ty.clone());
                locals.push((name.clone(), ty.clone()));
            }
            CURRENT_EXPECTED_TYPE.with(|c| *c.borrow_mut() = ty);
            scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
            CURRENT_EXPECTED_TYPE.with(|c| *c.borrow_mut() = "".to_string());
        }
        Stmt::LetTuple(bindings, expr) => {
            let tuple_ty = get_expr_type(expr, sym, funcs, structs);
            let sub_tys = if tuple_ty.starts_with('(') && tuple_ty.ends_with(')') {
                split_types(&tuple_ty[1..tuple_ty.len() - 1])
            } else {
                Vec::new()
            };
            for (i, (var_name, var_ty)) in bindings.iter().enumerate() {
                let inferred_ty = if var_ty.to_string().is_empty() {
                    if i < sub_tys.len() {
                        sub_tys[i].clone()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    var_ty.to_string()
                };
                if !sym.contains_key(var_name) {
                    sym.insert(var_name.clone(), inferred_ty.clone());
                    locals.push((var_name.clone(), inferred_ty.clone()));
                }
            }
            let temp_var = format!("_tuple_tmp_{}", loop_idx);
            *loop_idx += 1;
            if !sym.contains_key(&temp_var) {
                sym.insert(temp_var.clone(), tuple_ty.clone());
                locals.push((temp_var, tuple_ty));
            }
            scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
        }
        Stmt::For(item_var, arr_var, body) => {
            if !sym.contains_key(item_var) {
                let item_ty = if let Some(arr_ty) = sym.get(arr_var) {
                    if arr_ty.starts_with("[]") {
                        arr_ty[2..].to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                sym.insert(item_var.clone(), item_ty.clone());
                locals.push((item_var.clone(), item_ty));
            }
            let i_var = format!("_i{}", loop_idx);
            let len_var = format!("_len{}", loop_idx);
            sym.insert(i_var.clone(), "i32".to_string());
            locals.push((i_var, "i32".to_string()));
            sym.insert(len_var.clone(), "i32".to_string());
            locals.push((len_var, "i32".to_string()));
            *loop_idx += 1;
            for bstmt in body {
                scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
            }
        }
        Stmt::If(cond, body, else_body) => {
            *loop_idx += 1;
            if else_body.is_some() {
                *loop_idx += 1;
            }
            scan_locals_expr(cond, sym, locals, loop_idx, funcs, structs);
            for bstmt in body {
                scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
            }
            if let Some(e_body) = else_body {
                for bstmt in e_body {
                    scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
                }
            }
        }
        Stmt::While(cond, body) => {
            *loop_idx += 1;
            scan_locals_expr(cond, sym, locals, loop_idx, funcs, structs);
            for bstmt in body {
                scan_locals(bstmt, sym, locals, loop_idx, funcs, structs);
            }
        }
        Stmt::ExprStmt(expr) => {
            scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
            }
        }
        Stmt::Assign(name, expr) => {
            if let Some(ty) = sym.get(name) {
                CURRENT_EXPECTED_TYPE.with(|c| *c.borrow_mut() = ty.clone());
            } else if let Some(resolved) = resolve_const_name(name) {
                let const_ty = GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                });
                CURRENT_EXPECTED_TYPE.with(|c| *c.borrow_mut() = const_ty);
            }
            scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
            CURRENT_EXPECTED_TYPE.with(|c| *c.borrow_mut() = "".to_string());
        }
        Stmt::AssignPlus(_, expr) => {
            scan_locals_expr(expr, sym, locals, loop_idx, funcs, structs);
        }
        Stmt::AssignIndex(arr, idx, val) => {
            scan_locals_expr(arr, sym, locals, loop_idx, funcs, structs);
            scan_locals_expr(idx, sym, locals, loop_idx, funcs, structs);
            scan_locals_expr(val, sym, locals, loop_idx, funcs, structs);
        }
        Stmt::AssignField(obj, _, val) => {
            scan_locals_expr(obj, sym, locals, loop_idx, funcs, structs);
            scan_locals_expr(val, sym, locals, loop_idx, funcs, structs);
        }
    }
}

pub fn is_pure_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Integer(_) | Expr::Float(_) | Expr::Identifier(_) | Expr::StringLit(_) | Expr::Bool(_) => true,
        Expr::Binary(l, op, r) => {
            if matches!(op, Op::Div | Op::Rem) {
                false
            } else {
                is_pure_expr(l) && is_pure_expr(r)
            }
        }
        Expr::FieldAccess(obj, _) => is_pure_expr(obj),
        _ => false,
    }
}

pub fn generate_stmt(
    stmt: &Stmt,
    sym: &HashMap<String, String>,
    expected_return_ty: &str,
    loop_idx: &mut i32,
    wat: &mut String,
    funcs: &HashMap<String, Function>,
    structs: &HashMap<String, StructDef>,
    string_lit_ids: &HashMap<String, String>,
    varr_depth: &mut i32,
) {
    crate::diagnostics::set_current_span(crate::ast::get_span(stmt));
    match stmt {
        Stmt::Let(name, _, expr) => {
            let ty = sym.get(name).unwrap();
            generate_expr(expr, sym, ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str(&format!("    local.set ${}\n", name));
        }
        Stmt::LetTuple(bindings, expr) => {
            let tuple_ty = get_expr_type(expr, sym, funcs, structs);
            let temp_var = format!("_tuple_tmp_{}", loop_idx);
            *loop_idx += 1;
            
            generate_expr(expr, sym, &tuple_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str(&format!("    local.set ${}\n", temp_var));
            
            let resolved_name = resolve_struct_name(&tuple_ty, structs);
            for (idx, (var_name, _)) in bindings.iter().enumerate() {
                wat.push_str(&format!("    local.get ${}\n", temp_var));
                wat.push_str(&format!(
                    "    struct.get ${} $f{}\n",
                    sanitize_wat_name(&resolved_name),
                    idx
                ));
                wat.push_str(&format!("    local.set ${}\n", var_name));
            }
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                if let Expr::Call(name, args) = expr {
                    emit_call(name, args, sym, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth, true, expected_return_ty);
                } else if let Expr::MethodCall(obj, method, args) = expr {
                    let obj_ty = get_expr_type(obj, sym, funcs, structs);
                    if lookup_builtin_intrinsic(&obj_ty, method).is_none() {
                        let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
                        let actual_name = resolve_method_name(&resolved_obj_ty, method, &[], funcs);
                        generate_expr(obj, sym, &resolved_obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        let param_types: Vec<String> = if let Some(f) = funcs.get(&actual_name) {
                            f.params.iter().skip(1).map(|p| p.ty.to_string()).collect()
                        } else {
                            vec!["unknown".to_string(); args.len()]
                        };
                        for (i, arg) in args.iter().enumerate() {
                            let expected = param_types.get(i).map(|s| s.as_str()).unwrap_or("unknown");
                            generate_expr(arg, sym, expected, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        }
                        let safe_name = sanitize_wat_name(&actual_name);
                        wat.push_str(&format!("    return_call ${}\n", safe_name));
                    } else {
                        generate_expr(expr, sym, expected_return_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                        wat.push_str("    return\n");
                    }
                } else {
                    generate_expr(expr, sym, expected_return_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                    wat.push_str("    return\n");
                }
            } else {
                wat.push_str("    return\n");
            }
        }
        Stmt::AssignPlus(name, expr) => {
            if sym.contains_key(name) {
                let ty = sym.get(name).unwrap();
                wat.push_str(&format!("    local.get ${}\n", name));
                generate_expr(expr, sym, ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                wat.push_str(&format!("    {}.add\n", ty));
                wat.push_str(&format!("    local.set ${}\n", name));
            } else if let Some(resolved) = resolve_const_name(name) {
                let const_ty = GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                });
                let safe_name = sanitize_wat_name(&resolved);
                wat.push_str(&format!("    global.get ${}\n", safe_name));
                generate_expr(expr, sym, &const_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                wat.push_str(&format!("    {}.add\n", const_ty));
                wat.push_str(&format!("    global.set ${}\n", safe_name));
            } else {
                panic!("AssignPlus to unknown identifier: {}", name);
            }
        }
        Stmt::ExprStmt(expr) => {
            let ty = get_expr_type(expr, sym, funcs, structs);
            generate_expr(expr, sym, &ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            if ty != "void" {
                wat.push_str("    drop\n");
            }
        }
        Stmt::If(cond, body, else_body) => {
            *loop_idx += 1;
            if else_body.is_some() {
                *loop_idx += 1;
            }
            let cond_ty = get_expr_type(cond, sym, funcs, structs);
            if cond_ty != "bool" {
                crate::diagnostics::report_error(format!("if condition must be a boolean, got type '{}'", cond_ty), None);
            }
            generate_expr(cond, sym, "bool", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str("    if\n");
            for bstmt in body {
                generate_stmt(bstmt, sym, expected_return_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth);
            }
            if let Some(e_body) = else_body {
                wat.push_str("    else\n");
                for bstmt in e_body {
                    generate_stmt(bstmt, sym, expected_return_ty, loop_idx, wat, funcs, structs, string_lit_ids, varr_depth);
                }
            }
            wat.push_str("    end\n");
        }
        Stmt::While(cond, body) => {
            let l_var = format!("_wloop{}", loop_idx);
            let b_var = format!("_wblock{}", loop_idx);
            *loop_idx += 1;

            wat.push_str(&format!("    block ${}\n", b_var));
            wat.push_str(&format!("    loop ${}\n", l_var));

            let cond_ty = get_expr_type(cond, sym, funcs, structs);
            if cond_ty != "bool" {
                crate::diagnostics::report_error("while condition must be a boolean".to_string(), None);
            }
            generate_expr(cond, sym, "bool", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str("    i32.eqz\n");
            wat.push_str(&format!("    br_if ${}\n", b_var));

            for bstmt in body {
                generate_stmt(bstmt,
                    sym,
                    expected_return_ty,
                    loop_idx,
                    wat,
                    funcs,
                    structs,
                    string_lit_ids,
                    varr_depth,
                );
            }

            wat.push_str(&format!("    br ${}\n", l_var));
            wat.push_str("    end\n");
            wat.push_str("    end\n");
        }
        Stmt::Assign(name, expr) => {
            if sym.contains_key(name) {
                let ty = sym.get(name).unwrap();
                generate_expr(expr, sym, ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                wat.push_str(&format!("    local.set ${}\n", name));
            } else if let Some(resolved) = resolve_const_name(name) {
                let const_ty = GLOBAL_CONSTS.with(|gc| {
                    gc.borrow().get(&resolved).cloned().unwrap_or_else(|| "i32".to_string())
                });
                generate_expr(expr, sym, &const_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
                let safe_name = sanitize_wat_name(&resolved);
                wat.push_str(&format!("    global.set ${}\n", safe_name));
            } else {
                panic!("Assigning to unknown identifier: {}", name);
            }
        }
        Stmt::AssignIndex(arr, idx, val) => {
            let ty = get_expr_type(arr, sym, funcs, structs);
            generate_expr(arr, sym, &ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            generate_expr(idx, sym, "i32", wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            let val_ty = if ty.starts_with("[]") { &ty[2..] } else { "unknown" };
            generate_expr(val, sym, val_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            if ty.starts_with("[]") {
                let inner = &ty[2..];
                let resolved_inner = resolve_struct_name(inner, structs);
                wat.push_str(&format!("    array.set $array_{}\n", sanitize_wat_name(&resolved_inner)));
            } else {
                panic!("Assigning index on non-array type: {}", ty);
            }
        }
        Stmt::AssignField(obj, field, val) => {
            let obj_ty = get_expr_type(obj, sym, funcs, structs);
            let resolved_obj_ty = resolve_struct_name(&obj_ty, structs);
            let field_ty = structs.get(&resolved_obj_ty)
                .and_then(|s| s.fields.iter().find(|f| f.name == *field))
                .map(|f| f.ty.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            generate_expr(obj, sym, &resolved_obj_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            generate_expr(val, sym, &field_ty, wat, funcs, structs, string_lit_ids, loop_idx, varr_depth);
            wat.push_str(&format!("    struct.set ${} ${}\n", sanitize_wat_name(&resolved_obj_ty), field));
        }
        Stmt::For(item_var, arr_var, body) => {
            let i_var = format!("_i{}", loop_idx);
            let len_var = format!("_len{}", loop_idx);
            *loop_idx += 1;

            let arr_ty = sym.get(arr_var).expect("Array variable not found in symbol table");
            let inner_ty = if arr_ty.starts_with("[]") { &arr_ty[2..] } else { "unknown" };

            wat.push_str(&format!("    local.get ${}\n", arr_var));
            wat.push_str("    array.len\n");
            wat.push_str(&format!("    local.set ${}\n", len_var));
            wat.push_str(&format!("    i32.const 0\n    local.set ${}\n", i_var));

            wat.push_str(&format!("    (block $exit_{}\n", i_var));
            wat.push_str(&format!("      (loop $loop_{}\n", i_var));

            wat.push_str(&format!("        local.get ${}\n", i_var));
            wat.push_str(&format!("        local.get ${}\n", len_var));
            wat.push_str(&format!(
                "        i32.ge_u\n        br_if $exit_{}\n",
                i_var
            ));

            wat.push_str(&format!("    local.get ${}\n", arr_var));
            wat.push_str(&format!("    local.get ${}\n", i_var));
            let resolved_inner = resolve_struct_name(inner_ty, structs);
            wat.push_str(&format!("        array.get $array_{}\n", sanitize_wat_name(&resolved_inner)));
            wat.push_str(&format!("        local.set ${}\n", item_var));

            for bstmt in body {
                generate_stmt(bstmt,
                    sym,
                    expected_return_ty,
                    loop_idx,
                    wat,
                    funcs,
                    structs,
                    string_lit_ids,
                    varr_depth,
                );
            }

            wat.push_str(&format!("        local.get ${}\n", i_var));
            wat.push_str("        i32.const 1\n        i32.add\n");
            wat.push_str(&format!("        local.set ${}\n", i_var));
            wat.push_str(&format!("        br $loop_{}\n", i_var));

            wat.push_str("      )\n    )\n");
        }
    }
}

pub fn collect_string_literals(funcs: &[Function]) -> Vec<String> {
    let mut literals = std::collections::HashSet::new();
    fn visit_expr(expr: &Expr, literals: &mut std::collections::HashSet<String>) {
        match expr {
            Expr::StringLit(s) => { literals.insert(s.clone()); }
            Expr::Binary(l, _, r) => { visit_expr(l, literals); visit_expr(r, literals); }
            Expr::MethodCall(obj, _, args) => { visit_expr(obj, literals); for a in args { visit_expr(a, literals); } }
            Expr::FieldAccess(obj, _) => visit_expr(obj, literals),
            Expr::IndexAccess(arr, idx) => { visit_expr(arr, literals); visit_expr(idx, literals); }
            Expr::StructInit(_, fields) => { for (_, e) in fields { visit_expr(e, literals); } }
            Expr::New(_, args) => { for a in args { visit_expr(a, literals); } }
            Expr::Call(_, args) => { for a in args { visit_expr(a, literals); } }
            Expr::If(cond, then_b, else_b) => {
                visit_expr(cond, literals);
                let (t_stmts, t_val) = &**then_b;
                for s in t_stmts { visit_stmt(s, literals); }
                if let Some(v) = t_val { visit_expr(v, literals); }
                if let Some(eb) = else_b {
                    let (e_stmts, e_val) = &**eb;
                    for s in e_stmts { visit_stmt(s, literals); }
                    if let Some(v) = e_val { visit_expr(v, literals); }
                }
            }
            Expr::Match(target, arms) => {
                visit_expr(target, literals);
                for arm in arms {
                    for s in &arm.body { visit_stmt(s, literals); }
                    if let Some(v) = &arm.val { visit_expr(v, literals); }
                }
            }
            Expr::Spread(e) => visit_expr(e, literals),
            Expr::Tuple(exprs) => { for e in exprs { visit_expr(e, literals); } }
            Expr::MapLit(pairs) => { for (k, v) in pairs { visit_expr(k, literals); visit_expr(v, literals); } }
            Expr::VecLit(elems) => { for e in elems { visit_expr(e, literals); } }
            _ => {}
        }
    }
    fn visit_stmt(stmt: &Stmt, literals: &mut std::collections::HashSet<String>) {
        match stmt {
            Stmt::Let(_, _, e) => visit_expr(e, literals),
            Stmt::LetTuple(_, e) => visit_expr(e, literals),
            Stmt::ExprStmt(e) => visit_expr(e, literals),
            Stmt::Return(Some(e)) => visit_expr(e, literals),
            Stmt::Return(None) => {}
            Stmt::Assign(_, e) => visit_expr(e, literals),
            Stmt::AssignPlus(_, e) => visit_expr(e, literals),
            Stmt::AssignIndex(a, i, v) => { visit_expr(a, literals); visit_expr(i, literals); visit_expr(v, literals); }
            Stmt::AssignField(o, _, v) => { visit_expr(o, literals); visit_expr(v, literals); }
            Stmt::If(c, t, e) => { visit_expr(c, literals); for s in t { visit_stmt(s, literals); } if let Some(eb) = e { for s in eb { visit_stmt(s, literals); } } }
            Stmt::While(c, b) => { visit_expr(c, literals); for s in b { visit_stmt(s, literals); } }
            Stmt::For(_, _, b) => { for s in b { visit_stmt(s, literals); } }
        }
    }
    for f in funcs {
        for s in &f.body { visit_stmt(s, &mut literals); }
    }
    let mut v: Vec<String> = literals.into_iter().collect();
    v.sort();
    v
}

pub fn generate_wat(
    funcs: &[Function],
    structs: &[StructDef],
    string_literals: &[String],
    consts: &[ConstDef],
    imports_registry: &HashMap<String, HashSet<String>>
) -> (String, Vec<StructDef>) {
    let mut consts_map = HashMap::new();
    for c in consts {
        consts_map.insert(c.name.clone(), c.clone());
    }
    let mut global_init_statements = String::new();
    let mut global_loop_idx = 0;
    let mut global_varr_depth = 0;

    GLOBAL_CONSTS.with(|gc| {
        let mut map = gc.borrow_mut();
        map.clear();
        for c in consts {
            map.insert(c.name.clone(), c.ty.to_string());
        }
    });
    GLOBAL_CONST_VALUES.with(|gcv| {
        gcv.borrow_mut().clear();
    });

    let mut funcs_map = HashMap::new();
    IMPORTS_REGISTRY.with(|r| {
        *r.borrow_mut() = imports_registry.clone();
    });

    let mut structs_map = HashMap::new();
    let mut string_lit_ids: HashMap<String, String> = HashMap::new();
    for (id, lit) in string_literals.iter().enumerate() {
        string_lit_ids.insert(lit.clone(), format!("s{}", id));
    }
    for s in structs {
        structs_map.insert(s.name.clone(), s.clone());
    }
    for f in funcs {
        funcs_map.insert(f.name.clone(), f.clone());
    }

    let mut array_types = std::collections::HashSet::new();
    let mut used_struct_names = std::collections::HashSet::new();
    let mut fn_types = std::collections::HashSet::new();

    fn extract_types(ty: &str, array_types: &mut std::collections::HashSet<String>, used_struct_names: &mut std::collections::HashSet<String>, structs_map: &HashMap<String, StructDef>, fn_types: &mut std::collections::HashSet<String>) {
        if ty.starts_with("[]") {
            let inner = &ty[2..];
            let resolved_inner = resolve_struct_name(inner, structs_map);
            array_types.insert(resolved_inner.clone());
            extract_types(&resolved_inner, array_types, used_struct_names, structs_map, fn_types);
            } else if ty.starts_with("fn(") {
                fn_types.insert(normalize_fn_ty(ty));
            // Extract inner types
            let inner = ty[3..].to_string();
            // Find the "):" separator at paren depth 0 to correctly split params and return type
            // Start at depth 1 because we skipped "fn(" and the inner string starts inside the params
            let mut depth = 1;
            let mut colon_pos = None;
            for (i, c) in inner.chars().enumerate() {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        // Found closing paren at depth 0, next should be ':' for return type
                        if inner.chars().nth(i + 1) == Some(':') {
                            colon_pos = Some(i);
                            break;
                        }
                    }
                }
            }
            if let Some(colon) = colon_pos {
                let params_str = &inner[..colon];
                let ret_ty = &inner[colon+2..];
                extract_types(ret_ty, array_types, used_struct_names, structs_map, fn_types);
                if !params_str.is_empty() {
                    for p in params_str.split(',') {
                        extract_types(p.trim(), array_types, used_struct_names, structs_map, fn_types);
                    }
                }
            }
        } else {
            let resolved = resolve_struct_name(ty, structs_map);
            if structs_map.contains_key(&resolved) {
                if used_struct_names.insert(resolved.clone()) {
                    for f in &structs_map[&resolved].fields {
                        extract_types(&f.ty.to_string(), array_types, used_struct_names, structs_map, fn_types);
                    }
                }
            }
        }
    }

    fn extract_closure_types_expr(
        expr: &Expr,
        funcs: &HashMap<String, Function>,
        array_types: &mut std::collections::HashSet<String>,
        used_struct_names: &mut std::collections::HashSet<String>,
        structs_map: &HashMap<String, StructDef>,
        fn_types: &mut std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::ClosureInstantiate(func_name, _, captured) => {
                if let Some(f) = funcs.get(func_name) {
                    let mut params_str = Vec::new();
                    for p in &f.params {
                        if p.name != "__env" {
                            params_str.push(p.ty.to_string());
                        }
                    }
                    let fn_ty = format!("fn({}):{}", params_str.join(","), f.return_ty);
                    extract_types(&fn_ty, array_types, used_struct_names, structs_map, fn_types);
                }
                for c in captured {
                    extract_closure_types_expr(c, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::Tuple(exprs) => {
                for e in exprs {
                    extract_closure_types_expr(e, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::Binary(l, _, r) => {
                extract_closure_types_expr(l, funcs, array_types, used_struct_names, structs_map, fn_types);
                extract_closure_types_expr(r, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Expr::Call(_, args) => {
                for a in args {
                    extract_closure_types_expr(a, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::MethodCall(obj, _, args) => {
                extract_closure_types_expr(obj, funcs, array_types, used_struct_names, structs_map, fn_types);
                for a in args {
                    extract_closure_types_expr(a, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::InvokeFuncPtr(func_expr, args) => {
                extract_closure_types_expr(func_expr, funcs, array_types, used_struct_names, structs_map, fn_types);
                for a in args {
                    extract_closure_types_expr(a, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::StructInit(_, fields) => {
                for (_, e) in fields {
                    extract_closure_types_expr(e, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::IndexAccess(arr, idx) => {
                extract_closure_types_expr(arr, funcs, array_types, used_struct_names, structs_map, fn_types);
                extract_closure_types_expr(idx, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Expr::FieldAccess(obj, _) => {
                extract_closure_types_expr(obj, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Expr::Cast(e, _) => {
                extract_closure_types_expr(e, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Expr::Match(cond, arms) => {
                extract_closure_types_expr(cond, funcs, array_types, used_struct_names, structs_map, fn_types);
                for arm in arms {
                    for s in &arm.body {
                        extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                    }
                    if let Some(v) = &arm.val {
                        extract_closure_types_expr(v, funcs, array_types, used_struct_names, structs_map, fn_types);
                    }
                }
            }
            Expr::If(cond, then_block, else_block) => {
                extract_closure_types_expr(cond, funcs, array_types, used_struct_names, structs_map, fn_types);
                for s in &then_block.0 {
                    extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
                if let Some(v) = &then_block.1 {
                    extract_closure_types_expr(v, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
                if let Some(eb) = else_block {
                    for s in &eb.0 {
                        extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                    }
                    if let Some(v) = &eb.1 {
                        extract_closure_types_expr(v, funcs, array_types, used_struct_names, structs_map, fn_types);
                    }
                }
            }
            Expr::New(_, args) => {
                for a in args {
                    extract_closure_types_expr(a, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::Spread(e) => {
                extract_closure_types_expr(e, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Expr::MapLit(pairs) => {
                for (k, v) in pairs {
                    extract_closure_types_expr(k, funcs, array_types, used_struct_names, structs_map, fn_types);
                    extract_closure_types_expr(v, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Expr::VecLit(elems) => {
                for e in elems {
                    extract_closure_types_expr(e, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            _ => {}
        }
    }

    fn extract_closure_types_stmt(
        stmt: &Stmt,
        funcs: &HashMap<String, Function>,
        array_types: &mut std::collections::HashSet<String>,
        used_struct_names: &mut std::collections::HashSet<String>,
        structs_map: &HashMap<String, StructDef>,
        fn_types: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Stmt::Let(_, _, expr) | Stmt::Assign(_, expr) | Stmt::AssignPlus(_, expr) | Stmt::ExprStmt(expr) => {
                extract_closure_types_expr(expr, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Stmt::LetTuple(_, expr) => {
                extract_closure_types_expr(expr, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Stmt::AssignIndex(arr, idx, expr) => {
                extract_closure_types_expr(arr, funcs, array_types, used_struct_names, structs_map, fn_types);
                extract_closure_types_expr(idx, funcs, array_types, used_struct_names, structs_map, fn_types);
                extract_closure_types_expr(expr, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Stmt::AssignField(obj, _, expr) => {
                extract_closure_types_expr(obj, funcs, array_types, used_struct_names, structs_map, fn_types);
                extract_closure_types_expr(expr, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            Stmt::If(cond, body, else_body) => {
                extract_closure_types_expr(cond, funcs, array_types, used_struct_names, structs_map, fn_types);
                for s in body {
                    extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                    }
                }
            }
            Stmt::While(cond, body) => {
                extract_closure_types_expr(cond, funcs, array_types, used_struct_names, structs_map, fn_types);
                for s in body {
                    extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Stmt::For(_, _, body) => {
                for s in body {
                    extract_closure_types_stmt(s, funcs, array_types, used_struct_names, structs_map, fn_types);
                }
            }
            Stmt::Return(Some(expr)) => {
                extract_closure_types_expr(expr, funcs, array_types, used_struct_names, structs_map, fn_types);
            }
            _ => {}
        }
    }

    if funcs.iter().any(|f| f.is_variadic()) {
        array_types.insert("byte".to_string());
    }

    for f in funcs {
        extract_types(&f.return_ty.to_string(), &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
        for p in &f.params {
            extract_types(&p.ty.to_string(), &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
            if p.is_variadic {
                extract_types(&format!("[]{}", p.ty), &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
            }
        }
        let mut func_types = Vec::new();
        let mut env = HashMap::new();
        for p in &f.params {
            env.insert(p.name.clone(), p.ty.to_string());
        }
        for s in &f.body {
            collect_types_from_stmt(s, &mut func_types, &mut env);
        }
        for ty in func_types {
            extract_types(&ty, &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
        }
        for s in &f.body {
            extract_closure_types_stmt(s, &funcs_map, &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
        }
    }
    
    for c in consts {
        extract_types(&c.ty.to_string(), &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
        let mut expr_types = Vec::new();
        collect_types_from_expr(&c.value, &mut expr_types, &HashMap::new());
        for ty in expr_types {
            extract_types(&ty, &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
        }
        extract_closure_types_expr(&c.value, &funcs_map, &mut array_types, &mut used_struct_names, &structs_map, &mut fn_types);
    }
    
    let filtered_structs: Vec<&StructDef> = structs.iter().filter(|s| used_struct_names.contains(&s.name)).collect();

    let mut wat = String::new();

    // Collect function and fat pointer type definitions separately so they
    // can be emitted BEFORE the (rec ...) block in final_wat.  Types inside
    // the rec block may reference $fat_* types (e.g. Option<fn()>, Vec<fn()>),
    // so $sig_*/$fat_* must have lower type indices than the rec group.
    let mut fn_type_wat = String::new();
    for fn_ty in &fn_types {
        let fat_name = fn_type_to_wasm_name(fn_ty);
        let inner = fn_ty[3..].to_string();
        // Find the "):" separator at paren depth 0 to correctly split params and return type
        // Start at depth 1 because we skipped "fn(" and the inner string starts inside the params
        let mut depth = 1;
        let mut colon_pos = None;
        for (i, c) in inner.chars().enumerate() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    // Found closing paren at depth 0, next should be ':' for return type
                    if inner.chars().nth(i + 1) == Some(':') {
                        colon_pos = Some(i);
                        break;
                    }
                }
            }
        }
        if let Some(colon) = colon_pos {
            let params_str = &inner[..colon];
            let ret_ty = &inner[colon+2..];
            
            let mut param_wasm = Vec::new();
            if !params_str.is_empty() {
                for p in params_str.split(',') {
                    param_wasm.push(map_wasm_ty(p.trim(), &structs_map));
                }
            }
            let ret_wasm = map_wasm_ty(ret_ty, &structs_map);
            
            let sig_name = format!("sig_{}", fat_name);
            fn_type_wat.push_str(&format!("    (type ${} (func", sig_name));
            for p in &param_wasm {
                fn_type_wat.push_str(&format!(" (param {})", p));
            }
            fn_type_wat.push_str(" (param (ref null any))"); // __env
            if ret_wasm != "void" {
                fn_type_wat.push_str(&format!(" (result {})", ret_wasm));
            }
            fn_type_wat.push_str("))\n");
            
            fn_type_wat.push_str(&format!("    (type ${} (struct (field $func_ref (mut (ref null ${}))) (field $env (mut (ref null any)))))\n", fat_name, sig_name));
        }
    }

    // Build a map from fn_ty string → $sig type name so that function
    // definitions (which create $ty_* types outside the rec block) can
    // reuse the $sig type instead.  This ensures ref.func and struct.new
    // reference the SAME type index, which the Wasm validator requires
    // since types in different rec groups are nominally distinct.
    let fn_ty_to_sig_name: HashMap<String, String> = fn_types.iter().map(|fn_ty| {
        let fat_name = fn_type_to_wasm_name(fn_ty);
        let sig_name = format!("sig_{}", fat_name);
        (fn_ty.clone(), sig_name)
    }).collect();



    if funcs.iter().any(|f| f.is_variadic()) {
        wat.push_str(&emit_fmt_runtime_helper_funcs());
    }

    let mut wat_imports = String::new();
    // Auto-inject imports
    for func in funcs {
        CURRENT_NAMESPACE.with(|c| {
            *c.borrow_mut() = get_namespace(&func.name);
        });
        if func.is_extern {
            let safe_name = sanitize_wat_name(&func.name);
            let import_name = shorten_import_name(&func.name);
            wat_imports.push_str(&format!(
                "  (import \"env\" \"{}\" (func ${}",
                import_name, safe_name
            ));
            for param in &func.params {
                let ty = if param.is_variadic {
                    map_wasm_ty(&format!("[]{}", param.ty), &structs_map)
                } else {
                    map_wasm_ty(&param.ty.to_string(), &structs_map)
                };
                wat_imports.push_str(&format!(" (param ${} {})", param.name, ty));
            }
            let func_return_ty_str = func.return_ty.to_string();
            if func_return_ty_str != "void" && !func_return_ty_str.is_empty() {
                wat_imports.push_str(&format!(" (result {})", map_wasm_ty(&func_return_ty_str, &structs_map)));
            }
            wat_imports.push_str("))\n");
        }
    }

    for func in funcs {
        CURRENT_NAMESPACE.with(|c| {
            *c.borrow_mut() = get_namespace(&func.name);
        });
        if func.is_extern {
            continue;
        }

        let safe_name = sanitize_wat_name(&func.name);

        // Determine the Wasm types for params and return
        let mut param_wasm_tys: Vec<String> = Vec::new();
        let mut has_ref_type = false;
        for param in &func.params {
            let ty = if param.is_variadic {
                map_wasm_ty(&format!("[]{}", param.ty), &structs_map)
            } else {
                map_wasm_ty(&param.ty.to_string(), &structs_map)
            };
            if ty.starts_with("(ref") { has_ref_type = true; }
            param_wasm_tys.push(ty);
        }
        let func_return_ty_str = func.return_ty.to_string();
        let return_wasm_ty = if func_return_ty_str != "void" && !func_return_ty_str.is_empty() {
            let rty = map_wasm_ty(&func_return_ty_str, &structs_map);
            if rty.starts_with("(ref") { has_ref_type = true; }
            Some(rty)
        } else {
            None
        };

        // Emit an explicit type declaration to avoid wat crate bugs with inline
        // function type signatures referencing rec-block types.
        if has_ref_type {
            // Reconstruct the fn_ty string to check whether this function's
            // signature matches an existing $sig type in the rec block.
            let fn_params: Vec<String> = func.params.iter()
                .filter(|p| p.name != "__env")
                .map(|p| p.ty.to_string())
                .collect();
            let func_fn_ty = normalize_fn_ty(&format!("fn({}):{}", fn_params.join(","), func.return_ty));
            let is_closure = func.params.iter().any(|p| p.name == "__env");
            if is_closure && fn_ty_to_sig_name.contains_key(&func_fn_ty) {
                let sig_name = fn_ty_to_sig_name.get(&func_fn_ty).unwrap();
                wat.push_str(&format!("  (func ${} (type ${})\n", safe_name, sig_name));
            } else {
                let type_name = format!("$ty_{}", safe_name);
                wat.push_str(&format!("  (type {} (func", type_name));
                for (i, _param) in func.params.iter().enumerate() {
                    wat.push_str(&format!(" (param {})", param_wasm_tys[i]));
                }
                if let Some(ref rty) = return_wasm_ty {
                    wat.push_str(&format!(" (result {})", rty));
                }
                wat.push_str("))\n");
                // Use (type) reference with no inline params to avoid wat crate
                // creating a duplicate function type from inline param annotations.
                wat.push_str(&format!("  (func ${} (type {})\n", safe_name, type_name));
            }
        } else {
            wat.push_str(&format!("  (func ${}", safe_name));
            for (i, param) in func.params.iter().enumerate() {
                wat.push_str(&format!(
                    " (param ${} {})",
                    param.name,
                    param_wasm_tys[i]
                ));
            }
            if let Some(ref rty) = return_wasm_ty {
                wat.push_str(&format!(" (result {})\n", rty));
            } else {
                wat.push_str("\n");
            }
        }

        let mut sym = HashMap::new();
        for param in &func.params {
            let sym_ty = if param.is_variadic {
                format!("[]{}", param.ty)
            } else {
                param.ty.to_string()
            };
            sym.insert(param.name.clone(), sym_ty);
        }

        let mut locals = Vec::new();
        let mut loop_idx = 0;

        for stmt in &func.body {
            scan_locals(
                stmt,
                &mut sym,
                &mut locals,
                &mut loop_idx,
                &funcs_map,
                &structs_map,
            );
        }

        let mut func_uses_variadic = false;
        for stmt in &func.body {
            if has_variadic_call_stmt(stmt, &sym, &funcs_map, &structs_map) {
                func_uses_variadic = true;
                break;
            }
        }
        if func_uses_variadic {
            for i in 0..8 {
                wat.push_str(&format!("    (local $_varr_{} (ref null $array_anyref))\n", i));
            }
        }

        for (name, ty) in locals {
            wat.push_str(&format!(
                "    (local ${} {})\n",
                name,
                map_wasm_ty(&ty, &structs_map)
            ));
        }

        // When has_ref_type, params are not inline. We need to declare param
        // locals and copy from indices to named locals.
        if has_ref_type {
            for (i, param) in func.params.iter().enumerate() {
                wat.push_str(&format!("    (local ${} {})\n", param.name, param_wasm_tys[i]));
            }
        }

        loop_idx = 0;
        let mut varr_depth: i32 = 0;

        if func.is_compiler && (func.name.ends_with("::sprintf") || func.name == "sprintf") {
            // Special case: std::fmt::sprintf. Emit a hand-written Wasm
            // implementation. For has_ref_type functions, split the body into
            // locals and instructions, emit param copies in between.
            if has_ref_type {
                let body = emit_sprintf_body();
                let mut local_part = String::new();
                let mut rest = String::new();
                let mut found_non_local = false;
                for line in body.lines() {
                    let trimmed = line.trim();
                    if !found_non_local && (trimmed.starts_with("(local ") || trimmed.is_empty()) {
                        local_part.push_str(line);
                        local_part.push('\n');
                    } else {
                        found_non_local = true;
                        rest.push_str(line);
                        rest.push('\n');
                    }
                }
                wat.push_str(&local_part);
                for (i, param) in func.params.iter().enumerate() {
                    wat.push_str(&format!(
                        "    local.get {}\n    local.set ${}\n",
                        i, param.name
                    ));
                }
                wat.push_str(&rest);
            } else {
                wat.push_str(&emit_sprintf_body());
            }
        } else {
            // When has_ref_type, emit param-to-local copies before any body instructions.
            // Params are unnamed in the WAT type, so we declared (local $name type) and
            // must copy from the positional index to the named local.
            if has_ref_type {
                for (i, param) in func.params.iter().enumerate() {
                    wat.push_str(&format!(
                        "    local.get {}\n    local.set ${}\n",
                        i, param.name
                    ));
                }
            }
            let expected_ret_str = func.return_ty.to_string();
            for stmt in &func.body {
                generate_stmt(
                    stmt,
                    &sym,
                    &expected_ret_str,
                    &mut loop_idx,
                    &mut wat,
                    &funcs_map,
                    &structs_map,
                    &string_lit_ids,
                    &mut varr_depth,
                );
            }
        }

        // Safety: WebAssembly blocks require exactly one value on the stack if a return value is expected.
        // If the user's code relies entirely on `return` statements, reaching the end without pushing a value
        // will fail WAT compilation. For primitives, we can push a dummy value just in case.
        let expected_ret_str = func.return_ty.to_string();
        if expected_ret_str == "i32" {
            wat.push_str("    i32.const 0\n");
        }
        if expected_ret_str == "i64" {
            wat.push_str("    i64.const 0\n");
        }
        if expected_ret_str == "f32" {
            wat.push_str("    f32.const 0\n");
        }
        if structs_map.contains_key(&expected_ret_str) {
            wat.push_str("    unreachable\n");
        }

        wat.push_str("  )\n");
        if (func.is_pub && get_namespace(&func.name) == "") || func.name == "task::fox_run_task" || func.name == "main" || func.is_compiler {
            let export_name = if func.name == "task::fox_run_task" {
                "fox_run_task"
            } else {
                &safe_name
            };
            wat.push_str(&format!(
                "  (export \"{}\" (func ${}))\n",
                export_name, safe_name
            ));
        }
    }

    for s in &filtered_structs {
        let safe_s_name = sanitize_wat_name(&s.name);
        wat.push_str(&format!("  (func $fox_alloc_{} ", safe_s_name));
        for f in &s.fields {
            wat.push_str(&format!(
                "(param ${} {}) ",
                f.name,
                map_wasm_ty(&f.ty.to_string(), &structs_map)
            ));
        }
        wat.push_str(&format!("(result (ref ${}))\n", safe_s_name));
        for f in &s.fields {
            wat.push_str(&format!("    local.get ${}\n", f.name));
        }
        wat.push_str(&format!("    struct.new ${}\n  )\n", safe_s_name));
        wat.push_str(&format!(
            "  (export \"fox_alloc_{}\" (func $fox_alloc_{}))\n",
            safe_s_name, safe_s_name
        ));
    }

    let mut final_wat = String::new();
    final_wat.push_str("(module\n");
    for (id, _lit) in string_literals.iter().enumerate() {
        final_wat.push_str(&format!("  (import \"env\" \"s{}\" (global $s{} externref))\n", id, id));
    }
    // Put all types (fn pointer + struct + array) into a single rec block so they
    // can reference each other freely (e.g. a fn type may return Option<fn()>, and
    // Option<fn()> may reference a fat fn pointer type).
    let has_fn_types = !fn_type_wat.is_empty();
    let has_struct_types = !filtered_structs.is_empty() || !array_types.is_empty();
    if has_fn_types || has_struct_types {
        final_wat.push_str("  (rec\n");
        final_wat.push_str(&fn_type_wat);
        for s in &filtered_structs {
            final_wat.push_str(&format!("    (type ${} (struct ", sanitize_wat_name(&s.name)));
            for f in &s.fields {
                final_wat.push_str(&format!(
                    "(field ${} (mut {})) ",
                    f.name,
                    map_wasm_ty(&f.ty.to_string(), &structs_map)
                ));
            }
            final_wat.push_str("))\n");
        }
        for inner in &array_types {
            let wasm_inner = map_wasm_ty(inner, &structs_map);
            final_wat.push_str(&format!("    (type $array_{} (array (mut {})))\n", sanitize_wat_name(inner), wasm_inner));
        }
        final_wat.push_str("  )\n");
    }

    // Inject Wasm imports for any builtin intrinsic that was referenced.
    // This iterates the same lookup table that `generate_expr` uses, so a
    // newly declared builtin method only needs to be added to
    // `lookup_builtin_intrinsic` to be fully wired up.
    let builtin_methods = [
        ("str", "len"), ("str", "char_at"),
        ("str", "starts_with"), ("str", "ends_with"), ("str", "contains"),
        ("str", "index_of"), ("str", "last_index_of"),
        ("str", "is_empty"), ("str", "eq"), ("str", "join"), ("str", "compare"), ("str", "substring"),
    ];
    for (parent, method) in builtin_methods.iter() {
        if let Some(intr) = lookup_builtin_intrinsic(parent, method) {
            if intr.uses_wasm_helper {
                continue;
            }
            let wasm_fn = match intr.wasm_fn {
                Some(f) => f,
                None => continue,
            };
            let call_marker = format!("call {}", wasm_fn);
            if !wat.contains(&call_marker) {
                continue;
            }
            let mut sig = String::from(" (param externref)"); // self
            for ty in intr.param_wasm_tys {
                sig.push_str(&format!(" (param {})", ty));
            }
            if intr.result_wasm != "void" && !intr.result_wasm.is_empty() {
                sig.push_str(&format!(" (result {})", intr.result_wasm));
            }
            let import_name = shorten_import_name(intr.import_name.unwrap());
            final_wat.push_str(&format!(
                "  (import \"{}\" \"{}\" (func {}{}))\n",
                intr.module.unwrap(), import_name, wasm_fn, sig
            ));
        }
    }
    // The `bytes` intrinsic needs the Wasm GC builtins it composes out of.
    if wat.contains("call $fox_str_bytes") {
        if !wat.contains("call $fox_str_len") {
            final_wat.push_str("  (import \"wasm:js-string\" \"length\" (func $fox_str_len (param externref) (result i32)))\n");
        }
        if !wat.contains("call $fox_str_char_at") {
            final_wat.push_str("  (import \"wasm:js-string\" \"charCodeAt\" (func $fox_str_char_at (param externref i32) (result i32)))\n");
        }
    }
    // Legacy: the `==` and `+` operators on `str` and the hand-written
    // `sprintf` body use the older `$fox_js_string_*` names. Inject the
    // matching `wasm:js-string` imports when they are referenced.
    if wat.contains("call $fox_js_string_equals") {
        final_wat.push_str("  (import \"wasm:js-string\" \"equals\" (func $fox_js_string_equals (param externref externref) (result i32)))\n");
    }
    if wat.contains("call $fox_js_string_concat") {
        final_wat.push_str("  (import \"wasm:js-string\" \"concat\" (func $fox_js_string_concat (param externref externref) (result (ref extern))))\n");
    }
    if wat.contains("call $fox_js_string_length") {
        final_wat.push_str("  (import \"wasm:js-string\" \"length\" (func $fox_js_string_length (param externref) (result i32)))\n");
    }
    if wat.contains("call $fox_js_string_char_code_at") {
        final_wat.push_str("  (import \"wasm:js-string\" \"charCodeAt\" (func $fox_js_string_char_code_at (param externref i32) (result i32)))\n");
    }
    if wat.contains("$fox_fromCharCode") {
        final_wat.push_str("  (import \"wasm:js-string\" \"fromCharCode\" (func $fox_fromCharCode (param i32) (result (ref extern))))\n");
    }
    if wat.contains("call $__fox_f64_to_str") && !wat_imports.contains("func $__fox_f64_to_str") {
        let import_name = shorten_import_name("__fox_f64_to_str");
        final_wat.push_str(&format!(
            "  (import \"env\" \"{}\" (func $__fox_f64_to_str (param f64) (result externref)))\n",
            import_name
        ));
    }
    final_wat.push_str(&wat_imports);

    for c in consts {
        let ns = get_namespace(&c.name);
        CURRENT_NAMESPACE.with(|cn| {
            *cn.borrow_mut() = ns;
        });
        let safe_name = sanitize_wat_name(&c.name);
        if (!c.is_pub || get_namespace(&c.name) != "")
            && !wat.contains(&format!("global.get ${}", safe_name))
            && !wat.contains(&format!("global.set ${}", safe_name)) {
            continue;
        }
        let wasm_ty = map_wasm_ty(&c.ty.to_string(), &structs_map);
        let global_ty = if c.is_mutable {
            format!("(mut {})", wasm_ty)
        } else {
            wasm_ty.clone()
        };

        if c.is_mutable && !is_const_expr(&c.value, &consts_map) {
            let default_init = get_wasm_default_const(&wasm_ty);
            final_wat.push_str(&format!("  (global ${} {} ({}))\n", safe_name, global_ty, default_init));
            
            let mut init_code = String::new();
            generate_expr(&c.value, &HashMap::new(), &c.ty.to_string(), &mut init_code, &funcs_map, &structs_map, &string_lit_ids, &mut global_loop_idx, &mut global_varr_depth);
            global_init_statements.push_str(&init_code);
            global_init_statements.push_str(&format!("    global.set ${}\n", safe_name));
        } else {
            let init_wat = eval_const_expr(&c.value, &wasm_ty);
            let val = eval_const_val(&c.value);
            GLOBAL_CONST_VALUES.with(|gcv| {
                gcv.borrow_mut().insert(c.name.clone(), val);
            });
            final_wat.push_str(&format!("  (global ${} {} ({}))\n", safe_name, global_ty, init_wat));
        }

        if c.is_pub && get_namespace(&c.name) == "" {
            final_wat.push_str(&format!("  (export \"{}\" (global ${}))\n", safe_name, safe_name));
        }
    }
    CURRENT_NAMESPACE.with(|cn| {
        *cn.borrow_mut() = "".to_string();
    });
    if wat.contains("call $fox_str_bytes") {
        final_wat.push_str("  (func $fox_str_bytes (param $s externref) (result (ref null $array_byte))\n");
        final_wat.push_str("    (local $len i32)\n");
        final_wat.push_str("    (local $arr (ref null $array_byte))\n");
        final_wat.push_str("    (local $i i32)\n");
        final_wat.push_str("    local.get $s\n");
        final_wat.push_str("    call $fox_str_len\n");
        final_wat.push_str("    local.set $len\n");
        final_wat.push_str("    local.get $len\n");
        final_wat.push_str("    array.new_default $array_byte\n");
        final_wat.push_str("    local.set $arr\n");
        final_wat.push_str("    i32.const 0\n");
        final_wat.push_str("    local.set $i\n");
        final_wat.push_str("    (block $exit\n");
        final_wat.push_str("      (loop $loop\n");
        final_wat.push_str("        local.get $i\n");
        final_wat.push_str("        local.get $len\n");
        final_wat.push_str("        i32.ge_u\n");
        final_wat.push_str("        br_if $exit\n");
        final_wat.push_str("        local.get $arr\n");
        final_wat.push_str("        local.get $i\n");
        final_wat.push_str("        local.get $s\n");
        final_wat.push_str("        local.get $i\n");
        final_wat.push_str("        call $fox_str_char_at\n");
        final_wat.push_str("        array.set $array_byte\n");
        final_wat.push_str("        local.get $i\n");
        final_wat.push_str("        i32.const 1\n");
        final_wat.push_str("        i32.add\n");
        final_wat.push_str("        local.set $i\n");
        final_wat.push_str("        br $loop\n");
        final_wat.push_str("      )\n");
        final_wat.push_str("    )\n");
        final_wat.push_str("    local.get $arr\n");
        final_wat.push_str("  )\n");
    }
    
    let mut elem_declares = Vec::new();
    for f in funcs {
        if f.name.contains("__closure_") {
            elem_declares.push(format!("${}", sanitize_wat_name(&f.name)));
        }
    }

    if !elem_declares.is_empty() {
        final_wat.push_str(&format!("  (elem declare func {})\n", elem_declares.join(" ")));
    }
    
    final_wat.push_str(&wat);
    for inner in &array_types {
        let wasm_inner = map_wasm_ty(inner, &structs_map);
        let safe_inner = sanitize_wat_name(inner);
        final_wat.push_str(&format!(
            "  (func $fox_alloc_array_{} (param $len i32) (result (ref $array_{}))\n",
            safe_inner, safe_inner
        ));
        final_wat.push_str(&format!(
            "    local.get $len\n    array.new_default $array_{}\n  )\n",
            safe_inner
        ));
        final_wat.push_str(&format!(
            "  (export \"fox_alloc_array_{}\" (func $fox_alloc_array_{}))\n",
            safe_inner, safe_inner
        ));

        final_wat.push_str(&format!(
            "  (func $fox_set_array_{} (param $arr (ref $array_{})) (param $idx i32) (param $val {})\n",
            safe_inner, safe_inner, wasm_inner
        ));
        final_wat.push_str(&format!(
            "    local.get $arr\n    local.get $idx\n    local.get $val\n    array.set $array_{}\n  )\n",
            safe_inner
        ));
        final_wat.push_str(&format!(
            "  (export \"fox_set_array_{}\" (func $fox_set_array_{}))\n",
            safe_inner, safe_inner
        ));
    }
    if !global_init_statements.is_empty() {
        final_wat.push_str("  (func $__fox_global_init\n");
        final_wat.push_str(&global_init_statements);
        final_wat.push_str("  )\n");
        final_wat.push_str("  (start $__fox_global_init)\n");
    }
    final_wat.push_str(")\n");
    (final_wat, filtered_structs.into_iter().cloned().collect())
}

pub fn collect_types_from_expr(expr: &Expr, types: &mut Vec<String>, env: &HashMap<String, String>) {
    match expr {
        Expr::New(ty, args) => {
            types.push(ty.to_string());
            for arg in args {
                collect_types_from_expr(arg, types, env);
            }
        }
        Expr::StructInit(name, fields) => {
            types.push(name.clone());
            for (_, e) in fields {
                collect_types_from_expr(e, types, env);
            }
        }
        Expr::Call(name, args) => {
            if let Some(start) = name.find('<') {
                if let Some(end) = name.rfind('>') {
                    let mut base_name = name[..start].to_string();
                    if end + 1 < name.len() {
                        base_name.push_str(&name[end + 1..]);
                    }
                    let args_str = &name[start..end + 1];
                    types.push(format!("{}{}", base_name, args_str));
                }
            } else {
                types.push(name.clone());
            }
            if name.contains("::") {
                let mut last_colon_idx = None;
                let mut depth = 0;
                let chars: Vec<char> = name.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '<' {
                        depth += 1;
                    } else if chars[i] == '>' {
                        depth -= 1;
                    } else if chars[i] == ':' && i + 1 < chars.len() && chars[i+1] == ':' && depth == 0 {
                        last_colon_idx = Some(i);
                        i += 1;
                    }
                    i += 1;
                }
                if let Some(idx) = last_colon_idx {
                    types.push(name[..idx].to_string());
                }
            }
            for arg in args {
                collect_types_from_expr(arg, types, env);
            }
        }
        Expr::MethodCall(obj, method, args) => {
            collect_types_from_expr(obj, types, env);
            for arg in args {
                collect_types_from_expr(arg, types, env);
            }
            if method == "bytes" {
                types.push("[]byte".to_string());
            }
        }
        Expr::FieldAccess(obj, _) => {
            collect_types_from_expr(obj, types, env);
        }
        Expr::IndexAccess(arr, idx) => {
            collect_types_from_expr(arr, types, env);
            collect_types_from_expr(idx, types, env);
        }
        Expr::Binary(l, _, r) => {
            collect_types_from_expr(l, types, env);
            collect_types_from_expr(r, types, env);
        }
        Expr::If(cond, then_b, else_b) => {
            collect_types_from_expr(cond, types, env);
            let (t_stmts, t_val) = &**then_b;
            let mut then_env = env.clone();
            for s in t_stmts { collect_types_from_stmt(s, types, &mut then_env); }
            if let Some(v) = t_val { collect_types_from_expr(v, types, env); }
            if let Some(eb) = else_b {
                let (e_stmts, e_val) = &**eb;
                let mut else_env = env.clone();
                for s in e_stmts { collect_types_from_stmt(s, types, &mut else_env); }
                if let Some(v) = e_val { collect_types_from_expr(v, types, env); }
            }
        }
        Expr::Match(target, arms) => {
            collect_types_from_expr(target, types, env);
            let target_ty = infer_expr_type(target, env);
            for arm in arms {
                let mut arm_env = env.clone();
                match &arm.pattern {
                    MatchPattern::Some(name) => {
                        let ty = if target_ty.starts_with("Option<") && target_ty.ends_with('>') {
                            target_ty["Option<".len()..target_ty.len() - 1].to_string()
                        } else {
                            "unknown".to_string()
                        };
                        arm_env.insert(name.clone(), ty);
                    }
                    MatchPattern::Ok(name) => {
                        let ty = if target_ty.starts_with("Result<") && target_ty.ends_with('>') {
                            let inner = &target_ty["Result<".len()..target_ty.len() - 1];
                            let parts = split_types(inner);
                            if !parts.is_empty() {
                                parts[0].clone()
                            } else {
                                "unknown".to_string()
                            }
                        } else {
                            "unknown".to_string()
                        };
                        arm_env.insert(name.clone(), ty);
                    }
                    MatchPattern::Err(name) => {
                        let ty = if target_ty.starts_with("Result<") && target_ty.ends_with('>') {
                            let inner = &target_ty["Result<".len()..target_ty.len() - 1];
                            let parts = split_types(inner);
                            if parts.len() > 1 {
                                parts[1].clone()
                            } else {
                                "unknown".to_string()
                            }
                        } else {
                            "unknown".to_string()
                        };
                        arm_env.insert(name.clone(), ty);
                    }
                    MatchPattern::Variant(_, bindings) => {
                        for binding in bindings {
                            arm_env.insert(binding.clone(), "unknown".to_string());
                        }
                    }
                    _ => {}
                }
                for s in &arm.body { collect_types_from_stmt(s, types, &mut arm_env); }
                if let Some(v) = &arm.val { collect_types_from_expr(v, types, env); }
            }
        }
        Expr::ClosureInstantiate(_, env_name, captured) => {
            types.push(env_name.clone());
            for c in captured {
                collect_types_from_expr(c, types, env);
            }
        }
        Expr::Cast(e, t) => {
            collect_types_from_expr(e, types, env);
            types.push(t.to_string());
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            collect_types_from_expr(func_expr, types, env);
            for arg in args {
                collect_types_from_expr(arg, types, env);
            }
        }
        Expr::Spread(e) => {
            collect_types_from_expr(e, types, env);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                collect_types_from_expr(e, types, env);
            }
        }
        Expr::MapLit(pairs) => {
            let map_ty = infer_expr_type(expr, env);
            types.push(map_ty);
            for (k, v) in pairs {
                collect_types_from_expr(k, types, env);
                collect_types_from_expr(v, types, env);
            }
        }
        Expr::VecLit(elems) => {
            let vec_ty = infer_expr_type(expr, env);
            types.push(vec_ty);
            for e in elems {
                collect_types_from_expr(e, types, env);
            }
        }
        _ => {}
    }
}

pub fn collect_types_from_stmt(stmt: &Stmt, types: &mut Vec<String>, env: &mut HashMap<String, String>) {
    match stmt {
        Stmt::Let(name, ty_annot, expr) => {
            let ty = if let Some(ty) = ty_annot {
                types.push(ty.to_string());
                ty.to_string()
            } else {
                infer_expr_type(expr, env)
            };
            env.insert(name.clone(), ty);
            collect_types_from_expr(expr, types, env);
        }
        Stmt::LetTuple(bindings, expr) => {
            let expr_ty = infer_expr_type(expr, env);
            let sub_tys = if expr_ty.starts_with('(') && expr_ty.ends_with(')') {
                split_types(&expr_ty[1..expr_ty.len() - 1])
            } else {
                Vec::new()
            };
            let mut resolved_bindings_tys = Vec::new();
            for (i, (name, ty)) in bindings.iter().enumerate() {
                let inferred_ty = if ty.to_string().is_empty() {
                    if i < sub_tys.len() {
                        sub_tys[i].clone()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    ty.to_string()
                };
                types.push(inferred_ty.clone());
                env.insert(name.clone(), inferred_ty.clone());
                resolved_bindings_tys.push(inferred_ty);
            }
            types.push(format!("({})", resolved_bindings_tys.join(",")));
            collect_types_from_expr(expr, types, env);
        }
        Stmt::ExprStmt(expr) => collect_types_from_expr(expr, types, env),
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_types_from_expr(expr, types, env);
            }
        }
        Stmt::Assign(_, expr) => collect_types_from_expr(expr, types, env),
        Stmt::AssignPlus(_, expr) => collect_types_from_expr(expr, types, env),
        Stmt::AssignIndex(arr, idx, val) => {
            collect_types_from_expr(arr, types, env);
            collect_types_from_expr(idx, types, env);
            collect_types_from_expr(val, types, env);
        }
        Stmt::AssignField(obj, _, val) => {
            collect_types_from_expr(obj, types, env);
            collect_types_from_expr(val, types, env);
        }
        Stmt::If(cond, body, else_body) => {
            collect_types_from_expr(cond, types, env);
            let mut then_env = env.clone();
            for s in body {
                collect_types_from_stmt(s, types, &mut then_env);
            }
            if let Some(eb) = else_body {
                let mut else_env = env.clone();
                for s in eb {
                    collect_types_from_stmt(s, types, &mut else_env);
                }
            }
        }
        Stmt::While(cond, body) => {
            collect_types_from_expr(cond, types, env);
            let mut body_env = env.clone();
            for s in body {
                collect_types_from_stmt(s, types, &mut body_env);
            }
        }
        Stmt::For(loop_var, iter_target, body) => {
            let mut body_env = env.clone();
            let loop_var_ty = if let Some(target_ty) = env.get(iter_target) {
                if target_ty.starts_with("[]") {
                    target_ty[2..].to_string()
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };
            body_env.insert(loop_var.clone(), loop_var_ty);
            for s in body {
                collect_types_from_stmt(s, types, &mut body_env);
            }
        }
    }
}

pub fn dead_code_eliminate(
    funcs: Vec<Function>,
    structs: &[StructDef],
    consts: &[ConstDef],
) -> Vec<Function> {
    GLOBAL_CONSTS.with(|gc| {
        let mut map = gc.borrow_mut();
        map.clear();
        for c in consts {
            map.insert(c.name.clone(), c.ty.to_string());
        }
    });

    let func_map: HashMap<String, Function> =
        funcs.iter().map(|f| (f.name.clone(), f.clone())).collect();
    let structs_map: HashMap<String, StructDef> =
        structs.iter().map(|s| (s.name.clone(), s.clone())).collect();
    
    
    let graph = build_call_graph(&func_map, &structs_map);

    let mut reachable: HashSet<String> = HashSet::new();
    let mut work: Vec<String> = funcs
        .iter()
        .filter(|f| (f.is_pub && get_namespace(&f.name) == "") || f.is_extern || f.name == "task::fox_run_task" || f.name == "main")
        .map(|f| f.name.clone())
        .collect();

    let mut global_callees = HashSet::new();
    let mut tmp_sym = HashMap::new();
    for c in consts {
        collect_callees_expr(&c.value, &mut tmp_sym, &func_map, &structs_map, &mut global_callees);
    }
    work.extend(global_callees);

    // Keep Vec constructors alive since they are needed for default values of omitted fields
    for s in structs {
        let name = &s.name;
        if name.starts_with("vec::Vec") || name.contains("vec::Vec") || name.contains("std_collections_vec_Vec") {
            let func_name = format!("{}::new", name);
            if func_map.contains_key(&func_name) {
                work.push(func_name);
            }
        }
    }

    while let Some(name) = work.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(callees) = graph.get(&name) {
            for c in callees {
                if !reachable.contains(c) {
                    work.push(c.clone());
                }
            }
        }
    }

    funcs.into_iter().filter(|f| reachable.contains(&f.name)).collect()
}

pub fn extract_tuple_types(ty: &str, out: &mut HashSet<String>) {
    if ty.contains("unknown") {
        return;
    }
    if ty.starts_with("[]") {
        extract_tuple_types(&ty[2..], out);
    } else if ty.starts_with("fn(") {
        if let Some(start) = ty.find('(') {
            if let Some(end) = ty.rfind("):") {
                let params = &ty[start+1..end];
                let ret = &ty[end+2..];
                extract_tuple_types(ret, out);
                for p in split_types(params) {
                    extract_tuple_types(&p, out);
                }
            }
        }
    } else if ty.starts_with('(') && ty.ends_with(')') {
        let inner = &ty[1..ty.len()-1];
        let parts = split_types(inner);
        if parts.iter().all(|p| !p.is_empty()) {
            out.insert(ty.to_string());
            for p in parts {
                extract_tuple_types(&p, out);
            }
        }
    } else if let Some(start) = ty.find('<') {
        if ty.ends_with('>') {
            let inner = &ty[start+1..ty.len()-1];
            for p in split_types(inner) {
                extract_tuple_types(&p, out);
            }
        }
    }
}


pub fn make_tuple_struct_def(ty: &str) -> StructDef {
    let inner = &ty[1..ty.len()-1];
    let parts = split_types(inner);
    let mut fields = Vec::new();
    for (idx, part_ty) in parts.iter().enumerate() {
        fields.push(Field {
            name: format!("f{}", idx),
            ty: part_ty.parse().unwrap(),
            attributes: Vec::new(),
        });
    }

    StructDef {
        is_pub: true,
        name: ty.to_string(),
        generic: GenericParams::default(),
        fields,
        methods: Vec::new(),
        is_enum: false,
        variants: Vec::new(),
        attributes: Vec::new(),
    }
}

pub fn collect_type_strings_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Let(_, ty_annot, expr) => {
            if let Some(t) = ty_annot {
                out.insert(t.to_string());
            }
            collect_type_strings_expr(expr, out);
        }
        Stmt::LetTuple(bindings, expr) => {
            let binding_tys: Vec<String> = bindings.iter().map(|(_, t)| t.to_string()).collect();
            if binding_tys.iter().all(|t| !t.is_empty()) {
                out.insert(format!("({})", binding_tys.join(",")));
                for (_, t) in bindings {
                    out.insert(t.to_string());
                }
            }
            collect_type_strings_expr(expr, out);
        }
        Stmt::ExprStmt(expr) => collect_type_strings_expr(expr, out),
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_type_strings_expr(expr, out);
            }
        }
        Stmt::Assign(_, expr) => collect_type_strings_expr(expr, out),
        Stmt::AssignPlus(_, expr) => collect_type_strings_expr(expr, out),
        Stmt::AssignIndex(arr, idx, val) => {
            collect_type_strings_expr(arr, out);
            collect_type_strings_expr(idx, out);
            collect_type_strings_expr(val, out);
        }
        Stmt::AssignField(obj, _, val) => {
            collect_type_strings_expr(obj, out);
            collect_type_strings_expr(val, out);
        }
        Stmt::If(cond, body, else_body) => {
            collect_type_strings_expr(cond, out);
            for s in body {
                collect_type_strings_stmt(s, out);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    collect_type_strings_stmt(s, out);
                }
            }
        }
        Stmt::While(cond, body) => {
            collect_type_strings_expr(cond, out);
            for s in body {
                collect_type_strings_stmt(s, out);
            }
        }
        Stmt::For(_, _, body) => {
            for s in body {
                collect_type_strings_stmt(s, out);
            }
        }
    }
}

pub fn collect_type_strings_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::New(ty, args) => {
            out.insert(ty.to_string());
            for arg in args {
                collect_type_strings_expr(arg, out);
            }
        }
        Expr::StructInit(name, fields) => {
            out.insert(name.clone());
            for (_, e) in fields {
                collect_type_strings_expr(e, out);
            }
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_type_strings_expr(arg, out);
            }
        }
        Expr::MethodCall(obj, _, args) => {
            collect_type_strings_expr(obj, out);
            for arg in args {
                collect_type_strings_expr(arg, out);
            }
        }
        Expr::FieldAccess(obj, _) => {
            collect_type_strings_expr(obj, out);
        }
        Expr::IndexAccess(arr, idx) => {
            collect_type_strings_expr(arr, out);
            collect_type_strings_expr(idx, out);
        }
        Expr::Binary(l, _, r) => {
            collect_type_strings_expr(l, out);
            collect_type_strings_expr(r, out);
        }
        Expr::If(cond, then_b, else_b) => {
            collect_type_strings_expr(cond, out);
            let (t_stmts, t_val) = &**then_b;
            for s in t_stmts { collect_type_strings_stmt(s, out); }
            if let Some(v) = t_val { collect_type_strings_expr(v, out); }
            if let Some(eb) = else_b {
                let (e_stmts, e_val) = &**eb;
                for s in e_stmts { collect_type_strings_stmt(s, out); }
                if let Some(v) = e_val { collect_type_strings_expr(v, out); }
            }
        }
        Expr::Match(target, arms) => {
            collect_type_strings_expr(target, out);
            for arm in arms {
                for s in &arm.body { collect_type_strings_stmt(s, out); }
                if let Some(v) = &arm.val { collect_type_strings_expr(v, out); }
            }
        }
        Expr::ClosureInstantiate(_, env_name, captured) => {
            out.insert(env_name.clone());
            for c in captured {
                collect_type_strings_expr(c, out);
            }
        }
        Expr::Cast(e, t) => {
            collect_type_strings_expr(e, out);
            out.insert(t.to_string());
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            collect_type_strings_expr(func_expr, out);
            for arg in args {
                collect_type_strings_expr(arg, out);
            }
        }
        Expr::Spread(e) => {
            collect_type_strings_expr(e, out);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                collect_type_strings_expr(e, out);
            }
        }
        _ => {}
    }
}
