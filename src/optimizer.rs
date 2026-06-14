use crate::ast::*;
use std::collections::{HashMap, HashSet};

use std::str::FromStr;

pub fn substitute_type(ty: &str, generic_name: &str, replacement: &str) -> String {
    let ty_parsed = Type::from_str(ty).unwrap_or(Type::GenericParam(ty.to_string()));
    let rep_parsed = Type::from_str(replacement).unwrap_or(Type::GenericParam(replacement.to_string()));
    ty_parsed.substitute(generic_name, &rep_parsed).to_string()
}

pub fn substitute_expr(expr: &Expr, generic_name: &str, replacement: &str) -> Expr {
    let replacement_ty = Type::from_str(replacement).unwrap_or(Type::GenericParam(replacement.to_string()));
    substitute_expr_typed(expr, generic_name, &replacement_ty)
}

fn substitute_expr_typed(expr: &Expr, generic_name: &str, replacement: &Type) -> Expr {
    match expr {
        Expr::Binary(l, op, r) => Expr::Binary(
            Box::new(substitute_expr_typed(l, generic_name, replacement)),
            *op,
            Box::new(substitute_expr_typed(r, generic_name, replacement)),
        ),
        Expr::Identifier(n) => Expr::Identifier(n.clone()),
        Expr::Integer(v) => Expr::Integer(v.clone()),
        Expr::Float(f) => Expr::Float(*f),
        Expr::StringLit(s) => Expr::StringLit(s.clone()),
        Expr::Bool(b) => Expr::Bool(*b),
        Expr::MethodCall(obj, method, args) => Expr::MethodCall(
            Box::new(substitute_expr_typed(obj, generic_name, replacement)),
            method.clone(),
            args.iter()
                .map(|a| substitute_expr_typed(a, generic_name, replacement))
                .collect(),
        ),
        Expr::FieldAccess(obj, field) => Expr::FieldAccess(
            Box::new(substitute_expr_typed(obj, generic_name, replacement)),
            field.clone(),
        ),
        Expr::StructInit(name, fields) => {
            let name_ty = Type::from_str(name).unwrap_or(Type::GenericParam(name.clone()));
            let subbed_name = name_ty.substitute(generic_name, replacement).to_string();
            Expr::StructInit(
                subbed_name,
                fields
                    .iter()
                    .map(|(n, e)| (n.clone(), substitute_expr_typed(e, generic_name, replacement)))
                    .collect(),
            )
        }
        Expr::Call(name, args) => {
            let name_ty = Type::from_str(name).unwrap_or(Type::GenericParam(name.clone()));
            let subbed_name = name_ty.substitute(generic_name, replacement).to_string();
            Expr::Call(
                subbed_name,
                args.iter()
                    .map(|a| substitute_expr_typed(a, generic_name, replacement))
                    .collect(),
            )
        }
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(substitute_expr_typed(arr, generic_name, replacement)),
            Box::new(substitute_expr_typed(idx, generic_name, replacement)),
        ),
        Expr::New(ty, args) => Expr::New(
            ty.substitute(generic_name, replacement),
            args.iter()
                .map(|a| substitute_expr_typed(a, generic_name, replacement))
                .collect(),
        ),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.iter().map(|s| substitute_stmt_typed(s, generic_name, replacement)).collect(),
                    e_val.as_ref().map(|v| substitute_expr_typed(v, generic_name, replacement))
                ))
            });
            Expr::If(
                Box::new(substitute_expr_typed(cond, generic_name, replacement)),
                Box::new((
                    t_stmts.iter().map(|s| substitute_stmt_typed(s, generic_name, replacement)).collect(),
                    t_val.as_ref().map(|v| substitute_expr_typed(v, generic_name, replacement))
                )),
                new_else
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(substitute_expr_typed(cond, generic_name, replacement)),
            arms.iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: arm.body.iter().map(|s| substitute_stmt_typed(s, generic_name, replacement)).collect(),
                    val: arm.val.as_ref().map(|v| substitute_expr_typed(v, generic_name, replacement)),
                })
                .collect(),
        ),
        Expr::Default => Expr::Default,
        Expr::InvokeFuncPtr(func_expr, args) => Expr::InvokeFuncPtr(Box::new(substitute_expr_typed(func_expr, generic_name, replacement)), args.iter().map(|a| substitute_expr_typed(a, generic_name, replacement)).collect()),
        Expr::Closure(func) => Expr::Closure(func.clone()),
        Expr::ClosureInstantiate(f, env, captured) => Expr::ClosureInstantiate(f.clone(), env.clone(), captured.iter().map(|a| substitute_expr_typed(a, generic_name, replacement)).collect()),
        Expr::Cast(e, t) => Expr::Cast(Box::new(substitute_expr_typed(e, generic_name, replacement)), t.substitute(generic_name, replacement)),
        Expr::Spread(e) => Expr::Spread(Box::new(substitute_expr_typed(e, generic_name, replacement))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|a| substitute_expr_typed(a, generic_name, replacement)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (substitute_expr_typed(k, generic_name, replacement), substitute_expr_typed(v, generic_name, replacement))).collect()),
    }
}

pub fn substitute_stmt(stmt: &Stmt, generic_name: &str, replacement: &str) -> Stmt {
    let replacement_ty = Type::from_str(replacement).unwrap_or(Type::GenericParam(replacement.to_string()));
    substitute_stmt_typed(stmt, generic_name, &replacement_ty)
}

fn substitute_stmt_typed(stmt: &Stmt, generic_name: &str, replacement: &Type) -> Stmt {
    match stmt {
        Stmt::Let(name, ty_opt, expr) => {
            let new_ty_opt = ty_opt
                .as_ref()
                .map(|t| t.substitute(generic_name, replacement));
            Stmt::Let(
                name.clone(),
                new_ty_opt,
                substitute_expr_typed(expr, generic_name, replacement),
            )
        }
        Stmt::ExprStmt(expr) => {
            Stmt::ExprStmt(substitute_expr_typed(expr, generic_name, replacement))
        }
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(substitute_expr_typed(arr, generic_name, replacement)),
            Box::new(substitute_expr_typed(idx, generic_name, replacement)),
            substitute_expr_typed(val, generic_name, replacement),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(substitute_expr_typed(obj, generic_name, replacement)),
            field.clone(),
            substitute_expr_typed(val, generic_name, replacement),
        ),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|expr| substitute_expr_typed(expr, generic_name, replacement))),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(
            name.clone(),
            substitute_expr_typed(expr, generic_name, replacement),
        ),
        Stmt::If(cond, body, else_body) => {
            let mut new_else = None;
            if let Some(e_body) = else_body {
                new_else = Some(
                    e_body
                        .iter()
                        .map(|s| substitute_stmt_typed(s, generic_name, replacement))
                        .collect(),
                );
            }
            Stmt::If(
                substitute_expr_typed(cond, generic_name, replacement),
                body.iter()
                    .map(|s| substitute_stmt_typed(s, generic_name, replacement))
                    .collect(),
                new_else,
            )
        }
        Stmt::While(cond, body) => Stmt::While(
            substitute_expr_typed(cond, generic_name, replacement),
            body.iter()
                .map(|s| substitute_stmt_typed(s, generic_name, replacement))
                .collect(),
        ),
        Stmt::Assign(name, expr) => Stmt::Assign(
            name.clone(),
            substitute_expr_typed(expr, generic_name, replacement),
        ),
        Stmt::For(loop_var, target, body) => Stmt::For(
            loop_var.clone(),
            target.clone(),
            body.iter()
                .map(|s| substitute_stmt_typed(s, generic_name, replacement))
                .collect(),
        ),
        Stmt::LetTuple(bindings, expr) => {
            let new_bindings = bindings.iter().map(|(n, t)| (n.clone(), t.substitute(generic_name, replacement))).collect();
            Stmt::LetTuple(new_bindings, substitute_expr_typed(expr, generic_name, replacement))
        }
    }
}

pub fn generate_combinations(
    params: &[GenericParam],
    idx: usize,
    current: &mut Vec<String>,
    out: &mut Vec<Vec<String>>,
) {
    if idx == params.len() {
        out.push(current.clone());
        return;
    }
    for c in &params[idx].constraints {
        current.push(c.to_string());
        generate_combinations(params, idx + 1, current, out);
        current.pop();
    }
}

pub fn apply_multi_substitute_type(
    ty: &Type,
    params: &[GenericParam],
    choices: &[String],
) -> Type {
    let mut result = ty.clone();
    for (p, choice) in params.iter().zip(choices.iter()) {
        let replacement_ty = Type::from_str(choice).unwrap_or(Type::GenericParam(choice.to_string()));
        result = result.substitute(&p.name, &replacement_ty);
    }
    result
}

pub fn apply_multi_substitute_stmt(
    stmt: &Stmt,
    params: &[GenericParam],
    choices: &[String],
) -> Stmt {
    let mut result = stmt.clone();
    for (p, choice) in params.iter().zip(choices.iter()) {
        result = substitute_stmt(&result, &p.name, choice);
    }
    result
}

pub fn remap_type(ty: &Type, map: &std::collections::HashMap<String, String>) -> Type {
    let ty_str = ty.to_string();
    if let Some(r) = map.get(&ty_str) {
        return Type::from_str(r).unwrap_or(Type::GenericParam(r.clone()));
    }
    match ty {
        Type::GenericParam(name) => {
            if let Some(r) = map.get(name) {
                Type::from_str(r).unwrap_or(Type::GenericParam(r.clone()))
            } else {
                ty.clone()
            }
        }
        Type::Struct(name, args) => {
            let remap_name = map.get(name).cloned().unwrap_or(name.clone());
            let remap_args = args.iter().map(|arg| remap_type(arg, map)).collect();
            Type::Struct(remap_name, remap_args)
        }
        Type::Array(inner) => Type::Array(Box::new(remap_type(inner, map))),
        Type::Tuple(elems) => Type::Tuple(elems.iter().map(|el| remap_type(el, map)).collect()),
        Type::Function(params, ret) => {
            Type::Function(
                params.iter().map(|p| remap_type(p, map)).collect(),
                Box::new(remap_type(ret, map))
            )
        }
        _ => ty.clone(),
    }
}

