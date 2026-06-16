# Fox Language Specification

## 1. Introduction
Fox is a statically-typed, garbage-collected programming language designed to target WebAssembly natively. It utilizes standard WebAssembly Garbage Collection (Wasm GC) and JavaScript-String integration (`wasm:js-string` built-ins) for efficient execution and memory management. Fox features structured control flow, a strict type system without type inference, support for generic structures and functions, trait-based polymorphism, and safe pattern matching on Option types.

---

## 2. Lexical Structure & Tokens

### 2.1 Comments
Single-line comments start with `//` and continue to the end of the line. Block comments are not supported.
```fox
// This is a comment
```

### 2.2 Identifiers & Case-Sensitivity
Identifiers consist of alphanumeric characters and underscores, beginning with a letter or underscore: `[a-zA-Z_][a-zA-Z0-9_]*`.
- **Casing Semantics:** Identifiers starting with uppercase letters are treated differently by the parser in conditional expressions to resolve syntactic ambiguity between block conditions and struct initialization blocks. For example:
  ```fox
  if x < PI { ... } // PI is parsed as an identifier, not a struct initialization
  ```

### 2.3 Literals
- **Booleans:** `true` and `false`.
- **Integers:** Sequence of digits. e.g., `42`.
- **Floats:** Sequence of digits containing a decimal point. e.g., `3.14159`.
- **Strings:** Character sequences enclosed in double quotes `""`. e.g., `"hello world"`.

### 2.4 Keywords & Reserved Types
The following keywords are reserved in Fox:
- `use`, `as`, `extern`, `fn`, `true`, `false`, `return`, `let`, `const`, `for`, `in`, `if`, `while`, `struct`, `pub`, `static`, `new`, `map`, `trait`, `impl`, `else`, `match`, `enum`

The following type names are reserved:
- `i32`, `i64`, `u32`, `u64`, `f32`, `f64`, `str`, `void`, `byte`, `bool`, `anyref`, `externref`

---

## 3. Type System

Fox is a strictly typed language. **Type annotations are required for numeric literals**; type inference is supported for non-numeric expressions.
```fox
let a: i32 = 5; // Valid
let b = 5;     // Compiler Error: Type annotation is required for variable binding
```

### 3.1 Primitive Types & WebAssembly Mapping
When compiled to WebAssembly Text (WAT), Fox types map directly to Wasm types:

| Fox Type | Wasm / WAT Representation | Description |
| :--- | :--- | :--- |
| `i32` | `i32` | 32-bit signed integer |
| `i64` | `i64` | 64-bit signed integer |
| `u32` | `i32` | 32-bit unsigned integer (represented as i32) |
| `u64` | `i64` | 64-bit unsigned integer (represented as i64) |
| `f32` | `f32` | 32-bit single-precision float |
| `f64` | `f64` | 64-bit double-precision float |
| `bool` | `i32` | Boolean value (represented as `1` or `0`) |
| `byte` | `i32` | 8-bit unsigned byte (represented as i32) |
| `str` | `externref` | Native string referencing JavaScript string |
| `anyref` | `externref` | General external reference type |
| `externref` | `externref` | WebAssembly external reference |
| `void` | None | Empty return type |

### 3.2 Compound Types
- **Arrays (`[]T`):** Reference types representing garbage-collected arrays of type `T`. Mapped to WAT as `(ref null $array_T)`.
- **Structs (`S`):** Reference types representing garbage-collected struct definitions. Mapped to WAT as `(ref null $S)`.
- **Tuples (`(T1, T2, ...)`):** Types representing fixed-size collections of values of potentially different types. Mapped to WAT as garbage-collected struct types.

### 3.3 Generics & Constraints
Fox supports generic parameters for structs, functions, traits, and implementations.
Constraints on generic parameters are declared using the `in` keyword:
- **Trait Constraint:** A generic parameter can be constrained to implement a specific trait.
  ```fox
  struct Formatter<T in Display> { ... }
  ```
- **Type Union Constraint:** A generic parameter can be restricted to a set of specified primitive types separated by `|`.
  ```fox
  pub fn assert_eq<T in i32 | i64 | str | bool>(a: T, b: T): void { ... }
  ```
