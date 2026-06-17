use crate::ast::*;
use std::collections::HashMap;
use std::collections::HashSet;
use wasmtime::*;

fn to_str<'a>(
    caller: &'a impl AsContext,
    val: Option<Rooted<ExternRef>>,
) -> Result<&'a str, wasmtime::Error> {
    if let Some(r) = val {
        if let Some(data) = r.data(caller)? {
            if let Some(s) = data.downcast_ref::<String>() {
                return Ok(s.as_str());
            }
        }
    }
    Ok("")
}

fn to_string(
    caller: &impl AsContext,
    val: Option<Rooted<ExternRef>>,
) -> Result<String, wasmtime::Error> {
    if let Some(r) = val {
        if let Some(data) = r.data(caller)? {
            if let Some(s) = data.downcast_ref::<String>() {
                return Ok(s.clone());
            }
        }
    }
    Ok(String::new())
}

fn to_ref(
    caller: impl AsContextMut,
    s: String,
) -> Result<Option<Rooted<ExternRef>>, wasmtime::Error> {
    let r = ExternRef::new(caller, s)?;
    Ok(Some(r))
}

fn to_ref_nonnull(
    caller: impl AsContextMut,
    s: String,
) -> Result<Rooted<ExternRef>, wasmtime::Error> {
    ExternRef::new(caller, s)
}

fn find_export(instance: &Instance, store: &mut Store<()>, suffix: &str) -> Option<Func> {
    for export in instance.exports(&mut *store) {
        if export.name().ends_with(suffix) {
            if let Some(func) = export.into_func() {
                return Some(func);
            }
        }
    }
    None
}