pub fn remap_expr(expr: &Expr, map: &std::collections::HashMap<String, String>) -> Expr {
    match expr {
        Expr::Binary(l, op, r) => Expr::Binary(
            Box::new(remap_expr(l, map)),
            *op,
            Box::new(remap_expr(r, map)),
        ),
        Expr::MethodCall(obj, method, args) => Expr::MethodCall(
            Box::new(remap_expr(obj, map)),
            method.clone(),
            args.iter().map(|a| remap_expr(a, map)).collect(),
        ),
        Expr::FieldAccess(obj, field) => Expr::FieldAccess(
            Box::new(remap_expr(obj, map)),
            field.clone(),
        ),
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(remap_expr(arr, map)),
            Box::new(remap_expr(idx, map)),
        ),
        Expr::StructInit(name, fields) => {
            let name_ty = Type::from_str(name).unwrap_or(Type::GenericParam(name.clone()));
            let remap_name = remap_type(&name_ty, map).to_string();
            Expr::StructInit(
                remap_name,
                fields.iter().map(|(n, e)| (n.clone(), remap_expr(e, map))).collect(),
            )
        }
        Expr::New(ty, args) => Expr::New(
            remap_type(ty, map),
            args.iter().map(|a| remap_expr(a, map)).collect(),
        ),
        Expr::Call(name, args) => {
            let name_ty = Type::from_str(name).unwrap_or(Type::GenericParam(name.clone()));
            let remap_name = remap_type(&name_ty, map).to_string();
            Expr::Call(remap_name, args.iter().map(|a| remap_expr(a, map)).collect())
        }
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.iter().map(|s| remap_stmt(s, map)).collect(),
                    e_val.as_ref().map(|v| remap_expr(v, map))
                ))
            });
            Expr::If(
                Box::new(remap_expr(cond, map)),
                Box::new((
                    t_stmts.iter().map(|s| remap_stmt(s, map)).collect(),
                    t_val.as_ref().map(|v| remap_expr(v, map))
                )),
                new_else
            )
        }
        Expr::Match(expr, arms) => Expr::Match(
            Box::new(remap_expr(expr, map)),
            arms.iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: arm.body.iter().map(|s| remap_stmt(s, map)).collect(),
                    val: arm.val.as_ref().map(|v| remap_expr(v, map)),
                })
                .collect(),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(remap_expr(e, map))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|e| remap_expr(e, map)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (remap_expr(k, map), remap_expr(v, map))).collect()),
        Expr::Cast(e, t) => Expr::Cast(Box::new(remap_expr(e, map)), remap_type(t, map)),
        _ => expr.clone(),
    }
}

pub fn remap_stmt(stmt: &Stmt, map: &std::collections::HashMap<String, String>) -> Stmt {
    match stmt {
        Stmt::Let(name, ty_opt, expr) => Stmt::Let(
            name.clone(),
            ty_opt.as_ref().map(|t| remap_type(t, map)),
            remap_expr(expr, map),
        ),
        Stmt::Assign(name, expr) => Stmt::Assign(name.clone(), remap_expr(expr, map)),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(name.clone(), remap_expr(expr, map)),
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(remap_expr(arr, map)),
            Box::new(remap_expr(idx, map)),
            remap_expr(val, map),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(remap_expr(obj, map)),
            field.clone(),
            remap_expr(val, map),
        ),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|expr| remap_expr(expr, map))),
        Stmt::ExprStmt(expr) => Stmt::ExprStmt(remap_expr(expr, map)),
        Stmt::If(cond, body, else_body) => Stmt::If(
            remap_expr(cond, map),
            body.iter().map(|s| remap_stmt(s, map)).collect(),
            else_body.as_ref().map(|e| e.iter().map(|s| remap_stmt(s, map)).collect()),
        ),
        Stmt::While(cond, body) => Stmt::While(
            remap_expr(cond, map),
            body.iter().map(|s| remap_stmt(s, map)).collect(),
        ),
        Stmt::For(lv, t, body) => Stmt::For(
            lv.clone(),
            t.clone(),
            body.iter().map(|s| remap_stmt(s, map)).collect(),
        ),
        Stmt::LetTuple(bindings, expr) => {
            let new_bindings = bindings.iter().map(|(n, t)| (n.clone(), remap_type(t, map))).collect();
            Stmt::LetTuple(new_bindings, remap_expr(expr, map))
        }
    }
}


pub fn substitute_identifier_in_expr(expr: &Expr, target: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::Identifier(_) | Expr::Integer(_) | Expr::Float(_) | Expr::StringLit(_) | Expr::Bool(_) | Expr::Default => expr.clone(),
        Expr::InvokeFuncPtr(func_expr, args) => Expr::InvokeFuncPtr(Box::new(substitute_identifier_in_expr(func_expr, target, replacement)), args.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect()),
        Expr::Closure(func) => Expr::Closure(func.clone()), // Functions capture their own closure
        Expr::ClosureInstantiate(f, env, captured) => Expr::ClosureInstantiate(f.clone(), env.clone(), captured.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect()),
        Expr::Cast(e, t) => Expr::Cast(Box::new(substitute_identifier_in_expr(e, target, replacement)), t.clone()),
        Expr::Binary(l, op, r) => Expr::Binary(
            Box::new(substitute_identifier_in_expr(l, target, replacement)),
            *op,
            Box::new(substitute_identifier_in_expr(r, target, replacement)),
        ),
        Expr::MethodCall(obj, m, args) => Expr::MethodCall(
            Box::new(substitute_identifier_in_expr(obj, target, replacement)),
            m.clone(),
            args.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect(),
        ),
        Expr::FieldAccess(obj, f) => Expr::FieldAccess(
            Box::new(substitute_identifier_in_expr(obj, target, replacement)),
            f.clone(),
        ),
        Expr::StructInit(n, fields) => Expr::StructInit(
            n.clone(),
            fields.iter().map(|(fname, e)| (fname.clone(), substitute_identifier_in_expr(e, target, replacement))).collect(),
        ),
        Expr::Call(n, args) => Expr::Call(
            n.clone(),
            args.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect(),
        ),
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(substitute_identifier_in_expr(arr, target, replacement)),
            Box::new(substitute_identifier_in_expr(idx, target, replacement)),
        ),
        Expr::New(ty, args) => Expr::New(
            ty.clone(),
            args.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect(),
        ),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.clone(),
                    e_val.as_ref().map(|v| substitute_identifier_in_expr(v, target, replacement))
                ))
            });
            Expr::If(
                Box::new(substitute_identifier_in_expr(cond, target, replacement)),
                Box::new((
                    t_stmts.clone(),
                    t_val.as_ref().map(|v| substitute_identifier_in_expr(v, target, replacement))
                )),
                new_else
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(substitute_identifier_in_expr(cond, target, replacement)),
            arms.iter()
                .map(|arm| {
                    let is_shadowed = match &arm.pattern {
                        MatchPattern::Some(name) | MatchPattern::Ok(name) | MatchPattern::Err(name) => name == target,
                        MatchPattern::None => false,
                        MatchPattern::Variant(_, bindings) => bindings.contains(&target.to_string()),
                        MatchPattern::CatchAll => false,
                    };
                    MatchArm {
                        pattern: arm.pattern.clone(),
                        body: arm.body.iter().map(|s| {
                            if is_shadowed {
                                s.clone()
                            } else {
                                // Since we don't have substitute_identifier_in_stmt, we keep it as is
                                s.clone()
                            }
                        }).collect(),
                        val: arm.val.as_ref().map(|v| {
                            if is_shadowed {
                                v.clone()
                            } else {
                                substitute_identifier_in_expr(v, target, replacement)
                            }
                        }),
                    }
                })
                .collect(),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(substitute_identifier_in_expr(e, target, replacement))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|a| substitute_identifier_in_expr(a, target, replacement)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (substitute_identifier_in_expr(k, target, replacement), substitute_identifier_in_expr(v, target, replacement))).collect()),
    }
}

pub fn inline_calls_in_expr(expr: &Expr, func_map: &std::collections::HashMap<String, Function>, depth: usize) -> Expr {
    if depth > 10 { return expr.clone(); }
    match expr {
        Expr::Identifier(_) | Expr::Integer(_) | Expr::Float(_) | Expr::StringLit(_) | Expr::Bool(_) | Expr::Default => expr.clone(),
        Expr::InvokeFuncPtr(func_expr, args) => Expr::InvokeFuncPtr(Box::new(inline_calls_in_expr(func_expr, func_map, depth)), args.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::Closure(func) => Expr::Closure(func.clone()),
        Expr::ClosureInstantiate(f, env, captured) => Expr::ClosureInstantiate(f.clone(), env.clone(), captured.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::Cast(e, t) => Expr::Cast(Box::new(inline_calls_in_expr(e, func_map, depth)), t.clone()),
        Expr::Binary(l, op, r) => Expr::Binary(Box::new(inline_calls_in_expr(l, func_map, depth)), *op, Box::new(inline_calls_in_expr(r, func_map, depth))),
        Expr::MethodCall(obj, m, args) => Expr::MethodCall(Box::new(inline_calls_in_expr(obj, func_map, depth)), m.clone(), args.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::FieldAccess(obj, f) => Expr::FieldAccess(Box::new(inline_calls_in_expr(obj, func_map, depth)), f.clone()),
        Expr::StructInit(n, fields) => Expr::StructInit(n.clone(), fields.iter().map(|(fname, e)| (fname.clone(), inline_calls_in_expr(e, func_map, depth))).collect()),
        Expr::Call(name, args) => Expr::Call(name.clone(), args.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(Box::new(inline_calls_in_expr(arr, func_map, depth)), Box::new(inline_calls_in_expr(idx, func_map, depth))),
        Expr::New(ty, args) => Expr::New(ty.clone(), args.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
                    e_val.as_ref().map(|v| inline_calls_in_expr(v, func_map, depth))
                ))
            });
            Expr::If(
                Box::new(inline_calls_in_expr(cond, func_map, depth)),
                Box::new((
                    t_stmts.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
                    t_val.as_ref().map(|v| inline_calls_in_expr(v, func_map, depth))
                )),
                new_else
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(inline_calls_in_expr(cond, func_map, depth)),
            arms.iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: arm.body.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
                    val: arm.val.as_ref().map(|v| inline_calls_in_expr(v, func_map, depth)),
                })
                .collect(),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(inline_calls_in_expr(e, func_map, depth))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|a| inline_calls_in_expr(a, func_map, depth)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (inline_calls_in_expr(k, func_map, depth), inline_calls_in_expr(v, func_map, depth))).collect()),
    }
}

