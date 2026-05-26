# Macros Rust — Minh Hoạ Trực Quan

> Companion visual cho [macros.md](./macros.md). Đọc song song.

---

## 1. Bức tranh lớn — Macros Universe

```
                          MACROS TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   COMPILE TIME (không có runtime presence)             │
       │   ────────────────────────────────────────             │
       │                                                        │
       │            ┌──────────────────────┐                    │
       │            │  Source code         │                    │
       │            │  with macros         │                    │
       │            └──────────┬───────────┘                    │
       │                       │                                │
       │                       ▼                                │
       │            ┌──────────────────────┐                    │
       │            │  Lexer + Parser      │                    │
       │            │  → TokenStream       │                    │
       │            └──────────┬───────────┘                    │
       │                       │                                │
       │                       ▼                                │
       │     ┌──────────────────────────────────┐               │
       │     │      MACRO EXPANSION             │               │
       │     │                                  │               │
       │     │  ┌─────────────┐ ┌────────────┐  │               │
       │     │  │macro_rules! │ │ proc-macro │  │               │
       │     │  │declarative  │ │ procedural │  │               │
       │     │  └─────────────┘ └────────────┘  │               │
       │     └─────────────┬────────────────────┘               │
       │                   │                                    │
       │                   ▼                                    │
       │            ┌──────────────────────┐                    │
       │            │  Expanded Rust code  │                    │
       │            │  → compile bình thường│                   │
       │            └──────────────────────┘                    │
       │                       │                                │
       │                       ▼                                │
       │            ┌──────────────────────┐                    │
       │            │     BINARY           │  ← runtime,        │
       │            │  (không trace macro) │    không có macro  │
       │            └──────────────────────┘                    │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Tại sao Macros — Function không đủ

```
   GIỚI HẠN CỦA FUNCTION              MACROS GIẢI ĐƯỢC
   ──────────────────────             ─────────────────
   
   ❌ Số tham số cố định               ✅ Variable arity
      fn add(a, b)                       println!("a={}, b={}, c={}", a, b, c);
                                         vec![1, 2, 3, 4, 5]
   
   ❌ Tham số phải biết type           ✅ Nhận TOKEN bất kỳ
      generic + trait bounds             macro nhận expr/ident/ty/...
   
   ❌ Không đọc được struct def        ✅ Phân tích AST tại compile
      runtime mới có giá trị            #[derive(Debug)] đọc fields
   
   ❌ Không tạo được item              ✅ Sinh function/struct/impl
      function chỉ return value         #[tokio::main] sinh fn main()
                                        impl_display!(Foo, "{}") 
                                        → sinh impl block
   
   ❌ Không tạo được DSL                ✅ Cú pháp ngoài Rust
      function gọi qua args             html! { <div>...</div> }
                                        sql! { SELECT * FROM ... }
```

---

## 3. 2 Loại Macros Trong Rust

```
                              MACROS
                                │
              ┌─────────────────┴─────────────────┐
              │                                   │
       DECLARATIVE                          PROCEDURAL
       macro_rules!                         proc-macro
              │                                   │
       Cùng file .rs                        Crate riêng
       Pattern matching                     (proc-macro = true)
       trên token tree                      Rust code at compile
              │                                   │
       Đủ cho 80%                          ┌──────┴──────┐
       use cases                           │      │      │
              │                          Derive Attribute Function
              │                                                like
              │                          #[derive  #[my_attr]   my!()
              │                            (X)]
              │
       Ví dụ:                              Ví dụ:
       vec![], println!()                  #[derive(Debug,
       assert!(), format!()                  Serialize)]
       matches!(), todo!()                 #[tokio::main]
                                           sqlx::query!()
                                           html! {}
