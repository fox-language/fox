(module
  (import "env" "s0" (global $s0 externref))
  (import "env" "s1" (global $s1 externref))
  (import "env" "s2" (global $s2 externref))
  (import "env" "s3" (global $s3 externref))
  (import "env" "s4" (global $s4 externref))
  (import "env" "s5" (global $s5 externref))
  (import "env" "s6" (global $s6 externref))
  (import "env" "s7" (global $s7 externref))
  (rec
    (type $sig_fat_fn___void (func (param (ref null any))))
    (type $fat_fn___void (struct (field $func_ref (mut (ref null $sig_fat_fn___void))) (field $env (mut (ref null any)))))
    (type $sig_fat_fn___Option_fn___void (func (param (ref null any)) (result (ref null $option_Option_fn___void))))
    (type $fat_fn___Option_fn___void (struct (field $func_ref (mut (ref null $sig_fat_fn___Option_fn___void))) (field $env (mut (ref null any)))))
    (type $sig_fat_fn_i32__i32 (func (param i32) (param (ref null any)) (result i32)))
    (type $fat_fn_i32__i32 (struct (field $func_ref (mut (ref null $sig_fat_fn_i32__i32))) (field $env (mut (ref null any)))))
    (type $console_Console (struct ))
    (type $option_Option_dom_Element (struct (field $_tag (mut i32)) (field $Some_0 (mut (ref null $dom_Element))) ))
    (type $option_Option_fn___void (struct (field $_tag (mut i32)) (field $Some_0 (mut (ref null $fat_fn___void))) ))
    (type $option_Option_i32 (struct (field $_tag (mut i32)) (field $Some_0 (mut i32)) ))
    (type $option_Option_i64 (struct (field $_tag (mut i32)) (field $Some_0 (mut i64)) ))
    (type $option_Option_signals_Effect (struct (field $_tag (mut i32)) (field $Some_0 (mut (ref null $signals_Effect))) ))
    (type $option_Option_str (struct (field $_tag (mut i32)) (field $Some_0 (mut externref)) ))
    (type $fnv1a_Hasher32 (struct (field $state (mut i32)) ))
    (type $set_Set_i32 (struct (field $keys (mut (ref null $vec_Vec_i32))) (field $states (mut (ref null $vec_Vec_i32))) (field $capacity (mut i32)) (field $mask (mut i32)) (field $size (mut i32)) (field $hasher (mut (ref null $fnv1a_Hasher32))) ))
    (type $set_Set_i64 (struct (field $keys (mut (ref null $vec_Vec_i64))) (field $states (mut (ref null $vec_Vec_i32))) (field $capacity (mut i32)) (field $mask (mut i32)) (field $size (mut i32)) (field $hasher (mut (ref null $fnv1a_Hasher32))) ))
    (type $set_Set_str (struct (field $keys (mut (ref null $vec_Vec_str))) (field $states (mut (ref null $vec_Vec_i32))) (field $capacity (mut i32)) (field $mask (mut i32)) (field $size (mut i32)) (field $hasher (mut (ref null $fnv1a_Hasher32))) ))
    (type $set_Set_signals_Effect (struct (field $keys (mut (ref null $vec_Vec_signals_Effect))) (field $states (mut (ref null $vec_Vec_i32))) (field $capacity (mut i32)) (field $mask (mut i32)) (field $size (mut i32)) (field $hasher (mut (ref null $fnv1a_Hasher32))) ))
    (type $set_SetIterator_signals_Effect (struct (field $set (mut (ref null $set_Set_signals_Effect))) (field $index (mut i32)) ))
    (type $vec_Vec_fn___void (struct (field $data (mut (ref null $array_fn___void))) (field $len (mut i32)) (field $cap (mut i32)) ))
    (type $vec_Vec_i32 (struct (field $data (mut (ref null $array_i32))) (field $len (mut i32)) (field $cap (mut i32)) ))
    (type $vec_Vec_i64 (struct (field $data (mut (ref null $array_i64))) (field $len (mut i32)) (field $cap (mut i32)) ))
    (type $vec_Vec_signals_Effect (struct (field $data (mut (ref null $array_signals_Effect))) (field $len (mut i32)) (field $cap (mut i32)) ))
    (type $vec_Vec_str (struct (field $data (mut (ref null $array_str))) (field $len (mut i32)) (field $cap (mut i32)) ))
    (type $dom_Element (struct (field $_ref (mut externref)) ))
    (type $dom_Document (struct ))
    (type $signals_Signal_i32 (struct (field $value (mut i32)) (field $subs (mut (ref null $set_Set_signals_Effect))) ))
    (type $signals_Effect (struct (field $id (mut i64)) (field $run (mut (ref null $fat_fn___Option_fn___void))) (field $teardowns (mut (ref null $vec_Vec_fn___void))) ))
    (type $__closure_env_1 (struct ))
    (type $__closure_env_2 (struct (field $count (mut (ref null $signals_Signal_i32))) (field $display (mut (ref null $dom_Element))) ))
    (type $__closure_env_3 (struct ))
    (type $__closure_env_4 (struct (field $count (mut (ref null $signals_Signal_i32))) ))
    (type $array_signals_Effect (array (mut (ref null $signals_Effect))))
    (type $array_byte (array (mut i32)))
    (type $array_anyref (array (mut (ref null any))))
    (type $array_i64 (array (mut i64)))
    (type $array_fn___void (array (mut (ref null $fat_fn___void))))
    (type $array_str (array (mut externref)))
    (type $array_i32 (array (mut i32)))
  )
  (import "wasm:js-string" "concat" (func $fox_js_string_concat (param externref externref) (result (ref extern))))
  (import "wasm:js-string" "length" (func $fox_js_string_length (param externref) (result i32)))
  (import "wasm:js-string" "charCodeAt" (func $fox_js_string_char_code_at (param externref i32) (result i32)))
  (import "wasm:js-string" "fromCharCode" (func $fox_fromCharCode (param i32) (result (ref extern))))
  (import "env" "f_con" (func $__fox_dom_console (param $level i32) (param $msg externref)))
  (import "env" "f_dn" (func $__fox_dom_is_null (param $r externref) (result i32)))
  (import "env" "f_dns" (func $__fox_dom_is_null_str (param $s externref) (result i32)))
  (import "env" "f_dac" (func $__fox_dom_element_append_child (param $parent externref) (param $child externref)))
  (import "env" "f_dsa" (func $__fox_dom_element_set_attribute (param $el externref) (param $name externref) (param $value externref)))
  (import "env" "f_dga" (func $__fox_dom_element_get_attribute (param $el externref) (param $name externref) (result externref)))
  (import "env" "f_dra" (func $__fox_dom_element_remove_attribute (param $el externref) (param $name externref)))
  (import "env" "f_dst" (func $__fox_dom_element_set_text_content (param $el externref) (param $text externref)))
  (import "env" "f_dgt" (func $__fox_dom_element_get_text_content (param $el externref) (result externref)))
  (import "env" "f_dcl" (func $__fox_dom_element_add_click_listener (param $el externref) (param $handler (ref null $fat_fn___void))))
  (import "env" "f_dqs" (func $__fox_dom_document_query_selector (param $selector externref) (result externref)))
  (import "env" "f_dce" (func $__fox_dom_document_create_element (param $tag externref) (result externref)))
  (import "env" "f_dpn" (func $__fox_dom_performance_now (result f64)))
  (import "env" "f_ay" (func $__fox_async_yield))
  (import "env" "f_as" (func $__fox_async_sleep (param $ms i32)))
  (import "env" "f_qt" (func $__fox_queue_task (param $f (ref null $fat_fn___void))))
  (global $fnv1a_OFFSET_32 i32 (i32.const -2128831035))
  (global $fnv1a_PRIME_32 i32 (i32.const 16777619))
  (global $signals_current_effect (mut (ref null $option_Option_signals_Effect)) (ref.null $option_Option_signals_Effect))
  (global $signals_effect_id (mut i64) (i64.const 0))
  (elem declare func $counter__closure_1 $counter__closure_2 $counter__closure_3 $counter__closure_4)
  (func $fox_int_to_str (param $v i32) (result externref)
    (local $len i32)
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
  )

  (func $fox_str4 (param $c1 i32) (param $c2 i32) (param $c3 i32) (param $c4 i32) (result externref)
    (local $res externref)
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
  )

  (type $ty_sprintf_sprintf (func (param externref) (param (ref null $array_anyref)) (result externref)))
  (func $sprintf_sprintf (type $ty_sprintf_sprintf)
    (local $fmt externref)
    (local $args (ref null $array_anyref))
    (local $result externref)
    (local $i i32)
    (local $arg_idx i32)
    (local $c i32)
    (local $arg (ref null any))
    (local $n_args i32)
    (local $fmt_len i32)
    (local $char_str externref)

    local.get 0
    local.set $fmt
    local.get 1
    local.set $args
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
  )
  (export "sprintf_sprintf" (func $sprintf_sprintf))
  (func $console_Console_log (param $msg externref)
    i32.const 1
    local.get $msg
    call $__fox_dom_console
  )
  (type $ty_fnv1a_Hasher32_new (func (result (ref null $fnv1a_Hasher32))))
  (func $fnv1a_Hasher32_new (type $ty_fnv1a_Hasher32_new)
    global.get $fnv1a_OFFSET_32
    struct.new $fnv1a_Hasher32
    return
    unreachable
  )
  (type $ty_fnv1a_Hasher32_reset (func (param (ref null $fnv1a_Hasher32))))
  (func $fnv1a_Hasher32_reset (type $ty_fnv1a_Hasher32_reset)
    (local $self (ref null $fnv1a_Hasher32))
    local.get 0
    local.set $self
    local.get $self
    global.get $fnv1a_OFFSET_32
    struct.set $fnv1a_Hasher32 $state
  )
  (type $ty_fnv1a_Hasher32_write_i64 (func (param (ref null $fnv1a_Hasher32)) (param i64)))
  (func $fnv1a_Hasher32_write_i64 (type $ty_fnv1a_Hasher32_write_i64)
    (local $i i32)
    (local $shift i32)
    (local $b i32)
    (local $self (ref null $fnv1a_Hasher32))
    (local $x i64)
    local.get 0
    local.set $self
    local.get 1
    local.set $x
    i32.const 0
    local.set $i
    block $_wblock0
    loop $_wloop0
    local.get $i
    i32.const 8
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $i
    i32.const 8
    i32.mul
    local.set $shift
    local.get $x
    local.get $shift
    i64.extend_i32_s
    i64.shr_s
    i64.const 255
    i64.and
    i32.wrap_i64
    local.set $b
    local.get $self
    local.get $self
    struct.get $fnv1a_Hasher32 $state
    local.get $b
    i32.xor
    global.get $fnv1a_PRIME_32
    i32.mul
    struct.set $fnv1a_Hasher32 $state
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $_wloop0
    end
    end
  )
  (type $ty_fnv1a_Hasher32_finish (func (param (ref null $fnv1a_Hasher32)) (result i32)))
  (func $fnv1a_Hasher32_finish (type $ty_fnv1a_Hasher32_finish)
    (local $self (ref null $fnv1a_Hasher32))
    local.get 0
    local.set $self
    local.get $self
    struct.get $fnv1a_Hasher32 $state
    return
    i32.const 0
  )
  (type $ty_dom_Element_append_child (func (param (ref null $dom_Element)) (param (ref null $dom_Element))))
  (func $dom_Element_append_child (type $ty_dom_Element_append_child)
    (local $self (ref null $dom_Element))
    (local $child (ref null $dom_Element))
    local.get 0
    local.set $self
    local.get 1
    local.set $child
    local.get $self
    struct.get $dom_Element $_ref
    local.get $child
    struct.get $dom_Element $_ref
    call $__fox_dom_element_append_child
  )
  (type $ty_dom_Element_set_text_content (func (param (ref null $dom_Element)) (param externref)))
  (func $dom_Element_set_text_content (type $ty_dom_Element_set_text_content)
    (local $self (ref null $dom_Element))
    (local $text externref)
    local.get 0
    local.set $self
    local.get 1
    local.set $text
    local.get $self
    struct.get $dom_Element $_ref
    local.get $text
    call $__fox_dom_element_set_text_content
  )
  (type $ty_dom_Element_on_click (func (param (ref null $dom_Element)) (param (ref null $fat_fn___void))))
  (func $dom_Element_on_click (type $ty_dom_Element_on_click)
    (local $self (ref null $dom_Element))
    (local $handler (ref null $fat_fn___void))
    local.get 0
    local.set $self
    local.get 1
    local.set $handler
    local.get $self
    struct.get $dom_Element $_ref
    local.get $handler
    call $__fox_dom_element_add_click_listener
  )
  (type $ty_dom_Document_query_selector (func (param externref) (result (ref null $option_Option_dom_Element))))
  (func $dom_Document_query_selector (type $ty_dom_Document_query_selector)
    (local $el_ref externref)
    (local $selector externref)
    local.get 0
    local.set $selector
    local.get $selector
    call $__fox_dom_document_query_selector
    local.set $el_ref
    local.get $el_ref
    call $__fox_dom_is_null
    if
    return_call $option_Option_dom_Element_None
    end
    local.get $el_ref
    struct.new $dom_Element
    return_call $option_Option_dom_Element_Some
    unreachable
  )
  (type $ty_dom_Document_create_element (func (param externref) (result (ref null $dom_Element))))
  (func $dom_Document_create_element (type $ty_dom_Document_create_element)
    (local $el_ref externref)
    (local $tag externref)
    local.get 0
    local.set $tag
    local.get $tag
    call $__fox_dom_document_create_element
    local.set $el_ref
    local.get $el_ref
    struct.new $dom_Element
    return
    unreachable
  )
  (type $ty_task_fox_run_task (func (param (ref null $fat_fn___void))))
  (func $task_fox_run_task (type $ty_task_fox_run_task)
    (local $_invoke_ptr_0 (ref null $fat_fn___void))
    (local $f (ref null $fat_fn___void))
    local.get 0
    local.set $f
    local.get $f
    local.set $_invoke_ptr_0
    local.get $_invoke_ptr_0
    struct.get $fat_fn___void 1
    local.get $_invoke_ptr_0
    struct.get $fat_fn___void 0
    call_ref $sig_fat_fn___void
  )
  (export "fox_run_task" (func $task_fox_run_task))
  (type $ty_signals_Effect_hash (func (param (ref null $signals_Effect)) (param (ref null $fnv1a_Hasher32))))
  (func $signals_Effect_hash (type $ty_signals_Effect_hash)
    (local $self (ref null $signals_Effect))
    (local $hasher (ref null $fnv1a_Hasher32))
    local.get 0
    local.set $self
    local.get 1
    local.set $hasher
    local.get $hasher
    local.get $self
    struct.get $signals_Effect $id
    call $fnv1a_Hasher32_write_i64
  )
  (func $signals_get_effect_id (result i64)
    global.get $signals_effect_id
    i64.const 1
    i64.add
    global.set $signals_effect_id
    global.get $signals_effect_id
    return
    i64.const 0
  )
  (type $ty_signals_signal_i32 (func (param i32) (result (ref null $signals_Signal_i32))))
  (func $signals_signal_i32 (type $ty_signals_signal_i32)
    (local $val i32)
    local.get 0
    local.set $val
    local.get $val
    call $set_Set_signals_Effect_new
    struct.new $signals_Signal_i32
    return
  )
  (type $ty_signals_effect (func (param (ref null $fat_fn___Option_fn___void))))
  (func $signals_effect (type $ty_signals_effect)
    (local $f (ref null $fat_fn___Option_fn___void))
    local.get 0
    local.set $f
    call $signals_get_effect_id
    local.get $f
    call $vec_Vec_fn___void_new
    struct.new $signals_Effect
    call $signals_run_effect
  )
  (type $ty_signals_run_effect (func (param (ref null $signals_Effect))))
  (func $signals_run_effect (type $ty_signals_run_effect)
    (local $prev (ref null $option_Option_signals_Effect))
    (local $_match_val0 (ref null $option_Option_fn___void))
    (local $_field_call_1 (ref null $fat_fn___Option_fn___void))
    (local $td (ref null $fat_fn___void))
    (local $ef (ref null $signals_Effect))
    local.get 0
    local.set $ef
    global.get $signals_current_effect
    local.set $prev
    local.get $ef
    call $option_Option_signals_Effect_Some
    global.set $signals_current_effect
    local.get $ef
    struct.get $signals_Effect $run
    local.set $_field_call_1
    local.get $_field_call_1
    struct.get $fat_fn___Option_fn___void 1
    local.get $_field_call_1
    struct.get $fat_fn___Option_fn___void 0
    call_ref $sig_fat_fn___Option_fn___void
    local.set $_match_val0
    block $_match_end0
    block $_match_arm_0_1
    block $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_fn___void $_tag
    i32.const 0
    i32.ne
    br_if $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_fn___void $Some_0
    local.set $td
    local.get $ef
    struct.get $signals_Effect $teardowns
    local.get $td
    call $vec_Vec_fn___void_push
    br $_match_end0
    end
    local.get $_match_val0
    struct.get $option_Option_fn___void $_tag
    i32.const 1
    i32.ne
    br_if $_match_arm_0_1
    br $_match_end0
    end
    end
    local.get $prev
    global.set $signals_current_effect
  )
  (type $ty_signals_cleanup (func (param (ref null $signals_Effect))))
  (func $signals_cleanup (type $ty_signals_cleanup)
    (local $_match_val1 (ref null $option_Option_fn___void))
    (local $_match_res1 i32)
    (local $td (ref null $fat_fn___void))
    (local $_invoke_ptr_2 (ref null $fat_fn___void))
    (local $ef (ref null $signals_Effect))
    local.get 0
    local.set $ef
    block $_wblock0
    loop $_wloop0
    local.get $ef
    struct.get $signals_Effect $teardowns
    call $vec_Vec_fn___void_pop
    local.set $_match_val1
    block $_match_end1
    block $_match_arm_1_1
    block $_match_arm_1_0
    local.get $_match_val1
    struct.get $option_Option_fn___void $_tag
    i32.const 0
    i32.ne
    br_if $_match_arm_1_0
    local.get $_match_val1
    struct.get $option_Option_fn___void $Some_0
    local.set $td
    local.get $td
    local.set $_invoke_ptr_2
    local.get $_invoke_ptr_2
    struct.get $fat_fn___void 1
    local.get $_invoke_ptr_2
    struct.get $fat_fn___void 0
    call_ref $sig_fat_fn___void
    i32.const 1
    local.set $_match_res1
    br $_match_end1
    end
    i32.const 0
    local.set $_match_res1
    br $_match_end1
    end
    end
    local.get $_match_res1
    i32.eqz
    br_if $_wblock0
    br $_wloop0
    end
    end
  )
  (func $main
    (local $_match_val0 (ref null $option_Option_dom_Element))
    (local $app_el (ref null $dom_Element))
    global.get $s4
    call $dom_Document_query_selector
    local.set $_match_val0
    block $_match_end0
    block $_match_arm_0_1
    block $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_dom_Element $_tag
    i32.const 0
    i32.ne
    br_if $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_dom_Element $Some_0
    local.set $app_el
    local.get $app_el
    call $counter
    call $dom_Element_append_child
    br $_match_end0
    end
    global.get $s2
    call $console_Console_log
    br $_match_end0
    end
    end
  )
  (export "main" (func $main))
  (type $ty_counter (func (result (ref null $dom_Element))))
  (func $counter (type $ty_counter)
    (local $app_el (ref null $dom_Element))
    (local $count (ref null $signals_Signal_i32))
    (local $display (ref null $dom_Element))
    (local $btn (ref null $dom_Element))
    global.get $s7
    call $dom_Document_create_element
    local.set $app_el
    i32.const 0
    call $signals_signal_i32
    local.set $count
    global.get $s7
    call $dom_Document_create_element
    local.set $display
    local.get $app_el
    local.get $display
    call $dom_Element_append_child
    ref.func $counter__closure_2
    local.get $count
    local.get $display
    struct.new $__closure_env_2
    struct.new $fat_fn___Option_fn___void
    call $signals_effect
    global.get $s5
    call $dom_Document_create_element
    local.set $btn
    local.get $btn
    global.get $s3
    call $dom_Element_set_text_content
    local.get $btn
    ref.func $counter__closure_4
    local.get $count
    struct.new $__closure_env_4
    struct.new $fat_fn___void
    call $dom_Element_on_click
    local.get $app_el
    local.get $btn
    call $dom_Element_append_child
    local.get $app_el
    return
    unreachable
  )
  (func $counter__closure_1 (type $sig_fat_fn___void)
    (local $__env (ref null any))
    local.get 0
    local.set $__env
    global.get $s6
    return_call $console_Console_log
  )
  (func $counter__closure_2 (type $sig_fat_fn___Option_fn___void)
    (local $_varr_0 (ref null $array_anyref))
    (local $_varr_1 (ref null $array_anyref))
    (local $_varr_2 (ref null $array_anyref))
    (local $_varr_3 (ref null $array_anyref))
    (local $_varr_4 (ref null $array_anyref))
    (local $_varr_5 (ref null $array_anyref))
    (local $_varr_6 (ref null $array_anyref))
    (local $_varr_7 (ref null $array_anyref))
    (local $__env_struct (ref null $__closure_env_2))
    (local $count (ref null $signals_Signal_i32))
    (local $display (ref null $dom_Element))
    (local $current i32)
    (local $__env (ref null any))
    local.get 0
    local.set $__env
    local.get $__env
    ref.cast (ref null $__closure_env_2)
    local.set $__env_struct
    local.get $__env_struct
    struct.get $__closure_env_2 $count
    local.set $count
    local.get $__env_struct
    struct.get $__closure_env_2 $display
    local.set $display
    local.get $count
    call $signals_Signal_i32_get
    local.set $current
    global.get $s0
    i32.const 1
    array.new_default $array_anyref
    local.set $_varr_0
    local.get $_varr_0
    i32.const 0
    local.get $current
    ref.i31
    array.set $array_anyref
    local.get $_varr_0
    call $sprintf_sprintf
    call $console_Console_log
    local.get $display
    global.get $s1
    i32.const 1
    array.new_default $array_anyref
    local.set $_varr_0
    local.get $_varr_0
    i32.const 0
    local.get $current
    ref.i31
    array.set $array_anyref
    local.get $_varr_0
    call $sprintf_sprintf
    call $dom_Element_set_text_content
    ref.func $counter__closure_1
    struct.new $__closure_env_1
    struct.new $fat_fn___void
    return_call $option_Option_fn___void_Some
    unreachable
  )
  (func $counter__closure_3 (type $sig_fat_fn_i32__i32)
    (local $v i32)
    (local $__env (ref null any))
    local.get 0
    local.set $v
    local.get 1
    local.set $__env
    local.get $v
    i32.const 1
    i32.add
    return
    i32.const 0
  )
  (func $counter__closure_4 (type $sig_fat_fn___void)
    (local $__env_struct (ref null $__closure_env_4))
    (local $count (ref null $signals_Signal_i32))
    (local $__env (ref null any))
    local.get 0
    local.set $__env
    local.get $__env
    ref.cast (ref null $__closure_env_4)
    local.set $__env_struct
    local.get $__env_struct
    struct.get $__closure_env_4 $count
    local.set $count
    local.get $count
    ref.func $counter__closure_3
    struct.new $__closure_env_3
    struct.new $fat_fn_i32__i32
    return_call $signals_Signal_i32_update
  )
  (type $ty_option_Option_dom_Element_Some (func (param (ref null $dom_Element)) (result (ref null $option_Option_dom_Element))))
  (func $option_Option_dom_Element_Some (type $ty_option_Option_dom_Element_Some)
    (local $payload_0 (ref null $dom_Element))
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_dom_Element
    return
    unreachable
  )
  (type $ty_option_Option_dom_Element_None (func (result (ref null $option_Option_dom_Element))))
  (func $option_Option_dom_Element_None (type $ty_option_Option_dom_Element_None)
    i32.const 1
    ref.null $dom_Element
    struct.new $option_Option_dom_Element
    return
    unreachable
  )
  (type $ty_option_Option_fn___void_Some (func (param (ref null $fat_fn___void)) (result (ref null $option_Option_fn___void))))
  (func $option_Option_fn___void_Some (type $ty_option_Option_fn___void_Some)
    (local $payload_0 (ref null $fat_fn___void))
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_fn___void
    return
    unreachable
  )
  (type $ty_option_Option_fn___void_None (func (result (ref null $option_Option_fn___void))))
  (func $option_Option_fn___void_None (type $ty_option_Option_fn___void_None)
    i32.const 1
    ref.null $sig_fat_fn___void
    ref.null any
    struct.new $fat_fn___void
    struct.new $option_Option_fn___void
    return
    unreachable
  )
  (type $ty_option_Option_i32_Some (func (param i32) (result (ref null $option_Option_i32))))
  (func $option_Option_i32_Some (type $ty_option_Option_i32_Some)
    (local $payload_0 i32)
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_i32
    return
    unreachable
  )
  (type $ty_option_Option_i32_None (func (result (ref null $option_Option_i32))))
  (func $option_Option_i32_None (type $ty_option_Option_i32_None)
    i32.const 1
    i32.const 0
    struct.new $option_Option_i32
    return
    unreachable
  )
  (type $ty_option_Option_i64_Some (func (param i64) (result (ref null $option_Option_i64))))
  (func $option_Option_i64_Some (type $ty_option_Option_i64_Some)
    (local $payload_0 i64)
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_i64
    return
    unreachable
  )
  (type $ty_option_Option_i64_None (func (result (ref null $option_Option_i64))))
  (func $option_Option_i64_None (type $ty_option_Option_i64_None)
    i32.const 1
    i64.const 0
    struct.new $option_Option_i64
    return
    unreachable
  )
  (type $ty_option_Option_signals_Effect_Some (func (param (ref null $signals_Effect)) (result (ref null $option_Option_signals_Effect))))
  (func $option_Option_signals_Effect_Some (type $ty_option_Option_signals_Effect_Some)
    (local $payload_0 (ref null $signals_Effect))
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_signals_Effect
    return
    unreachable
  )
  (type $ty_option_Option_signals_Effect_None (func (result (ref null $option_Option_signals_Effect))))
  (func $option_Option_signals_Effect_None (type $ty_option_Option_signals_Effect_None)
    i32.const 1
    ref.null $signals_Effect
    struct.new $option_Option_signals_Effect
    return
    unreachable
  )
  (type $ty_option_Option_str_Some (func (param externref) (result (ref null $option_Option_str))))
  (func $option_Option_str_Some (type $ty_option_Option_str_Some)
    (local $payload_0 externref)
    local.get 0
    local.set $payload_0
    i32.const 0
    local.get $payload_0
    struct.new $option_Option_str
    return
    unreachable
  )
  (type $ty_option_Option_str_None (func (result (ref null $option_Option_str))))
  (func $option_Option_str_None (type $ty_option_Option_str_None)
    i32.const 1
    ref.null extern
    struct.new $option_Option_str
    return
    unreachable
  )
  (type $ty_set_Set_i32_new (func (result (ref null $set_Set_i32))))
  (func $set_Set_i32_new (type $ty_set_Set_i32_new)
    i32.const 16
    return_call $set_Set_i32_with_capacity
    unreachable
  )
  (type $ty_set_Set_i32_with_capacity (func (param i32) (result (ref null $set_Set_i32))))
  (func $set_Set_i32_with_capacity (type $ty_set_Set_i32_with_capacity)
    (local $power i32)
    (local $keys (ref null $vec_Vec_i32))
    (local $states (ref null $vec_Vec_i32))
    (local $i i32)
    (local $cap i32)
    local.get 0
    local.set $cap
    i32.const 1
    local.set $power
    block $_wblock0
    loop $_wloop0
    local.get $power
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $power
    i32.const 2
    i32.mul
    local.set $power
    br $_wloop0
    end
    end
    local.get $power
    local.set $cap
    local.get $cap
    call $vec_Vec_i32_with_len
    local.set $keys
    local.get $cap
    call $vec_Vec_i32_with_len
    local.set $states
    i32.const 0
    local.set $i
    block $_wblock1
    loop $_wloop1
    local.get $i
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock1
    local.get $states
    local.get $i
    i32.const 0
    call $vec_Vec_i32_set
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $_wloop1
    end
    end
    local.get $keys
    local.get $states
    local.get $cap
    local.get $cap
    i32.const 1
    i32.sub
    i32.const 0
    call $fnv1a_Hasher32_new
    struct.new $set_Set_i32
    return
    unreachable
  )
  (type $ty_set_Set_i64_new (func (result (ref null $set_Set_i64))))
  (func $set_Set_i64_new (type $ty_set_Set_i64_new)
    i32.const 16
    return_call $set_Set_i64_with_capacity
    unreachable
  )
  (type $ty_set_Set_i64_with_capacity (func (param i32) (result (ref null $set_Set_i64))))
  (func $set_Set_i64_with_capacity (type $ty_set_Set_i64_with_capacity)
    (local $power i32)
    (local $keys (ref null $vec_Vec_i64))
    (local $states (ref null $vec_Vec_i32))
    (local $i i32)
    (local $cap i32)
    local.get 0
    local.set $cap
    i32.const 1
    local.set $power
    block $_wblock0
    loop $_wloop0
    local.get $power
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $power
    i32.const 2
    i32.mul
    local.set $power
    br $_wloop0
    end
    end
    local.get $power
    local.set $cap
    local.get $cap
    call $vec_Vec_i64_with_len
    local.set $keys
    local.get $cap
    call $vec_Vec_i32_with_len
    local.set $states
    i32.const 0
    local.set $i
    block $_wblock1
    loop $_wloop1
    local.get $i
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock1
    local.get $states
    local.get $i
    i32.const 0
    call $vec_Vec_i32_set
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $_wloop1
    end
    end
    local.get $keys
    local.get $states
    local.get $cap
    local.get $cap
    i32.const 1
    i32.sub
    i32.const 0
    call $fnv1a_Hasher32_new
    struct.new $set_Set_i64
    return
    unreachable
  )
  (type $ty_set_Set_str_new (func (result (ref null $set_Set_str))))
  (func $set_Set_str_new (type $ty_set_Set_str_new)
    i32.const 16
    return_call $set_Set_str_with_capacity
    unreachable
  )
  (type $ty_set_Set_str_with_capacity (func (param i32) (result (ref null $set_Set_str))))
  (func $set_Set_str_with_capacity (type $ty_set_Set_str_with_capacity)
    (local $power i32)
    (local $keys (ref null $vec_Vec_str))
    (local $states (ref null $vec_Vec_i32))
    (local $i i32)
    (local $cap i32)
    local.get 0
    local.set $cap
    i32.const 1
    local.set $power
    block $_wblock0
    loop $_wloop0
    local.get $power
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $power
    i32.const 2
    i32.mul
    local.set $power
    br $_wloop0
    end
    end
    local.get $power
    local.set $cap
    local.get $cap
    call $vec_Vec_str_with_len
    local.set $keys
    local.get $cap
    call $vec_Vec_i32_with_len
    local.set $states
    i32.const 0
    local.set $i
    block $_wblock1
    loop $_wloop1
    local.get $i
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock1
    local.get $states
    local.get $i
    i32.const 0
    call $vec_Vec_i32_set
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $_wloop1
    end
    end
    local.get $keys
    local.get $states
    local.get $cap
    local.get $cap
    i32.const 1
    i32.sub
    i32.const 0
    call $fnv1a_Hasher32_new
    struct.new $set_Set_str
    return
    unreachable
  )
  (type $ty_set_Set_signals_Effect_new (func (result (ref null $set_Set_signals_Effect))))
  (func $set_Set_signals_Effect_new (type $ty_set_Set_signals_Effect_new)
    i32.const 16
    return_call $set_Set_signals_Effect_with_capacity
    unreachable
  )
  (type $ty_set_Set_signals_Effect_with_capacity (func (param i32) (result (ref null $set_Set_signals_Effect))))
  (func $set_Set_signals_Effect_with_capacity (type $ty_set_Set_signals_Effect_with_capacity)
    (local $power i32)
    (local $keys (ref null $vec_Vec_signals_Effect))
    (local $states (ref null $vec_Vec_i32))
    (local $i i32)
    (local $cap i32)
    local.get 0
    local.set $cap
    i32.const 1
    local.set $power
    block $_wblock0
    loop $_wloop0
    local.get $power
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $power
    i32.const 2
    i32.mul
    local.set $power
    br $_wloop0
    end
    end
    local.get $power
    local.set $cap
    local.get $cap
    call $vec_Vec_signals_Effect_with_len
    local.set $keys
    local.get $cap
    call $vec_Vec_i32_with_len
    local.set $states
    i32.const 0
    local.set $i
    block $_wblock1
    loop $_wloop1
    local.get $i
    local.get $cap
    i32.lt_s
    i32.eqz
    br_if $_wblock1
    local.get $states
    local.get $i
    i32.const 0
    call $vec_Vec_i32_set
    local.get $i
    i32.const 1
    i32.add
    local.set $i
    br $_wloop1
    end
    end
    local.get $keys
    local.get $states
    local.get $cap
    local.get $cap
    i32.const 1
    i32.sub
    i32.const 0
    call $fnv1a_Hasher32_new
    struct.new $set_Set_signals_Effect
    return
    unreachable
  )
  (type $ty_set_Set_signals_Effect_bucket_index (func (param (ref null $set_Set_signals_Effect)) (param (ref null $signals_Effect)) (result i32)))
  (func $set_Set_signals_Effect_bucket_index (type $ty_set_Set_signals_Effect_bucket_index)
    (local $h i32)
    (local $self (ref null $set_Set_signals_Effect))
    (local $key (ref null $signals_Effect))
    local.get 0
    local.set $self
    local.get 1
    local.set $key
    local.get $self
    struct.get $set_Set_signals_Effect $hasher
    call $fnv1a_Hasher32_reset
    local.get $key
    local.get $self
    struct.get $set_Set_signals_Effect $hasher
    call $signals_Effect_hash
    local.get $self
    struct.get $set_Set_signals_Effect $hasher
    call $fnv1a_Hasher32_finish
    local.set $h
    local.get $h
    local.get $self
    struct.get $set_Set_signals_Effect $mask
    i32.and
    return
    i32.const 0
  )
  (type $ty_set_Set_signals_Effect_add (func (param (ref null $set_Set_signals_Effect)) (param (ref null $signals_Effect))))
  (func $set_Set_signals_Effect_add (type $ty_set_Set_signals_Effect_add)
    (local $mask i32)
    (local $idx i32)
    (local $start_idx i32)
    (local $state i32)
    (local $k (ref null $signals_Effect))
    (local $self (ref null $set_Set_signals_Effect))
    (local $key (ref null $signals_Effect))
    local.get 0
    local.set $self
    local.get 1
    local.set $key
    local.get $self
    struct.get $set_Set_signals_Effect $mask
    local.set $mask
    local.get $self
    local.get $key
    call $set_Set_signals_Effect_bucket_index
    local.set $idx
    local.get $idx
    local.set $start_idx
    block $_wblock0
    loop $_wloop0
    i32.const 1
    i32.eqz
    br_if $_wblock0
    local.get $self
    struct.get $set_Set_signals_Effect $states
    struct.get $vec_Vec_i32 $data
    local.get $idx
    array.get $array_i32
    local.set $state
    local.get $state
    i32.const 0
    i32.eq
    if
    local.get $self
    struct.get $set_Set_signals_Effect $keys
    struct.get $vec_Vec_signals_Effect $data
    local.get $idx
    local.get $key
    array.set $array_signals_Effect
    local.get $self
    struct.get $set_Set_signals_Effect $states
    struct.get $vec_Vec_i32 $data
    local.get $idx
    i32.const 1
    array.set $array_i32
    local.get $self
    local.get $self
    struct.get $set_Set_signals_Effect $size
    i32.const 1
    i32.add
    struct.set $set_Set_signals_Effect $size
    return
    end
    local.get $state
    i32.const 1
    i32.eq
    if
    local.get $self
    struct.get $set_Set_signals_Effect $keys
    struct.get $vec_Vec_signals_Effect $data
    local.get $idx
    array.get $array_signals_Effect
    local.set $k
    local.get $k
    local.get $key
    ref.eq
    if
    return
    end
    end
    local.get $idx
    i32.const 1
    i32.add
    local.get $mask
    i32.and
    local.set $idx
    local.get $idx
    local.get $start_idx
    i32.eq
    if
    return
    end
    br $_wloop0
    end
    end
  )
  (type $ty_set_Set_signals_Effect_iter (func (param (ref null $set_Set_signals_Effect)) (result (ref null $set_SetIterator_signals_Effect))))
  (func $set_Set_signals_Effect_iter (type $ty_set_Set_signals_Effect_iter)
    (local $self (ref null $set_Set_signals_Effect))
    local.get 0
    local.set $self
    local.get $self
    i32.const 0
    struct.new $set_SetIterator_signals_Effect
    return
    unreachable
  )
  (type $ty_set_SetIterator_signals_Effect_next (func (param (ref null $set_SetIterator_signals_Effect)) (result (ref null $option_Option_signals_Effect))))
  (func $set_SetIterator_signals_Effect_next (type $ty_set_SetIterator_signals_Effect_next)
    (local $state i32)
    (local $key (ref null $signals_Effect))
    (local $self (ref null $set_SetIterator_signals_Effect))
    local.get 0
    local.set $self
    block $_wblock0
    loop $_wloop0
    local.get $self
    struct.get $set_SetIterator_signals_Effect $index
    local.get $self
    struct.get $set_SetIterator_signals_Effect $set
    struct.get $set_Set_signals_Effect $capacity
    i32.lt_s
    i32.eqz
    br_if $_wblock0
    local.get $self
    struct.get $set_SetIterator_signals_Effect $set
    struct.get $set_Set_signals_Effect $states
    struct.get $vec_Vec_i32 $data
    local.get $self
    struct.get $set_SetIterator_signals_Effect $index
    array.get $array_i32
    local.set $state
    local.get $state
    i32.const 1
    i32.eq
    if
    local.get $self
    struct.get $set_SetIterator_signals_Effect $set
    struct.get $set_Set_signals_Effect $keys
    struct.get $vec_Vec_signals_Effect $data
    local.get $self
    struct.get $set_SetIterator_signals_Effect $index
    array.get $array_signals_Effect
    local.set $key
    local.get $self
    local.get $self
    struct.get $set_SetIterator_signals_Effect $index
    i32.const 1
    i32.add
    struct.set $set_SetIterator_signals_Effect $index
    local.get $key
    return_call $option_Option_signals_Effect_Some
    end
    local.get $self
    local.get $self
    struct.get $set_SetIterator_signals_Effect $index
    i32.const 1
    i32.add
    struct.set $set_SetIterator_signals_Effect $index
    br $_wloop0
    end
    end
    return_call $option_Option_signals_Effect_None
    unreachable
  )
  (type $ty_vec_Vec_fn___void_new (func (result (ref null $vec_Vec_fn___void))))
  (func $vec_Vec_fn___void_new (type $ty_vec_Vec_fn___void_new)
    i32.const 0
    array.new_default $array_fn___void
    i32.const 0
    i32.const 0
    struct.new $vec_Vec_fn___void
    return
    unreachable
  )
  (type $ty_vec_Vec_fn___void_with_len (func (param i32) (result (ref null $vec_Vec_fn___void))))
  (func $vec_Vec_fn___void_with_len (type $ty_vec_Vec_fn___void_with_len)
    (local $l i32)
    local.get 0
    local.set $l
    local.get $l
    array.new_default $array_fn___void
    local.get $l
    local.get $l
    struct.new $vec_Vec_fn___void
    return
    unreachable
  )
  (type $ty_vec_Vec_fn___void_grow (func (param (ref null $vec_Vec_fn___void)) (param i32)))
  (func $vec_Vec_fn___void_grow (type $ty_vec_Vec_fn___void_grow)
    (local $new_data (ref null $array_fn___void))
    (local $self (ref null $vec_Vec_fn___void))
    (local $ncap i32)
    local.get 0
    local.set $self
    local.get 1
    local.set $ncap
    local.get $ncap
    local.get $self
    struct.get $vec_Vec_fn___void $cap
    i32.le_s
    if
    return
    end
    local.get $ncap
    array.new_default $array_fn___void
    local.set $new_data
    local.get $new_data
    i32.const 0
    local.get $self
    struct.get $vec_Vec_fn___void $data
    i32.const 0
    local.get $self
    struct.get $vec_Vec_fn___void $len
    array.copy $array_fn___void $array_fn___void
    local.get $self
    local.get $new_data
    struct.set $vec_Vec_fn___void $data
    local.get $self
    local.get $ncap
    struct.set $vec_Vec_fn___void $cap
  )
  (type $ty_vec_Vec_fn___void_push (func (param (ref null $vec_Vec_fn___void)) (param (ref null $fat_fn___void))))
  (func $vec_Vec_fn___void_push (type $ty_vec_Vec_fn___void_push)
    (local $ncap i32)
    (local $self (ref null $vec_Vec_fn___void))
    (local $val (ref null $fat_fn___void))
    local.get 0
    local.set $self
    local.get 1
    local.set $val
    local.get $self
    struct.get $vec_Vec_fn___void $len
    local.get $self
    struct.get $vec_Vec_fn___void $cap
    i32.ge_s
    if
    local.get $self
    struct.get $vec_Vec_fn___void $cap
    i32.const 2
    i32.mul
    local.set $ncap
    local.get $ncap
    i32.const 4
    i32.le_s
    if
    i32.const 4
    local.set $ncap
    end
    local.get $self
    local.get $ncap
    call $vec_Vec_fn___void_grow
    end
    local.get $self
    struct.get $vec_Vec_fn___void $data
    local.get $self
    struct.get $vec_Vec_fn___void $len
    local.get $val
    array.set $array_fn___void
    local.get $self
    local.get $self
    struct.get $vec_Vec_fn___void $len
    i32.const 1
    i32.add
    struct.set $vec_Vec_fn___void $len
  )
  (type $ty_vec_Vec_fn___void_pop (func (param (ref null $vec_Vec_fn___void)) (result (ref null $option_Option_fn___void))))
  (func $vec_Vec_fn___void_pop (type $ty_vec_Vec_fn___void_pop)
    (local $self (ref null $vec_Vec_fn___void))
    local.get 0
    local.set $self
    local.get $self
    struct.get $vec_Vec_fn___void $len
    i32.const 0
    i32.le_s
    if
    return_call $option_Option_fn___void_None
    end
    local.get $self
    local.get $self
    struct.get $vec_Vec_fn___void $len
    i32.const 1
    i32.sub
    struct.set $vec_Vec_fn___void $len
    local.get $self
    struct.get $vec_Vec_fn___void $data
    local.get $self
    struct.get $vec_Vec_fn___void $len
    array.get $array_fn___void
    return_call $option_Option_fn___void_Some
    unreachable
  )
  (type $ty_vec_Vec_i32_new (func (result (ref null $vec_Vec_i32))))
  (func $vec_Vec_i32_new (type $ty_vec_Vec_i32_new)
    i32.const 0
    array.new_default $array_i32
    i32.const 0
    i32.const 0
    struct.new $vec_Vec_i32
    return
    unreachable
  )
  (type $ty_vec_Vec_i32_with_len (func (param i32) (result (ref null $vec_Vec_i32))))
  (func $vec_Vec_i32_with_len (type $ty_vec_Vec_i32_with_len)
    (local $l i32)
    local.get 0
    local.set $l
    local.get $l
    array.new_default $array_i32
    local.get $l
    local.get $l
    struct.new $vec_Vec_i32
    return
    unreachable
  )
  (type $ty_vec_Vec_i32_set (func (param (ref null $vec_Vec_i32)) (param i32) (param i32)))
  (func $vec_Vec_i32_set (type $ty_vec_Vec_i32_set)
    (local $self (ref null $vec_Vec_i32))
    (local $i i32)
    (local $val i32)
    local.get 0
    local.set $self
    local.get 1
    local.set $i
    local.get 2
    local.set $val
    local.get $self
    struct.get $vec_Vec_i32 $data
    local.get $i
    local.get $val
    array.set $array_i32
  )
  (type $ty_vec_Vec_i64_new (func (result (ref null $vec_Vec_i64))))
  (func $vec_Vec_i64_new (type $ty_vec_Vec_i64_new)
    i32.const 0
    array.new_default $array_i64
    i32.const 0
    i32.const 0
    struct.new $vec_Vec_i64
    return
    unreachable
  )
  (type $ty_vec_Vec_i64_with_len (func (param i32) (result (ref null $vec_Vec_i64))))
  (func $vec_Vec_i64_with_len (type $ty_vec_Vec_i64_with_len)
    (local $l i32)
    local.get 0
    local.set $l
    local.get $l
    array.new_default $array_i64
    local.get $l
    local.get $l
    struct.new $vec_Vec_i64
    return
    unreachable
  )
  (type $ty_vec_Vec_signals_Effect_new (func (result (ref null $vec_Vec_signals_Effect))))
  (func $vec_Vec_signals_Effect_new (type $ty_vec_Vec_signals_Effect_new)
    i32.const 0
    array.new_default $array_signals_Effect
    i32.const 0
    i32.const 0
    struct.new $vec_Vec_signals_Effect
    return
    unreachable
  )
  (type $ty_vec_Vec_signals_Effect_with_len (func (param i32) (result (ref null $vec_Vec_signals_Effect))))
  (func $vec_Vec_signals_Effect_with_len (type $ty_vec_Vec_signals_Effect_with_len)
    (local $l i32)
    local.get 0
    local.set $l
    local.get $l
    array.new_default $array_signals_Effect
    local.get $l
    local.get $l
    struct.new $vec_Vec_signals_Effect
    return
    unreachable
  )
  (type $ty_vec_Vec_str_new (func (result (ref null $vec_Vec_str))))
  (func $vec_Vec_str_new (type $ty_vec_Vec_str_new)
    i32.const 0
    array.new_default $array_str
    i32.const 0
    i32.const 0
    struct.new $vec_Vec_str
    return
    unreachable
  )
  (type $ty_vec_Vec_str_with_len (func (param i32) (result (ref null $vec_Vec_str))))
  (func $vec_Vec_str_with_len (type $ty_vec_Vec_str_with_len)
    (local $l i32)
    local.get 0
    local.set $l
    local.get $l
    array.new_default $array_str
    local.get $l
    local.get $l
    struct.new $vec_Vec_str
    return
    unreachable
  )
  (type $ty_signals_Signal_i32_set (func (param (ref null $signals_Signal_i32)) (param i32)))
  (func $signals_Signal_i32_set (type $ty_signals_Signal_i32_set)
    (local $iter (ref null $set_SetIterator_signals_Effect))
    (local $_match_val1 (ref null $option_Option_signals_Effect))
    (local $_match_res1 i32)
    (local $ef (ref null $signals_Effect))
    (local $self (ref null $signals_Signal_i32))
    (local $v i32)
    local.get 0
    local.set $self
    local.get 1
    local.set $v
    local.get $self
    local.get $v
    struct.set $signals_Signal_i32 $value
    local.get $self
    struct.get $signals_Signal_i32 $subs
    call $set_Set_signals_Effect_iter
    local.set $iter
    block $_wblock0
    loop $_wloop0
    local.get $iter
    call $set_SetIterator_signals_Effect_next
    local.set $_match_val1
    block $_match_end1
    block $_match_arm_1_1
    block $_match_arm_1_0
    local.get $_match_val1
    struct.get $option_Option_signals_Effect $_tag
    i32.const 0
    i32.ne
    br_if $_match_arm_1_0
    local.get $_match_val1
    struct.get $option_Option_signals_Effect $Some_0
    local.set $ef
    local.get $ef
    call $signals_cleanup
    local.get $ef
    call $signals_run_effect
    i32.const 1
    local.set $_match_res1
    br $_match_end1
    end
    i32.const 0
    local.set $_match_res1
    br $_match_end1
    end
    end
    local.get $_match_res1
    i32.eqz
    br_if $_wblock0
    br $_wloop0
    end
    end
  )
  (type $ty_signals_Signal_i32_update (func (param (ref null $signals_Signal_i32)) (param (ref null $fat_fn_i32__i32))))
  (func $signals_Signal_i32_update (type $ty_signals_Signal_i32_update)
    (local $_invoke_ptr_0 (ref null $fat_fn_i32__i32))
    (local $self (ref null $signals_Signal_i32))
    (local $f (ref null $fat_fn_i32__i32))
    local.get 0
    local.set $self
    local.get 1
    local.set $f
    local.get $self
    local.get $f
    local.set $_invoke_ptr_0
    local.get $self
    struct.get $signals_Signal_i32 $value
    local.get $_invoke_ptr_0
    struct.get $fat_fn_i32__i32 1
    local.get $_invoke_ptr_0
    struct.get $fat_fn_i32__i32 0
    call_ref $sig_fat_fn_i32__i32
    call $signals_Signal_i32_set
  )
  (type $ty_signals_Signal_i32_get (func (param (ref null $signals_Signal_i32)) (result i32)))
  (func $signals_Signal_i32_get (type $ty_signals_Signal_i32_get)
    (local $_match_val0 (ref null $option_Option_signals_Effect))
    (local $ef (ref null $signals_Effect))
    (local $self (ref null $signals_Signal_i32))
    local.get 0
    local.set $self
    global.get $signals_current_effect
    local.set $_match_val0
    block $_match_end0
    block $_match_arm_0_1
    block $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_signals_Effect $_tag
    i32.const 0
    i32.ne
    br_if $_match_arm_0_0
    local.get $_match_val0
    struct.get $option_Option_signals_Effect $Some_0
    local.set $ef
    local.get $self
    struct.get $signals_Signal_i32 $subs
    local.get $ef
    call $set_Set_signals_Effect_add
    br $_match_end0
    end
    br $_match_end0
    end
    end
    local.get $self
    struct.get $signals_Signal_i32 $value
    return
    i32.const 0
  )
  (func $fox_alloc_console_Console (result (ref $console_Console))
    struct.new $console_Console
  )
  (export "fox_alloc_console_Console" (func $fox_alloc_console_Console))
  (func $fox_alloc_option_Option_dom_Element (param $_tag i32) (param $Some_0 (ref null $dom_Element)) (result (ref $option_Option_dom_Element))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_dom_Element
  )
  (export "fox_alloc_option_Option_dom_Element" (func $fox_alloc_option_Option_dom_Element))
  (func $fox_alloc_option_Option_fn___void (param $_tag i32) (param $Some_0 (ref null $fat_fn___void)) (result (ref $option_Option_fn___void))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_fn___void
  )
  (export "fox_alloc_option_Option_fn___void" (func $fox_alloc_option_Option_fn___void))
  (func $fox_alloc_option_Option_i32 (param $_tag i32) (param $Some_0 i32) (result (ref $option_Option_i32))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_i32
  )
  (export "fox_alloc_option_Option_i32" (func $fox_alloc_option_Option_i32))
  (func $fox_alloc_option_Option_i64 (param $_tag i32) (param $Some_0 i64) (result (ref $option_Option_i64))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_i64
  )
  (export "fox_alloc_option_Option_i64" (func $fox_alloc_option_Option_i64))
  (func $fox_alloc_option_Option_signals_Effect (param $_tag i32) (param $Some_0 (ref null $signals_Effect)) (result (ref $option_Option_signals_Effect))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_signals_Effect
  )
  (export "fox_alloc_option_Option_signals_Effect" (func $fox_alloc_option_Option_signals_Effect))
  (func $fox_alloc_option_Option_str (param $_tag i32) (param $Some_0 externref) (result (ref $option_Option_str))
    local.get $_tag
    local.get $Some_0
    struct.new $option_Option_str
  )
  (export "fox_alloc_option_Option_str" (func $fox_alloc_option_Option_str))
  (func $fox_alloc_fnv1a_Hasher32 (param $state i32) (result (ref $fnv1a_Hasher32))
    local.get $state
    struct.new $fnv1a_Hasher32
  )
  (export "fox_alloc_fnv1a_Hasher32" (func $fox_alloc_fnv1a_Hasher32))
  (func $fox_alloc_set_Set_i32 (param $keys (ref null $vec_Vec_i32)) (param $states (ref null $vec_Vec_i32)) (param $capacity i32) (param $mask i32) (param $size i32) (param $hasher (ref null $fnv1a_Hasher32)) (result (ref $set_Set_i32))
    local.get $keys
    local.get $states
    local.get $capacity
    local.get $mask
    local.get $size
    local.get $hasher
    struct.new $set_Set_i32
  )
  (export "fox_alloc_set_Set_i32" (func $fox_alloc_set_Set_i32))
  (func $fox_alloc_set_Set_i64 (param $keys (ref null $vec_Vec_i64)) (param $states (ref null $vec_Vec_i32)) (param $capacity i32) (param $mask i32) (param $size i32) (param $hasher (ref null $fnv1a_Hasher32)) (result (ref $set_Set_i64))
    local.get $keys
    local.get $states
    local.get $capacity
    local.get $mask
    local.get $size
    local.get $hasher
    struct.new $set_Set_i64
  )
  (export "fox_alloc_set_Set_i64" (func $fox_alloc_set_Set_i64))
  (func $fox_alloc_set_Set_str (param $keys (ref null $vec_Vec_str)) (param $states (ref null $vec_Vec_i32)) (param $capacity i32) (param $mask i32) (param $size i32) (param $hasher (ref null $fnv1a_Hasher32)) (result (ref $set_Set_str))
    local.get $keys
    local.get $states
    local.get $capacity
    local.get $mask
    local.get $size
    local.get $hasher
    struct.new $set_Set_str
  )
  (export "fox_alloc_set_Set_str" (func $fox_alloc_set_Set_str))
  (func $fox_alloc_set_Set_signals_Effect (param $keys (ref null $vec_Vec_signals_Effect)) (param $states (ref null $vec_Vec_i32)) (param $capacity i32) (param $mask i32) (param $size i32) (param $hasher (ref null $fnv1a_Hasher32)) (result (ref $set_Set_signals_Effect))
    local.get $keys
    local.get $states
    local.get $capacity
    local.get $mask
    local.get $size
    local.get $hasher
    struct.new $set_Set_signals_Effect
  )
  (export "fox_alloc_set_Set_signals_Effect" (func $fox_alloc_set_Set_signals_Effect))
  (func $fox_alloc_set_SetIterator_signals_Effect (param $set (ref null $set_Set_signals_Effect)) (param $index i32) (result (ref $set_SetIterator_signals_Effect))
    local.get $set
    local.get $index
    struct.new $set_SetIterator_signals_Effect
  )
  (export "fox_alloc_set_SetIterator_signals_Effect" (func $fox_alloc_set_SetIterator_signals_Effect))
  (func $fox_alloc_vec_Vec_fn___void (param $data (ref null $array_fn___void)) (param $len i32) (param $cap i32) (result (ref $vec_Vec_fn___void))
    local.get $data
    local.get $len
    local.get $cap
    struct.new $vec_Vec_fn___void
  )
  (export "fox_alloc_vec_Vec_fn___void" (func $fox_alloc_vec_Vec_fn___void))
  (func $fox_alloc_vec_Vec_i32 (param $data (ref null $array_i32)) (param $len i32) (param $cap i32) (result (ref $vec_Vec_i32))
    local.get $data
    local.get $len
    local.get $cap
    struct.new $vec_Vec_i32
  )
  (export "fox_alloc_vec_Vec_i32" (func $fox_alloc_vec_Vec_i32))
  (func $fox_alloc_vec_Vec_i64 (param $data (ref null $array_i64)) (param $len i32) (param $cap i32) (result (ref $vec_Vec_i64))
    local.get $data
    local.get $len
    local.get $cap
    struct.new $vec_Vec_i64
  )
  (export "fox_alloc_vec_Vec_i64" (func $fox_alloc_vec_Vec_i64))
  (func $fox_alloc_vec_Vec_signals_Effect (param $data (ref null $array_signals_Effect)) (param $len i32) (param $cap i32) (result (ref $vec_Vec_signals_Effect))
    local.get $data
    local.get $len
    local.get $cap
    struct.new $vec_Vec_signals_Effect
  )
  (export "fox_alloc_vec_Vec_signals_Effect" (func $fox_alloc_vec_Vec_signals_Effect))
  (func $fox_alloc_vec_Vec_str (param $data (ref null $array_str)) (param $len i32) (param $cap i32) (result (ref $vec_Vec_str))
    local.get $data
    local.get $len
    local.get $cap
    struct.new $vec_Vec_str
  )
  (export "fox_alloc_vec_Vec_str" (func $fox_alloc_vec_Vec_str))
  (func $fox_alloc_dom_Element (param $_ref externref) (result (ref $dom_Element))
    local.get $_ref
    struct.new $dom_Element
  )
  (export "fox_alloc_dom_Element" (func $fox_alloc_dom_Element))
  (func $fox_alloc_dom_Document (result (ref $dom_Document))
    struct.new $dom_Document
  )
  (export "fox_alloc_dom_Document" (func $fox_alloc_dom_Document))
  (func $fox_alloc_signals_Signal_i32 (param $value i32) (param $subs (ref null $set_Set_signals_Effect)) (result (ref $signals_Signal_i32))
    local.get $value
    local.get $subs
    struct.new $signals_Signal_i32
  )
  (export "fox_alloc_signals_Signal_i32" (func $fox_alloc_signals_Signal_i32))
  (func $fox_alloc_signals_Effect (param $id i64) (param $run (ref null $fat_fn___Option_fn___void)) (param $teardowns (ref null $vec_Vec_fn___void)) (result (ref $signals_Effect))
    local.get $id
    local.get $run
    local.get $teardowns
    struct.new $signals_Effect
  )
  (export "fox_alloc_signals_Effect" (func $fox_alloc_signals_Effect))
  (func $fox_alloc___closure_env_1 (result (ref $__closure_env_1))
    struct.new $__closure_env_1
  )
  (export "fox_alloc___closure_env_1" (func $fox_alloc___closure_env_1))
  (func $fox_alloc___closure_env_2 (param $count (ref null $signals_Signal_i32)) (param $display (ref null $dom_Element)) (result (ref $__closure_env_2))
    local.get $count
    local.get $display
    struct.new $__closure_env_2
  )
  (export "fox_alloc___closure_env_2" (func $fox_alloc___closure_env_2))
  (func $fox_alloc___closure_env_3 (result (ref $__closure_env_3))
    struct.new $__closure_env_3
  )
  (export "fox_alloc___closure_env_3" (func $fox_alloc___closure_env_3))
  (func $fox_alloc___closure_env_4 (param $count (ref null $signals_Signal_i32)) (result (ref $__closure_env_4))
    local.get $count
    struct.new $__closure_env_4
  )
  (export "fox_alloc___closure_env_4" (func $fox_alloc___closure_env_4))
  
  
  
  
  
  
  
  
  (func $fox_alloc_array_anyref (param $len i32) (result (ref $array_anyref))
    local.get $len
    array.new_default $array_anyref
  )
  (export "fox_alloc_array_anyref" (func $fox_alloc_array_anyref))
  (func $fox_set_array_anyref (param $arr (ref $array_anyref)) (param $idx i32) (param $val (ref null any))
    local.get $arr
    local.get $idx
    local.get $val
    array.set $array_anyref
  )
  (export "fox_set_array_anyref" (func $fox_set_array_anyref))
  
  
  
  
  
  
  
  
  
  
  
  
  
  
  
  
  (func $__fox_global_init
    call $option_Option_signals_Effect_None
    global.set $signals_current_effect
  )
  (start $__fox_global_init)
)