pub fn inline_calls_in_stmt(stmt: &Stmt, func_map: &std::collections::HashMap<String, Function>) -> Stmt {
    match stmt {
        Stmt::Let(name, ty, expr) => Stmt::Let(name.clone(), ty.clone(), inline_calls_in_expr(expr, func_map, 0)),
        Stmt::ExprStmt(expr) => Stmt::ExprStmt(inline_calls_in_expr(expr, func_map, 0)),
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(inline_calls_in_expr(arr, func_map, 0)),
            Box::new(inline_calls_in_expr(idx, func_map, 0)),
            inline_calls_in_expr(val, func_map, 0),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(inline_calls_in_expr(obj, func_map, 0)),
            field.clone(),
            inline_calls_in_expr(val, func_map, 0),
        ),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|expr| inline_calls_in_expr(expr, func_map, 0))),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(name.clone(), inline_calls_in_expr(expr, func_map, 0)),
        Stmt::Assign(name, expr) => Stmt::Assign(name.clone(), inline_calls_in_expr(expr, func_map, 0)),
        Stmt::If(cond, body, else_body) => Stmt::If(
            inline_calls_in_expr(cond, func_map, 0),
            body.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
            else_body.as_ref().map(|e| e.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect()),
        ),
        Stmt::While(cond, body) => Stmt::While(
            inline_calls_in_expr(cond, func_map, 0),
            body.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
        ),
        Stmt::For(loop_var, target, body) => Stmt::For(
            loop_var.clone(),
            target.clone(),
            body.iter().map(|s| inline_calls_in_stmt(s, func_map)).collect(),
        ),
        Stmt::LetTuple(bindings, expr) => Stmt::LetTuple(
            bindings.clone(),
            inline_calls_in_expr(expr, func_map, 0),
        ),
    }
}

pub fn get_modified_vars_expr(expr: &Expr, vars: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::If(cond, then_b, else_b) => {
            get_modified_vars_expr(cond, vars);
            let (t_stmts, t_val) = &**then_b;
            for s in t_stmts { get_modified_vars(s, vars); }
            if let Some(v) = t_val { get_modified_vars_expr(v, vars); }
            if let Some(eb) = else_b {
                let (e_stmts, e_val) = &**eb;
                for s in e_stmts { get_modified_vars(s, vars); }
                if let Some(v) = e_val { get_modified_vars_expr(v, vars); }
            }
        }
        Expr::Match(cond, arms) => {
            get_modified_vars_expr(cond, vars);
            for arm in arms {
                match &arm.pattern {
                    MatchPattern::Some(var_name) | MatchPattern::Ok(var_name) | MatchPattern::Err(var_name) => {
                        vars.insert(var_name.clone());
                    }
                    MatchPattern::Variant(_, bindings) => {
                        for binding_name in bindings {
                            vars.insert(binding_name.clone());
                        }
                    }
                    _ => {}
                }
                for s in &arm.body {
                    get_modified_vars(s, vars);
                }
                if let Some(v) = &arm.val {
                    get_modified_vars_expr(v, vars);
                }
            }
        }
        Expr::Binary(l, _, r) => {
            get_modified_vars_expr(l, vars);
            get_modified_vars_expr(r, vars);
        }
        Expr::MethodCall(obj, _, args) => {
            get_modified_vars_expr(obj, vars);
            for a in args { get_modified_vars_expr(a, vars); }
        }
        Expr::FieldAccess(obj, _) => {
            get_modified_vars_expr(obj, vars);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields { get_modified_vars_expr(e, vars); }
        }
        Expr::Call(_, args) => {
            for a in args { get_modified_vars_expr(a, vars); }
        }
        Expr::IndexAccess(arr, idx) => {
            get_modified_vars_expr(arr, vars);
            get_modified_vars_expr(idx, vars);
        }
        Expr::New(_, args) => {
            for a in args { get_modified_vars_expr(a, vars); }
        }
        Expr::Spread(e) => get_modified_vars_expr(e, vars),
        Expr::Tuple(exprs) => {
            for e in exprs { get_modified_vars_expr(e, vars); }
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                get_modified_vars_expr(k, vars);
                get_modified_vars_expr(v, vars);
            }
        }
        _ => {}
    }
}

pub fn get_modified_vars(stmt: &Stmt, vars: &mut std::collections::HashSet<String>) {
    match stmt {
        Stmt::Let(name, _, expr) => {
            vars.insert(name.clone());
            get_modified_vars_expr(expr, vars);
        }
        Stmt::LetTuple(bindings, expr) => {
            for (name, _) in bindings {
                vars.insert(name.clone());
            }
            get_modified_vars_expr(expr, vars);
        }
        Stmt::Assign(name, expr) => {
            vars.insert(name.clone());
            get_modified_vars_expr(expr, vars);
        }
        Stmt::AssignPlus(name, expr) => {
            vars.insert(name.clone());
            get_modified_vars_expr(expr, vars);
        }
        Stmt::AssignIndex(arr, idx, val) => {
            if let Expr::Identifier(n) = &**arr {
                vars.insert(n.clone());
            }
            get_modified_vars_expr(arr, vars);
            get_modified_vars_expr(idx, vars);
            get_modified_vars_expr(val, vars);
        }
        Stmt::AssignField(obj, _, val) => {
            get_modified_vars_expr(obj, vars);
            get_modified_vars_expr(val, vars);
        }
        Stmt::ExprStmt(expr) => {
            get_modified_vars_expr(expr, vars);
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                get_modified_vars_expr(expr, vars);
            }
        }
        Stmt::If(cond, body, else_body) => {
            get_modified_vars_expr(cond, vars);
            for s in body { get_modified_vars(s, vars); }
            if let Some(e) = else_body {
                for s in e { get_modified_vars(s, vars); }
            }
        }
        Stmt::While(cond, body) => {
            get_modified_vars_expr(cond, vars);
            for s in body { get_modified_vars(s, vars); }
        }
        Stmt::For(loop_var, _, body) => {
            vars.insert(loop_var.clone());
            for s in body { get_modified_vars(s, vars); }
        }
    }
}

pub fn get_read_vars(expr: &Expr, vars: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::Integer(_) | Expr::Float(_) | Expr::StringLit(_) | Expr::Bool(_) => {}
        Expr::Identifier(n) => { vars.insert(n.clone()); }
        Expr::Binary(l, _, r) => {
            get_read_vars(l, vars);
            get_read_vars(r, vars);
        }
        Expr::MethodCall(obj, _, args) => {
            get_read_vars(obj, vars);
            for a in args { get_read_vars(a, vars); }
        }
        Expr::FieldAccess(obj, _) => {
            get_read_vars(obj, vars);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields { get_read_vars(e, vars); }
        }
        Expr::Call(_, args) => {
            for a in args { get_read_vars(a, vars); }
        }
        Expr::IndexAccess(arr, idx) => {
            get_read_vars(arr, vars);
            get_read_vars(idx, vars);
        }
        Expr::New(_, args) => {
            for a in args { get_read_vars(a, vars); }
        }
        Expr::If(cond, then_b, else_b) => {
            get_read_vars(cond, vars);
            let (_, t_val) = &**then_b;
            if let Some(v) = t_val { get_read_vars(v, vars); }
            if let Some(eb) = else_b {
                let (_, e_val) = &**eb;
                if let Some(v) = e_val { get_read_vars(v, vars); }
            }
        }
        Expr::Match(cond, arms) => {
            get_read_vars(cond, vars);
            for arm in arms {
                if let Some(v) = &arm.val {
                    let mut val_vars = std::collections::HashSet::new();
                    get_read_vars(v, &mut val_vars);
                    match &arm.pattern {
                        MatchPattern::Some(name) | MatchPattern::Ok(name) | MatchPattern::Err(name) => {
                            val_vars.remove(name);
                        }
                        MatchPattern::Variant(_, bindings) => {
                            for binding in bindings {
                                val_vars.remove(binding);
                            }
                        }
                        _ => {}
                    }
                    vars.extend(val_vars);
                }
            }
        }
        Expr::Default => {}
        Expr::InvokeFuncPtr(func_expr, args) => {
            get_read_vars(func_expr, vars);
            for a in args { get_read_vars(a, vars); }
        }
        Expr::Closure(_) => {}
        Expr::ClosureInstantiate(_, _, captured) => {
            for a in captured { get_read_vars(a, vars); }
        }
        Expr::Cast(e, _) => get_read_vars(e, vars),
        Expr::Spread(e) => get_read_vars(e, vars),
        Expr::Tuple(exprs) => {
            for e in exprs { get_read_vars(e, vars); }
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                get_read_vars(k, vars);
                get_read_vars(v, vars);
            }
        }
    }
}

