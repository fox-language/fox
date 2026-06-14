/// Description of an inherent method on a builtin type (`str`, `f64`, `f32`,
/// `i32`, `i64`, `[]T`) declared in `std::builtin`. The body is provided
/// either by emitting a single Wasm opcode or by calling a Wasm import.
pub struct BuiltinIntrinsic {
    /// The Wasm function name to call (with leading `$`), e.g. `"$fox_str_len"`.
    /// `None` for `Opcode` variants (no call is emitted).
    pub wasm_fn: Option<&'static str>,
    /// The Wasm import module (`"env"` or `"wasm:js-string"`).
    /// `None` for `Opcode` variants.
    pub module: Option<&'static str>,
    /// The Wasm import name within the module.
    /// `None` for `Opcode` variants.
    pub import_name: Option<&'static str>,
    /// Wasm types of the explicit parameters (after `self`). Empty for `Opcode`
    /// variants that take no extra args (e.g. `f64.abs`).
    pub param_wasm_tys: &'static [&'static str],
    /// The Wasm result type (`"i32"` for bool/i32 returns, `"externref"` for str,
    /// or the opcode itself for `Opcode` variants — the caller picks).
    pub result_wasm: &'static str,
    /// The Fox type of the return value (`"i32"`, `"bool"`, `"str"`, `"[]byte"`).
    pub return_ty: &'static str,
    /// `true` if the method is implemented by a hand-written Wasm helper
    /// emitted alongside the module (currently only `str::bytes`).
    pub uses_wasm_helper: bool,
    /// `true` if `result_wasm` is actually a Wasm opcode to emit (e.g. `"f64.abs"`)
    /// rather than a Wasm type tag. When `is_opcode` is set, the codegen pushes
    /// self + args, then emits `result_wasm` as a single opcode.
    pub is_opcode: bool,
}

pub fn lookup_builtin_intrinsic(parent_ty: &str, method: &str) -> Option<BuiltinIntrinsic> {
    if parent_ty == "str" {
        return lookup_str_intrinsic(method);
    }
    if parent_ty == "f64" || parent_ty == "f32" {
        return lookup_float_intrinsic(parent_ty, method);
    }
    if parent_ty == "i32" || parent_ty == "i64" {
        return lookup_int_intrinsic(parent_ty, method);
    }
    if parent_ty.starts_with("[]") {
        return lookup_array_intrinsic(method);
    }
    None
}

fn lookup_str_intrinsic(method: &str) -> Option<BuiltinIntrinsic> {
    let imp = |wasm_fn, module, import_name, param_wasm_tys, result_wasm, return_ty| BuiltinIntrinsic {
        wasm_fn: Some(wasm_fn),
        module: Some(module),
        import_name: Some(import_name),
        param_wasm_tys,
        result_wasm,
        return_ty,
        uses_wasm_helper: false,
        is_opcode: false,
    };
    match method {
        "len" => Some(imp("$fox_str_len", "wasm:js-string", "length", &[], "i32", "i32")),
        "char_at" => Some(imp("$fox_str_char_at", "wasm:js-string", "charCodeAt", &["i32"], "i32", "byte")),
        "starts_with" => Some(imp("$fox_str_starts_with", "env", "__fox_str_starts_with", &["externref"], "i32", "bool")),
        "ends_with" => Some(imp("$fox_str_ends_with", "env", "__fox_str_ends_with", &["externref"], "i32", "bool")),
        "contains" => Some(imp("$fox_str_contains", "env", "__fox_str_contains", &["externref"], "i32", "bool")),
        "index_of" => Some(imp("$fox_str_index_of", "env", "__fox_str_index_of", &["externref"], "i32", "i32")),
        "last_index_of" => Some(imp("$fox_str_last_index_of", "env", "__fox_str_last_index_of", &["externref"], "i32", "i32")),
        "is_empty" => Some(imp("$fox_str_is_empty", "env", "__fox_str_is_empty", &[], "i32", "bool")),
        "eq" => Some(imp("$fox_str_eq", "env", "__fox_str_eq", &["externref"], "i32", "bool")),
        "join" => Some(imp("$fox_str_join", "env", "__fox_str_join", &["externref"], "externref", "str")),
        "compare" => Some(imp("$fox_str_compare", "env", "__fox_str_compare", &["externref"], "i32", "i32")),
        "substring" => Some(imp("$fox_str_substring", "env", "__fox_str_substring", &["i32", "i32"], "externref", "str")),
        "bytes" => Some(BuiltinIntrinsic {
            wasm_fn: Some("$fox_str_bytes"),
            module: None,
            import_name: None,
            param_wasm_tys: &[],
            result_wasm: "(ref null $array_byte)",
            return_ty: "[]byte",
            uses_wasm_helper: true,
            is_opcode: false,
        }),
        _ => None,
    }
}