Unconstrained generic parameters automatically undergo monomorphization analysis at compile time to specialize types based on AST-level instantiations.

---

## 4. Program Structure & Declarations

### 4.1 Modules and Imports
- **Files as Modules**: Every `.fox` file constitutes a separate module.
- **File Isolation**: Items (structs, enums, functions, constants, traits) declared in a file are entirely private to that file by default. They cannot be seen or used by any other file, even files in the same directory, unless explicitly exported and imported.
- **Visibility (`pub`)**: To allow an item to be imported by other files, it must be prefixed with the `pub` keyword (e.g., `pub struct`, `pub enum`, `pub fn`, `pub const`, `pub trait`).
- **Directory Imports**: The `use` statement operates on directories (packages) to import items. For example, `use std::collections::{Vec, Map};` searches the `std/collections` directory.
- **Aliasing / Renaming (`as`)**: Imported symbols can be renamed using the `as` keyword: `use std::fmt as custom_fmt;` or `use std::collections::{Vec as Vector};`.
- **Strict Explicit Imports**: `use path::dir::{A};` explicitly imports *only* `A` into the current module. Attempting to use unimported items from that directory or attempting to import an item that is not `pub` is a compile error.
- **Resolution**:
  - Modules under the `std` namespace are searched in the directory specified by the `FOX_PATH` environment variable.
  - Modules under the `self` namespace are resolved relative to the current workspace root directory.
  - Imported symbols are merged into the program's global AST with their fully qualified names (e.g., `std::collections::Vec`), but are resolved correctly based on explicit module imports.

### 4.2 Constants
Global constants are declared using the `const` keyword. They must specify a type and be initialized with a compile-time constant expression.
```fox
pub const PI: f64 = 3.141592653589793;
const PRECISION_LIMIT: f64 = 4503599627370496.0;
```
- Constant initializers support basic arithmetic (`+`, `-`, `*`, `/`, `%`) and references to other declared constants.
- User-defined constants and extern functions are forbidden from using the protected namespace prefix `__fox_`, which is reserved for compiler and standard library built-ins.

### 4.3 Functions
Functions are declared using the `fn` keyword. Parameters and return types must be explicitly typed.
```fox
pub fn add(a: i32, b: i32): i32 {
    return a + b;
}
```
- **Variadic Functions:** Functions can accept variadic arguments using the `...` parameter syntax. The final parameter is of type `anyref`.
  ```fox
  pub fn sprintf(fmt: str, ...args: anyref): str { ... }
  ```

### 4.4 Extern Functions
Extern functions represent imports from the host environment (JavaScript) and use the `extern fn` syntax without a body.
```fox
extern fn __fox_dom_performance_now(): f64;
```
- The standard host environment provides `__fox_panic` (throws an Error in JS) and `__fox_dom_performance_now` (returns `performance.now()` or `Date.now()`).

### 4.5 Structs & Methods
Structs define custom data types containing fields and associated methods.
```fox
struct Point {
    x: f64;
    y: f64;

    pub static fn new(x: f64, y: f64): Point {
        return Point { x: x, y: y };
    }

    pub fn distance_from_origin(): f64 {
        return sqrt(self.x * self.x + self.y * self.y);
    }
}
```
- **Instantiation:** Struct instances are created via curly brace initializers: `Point { x: 1.0, y: 2.0 }`. Fields can be omitted; any omitted fields are automatically initialized to their zero or default values (e.g., `0` for numeric types, `false` for booleans, `null`/`default` for references and arrays).
- **Instance Methods:** Methods without the `static` keyword are instance methods. If the first parameter is named `self`, its type is automatically inferred as the parent struct type. Inside the method body, fields are accessed on `self`.
- **Static Methods:** Associated functions called via `StructName::method_name(...)`.

### 4.6 Traits & Implementations
Traits define shared behavior signatures that target types can implement.
```fox
pub trait Display {
    fn format(): str;
}

impl Display for i32 {
    fn format(): str {
        // ... formatting logic
    }
}
```
- Primitive types like `i32` and `str` can implement traits.
- Within an `impl` body, the special identifier `self` references the instance of the target type.

