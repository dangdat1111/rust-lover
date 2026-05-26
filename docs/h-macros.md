# Macros trong Rust — Từ macro_rules! đến Procedural Macros

> Tài liệu thứ 8 trong bộ Rust nền tảng. Đọc sau khi đã quen:
> - [trait.md](./trait.md) — macros thường derive traits
> - [generic.md](./generic.md) — macros có thể "thay thế" một số use case generic
> - [error-handling.md](./error-handling.md) — `thiserror` là 1 proc-macro
>
> Macros là **siêu năng lực** của Rust. Chúng cho phép bạn:
> - Sinh code lặp đi lặp lại (vd: `vec![]`, `format!`)
> - Define DSL nhỏ trong code (vd: `quote! { ... }`, `html! { ... }`)
> - Tạo derive tự động (vd: `#[derive(Debug, Clone, Serialize)]`)
> - Phân tích code dạng AST tại compile time
>
> Nhưng macros cũng là **dao 2 lưỡi** — dễ viết code khó debug, error message kỳ lạ.
> Tài liệu này dạy bạn dùng macros như senior: biết khi nào dùng, khi nào tránh.

---

# Mục lục

- [Tầng 1: Tại sao cần Macros?](#tầng-1-tại-sao-cần-macros)
- [Tầng 2: 2 loại Macros trong Rust](#tầng-2-2-loại-macros-trong-rust)
- [Tầng 3: macro_rules! — Pattern matching trên token](#tầng-3-macro_rules--pattern-matching-trên-token)
- [Tầng 4: Fragment specifiers — Phân loại token](#tầng-4-fragment-specifiers--phân-loại-token)
- [Tầng 5: Repetition — `$( )*`, `$( ),*`, `$( );?`](#tầng-5-repetition----)
- [Tầng 6: Hygiene — Vì sao macro Rust không phá scope](#tầng-6-hygiene--vì-sao-macro-rust-không-phá-scope)
- [Tầng 7: Macro trong macro, debug, scoping](#tầng-7-macro-trong-macro-debug-scoping)
- [Tầng 8: Procedural Macros — Cấp độ tiếp theo](#tầng-8-procedural-macros--cấp-độ-tiếp-theo)
- [Tầng 9: Derive Macros — Tự sinh impl trait](#tầng-9-derive-macros--tự-sinh-impl-trait)
- [Tầng 10: Attribute và Function-like proc macros](#tầng-10-attribute-và-function-like-proc-macros)
- [Tầng 11: Hệ sinh thái — syn, quote, proc-macro2](#tầng-11-hệ-sinh-thái--syn-quote-proc-macro2)
- [Tầng 12: Khi nào dùng / không dùng macros](#tầng-12-khi-nào-dùng--không-dùng-macros)
- [Tầng 13: Antipatterns và Pitfalls](#tầng-13-antipatterns-và-pitfalls)

---

# Tầng 1: Tại sao cần Macros?

## 1.1 Vấn đề mà function không giải được

Function trong Rust có nhiều giới hạn:

### Giới hạn 1: Số lượng tham số cố định

```rust
fn add(a: i32, b: i32) -> i32 { a + b }
// Muốn add(1,2,3,4,5)? Không thể với function bình thường.
```

Macros giải:
```rust
println!("a={}, b={}, c={}", 1, 2, 3);
// In ra: a=1, b=2, c=3
```

`println!` nhận **số lượng tham số bất kỳ** — không function nào làm được.

### Giới hạn 2: Type của tham số phải biết trước

Function generic vẫn yêu cầu trait bounds. Macros nhận **token** — có thể là gì cũng được.

```rust
macro_rules! print_anything {
    ($x:expr) => { println!("{:?}", $x) };
}
print_anything!(42);
print_anything!("hello");
print_anything!(vec![1,2,3]);
// Mỗi cái gen ra code khác nhau
```

### Giới hạn 3: Không thể sinh code dựa trên struct definition

```rust
#[derive(Debug)]
struct Point { x: i32, y: i32 }
```

`#[derive(Debug)]` đọc field `x`, `y` rồi sinh `impl Debug`. Function không làm được — function chỉ nhận giá trị runtime, không nhận **kiểu** dưới dạng dữ liệu.

### Giới hạn 4: Không thể tạo control flow mới

```rust
let _ = matches!(value, Some(x) if x > 0);
```

`matches!` là macro — nó "tạo ra" một pattern matching expression mà nếu là function, bạn phải truyền closure.

## 1.2 Macros là gì?

**Macros = code sinh code** tại compile time.

```
   Input: Token stream của bạn viết
              │
              ▼
   ┌─────────────────────────┐
   │ Macro expansion         │ ← chạy tại compile time
   │ (compiler hoặc proc-    │   (không có runtime cost)
   │  macro của bạn)         │
   └─────────────────────────┘
              │
              ▼
   Output: Token stream khác → compile bình thường
```

Code `println!("hello {}", name)` không tồn tại trong binary cuối. Nó được **expand** ra:
```rust
{
    use ::std::io::Write as _;
    let mut buf = ::std::fmt::Arguments::new_v1(&["hello "], &[
        ::std::fmt::ArgumentV1::new_display(&name)
    ]);
    ::std::io::_print(buf);
}
```

→ Macros là **0-cost runtime** (chỉ tốn compile time).

## 1.3 So sánh với các ngôn ngữ khác

| Ngôn ngữ | Macro |
|----------|-------|
| **C/C++** | `#define`, `#include`: text replacement đơn giản, dễ bug (vd `#define MAX(a,b) a>b?a:b` → `MAX(i++,j)` sai) |
| **Lisp/Scheme** | Macros là first-class, dùng AST trực tiếp (homoiconicity). Rất powerful nhưng cú pháp khó học |
| **Scala** | Macros = chạy code Scala tại compile time. Powerful nhưng đã deprecate |
| **Template Haskell** | Quote/splice code. Rất sâu nhưng complex |
| **Java** | Annotation processor — chỉ tạo file mới, không modify code có sẵn |
| **TypeScript** | Không macros. Một số crate dùng `tsc transformer` (hack) |
| **Rust** | 2 loại: declarative (token pattern matching) + procedural (Rust code chạy compile time) |

Rust macros mạnh hơn C nhiều (token-aware, hygiene) nhưng đơn giản hơn Lisp (không homoiconic).

## 1.4 Một số macros bạn dùng hằng ngày

```rust
println!("..."); eprintln!(); format!();           // I/O
vec![1, 2, 3]; vec![0; 100];                       // Collections
assert!(); assert_eq!(); assert_ne!();             // Test
panic!(); todo!(); unimplemented!(); unreachable!(); // Control
matches!(value, pattern);                          // Pattern
write!(buf, "..."); writeln!();                    // Format
include_str!("file.txt"); include_bytes!();        // Build-time
env!("CARGO_PKG_VERSION"); option_env!();          // Env
concat!("a", "b", 1, 2.0);                         // Concat literals
stringify!(x + y);                                 // Stringify token

// Proc macros:
#[derive(Debug, Clone, PartialEq, Hash, Serialize)]
#[tokio::main] async fn main() { ... }
#[test] fn test_foo() { ... }
sqlx::query!("SELECT * FROM users WHERE id = ?", id);
```

Khả năng cao bạn đã dùng 20+ macros mà không nhận ra!

---

# Tầng 2: 2 loại Macros trong Rust

## 2.1 Tổng quan

```
                     RUST MACROS
                         │
              ┌──────────┴──────────┐
              │                     │
       Declarative           Procedural
       (macro_rules!)        (proc-macro)
              │                     │
       Token pattern         Rust code chạy
       matching              tại compile time
              │                     │
              │              ┌──────┴──────┐
              │              │      │      │
              │            Derive Attribute Function-like
              │           (#[derive])  (#[foo])  (foo!())
              │
       Single .rs file       Crate riêng kiểu
                            proc-macro = true
```

## 2.2 Declarative macros (macro_rules!)

```rust
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
}

fn main() {
    say_hello!();  // expand thành println!("Hello!");
}
```

- Định nghĩa với `macro_rules!`
- Nằm trong file `.rs` bình thường
- Pattern matching trên **token tree** của input
- Hygiene tự động (sẽ giải thích Tầng 6)
- Dễ học, đủ mạnh cho 80% nhu cầu

## 2.3 Procedural macros (proc-macro)

```rust
// trong crate proc-macro = true
use proc_macro::TokenStream;

#[proc_macro_derive(MyTrait)]
pub fn derive_my_trait(input: TokenStream) -> TokenStream {
    // viết Rust code để parse và sinh code
    // ...
}
```

- Là **chương trình Rust** chạy tại compile time
- Phải ở crate riêng (loại `proc-macro = true` trong Cargo.toml)
- Nhận `TokenStream` input, trả `TokenStream` output
- Khả năng cao: phân tích AST, sinh code phức tạp
- Phổ biến crate: `syn` (parse), `quote` (sinh code), `proc-macro2`

## 2.4 So sánh

| Aspect | `macro_rules!` | proc-macro |
|--------|----------------|------------|
| Định nghĩa | Cùng file | Crate riêng |
| Pattern | Token tree matching | Rust code |
| Quyền hạn | Hạn chế | Toàn bộ Rust |
| Học khó | Trung bình | Khó |
| Compile time | Nhanh | Chậm hơn (cần build crate proc-macro) |
| Error message | Khá tốt | Tuỳ implementation |
| Use cases | DSL nhỏ, code repetition | derive, attribute, code generation phức tạp |

**Quy tắc senior**:
- Bắt đầu với `macro_rules!` (đủ cho phần lớn nhu cầu)
- Chỉ đi proc-macro khi:
  - Cần đọc struct fields (derive)
  - Cần modify function definitions (attribute)
  - Cần generate code phức tạp dựa trên parsing
  - macro_rules! không xử lý được token

---

# Tầng 3: macro_rules! — Pattern matching trên token

## 3.1 Cú pháp cơ bản

```rust
macro_rules! macro_name {
    // Rule 1
    (pattern1) => {
        expansion1
    };
    // Rule 2
    (pattern2) => {
        expansion2
    };
    // ...
}
```

- Mỗi rule = pattern → expansion
- Compiler thử rules theo thứ tự
- Match đầu tiên thắng

## 3.2 Ví dụ đầu tiên — Macro không tham số

```rust
macro_rules! print_hi {
    () => {
        println!("Hi!");
    };
}

print_hi!();        // "Hi!"
print_hi![];        // cùng kết quả — () [] {} đều được
print_hi! {};       // cũng OK
```

Rust cho phép gọi macro với `()`, `[]`, hoặc `{}`. Convention:
- `()` cho hầu hết: `println!()`, `vec!()`, `format!()`
- `[]` cho collection-like: `vec![1,2,3]`, `assert![...]` (ít gặp)
- `{}` cho block-like / multi-line: `thread::spawn { ... }` (rare)

## 3.3 Tham số đơn

```rust
macro_rules! double {
    ($x:expr) => {
        $x * 2
    };
}

let n = double!(5);  // 10
```

- `$x` là **metavariable**, bắt đầu với `$`
- `:expr` là **fragment specifier** — chỉ định loại token (expression ở đây)
- Trong expansion, `$x` được thay bằng token match được

## 3.4 Nhiều tham số

```rust
macro_rules! sum_three {
    ($a:expr, $b:expr, $c:expr) => {
        $a + $b + $c
    };
}

let s = sum_three!(1, 2, 3);  // 6
```

## 3.5 Multiple rules

```rust
macro_rules! greet {
    () => {
        println!("Hello, stranger!");
    };
    ($name:expr) => {
        println!("Hello, {}!", $name);
    };
    ($name:expr, $greeting:expr) => {
        println!("{}, {}!", $greeting, $name);
    };
}

greet!();                    // "Hello, stranger!"
greet!("Alice");             // "Hello, Alice!"
greet!("Bob", "Good morning"); // "Good morning, Bob!"
```

3 rules → "overload" theo số lượng/loại tham số. Function thông thường không làm được.

## 3.6 Đáy của Token Tree

Pattern matching trong macros KHÔNG phải string matching. Compiler đã **lex** input thành **token tree** trước khi match:

```
Input: foo(1 + 2, "hello")
              │
              ▼ Lex
   ┌──────────────────────────────────────────┐
   │ Ident: foo                               │
   │ Group(()):                               │
   │   ├─ Literal: 1                          │
   │   ├─ Punct: +                            │
   │   ├─ Literal: 2                          │
   │   ├─ Punct: ,                            │
   │   └─ Literal: "hello"                    │
   └──────────────────────────────────────────┘
              │
              ▼ Macro pattern matching
```

Vì vậy macros **không thể** match arbitrary strings — chỉ match token theo cấu trúc.

---

# Tầng 4: Fragment specifiers — Phân loại token

## 4.1 Danh sách đầy đủ

| Specifier | Match | Ví dụ |
|-----------|-------|-------|
| `expr` | Expression | `1 + 2`, `foo()`, `if x { y } else { z }` |
| `ident` | Identifier | `foo`, `MyStruct`, `_x` |
| `ty` | Type | `i32`, `Vec<u8>`, `&str` |
| `pat` | Pattern | `Some(x)`, `_`, `1..=10` |
| `path` | Path | `std::vec::Vec`, `foo::bar` |
| `stmt` | Statement | `let x = 5;`, `foo();` |
| `block` | Block (`{ ... }`) | `{ a; b; c }` |
| `item` | Item (fn, struct, ...) | `fn foo() {}`, `struct S;` |
| `meta` | Inner of `#[...]` | `derive(Debug)`, `cfg(test)` |
| `tt` | Single token tree | bất cứ gì (catch-all) |
| `lifetime` | Lifetime | `'a`, `'static` |
| `vis` | Visibility | `pub`, `pub(crate)`, "" (private) |
| `literal` | Literal | `42`, `"hi"`, `3.14`, `true` |
| `pat_param` | Pattern (param-restricted) | (rare, dùng trong function params) |

## 4.2 Chọn specifier đúng

```rust
// Macro nhận một type và sinh hàm
macro_rules! make_getter {
    ($name:ident, $ty:ty) => {
        fn $name(&self) -> &$ty {
            &self.$name
        }
    };
}

struct User { name: String, age: u32 }
impl User {
    make_getter!(name, String);  // sinh: fn name(&self) -> &String { &self.name }
    make_getter!(age, u32);      // sinh: fn age(&self) -> &u32 { &self.age }
}
```

Quan trọng: chọn specifier hẹp nhất phù hợp. Nếu cần ident, dùng `ident` (không phải `expr`) — vì `ident` chỉ match identifier, không match expression — error sẽ rõ hơn.

## 4.3 `tt` — Catch-all token tree

`tt` match **bất cứ single token hoặc group `(...)`/`[...]`/`{...}`** nào.

```rust
macro_rules! pass_through {
    ($($args:tt)*) => {
        println!($($args)*);
    };
}

pass_through!("x={}, y={}", x, y);
```

`$($args:tt)*` = "match nhiều `tt` liên tục". Đây là cách `vec![]`, `println!()` accept tham số bất kỳ.

⚠️ `tt` mất type info. Nếu chắc input là expression, dùng `expr` để error message tốt hơn.

## 4.4 Match macro signature theo nhiều cách

```rust
macro_rules! demo {
    // Match expression
    ($e:expr) => { println!("expr: {}", $e) };
    // Match block
    ($b:block) => { println!("block: {:?}", $b) };
    // Match item
    ($i:item) => { println!("item: {:?}", stringify!($i)) };
}

demo!(1 + 2);              // expr
demo!({ let x = 5; x });   // block
demo!(struct Foo;);        // item
```

Multiple rules với specifier khác nhau → macro xử lý cấu trúc khác nhau.

## 4.5 Specifier thay đổi qua các Rust edition

Có một số nuance: trong edition cũ, `expr` không match `let _ = ...`. Trong edition 2021+, fragment specifier mạnh hơn. Đọc Reference của Rust để biết chính xác.

---

# Tầng 5: Repetition — `$( )*`, `$( ),*`, `$( );?`

## 5.1 Cú pháp repetition

```
$( pattern ) sep rep
```

- `pattern` — pattern lặp lại
- `sep` (optional) — separator (vd `,`, `;`)
- `rep` — quantifier:
  - `*` — 0 hoặc nhiều
  - `+` — 1 hoặc nhiều
  - `?` — 0 hoặc 1

## 5.2 Ví dụ cơ bản

```rust
macro_rules! sum {
    ($($x:expr),*) => {
        {
            let mut total = 0;
            $(
                total += $x;
            )*
            total
        }
    };
}

let s = sum!();           // 0
let s = sum!(1, 2, 3, 4); // 10
```

Phân tích:
- Pattern: `$($x:expr),*` — 0+ expressions, ngăn cách bằng `,`
- Expansion: `$( total += $x; )*` — lặp lại body cho mỗi `$x`

Generated code cho `sum!(1, 2, 3)`:
```rust
{
    let mut total = 0;
    total += 1;
    total += 2;
    total += 3;
    total
}
```

## 5.3 Trailing separator

```rust
macro_rules! vec_clone {
    // Trailing comma OK (note: $( ),* )
    ($($x:expr),* $(,)?) => {
        vec![$($x),*]
    };
}

vec_clone!(1, 2, 3);
vec_clone!(1, 2, 3,);  // trailing comma OK
```

`$(,)?` = optional trailing comma. Idiom phổ biến.

## 5.4 Nested repetition

```rust
macro_rules! matrix {
    ($($($x:expr),*);*) => {
        vec![$(vec![$($x),*]),*]
    };
}

let m = matrix!(1, 2, 3; 4, 5, 6; 7, 8, 9);
// = vec![vec![1,2,3], vec![4,5,6], vec![7,8,9]]
```

`$($($x:expr),*);*` = list of (list of exprs separated by `,`) separated by `;`.

## 5.5 Tham chiếu metavar trong repetition

Trong expansion, `$x` phải nằm trong cùng level repetition như khai báo:

```rust
macro_rules! pairs {
    ($($k:expr => $v:expr),*) => {
        $(
            println!("{} = {}", $k, $v);  // $k, $v trong $( )*
        )*
    };
}

pairs!("a" => 1, "b" => 2, "c" => 3);
```

Sinh code:
```rust
println!("{} = {}", "a", 1);
println!("{} = {}", "b", 2);
println!("{} = {}", "c", 3);
```

## 5.6 `$#` và `${count}` (nightly)

Đang phát triển:
```rust
${count($x)}  // số lần lặp
${index()}    // index hiện tại
${ignore($x)} // expansion không emit gì cho $x
```

Chưa stable. Stable Rust workaround: dùng `0_usize $(+ 1 + ignore!($x))*` để đếm — hacky.

## 5.7 Implement `vec!` từ đầu

```rust
macro_rules! my_vec {
    // Empty
    () => {
        Vec::new()
    };
    // Comma separated list (with optional trailing comma)
    ($($x:expr),+ $(,)?) => {
        {
            let mut v = Vec::new();
            $( v.push($x); )+
            v
        }
    };
    // [value; count] form
    ($value:expr ; $count:expr) => {
        {
            let mut v = Vec::new();
            let val = $value;
            for _ in 0..$count {
                v.push(val.clone());
            }
            v
        }
    };
}

let v1: Vec<i32> = my_vec!();
let v2 = my_vec!(1, 2, 3);
let v3 = my_vec!(1, 2, 3,);
let v4 = my_vec!(0; 10);
```

Đây gần như chính xác `vec!` của std (tối ưu hơn dùng `Vec::with_capacity`).

---

# Tầng 6: Hygiene — Vì sao macro Rust không phá scope

## 6.1 Vấn đề: Variable capture trong macros

Trong C `#define`:
```c
#define SWAP(a, b) { int tmp = a; a = b; b = tmp; }

int tmp = 5;
SWAP(tmp, x);  // Đại loạn! tmp bị shadow, x KHÔNG được swap với original tmp
```

C macros là **text substitution** — không hiểu scope. Đây là bug famous.

## 6.2 Rust hygiene — Token có identity

Rust macros **hygienic**: identifier sinh trong macro **không xung đột** với identifier ở scope gọi macro.

```rust
macro_rules! make_x {
    () => {
        let x = 42;
    };
}

fn main() {
    let x = 10;
    make_x!();
    println!("{}", x);  // In ra 10, KHÔNG phải 42
}
```

Lý do: `x` trong `make_x!` được compiler "đánh dấu" thuộc về scope của macro definition, không phải caller. Đây là **hygiene**.

## 6.3 Khi nào break hygiene? — `$crate`

Nếu macro reference symbol từ crate định nghĩa, dùng `$crate`:

```rust
// Trong crate "mylib"
pub fn helper() {}

#[macro_export]
macro_rules! call_helper {
    () => {
        $crate::helper();   // Tham chiếu helper của "mylib"
    };
}
```

User dùng `call_helper!()` ở crate khác → expand thành `mylib::helper()` (đúng path). Không có `$crate` → user phải `use mylib::helper` trước.

## 6.4 `#[macro_export]` — Export macro

```rust
#[macro_export]
macro_rules! my_macro {
    () => { ... };
}
```

Không có `#[macro_export]` → macro chỉ visible trong cùng crate.
Có → user có thể `use mycrate::my_macro;` hoặc gọi `mycrate::my_macro!()`.

## 6.5 Hygiene không hoàn toàn — Caveat

Rust hygiene là **partial**:
- ✅ Identifier (`x`, `tmp`) — fully hygienic
- ⚠️ Item path (function, type) — không hygiene, dùng `$crate` để fix
- ⚠️ Lifetime — không hoàn toàn hygiene (edge cases)

Đa số code hằng ngày, hygiene "just works".

---

# Tầng 7: Macro trong macro, debug, scoping

## 7.1 Macro recursion

```rust
macro_rules! pow {
    ($base:expr, 0) => { 1 };
    ($base:expr, $n:expr) => {
        $base * pow!($base, $n - 1)
    };
}

let r = pow!(2, 5);
```

Lưu ý: số lần recurse có giới hạn (mặc định 128, set bằng `#![recursion_limit = "256"]`).

## 7.2 Helper macros

```rust
macro_rules! helper {
    ($x:expr) => { $x * 2 };
}

macro_rules! main_macro {
    ($x:expr) => {
        helper!($x) + 1
    };
}
```

Macros gọi nhau bình thường. Order definition quan trọng (macros must be defined before use, không như functions).

## 7.3 Tip debug: cargo expand

```bash
cargo install cargo-expand
cargo expand              # Xem code sau expand
cargo expand --bin myapp
cargo expand foo::bar     # Xem một module cụ thể
```

Output là code đã expand tất cả macros. Cực kỳ hữu ích khi:
- Debug macro của bạn
- Hiểu macro của crate khác
- Học cách `serde`, `tokio` sinh code

## 7.4 trace_macros! (nightly)

```rust
#![feature(trace_macros)]

trace_macros!(true);
my_macro!(...);    // In ra mỗi step expansion
trace_macros!(false);
```

Chỉ nightly. Hữu ích cho debug deep recursion.

## 7.5 Scoping rules

```rust
// File scope
macro_rules! local_macro { ... }
// → chỉ visible trong file/module này

#[macro_export]
macro_rules! exported {
    ...
}
// → exported tới root của crate
// → user crate khác dùng: use mycrate::exported;

#[macro_use]
extern crate some_crate;   // Old edition; nay không cần
```

Edition 2018+: `use my_crate::macro_name;` thay vì `#[macro_use]`.

## 7.6 Macro names share namespace với items

```rust
fn foo() {}
macro_rules! foo { ... }   // ERROR: name conflict
```

Nhưng macros và functions thường có suffix `!`:
```rust
foo();      // calls function
foo!();     // calls macro
// → có thể coexist nếu compiler không nhầm lẫn (edge case)
```

Convention: macro tên có suffix nghĩa rõ (`make_vec!` không xung đột với `make_vec` function).

## 7.7 Macros sinh impl block

```rust
macro_rules! impl_display {
    ($t:ty, $fmt:expr) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, $fmt, self)
            }
        }
    };
}

struct Celsius(f64);
impl_display!(Celsius, "{:.1}°C");
```

Macros generate items (not just expressions). Đây là cách scale code cực mạnh.

---

# Tầng 8: Procedural Macros — Cấp độ tiếp theo

## 8.1 Khi macro_rules! không đủ

Vấn đề `macro_rules!`:
- Không thể đọc fields của struct
- Không thể "parse" Rust syntax phức tạp (vd: nested generic)
- Không thể tạo nhiều item ở các scope khác nhau
- Error message khó customize

Procedural macros = **chạy Rust code** lúc compile để **transform tokens**.

## 8.2 Setup proc-macro crate

Proc-macro phải ở crate riêng. Cargo.toml:

```toml
[package]
name = "my-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true       # ← key này

[dependencies]
proc-macro2 = "1"
syn = { version = "2", features = ["full"] }
quote = "1"
```

3 crate "bộ ba" của proc-macro:
- **proc-macro2** — tokenstream wrapper (cross compile boundary)
- **syn** — parse TokenStream thành Rust AST
- **quote** — sinh TokenStream từ Rust code

## 8.3 3 loại proc-macro

```
                    PROCEDURAL MACROS
                          │
       ┌──────────────────┼─────────────────────┐
       │                  │                     │
   Derive            Attribute          Function-like
   #[derive(...)]    #[my_attr]         my_macro!()
       │                  │                     │
   Apply lên          Apply lên           Gọi như function
   struct/enum        bất cứ item        nhưng nhận token
                                          stream
       │                  │                     │
       │              Modify or             Custom DSL,
   Tự sinh impl       wrap function/         code gen
   blocks             struct/...
   (vd Debug,
   Clone, Serde)
```

## 8.4 Skeleton chung

Mọi proc-macro có signature:
```rust
use proc_macro::TokenStream;

#[proc_macro_derive(MyDerive)]  // hoặc proc_macro_attribute / proc_macro
pub fn name(input: TokenStream) -> TokenStream {
    // 1. Parse input thành AST
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    
    // 2. Phân tích / transform AST
    let name = &ast.ident;
    
    // 3. Generate output tokens với quote!
    let output = quote::quote! {
        impl MyTrait for #name {
            fn hello() { println!("Hello from {}", stringify!(#name)); }
        }
    };
    
    // 4. Return TokenStream
    output.into()
}
```

---

# Tầng 9: Derive Macros — Tự sinh impl trait

## 9.1 Setup derive macro

```rust
// Trong crate my-macros (proc-macro = true)
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(HelloWorld)]
pub fn derive_hello_world(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    
    let expanded = quote! {
        impl HelloWorld for #name {
            fn hello_world() {
                println!("Hello from {}", stringify!(#name));
            }
        }
    };
    
    TokenStream::from(expanded)
}
```

User crate:
```rust
use my_macros::HelloWorld;

trait HelloWorld { fn hello_world(); }

#[derive(HelloWorld)]
struct Pancakes;

fn main() {
    Pancakes::hello_world();  // "Hello from Pancakes"
}
```

## 9.2 Đọc fields của struct

```rust
#[proc_macro_derive(FieldNames)]
pub fn derive_field_names(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    
    // Match struct data
    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("FieldNames only works on structs"),
    };
    
    let field_names: Vec<_> = fields.iter()
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect();
    
    let expanded = quote! {
        impl #name {
            pub fn field_names() -> Vec<&'static str> {
                vec![#(#field_names),*]
            }
        }
    };
    
    expanded.into()
}
```

Sử dụng:
```rust
#[derive(FieldNames)]
struct Point { x: i32, y: i32, z: i32 }

assert_eq!(Point::field_names(), vec!["x", "y", "z"]);
```

## 9.3 Derive macro với attributes

```rust
#[derive(FieldNames)]
#[field_names(uppercase)]   // ← helper attribute
struct Point { 
    x: i32,
    #[field_names(skip)]    // ← per-field attribute
    secret: String,
}
```

Trong proc-macro:
```rust
#[proc_macro_derive(FieldNames, attributes(field_names))]
pub fn derive_field_names(input: TokenStream) -> TokenStream {
    // Parse the attributes
    for attr in &input.attrs {
        if attr.path().is_ident("field_names") {
            // ...
        }
    }
    // ...
}
```

`attributes(name)` declare helper attributes mà derive này hiểu.

## 9.4 Ví dụ thật: cách `thiserror` work

```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("not found: {0}")]
    NotFound(String),
    
    #[error("IO")]
    Io(#[from] std::io::Error),
}
```

Bên trong, `thiserror`:
1. Parse enum
2. Cho mỗi variant, đọc `#[error("...")]`
3. Generate `impl Display`
4. Cho variant có `#[from]`, generate `impl From<InnerType>`
5. Generate `impl Error` với `source()` từ inner field

Đây là use case kinh điển của derive macro: **đọc structure → sinh trait impl**.

## 9.5 quote! — DSL sinh TokenStream

`quote!` cho phép viết Rust code template với `#var` để inject value:

```rust
let name = quote::format_ident!("Foo");
let expanded = quote! {
    struct #name {
        x: i32,
    }
    
    impl #name {
        fn new() -> Self { Self { x: 0 } }
    }
};
```

`#var` thay bằng giá trị `var`. `#(#vars),*` cho lặp.

```rust
let fields = vec!["a", "b", "c"];
let q = quote! {
    fn list() -> Vec<&'static str> {
        vec![#(#fields),*]   // expand: vec!["a", "b", "c"]
    }
};
```

## 9.6 Span và error reporting

```rust
let err_span = field.span();   // span của token

// Generate error tied to specific source location
let error = syn::Error::new(err_span, "field cannot be ignored")
    .to_compile_error();
return error.into();
```

`compile_error!` macro hoặc `syn::Error` → user thấy error point đúng vị trí. Đây là làm cho error UX tốt.

---

# Tầng 10: Attribute và Function-like proc macros

## 10.1 Attribute macros

```rust
#[proc_macro_attribute]
pub fn my_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    // attr: token trong #[my_attr(here)]
    // item: function/struct/... attached
    
    let fn_item = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &fn_item.sig.ident;
    let fn_body = &fn_item.block;
    
    let expanded = quote! {
        fn #fn_name() {
            println!("Entering {}", stringify!(#fn_name));
            let result = (|| #fn_body)();
            println!("Leaving {}", stringify!(#fn_name));
            result
        }
    };
    
    expanded.into()
}
```

Sử dụng:
```rust
#[my_attr]
fn do_work() {
    println!("Working");
}
// expand thành function in "Entering" / "Leaving"
```

## 10.2 Ví dụ thật: `#[tokio::main]`

```rust
#[tokio::main]
async fn main() {
    println!("Hello");
}

// Expand đại loại thành:
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("Hello");
        });
}
```

`#[tokio::main]` là attribute macro:
- Nhận `async fn main()`
- Wrap body trong runtime block_on
- Return non-async `fn main()`

Bây giờ bạn hiểu cách macro này hoạt động — không có ma thuật.

## 10.3 Attribute với arguments

```rust
#[my_attr(level = "debug", skip_if(empty))]
fn do_thing() {}
```

Parse trong macro:
```rust
let args = parse_macro_input!(attr as syn::AttributeArgs);
// hoặc với syn 2:
let args = parse_macro_input!(attr with parse_my_args);
```

## 10.4 Function-like proc macros

```rust
#[proc_macro]
pub fn make_function(input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(input as syn::Ident);
    
    let expanded = quote! {
        fn #name() {
            println!("I am {}", stringify!(#name));
        }
    };
    
    expanded.into()
}
```

Sử dụng:
```rust
my_macros::make_function!(hello);  // sinh fn hello() { ... }
my_macros::make_function!(world);  // sinh fn world() { ... }

fn main() {
    hello();
    world();
}
```

## 10.5 Ví dụ thật: `sqlx::query!`

```rust
let users = sqlx::query!(
    "SELECT id, name FROM users WHERE active = $1",
    true
).fetch_all(&pool).await?;
```

`sqlx::query!` là function-like macro. Tại compile time:
1. Connect tới DB
2. Phân tích SQL string
3. Verify columns/types khớp với struct fields
4. Sinh code typed return

→ SQL injection impossible, type mismatch caught at compile time. Đây là **power level cuối cùng** của proc-macro.

## 10.6 So sánh khả năng

| Aspect | macro_rules! | Function-like proc | Attribute | Derive |
|--------|--------------|--------------------|-----------|--------|
| Đọc fields struct | ❌ | ⚠️ Một chiều, không tự apply | ✅ | ✅ |
| Modify item | ❌ | ❌ | ✅ | ❌ (chỉ thêm) |
| DSL ngoài Rust syntax | ⚠️ Hạn chế | ✅ Tự do | ❌ | ❌ |
| Compile time | Nhanh | Chậm hơn | Chậm hơn | Chậm hơn |
| Khả năng error | Trung bình | Cao | Cao | Cao |

---

# Tầng 11: Hệ sinh thái — syn, quote, proc-macro2

## 11.1 proc-macro vs proc-macro2

`proc-macro` (std lib) chỉ work trong proc-macro crates — không test được, không pass cross-crate.

`proc-macro2` là wrapper:
- Work everywhere (test files, regular crates)
- API tương thích
- Crate `quote` và `syn` dùng `proc-macro2::TokenStream`

Pattern chuẩn:
```rust
use proc_macro::TokenStream;       // std
use proc_macro2::TokenStream as TokenStream2;  // wrapper

#[proc_macro_derive(Foo)]
pub fn derive_foo(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();   // convert
    let output2 = real_implementation(input2);  // logic ở đây, testable
    output2.into()
}

fn real_implementation(input: TokenStream2) -> TokenStream2 {
    // unit test được hàm này
    // ...
}
```

## 11.2 syn — Parser tốt nhất Rust

`syn` parse `TokenStream` thành AST type-safe:

```rust
let input = syn::parse2::<syn::DeriveInput>(tokens)?;
let input = syn::parse_macro_input!(tokens as syn::DeriveInput);

// Types syn cung cấp:
syn::DeriveInput      // input của derive
syn::ItemFn           // function definition
syn::ItemStruct       // struct
syn::ItemEnum         // enum
syn::Type             // any type
syn::Expr             // any expression
syn::Pat              // any pattern
syn::Path             // path (foo::bar::baz)
// ... ~100 types
```

`syn` có `Parse` trait — bạn có thể parse custom syntax:

```rust
struct MyMacroInput {
    name: syn::Ident,
    eq: syn::Token![=],
    value: syn::LitInt,
}

impl syn::parse::Parse for MyMacroInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MyMacroInput {
            name: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

// Bây giờ parse được: foo = 42
```

## 11.3 quote — Reverse direction

`quote!` template DSL cho TokenStream:

```rust
let name = syn::Ident::new("Foo", proc_macro2::Span::call_site());
let fields = vec!["x", "y", "z"];
let field_idents: Vec<_> = fields.iter()
    .map(|n| syn::Ident::new(n, proc_macro2::Span::call_site()))
    .collect();

let tokens = quote! {
    struct #name {
        #( pub #field_idents: i32 ),*
    }
};
```

Output:
```rust
struct Foo {
    pub x: i32,
    pub y: i32,
    pub z: i32
}
```

`#var` substitute; `#( ... )*` lặp.

## 11.4 Span và source location

```rust
use proc_macro2::Span;

let span = Span::call_site();      // location where macro called
let span = Span::mixed_site();     // hybrid hygiene (Rust 1.45+)
let span = item.span();            // span của token cụ thể

let ident_with_span = syn::Ident::new("x", span);
```

Span quan trọng cho error reporting — error sẽ point đúng chỗ user gõ.

## 11.5 Testing proc-macros

Test "bên trong" proc-macro crate khó (vì `TokenStream` chỉ work compile-time). Patterns:

### Pattern 1: Test internal function với proc-macro2

```rust
// trong proc-macro crate
fn my_impl(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // ...
}

#[proc_macro]
pub fn my_macro(input: TokenStream) -> TokenStream {
    my_impl(input.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    
    #[test]
    fn test_my_impl() {
        let input = quote! { struct Foo; };
        let output = my_impl(input);
        assert!(output.to_string().contains("impl"));
    }
}
```

### Pattern 2: trybuild (crate)

`trybuild` test compile-time behavior:

```rust
#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/01-pass.rs");
    t.compile_fail("tests/02-fail.rs");  // expect compile error
}
```

Pass `01-pass.rs` qua compile, fail `02-fail.rs` với error chính xác. Đây là cách `serde`, `thiserror` test.

---

# Tầng 12: Khi nào dùng / không dùng macros

## 12.1 ✅ Khi nào dùng macros

### 1. Variable-arity API
```rust
println!("a={}, b={}", a, b);
vec![1, 2, 3, 4];
```
Function không làm được. Macro cần thiết.

### 2. Repetitive boilerplate
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct User { name: String, age: u32 }
```
Hand-writing 6 impl khổng lồ → derive macro 1 dòng.

### 3. DSL bên trong Rust
```rust
html! {
    <div class="foo">
        <p>{ format!("Count: {}", n) }</p>
    </div>
}
```
Cú pháp ngoài Rust → cần function-like proc macro (`html!` của yew, `view!` của leptos).

### 4. Compile-time validation
```rust
sqlx::query!("SELECT * FROM users WHERE id = $1", id)
```
SQL check tại compile → cần connect DB tại compile → proc macro.

### 5. Code generation từ external source
```rust
include_proto!("schema.proto");  // sinh struct từ .proto file
```

### 6. Phải có behavior đặc biệt
```rust
assert!(x > 0);   // muốn message in expression chứ không chỉ value → cần macro
```

## 12.2 ❌ Khi nào KHÔNG dùng macros

### 1. Function bình thường đã làm được
```rust
// ❌
macro_rules! double { ($x:expr) => { $x * 2 } }

// ✅
fn double(x: i32) -> i32 { x * 2 }
```
Generic + trait đủ cho phần lớn case.

### 2. Trait đủ
```rust
// ❌
macro_rules! print_them { ... }

// ✅
trait Printable { fn print(&self); }
impl<T: Debug> Printable for T { fn print(&self) { println!("{:?}", self); } }
```

### 3. Code phức tạp nhưng không cần meta-programming
Đừng dùng macro để "tiết kiệm" 2 dòng code. Macro tăng cognitive load — code reviewer phải hiểu macro mới hiểu code.

### 4. Trong hot path / performance critical
Macros expand → có thể tạo nhiều code → binary size lớn → instruction cache miss. Inline function thường tốt hơn.

## 12.3 Cây quyết định

```
   Code lặp lại hoặc cần code gen?
            │
       ┌────┴────┐
      Có       Không → dùng function/trait
       │
       ▼
   Function + generic đủ?
       │
   ┌───┴───┐
  Có      Không
   │       │
   │       ▼
   │   macro_rules! đủ?
   │       │
   │   ┌───┴───┐
   │  Có      Không
   │   │       │
   │   │       ▼
   │   │   Đọc struct/parse AST?
   │   │       │
   │   │   ┌───┴───┐
   │   │  Có      Không (DSL ngoài Rust syntax)
   │   │   │       │
   │   │   ▼       ▼
   │   │ derive   function-like proc
   │   │   /
   │   │ attribute
   │   │ proc
   │   ▼
   │ macro_rules!
   ▼
 function
```

---

# Tầng 13: Antipatterns và Pitfalls

## 13.1 ❌ Quá phụ thuộc vào macros khi function đủ

```rust
// Antipattern
macro_rules! square { ($x:expr) => { $x * $x } };

square!(2 + 3);
// expand: 2 + 3 * 2 + 3 = 11 (WRONG! Expected 25)
```

Macros không hiểu operator precedence. Fix: `( $x:expr ) => { ($x) * ($x) }`.

**Tốt hơn**: dùng function `fn square(x: i32) -> i32 { x * x }`. Function gọi `square(2 + 3)` evaluate `5` trước rồi square.

## 13.2 ❌ Macros expand quá nhiều, làm binary phình

```rust
macro_rules! big_inline {
    ($x:expr) => { /* 100 dòng inline code */ }
}

// Gọi 100 lần → 10000 dòng expanded
```

Mỗi callsite duplicate code → binary size tăng. Đặc biệt thấy ở `derive(...)` cho struct lớn (mỗi field sinh code).

Fix: macro chỉ wrap function call, body của function tại 1 nơi:

```rust
macro_rules! call_helper {
    ($x:expr) => { my_lib::__helper($x) };  // delegate vào function
}
```

## 13.3 ❌ Macro tên xấu, không sufix

```rust
macro_rules! check { ... }   // ← không gợi ý gì
macro_rules! do_thing { ... }
```

Macros mà gọi với `!` đã ngầm cho biết là macro. Tên nên gợi ý chức năng cụ thể: `assert_sorted!`, `define_module!`.

## 13.4 ❌ Hygiene mistakes

```rust
macro_rules! count_down {
    () => {
        let mut i = 10;          // ← i thuộc macro scope
        while i > 0 {
            println!("{}", i);
            i -= 1;
        }
    };
}

fn main() {
    let i = 5;          // ← i bên ngoài
    count_down!();      // OK, không ảnh hưởng i bên ngoài
    println!("{}", i);  // vẫn 5
}
```

✅ Đây hoạt động đúng nhờ hygiene. Nhưng nếu macro **muốn** reference biến caller, phải nhận làm tham số:

```rust
macro_rules! use_i {
    ($i:ident) => {
        println!("{}", $i);   // dùng $i được pass từ caller
    };
}
let x = 42;
use_i!(x);                   // OK
```

## 13.5 ❌ Confusing error messages

```rust
macro_rules! my_assert {
    ($cond:expr) => {
        if !$cond {
            panic!("Assertion failed");
        }
    };
}

my_assert!(x = 5);   // Mistake: x = 5 is assignment, not comparison
```

Error message có thể không chỉ đúng dòng / không gợi ý chính xác.

Fix: dùng `compile_error!` trong macro nếu detect sai pattern, hoặc dùng proc-macro với span đúng.

## 13.6 ❌ Macro recursion depth limit

```rust
macro_rules! recurse {
    ($n:expr) => {
        if $n > 0 { recurse!($n - 1) }
    };
}

recurse!(200);   // ERROR: recursion limit reached (default 128)
```

Fix: `#![recursion_limit = "256"]` ở crate root. Nhưng nếu cần > 1000, nên refactor.

## 13.7 ❌ Quá nhiều export macros làm pollute namespace

```rust
// trong crate utils
#[macro_export]
macro_rules! print_debug { ... }   // ← export

#[macro_export]
macro_rules! check { ... }          // ← export, generic name

#[macro_export]
macro_rules! validate { ... }       // ← export, conflict potential
```

Tên `check`, `validate` quá chung. User crate có 2 dependencies cùng export `check!` → conflict.

Fix: prefix theo tên crate: `mycrate_check!`, hoặc dùng path syntax `use mycrate::check`.

## 13.8 ❌ Macro tạo IDE confusion

IDE (rust-analyzer) phải expand macros để cung cấp completion. Macros phức tạp:
- Goto definition không work
- Refactoring rename không work
- Type inference giảm chính xác

Fix: prefer trait/function khi possible. Macros chỉ khi giá trị > cognitive cost.

## 13.9 ❌ Proc-macro compile time bùng nổ

Proc-macro thêm:
- Build crate proc-macro (lần đầu chậm)
- Mỗi callsite chạy code Rust để expand

Crate dùng nhiều proc-macro (serde, sqlx, axum) có incremental compile chậm.

Fix:
- Bật `lto = "thin"` chỉ khi release
- Chia code thành crates nhỏ
- Xem `cargo build --timings`

## 13.10 ❌ Macro làm code review khó

```rust
configure_routes! {
    /users => UserController,
    /posts => PostController,
    /admin/* => AdminGuard => AdminController,
}
```

DSL đẹp nhưng reviewer phải biết macro hoạt động sao. Trade-off readability cho người mới.

Quy tắc: macros nên thay thế **repetitive boilerplate**, không nên **hide complexity**.

---

# Tổng kết — Macros như senior

## Nguyên tắc vàng

1. **Macros là dao cuối** — thử function/generic/trait trước
2. **macro_rules! đủ cho 80% nhu cầu** — chỉ proc-macro khi cần đọc AST
3. **Hygiene tự lo, nhưng $crate phải nhớ** — khi export macro
4. **cargo expand là bạn** — debug macros nhanh
5. **Error UX quan trọng** — dùng `compile_error!` / `syn::Error` với đúng span
6. **Naming rõ ràng** — macros export phải có prefix crate
7. **Testing**: tách proc-macro logic ra `fn` thường, test với `proc_macro2::TokenStream`
8. **Don't optimize via macro** — function inline thường đủ

## Bộ ba phải biết khi viết proc-macro

```
   ┌────────────────────────────────────────┐
   │  proc-macro2  — TokenStream cross-crate│
   │  syn          — Parse Rust → AST       │
   │  quote        — Sinh code template DSL │
   └────────────────────────────────────────┘
```

## Tài nguyên đọc thêm

- **The Little Book of Rust Macros** — https://veykril.github.io/tlborm/
- **proc-macro-workshop** (dtolnay) — bài tập viết proc-macro
- **Rust Reference: Macros chapter**
- Source code `thiserror`, `serde`, `derive_more` — học từ best-in-class

---

# Liên kết về memory model

Macros = **compile-time tool**, không có runtime presence. Nhìn theo memory:

| Macro pattern | Effect tại runtime |
|---------------|-------------------|
| `vec![1,2,3]` | Expand thành `Vec::from([1,2,3])` — bình thường |
| `format!("{}", x)` | Sinh `Arguments` struct trên stack, không alloc trừ khi cần |
| `#[derive(Clone)]` | Sinh `clone()` method bình thường — gọi runtime như mọi method |
| `#[tokio::main]` | Sinh `fn main()` non-async, wrap async block trong runtime |
| `println!()` | Sinh `Arguments::new_v1` + `io::_print` call |

→ Macros **không add runtime overhead**. Cost duy nhất: compile time + binary size (nếu inline nhiều).

---

# Lộ trình tiếp theo

Bạn đã có:

```
1. memory-model
2. ownership-borrowing
3. trait
4. generic
5. closure
6. async
7. error-handling
8. macros            ← MỚI
```

Các chủ đề "intermediate" tiếp theo có thể chọn:

- **Testing patterns** — unit, integration, proptest, criterion bench, mocking với `mockall`
- **Unsafe Rust** — raw pointers, FFI, atomic ordering, soundness contracts
- **Logging & Observability** — tracing nâng cao, OpenTelemetry, structured logs
- **Iterator deep dive** — implement Iterator, lazy evaluation, parallel với rayon
- **Smart pointers** — Box, Rc, Arc, Cell, RefCell, Mutex, RwLock deep dive
- **Web framework** — axum/actix realistic project (apply tất cả 8 chủ đề)
- **Database** — sqlx, sea-orm, diesel

Báo chủ đề muốn đi tiếp! 🦀⚡