fn lookup_float_intrinsic(prefix: &str, method: &str) -> Option<BuiltinIntrinsic> {
    let prefix_static: &'static str = Box::leak(prefix.to_string().into_boxed_str());
    let opcode = match method {
        "abs" => format!("{}.abs", prefix),
        "sqrt" => format!("{}.sqrt", prefix),
        "ceil" => format!("{}.ceil", prefix),
        "floor" => format!("{}.floor", prefix),
        "trunc" => format!("{}.trunc", prefix),
        "min" => format!("{}.min", prefix),
        "max" => format!("{}.max", prefix),
        _ => return None,
    };
    let has_arg = matches!(method, "min" | "max");
    let param_wasm_tys: &'static [&'static str] = if has_arg {
        Box::leak(
            vec![prefix_static].into_boxed_slice() as Box<[&'static str]>
        ) as &'static [&'static str]
    } else {
        &[]
    };
    let opcode_static: &'static str = Box::leak(opcode.into_boxed_str());
    Some(BuiltinIntrinsic {
        wasm_fn: None,
        module: None,
        import_name: None,
        param_wasm_tys,
        result_wasm: opcode_static,
        return_ty: prefix_static,
        uses_wasm_helper: false,
        is_opcode: true,
    })
}

fn lookup_int_intrinsic(prefix: &str, method: &str) -> Option<BuiltinIntrinsic> {
    let prefix_static: &'static str = Box::leak(prefix.to_string().into_boxed_str());
    let opcode = match method {
        "abs" => format!("{}.abs", prefix),
        "min" => format!("{}.min", prefix),
        "max" => format!("{}.max", prefix),
        _ => return None,
    };
    let has_arg = matches!(method, "min" | "max");
    let param_wasm_tys: &'static [&'static str] = if has_arg {
        Box::leak(
            vec![prefix_static].into_boxed_slice() as Box<[&'static str]>
        ) as &'static [&'static str]
    } else {
        &[]
    };
    let opcode_static: &'static str = Box::leak(opcode.into_boxed_str());
    Some(BuiltinIntrinsic {
        wasm_fn: None,
        module: None,
        import_name: None,
        param_wasm_tys,
        result_wasm: opcode_static,
        return_ty: prefix_static,
        uses_wasm_helper: false,
        is_opcode: true,
    })
}

fn lookup_array_intrinsic(method: &str) -> Option<BuiltinIntrinsic> {
    match method {
        "len" => Some(BuiltinIntrinsic {
            wasm_fn: None,
            module: None,
            import_name: None,
            param_wasm_tys: &[],
            result_wasm: "array.len",
            return_ty: "i32",
            uses_wasm_helper: false,
            is_opcode: true,
        }),
        "copy_from" => Some(BuiltinIntrinsic {
            wasm_fn: None,
            module: None,
            import_name: None,
            param_wasm_tys: &["i32", "unknown", "i32", "i32"],
            result_wasm: "void",
            return_ty: "void",
            uses_wasm_helper: false,
            is_opcode: true,
        }),
        _ => None,
    }
}

