use crate::ast::*;
use std::collections::{HashMap, HashSet};

pub fn lift_closures_in_funcs(funcs: &mut Vec<Function>, structs: &mut Vec<StructDef>) {
    let mut new_funcs = Vec::new();
    let mut closure_idx = 0;

    let original_funcs = funcs.clone();

    for f in funcs.iter_mut() {
        let mut var_types = HashMap::new();
        for p in &f.params {
            var_types.insert(p.name.clone(), p.ty.to_string());
        }
        for s in &mut f.body {
            lift_closures_stmt(s, &mut var_types, &mut closure_idx, &mut new_funcs, structs, &f.name, &original_funcs);
        }
    }
    funcs.extend(new_funcs);
}

fn resolve_struct_name_for_closure(name: &str, structs: &[StructDef]) -> String {
    let normalized = if let Some(start) = name.find('<') {
        if let Some(end) = name.rfind('>') {
            let base = &name[..start];
            let args_str = &name[start + 1..end];
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
            let mut mono = base.to_string();
            for arg in args {
                mono.push('_');
                let resolved_arg = resolve_struct_name_for_closure(arg, structs);
                mono.push_str(&resolved_arg.replace("::", "_"));
            }
            mono
        } else {
            name.to_string()
        }
    } else {
        name.to_string()
    };

    for s in structs {
        if s.name == normalized || s.name.ends_with(&format!("::{}", normalized)) || normalized.ends_with(&format!("::{}", s.name)) {
            return s.name.clone();
        }
    }
    let last_seg = normalized.split("::").last().unwrap_or(&normalized);
    for s in structs {
        let s_last_seg = s.name.split("::").last().unwrap_or(&s.name);
        if s_last_seg == last_seg {
            return s.name.clone();
        }
    }
    normalized
}