pub fn pass_loop_unswitch_stmt(stmt: &Stmt) -> Stmt {
    match stmt {
        Stmt::If(cond, body, else_body) => {
            Stmt::If(
                cond.clone(),
                pass_loop_unswitch_block(body),
                else_body.as_ref().map(|e| pass_loop_unswitch_block(e)),
            )
        }
        Stmt::While(cond, body) => {
            let mut modified_vars = std::collections::HashSet::new();
            for s in body {
                get_modified_vars(s, &mut modified_vars);
            }
            
            let mut unswitched_if = None;
            
            for (i, s) in body.iter().enumerate() {
                if let Stmt::If(if_cond, if_body, else_body) = s {
                    let mut read_vars = std::collections::HashSet::new();
                    get_read_vars(if_cond, &mut read_vars);
                    
                    let is_invariant = read_vars.intersection(&modified_vars).count() == 0;
                    if is_invariant {
                        unswitched_if = Some((i, if_cond.clone(), if_body.clone(), else_body.clone()));
                        break;
                    }
                }
            }
            
            if let Some((idx, if_cond, if_body, else_body)) = unswitched_if {
                let mut true_body = body.clone();
                true_body.remove(idx);
                for (j, insert_s) in if_body.iter().enumerate() {
                    true_body.insert(idx + j, insert_s.clone());
                }
                
                let mut false_body = body.clone();
                false_body.remove(idx);
                if let Some(e_body) = else_body {
                    for (j, insert_s) in e_body.iter().enumerate() {
                        false_body.insert(idx + j, insert_s.clone());
                    }
                }
                
                return Stmt::If(
                    if_cond,
                    vec![Stmt::While(cond.clone(), pass_loop_unswitch_block(&true_body))],
                    Some(vec![Stmt::While(cond.clone(), pass_loop_unswitch_block(&false_body))])
                );
            }
            
            Stmt::While(cond.clone(), pass_loop_unswitch_block(body))
        }
        Stmt::For(loop_var, target, body) => {
            Stmt::For(
                loop_var.clone(),
                target.clone(),
                pass_loop_unswitch_block(body),
            )
        }
        _ => stmt.clone()
    }
}

pub fn pass_loop_unswitch_block(stmts: &[Stmt]) -> Vec<Stmt> {
    stmts.iter().map(pass_loop_unswitch_stmt).collect()
}

pub fn optimize_expr(expr: &Expr) -> Expr {
    match expr {
        Expr::Binary(l, op, r) => {
            let opt_l = optimize_expr(l);
            let opt_r = optimize_expr(r);
            if let (Expr::Integer(l_val), Expr::Integer(r_val)) = (&opt_l, &opt_r) {
                if let (Ok(l_num), Ok(r_num)) = (l_val.parse::<i32>(), r_val.parse::<i32>()) {
                    match op {
                        Op::Add => return Expr::Integer((l_num.wrapping_add(r_num)).to_string()),
                        Op::Sub => return Expr::Integer((l_num.wrapping_sub(r_num)).to_string()),
                        Op::Mul => return Expr::Integer((l_num.wrapping_mul(r_num)).to_string()),
                        Op::Div => if r_num != 0 { return Expr::Integer((l_num.wrapping_div(r_num)).to_string()); },
                        Op::BitAnd => return Expr::Integer((l_num & r_num).to_string()),
                        Op::BitXor => return Expr::Integer((l_num ^ r_num).to_string()),
                        Op::Rem => if r_num != 0 { return Expr::Integer((l_num.wrapping_rem(r_num)).to_string()); },
                        Op::ShiftLeft => return Expr::Integer((l_num << r_num).to_string()),
                        Op::ShiftRight => return Expr::Integer((l_num >> r_num).to_string()),
                        _ => {}
                    }
                }
            }
            if let (Expr::Float(l_val), Expr::Float(r_val)) = (&opt_l, &opt_r) {
                match op {
                    Op::Add => return Expr::Float(l_val + r_val),
                    Op::Sub => return Expr::Float(l_val - r_val),
                    Op::Mul => return Expr::Float(l_val * r_val),
                    Op::Div => return Expr::Float(l_val / r_val),
                    _ => {}
                }
            }
            Expr::Binary(Box::new(opt_l), *op, Box::new(opt_r))
        }
        Expr::MethodCall(obj, method, args) => Expr::MethodCall(
            Box::new(optimize_expr(obj)),
            method.clone(),
            args.iter().map(optimize_expr).collect(),
        ),
        Expr::FieldAccess(obj, field) => Expr::FieldAccess(
            Box::new(optimize_expr(obj)),
            field.clone(),
        ),
        Expr::StructInit(name, fields) => Expr::StructInit(
            name.clone(),
            fields.iter().map(|(n, e)| (n.clone(), optimize_expr(e))).collect(),
        ),
        Expr::Call(name, args) => Expr::Call(
            name.clone(),
            args.iter().map(optimize_expr).collect(),
        ),
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(optimize_expr(arr)),
            Box::new(optimize_expr(idx)),
        ),
        Expr::New(ty, args) => Expr::New(
            ty.clone(),
            args.iter().map(optimize_expr).collect(),
        ),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    optimize_block(e_stmts),
                    e_val.as_ref().map(|v| optimize_expr(v))
                ))
            });
            Expr::If(
                Box::new(optimize_expr(cond)),
                Box::new((
                    optimize_block(t_stmts),
                    t_val.as_ref().map(|v| optimize_expr(v))
                )),
                new_else
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(optimize_expr(cond)),
            arms.iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: optimize_block(&arm.body),
                    val: arm.val.as_ref().map(|v| optimize_expr(v)),
                })
                .collect(),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(optimize_expr(e))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(optimize_expr).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (optimize_expr(k), optimize_expr(v))).collect()),
        _ => expr.clone(),
    }
}

fn find_mutated_vars_block(stmts: &[Stmt], mutated: &mut std::collections::HashSet<String>) {
    for stmt in stmts {
        find_mutated_vars_stmt(stmt, mutated);
    }
}

fn find_mutated_vars_stmt(stmt: &Stmt, mutated: &mut std::collections::HashSet<String>) {
    match stmt {
        Stmt::Let(var, _, expr) => {
            mutated.insert(var.clone());
            find_mutated_vars_expr(expr, mutated);
        }
        Stmt::LetTuple(bindings, expr) => {
            for (var, _) in bindings {
                mutated.insert(var.clone());
            }
            find_mutated_vars_expr(expr, mutated);
        }
        Stmt::Assign(var, expr) => {
            mutated.insert(var.clone());
            find_mutated_vars_expr(expr, mutated);
        }
        Stmt::AssignPlus(var, expr) => {
            mutated.insert(var.clone());
            find_mutated_vars_expr(expr, mutated);
        }
        Stmt::AssignIndex(arr, idx, val) => {
            find_mutated_vars_expr(arr, mutated);
            find_mutated_vars_expr(idx, mutated);
            find_mutated_vars_expr(val, mutated);
        }
        Stmt::AssignField(obj, _, val) => {
            find_mutated_vars_expr(obj, mutated);
            find_mutated_vars_expr(val, mutated);
        }
        Stmt::ExprStmt(expr) => {
            find_mutated_vars_expr(expr, mutated);
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                find_mutated_vars_expr(expr, mutated);
            }
        }
        Stmt::If(cond, then_body, else_body) => {
            find_mutated_vars_expr(cond, mutated);
            find_mutated_vars_block(then_body, mutated);
            if let Some(eb) = else_body {
                find_mutated_vars_block(eb, mutated);
            }
        }
        Stmt::While(cond, body) => {
            find_mutated_vars_expr(cond, mutated);
            find_mutated_vars_block(body, mutated);
        }
        Stmt::For(loop_var, _, body) => {
            mutated.insert(loop_var.clone());
            find_mutated_vars_block(body, mutated);
        }
    }
}