fn sanitize(name: &str) -> String {
    name.replace("::", "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}

fn call_func(func: &Func, store: &mut Store<()>, args: &[Val]) -> Result<Val, wasmtime::Error> {
    let mut results = vec![Val::null_any_ref(); func.ty(&*store).results().len()];
    func.call(&mut *store, args, &mut results)?;
    Ok(results.into_iter().next().unwrap_or(Val::null_any_ref()))
}

fn func_call_void(func: &Func, store: &mut Store<()>, args: &[Val]) -> Result<(), wasmtime::Error> {
    func.call(&mut *store, args, &mut [])?;
    Ok(())
}

fn setup_wasmtime_linker(
    linker: &mut Linker<()>,
    store: &mut Store<()>,
    module: &Module,
) -> Result<(), wasmtime::Error> {
    macro_rules! reg {
        ($full:expr, $short:expr, $func:expr) => {
            linker.func_wrap("env", $full, $func)?;
            linker.func_wrap("env", $short, $func)?;
        };
    }

    // 1. env module functions (registered under both full and shortened names)
    reg!("__fox_panic", "f_p", |caller: Caller<'_, ()>,
                                val: Option<Rooted<ExternRef>>|
     -> Result<(), wasmtime::Error> {
        let msg = to_str(&caller, val)?;
        Err(wasmtime::Error::msg(format!("Fox Panic: {}", msg)))
    });

    reg!("__fox_str_starts_with", "f_ss", |caller: Caller<'_, ()>,
                                           s: Option<
        Rooted<ExternRef>,
    >,
                                           prefix: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        let pre_str = to_str(&caller, prefix)?;
        Ok(if s_str.starts_with(pre_str) { 1 } else { 0 })
    });

    reg!("__fox_str_ends_with", "f_se", |caller: Caller<'_, ()>,
                                         s: Option<
        Rooted<ExternRef>,
    >,
                                         suffix: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        let suf_str = to_str(&caller, suffix)?;
        Ok(if s_str.ends_with(suf_str) { 1 } else { 0 })
    });

    reg!("__fox_str_contains", "f_sc", |caller: Caller<'_, ()>,
                                        s: Option<
        Rooted<ExternRef>,
    >,
                                        sub: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        let sub_str = to_str(&caller, sub)?;
        Ok(if s_str.contains(sub_str) { 1 } else { 0 })
    });

    reg!("__fox_str_index_of", "f_si", |caller: Caller<'_, ()>,
                                        s: Option<
        Rooted<ExternRef>,
    >,
                                        sub: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        let sub_str = to_str(&caller, sub)?;
        Ok(s_str.find(sub_str).map(|i| i as i32).unwrap_or(-1))
    });

    reg!("__fox_str_last_index_of", "f_sl", |caller: Caller<
        '_,
        (),
    >,
                                             s: Option<
        Rooted<ExternRef>,
    >,
                                             sub: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        let sub_str = to_str(&caller, sub)?;
        Ok(s_str.rfind(sub_str).map(|i| i as i32).unwrap_or(-1))
    });

    reg!("__fox_str_is_empty", "f_semp", |caller: Caller<'_, ()>,
                                          s: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s_str = to_str(&caller, s)?;
        Ok(if s_str.is_empty() { 1 } else { 0 })
    });

    reg!("__fox_str_eq", "f_seq", |caller: Caller<'_, ()>,
                                   a: Option<Rooted<ExternRef>>,
                                   b: Option<Rooted<ExternRef>>|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let a_str = to_str(&caller, a)?;
        let b_str = to_str(&caller, b)?;
        Ok(if a_str == b_str { 1 } else { 0 })
    });

    reg!("__fox_str_join", "f_sjn", |caller: Caller<'_, ()>,
                                     a: Option<Rooted<ExternRef>>,
                                     b: Option<Rooted<ExternRef>>|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        let a_str = to_string(&caller, a)?;
        let b_str = to_string(&caller, b)?;
        let joined = format!("{}{}", a_str, b_str);
        to_ref(caller, joined)
    });

    reg!("__fox_str_compare", "f_scmp", |caller: Caller<'_, ()>,
                                         a: Option<
        Rooted<ExternRef>,
    >,
                                         b: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let a_str = to_str(&caller, a)?;
        let b_str = to_str(&caller, b)?;
        Ok(match a_str.cmp(b_str) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        })
    });

    reg!("__fox_str_substring", "f_sub", |caller: Caller<'_, ()>,
                                          s: Option<
        Rooted<ExternRef>,
    >,
                                          start: i32,
                                          end: i32|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        let s_str = to_string(&caller, s)?;
        let start_idx = (start.max(0) as usize).min(s_str.len());
        let end_idx = (end.max(0) as usize).min(s_str.len());
        let sub = if start_idx <= end_idx {
            &s_str[start_idx..end_idx]
        } else {
            ""
        };
        to_ref(caller, sub.to_string())
    });

    reg!("__fox_f64_to_str", "f_f2s", |caller: Caller<'_, ()>,
                                       val: f64|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        to_ref(caller, val.to_string())
    });

    reg!("__fox_i32_to_str", "f_i2s", |caller: Caller<'_, ()>,
                                       val: i32|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        to_ref(caller, val.to_string())
    });

    reg!("__fox_i64_to_str", "f_l2s", |caller: Caller<'_, ()>,
                                       val: i64|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        to_ref(caller, val.to_string())
    });

    reg!("__fox_dom_console", "f_con", |caller: Caller<'_, ()>,
                                        level: i32,
                                        msg: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        (),
        wasmtime::Error,
    > {
        let msg_str = to_str(&caller, msg)?;
        match level {
            1 => eprintln!("{}", msg_str),
            2 => eprintln!("INFO: {}", msg_str),
            3 => eprintln!("WARN: {}", msg_str),
            4 => eprintln!("ERROR: {}", msg_str),
            5 => eprintln!("DEBUG: {}", msg_str),
            _ => eprintln!("{}", msg_str),
        }
        Ok(())
    });

    reg!("__fox_json_parse_int", "f_jpi", |caller: Caller<'_, ()>,
                                           val: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        i32,
        wasmtime::Error,
    > {
        let s = to_str(&caller, val)?;
        Ok(s.parse::<i32>().unwrap_or(0))
    });

    reg!("__fox_json_parse_float", "f_jpf", |caller: Caller<
        '_,
        (),
    >,
                                             val: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        f64,
        wasmtime::Error,
    > {
        let s = to_str(&caller, val)?;
        Ok(s.parse::<f64>().unwrap_or(0.0))
    });

    reg!("__fox_json_encode_string", "f_jes", |caller: Caller<
        '_,
        (),
    >,
                                               val: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        let s = to_string(&caller, val)?;
        let encoded = serde_json::to_string(&s).unwrap_or_default();
        to_ref(caller, encoded)
    });

    reg!("__fox_json_parse_string", "f_jps", |caller: Caller<
        '_,
        (),
    >,
                                              val: Option<
        Rooted<ExternRef>,
    >|
     -> Result<
        Option<Rooted<ExternRef>>,
        wasmtime::Error,
    > {
        let s = to_string(&caller, val)?;
        let parsed: String = serde_json::from_str(&s).unwrap_or_else(|_| s);
        to_ref(caller, parsed)
    });

    // 2. wasm:js-string module functions
    linker.func_wrap(
        "wasm:js-string",
        "length",
        |caller: Caller<'_, ()>, s: Option<Rooted<ExternRef>>| -> Result<i32, wasmtime::Error> {
            let s_str = to_str(&caller, s)?;
            Ok(s_str.encode_utf16().count() as i32)
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "charCodeAt",
        |caller: Caller<'_, ()>,
         s: Option<Rooted<ExternRef>>,
         index: i32|
         -> Result<i32, wasmtime::Error> {
            let s_str = to_str(&caller, s)?;
            let char_code = s_str.encode_utf16().nth(index as usize).unwrap_or(0);
            Ok(char_code as i32)
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "fromCharCode",
        |caller: Caller<'_, ()>, c: i32| -> Result<Rooted<ExternRef>, wasmtime::Error> {
            let character = std::char::from_u32(c as u32).unwrap_or(' ').to_string();
            to_ref_nonnull(caller, character)
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "concat",
        |caller: Caller<'_, ()>,
         a: Option<Rooted<ExternRef>>,
         b: Option<Rooted<ExternRef>>|
         -> Result<Rooted<ExternRef>, wasmtime::Error> {
            let a_str = to_string(&caller, a)?;
            let b_str = to_string(&caller, b)?;
            let joined = format!("{}{}", a_str, b_str);
            to_ref_nonnull(caller, joined)
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "substring",
        |caller: Caller<'_, ()>,
         s: Option<Rooted<ExternRef>>,
         start: i32,
         end: i32|
         -> Result<Rooted<ExternRef>, wasmtime::Error> {
            let s_str = to_string(&caller, s)?;
            let start_idx = (start.max(0) as usize).min(s_str.len());
            let end_idx = (end.max(0) as usize).min(s_str.len());
            let sub = if start_idx <= end_idx {
                &s_str[start_idx..end_idx]
            } else {
                ""
            };
            to_ref_nonnull(caller, sub.to_string())
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "equals",
        |caller: Caller<'_, ()>,
         a: Option<Rooted<ExternRef>>,
         b: Option<Rooted<ExternRef>>|
         -> Result<i32, wasmtime::Error> {
            let a_str = to_str(&caller, a)?;
            let b_str = to_str(&caller, b)?;
            Ok(if a_str == b_str { 1 } else { 0 })
        },
    )?;

    linker.func_wrap(
        "wasm:js-string",
        "compare",
        |caller: Caller<'_, ()>,
         a: Option<Rooted<ExternRef>>,
         b: Option<Rooted<ExternRef>>|
         -> Result<i32, wasmtime::Error> {
            let a_str = to_str(&caller, a)?;
            let b_str = to_str(&caller, b)?;
            Ok(match a_str.cmp(b_str) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            })
        },
    )?;

    // 3. Register stubs for any other function imports that are not explicitly defined.
    // This handles any missing import from env or wasm:js-string.
    for import in module.imports() {
        let module_name = import.module();
        if module_name == "env" || module_name == "wasm:js-string" {
            if let ExternType::Func(func_ty) = import.ty() {
                let name = import.name();
                if linker.get(&mut *store, module_name, name).is_none() {
                    let func_ty_clone = func_ty.clone();
                    let dummy_func =
                        Func::new(&mut *store, func_ty, move |mut caller, _args, results| {
                            for (i, ty) in func_ty_clone.results().enumerate() {
                                results[i] = match ty {
                                    ValType::I32 => Val::I32(0),
                                    ValType::I64 => Val::I64(0),
                                    ValType::F32 => Val::F32(0),
                                    ValType::F64 => Val::F64(0),
                                    ValType::Ref(ref_ty) => {
                                        if ref_ty.is_nullable() {
                                            Val::null_ref(ref_ty.heap_type())
                                        } else {
                                            let dummy =
                                                ExternRef::new(&mut caller, String::new()).unwrap();
                                            Val::ExternRef(Some(dummy))
                                        }
                                    }
                                    _ => Val::null_any_ref(),
                                };
                            }
                            Ok(())
                        });
                    linker.define(&mut *store, module_name, name, dummy_func)?;
                }
            }
        }
    }

    Ok(())
}

fn run_macros_inner(
    parsed_structs: &mut Vec<StructDef>,
    parsed_funcs: &mut Vec<Function>,
    parsed_impls: &mut Vec<ImplDef>,
    parsed_consts: &mut Vec<ConstDef>,
    imports_registry: &HashMap<String, HashSet<String>>,
) -> Result<bool, wasmtime::Error> {
    let mut has_macros = false;
    for s in parsed_structs.iter() {
        if !s.attributes.is_empty() {
            has_macros = true;
            break;
        }
    }

    if !has_macros {
        return Ok(false);
    }

    // Filter funcs to only those needed by macros: compiler fns, externs, and std:: fns
    let mut macro_funcs = Vec::new();
    let mut seen_funcs = HashSet::new();
    for f in parsed_funcs.iter() {
        if (f.is_compiler || f.is_extern || f.name.starts_with("std::"))
            && seen_funcs.insert(f.name.clone())
        {
            macro_funcs.push(f.clone());
        }
    }

    // Filter structs to remove generic ones, to avoid codegen errors
    let mut macro_structs = Vec::new();
    for s in parsed_structs.iter() {
        if s.generic.params.is_empty() {
            macro_structs.push(s.clone());
        }
    }

    let string_literals = crate::collect_string_literals(&macro_funcs, parsed_consts);

    let (wat_content, _filtered_structs) = crate::codegen::generate_wat(
        &macro_funcs,
        &macro_structs,
        &string_literals,
        parsed_consts,
        imports_registry,
    );

    let wasm_bytes = wat::parse_str(&wat_content)
        .map_err(|e| wasmtime::Error::msg(format!("Failed to compile macros to WASM: {:?}", e)))?;

    // Initialize Wasmtime engine with Wasm GC enabled
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm_bytes)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Setup host functions and stubs
    setup_wasmtime_linker(&mut linker, &mut store, &module)?;

    // Register string literal globals
    for (id, lit) in string_literals.iter().enumerate() {
        let val = Val::ExternRef(Some(ExternRef::new(&mut store, lit.clone())?));
        let global = Global::new(
            &mut store,
            GlobalType::new(ValType::EXTERNREF, Mutability::Const),
            val,
        )?;
        linker.define(&mut store, "env", &format!("s{}", id), global)?;
    }

    // Instantiate module
    let instance = linker.instantiate(&mut store, &module)?;

    // Identify AST types from parsed structs
    let mut node_struct_name = None;
    let mut field_struct_name = None;
    for s in parsed_structs.iter() {
        if s.name == "StructDef" || s.name.ends_with("::StructDef") {
            node_struct_name = Some(s.name.clone());
        }
        if s.name == "StructFieldDef" || s.name.ends_with("::StructFieldDef") {
            field_struct_name = Some(s.name.clone());
        }
    }

    // Find AST memory allocation exports
    let alloc_node = node_struct_name
        .as_ref()
        .and_then(|name| find_export(&instance, &mut store, &sanitize(name)));
    let alloc_field = field_struct_name
        .as_ref()
        .and_then(|name| find_export(&instance, &mut store, &sanitize(name)));
    let alloc_array = field_struct_name
        .as_ref()
        .and_then(|name| find_export(&instance, &mut store, &format!("array_{}", sanitize(name))));
    let set_array = field_struct_name.as_ref().and_then(|name| {
        find_export(
            &instance,
            &mut store,
            &format!("set_array_{}", sanitize(name)),
        )
    });

    let mut output_lines = Vec::new();

    // Iterate and run macros
    for struct_def in parsed_structs.iter() {
        for attr in &struct_def.attributes {
            let macro_func = instance
                .get_func(&mut store, &attr.name)
                .or_else(|| {
                    find_export(&instance, &mut store, &format!("_{}", sanitize(&attr.name)))
                })
                .or_else(|| find_export(&instance, &mut store, &sanitize(&attr.name)));

            if let Some(func) = macro_func {
                let node_ptr = if let Some(ref alloc_node_fn) = alloc_node {
                    let name_ref =
                        Val::ExternRef(Some(ExternRef::new(&mut store, struct_def.name.clone())?));
                    Some(call_func(alloc_node_fn, &mut store, &[name_ref])?)
                } else {
                    None
                };

                let fields_arr = if let (
                    Some(alloc_array_fn),
                    Some(alloc_field_fn),
                    Some(set_array_fn),
                ) = (
                    alloc_array.as_ref(),
                    alloc_field.as_ref(),
                    set_array.as_ref(),
                ) {
                    let len_val = Val::I32(struct_def.fields.len() as i32);
                    let arr_val = call_func(alloc_array_fn, &mut store, &[len_val])?;

                    for (i, field) in struct_def.fields.iter().enumerate() {
                        let mut rename_to = String::new();
                        for field_attr in &field.attributes {
                            if field_attr.name == "Rename" && !field_attr.args.is_empty() {
                                rename_to = field_attr.args[0].clone();
                                break;
                            }
                        }

                        let name_ref =
                            Val::ExternRef(Some(ExternRef::new(&mut store, field.name.clone())?));
                        let ty_ref =
                            Val::ExternRef(Some(ExternRef::new(&mut store, field.ty.to_string())?));
                        let rename_ref =
                            Val::ExternRef(Some(ExternRef::new(&mut store, rename_to)?));

                        let field_ptr =
                            call_func(alloc_field_fn, &mut store, &[name_ref, ty_ref, rename_ref])?;

                        let idx_val = Val::I32(i as i32);
                        func_call_void(
                            set_array_fn,
                            &mut store,
                            &[arr_val.clone(), idx_val, field_ptr],
                        )?;
                    }
                    Some(arr_val)
                } else {
                    None
                };

                let param_count = func.ty(&store).params().len();
                let result_val = match (&node_ptr, &fields_arr) {
                    (Some(np), Some(fa)) if param_count >= 2 => {
                        call_func(&func, &mut store, &[np.clone(), fa.clone()])?
                    }
                    (Some(np), _) if param_count >= 1 => {
                        call_func(&func, &mut store, &[np.clone()])?
                    }
                    _ => call_func(&func, &mut store, &[])?,
                };

                if let Val::ExternRef(Some(ext_ref)) = result_val {
                    if let Some(data) = ext_ref.data(&store)? {
                        if let Some(s) = data.downcast_ref::<String>() {
                            output_lines.push(s.clone());
                        }
                    }
                }
            }
        }
    }

    let generated_code = output_lines.join("\n");
    if generated_code.trim().is_empty() {
        return Ok(false);
    }

    // Parse the generated code
    let lexer = crate::lexer::Lexer::new(&generated_code);
    let mut parser = crate::parser::Parser::new(lexer);
    let items = parser.parse_module();

    let mut added_anything = false;
    for item in items {
        match item {
            Item::Struct(s, span) => {
                parsed_structs.push(s);
                if let Some(s) = parsed_structs.last() {
                    crate::ast::register_span(s, span);
                }
                added_anything = true;
            }
            Item::Function(f, span) => {
                parsed_funcs.push(f);
                if let Some(f) = parsed_funcs.last() {
                    crate::ast::register_span(f, span);
                }
                added_anything = true;
            }
            Item::Impl(i, span) => {
                for f in i.methods.clone() {
                    parsed_funcs.push(f);
                }
                parsed_impls.push(i);
                if let Some(i) = parsed_impls.last() {
                    crate::ast::register_span(i, span);
                }
                added_anything = true;
            }
            Item::Const(c, span) => {
                parsed_consts.push(c);
                if let Some(c) = parsed_consts.last() {
                    crate::ast::register_span(c, span);
                }
                added_anything = true;
            }
            _ => {}
        }
    }

    Ok(added_anything)
}

pub fn run_macros(
    parsed_structs: &mut Vec<StructDef>,
    parsed_funcs: &mut Vec<Function>,
    parsed_impls: &mut Vec<ImplDef>,
    parsed_consts: &mut Vec<ConstDef>,
    imports_registry: &HashMap<String, HashSet<String>>,
) -> bool {
    match run_macros_inner(
        parsed_structs,
        parsed_funcs,
        parsed_impls,
        parsed_consts,
        imports_registry,
    ) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Macro execution failed: {:?}", e);
            false
        }
    }
}