pub fn emit_int_to_str_wasm() -> String {
    r#"    (local $len i32)
    (local $temp i32)
    (local $val i32)
    (local $char i32)
    (local $str_res externref)

    (ref.null extern)
    local.set $str_res

    local.get $v
    local.set $val

    ;; Special case: 0
    (if (i32.eq (local.get $val) (i32.const 0))
      (then
        (return (call $fox_fromCharCode (i32.const 48)))
      )
    )

    ;; Handle negative sign
    (if (i32.lt_s (local.get $val) (i32.const 0))
      (then
        (local.set $str_res (call $fox_fromCharCode (i32.const 45)))
        ;; negate
        (local.set $val (i32.sub (i32.const 0) (local.get $val)))
      )
    )

    ;; Count digits
    (i32.const 0)
    local.set $len
    local.get $val
    local.set $temp

    (block $digit_count_done
      (loop $digit_count_loop
        (br_if $digit_count_done (i32.eqz (local.get $temp)))
        (local.set $len (i32.add (local.get $len) (i32.const 1)))
        (local.set $temp (i32.div_u (local.get $temp) (i32.const 10)))
        br $digit_count_loop
      )
    )

    ;; Generate chars backward and prepend/concat
    (block $done
      (loop $loop
        (br_if $done (i32.eqz (local.get $val)))

        ;; char = 48 + (val % 10)
        (local.set $char
          (i32.add
            (i32.const 48)
            (i32.rem_u (local.get $val) (i32.const 10))
          )
        )

        ;; prepend
        (if (ref.is_null (local.get $str_res))
          (then
            (local.set $str_res (call $fox_fromCharCode (local.get $char)))
          )
          (else
            (local.set $str_res
              (call $fox_js_string_concat
                (call $fox_fromCharCode (local.get $char))
                (local.get $str_res)
              )
            )
          )
        )

        (local.set $val (i32.div_u (local.get $val) (i32.const 10)))
        br $loop
      )
    )

    local.get $str_res
"#
    .to_string()
}

pub fn emit_str4_wasm() -> String {
    r#"    (local $res externref)
    (call $fox_fromCharCode (local.get $c1))
    local.set $res

    local.get $res
    (call $fox_fromCharCode (local.get $c2))
    call $fox_js_string_concat
    local.set $res

    local.get $res
    (call $fox_fromCharCode (local.get $c3))
    call $fox_js_string_concat
    local.set $res

    local.get $res
    (call $fox_fromCharCode (local.get $c4))
    call $fox_js_string_concat
    local.set $res

    local.get $res
"#
    .to_string()
}

pub fn emit_fmt_runtime_helpers() -> String {
    r#"  (import "wasm:js-string" "fromCharCode" (func $fox_fromCharCode (param i32) (result externref)))
  (import "wasm:js-string" "concat" (func $fox_js_string_concat (param externref externref) (result externref)))
  (import "wasm:js-string" "length" (func $fox_js_string_length (param externref) (result i32)))
  (import "wasm:js-string" "charCodeAt" (func $fox_js_string_char_code_at (param externref i32) (result i32)))
"#
    .to_string()
}

pub fn emit_fmt_runtime_helper_funcs() -> String {
    let mut helpers = String::new();
    helpers.push_str("  (func $fox_int_to_str (param $v i32) (result externref)\n");
    helpers.push_str(&emit_int_to_str_wasm());
    helpers.push_str("  )\n\n");
    helpers.push_str("  (func $fox_str4 (param $c1 i32) (param $c2 i32) (param $c3 i32) (param $c4 i32) (result externref)\n");
    helpers.push_str(&emit_str4_wasm());
    helpers.push_str("  )\n\n");
    helpers
}