fn find_mutated_vars_expr(expr: &Expr, mutated: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::MethodCall(obj, method, args) => {
            if let Expr::Identifier(var) = &**obj {
                if method != "len" {
                    mutated.insert(var.clone());
                }
            }
            find_mutated_vars_expr(obj, mutated);
            for arg in args {
                if let Expr::Identifier(var) = arg {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(arg, mutated);
            }
        }
        Expr::Call(_, args) => {
            for arg in args {
                if let Expr::Identifier(var) = arg {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(arg, mutated);
            }
        }
        Expr::InvokeFuncPtr(e, args) => {
            find_mutated_vars_expr(e, mutated);
            for arg in args {
                if let Expr::Identifier(var) = arg {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(arg, mutated);
            }
        }
        Expr::ClosureInstantiate(_, _, args) => {
            for arg in args {
                if let Expr::Identifier(var) = arg {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(arg, mutated);
            }
        }
        Expr::Binary(l, _, r) => {
            find_mutated_vars_expr(l, mutated);
            find_mutated_vars_expr(r, mutated);
        }
        Expr::FieldAccess(obj, _) => {
            find_mutated_vars_expr(obj, mutated);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields {
                if let Expr::Identifier(var) = e {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(e, mutated);
            }
        }
        Expr::IndexAccess(arr, idx) => {
            find_mutated_vars_expr(arr, mutated);
            find_mutated_vars_expr(idx, mutated);
        }
        Expr::New(_, args) => {
            for arg in args {
                if let Expr::Identifier(var) = arg {
                    mutated.insert(var.clone());
                }
                find_mutated_vars_expr(arg, mutated);
            }
        }
        Expr::If(cond, then_b, else_b) => {
            find_mutated_vars_expr(cond, mutated);
            let (t_stmts, t_val) = &**then_b;
            find_mutated_vars_block(t_stmts, mutated);
            if let Some(v) = t_val {
                find_mutated_vars_expr(v, mutated);
            }
            if let Some(eb) = else_b {
                let (e_stmts, e_val) = &**eb;
                find_mutated_vars_block(e_stmts, mutated);
                if let Some(v) = e_val {
                    find_mutated_vars_expr(v, mutated);
                }
            }
        }
        Expr::Match(cond, arms) => {
            find_mutated_vars_expr(cond, mutated);
            for arm in arms {
                find_mutated_vars_block(&arm.body, mutated);
                if let Some(v) = &arm.val {
                    find_mutated_vars_expr(v, mutated);
                }
            }
        }
        Expr::Closure(func) => {
            find_mutated_vars_block(&func.body, mutated);
        }
        Expr::Spread(e) => {
            find_mutated_vars_expr(e, mutated);
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                find_mutated_vars_expr(e, mutated);
            }
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                find_mutated_vars_expr(k, mutated);
                find_mutated_vars_expr(v, mutated);
            }
        }
        Expr::Cast(e, _) => {
            find_mutated_vars_expr(e, mutated);
        }
        _ => {}
    }
}

pub fn hoist_array_len_expr(expr: &Expr, hoisted: &mut std::collections::HashSet<String>, mutated_vars: &std::collections::HashSet<String>) -> Expr {
    match expr {
        Expr::MethodCall(obj, method, args) if method == "len" && args.is_empty() => {
            if let Expr::Identifier(arr_name) = &**obj {
                if !mutated_vars.contains(arr_name) {
                    hoisted.insert(arr_name.clone());
                    return Expr::Identifier(format!("_hoist_{}_len", arr_name));
                }
            }
            Expr::MethodCall(Box::new(hoist_array_len_expr(obj, hoisted, mutated_vars)), method.clone(), args.clone())
        }
        Expr::Binary(l, op, r) => Expr::Binary(
            Box::new(hoist_array_len_expr(l, hoisted, mutated_vars)),
            *op,
            Box::new(hoist_array_len_expr(r, hoisted, mutated_vars)),
        ),
        Expr::MethodCall(obj, method, args) => Expr::MethodCall(
            Box::new(hoist_array_len_expr(obj, hoisted, mutated_vars)),
            method.clone(),
            args.iter().map(|a| hoist_array_len_expr(a, hoisted, mutated_vars)).collect(),
        ),
        Expr::FieldAccess(obj, field) => Expr::FieldAccess(
            Box::new(hoist_array_len_expr(obj, hoisted, mutated_vars)),
            field.clone(),
        ),
        Expr::StructInit(name, fields) => Expr::StructInit(
            name.clone(),
            fields.iter().map(|(n, e)| (n.clone(), hoist_array_len_expr(e, hoisted, mutated_vars))).collect(),
        ),
        Expr::Call(name, args) => Expr::Call(
            name.clone(),
            args.iter().map(|a| hoist_array_len_expr(a, hoisted, mutated_vars)).collect(),
        ),
        Expr::IndexAccess(arr, idx) => Expr::IndexAccess(
            Box::new(hoist_array_len_expr(arr, hoisted, mutated_vars)),
            Box::new(hoist_array_len_expr(idx, hoisted, mutated_vars)),
        ),
        Expr::New(ty, args) => Expr::New(
            ty.clone(),
            args.iter().map(|a| hoist_array_len_expr(a, hoisted, mutated_vars)).collect(),
        ),
        Expr::If(cond, then_b, else_b) => {
            let (t_stmts, t_val) = &**then_b;
            let new_else = else_b.as_ref().map(|eb| {
                let (e_stmts, e_val) = &**eb;
                Box::new((
                    e_stmts.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
                    e_val.as_ref().map(|v| hoist_array_len_expr(v, hoisted, mutated_vars))
                ))
            });
            Expr::If(
                Box::new(hoist_array_len_expr(cond, hoisted, mutated_vars)),
                Box::new((
                    t_stmts.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
                    t_val.as_ref().map(|v| hoist_array_len_expr(v, hoisted, mutated_vars))
                )),
                new_else
            )
        }
        Expr::Match(cond, arms) => Expr::Match(
            Box::new(hoist_array_len_expr(cond, hoisted, mutated_vars)),
            arms.iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: arm.body.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
                    val: arm.val.as_ref().map(|v| hoist_array_len_expr(v, hoisted, mutated_vars)),
                })
                .collect(),
        ),
        Expr::Spread(e) => Expr::Spread(Box::new(hoist_array_len_expr(e, hoisted, mutated_vars))),
        Expr::Tuple(exprs) => Expr::Tuple(exprs.iter().map(|a| hoist_array_len_expr(a, hoisted, mutated_vars)).collect()),
        Expr::MapLit(pairs) => Expr::MapLit(pairs.iter().map(|(k, v)| (hoist_array_len_expr(k, hoisted, mutated_vars), hoist_array_len_expr(v, hoisted, mutated_vars))).collect()),
        _ => expr.clone(),
    }
}

pub fn hoist_array_len_stmt(stmt: &Stmt, hoisted: &mut std::collections::HashSet<String>, mutated_vars: &std::collections::HashSet<String>) -> Stmt {
    match stmt {
        Stmt::Let(name, ty, expr) => Stmt::Let(name.clone(), ty.clone(), hoist_array_len_expr(expr, hoisted, mutated_vars)),
        Stmt::ExprStmt(expr) => Stmt::ExprStmt(hoist_array_len_expr(expr, hoisted, mutated_vars)),
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(hoist_array_len_expr(arr, hoisted, mutated_vars)),
            Box::new(hoist_array_len_expr(idx, hoisted, mutated_vars)),
            hoist_array_len_expr(val, hoisted, mutated_vars),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(hoist_array_len_expr(obj, hoisted, mutated_vars)),
            field.clone(),
            hoist_array_len_expr(val, hoisted, mutated_vars),
        ),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|expr| hoist_array_len_expr(expr, hoisted, mutated_vars))),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(name.clone(), hoist_array_len_expr(expr, hoisted, mutated_vars)),
        Stmt::Assign(name, expr) => Stmt::Assign(name.clone(), hoist_array_len_expr(expr, hoisted, mutated_vars)),
        Stmt::If(cond, body, else_body) => Stmt::If(
            hoist_array_len_expr(cond, hoisted, mutated_vars),
            body.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
            else_body.as_ref().map(|e| e.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect()),
        ),
        Stmt::While(cond, body) => Stmt::While(
            hoist_array_len_expr(cond, hoisted, mutated_vars),
            body.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
        ),
        Stmt::For(loop_var, target, body) => Stmt::For(
            loop_var.clone(),
            target.clone(),
            body.iter().map(|s| hoist_array_len_stmt(s, hoisted, mutated_vars)).collect(),
        ),
        Stmt::LetTuple(bindings, expr) => Stmt::LetTuple(bindings.clone(), hoist_array_len_expr(expr, hoisted, mutated_vars)),
    }
}

pub fn optimize_stmt(stmt: &Stmt) -> Stmt {
    match stmt {
        Stmt::Let(name, ty, expr) => Stmt::Let(name.clone(), ty.clone(), optimize_expr(expr)),
        Stmt::ExprStmt(expr) => Stmt::ExprStmt(optimize_expr(expr)),
        Stmt::AssignIndex(arr, idx, val) => Stmt::AssignIndex(
            Box::new(optimize_expr(arr)),
            Box::new(optimize_expr(idx)),
            optimize_expr(val),
        ),
        Stmt::AssignField(obj, field, val) => Stmt::AssignField(
            Box::new(optimize_expr(obj)),
            field.clone(),
            optimize_expr(val),
        ),
        Stmt::Return(opt_expr) => Stmt::Return(opt_expr.as_ref().map(|expr| optimize_expr(expr))),
        Stmt::AssignPlus(name, expr) => Stmt::AssignPlus(name.clone(), optimize_expr(expr)),
        Stmt::Assign(name, expr) => Stmt::Assign(name.clone(), optimize_expr(expr)),
        Stmt::If(cond, body, else_body) => Stmt::If(
            optimize_expr(cond),
            optimize_block(body),
            else_body.as_ref().map(|e| optimize_block(e)),
        ),
        Stmt::While(cond, body) => Stmt::While(
            optimize_expr(cond),
            optimize_block(body),
        ),
        Stmt::For(loop_var, target, body) => Stmt::For(
            loop_var.clone(),
            target.clone(),
            optimize_block(body),
        ),
        Stmt::LetTuple(bindings, expr) => Stmt::LetTuple(bindings.clone(), optimize_expr(expr)),
    }
}

pub fn optimize_block(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut optimized = Vec::new();
    for stmt in stmts {
        let opt_stmt = optimize_stmt(stmt);
        
        let is_return = matches!(&opt_stmt, Stmt::Return(_));
        
        match opt_stmt {
            Stmt::If(cond, body, else_body) => {
                if let Expr::Integer(val) = &cond {
                    if val == "0" {
                        if let Some(e_body) = else_body {
                            optimized.extend(e_body);
                        }
                        continue;
                    } else {
                        optimized.extend(body);
                        continue;
                    }
                } else {
                    optimized.push(Stmt::If(cond, body, else_body));
                }
            }
            Stmt::While(cond, body) => {
                let mut mutated_vars = std::collections::HashSet::new();
                find_mutated_vars_block(&body, &mut mutated_vars);

                let mut hoisted = std::collections::HashSet::new();
                let new_cond = hoist_array_len_expr(&cond, &mut hoisted, &mutated_vars);
                
                if let Expr::Integer(val) = &new_cond {
                    if val == "0" {
                        continue;
                    }
                }
                
                let mut sorted_hoists: Vec<String> = hoisted.into_iter().collect();
                sorted_hoists.sort();
                for arr_name in sorted_hoists {
                    optimized.push(Stmt::Let(
                        format!("_hoist_{}_len", arr_name),
                        Some(Type::I32),
                        Expr::MethodCall(Box::new(Expr::Identifier(arr_name)), "len".to_string(), vec![])
                    ));
                }
                
                optimized.push(Stmt::While(new_cond, body));
            }
            _ => {
                optimized.push(opt_stmt);
            }
        }
        
        if is_return {
            break;
        }
    }
    optimized
}