### 4.7 Inherent Impls on Builtins
The standard library attaches methods to builtin primitive types via inherent `impl` blocks (no trait). Methods declared this way are always in scope — no `use` import is required at the call site.
```fox
impl str {
    pub compiler fn starts_with(self, prefix: str): bool;
    pub compiler fn ends_with(self, suffix: str): bool;
    pub compiler fn is_empty(self): bool;
    fn join(self, other: str): str {
        return self + other;
    }
}

let s: str = "hello";
if s.starts_with("he") && !s.is_empty() { ... }   // no `use` needed
```
- The target of an inherent impl must be a builtin primitive type (`str`, `i32`, `f64`, `[]T`, ...).
- Methods inside the block may be either `pub compiler fn` forward declarations (whose body is supplied by the host) or regular `fn` methods with a Fox body.
- `self` is auto-inserted as the first parameter for non-static methods, just as in trait impls.
- Inherent methods on builtins take precedence over any trait method of the same name during method-call resolution.

### 4.8 Enums
Enums define custom types that can have multiple variants, optionally containing payloads (associated types).
```fox
pub enum TrafficLight {
    Red();
    Yellow();
    Green();
}

pub enum Shape {
    Circle(i32);
    Rectangle(i32, i32);
    Point();
}
```
- **Instantiation:** Enum variants are constructed by calling their constructor name like a function: `let r = Red();` or `let c = Circle(15);`. For generic enums, type parameters are specified: `Boxed<i32>::Full(100)`.
- **Associated Methods:** Enums can have methods declared on them using `impl` blocks:
  ```fox
  impl TrafficLight {
      pub fn is_red(self): bool {
          return match self {
              Red() => true,
              _ => false,
          };
      }
  }
  ```

---

## 5. Statements & Control Flow

### 5.1 Variable Bindings & Assignments
- **Let Binding:** `let name: Type = expr;`
- **Reassignment:** `name = expr;`
- **Plus Assignment:** `name += expr;` (Supported exclusively on identifiers).
- **Index Assignment:** `array[index] = expr;`
- **Field Assignment:** `object.field = expr;`

### 5.2 Conditionals
```fox
if x < 0.0 {
    return 0.0 - x;
} else {
    return x;
}
```

### 5.3 Loops
- **While Loop:** Runs as long as the condition evaluates to `true`.
  ```fox
  while idx < limit {
      idx = idx + 1;
  }
  ```
- **For Loop:** Iterates over a collection.
  ```fox
  for item in vector {
      // Loop body
  }
  ```

### 5.4 Pattern Conditional Bindings (`if let` & `while let`)
Fox supports pattern matching in conditional blocks:
- **`if let` Statement:** Conditionally executes a block if a pattern matches:
  ```fox
  if let Some(val) = opt {
      assert_eq(42, val);
  } else {
      // executes if opt is None
  }
  ```
- **`while let` / `while` Loop:** Loops as long as a pattern matches a expression value:
  ```fox
  while let Some(val) = vec.pop() {
      sum = sum + val;
  }
  
  // The 'let' keyword is optional in while pattern conditions:
  while Some(val) = vec.pop() {
      sum = sum + val;
  }
  ```

---

## 6. Expressions

### 6.1 Instantiation & Allocation
- **Struct Allocation:** Created using struct literals: `Point { x: 0.0, y: 0.0 }`. Wasm GC allocations are handled natively via `struct.new`.
- **Array Allocation:** Array instances are created using the `new` keyword followed by `[size]T` syntax. This allocates a default-initialized Wasm GC array via `array.new_default`.
  ```fox
  let data: []i32 = new [10]i32; // Allocates array of size 10
  ```

### 6.2 Index & Field Access
- **Index Access:** `array[index]` retrieves elements from a GC array.
- **Field Access:** `struct_val.field_name` accesses fields of a struct.

### 6.3 Pattern Matching (`match`)
Pattern matching can be performed using the `match` construct as either a statement or an expression. It supports matching on:
1. `Option<T>` variants: `Some(val)` and `None()`.
2. `Result<T, E>` variants: `Ok(val)` and `Err(err)`.
3. User-defined custom `enum` variants, with payload destructuring (e.g., `Circle(rad)` or `Rectangle(w, h)`).
4. Catch-all pattern `_`.