```

---

## 4. macro_rules! — Cú pháp cơ bản

```
   macro_rules! macro_name {
       (PATTERN_1) => {
           EXPANSION_1
       };
       (PATTERN_2) => {
           EXPANSION_2
       };
   }
                ▲                    ▲
                │                    │
        Match input            Sinh ra token
        token theo             này tại callsite
        pattern này
   
   
   Compiler thử các rules theo thứ tự — first match wins:
   
   ┌─────────────────────────────────────────────────────┐
   │                                                     │
   │   say_hello!(); ─── input ──┐                       │
   │                             ▼                       │
   │                  ┌────────────────────┐             │
   │                  │ Rule 1: ()         │ ✅ MATCH    │
   │                  │  → println!("Hi!") │ → STOP      │
   │                  └────────────────────┘             │
   │                                                     │
   │   say_hello!("Alice"); ─── input ──┐                │
   │                                    ▼                │
   │                  ┌────────────────────┐             │
   │                  │ Rule 1: ()         │ ❌ no match │
   │                  └────────────────────┘             │
   │                  ┌────────────────────┐             │
   │                  │ Rule 2: ($n:expr)  │ ✅ MATCH    │
   │                  │  → println!("Hello,│             │
   │                  │      {}!", $n)     │             │
   │                  └────────────────────┘             │
   │                                                     │
   └─────────────────────────────────────────────────────┘
```

---

## 5. Token Tree — Đáy của Macros

```
   Input source code:        foo(1 + 2, "hello")
                                       │
                                       ▼ Lex
   
   ┌────────────────────────────────────────────────────┐
   │ Token Tree:                                        │
   │                                                    │
   │   Ident("foo")                                     │
   │   Group "()":                                      │
   │     ├── Literal(1)                                 │
   │     ├── Punct('+')                                 │
   │     ├── Literal(2)                                 │
   │     ├── Punct(',')                                 │
   │     └── Literal("hello")                           │
   │                                                    │
   └────────────────────────────────────────────────────┘
                                       │
                                       ▼
   
   Macros MATCH dựa trên CẤU TRÚC tokens, không phải string.
   Vì vậy macros KHÔNG thể split "hello world" thành 2 chữ —
   "hello world" là 1 literal token.
```

---

## 6. Fragment Specifiers — Cheatsheet

```
   ┌───────────┬─────────────────────┬─────────────────────────┐
   │ Specifier │ MATCH GÌ            │ VÍ DỤ                   │
   ├───────────┼─────────────────────┼─────────────────────────┤
   │ expr      │ Expression          │ 1+2, foo(), if x {y}    │
   │ ident     │ Identifier          │ foo, MyStruct, _x       │
   │ ty        │ Type                │ i32, Vec<u8>, &str      │
   │ pat       │ Pattern             │ Some(x), _, 1..10       │
   │ path      │ Path                │ std::vec::Vec           │
   │ stmt      │ Statement           │ let x=5;                │
   │ block     │ Block { ... }       │ { a; b; c }             │
   │ item      │ Item                │ fn foo(){}, struct S;   │
   │ meta      │ Inner of #[...]     │ derive(Debug)           │
   │ tt        │ Single token tree   │ catch-all (1 token or   │
   │           │                     │ 1 (...) group)          │
   │ lifetime  │ Lifetime            │ 'a, 'static             │
   │ vis       │ Visibility          │ pub, pub(crate), ""     │
   │ literal   │ Literal             │ 42, "hi", 3.14, true    │
   └───────────┴─────────────────────┴─────────────────────────┘
   
   
   📌 Quy tắc: chọn specifier HẸP NHẤT phù hợp.
              → error message tốt hơn cho user.
```

---

## 7. Repetition — `$( )sep*`

```
   Cú pháp repetition:
   ──────────────────
   
            $(  PATTERN  )  SEP  REP
                 │           │     │
                 │           │     └── * = 0+ lần
                 │           │        + = 1+ lần
                 │           │        ? = 0 hoặc 1
                 │           │
                 │           └── Separator (optional: ',' hoặc ';' ...)
                 │
                 └── Pattern lặp lại
   
   
   Ví dụ:
   
   ┌────────────────────────────────────────────────────────────┐
   │ macro_rules! sum {                                         │
   │     ($($x:expr),*) => {                                    │
   │         //  ↑      ↑                                       │
   │         //  │      └── sep=',' rep='*' (0+ items)         │
   │         //  └── pattern: 1 expr với metavar $x             │
   │         {                                                  │
   │             let mut total = 0;                             │
   │             $(                                             │
   │                 total += $x;     // ← repeat body          │
   │             )*                   //   với mỗi $x          │
   │             total                                          │
   │         }                                                  │
   │     };                                                     │
   │ }                                                          │
   └────────────────────────────────────────────────────────────┘
   
   
   sum!(1, 2, 3)  expand:
   ──────────────
   {
       let mut total = 0;
       total += 1;     ← từ $x = 1
       total += 2;     ← từ $x = 2
       total += 3;     ← từ $x = 3
       total
   }
```

---

## 8. Implement vec! từ đầu

```
   macro_rules! my_vec {
       // Rule 1: Empty
       () => {
           Vec::new()
       };
   
       // Rule 2: List with elements
       ($($x:expr),+ $(,)?) => {
           //              ↑
           //  optional trailing comma
           {
               let mut v = Vec::new();
               $(
                   v.push($x);
               )+
               v
           }
       };
   
       // Rule 3: [value; count]
       ($value:expr ; $count:expr) => {
           {
               let mut v = Vec::new();
               let val = $value;
               for _ in 0..$count { v.push(val.clone()); }
               v
           }
       };
   }
   
   ────────────────────────────────────────────────────
   
   my_vec!()              ──► Vec::new()
   my_vec!(1, 2, 3)       ──► {let mut v=Vec::new(); v.push(1); v.push(2); v.push(3); v}
   my_vec!(1, 2, 3,)      ──► same (trailing comma OK)
   my_vec!(0; 5)          ──► [0,0,0,0,0]
```

---

## 9. Hygiene — Variable Scope an toàn

```
   C MACRO (text substitution):           RUST MACRO (hygienic):
   ────────────────────────────           ──────────────────────
   
   #define SWAP(a, b) {              macro_rules! make_x {
     int tmp = a;                       () => {
     a = b;                                let x = 42;
     b = tmp;                           };
   }                                  }
   
   int tmp = 5;                       fn main() {
   int x = 10;                            let x = 10;
   SWAP(tmp, x);                          make_x!();
   //  ↑                                  println!("{}", x);
   //  Đại loạn! tmp đè lên              //   ↑
   //  tmp macro internal                //   In ra 10, KHÔNG phải 42
   //  → x KHÔNG được swap                }
   
   
   ┌──────────────────────────────────────────────────────────┐
   │ TRỰC QUAN HYGIENE:                                       │
   │                                                          │
   │   Caller scope          Macro scope                      │
   │  ──────────────        ──────────────                    │
   │                                                          │
   │   let x = 10;    │     │ let x = 42;                     │
   │       │          │     │     │                           │
   │       │  ←──── invisible barrier ────→                   │
   │       │          │     │     │                           │
   │       └──────────┘     └─────┘                           │
   │       x = 10                  x = 42                     │
   │       (vẫn 10 sau macro)      (chỉ trong macro scope)    │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. `$crate` — Cross-crate hygiene

```
   ┌─────────────────────────────────────────────────────────────┐
   │ Trong crate mylib:                                          │
   │                                                             │
   │   pub fn helper() {}                                        │
   │                                                             │
   │   #[macro_export]                                           │
   │   macro_rules! call_helper {                                │
   │       () => {                                               │
   │           $crate::helper();                                 │
   │           //   ↑                                            │
   │           //   $crate = "đường dẫn đến crate ROOT          │
   │           //            của crate định nghĩa macro này"     │
   │       };                                                    │
   │   }                                                         │
   └─────────────────────────────────────────────────────────────┘
   
   
   ┌─────────────────────────────────────────────────────────────┐
   │ User crate (myapp) — chỉ thêm dependency, không cần `use`:  │
   │                                                             │
   │   fn main() {                                               │
   │       mylib::call_helper!();                                │
   │       //                  expand thành:                     │
   │       //                  mylib::helper();   ← đúng path!  │
   │   }                                                         │
   └─────────────────────────────────────────────────────────────┘
   
   
   ❌ Nếu KHÔNG có $crate:
   ────────────────────────
   macro_rules! call_helper {
       () => { helper(); };           ← Wrong! User phải `use mylib::helper`
   }                                    trước khi gọi macro.
```

---

## 11. cargo expand — Vũ khí debug

```
   $ cargo install cargo-expand
   
   $ cat src/main.rs
   ────────────────
   fn main() {
       let v = vec![1, 2, 3];
       println!("{:?}", v);
   }
   
   
   $ cargo expand
   ──────────────
   fn main() {
       let v = ::std::vec::from_elem(0, 3);   ← vec! expanded
       // ... hoặc với literal items:
       // let v = <[_]>::into_vec(::std::boxed::box_new([1,2,3]));
       
       {                                       ← println! expanded
           ::std::io::_print(
               ::core::fmt::Arguments::new_v1(
                   &["", "\n"],
                   &[::core::fmt::ArgumentV1::new(&v, ::core::fmt::Debug::fmt)]
               )
           );
       };
   }
   
   
   ┌──────────────────────────────────────────────────────┐
   │  Use cases:                                          │
   │   • Debug macro của bạn                              │
   │   • Hiểu cách serde/tokio sinh code                  │
   │   • Verify proc-macro output                         │
   └──────────────────────────────────────────────────────┘
```

---

## 12. Procedural Macros — Setup crate

```
   ┌────────────────────────────────────────────────────────┐
   │ Cargo.toml của my-macros crate:                        │
   │                                                        │
   │   [package]                                            │
   │   name = "my-macros"                                   │
   │   version = "0.1.0"                                    │
   │   edition = "2021"                                     │
   │                                                        │
   │   [lib]                                                │
   │   proc-macro = true        ← KEY: marks as proc-macro │
   │                                                        │
   │   [dependencies]                                       │
   │   proc-macro2 = "1"                                    │
   │   syn = { version = "2", features = ["full"] }         │
   │   quote = "1"                                          │
   └────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────┐
   │ Workspace cấu trúc:                                    │
   │                                                        │
   │   myproject/                                           │
   │   ├── Cargo.toml (workspace)                           │
   │   ├── my-macros/         ← proc-macro = true          │
   │   │   ├── Cargo.toml                                   │
   │   │   └── src/lib.rs                                   │
   │   └── my-app/            ← consumer                   │
   │       ├── Cargo.toml     (depends on my-macros)        │
   │       └── src/main.rs                                  │
   └────────────────────────────────────────────────────────┘
```

---

## 13. Proc-Macro Flow — Token in, Token out

```
   ┌───────────────────────────────────────────────────────────┐
   │                                                           │
   │   User code:                                              │
   │   #[derive(MyTrait)]                                      │
   │   struct Foo { x: i32 }                                   │
   │              │                                            │
   │              ▼                                            │
   │   ┌──────────────────────────────────┐                   │
   │   │  proc_macro::TokenStream input   │ ← struct Foo {...}│
   │   └─────────────────┬────────────────┘                   │
   │                     │                                    │
   │                     ▼                                    │
   │   ┌──────────────────────────────────┐                   │
   │   │  syn::parse → DeriveInput AST    │                   │
   │   │                                  │                   │
   │   │   ident: Foo                     │                   │
   │   │   data: Struct { fields: ... }   │                   │
   │   └─────────────────┬────────────────┘                   │
   │                     │                                    │
   │                     ▼                                    │
   │   ┌──────────────────────────────────┐                   │
   │   │  Phân tích / transform           │ ← logic của bạn   │
   │   │  (Rust code thực thi)            │                   │
   │   └─────────────────┬────────────────┘                   │
   │                     │                                    │
   │                     ▼                                    │
   │   ┌──────────────────────────────────┐                   │
   │   │  quote! { impl MyTrait ... }     │                   │
   │   └─────────────────┬────────────────┘                   │
   │                     │                                    │
   │                     ▼                                    │
   │   ┌──────────────────────────────────┐                   │
   │   │  proc_macro::TokenStream output  │ ← impl MyTrait... │
   │   └─────────────────┬────────────────┘                   │
   │                     │                                    │
   │                     ▼                                    │
   │   Compile thành code bình thường                          │
   │                                                           │
   └───────────────────────────────────────────────────────────┘
```

---

## 14. 3 loại Proc-Macro

```
   ┌──────────────────────────────────────────────────────────────┐
   │ 1. DERIVE MACRO                                              │
   │ ─────────────────                                            │
   │                                                              │
   │   #[derive(MyDerive)]                                        │
   │   struct Foo {x: i32}        ← attach to struct/enum         │
   │                                                              │
   │   Sinh:  impl ... for Foo {...}    (THÊM impl, không sửa)    │
   │                                                              │
   │   Use cases: Debug, Clone, Serialize, thiserror::Error       │
   │                                                              │
   │   #[proc_macro_derive(MyDerive, attributes(my_attr))]        │
   │   pub fn my_derive(input: TokenStream) -> TokenStream {...}  │
   └──────────────────────────────────────────────────────────────┘
   
   
   ┌──────────────────────────────────────────────────────────────┐
   │ 2. ATTRIBUTE MACRO                                           │
   │ ───────────────────                                          │
   │                                                              │
   │   #[my_attr(args)]                                           │
   │   fn do_work() {...}         ← attach to function/struct     │
   │                                                              │
   │   Sinh: SỬA item (wrap, transform, replace)                  │
   │                                                              │
   │   Use cases: tokio::main, test, route handlers               │
   │                                                              │
   │   #[proc_macro_attribute]                                    │
   │   pub fn my_attr(args: TokenStream, item: TokenStream)       │
   │       -> TokenStream {...}                                   │
   └──────────────────────────────────────────────────────────────┘
   
   
   ┌──────────────────────────────────────────────────────────────┐
   │ 3. FUNCTION-LIKE MACRO                                       │
   │ ────────────────────────                                     │
   │                                                              │
   │   my_macro!(arbitrary input);                                │
   │                                                              │
   │   Sinh: TÙY (như macro_rules! nhưng phân tích phức tạp hơn) │
   │                                                              │
   │   Use cases: sqlx::query!, html!, custom DSL                 │
   │                                                              │
   │   #[proc_macro]                                              │
   │   pub fn my_macro(input: TokenStream) -> TokenStream {...}   │
   └──────────────────────────────────────────────────────────────┘
```

---

## 15. Bộ ba: proc-macro2, syn, quote

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │   proc-macro2  ← TokenStream wrapper (cross-crate)           │
   │       │                                                      │
   │       ├── chuyển từ proc_macro::TokenStream                  │
   │       └── usable trong test + library code                   │
   │                                                              │
   │   syn          ← PARSE: tokens → AST                         │
   │       │                                                      │
   │       ├── syn::DeriveInput                                   │
   │       ├── syn::ItemFn                                        │
   │       ├── syn::Type                                          │
   │       └── ~100 AST types                                     │
   │                                                              │
   │   quote        ← UNPARSE: Rust code → tokens                 │
   │       │                                                      │
   │       ├── quote! { #name impl ... }                          │
   │       ├── #var = substitute                                  │
   │       └── #( ... )* = repetition                             │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   
   Flow trong proc-macro:
   ──────────────────────
   
        TokenStream input
              │
              ▼
        syn::parse() ────► Rust AST (typed)
              │
              ▼
        Logic của bạn ───► transform/analyze AST
              │
              ▼
        quote! { ... } ──► TokenStream output
              │
              ▼
        Compile bình thường
```

---

## 16. Ví dụ: Derive Macro hoàn chỉnh

```
   ┌──────────────────────────────────────────────────────────────┐
   │ // my-macros/src/lib.rs                                      │
   │                                                              │
   │ use proc_macro::TokenStream;                                 │
   │ use quote::quote;                                            │
   │ use syn::{parse_macro_input, DeriveInput};                   │
   │                                                              │
   │ #[proc_macro_derive(HelloWorld)]                             │
   │ pub fn derive_hello(input: TokenStream) -> TokenStream {     │
   │     let input = parse_macro_input!(input as DeriveInput);    │
   │     let name = input.ident;     // ← lấy tên struct          │
   │                                                              │
   │     let expanded = quote! {                                  │
   │         impl HelloWorld for #name {                          │
   │             fn hello() {                                     │
   │                 println!("Hi from {}", stringify!(#name));   │
   │             }                                                │
   │         }                                                    │
   │     };                                                       │
   │                                                              │
   │     TokenStream::from(expanded)                              │
   │ }                                                            │
   └──────────────────────────────────────────────────────────────┘
                              │
                              ▼ user code
   ┌──────────────────────────────────────────────────────────────┐
   │ // my-app/src/main.rs                                        │
   │                                                              │
   │ use my_macros::HelloWorld;                                   │
   │ trait HelloWorld { fn hello(); }                             │
   │                                                              │
   │ #[derive(HelloWorld)]                                        │
   │ struct Pancakes;                                             │
   │                                                              │
   │ fn main() {                                                  │
   │     Pancakes::hello();   // "Hi from Pancakes"               │
   │ }                                                            │
   └──────────────────────────────────────────────────────────────┘
                              │
                              ▼ expand
   ┌──────────────────────────────────────────────────────────────┐
   │ // Code thực tế sau expand:                                  │
   │                                                              │
   │ struct Pancakes;                                             │
   │                                                              │
   │ impl HelloWorld for Pancakes {                               │
   │     fn hello() {                                             │
   │         println!("Hi from {}", "Pancakes");                  │
   │     }                                                        │
   │ }                                                            │
   │                                                              │
   │ fn main() { Pancakes::hello(); }                             │
   └──────────────────────────────────────────────────────────────┘
```

---

## 17. quote! — Template DSL

```
   Variable substitution:
   ──────────────────────
   
   let name = format_ident!("Foo");
   
   quote! { struct #name; }
   //              ↑
   //              substitute: thay #name bằng "Foo"
   //              → struct Foo;
   
   
   Repetition:
   ───────────
   
   let fields = vec!["x", "y", "z"];
   let field_idents: Vec<_> = fields.iter()
       .map(|n| format_ident!("{}", n))
       .collect();
   
   quote! {
       struct Point {
           #( pub #field_idents : i32, )*
       }
   }
   //         ↑
   //         lặp lại body cho mỗi field_ident
   //         → struct Point {
   //               pub x: i32,
   //               pub y: i32,
   //               pub z: i32,
   //           }
```

---

## 18. Cách `thiserror` hoạt động

```
   ┌──────────────────────────────────────────────────────────────┐
   │ User viết:                                                   │
   │                                                              │
   │ #[derive(Error, Debug)]                                      │
   │ pub enum MyError {                                           │
   │     #[error("not found: {0}")]                               │
   │     NotFound(String),                                        │
   │                                                              │
   │     #[error("IO")]                                           │
   │     Io(#[from] std::io::Error),                              │
   │ }                                                            │
   └──────────────────────┬───────────────────────────────────────┘
                          │
                          ▼  thiserror proc-macro chạy
   ┌──────────────────────────────────────────────────────────────┐
   │ syn::DeriveInput:                                            │
   │   ident: MyError                                             │
   │   data: Enum {                                               │
   │     variants: [                                              │
   │       NotFound(String) with #[error("not found: {0}")],      │
   │       Io(...) with #[error("IO")] #[from] std::io::Error,    │
   │     ]                                                        │
   │   }                                                          │
   └──────────────────────┬───────────────────────────────────────┘
                          │
                          ▼  quote! sinh:
   ┌──────────────────────────────────────────────────────────────┐
   │ impl std::fmt::Display for MyError {                         │
   │     fn fmt(&self, f) -> Result {                             │
   │         match self {                                         │
   │             Self::NotFound(s) => write!(f, "not found: {}", s),│
   │             Self::Io(_) => write!(f, "IO"),                  │
   │         }                                                    │
   │     }                                                        │
   │ }                                                            │
   │                                                              │
   │ impl std::error::Error for MyError {                         │
   │     fn source(&self) -> Option<&...> {                       │
   │         match self {                                         │
   │             Self::Io(e) => Some(e),                          │
   │             _ => None,                                       │
   │         }                                                    │
   │     }                                                        │
   │ }                                                            │
   │                                                              │
   │ impl From<std::io::Error> for MyError {                      │
   │     fn from(e: std::io::Error) -> Self { Self::Io(e) }       │
   │ }                                                            │
   └──────────────────────────────────────────────────────────────┘
   
   ⟹ User viết ~10 dòng, thiserror sinh ~40 dòng.
   ⟹ Zero runtime cost. Tất cả tại compile time.
```

---

## 19. Cách `#[tokio::main]` hoạt động

```
   ┌──────────────────────────────────────────────────────────────┐
   │ User viết:                                                   │
   │                                                              │
   │ #[tokio::main]                                               │
   │ async fn main() {                                            │
   │     println!("Hello async!");                                │
   │ }                                                            │
   └──────────────────────┬───────────────────────────────────────┘
                          │
                          ▼  attribute proc-macro
   ┌──────────────────────────────────────────────────────────────┐
   │ Sinh code:                                                   │
   │                                                              │
   │ fn main() {                          ← non-async             │
   │     let body = async {               ← original body         │
   │         println!("Hello async!");                            │
   │     };                                                       │
   │     tokio::runtime::Builder::new_multi_thread()              │
   │         .enable_all()                                        │
   │         .build()                                             │
   │         .unwrap()                                            │
   │         .block_on(body);                                     │
   │ }                                                            │
   └──────────────────────────────────────────────────────────────┘
   
   ⟹ Attribute macro NHẬN function, WRAP nó trong runtime.
   ⟹ User không cần biết tokio runtime detail.
```

---

## 20. Cây quyết định — Khi nào dùng cái gì?

```
                Cần generate / repeat code?
                          │
                  ┌───────┴────────┐
                 NO               YES
                  │                │
              ┌───┴───┐            ▼
              │       │       Function + generic + trait
              │       │       đủ giải quyết?
              │       │            │
              │       │       ┌────┴────┐
              │       │      YES        NO
              │       │       │          │
            DÙNG    DÙNG     ✅ DÙNG    ▼
        function   trait     function  macro_rules!
                                       đủ?
                                        │
                                   ┌────┴────┐
                                  YES        NO
                                   │          │
                              ✅ macro_rules!  ▼
                                          Đọc struct fields
                                          / parse AST?
                                               │
                                          ┌────┴────┐
                                         YES        NO
                                          │          │
                                          ▼          ▼
                                      Apply lên     Custom DSL
                                      item có sẵn?  cú pháp ngoài
                                          │        Rust syntax
                                      ┌───┴───┐         │
                                     YES     NO         ▼
                                      │       │     function-like
                                      ▼       ▼     proc macro
                                  Attribute  Derive
                                  macro      macro
   
   
   ┌───────────────────────────────────────────────────┐
   │  Quy tắc: thử ĐƠN GIẢN nhất trước.                │
   │                                                   │
   │  function < generic < trait < macro_rules!        │
   │      < proc-macro                                 │
   └───────────────────────────────────────────────────┘
```

---

## 21. Antipatterns visualization

```
   ❌ 1. Operator precedence bug
   ──────────────────────────────
   macro_rules! square { ($x:expr) => { $x * $x }; }
   
   square!(2 + 3)
        │
        ▼ expand:
   2 + 3 * 2 + 3 = 11      ← WRONG! Expected 25
        │
        ▼ fix:
   macro_rules! square { ($x:expr) => { ($x) * ($x) }; }
   //                                  ↑       ↑
   //                              wrap với ()
   
   square!(2 + 3) → (2 + 3) * (2 + 3) = 25 ✅
   
   
   ❌ 2. Macro phình code
   ────────────────────────
   macro_rules! big { ($x:expr) => { /* 100 dòng inline */ }; }
   
   big!(a); big!(b); big!(c); ... big!(z);
   //                ↓
   //                Mỗi callsite: 100 dòng × 26 = 2600 dòng
   //                Binary size: tăng đáng kể
   //                Instruction cache: miss nhiều
   
   ✅ Fix: macro chỉ wrap function call
   macro_rules! big { ($x:expr) => { __helper($x) }; }
   //                              ↑ logic ở 1 nơi
   
   
   ❌ 3. Tên macro chung chung khi export
   ───────────────────────────────────────
   #[macro_export]
   macro_rules! check { ... }  ← user dùng 2 crate cùng có check! → conflict
   
   ✅ Fix: prefix tên crate
   #[macro_export]
   macro_rules! mycrate_check { ... }
   // hoặc dùng path: mycrate::check!()
   
   
   ❌ 4. Macro hide complexity
   ────────────────────────────
   configure! {
       /users => UserController,
       /admin/* => AdminGuard => AdminController,
   }
   //   ↑
   //   Đẹp cho người quen, nhưng reviewer mới
   //   phải hiểu macro hoạt động ra sao
   
   Quy tắc: macros REPLACE BOILERPLATE, không HIDE COMPLEXITY
```

---

## 22. Compile-time overhead visualization

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  COMPILE TIME COST                                           │
   │                                                              │
   │  function           ░░░░░░░░ Fast                            │
   │  macro_rules!       ░░░░░░░░░░ Slightly slower               │
   │  proc-macro derive  ░░░░░░░░░░░░░░░░ Slower (build proc)     │
   │  proc-macro attr    ░░░░░░░░░░░░░░░░░░░ Slower yet           │
   │  serde::Serialize   ░░░░░░░░░░░░░░░░░░░░░░░░ Lot of fields  │
   │  sqlx::query!       ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ Connects to DB!│
   │                                                              │
   │  ───────────────────────────────────────────                 │
   │                                                              │
   │  RUNTIME COST                                                │
   │                                                              │
   │  Tất cả macros: ║░ Zero (compile time work, runtime same)    │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   
   📌 Trade-off:
      Macros bóc xếp compile time để get:
      • Code ngắn (boilerplate giảm)
      • Type-safe DSL
      • Runtime tốc độ tối ưu
   
   ⟹ Nếu compile chậm, xem `cargo build --timings` để biết
     crate/proc-macro nào tốn nhất.
```

---

## 23. Memory & Binary impact

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  Source code (TÁC GIẢ VIẾT)                                  │
   │  ───────────                                                 │
   │  #[derive(Debug, Clone, Serialize)]    ← 1 dòng              │
   │  struct User {                                               │
   │      id: u64,                                                │
   │      name: String,                                           │
   │  }                                                           │
   │                                                              │
   │                  ▼  derive macros expand                     │
   │                                                              │
   │  Expanded code (COMPILER THẤY)                               │
   │  ──────────────                                              │
   │  struct User { id: u64, name: String }                       │
   │                                                              │
   │  impl Debug for User { ... 5 lines ... }                     │
   │  impl Clone for User { ... 5 lines ... }                     │
   │  impl Serialize for User { ... 30 lines ... }                │
   │  ↑                                                           │
   │  Tổng: ~40 dòng                                              │
   │                                                              │
   │                  ▼  compile to binary                        │
   │                                                              │
   │  Machine code (BINARY)                                       │
   │  ─────────────                                               │
   │  Debug::fmt        ~100 bytes                                │
   │  Clone::clone      ~30 bytes                                 │
   │  Serialize::ser    ~200 bytes                                │
   │  ────                                                        │
   │  Total ~330 bytes added                                      │
   │                                                              │
   │  ⟹ Mỗi struct có 3 derive = 330 bytes binary.                │
   │     50 struct → ~16 KB                                       │
   │     500 struct → ~165 KB                                     │
   └──────────────────────────────────────────────────────────────┘
   
   📌 Big projects (vd: nhiều structs với serde) có thể có 
      binary lớn vì derive sinh nhiều code.
   📌 lto = "thin"/"fat" giúp loại bỏ code không dùng.
```

---

## 24. Hệ sinh thái macros — Crates đáng học

```
   ┌──────────────────────────────────────────────────────────────┐
   │                  MACROS ECOSYSTEM                            │
   │                                                              │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │ TỰ VIẾT proc-macro:                                    │  │
   │  │   proc-macro2  syn  quote          ← Bộ ba bắt buộc    │  │
   │  │   darling                          ← Parse attrs dễ     │  │
   │  │   trybuild                         ← Test compile-fail │  │
   │  └────────────────────────────────────────────────────────┘  │
   │                                                              │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │ DERIVE PHỔ BIẾN:                                       │  │
   │  │   serde         ← Serialize, Deserialize               │  │
   │  │   thiserror     ← Error trait                          │  │
   │  │   strum         ← Enum utilities (iter, display, ...)  │  │
   │  │   derive_more   ← Add, Sub, From, Into, Deref          │  │
   │  │   derive_builder ← Builder pattern                     │  │
   │  │   smart-default ← Default với customization           │  │
   │  └────────────────────────────────────────────────────────┘  │
   │                                                              │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │ ATTRIBUTE:                                              │  │
   │  │   tokio::main, tokio::test                              │  │
   │  │   tracing::instrument                                   │  │
   │  │   async_trait (trước Rust 1.75)                         │  │
   │  └────────────────────────────────────────────────────────┘  │
   │                                                              │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │ FUNCTION-LIKE:                                          │  │
   │  │   sqlx::query!, query_as!  ← Compile-time SQL check    │  │
   │  │   tracing::info!, error!   ← Structured logging        │  │
   │  │   html! (yew), view! (leptos) ← UI DSL                 │  │
   │  │   maud!                    ← HTML templating           │  │
   │  └────────────────────────────────────────────────────────┘  │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 25. Mind Map cuối

```
                            MACROS RUST
                                │
       ┌────────────┬───────────┼──────────┬──────────────┐
       │            │           │          │              │
   TRIẾT LÝ     LOẠI         CƠ CHẾ      TOOLS          PITFALLS
       │            │           │          │              │
   Code sinh    declarative   token tree  cargo expand   precedence
   code at      (macro_rules!)               syn         binary bloat
   compile      procedural    fragment    quote          hygiene leak
   time         (proc-macro)    specifiers proc-macro2   recursion limit
       │            │         repetition  trybuild       error UX
   Zero         macro_rules!  hygiene
   runtime      proc-macro    $crate
   cost         3 loại:
                  derive
                  attribute
                  function-like
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHT cho SENIOR             │
                │  ─────────────────────────           │
                │                                      │
                │  Macros là DAO CUỐI — thử            │
                │  function/generic/trait trước.       │
                │                                      │
                │  Khi cần:                            │
                │  • macro_rules! → 80% cases         │
                │  • derive → đọc struct → sinh impl   │
                │  • attribute → wrap function         │
                │  • function-like → custom DSL        │
                │                                      │
                │  Không bao giờ:                      │
                │  • Hide complexity bằng macro        │
                │  • Tránh function với macro          │
                │  • Quên cargo expand khi debug       │
                └──────────────────────────────────────┘
```

---

## 26. Bộ tài liệu Rust giờ có 8 chủ đề

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   1. memory-model            — Bộ nhớ                    │
   │      memory-model-visual                                 │
   │                                                          │
   │   2. ownership-borrowing     — Quyền sở hữu             │
   │      ownership-borrowing-visual                          │
   │                                                          │
   │   3. trait                   — Polymorphism             │
   │      trait-visual                                        │
   │                                                          │
   │   4. generic                 — Parametric polymorphism  │
   │      generic-visual                                      │
   │                                                          │
   │   5. closure                 — Function as value        │
   │      closure-visual                                      │
   │                                                          │
   │   6. async                   — Concurrency              │
   │      async-visual                                        │
   │                                                          │
   │   7. error-handling          — Error handling           │
   │      error-handling-visual                               │
   │                                                          │
   │   8. macros                  — Macros                   │
   │      macros-visual             ← VỪA HOÀN THÀNH          │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 16 files                                         │
   │                                                          │
   │   ✨ Bạn đã đầy đủ vũ khí Rust production 🦀            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau macros, bộ kỹ năng có thể đào sâu các nhánh thực hành:

- **Unsafe Rust** — raw pointers, FFI, atomic ordering nâng cao
- **Testing patterns** — unit, integration, proptest, criterion bench
- **Iterator deep dive** — implement Iterator, lazy, rayon parallel
- **Smart pointers deep dive** — Box, Rc, Arc, Cell, RefCell, Mutex
- **Logging & Observability** — tracing nâng cao, OpenTelemetry
- **Web framework** — axum/actix project realistic (apply 8 chủ đề đã học)

Báo cái nào bạn muốn đào sâu tiếp! 🦀⚡