fn count_usages_stmt(
    stmt: &Stmt,
    total_usages: &mut HashMap<String, usize>,
    call_usages: &mut HashMap<String, usize>,
) {
    match stmt {
        Stmt::Let(_, _, expr) => {
            count_usages_expr(expr, total_usages, call_usages);
        }
        Stmt::LetTuple(_, expr) => {
            count_usages_expr(expr, total_usages, call_usages);
        }
        Stmt::Assign(name, expr) => {
            *total_usages.entry(name.clone()).or_insert(0) += 1;
            count_usages_expr(expr, total_usages, call_usages);
        }
        Stmt::AssignPlus(name, expr) => {
            *total_usages.entry(name.clone()).or_insert(0) += 1;
            count_usages_expr(expr, total_usages, call_usages);
        }
        Stmt::AssignIndex(arr, idx, val) => {
            count_usages_expr(arr, total_usages, call_usages);
            count_usages_expr(idx, total_usages, call_usages);
            count_usages_expr(val, total_usages, call_usages);
        }
        Stmt::AssignField(obj, _, val) => {
            count_usages_expr(obj, total_usages, call_usages);
            count_usages_expr(val, total_usages, call_usages);
        }
        Stmt::ExprStmt(expr) => {
            count_usages_expr(expr, total_usages, call_usages);
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                count_usages_expr(expr, total_usages, call_usages);
            }
        }
        Stmt::If(cond, then_body, else_body) => {
            count_usages_expr(cond, total_usages, call_usages);
            for s in then_body {
                count_usages_stmt(s, total_usages, call_usages);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    count_usages_stmt(s, total_usages, call_usages);
                }
            }
        }
        Stmt::While(cond, body) => {
            count_usages_expr(cond, total_usages, call_usages);
            for s in body {
                count_usages_stmt(s, total_usages, call_usages);
            }
        }
        Stmt::For(_, _, body) => {
            for s in body {
                count_usages_stmt(s, total_usages, call_usages);
            }
        }
    }
}

fn count_usages_expr(
    expr: &Expr,
    total_usages: &mut HashMap<String, usize>,
    call_usages: &mut HashMap<String, usize>,
) {
    match expr {
        Expr::Identifier(name) => {
            *total_usages.entry(name.clone()).or_insert(0) += 1;
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            if let Expr::Identifier(name) = &**func_expr {
                *call_usages.entry(name.clone()).or_insert(0) += 1;
                *total_usages.entry(name.clone()).or_insert(0) += 1;
            } else {
                count_usages_expr(func_expr, total_usages, call_usages);
            }
            for a in args {
                count_usages_expr(a, total_usages, call_usages);
            }
        }
        Expr::Call(name, args) => {
            *call_usages.entry(name.clone()).or_insert(0) += 1;
            *total_usages.entry(name.clone()).or_insert(0) += 1;
            for a in args {
                count_usages_expr(a, total_usages, call_usages);
            }
        }
        Expr::MethodCall(obj, _, args) => {
            count_usages_expr(obj, total_usages, call_usages);
            for a in args {
                count_usages_expr(a, total_usages, call_usages);
            }
        }
        Expr::New(_, args) => {
            for a in args {
                count_usages_expr(a, total_usages, call_usages);
            }
        }
        Expr::Binary(l, _, r) => {
            count_usages_expr(l, total_usages, call_usages);
            count_usages_expr(r, total_usages, call_usages);
        }
        Expr::Cast(e, _) | Expr::Spread(e) | Expr::FieldAccess(e, _) => {
            count_usages_expr(e, total_usages, call_usages);
        }
        Expr::IndexAccess(arr, idx) => {
            count_usages_expr(arr, total_usages, call_usages);
            count_usages_expr(idx, total_usages, call_usages);
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields {
                count_usages_expr(e, total_usages, call_usages);
            }
        }
        Expr::Tuple(exprs) => {
            for e in exprs {
                count_usages_expr(e, total_usages, call_usages);
            }
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                count_usages_expr(k, total_usages, call_usages);
                count_usages_expr(v, total_usages, call_usages);
            }
        }
        Expr::Match(cond, arms) => {
            count_usages_expr(cond, total_usages, call_usages);
            for arm in arms {
                for s in &arm.body {
                    count_usages_stmt(s, total_usages, call_usages);
                }
                if let Some(val) = &arm.val {
                    count_usages_expr(val, total_usages, call_usages);
                }
            }
        }
        Expr::If(cond, then_branch, else_branch) => {
            count_usages_expr(cond, total_usages, call_usages);
            let (then_stmts, then_val) = &**then_branch;
            for s in then_stmts {
                count_usages_stmt(s, total_usages, call_usages);
            }
            if let Some(v) = then_val {
                count_usages_expr(v, total_usages, call_usages);
            }
            if let Some(eb) = else_branch {
                let (else_stmts, else_val) = &**eb;
                for s in else_stmts {
                    count_usages_stmt(s, total_usages, call_usages);
                }
                if let Some(v) = else_val {
                    count_usages_expr(v, total_usages, call_usages);
                }
            }
        }
        Expr::Closure(func) => {
            for s in &func.body {
                count_usages_stmt(s, total_usages, call_usages);
            }
        }
        _ => {}
    }
}

fn collect_locals_stmt(stmt: &Stmt, locals: &mut HashSet<String>) {
    match stmt {
        Stmt::Let(name, _, _) => {
            locals.insert(name.clone());
        }
        Stmt::LetTuple(bindings, _) => {
            for (name, _) in bindings {
                locals.insert(name.clone());
            }
        }
        Stmt::For(var, _, body) => {
            locals.insert(var.clone());
            for s in body {
                collect_locals_stmt(s, locals);
            }
        }
        Stmt::If(_, then_b, else_b) => {
            for s in then_b {
                collect_locals_stmt(s, locals);
            }
            if let Some(eb) = else_b {
                for s in eb {
                    collect_locals_stmt(s, locals);
                }
            }
        }
        Stmt::While(_, body) => {
            for s in body {
                collect_locals_stmt(s, locals);
            }
        }
        _ => {}
    }
}

fn collect_locals_in_body(body: &[Stmt], locals: &mut HashSet<String>) {
    for s in body {
        collect_locals_stmt(s, locals);
    }
}

fn has_complex_returns(body: &[Stmt]) -> bool {
    let mut return_count = 0;
    
    fn count_returns_stmt(stmt: &Stmt, count: &mut usize) {
        match stmt {
            Stmt::Return(_) => {
                *count += 1;
            }
            Stmt::If(_, then_b, else_b) => {
                for s in then_b {
                    count_returns_stmt(s, count);
                }
                if let Some(eb) = else_b {
                    for s in eb {
                        count_returns_stmt(s, count);
                    }
                }
            }
            Stmt::While(_, body) => {
                for s in body {
                    count_returns_stmt(s, count);
                }
            }
            Stmt::For(_, _, body) => {
                for s in body {
                    count_returns_stmt(s, count);
                }
            }
            _ => {}
        }
    }
    
    for s in body {
        count_returns_stmt(s, &mut return_count);
    }
    
    if return_count > 1 {
        return true;
    }
    
    if return_count == 1 {
        if let Some(Stmt::Return(_)) = body.last() {
            return false;
        }
        return true;
    }
    
    false
}

fn rename_vars_in_expr(expr: &Expr, rename_map: &HashMap<String, String>) -> Expr {
    match expr {
        Expr::Identifier(name) => {
            if let Some(new_name) = rename_map.get(name) {
                Expr::Identifier(new_name.clone())
            } else {
                expr.clone()
            }
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            Expr::InvokeFuncPtr(
                Box::new(rename_vars_in_expr(func_expr, rename_map)),
                args.iter().map(|a| rename_vars_in_expr(a, rename_map)).collect()
            )
        }
        Expr::Call(name, args) => {
            Expr::Call(name.clone(), args.iter().map(|a| rename_vars_in_expr(a, rename_map)).collect())
        }
        Expr::MethodCall(obj, m, args) => {
            Expr::MethodCall(
                Box::new(rename_vars_in_expr(obj, rename_map)),
                m.clone(),
                args.iter().map(|a| rename_vars_in_expr(a, rename_map)).collect()
            )
        }
        Expr::Binary(l, op, r) => {
            Expr::Binary(
                Box::new(rename_vars_in_expr(l, rename_map)),
                *op,
                Box::new(rename_vars_in_expr(r, rename_map))
            )
        }
        Expr::Cast(e, t) => {
            Expr::Cast(Box::new(rename_vars_in_expr(e, rename_map)), t.clone())
        }
        Expr::Spread(e) => {
            Expr::Spread(Box::new(rename_vars_in_expr(e, rename_map)))
        }
        Expr::FieldAccess(e, f) => {
            Expr::FieldAccess(Box::new(rename_vars_in_expr(e, rename_map)), f.clone())
        }
        Expr::IndexAccess(arr, idx) => {
            Expr::IndexAccess(
                Box::new(rename_vars_in_expr(arr, rename_map)),
                Box::new(rename_vars_in_expr(idx, rename_map))
            )
        }
        Expr::New(t, args) => {
            Expr::New(t.clone(), args.iter().map(|a| rename_vars_in_expr(a, rename_map)).collect())
        }
        Expr::StructInit(n, fields) => {
            Expr::StructInit(
                n.clone(),
                fields.iter().map(|(f, e)| (f.clone(), rename_vars_in_expr(e, rename_map))).collect()
            )
        }
        Expr::Tuple(exprs) => {
            Expr::Tuple(exprs.iter().map(|e| rename_vars_in_expr(e, rename_map)).collect())
        }
        Expr::MapLit(pairs) => {
            Expr::MapLit(pairs.iter().map(|(k, v)| (rename_vars_in_expr(k, rename_map), rename_vars_in_expr(v, rename_map))).collect())
        }
        Expr::Match(cond, arms) => {
            Expr::Match(
                Box::new(rename_vars_in_expr(cond, rename_map)),
                arms.iter().map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    body: arm.body.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect(),
                    val: arm.val.as_ref().map(|v| rename_vars_in_expr(v, rename_map)),
                }).collect()
            )
        }
        Expr::If(cond, then_branch, else_branch) => {
            let (then_stmts, then_val) = &**then_branch;
            let new_then = Box::new((
                then_stmts.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect(),
                then_val.as_ref().map(|v| rename_vars_in_expr(v, rename_map))
            ));
            let new_else = else_branch.as_ref().map(|eb| {
                let (else_stmts, else_val) = &**eb;
                Box::new((
                    else_stmts.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect(),
                    else_val.as_ref().map(|v| rename_vars_in_expr(v, rename_map))
                ))
            });
            Expr::If(
                Box::new(rename_vars_in_expr(cond, rename_map)),
                new_then,
                new_else
            )
        }
        Expr::Closure(func) => {
            let mut new_func = *func.clone();
            new_func.body = new_func.body.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect();
            Expr::Closure(Box::new(new_func))
        }
        _ => expr.clone()
    }
}

