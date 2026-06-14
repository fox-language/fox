/**
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

    const stdEnv = {
        f_dn: (r) => r == null || r === undefined,
        f_dns: (s) => s == null || s === undefined,
        f_dac: (parent, child) => parent.appendChild(child),
        f_dsa: (el, name, value) => el.setAttribute(name, value),
        f_dga: (el, name) => el.getAttribute(name),
        f_dra: (el, name) => el.removeAttribute(name),
        f_dst: (el, text) => el.textContent = text,
        f_dgt: (el) => el.textContent,
        f_dcl: (el, handler) => {
            el.addEventListener('click', () => {
                if (rawExports && rawExports.fox_run_task) {
                    if (WebAssembly.promising) {
                        WebAssembly.promising(rawExports.fox_run_task)(handler).catch(err => console.error('Error in click listener:', err));
                    } else {
                        rawExports.fox_run_task(handler);
                    }
                }
            });
        },
        f_dqs: (selector) => document.querySelector(selector),
        f_dce: (tag) => document.createElement(tag),
        f_con: (level, msg) => {
            switch (level) {
                case 1: console.log(msg); break;
                case 2: console.info(msg); break;
                case 3: console.warn(msg); break;
                case 4: console.error(msg); break;
                case 5: console.debug(msg); break;
                default: console.log(msg);
            }
        },
        f_dpn: () => performance.now(),
        f_ay: WebAssembly.Suspending ? new WebAssembly.Suspending(() => new Promise(resolve => queueMicrotask(resolve))) : () => {},
        f_as: WebAssembly.Suspending ? new WebAssembly.Suspending((ms) => new Promise(resolve => setTimeout(resolve, ms))) : (ms) => {},
        f_qt: (task) => {
            if (rawExports.fox_run_task) {
                if (WebAssembly.promising) {
                    queueMicrotask(() => WebAssembly.promising(rawExports.fox_run_task)(task));
                } else {
                    rawExports.fox_run_task(task);
                }
            }
        },
        s0: "Count changed to %d",
        s1: "Count: %d",
        s2: "Error: body element not found!",
        s3: "Increment",
        s4: "body",
        s5: "button",
        s6: "cleanup!",
        s7: "div",
    };

    const variadicFuncs = {
        'sprintf_sprintf': 1,
    };

    const imports = { env: { ...stdEnv, ...envImports } };
    const instance = await WebAssembly.instantiate(module, imports);
    const rawExports = instance.exports;
    const wrappedExports = {};
    const structsMeta = [
  { name: 'console::Console', alloc: 'fox_alloc_console_Console', fields: [] },
  { name: 'option::Option_dom_Element', alloc: 'fox_alloc_option_Option_dom_Element', fields: ['_tag', 'Some_0'] },
  { name: 'option::Option_fn__:void', alloc: 'fox_alloc_option_Option_fn___void', fields: ['_tag', 'Some_0'] },
  { name: 'option::Option_i32', alloc: 'fox_alloc_option_Option_i32', fields: ['_tag', 'Some_0'] },
  { name: 'option::Option_i64', alloc: 'fox_alloc_option_Option_i64', fields: ['_tag', 'Some_0'] },
  { name: 'option::Option_signals_Effect', alloc: 'fox_alloc_option_Option_signals_Effect', fields: ['_tag', 'Some_0'] },
  { name: 'option::Option_str', alloc: 'fox_alloc_option_Option_str', fields: ['_tag', 'Some_0'] },
  { name: 'fnv1a::Hasher32', alloc: 'fox_alloc_fnv1a_Hasher32', fields: ['state'] },
  { name: 'set::Set_i32', alloc: 'fox_alloc_set_Set_i32', fields: ['keys', 'states', 'capacity', 'mask', 'size', 'hasher'] },
  { name: 'set::Set_i64', alloc: 'fox_alloc_set_Set_i64', fields: ['keys', 'states', 'capacity', 'mask', 'size', 'hasher'] },
  { name: 'set::Set_str', alloc: 'fox_alloc_set_Set_str', fields: ['keys', 'states', 'capacity', 'mask', 'size', 'hasher'] },
  { name: 'set::Set_signals_Effect', alloc: 'fox_alloc_set_Set_signals_Effect', fields: ['keys', 'states', 'capacity', 'mask', 'size', 'hasher'] },
  { name: 'set::SetIterator_signals_Effect', alloc: 'fox_alloc_set_SetIterator_signals_Effect', fields: ['set', 'index'] },
  { name: 'vec::Vec_fn__:void', alloc: 'fox_alloc_vec_Vec_fn___void', fields: ['data', 'len', 'cap'] },
  { name: 'vec::Vec_i32', alloc: 'fox_alloc_vec_Vec_i32', fields: ['data', 'len', 'cap'] },
  { name: 'vec::Vec_i64', alloc: 'fox_alloc_vec_Vec_i64', fields: ['data', 'len', 'cap'] },
  { name: 'vec::Vec_signals_Effect', alloc: 'fox_alloc_vec_Vec_signals_Effect', fields: ['data', 'len', 'cap'] },
  { name: 'vec::Vec_str', alloc: 'fox_alloc_vec_Vec_str', fields: ['data', 'len', 'cap'] },
  { name: 'dom::Element', alloc: 'fox_alloc_dom_Element', fields: ['_ref'] },
  { name: 'dom::Document', alloc: 'fox_alloc_dom_Document', fields: [] },
  { name: 'signals::Signal_i32', alloc: 'fox_alloc_signals_Signal_i32', fields: ['value', 'subs'] },
  { name: 'signals::Effect', alloc: 'fox_alloc_signals_Effect', fields: ['id', 'run', 'teardowns'] },
  { name: '__closure_env_1', alloc: 'fox_alloc___closure_env_1', fields: [] },
  { name: '__closure_env_2', alloc: 'fox_alloc___closure_env_2', fields: ['count', 'display'] },
  { name: '__closure_env_3', alloc: 'fox_alloc___closure_env_3', fields: [] },
  { name: '__closure_env_4', alloc: 'fox_alloc___closure_env_4', fields: ['count'] },
];


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