pub fn emit_sprintf_body() -> String {
    r#"    (local $result externref)
    (local $i i32)
    (local $arg_idx i32)
    (local $c i32)
    (local $arg (ref null any))
    (local $n_args i32)
    (local $fmt_len i32)
    (local $char_str externref)

    (ref.null extern)
    local.set $result
    (i32.const 0)
    local.set $i
    (i32.const 0)
    local.set $arg_idx

    local.get $args
    array.len
    local.set $n_args

    local.get $fmt
    call $fox_js_string_length
    local.set $fmt_len

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $fmt_len)))

        local.get $fmt
        local.get $i
        call $fox_js_string_char_code_at
        local.set $c

        (if (i32.eq (local.get $c) (i32.const 37))
          (then
            ;; '%': advance past it, look at the spec char
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br_if $done (i32.ge_u (local.get $i) (local.get $fmt_len)))
          local.get $fmt
          local.get $i
          call $fox_js_string_char_code_at
          local.set $c

          (block $spec_handled
            ;; %%
            (if (i32.eq (local.get $c) (i32.const 37))
              (then
                (local.set $char_str (call $fox_fromCharCode (i32.const 37)))
                br $spec_handled
              )
            )
            ;; %s
            (if (i32.eq (local.get $c) (i32.const 115))
              (then
                (block $s_done
                  (br_if $s_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (local.set $char_str (call $fox_fromCharCode (i32.const 63)))
                    )
                    (else
                      (local.set $char_str (extern.convert_any (local.get $arg)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            ;; %d / %i
            (if (i32.eq (local.get $c) (i32.const 100))
              (then
                (block $d_done
                  (br_if $d_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (local.set $char_str
                        (call $fox_int_to_str
                          (i31.get_s (ref.cast (ref i31) (local.get $arg)))
                        )
                      )
                    )
                    (else
                      (local.set $char_str (call $fox_fromCharCode (i32.const 63)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            (if (i32.eq (local.get $c) (i32.const 105))
              (then
                (block $i_done
                  (br_if $i_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (local.set $char_str
                        (call $fox_int_to_str
                          (i31.get_s (ref.cast (ref i31) (local.get $arg)))
                        )
                      )
                    )
                    (else
                      (local.set $char_str (call $fox_fromCharCode (i32.const 63)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            ;; %b
            (if (i32.eq (local.get $c) (i32.const 98))
              (then
                (block $b_done
                  (br_if $b_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (if (i31.get_s (ref.cast (ref i31) (local.get $arg)))
                        (then
                          (local.set $char_str
                            (call $fox_str4
                              (i32.const 116) (i32.const 114)
                              (i32.const 117) (i32.const 101)
                            )
                          )
                        )
                        (else
                          (local.set $char_str
                            (call $fox_str4
                              (i32.const 102) (i32.const 97)
                              (i32.const 108) (i32.const 115)
                            )
                          )
                        )
                      )
                    )
                    (else
                      (local.set $char_str (call $fox_fromCharCode (i32.const 63)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            ;; %f (char code 102)
            (if (i32.eq (local.get $c) (i32.const 102))
              (then
                (block $f_done
                  (br_if $f_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (local.set $char_str
                        (call $fox_int_to_str
                          (i31.get_s (ref.cast (ref i31) (local.get $arg)))
                        )
                      )
                    )
                    (else
                      (local.set $char_str (extern.convert_any (local.get $arg)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            ;; %v
            (if (i32.eq (local.get $c) (i32.const 118))
              (then
                (block $v_done
                  (br_if $v_done (i32.ge_u (local.get $arg_idx) (local.get $n_args)))
                  (local.set $arg
                    (array.get $array_anyref (local.get $args) (local.get $arg_idx))
                  )
                  (local.set $arg_idx (i32.add (local.get $arg_idx) (i32.const 1)))
                  (if (ref.test (ref i31) (local.get $arg))
                    (then
                      (local.set $char_str
                        (call $fox_int_to_str
                          (i31.get_s (ref.cast (ref i31) (local.get $arg)))
                        )
                      )
                    )
                    (else
                      (local.set $char_str (extern.convert_any (local.get $arg)))
                    )
                  )
                )
                br $spec_handled
              )
            )
            ;; Unknown spec: emit '?'
            (local.set $char_str (call $fox_fromCharCode (i32.const 63)))
          )
          ;; spec_handled
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
        )
        (else
          ;; Literal char
          (local.set $char_str (call $fox_fromCharCode (local.get $c)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
        )
      )

      ;; Append char_str to result
      (if (ref.is_null (local.get $result))
        (then
          (local.set $result (local.get $char_str))
        )
        (else
          (local.set $result
            (call $fox_js_string_concat
              (local.get $result) (local.get $char_str)
            )
          )
        )
      )

      br $loop
      )
    )

    local.get $result
"#
    .to_string()
}