fn rename_vars_in_stmt(stmt: &Stmt, rename_map: &HashMap<String, String>) -> Stmt {
    match stmt {
        Stmt::Let(name, ty, expr) => {
            let new_name = rename_map.get(name).unwrap_or(name);
            Stmt::Let(new_name.clone(), ty.clone(), rename_vars_in_expr(expr, rename_map))
        }
        Stmt::LetTuple(bindings, expr) => {
            let new_bindings = bindings.iter().map(|(name, ty)| {
                let new_name = rename_map.get(name).unwrap_or(name);
                (new_name.clone(), ty.clone())
            }).collect();
            Stmt::LetTuple(new_bindings, rename_vars_in_expr(expr, rename_map))
        }
        Stmt::Assign(name, expr) => {
            let new_name = rename_map.get(name).unwrap_or(name);
            Stmt::Assign(new_name.clone(), rename_vars_in_expr(expr, rename_map))
        }
        Stmt::AssignPlus(name, expr) => {
            let new_name = rename_map.get(name).unwrap_or(name);
            Stmt::AssignPlus(new_name.clone(), rename_vars_in_expr(expr, rename_map))
        }
        Stmt::AssignIndex(arr, idx, val) => {
            Stmt::AssignIndex(
                Box::new(rename_vars_in_expr(arr, rename_map)),
                Box::new(rename_vars_in_expr(idx, rename_map)),
                rename_vars_in_expr(val, rename_map)
            )
        }
        Stmt::AssignField(obj, f, val) => {
            Stmt::AssignField(
                Box::new(rename_vars_in_expr(obj, rename_map)),
                f.clone(),
                rename_vars_in_expr(val, rename_map)
            )
        }
        Stmt::ExprStmt(expr) => {
            Stmt::ExprStmt(rename_vars_in_expr(expr, rename_map))
        }
        Stmt::Return(opt_expr) => {
            Stmt::Return(opt_expr.as_ref().map(|e| rename_vars_in_expr(e, rename_map)))
        }
        Stmt::If(cond, then_body, else_body) => {
            Stmt::If(
                rename_vars_in_expr(cond, rename_map),
                then_body.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect(),
                else_body.as_ref().map(|eb| eb.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect())
            )
        }
        Stmt::While(cond, body) => {
            Stmt::While(
                rename_vars_in_expr(cond, rename_map),
                body.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect()
            )
        }
        Stmt::For(var, iterable, body) => {
            let new_var = rename_map.get(var).unwrap_or(var);
            Stmt::For(
                new_var.clone(),
                iterable.clone(),
                body.iter().map(|s| rename_vars_in_stmt(s, rename_map)).collect()
            )
        }
    }
}

fn inline_local_closures_in_stmt(
    stmt: Stmt,
    closure_defs: &HashMap<String, Function>,
    var_counter: &mut usize,
    prepended_stmts: &mut Vec<Stmt>,
) -> Stmt {
    match stmt {
        Stmt::Let(name, ty, expr) => {
            if closure_defs.contains_key(&name) {
                Stmt::ExprStmt(Expr::Integer("0".to_string()))
            } else {
                Stmt::Let(
                    name,
                    ty,
                    inline_local_closures_in_expr(expr, closure_defs, var_counter, prepended_stmts)
                )
            }
        }
        Stmt::LetTuple(bindings, expr) => {
            Stmt::LetTuple(
                bindings,
                inline_local_closures_in_expr(expr, closure_defs, var_counter, prepended_stmts)
            )
        }
        Stmt::Assign(name, expr) => {
            Stmt::Assign(
                name,
                inline_local_closures_in_expr(expr, closure_defs, var_counter, prepended_stmts)
            )
        }
        Stmt::AssignPlus(name, expr) => {
            Stmt::AssignPlus(
                name,
                inline_local_closures_in_expr(expr, closure_defs, var_counter, prepended_stmts)
            )
        }
        Stmt::AssignIndex(arr, idx, val) => {
            Stmt::AssignIndex(
                Box::new(inline_local_closures_in_expr(*arr, closure_defs, var_counter, prepended_stmts)),
                Box::new(inline_local_closures_in_expr(*idx, closure_defs, var_counter, prepended_stmts)),
                inline_local_closures_in_expr(val, closure_defs, var_counter, prepended_stmts)
            )
        }
        Stmt::AssignField(obj, f, val) => {
            Stmt::AssignField(
                Box::new(inline_local_closures_in_expr(*obj, closure_defs, var_counter, prepended_stmts)),
                f,
                inline_local_closures_in_expr(val, closure_defs, var_counter, prepended_stmts)
            )
        }
        Stmt::ExprStmt(expr) => {
            Stmt::ExprStmt(inline_local_closures_in_expr(expr, closure_defs, var_counter, prepended_stmts))
        }
        Stmt::Return(opt_expr) => {
            Stmt::Return(opt_expr.map(|e| inline_local_closures_in_expr(e, closure_defs, var_counter, prepended_stmts)))
        }
        Stmt::If(cond, then_body, else_body) => {
            let new_cond = inline_local_closures_in_expr(cond, closure_defs, var_counter, prepended_stmts);
            
            let mut new_then = Vec::new();
            for s in then_body {
                let mut prep = Vec::new();
                let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                new_then.extend(prep);
                new_then.push(new_s);
            }
            
            let new_else = else_body.map(|eb| {
                let mut new_eb = Vec::new();
                for s in eb {
                    let mut prep = Vec::new();
                    let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                    new_eb.extend(prep);
                    new_eb.push(new_s);
                }
                new_eb
            });
            
            Stmt::If(new_cond, new_then, new_else)
        }
        Stmt::While(cond, body) => {
            let mut prep_cond = Vec::new();
            let new_cond = inline_local_closures_in_expr(cond, closure_defs, var_counter, &mut prep_cond);
            prepended_stmts.extend(prep_cond);
            
            let mut new_body = Vec::new();
            for s in body {
                let mut prep = Vec::new();
                let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                new_body.extend(prep);
                new_body.push(new_s);
            }
            
            Stmt::While(new_cond, new_body)
        }
        Stmt::For(var, iterable, body) => {
            let mut new_body = Vec::new();
            for s in body {
                let mut prep = Vec::new();
                let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                new_body.extend(prep);
                new_body.push(new_s);
            }
            Stmt::For(var, iterable, new_body)
        }
    }
}

