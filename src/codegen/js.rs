use crate::ast::StructDef;
use crate::codegen::sanitize_wat_name;
use std::collections::HashMap;

pub fn generate_js_bindings(
    structs: &[StructDef],
    string_literals: &[String],
    variadic_funcs: &HashMap<String, usize>,
    wat: Option<&str>,
) -> String {
    let mut struct_meta = String::from("const structsMeta = [\n");
    for s in structs {
        struct_meta.push_str(&format!(
            "  {{ name: '{}', alloc: '{}', fields: [{}] }},\n",
            s.name,
            format!("fox_alloc_{}", sanitize_wat_name(&s.name)),
            s.fields
                .iter()
                .map(|f| format!("'{}'", f.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    struct_meta.push_str("];\n");

    let helpers = [
        (
            "__fox_panic",
            "        __fox_panic: (v) => { throw new Error('Fox Panic: ' + v); },\n",
        ),
        (
            "__fox_str_starts_with",
            "        __fox_str_starts_with: (s, prefix) => (s != null && prefix != null && s.startsWith(prefix)) ? 1 : 0,\n",
        ),
        (
            "__fox_str_ends_with",
            "        __fox_str_ends_with: (s, suffix) => (s != null && suffix != null && s.endsWith(suffix)) ? 1 : 0,\n",
        ),
        (
            "__fox_str_contains",
            "        __fox_str_contains: (s, sub) => (s != null && sub != null && s.includes(sub)) ? 1 : 0,\n",
        ),
        (
            "__fox_str_index_of",
            "        __fox_str_index_of: (s, sub) => (s != null && sub != null) ? s.indexOf(sub) : -1,\n",
        ),
        (
            "__fox_str_last_index_of",
            "        __fox_str_last_index_of: (s, sub) => (s != null && sub != null) ? s.lastIndexOf(sub) : -1,\n",
        ),
        (
            "__fox_str_is_empty",
            "        __fox_str_is_empty: (s) => (s == null || s.length === 0) ? 1 : 0,\n",
        ),
        (
            "__fox_str_eq",
            "        __fox_str_eq: (a, b) => (a === b) ? 1 : 0,\n",
        ),
        (
            "__fox_str_join",
            "        __fox_str_join: (a, b) => (a == null ? '' : a) + (b == null ? '' : b),\n",
        ),
        (
            "__fox_str_compare",
            "        __fox_str_compare: (a, b) => {\n            if (a == null && b == null) return 0;\n            if (a == null) return -1;\n            if (b == null) return 1;\n            if (a < b) return -1;\n            if (a > b) return 1;\n            return 0;\n        },\n",
        ),
        (
            "__fox_dom_is_null",
            "        __fox_dom_is_null: (r) => r == null || r === undefined,\n",
        ),
        (
            "__fox_dom_is_null_str",
            "        __fox_dom_is_null_str: (s) => s == null || s === undefined,\n",
        ),
        (
            "__fox_dom_element_append_child",
            "        __fox_dom_element_append_child: (parent, child) => parent.appendChild(child),\n",
        ),
        (
            "__fox_dom_element_set_attribute",
            "        __fox_dom_element_set_attribute: (el, name, value) => el.setAttribute(name, value),\n",
        ),
        (
            "__fox_dom_element_get_attribute",
            "        __fox_dom_element_get_attribute: (el, name) => el.getAttribute(name),\n",
        ),
        (
            "__fox_dom_element_remove_attribute",
            "        __fox_dom_element_remove_attribute: (el, name) => el.removeAttribute(name),\n",
        ),
        (
            "__fox_dom_element_set_text_content",
            "        __fox_dom_element_set_text_content: (el, text) => el.textContent = text,\n",
        ),
        (
            "__fox_dom_element_get_text_content",
            "        __fox_dom_element_get_text_content: (el) => el.textContent,\n",
        ),
        (
            "__fox_dom_element_set_inner_html",
            "        __fox_dom_element_set_inner_html: (el, html) => el.innerHTML = html,\n",
        ),
        (
            "__fox_dom_element_add_click_listener",
            "        __fox_dom_element_add_click_listener: (el, handler) => {\n            el.addEventListener('click', () => {\n                if (rawExports && rawExports.fox_run_task) {\n                    if (WebAssembly.promising) {\n                        WebAssembly.promising(rawExports.fox_run_task)(handler).catch(err => console.error('Error in click listener:', err));\n                    } else {\n                        rawExports.fox_run_task(handler);\n                    }\n                }\n            });\n        },\n",
        ),
        (
            "__fox_dom_document_query_selector",
            "        __fox_dom_document_query_selector: (selector) => document.querySelector(selector),\n",
        ),
        (
            "__fox_dom_document_create_element",
            "        __fox_dom_document_create_element: (tag) => document.createElement(tag),\n",
        ),
        (
            "__fox_dom_console",
            "        __fox_dom_console: (level, msg) => {\n            switch (level) {\n                case 1: console.log(msg); break;\n                case 2: console.info(msg); break;\n                case 3: console.warn(msg); break;\n                case 4: console.error(msg); break;\n                case 5: console.debug(msg); break;\n                default: console.log(msg);\n            }\n        },\n",
        ),
        (
            "__fox_dom_performance_now",
            "        __fox_dom_performance_now: () => performance.now(),\n",
        ),
        (
            "__fox_time_now",
            "        __fox_time_now: () => BigInt(Date.now()),\n",
        ),
        (
            "__fox_time_local_offset",
            "        __fox_time_local_offset: () => new Date().getTimezoneOffset(),\n",
        ),
        (
            "__fox_f64_to_str",
            "        __fox_f64_to_str: (v) => String(v),\n",
        ),
        (
            "__fox_i32_to_str",
            "        __fox_i32_to_str: (v) => String(v),\n",
        ),
        (
            "__fox_i64_to_str",
            "        __fox_i64_to_str: (v) => String(v),\n",
        ),
        (
            "__fox_json_parse_int",
            "        __fox_json_parse_int: (s) => parseInt(s, 10),\n",
        ),
        (
            "__fox_json_parse_float",
            "        __fox_json_parse_float: (s) => parseFloat(s),\n",
        ),
        (
            "__fox_json_parse_string",
            "        __fox_json_parse_string: (s) => JSON.parse(s),\n",
        ),
        (
            "__fox_json_encode_string",
            "        __fox_json_encode_string: (s) => JSON.stringify(s),\n",
        ),
        (
            "__fox_str_substring",
            "        __fox_str_substring: (s, start, end) => s.substring(start, end),\n",
        ),
        (
            "__fox_http_send",
            "        __fox_http_send: WebAssembly.Suspending ? new WebAssembly.Suspending(async (url, method, headersRaw, body) => {\n            try {\n                const headers = {};\n                if (headersRaw) {\n                    const lines = headersRaw.split('\\n');\n                    for (const line of lines) {\n                        if (!line) continue;\n                        const idx = line.indexOf(':');\n                        if (idx !== -1) {\n                            const key = line.slice(0, idx).trim();\n                            const val = line.slice(idx + 1).trim();\n                            headers[key] = val;\n                        }\n                    }\n                }\n                const options = {\n                    method: method || 'GET',\n                    headers: headers\n                };\n                if (method !== 'GET' && method !== 'HEAD' && body !== undefined && body !== null) {\n                    options.body = body;\n                }\n                const res = await fetch(url, options);\n                const resBody = await res.text();\n                return {\n                    status: res.status,\n                    statusText: res.statusText,\n                    body: resBody\n                };\n            } catch (err) {\n                return {\n                    status: 0,\n                    statusText: err.message || String(err),\n                    body: ''\n                };\n            }\n        }) : () => ({ status: 0, statusText: 'No Suspending support', body: '' }),\n",
        ),
        (
            "__fox_http_get_status",
            "        __fox_http_get_status: (r) => r ? r.status : 0,\n",
        ),
        (
            "__fox_http_get_status_text",
            "        __fox_http_get_status_text: (r) => r ? r.statusText : '',\n",
        ),
        (
            "__fox_http_get_body",
            "        __fox_http_get_body: (r) => r ? r.body : '',\n",
        ),
        (
            "__fox_async_yield",
            "        __fox_async_yield: WebAssembly.Suspending ? new WebAssembly.Suspending(() => new Promise(resolve => queueMicrotask(resolve))) : () => {},\n",
        ),
        (
            "__fox_async_sleep",
            "        __fox_async_sleep: WebAssembly.Suspending ? new WebAssembly.Suspending((ms) => new Promise(resolve => setTimeout(resolve, ms))) : (ms) => {},\n",
        ),
        (
            "__fox_queue_task",
            "        __fox_queue_task: (task) => {\n            if (rawExports.fox_run_task) {\n                if (WebAssembly.promising) {\n                    queueMicrotask(() => WebAssembly.promising(rawExports.fox_run_task)(task));\n                } else {\n                    rawExports.fox_run_task(task);\n                }\n            }\n        },\n",
        ),
    ];

    let mut std_env = String::from("    const stdEnv = {\n");
    for (name, definition) in &helpers {
        let is_used = match wat {
            Some(wat_str) => {
                let short = crate::codegen::shorten_import_name(name);
                let pattern1 = format!("(import \"env\" \"{}\"", name);
                let pattern2 = format!("(import \"env\" \"{}\"", short);
                wat_str.contains(&pattern1) || wat_str.contains(&pattern2)
            }
            None => true,
        };

        if is_used {
            let short = crate::codegen::shorten_import_name(name);
            let shortened_def = definition.replace(&format!("{}:", name), &format!("{}:", short));
            std_env.push_str(&shortened_def);
        }
    }

    for (id, lit) in string_literals.iter().enumerate() {
        std_env.push_str(&format!(
            "        s{}: {},\n",
            id,
            serde_json::to_string(lit).unwrap_or_else(|_| "\"\"".to_string())
        ));
    }

    std_env.push_str("    };\n");

    let mut variadic_meta = String::from("    const variadicFuncs = {\n");
    for (name, arity) in variadic_funcs {
        variadic_meta.push_str(&format!("        '{}': {},\n", name, arity));
    }
    variadic_meta.push_str("    };\n");

    let template = r#"/**
 * Fox Companion Library
 * Handles Wasm GC allocation, JS-String builtins, and ergonomic proxying.
 */
export async function fox(wasmUrlOrBuffer, envImports = {}) {
    let module;
    const compileOptions = { builtins: ['js-string'] };
    if (typeof wasmUrlOrBuffer === 'string') {
        const response = await fetch(wasmUrlOrBuffer);
        module = await WebAssembly.compileStreaming(response, compileOptions);
    } else {
        module = await WebAssembly.compile(wasmUrlOrBuffer, compileOptions);
    }

// <INSERT_STDENV>
// <INSERT_VARR_META>
    const imports = { env: { ...stdEnv, ...envImports } };
    const instance = await WebAssembly.instantiate(module, imports);
    const rawExports = instance.exports;
    const wrappedExports = {};
    // <INSERT_META>

    const mapArg = (val) => {
        if (typeof val === 'object' && val !== null && !(val instanceof Object)) {
            return val;
        }
        if (Array.isArray(val)) {
            const wasmArr = rawExports.fox_alloc_array_f32(val.length);
            for (let i = 0; i < val.length; i++) {
                rawExports.fox_set_array_f32(wasmArr, i, mapArg(val[i]));
            }
            return wasmArr;
        }
        if (typeof val === 'object' && val !== null) {
            const keys = Object.keys(val);
            const meta = structsMeta.find(m => 
                m.fields.length === keys.length && m.fields.every(f => keys.includes(f))
            );
            if (meta) {
                const allocArgs = meta.fields.map(f => mapArg(val[f]));
                return rawExports[meta.alloc](...allocArgs);
            }
        }
        return val;
    };

    const isAsyncSupport = !!WebAssembly.promising;
    for (const key in rawExports) {
        if (typeof rawExports[key] === 'function' && !key.startsWith('fox_')) {
            const fixedArity = variadicFuncs[key];
            const rawFunc = rawExports[key];
            const promisingFunc = isAsyncSupport ? WebAssembly.promising(rawFunc) : rawFunc;
            let wrapped;
            if (fixedArity !== undefined) {
                wrapped = (...args) => {
                    const fixedArgs = args.slice(0, fixedArity).map(mapArg);
                    const variadicArgs = args.slice(fixedArity).map(mapArg);
                    return rawFunc(...fixedArgs, variadicArgs);
                };
                wrapped.promising = (...args) => {
                    const fixedArgs = args.slice(0, fixedArity).map(mapArg);
                    const variadicArgs = args.slice(fixedArity).map(mapArg);
                    return promisingFunc(...fixedArgs, variadicArgs);
                };
            } else {
                wrapped = (...args) => {
                    const mappedArgs = args.map(mapArg);
                    return rawFunc(...mappedArgs);
                };
                wrapped.promising = (...args) => {
                    const mappedArgs = args.map(mapArg);
                    return promisingFunc(...mappedArgs);
                };
            }
            wrappedExports[key] = wrapped;
        } else {
            wrappedExports[key] = rawExports[key];
        }
    }

    return { module, instance, exports: wrappedExports };
}
"#;

    template
        .replace("// <INSERT_META>", &struct_meta)
        .replace("// <INSERT_STDENV>", &std_env)
        .replace("// <INSERT_VARR_META>", &variadic_meta)
}