fn infer_expr_type(
    expr: &Expr,
    var_types: &HashMap<String, String>,
    structs: &[StructDef],
    funcs: &[Function],
) -> Option<String> {
    match expr {
        Expr::Identifier(n) => var_types.get(n).cloned(),
        Expr::Integer(_) => Some("i32".to_string()),
        Expr::Float(_) => Some("f32".to_string()),
        Expr::Bool(_) => Some("bool".to_string()),
        Expr::StringLit(_) => Some("str".to_string()),
        Expr::StructInit(n, _) => Some(n.clone()),
        Expr::Cast(_, ty) => Some(ty.to_string()),
        Expr::Closure(func) => {
            let mut params_str = Vec::new();
            for p in &func.params {
                params_str.push(p.ty.to_string());
            }
            Some(format!("fn({}):{}", params_str.join(","), func.return_ty.to_string()))
        }
        Expr::Tuple(exprs) => {
            let mut tys = Vec::new();
            for e in exprs {
                match infer_expr_type(e, var_types, structs, funcs) {
                    Some(t) => tys.push(t),
                    None => return None,
                }
            }
            Some(format!("({})", tys.join(",")))
        }
        Expr::MapLit(pairs) => {
            if pairs.is_empty() {
                Some("Map<str, anyref>".to_string())
            } else {
                let k_ty = infer_expr_type(&pairs[0].0, var_types, structs, funcs).unwrap_or_else(|| "str".to_string());
                let first_v_ty = infer_expr_type(&pairs[0].1, var_types, structs, funcs).unwrap_or_else(|| "anyref".to_string());
                let mut v_ty = first_v_ty;
                for (_, v) in pairs.iter().skip(1) {
                    let cur_v_ty = infer_expr_type(v, var_types, structs, funcs).unwrap_or_else(|| "anyref".to_string());
                    if cur_v_ty != v_ty {
                        v_ty = "anyref".to_string();
                        break;
                    }
                }
                Some(format!("Map<{}, {}>", k_ty, v_ty))
            }
        }
        Expr::Binary(left, _, right) => {
            let l_ty = infer_expr_type(left, var_types, structs, funcs);
            let r_ty = infer_expr_type(right, var_types, structs, funcs);
            l_ty.or(r_ty)
        }
        Expr::FieldAccess(obj, field_name) => {
            if let Some(obj_ty) = infer_expr_type(obj, var_types, structs, funcs) {
                let resolved_ty = resolve_struct_name_for_closure(&obj_ty, structs);
                if let Some(s_def) = structs.iter().find(|s| s.name == resolved_ty) {
                    if let Some(f) = s_def.fields.iter().find(|f| f.name == *field_name) {
                        return Some(f.ty.to_string());
                    }
                }
            }
            None
        }
        Expr::Call(name, _args) => {
            // Split name to get base name and generic arguments
            let (base_name, explicit_args) = if let Some(start) = name.find('<') {
                let end = name.rfind('>').unwrap_or(start);
                let args_str = &name[start + 1..end];
                let mut explicit = Vec::new();
                let mut depth = 0;
                let mut current = String::new();
                for c in args_str.chars() {
                    if c == '<' { depth += 1; current.push(c); }
                    else if c == '>' { depth -= 1; current.push(c); }
                    else if c == ',' && depth == 0 { explicit.push(current.trim().to_string()); current.clear(); }
                    else { current.push(c); }
                }
                if !current.is_empty() { explicit.push(current.trim().to_string()); }
                (name[..start].to_string(), explicit)
            } else {
                (name.clone(), Vec::new())
            };

            // Find matching function
            let matched_func = funcs.iter().find(|f| {
                f.name == base_name || f.name.ends_with(&format!("::{}", base_name)) || base_name.ends_with(&format!("::{}", f.name))
            }).or_else(|| {
                let last_seg = base_name.split("::").last().unwrap_or(&base_name);
                funcs.iter().find(|f| {
                    let f_last_seg = f.name.split("::").last().unwrap_or(&f.name);
                    f_last_seg == last_seg
                })
            });

            if let Some(f) = matched_func {
                let mut ret_ty = f.return_ty.clone();
                // Perform generic substitution if there are explicit generic arguments
                if !f.generic.params.is_empty() && !explicit_args.is_empty() {
                    for (idx, gp) in f.generic.params.iter().enumerate() {
                        if idx < explicit_args.len() {
                            let replacement_ty = explicit_args[idx].parse::<Type>().unwrap();
                            ret_ty = ret_ty.substitute(&gp.name, &replacement_ty);
                        }
                    }
                }
                Some(ret_ty.to_string())
            } else {
                None
            }
        }
        Expr::MethodCall(obj, method, _args) => {
            if let Some(obj_ty) = infer_expr_type(obj, var_types, structs, funcs) {
                // E.g. obj_ty = "Signal<i32>"
                let (base_struct_name, explicit_args) = if let Some(start) = obj_ty.find('<') {
                    let end = obj_ty.rfind('>').unwrap_or(start);
                    let args_str = &obj_ty[start + 1..end];
                    let mut explicit = Vec::new();
                    let mut depth = 0;
                    let mut current = String::new();
                    for c in args_str.chars() {
                        if c == '<' { depth += 1; current.push(c); }
                        else if c == '>' { depth -= 1; current.push(c); }
                        else if c == ',' && depth == 0 { explicit.push(current.trim().to_string()); current.clear(); }
                        else { current.push(c); }
                    }
                    if !current.is_empty() { explicit.push(current.trim().to_string()); }
                    (obj_ty[..start].to_string(), explicit)
                } else {
                    (obj_ty.clone(), Vec::new())
                };

                let resolved_struct_name = resolve_struct_name_for_closure(&base_struct_name, structs);
                
                // Find parent struct
                let parent_struct = structs.iter().find(|s| s.name == resolved_struct_name);
                
                // Find matching method
                let matched_method = funcs.iter().find(|f| {
                    if let Some(ref parent) = f.parent_struct {
                        let f_parent_resolved = resolve_struct_name_for_closure(parent, structs);
                        if f_parent_resolved == resolved_struct_name {
                            let last_segment = f.name.split("::").last().unwrap_or(&f.name);
                            if last_segment == *method {
                                return true;
                            }
                        }
                    }
                    false
                }).or_else(|| {
                    if let Some(s) = parent_struct {
                        s.methods.iter().find(|f| {
                            let last_segment = f.name.split("::").last().unwrap_or(&f.name);
                            last_segment == *method
                        })
                    } else {
                        None
                    }
                });

                if let Some(m) = matched_method {
                    let mut ret_ty = m.return_ty.clone();
                    
                    // Substitute parent struct generic params
                    if let Some(s) = parent_struct {
                        if !s.generic.params.is_empty() && !explicit_args.is_empty() {
                            for (idx, gp) in s.generic.params.iter().enumerate() {
                                if idx < explicit_args.len() {
                                    let replacement_ty = explicit_args[idx].parse::<Type>().unwrap();
                                    ret_ty = ret_ty.substitute(&gp.name, &replacement_ty);
                                }
                            }
                        }
                    }
                    Some(ret_ty.to_string())
                } else if let Some(s) = parent_struct {
                    if let Some(field) = s.fields.iter().find(|f| f.name == *method) {
                    if let Type::Function(_, ref ret) = field.ty {
                        Some(ret.to_string())
                    } else {
                        None
                    }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn lift_closures_stmt(
    stmt: &mut Stmt, 
    var_types: &mut HashMap<String, String>, 
    closure_idx: &mut usize, 
    new_funcs: &mut Vec<Function>, 
    structs: &mut Vec<StructDef>,
    parent_func_name: &str,
    funcs: &[Function],
) {
    match stmt {
        Stmt::Let(name, ty, expr) => {
            let inferred_ty = ty.as_ref().map(|t| t.to_string()).or_else(|| infer_expr_type(expr, var_types, structs, funcs));
            lift_closures_expr(expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            var_types.insert(name.clone(), inferred_ty.unwrap_or_else(|| "anyref".to_string()));
        }
        Stmt::LetTuple(bindings, expr) => {
            lift_closures_expr(expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for (name, ty) in bindings {
                var_types.insert(name.clone(), ty.to_string());
            }
        }
        Stmt::Assign(_, expr) => {
            lift_closures_expr(expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Stmt::ExprStmt(expr) => {
            lift_closures_expr(expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Stmt::If(cond, body, else_body) => {
            lift_closures_expr(cond, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for s in body {
                lift_closures_stmt(s, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    lift_closures_stmt(s, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                }
            }
        }
        Stmt::While(cond, body) => {
            lift_closures_expr(cond, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for s in body {
                lift_closures_stmt(s, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Stmt::Return(Some(expr)) => {
            lift_closures_expr(expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        _ => {}
    }
}

fn lift_closures_expr(
    expr: &mut Expr, 
    var_types: &mut HashMap<String, String>, 
    closure_idx: &mut usize, 
    new_funcs: &mut Vec<Function>, 
    structs: &mut Vec<StructDef>,
    parent_func_name: &str,
    funcs: &[Function],
) {
    match expr {
        Expr::Tuple(exprs) => {
            for e in exprs {
                lift_closures_expr(e, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::Binary(l, _, r) => {
            lift_closures_expr(l, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            lift_closures_expr(r, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Expr::Call(_, args) => {
            for a in args {
                lift_closures_expr(a, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::MethodCall(obj, _, args) => {
            lift_closures_expr(obj, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for a in args {
                lift_closures_expr(a, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::InvokeFuncPtr(func_expr, args) => {
            lift_closures_expr(func_expr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for a in args {
                lift_closures_expr(a, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::StructInit(_, fields) => {
            for (_, e) in fields {
                lift_closures_expr(e, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::IndexAccess(arr, idx) => {
            lift_closures_expr(arr, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            lift_closures_expr(idx, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Expr::FieldAccess(obj, _) => {
            lift_closures_expr(obj, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Expr::Cast(e, _) => {
            lift_closures_expr(e, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
        }
        Expr::Match(cond, arms) => {
            let cond_ty = infer_expr_type(cond, var_types, structs, funcs).unwrap_or_else(|| "anyref".to_string());
            lift_closures_expr(cond, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for arm in arms {
                let mut arm_var_types = var_types.clone();
                if cond_ty != "anyref" {
                    let resolved_ty = resolve_struct_name_for_closure(&cond_ty, structs);
                    if let Some(s_def) = structs.iter().find(|s| s.name == resolved_ty) {
                        let (variant_name, bindings) = match &arm.pattern {
                            MatchPattern::Some(v) => ("Some".to_string(), vec![v.clone()]),
                            MatchPattern::None => ("None".to_string(), vec![]),
                            MatchPattern::Ok(v) => ("Ok".to_string(), vec![v.clone()]),
                            MatchPattern::Err(v) => ("Err".to_string(), vec![v.clone()]),
                            MatchPattern::Variant(name, binds) => {
                                (name.rsplit("::").next().unwrap().to_string(), binds.clone())
                            }
                            MatchPattern::CatchAll => ("".to_string(), vec![]),
                        };
                        for (j, binding_name) in bindings.iter().enumerate() {
                            let field_name = format!("{}_{}", variant_name, j);
                            if let Some(f) = s_def.fields.iter().find(|f| f.name == field_name) {
                                arm_var_types.insert(binding_name.clone(), f.ty.to_string());
                            }
                        }
                    }
                }
                for s in &mut arm.body {
                    lift_closures_stmt(s, &mut arm_var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                }
                if let Some(v) = &mut arm.val {
                    lift_closures_expr(v, &mut arm_var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                }
            }
        }
        Expr::If(cond, then_block, else_block) => {
            lift_closures_expr(cond, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            for s in &mut then_block.0 {
                lift_closures_stmt(s, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
            if let Some(v) = &mut then_block.1 {
                lift_closures_expr(v, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
            if let Some(eb) = else_block {
                for s in &mut eb.0 {
                    lift_closures_stmt(s, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                }
                if let Some(v) = &mut eb.1 {
                    lift_closures_expr(v, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                }
            }
        }
        Expr::New(_, args) => {
            for a in args {
                lift_closures_expr(a, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        Expr::Closure(func) => {
            let mut inner_var_types = var_types.clone();
            for p in &func.params {
                inner_var_types.insert(p.name.clone(), p.ty.to_string());
            }

            for s in &mut func.body {
                lift_closures_stmt(s, &mut inner_var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }

            *closure_idx += 1;
            let env_name = format!("__closure_env_{}", closure_idx);
            let func_name = format!("{}__closure_{}", parent_func_name, closure_idx);
            
            let mut free_vars = HashSet::new();
            collect_free_vars_in_block(&func.body, &mut free_vars, &inner_var_types);
            
            let mut local_vars = HashSet::new();
            for p in &func.params {
                local_vars.insert(p.name.clone());
            }
            collect_local_vars_in_block(&func.body, &mut local_vars);
            
            let mut captured_vars = Vec::new();
            for f in free_vars {
                if !local_vars.contains(&f) && var_types.contains_key(&f) {
                    captured_vars.push(f);
                }
            }
            captured_vars.sort();

            let mut env_fields = Vec::new();
            for v in &captured_vars {
                env_fields.push(Field {
                    name: v.clone(),
                    ty: var_types[v].parse::<Type>().unwrap(),
                    attributes: Vec::new(),
                });
            }
            structs.push(StructDef {
                is_pub: false,
                name: env_name.clone(),
                generic: GenericParams::default(),
                fields: env_fields,
                methods: Vec::new(),
                is_enum: false,
                variants: Vec::new(),
                attributes: Vec::new(),
            });

            let mut new_body = Vec::new();
            if !captured_vars.is_empty() {
                new_body.push(Stmt::Let(
                    "__env_struct".to_string(),
                    Some(env_name.parse::<Type>().unwrap()),
                    Expr::Cast(Box::new(Expr::Identifier("__env".to_string())), env_name.parse::<Type>().unwrap())
                ));
                for v in &captured_vars {
                    new_body.push(Stmt::Let(
                        v.clone(),
                        Some(var_types[v].parse::<Type>().unwrap()),
                        Expr::FieldAccess(Box::new(Expr::Identifier("__env_struct".to_string())), v.clone())
                    ));
                }
            }
            new_body.extend(func.body.clone());

            let mut new_func = *func.clone();
            new_func.name = func_name.clone();
            new_func.params.push(Param {
                name: "__env".to_string(),
                ty: Type::Anyref,
                is_variadic: false,
            });
            new_func.body = new_body;
            new_funcs.push(new_func);

            let mut captured_exprs = Vec::new();
            for v in &captured_vars {
                captured_exprs.push(Expr::Identifier(v.clone()));
            }
            *expr = Expr::ClosureInstantiate(func_name, env_name, captured_exprs);
        }
        Expr::MapLit(pairs) => {
            for (k, v) in pairs {
                lift_closures_expr(k, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
                lift_closures_expr(v, var_types, closure_idx, new_funcs, structs, parent_func_name, funcs);
            }
        }
        _ => {}
    }
}

fn collect_free_vars_in_block(body: &[Stmt], free_vars: &mut HashSet<String>, _var_types: &HashMap<String, String>) {
    for s in body {
        get_read_vars_stmt(s, free_vars);
        crate::optimizer::get_modified_vars(s, free_vars);
    }
}

fn collect_local_vars_in_block(body: &[Stmt], local_vars: &mut HashSet<String>) {
    for s in body {
        match s {
            Stmt::Let(n, _, _) => { local_vars.insert(n.clone()); }
            Stmt::LetTuple(bindings, _) => {
                for (n, _) in bindings {
                    local_vars.insert(n.clone());
                }
            }
            Stmt::For(n, _, _) => { local_vars.insert(n.clone()); }
            Stmt::If(_, then_body, else_body) => {
                collect_local_vars_in_block(then_body, local_vars);
                if let Some(eb) = else_body {
                    collect_local_vars_in_block(eb, local_vars);
                }
            }
            Stmt::While(_, body) => {
                collect_local_vars_in_block(body, local_vars);
            }
            _ => {}
        }
    }
}

fn get_read_vars_stmt(stmt: &Stmt, vars: &mut HashSet<String>) {
    match stmt {
        Stmt::Let(_, _, expr) | Stmt::Assign(_, expr) | Stmt::AssignPlus(_, expr) | Stmt::ExprStmt(expr) => {
            crate::optimizer::get_read_vars(expr, vars);
        }
        Stmt::LetTuple(_, expr) => {
            crate::optimizer::get_read_vars(expr, vars);
        }
        Stmt::AssignIndex(arr, idx, expr) => {
            crate::optimizer::get_read_vars(arr, vars);
            crate::optimizer::get_read_vars(idx, vars);
            crate::optimizer::get_read_vars(expr, vars);
        }
        Stmt::AssignField(obj, _, expr) => {
            crate::optimizer::get_read_vars(obj, vars);
            crate::optimizer::get_read_vars(expr, vars);
        }
        Stmt::If(cond, body, else_body) => {
            crate::optimizer::get_read_vars(cond, vars);
            for s in body { get_read_vars_stmt(s, vars); }
            if let Some(eb) = else_body {
                for s in eb { get_read_vars_stmt(s, vars); }
            }
        }
        Stmt::While(cond, body) => {
            crate::optimizer::get_read_vars(cond, vars);
            for s in body { get_read_vars_stmt(s, vars); }
        }
        Stmt::For(_, iter, body) => {
            vars.insert(iter.clone());
            for s in body { get_read_vars_stmt(s, vars); }
        }
        Stmt::Return(Some(expr)) => {
            crate::optimizer::get_read_vars(expr, vars);
        }
        _ => {}
    }
}