fn inline_local_closures_in_expr(
    expr: Expr,
    closure_defs: &HashMap<String, Function>,
    var_counter: &mut usize,
    prepended_stmts: &mut Vec<Stmt>,
) -> Expr {
    match expr {
        Expr::InvokeFuncPtr(func_expr, args) => {
            if let Expr::Identifier(name) = &*func_expr {
                if let Some(func) = closure_defs.get(name) {
                    let mut rename_map = HashMap::new();
                    let mut locals = HashSet::new();
                    collect_locals_in_body(&func.body, &mut locals);
                    
                    let mut all_to_rename = locals;
                    for p in &func.params {
                        all_to_rename.insert(p.name.clone());
                    }
                    
                    for n in all_to_rename {
                        let unique_name = format!("__inline_var_{}_{}", n, *var_counter);
                        *var_counter += 1;
                        rename_map.insert(n, unique_name);
                    }
                    
                    for (i, p) in func.params.iter().enumerate() {
                        let arg_val = inline_local_closures_in_expr(args[i].clone(), closure_defs, var_counter, prepended_stmts);
                        let renamed_param = rename_map.get(&p.name).unwrap();
                        prepended_stmts.push(Stmt::Let(
                            renamed_param.clone(),
                            Some(p.ty.clone()),
                            arg_val
                        ));
                    }
                    
                    let mut cloned_body = Vec::new();
                    for s in &func.body {
                        cloned_body.push(rename_vars_in_stmt(s, &rename_map));
                    }
                    
                    if has_complex_returns(&cloned_body) {
                        panic!("Should not attempt to inline closure with complex returns");
                    }
                    
                    if let Some(last_stmt) = cloned_body.pop() {
                        match last_stmt {
                            Stmt::Return(Some(ret_expr)) => {
                                let ret_ty = func.return_ty.clone();
                                let return_var_name = format!("__inline_return_{}", *var_counter);
                                *var_counter += 1;
                                
                                prepended_stmts.extend(cloned_body);
                                
                                prepended_stmts.push(Stmt::Let(
                                    return_var_name.clone(),
                                    Some(ret_ty),
                                    ret_expr
                                ));
                                
                                return Expr::Identifier(return_var_name);
                            }
                            Stmt::Return(None) => {
                                prepended_stmts.extend(cloned_body);
                                return Expr::Default;
                            }
                            other => {
                                cloned_body.push(other);
                                prepended_stmts.extend(cloned_body);
                                return Expr::Default;
                            }
                        }
                    } else {
                        return Expr::Default;
                    }
                }
            }
            
            Expr::InvokeFuncPtr(
                Box::new(inline_local_closures_in_expr(*func_expr, closure_defs, var_counter, prepended_stmts)),
                args.into_iter().map(|a| inline_local_closures_in_expr(a, closure_defs, var_counter, prepended_stmts)).collect()
            )
        }
        Expr::Call(name, args) => {
            if let Some(func) = closure_defs.get(&name) {
                let mut rename_map = HashMap::new();
                let mut locals = HashSet::new();
                collect_locals_in_body(&func.body, &mut locals);
                
                let mut all_to_rename = locals;
                for p in &func.params {
                    all_to_rename.insert(p.name.clone());
                }
                
                for n in all_to_rename {
                    let unique_name = format!("__inline_var_{}_{}", n, *var_counter);
                    *var_counter += 1;
                    rename_map.insert(n, unique_name);
                }
                
                for (i, p) in func.params.iter().enumerate() {
                    let arg_val = inline_local_closures_in_expr(args[i].clone(), closure_defs, var_counter, prepended_stmts);
                    let renamed_param = rename_map.get(&p.name).unwrap();
                    prepended_stmts.push(Stmt::Let(
                        renamed_param.clone(),
                        Some(p.ty.clone()),
                        arg_val
                    ));
                }
                
                let mut cloned_body = Vec::new();
                for s in &func.body {
                    cloned_body.push(rename_vars_in_stmt(s, &rename_map));
                }
                
                if has_complex_returns(&cloned_body) {
                    panic!("Should not attempt to inline closure with complex returns");
                }
                
                if let Some(last_stmt) = cloned_body.pop() {
                    match last_stmt {
                        Stmt::Return(Some(ret_expr)) => {
                            let ret_ty = func.return_ty.clone();
                            let return_var_name = format!("__inline_return_{}", *var_counter);
                            *var_counter += 1;
                            
                            prepended_stmts.extend(cloned_body);
                            
                            prepended_stmts.push(Stmt::Let(
                                return_var_name.clone(),
                                Some(ret_ty),
                                ret_expr
                            ));
                            
                            return Expr::Identifier(return_var_name);
                        }
                        Stmt::Return(None) => {
                            prepended_stmts.extend(cloned_body);
                            return Expr::Default;
                        }
                        other => {
                            cloned_body.push(other);
                            prepended_stmts.extend(cloned_body);
                            return Expr::Default;
                        }
                    }
                } else {
                    return Expr::Default;
                }
            }
            
            Expr::Call(
                name,
                args.into_iter().map(|a| inline_local_closures_in_expr(a, closure_defs, var_counter, prepended_stmts)).collect()
            )
        }
        Expr::MethodCall(obj, m, args) => {
            Expr::MethodCall(
                Box::new(inline_local_closures_in_expr(*obj, closure_defs, var_counter, prepended_stmts)),
                m,
                args.into_iter().map(|a| inline_local_closures_in_expr(a, closure_defs, var_counter, prepended_stmts)).collect()
            )
        }
        Expr::Binary(l, op, r) => {
            Expr::Binary(
                Box::new(inline_local_closures_in_expr(*l, closure_defs, var_counter, prepended_stmts)),
                op,
                Box::new(inline_local_closures_in_expr(*r, closure_defs, var_counter, prepended_stmts))
            )
        }
        Expr::Cast(e, t) => {
            Expr::Cast(
                Box::new(inline_local_closures_in_expr(*e, closure_defs, var_counter, prepended_stmts)),
                t
            )
        }
        Expr::Spread(e) => {
            Expr::Spread(Box::new(inline_local_closures_in_expr(*e, closure_defs, var_counter, prepended_stmts)))
        }
        Expr::FieldAccess(e, f) => {
            Expr::FieldAccess(
                Box::new(inline_local_closures_in_expr(*e, closure_defs, var_counter, prepended_stmts)),
                f
            )
        }
        Expr::IndexAccess(arr, idx) => {
            Expr::IndexAccess(
                Box::new(inline_local_closures_in_expr(*arr, closure_defs, var_counter, prepended_stmts)),
                Box::new(inline_local_closures_in_expr(*idx, closure_defs, var_counter, prepended_stmts))
            )
        }
        Expr::New(t, args) => {
            Expr::New(
                t,
                args.into_iter().map(|a| inline_local_closures_in_expr(a, closure_defs, var_counter, prepended_stmts)).collect()
            )
        }
        Expr::StructInit(n, fields) => {
            Expr::StructInit(
                n,
                fields.into_iter().map(|(f, e)| (f, inline_local_closures_in_expr(e, closure_defs, var_counter, prepended_stmts))).collect()
            )
        }
        Expr::Tuple(exprs) => {
            Expr::Tuple(
                exprs.into_iter().map(|e| inline_local_closures_in_expr(e, closure_defs, var_counter, prepended_stmts)).collect()
            )
        }
        Expr::MapLit(pairs) => {
            Expr::MapLit(
                pairs.into_iter().map(|(k, v)| (
                    inline_local_closures_in_expr(k, closure_defs, var_counter, prepended_stmts),
                    inline_local_closures_in_expr(v, closure_defs, var_counter, prepended_stmts)
                )).collect()
            )
        }
        Expr::Match(cond, arms) => {
            Expr::Match(
                Box::new(inline_local_closures_in_expr(*cond, closure_defs, var_counter, prepended_stmts)),
                arms.into_iter().map(|arm| {
                    let mut new_body = Vec::new();
                    for s in arm.body {
                        let mut prep = Vec::new();
                        let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                        new_body.extend(prep);
                        new_body.push(new_s);
                    }
                    MatchArm {
                        pattern: arm.pattern,
                        body: new_body,
                        val: arm.val.map(|v| inline_local_closures_in_expr(v, closure_defs, var_counter, prepended_stmts)),
                    }
                }).collect()
            )
        }
        Expr::If(cond, then_branch, else_branch) => {
            let new_cond = inline_local_closures_in_expr(*cond, closure_defs, var_counter, prepended_stmts);
            
            let (then_stmts, then_val) = *then_branch;
            let mut new_then_stmts = Vec::new();
            for s in then_stmts {
                let mut prep = Vec::new();
                let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                new_then_stmts.extend(prep);
                new_then_stmts.push(new_s);
            }
            let new_then_val = then_val.map(|v| inline_local_closures_in_expr(v, closure_defs, var_counter, prepended_stmts));
            
            let new_else = else_branch.map(|eb| {
                let (else_stmts, else_val) = *eb;
                let mut new_else_stmts = Vec::new();
                for s in else_stmts {
                    let mut prep = Vec::new();
                    let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                    new_else_stmts.extend(prep);
                    new_else_stmts.push(new_s);
                }
                let new_else_val = else_val.map(|v| inline_local_closures_in_expr(v, closure_defs, var_counter, prepended_stmts));
                Box::new((new_else_stmts, new_else_val))
            });
            
            Expr::If(
                Box::new(new_cond),
                Box::new((new_then_stmts, new_then_val)),
                new_else
            )
        }
        Expr::Closure(func) => {
            let mut new_func = *func.clone();
            let mut new_body = Vec::new();
            for s in new_func.body {
                let mut prep = Vec::new();
                let new_s = inline_local_closures_in_stmt(s, closure_defs, var_counter, &mut prep);
                new_body.extend(prep);
                new_body.push(new_s);
            }
            new_func.body = new_body;
            Expr::Closure(Box::new(new_func))
        }
        other => other
    }
}

pub fn inline_closures_in_function(f: &mut Function) {
    let mut total_usages = HashMap::new();
    let mut call_usages = HashMap::new();
    
    for s in &f.body {
        count_usages_stmt(s, &mut total_usages, &mut call_usages);
    }
    
    let mut candidates = HashMap::new();
    
    fn find_closure_defs(stmt: &Stmt, candidates: &mut HashMap<String, Function>) {
        match stmt {
            Stmt::Let(name, _, Expr::Closure(func)) => {
                candidates.insert(name.clone(), *func.clone());
            }
            Stmt::If(_, then_b, else_b) => {
                for s in then_b {
                    find_closure_defs(s, candidates);
                }
                if let Some(eb) = else_b {
                    for s in eb {
                        find_closure_defs(s, candidates);
                    }
                }
            }
            Stmt::While(_, body) => {
                for s in body {
                    find_closure_defs(s, candidates);
                }
            }
            Stmt::For(_, _, body) => {
                for s in body {
                    find_closure_defs(s, candidates);
                }
            }
            _ => {}
        }
    }
    
    for s in &f.body {
        find_closure_defs(s, &mut candidates);
    }
    
    let mut closure_defs = HashMap::new();
    for (name, func) in candidates {
        let is_only_called = total_usages.get(&name) == call_usages.get(&name);
        if is_only_called && !has_complex_returns(&func.body) {
            closure_defs.insert(name, func);
        }
    }
    
    if closure_defs.is_empty() {
        return;
    }
    
    let mut var_counter = 0;
    let mut new_body = Vec::new();
    for s in f.body.drain(..) {
        let mut prep = Vec::new();
        let new_s = inline_local_closures_in_stmt(s, &closure_defs, &mut var_counter, &mut prep);
        new_body.extend(prep);
        new_body.push(new_s);
    }
    f.body = new_body;
}