Match expressions must be exhaustive, covering all variants or providing a catch-all `_` arm.

```fox
// Match statement
match opt {
    Some(val) => {
        assert_eq(42, val);
    },
    None() => {},
}

// Match expression
let n: i32 = match opt {
    Some(val) => val * 2,
    None() => 0,
};
```

### 6.4 Tuples
Tuples represent grouped values of heterogeneous types.
- **Tuple Literal:** Enclosed in parentheses, e.g., `(42, "hello", true)`.
- **Tuple Field Access:** Tuple members can be accessed directly by index (e.g., `t.0`, `t.1`) or via `.f<index>` notation (e.g., `t.f0`, `t.f1`).
- **Tuple Destructuring:** Tuples can be destructuring in `let` bindings:
  ```fox
  let (a: i32, b: str) = (42, "hello");
  let (x: i32, (y: str, z: bool)) = (1, ("two", true));
  ```

---

## 7. Built-in Core Types & Standard Library

### 7.1 Built-in Array Operations
For any array type `[]T`:
- `.len()` returns the size of the array as an `i32`.

### 7.2 String Methods
The primitive `str` type's methods are declared in `std::string` as an inherent `impl str` block (see §4.7). They are always in scope and require no import. The full set is:
- `.len()` returns the number of characters as an `i32`.
- `.char_at(index: i32)` returns the character code at the index as a `i32`.
- `.bytes()` returns the byte array of the string as a `[]byte`.
- `.starts_with(prefix: str)`, `.ends_with(suffix: str)`, `.contains(substr: str)` return `bool`.
- `.index_of(substr: str)`, `.last_index_of(substr: str)` return `i32` (or `-1` if not found).
- `.is_empty()` returns `bool`.
- `.eq(other: str)` returns `bool` (string equality).
- `.join(other: str)` returns `str` (concatenation).
- `.compare(other: str)` returns `i32` (lexicographic compare: < 0, 0, or > 0).

All `str` methods are implemented in the host (JavaScript) and exposed to the Wasm module as imports.

### 7.3 Standard Library Modules
The standard library contains the following modules under `std::`:

1. **`std::testing`**: Assertions for testing.
   - `assert_eq<T>(a: T, b: T)`: Asserts equality for basic types (`i32`, `i64`, `str`, `bool`).
   - `assert_approx_eq(a: f64, b: f64, epsilon: f64)`: Asserts equality of floats within a tolerance.
2. **`std::global`**: Environment and host global APIs.
   - `dom::Document::query_selector(selector: str) -> Option<dom::Element>`: Queries the DOM.
   - `dom::Document::create_element(tag: str) -> dom::Element`: Creates a DOM element.
   - `console::Console::log(msg: str)`: Logs to host console.
   - `performance::Performance::now() -> f64`: Gets high-resolution time.
3. **`std::collections`**: Generic collection types.
   - `Option<T>`: Represents optional values via `Some(T)` and `None()`.
   - `Result<T, E>`: Represents success (`Ok(T)`) or failure (`Err(E)`) values.
   - `Vec<T>`: Dynamic arrays supporting `push`, `pop`, `grow`, `get`, and `set`.
   - `Map<K, V>`: HashMap supporting `set`, `get`, and `has` (where K is constrained by `Hash` trait).
   - `Set<K>`: HashSet supporting `add`, `has`, `delete`, and `iter` (where K is constrained by `Hash` trait).
4. **`std::math`**: Trigonometric, logarithmic, and arithmetic utilities operating on `f64`.
5. **`std::string`**: String manipulation utilities under `StringExt` trait and `StringSlice` struct.
6. **`std::crypto`**: Hashing utilities including the standard `Hash` trait, FNV-1a (32-bit and 64-bit) functions, and stateful hasher structs.
7. **`std::fmt`**: Text formatting utilities.
   - `sprintf(fmt: str, ...args: anyref): str`: Built-in C-style formatted string generator.
