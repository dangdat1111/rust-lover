# Closure — TOÀN BỘ qua HÌNH VẼ

> Companion visual cho `closure.md`. Mọi khái niệm vẽ ra để người chưa biết gì cũng hiểu.

---

## Mục lục

1. [Bức tranh lớn: Closure là gì?](#1-bức-tranh-lớn)
2. [Function vs Closure](#2-function-vs-closure)
3. [Cú pháp closure trực quan](#3-cú-pháp)
4. [3 cách Capture](#4-3-cách-capture)
5. [Compiler tự chọn capture mode](#5-compiler-chọn-capture)
6. [`move` keyword](#6-move-keyword)
7. [Disjoint capture (Rust 2021)](#7-disjoint-capture)
8. [3 trait Fn/FnMut/FnOnce](#8-3-trait)
9. [Hierarchy Fn ⊆ FnMut ⊆ FnOnce](#9-hierarchy)
10. [Closure = Struct ẩn](#10-closure-struct-ẩn)
11. [Mỗi closure UNIQUE type](#11-unique-type)
12. [Closure trong memory](#12-closure-memory)
13. [Closure size phụ thuộc capture](#13-size)
14. [Pass closure: generic vs dyn](#14-pass-closure)
15. [Return closure: impl Fn vs Box dyn Fn](#15-return-closure)
16. [Closure trong struct](#16-closure-struct)
17. [Iterator combinator chain](#17-iterator-chain)
18. [Higher-order functions](#18-higher-order)
19. [Closure + thread + move](#19-closure-thread)
20. [Stateful closure (counter)](#20-stateful)
21. [Decision tree](#21-decision-tree)
22. [Common errors visual](#22-common-errors)
23. [Mind map](#23-mind-map)

---

## 1. Bức tranh lớn

```
   ╔═══════════════════════════════════════════════════════════╗
   ║                       CLOSURE                              ║
   ║                                                            ║
   ║    Function thường:                                        ║
   ║    fn add(a: i32, b: i32) -> i32 { a + b }                 ║
   ║                                                            ║
   ║    → Mỗi lần gọi: "khởi tạo lại từ đầu"                    ║
   ║    → Không nhớ gì                                          ║
   ║                                                            ║
   ║    Closure:                                                ║
   ║    let n = 5;                                              ║
   ║    let add_n = |x| x + n;                                  ║
   ║                                                            ║
   ║    → "Function nhớ context (n)"                            ║
   ║    → Capture environment                                   ║
   ║                                                            ║
   ║    ┌──────────────┐                                        ║
   ║    │   Closure    │                                        ║
   ║    │  ┌────────┐  │                                        ║
   ║    │  │ code   │  │  ← logic                               ║
   ║    │  ├────────┤  │                                        ║
   ║    │  │ env: n │  │  ← captured                           ║
   ║    │  └────────┘  │                                        ║
   ║    └──────────────┘                                        ║
   ║                                                            ║
   ╚═══════════════════════════════════════════════════════════╝
```

---

## 2. Function vs Closure

```
   ═════════════════════════════════════════════════════════════
                          FUNCTION
   ═════════════════════════════════════════════════════════════
   
   fn add(a: i32, b: i32) -> i32 {
       a + b
   }
   
   Layout:
   ──────
   ┌─────────────────┐
   │ TEXT segment    │ ← code
   │   add: ASM      │
   └─────────────────┘
   
   Gọi: add(1, 2);
        │
        ▼ call instruction
        │
        ▼ thực thi code
   
   Size pointer: 8 byte (fn pointer)
   State: KHÔNG có
   
   
   ═════════════════════════════════════════════════════════════
                          CLOSURE
   ═════════════════════════════════════════════════════════════
   
   let n = 5;
   let cl = |x| x + n;
   
   Layout:
   ──────
   ┌─────────────────┐         ┌─────────────────┐
   │ STACK:          │         │ TEXT segment    │
   │ ┌──────────┐    │         │   call() code   │
   │ │ cl:      │    │         │   for __Cl      │
   │ │   n: 5   │ ───┼────────►│                 │
   │ └──────────┘    │         └─────────────────┘
   │ struct __Cl     │
   └─────────────────┘
   
   Gọi: cl(3);
        │
        ▼ resolve to __Cl::call(&cl, (3,))
        │
        ▼ thực thi code (truy cập cl.n)
   
   Size: phụ thuộc captured vars
   State: CÓ (env)
```

---

## 3. Cú pháp closure trực quan

```
   ┌──────────────────────────────────────────────────────┐
   │                CLOSURE SYNTAX                        │
   │                                                      │
   │    |args| body                                       │
   │    └──┬──┘ └──┬─┘                                    │
   │       │      │                                       │
   │   parameters body (1 expression hoặc { block })      │
   │                                                      │
   └──────────────────────────────────────────────────────┘
   
   Ví dụ:
   ─────
   
   1. Không args:
      || println!("hi")
      ▲▲
      ││
      ││ body: print
      │└ parameters (empty)
      └ start
   
   2. 1 arg, suy type:
      |x| x + 1
      ▲   ▲▲▲
      │   │
      │   body
      └ parameters (x)
   
   3. Multiple args:
      |a, b| a + b
      └────┘ └───┘
        args  body
   
   4. Có type annotation:
      |x: i32| -> i32 { x + 1 }
      └─────────────┘ └───────┘
        signature        body
   
   5. move:
      move || x + 1
      ▲▲▲▲
      ▲ ép capture by value
```

---

## 4. 3 cách Capture

```
   ╔═════════════════════════════════════════════════════════╗
   ║              3 CAPTURE MODES                            ║
   ╚═════════════════════════════════════════════════════════╝
   
   Mode 1: BY REFERENCE (&)
   ──────────────────
   
   let x = 5;
   let cl = || println!("{}", x);
   
   STACK:
   ┌──────────┐
   │ x = 5    │ ◄────┐
   ├──────────┤      │
   │ cl: __Cl │      │ closure chứa &x
   │   x: ────┼──────┘
   └──────────┘
   
   → Closure implements Fn (đọc, gọi nhiều lần)
   → x VẪN dùng được bên ngoài
   
   
   Mode 2: BY MUTABLE REFERENCE (&mut)
   ─────────────────────────────
   
   let mut v = vec![1, 2];
   let mut cl = || v.push(3);
   
   STACK:
   ┌──────────┐
   │ v: Vec   │ ◄────┐
   │  ptr,len │      │
   ├──────────┤      │ closure chứa &mut v
   │ cl: __Cl │      │
   │   v: ────┼──────┘
   └──────────┘
   
   → Closure implements FnMut (sửa, gọi nhiều lần)
   → v KHÔNG dùng được trong khi cl alive (exclusive borrow)
   
   
   Mode 3: BY MOVE (consume)
   ──────────────────
   
   let s = String::from("hi");
   let cl = move || println!("{}", s);
   
   STACK:                          HEAP:
   ┌──────────┐                    ┌───┬───┐
   │ s: DEAD  │                    │ h │ i │
   │          │                    └───┴───┘
   ├──────────┤                       ▲
   │ cl: __Cl │                       │
   │   s: ────┼───────────────────────┘
   │   .len   │   (s đã MOVE vào closure)
   │   .cap   │
   └──────────┘
   
   → Closure implements FnOnce (consume)
   → s KHÔNG còn ngoài, closure SỞ HỮU
```

---

## 5. Compiler tự chọn capture mode

```
   Compiler quy tắc: chọn mode YẾU NHẤT đủ dùng
   
                 ┌──────────────────────────────────┐
                 │ Closure body sử dụng x như nào?  │
                 └────────────────┬─────────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
         CHỈ ĐỌC            ĐỌC + SỬA            MOVE/CONSUME
              │                   │                   │
              ▼                   ▼                   ▼
       Capture by &        Capture by &mut       Capture by move
       (impl Fn)           (impl FnMut)          (impl FnOnce)
   
   
   Ví dụ:
   ─────
   
   let x = 5;
   ─────────────
   
   |y| y + x         ← chỉ đọc x  → Fn
   
   let mut v = vec![];
   ────────────────────
   
   |y| v.push(y)     ← sửa v     → FnMut
   
   |y| { drop(v); y } ← consume  → FnOnce
```

### Move ép override

```
   let x = vec![1, 2];
   let cl = move || println!("{:?}", x);
                ▲▲▲▲
                ép capture by VALUE (move), kể cả khi đáng lẽ &
   
   → x đã move vào closure
   → x không dùng được ngoài nữa
```

---

## 6. `move` keyword

```
   Khi nào dùng move?
   ─────────
   
   1. THREAD                          2. RETURN CLOSURE
   ────                                ────
   
   spawn(|| use_x)                    fn make() -> impl Fn() {
   ❌ thread có thể outlive caller        let x = ...;
                                          || use(x)  ❌ x dies
   spawn(move || use_x)                }
   ✓ closure SỞ HỮU x                  
                                       fn make() -> impl Fn() {
                                          let x = ...;
                                          move || use(x)  ✓
                                       }
   
   
   3. STATIC LIFETIME                 4. EXPLICIT OWNERSHIP
   ─────                                ────
   
   fn need_static(f: impl Fn() + 'static)
                                       let buf = vec![0; 1024];
   let x = String::from("hi");          let cl = move || process(buf);
   need_static(move || drop(x))         // cl SỞ HỮU buf rõ ràng
   ✓ closure không vay local
```

### Visual

```
   Without move:                     With move:
   ────────                           ────
   
   let s = "hi".to_string();          let s = "hi".to_string();
   let cl = || drop(s);               let cl = move || drop(s);
   
   STACK:                             STACK:
   ┌────────┐                         ┌────────┐
   │ s ─────┼──► HEAP "hi"            │ s: DEAD│
   ├────────┤      ▲                  ├────────┤
   │ cl     │      │                  │ cl:    │
   │   &s ──┼──────┘                  │   s ───┼──► HEAP "hi"
   └────────┘                         │ (moved)│
                                      └────────┘
   (closure vay s)                    (closure sở hữu s)
```

---

## 7. Disjoint capture (Rust 2021)

```
   Trước 2021:                       Sau 2021:
   ────                                ────
   
   struct Big {                       struct Big {
       a: String,                         a: String,
       b: String,                         b: String,
   }                                  }
   
   let big = ...;                     let big = ...;
   let cl = || println!(big.a);       let cl = || println!(big.a);
   //          ▲ Capture WHOLE big    //          ▲ Capture only big.a
   
   println!(big.b);  ❌ ERROR         println!(big.b);  ✓ OK
   (cl borrows whole big)             (cl only borrows big.a)
```

### Visual

```
   Trước 2021:                       Sau 2021:
   ────────                           ────
   
       ┌─────────┐                       ┌─────────┐
       │  big    │                       │  big    │
       │   a ────┼─┐                     │   a ────┼──► cl
       │   b     │ │                     │   b     │
       └─────────┘ │                     └─────────┘
            ▲      │                          │
            │      │                          ▼
            │      │                       └ b vẫn truy cập được
            │      ▼
            │   captured
            │
        cl bắt WHOLE big
            
        b không dùng được
```

---

## 8. 3 trait Fn/FnMut/FnOnce

```rust
trait FnOnce<Args> {
    type Output;
    fn call_once(self, args: Args) -> Self::Output;
                 ▲▲▲▲
                 consume self
}

trait FnMut<Args>: FnOnce<Args> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
                ▲▲▲▲▲▲▲▲▲
                exclusive borrow
}

trait Fn<Args>: FnMut<Args> {
    fn call(&self, args: Args) -> Self::Output;
            ▲▲▲▲▲
            shared borrow
}
```

### Visual

```
   ╔══════════════════════════════════════════════════════╗
   ║                Closure receiver type                  ║
   ╠══════════════════════════════════════════════════════╣
   ║                                                      ║
   ║   Fn       fn(&self, ...) -> ...                     ║
   ║            ─────                                     ║
   ║            "Tôi chỉ ĐỌC environment"                 ║
   ║            "Gọi NHIỀU LẦN qua &"                     ║
   ║                                                      ║
   ║   FnMut    fn(&mut self, ...) -> ...                 ║
   ║            ─────────                                 ║
   ║            "Tôi SỬA environment"                     ║
   ║            "Gọi nhiều lần qua &mut"                  ║
   ║                                                      ║
   ║   FnOnce   fn(self, ...) -> ...                      ║
   ║            ────                                      ║
   ║            "Tôi CONSUME environment"                 ║
   ║            "Gọi ĐÚNG 1 LẦN"                          ║
   ║                                                      ║
   ╚══════════════════════════════════════════════════════╝
```

---

## 9. Hierarchy Fn ⊆ FnMut ⊆ FnOnce

```
                       ┌──────────┐
                       │ FnOnce   │  ← parent, weakest
                       └──────────┘
                            ▲
                            │ extends
                       ┌──────────┐
                       │  FnMut   │
                       └──────────┘
                            ▲
                            │ extends
                       ┌──────────┐
                       │   Fn     │  ← child, strongest
                       └──────────┘
   
   Subset chain:
   ────────
   
   Fn ⊆ FnMut ⊆ FnOnce
   
   (closure implements Fn → tự động cũng implements FnMut, FnOnce)
```

### Matrix kiểu closure → trait nào

```
   ┌────────────────────┬─────┬───────┬────────┐
   │ Capture nào?       │  Fn │ FnMut │ FnOnce │
   ├────────────────────┼─────┼───────┼────────┤
   │ Không capture      │  ✓  │   ✓   │   ✓    │
   │ Capture by &       │  ✓  │   ✓   │   ✓    │
   │ Capture by &mut    │  ❌ │   ✓   │   ✓    │
   │ Capture by move,   │  ❌ │   ❌  │   ✓    │
   │ và consume         │     │       │        │
   │ Capture by move,   │  ✓  │   ✓   │   ✓    │
   │ chỉ đọc (Copy)     │     │       │        │
   └────────────────────┴─────┴───────┴────────┘
```

### Nhận closure: yếu nhất → linh hoạt nhất

```
   ┌───────────────────────────────────────────────────┐
   │                                                   │
   │   Hàm nhận FnOnce — chấp nhận MỌI closure         │
   │       fn run<F: FnOnce()>(f: F) { f(); }          │
   │                                                   │
   │   Hàm nhận FnMut — chấp nhận FnMut và Fn          │
   │       fn run<F: FnMut()>(mut f: F) { f(); }       │
   │                                                   │
   │   Hàm nhận Fn — chỉ chấp nhận Fn                  │
   │       fn run<F: Fn()>(f: F) { f(); }              │
   │                                                   │
   │   Quy tắc: bound càng YẾU → API càng LINH HOẠT    │
   │           (nhưng giới hạn cách dùng trong body)   │
   │                                                   │
   └───────────────────────────────────────────────────┘
```

---

## 10. Closure = Struct ẩn

```rust
let x = 10;
let factor = 3;
let cl = |y| y * factor + x;
```

### Compiler sinh

```
   STRUCT ẨN (anonymous, compiler-generated):
   ──────────────────────────────
   
   struct __Closure_5 {        ← tên ẨN, không truy cập được
       x: i32,                 ← captured by & or value
       factor: i32,
   }
   
   impl Fn(i32) -> i32 for __Closure_5 {
       fn call(&self, args: (i32,)) -> i32 {
           let (y,) = args;
           y * self.factor + self.x
       }
   }
   
   
   STACK của bạn:
   ─────────
   ┌──────────────────┐
   │ x = 10           │
   ├──────────────────┤
   │ factor = 3       │
   ├──────────────────┤
   │ cl: __Closure_5  │
   │   x: 10          │  ← captured
   │   factor: 3      │  ← captured
   └──────────────────┘
   
   
   Khi gọi cl(5):
   ─────
   
   compiler dịch thành:
       __Closure_5::call(&cl, (5,))
                          │
                          ▼
                     trả về 5 * 3 + 10 = 25
```

---

## 11. Mỗi closure UNIQUE type

```
   Source:                            Compiler:
   ──────                             ────────
   
   let cl1 = || 42;                   struct __Closure_L1;
                                      impl Fn() -> i32 for __Closure_L1 ...
                                      let cl1: __Closure_L1 = ...;
   
   let cl2 = || 42;                   struct __Closure_L2;       ← KHÁC TYPE!
                                      impl Fn() -> i32 for __Closure_L2 ...
                                      let cl2: __Closure_L2 = ...;
   
   let v = vec![cl1, cl2];   ❌ ERROR: cl1 và cl2 khác type!
```

### Sơ đồ

```
              Universe of closures
                       │
                       │ each literal → unique type
                       ▼
   ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
   │ __Cl_5 │  │ __Cl_8 │  │ __Cl_11│  │ __Cl_15│
   └────────┘  └────────┘  └────────┘  └────────┘
        ▲           ▲           ▲           ▲
        │           │           │           │
        cl1         cl2         cl3         cl4
   
   Tất cả đều impl Fn() -> i32
   Nhưng TYPE khác nhau!
```

### Giải pháp: trait object

```
   ┌────────────────────────────────────────────────────┐
   │ Box<dyn Fn() -> i32> — type erased                 │
   │                                                    │
   │ Vec<Box<dyn Fn() -> i32>> = vec![                  │
   │     Box::new(cl1),                                 │
   │     Box::new(cl2),                                 │
   │     Box::new(cl3),                                 │
   │ ]                                                  │
   │                                                    │
   │ → Tất cả cùng type: Box<dyn Fn() -> i32>          │
   │ → Có thể vào 1 Vec                                 │
   │ → Cost: heap alloc + vtable                        │
   └────────────────────────────────────────────────────┘
```

---

## 12. Closure trong memory

```rust
fn main() {
    let x = 10;
    let s = String::from("hello");
    let cl = move |y: i32| -> i32 {
        println!("{}", s);
        x + y
    };
    cl(5);
}
```

### Layout chi tiết

```
   STACK (main frame):
   ┌────────────────────────┐
   │ x: i32 = 10            │  ← original
   ├────────────────────────┤
   │ s: String              │  ← original (DEAD sau move)
   │   ptr ─ DEAD           │
   ├────────────────────────┤
   │ cl: __Closure {        │
   │   x: i32 = 10          │  ← captured by COPY (i32 is Copy)
   │   s: String {          │  ← MOVED here
   │     ptr ──────────────┐│
   │     len = 5           ││
   │     cap = 5           ││
   │   }                   ││
   │ }                     ││
   └─────────────────────────│
                              ▼
                         HEAP:
                         ┌───┬───┬───┬───┬───┐
                         │ h │ e │ l │ l │ o │
                         └───┴───┴───┴───┴───┘
   
   Sau cl(5):
   ──────
   - Drop __Closure → drop s (in closure)
   - s.drop() → free heap "hello"
   - x ở main vẫn 10 (Copy không bị move)
```

---

## 13. Closure size phụ thuộc capture

```
   ╔══════════════════════════════════════════════════════╗
   ║          CLOSURE SIZE = SUM OF CAPTURED VARS         ║
   ╚══════════════════════════════════════════════════════╝
   
   let cl = || println!("hi");
   ──────────────────────────
   Không capture → size = 0 byte!
   ┌────┐
   │ ∅  │
   └────┘
   
   
   let x: i32 = 5;
   let cl = || println!("{}", x);
   ──────────────
   Capture &x → size = 8 byte (1 ref)
   ┌──────────┐
   │ &x       │
   │ (8 byte) │
   └──────────┘
   
   
   let s = String::from("hi");
   let cl = move || println!("{}", s);
   ──────────────
   Capture s by move → size = 24 byte (String header)
   ┌──────────┐
   │ s.ptr    │
   │ s.len    │
   │ s.cap    │
   └──────────┘
   
   
   let a = 1u8;
   let b = 2u64;
   let cl = move || (a, b);
   ──────────────
   size = 1 + padding 7 + 8 = 16 byte
   ┌──────────────┐
   │ a (1B) + pad │
   │ b (8B)        │
   └──────────────┘
```

### So sánh với function pointer

```
   fn pointer:                         closure with captures:
   ──────────                           ─────────
   ┌──────────┐                         ┌──────────────┐
   │ fn ptr   │                         │ field 1      │
   │ (8 byte) │                         │ field 2      │
   └──────────┘                         │   ...        │
                                        └──────────────┘
   Cố định 8 byte                       Phụ thuộc captured vars
```

---

## 14. Pass closure: generic vs dyn

### Generic — static dispatch

```rust
fn run<F: Fn() -> i32>(f: F) -> i32 {
    f()
}
```

```
   call site: run(cl1)
              │
              │ T = __Closure_for_cl1
              ▼
   compiler MONOMORPHIZE:
   fn run__Cl1(f: __Cl1) -> i32 {
       f()  ← inline
   }
   
   call site: run(cl2)
              │
              │ T = __Closure_for_cl2
              ▼
   fn run__Cl2(f: __Cl2) -> i32 {
       f()  ← inline
   }
   
   ⚡ Fast (inline)
   📦 Code bloat (mỗi closure type 1 version)
```

### Dynamic — &dyn Fn

```rust
fn run(f: &dyn Fn() -> i32) -> i32 {
    f()
}
```

```
   call site: run(&cl1)
              │
              ▼
   ┌────────────────────────┐
   │ &dyn Fn() -> i32       │ ← fat pointer
   │  data_ptr ──► cl1      │
   │  vtable_ptr ──► VT_1   │
   └────────────────────────┘
   
   call f():
   1. lookup vtable.call
   2. indirect call
   
   ⚡ ~2-3 ns extra
   📦 1 binary
   📦 No bloat
```

---

## 15. Return closure: impl Fn vs Box<dyn Fn>

```
   ╔══════════════════════════════════════════════════════════╗
   ║                  RETURN A CLOSURE                        ║
   ╠══════════════════════════════════════════════════════════╣
   ║                                                          ║
   ║   Option 1: impl Fn (Rust 1.26+)                         ║
   ║   ────                                                   ║
   ║                                                          ║
   ║   fn make() -> impl Fn(i32) -> i32 {                     ║
   ║       move |x| x + 1                                     ║
   ║   }                                                      ║
   ║                                                          ║
   ║   - Compiler "chôn" closure type                         ║
   ║   - 0-cost                                               ║
   ║   - Stack alloc                                          ║
   ║   - Chỉ 1 type per function                              ║
   ║                                                          ║
   ║                                                          ║
   ║   Option 2: Box<dyn Fn>                                  ║
   ║   ────                                                   ║
   ║                                                          ║
   ║   fn make() -> Box<dyn Fn(i32) -> i32> {                 ║
   ║       Box::new(move |x| x + 1)                           ║
   ║   }                                                      ║
   ║                                                          ║
   ║   - Heap alloc                                           ║
   ║   - vtable lookup                                        ║
   ║   - Có thể return KHÁC type qua branch:                  ║
   ║                                                          ║
   ║     fn make(plus: bool) -> Box<dyn Fn(i32, i32) -> i32> {║
   ║         if plus { Box::new(|a, b| a + b) }              ║
   ║         else { Box::new(|a, b| a - b) }                 ║
   ║     }                                                    ║
   ║                                                          ║
   ╚══════════════════════════════════════════════════════════╝
```

### Visual: impl Fn không đi với branch

```
   fn make(plus: bool) -> impl Fn(i32, i32) -> i32 {
       if plus { 
           |a, b| a + b           ← __Closure_A
       } else { 
           |a, b| a - b           ← __Closure_B
       }
   }
   
   ❌ ERROR: __Closure_A ≠ __Closure_B
            impl Fn cần MỘT type
```

### Fix với Box<dyn>

```
   fn make(plus: bool) -> Box<dyn Fn(i32, i32) -> i32> {
       if plus { 
           Box::new(|a, b| a + b)     ← Box<dyn Fn>
       } else { 
           Box::new(|a, b| a - b)     ← Box<dyn Fn>  (same type!)
       }
   }
   
   ✓ OK
```

---

## 16. Closure trong struct

```rust
struct EventHandler {
    callback: Box<dyn Fn(Event)>,
}
```

### Layout

```
   struct EventHandler {
       callback: Box<dyn Fn(Event)>,
   }
   
   STACK / HEAP:
   ┌──────────────────────────┐
   │ EventHandler             │
   │   callback:              │
   │     ┌──────────────────┐ │
   │     │ Box (16 byte fat │ │
   │     │  pointer):       │ │
   │     │  data_ptr ───────┼─┼──► HEAP (closure data)
   │     │  vtable_ptr ─────┼─┼──► VTABLE for Fn(Event)
   │     └──────────────────┘ │
   └──────────────────────────┘
   
   handler.callback(event)
        │
        ▼
   resolve vtable.call
        │
        ▼
   execute closure body
```

---

## 17. Iterator combinator chain

```rust
let v = vec![1, 2, 3, 4, 5];

let r: Vec<i32> = v.iter()
    .map(|&x| x * 2)
    .filter(|&x| x > 4)
    .collect();
```

### Lazy chain visualization

```
   ┌──────────┐
   │ v.iter() │ → Iter<i32>          ← lazy: 0 work
   └────┬─────┘
        │ .map(closure_M)
        ▼
   ┌──────────────────┐
   │ Map<Iter, F_M>   │              ← lazy: 0 work
   └────┬─────────────┘
        │ .filter(closure_F)
        ▼
   ┌──────────────────────────┐
   │ Filter<Map<Iter,F_M>,F_F>│      ← lazy: 0 work
   └────┬─────────────────────┘
        │ .collect()
        ▼
   ┌──────────────────────────┐
   │ Vec<i32>  ← TRIGGER!     │
   └──────────────────────────┘
   
   
   Khi collect chạy:
   ─────
   
   for each item in v:
       item → map(|&x| x * 2) → mapped
       mapped → filter(|&x| x > 4) → keep or skip
       if keep → push to Vec
   
   Item 1: 1 → 2 → reject
   Item 2: 2 → 4 → reject
   Item 3: 3 → 6 → keep
   Item 4: 4 → 8 → keep
   Item 5: 5 → 10 → keep
   
   r = [6, 8, 10]
```

### Không có intermediate Vec!

```
   NAIVE (Java style):                RUST IDIOM (lazy):
   ────────                            ────────
   
   tmp1 = v.map(...)                  pipeline mỗi item qua chain
   tmp2 = tmp1.filter(...)             rồi mới đến item kế
   r = tmp2.collect()
   
   3 Vec trong memory                  1 Vec final (r)
   ┌────┐ ┌────┐ ┌────┐                ┌────┐
   │tmp1│ │tmp2│ │ r  │                │ r  │
   └────┘ └────┘ └────┘                └────┘
```

---

## 18. Higher-order functions

```
   ┌──────────────────────────────────────────────────────┐
   │                  COMPOSITION                         │
   ├──────────────────────────────────────────────────────┤
   │                                                      │
   │  fn compose<F, G>(f: F, g: G) -> impl Fn(A) -> C     │
   │  where F: Fn(A) -> B, G: Fn(B) -> C                  │
   │  {                                                   │
   │      move |x| g(f(x))                                │
   │  }                                                   │
   │                                                      │
   │                                                      │
   │  Visualize:                                          │
   │                                                      │
   │       ┌───────────┐                                  │
   │   x ─►│  f: A→B   │─► f(x): B                        │
   │       └───────────┘                                  │
   │                          │                            │
   │                          ▼                            │
   │                    ┌───────────┐                     │
   │                    │  g: B→C   │─► g(f(x)): C        │
   │                    └───────────┘                     │
   │                                                      │
   │  pipeline = compose(f, g)                            │
   │  pipeline(x) → g(f(x))                               │
   │                                                      │
   └──────────────────────────────────────────────────────┘
```

### Currying

```
   ┌──────────────────────────────────────────────────────┐
   │                   CURRYING                           │
   ├──────────────────────────────────────────────────────┤
   │                                                      │
   │  fn add(x: i32) -> impl Fn(i32) -> i32 {             │
   │      move |y| x + y                                  │
   │  }                                                   │
   │                                                      │
   │  add(5)         → closure with x=5                   │
   │  add(5)(3)      → 8                                  │
   │                                                      │
   │  Visualize:                                          │
   │                                                      │
   │  add(5)                                              │
   │    │                                                 │
   │    │ partial application                             │
   │    ▼                                                 │
   │  ┌─────────────────────┐                             │
   │  │ closure(y) {        │                             │
   │  │   return 5 + y      │  ← x=5 captured             │
   │  │ }                   │                             │
   │  └─────────────────────┘                             │
   │           │                                          │
   │           │ apply 3                                  │
   │           ▼                                          │
   │         8                                            │
   │                                                      │
   └──────────────────────────────────────────────────────┘
```

---

## 19. Closure + thread + move

```rust
use std::thread;

let data = vec![1, 2, 3];
let handle = thread::spawn(move || {
    println!("{:?}", data);
});
handle.join().unwrap();
```

### Sơ đồ flow

```
   Main thread:                       Spawned thread:
   ──────────                          ──────────
   
   data = vec![1,2,3]
   ┌────────────────┐
   │ STACK:         │
   │  data ─────────┼──► HEAP [1,2,3]
   └────────────────┘
   
   thread::spawn(move || ...)
        │
        │ MOVE: data transferred
        ▼
   ┌────────────────┐                 ┌────────────────┐
   │ STACK (main):  │                 │ STACK (new):   │
   │  data: DEAD    │                 │  closure {     │
   │  handle ───────┼─┐               │    data ───────┼──► HEAP
   └────────────────┘ │               │  }             │    [1,2,3]
                       │               └────────────────┘
                       │ JoinHandle
                       └► (synchronization)
   
   ─────── time ───────►
   main: handle.join()
                                       println!("{:?}", data);
                                       (executes)
                                       
                                       thread exits
                                       ┌────────────────┐
                                       │ (frame popped) │
                                       │ data drops:    │
                                       │   free HEAP    │
                                       └────────────────┘
   main: join returns
```

### Vì sao cần `move`?

```
   Without move:
   ────
   thread::spawn(|| println!("{:?}", data));
                                     ▲▲▲▲
                                     vay &data
   
   nhưng thread có thể chạy LÂU HƠN main scope chứa data
   → &data có thể dangling
   → COMPILE ERROR
   
   With move:
   ────
   thread::spawn(move || println!("{:?}", data));
                  ▲▲▲▲             ▲▲▲▲
                  ép move          dùng data (owned by closure)
   
   data sống cùng closure
   closure sống cùng thread
   → safe
```

---

## 20. Stateful closure (counter)

```rust
fn make_counter() -> impl FnMut() -> i32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

let mut c = make_counter();
println!("{}", c());   // 1
println!("{}", c());   // 2
println!("{}", c());   // 3
```

### Sơ đồ state

```
   make_counter() returned closure cl:
   ─────────────
   
   STACK (caller):
   ┌─────────────────────┐
   │ c: __Closure_state  │
   │   count: 0          │  ← initial state, OWNED by closure
   │                     │
   │   call(&mut self) { │
   │     self.count += 1;│
   │     self.count       │
   │   }                  │
   └─────────────────────┘
   
   
   Mỗi lần c():
   ─────
   
   Time 1: c() →  self.count: 0 → 1  → return 1
   Time 2: c() →  self.count: 1 → 2  → return 2
   Time 3: c() →  self.count: 2 → 3  → return 3
   
   State PERSIST giữa các call!
   → FnMut (sửa state qua &mut self)
```

### State machine visual

```
   ┌─────────────────────────┐
   │  Closure (counter):      │
   │  count = 0               │
   └────────────┬─────────────┘
                │ c()
                ▼
   ┌─────────────────────────┐
   │  count = 1               │  → return 1
   └────────────┬─────────────┘
                │ c()
                ▼
   ┌─────────────────────────┐
   │  count = 2               │  → return 2
   └────────────┬─────────────┘
                │ c()
                ▼
   ┌─────────────────────────┐
   │  count = 3               │  → return 3
   └─────────────────────────┘
```

---

## 21. Decision tree

```
                  ┌──────────────────────────────────────┐
                  │   TÔI CẦN CLOSURE                    │
                  └────────────────┬─────────────────────┘
                                   │
                                   ▼
                  ┌──────────────────────────────────────┐
                  │ Capture mode?                        │
                  └────────────────┬─────────────────────┘
                                   │
            ┌──────────────────────┼──────────────────────┐
            ▼                      ▼                      ▼
        Chỉ đọc                  Sửa                Consume/move
            │                      │                      │
            ▼                      ▼                      ▼
       Capture &              Capture &mut         Capture by value
       impl Fn                impl FnMut           impl FnOnce
       
                                   │
                                   ▼
                  ┌──────────────────────────────────────┐
                  │ Cần pass closure?                    │
                  └────────────────┬─────────────────────┘
                                   │
            ┌──────────────────────┼──────────────────────┐
            ▼                      ▼                      ▼
       Single call site       Lưu collection       Plugin/dynamic
       Performance crit       Heterogeneous
            │                      │                      │
            ▼                      ▼                      ▼
       fn f<F: Fn>(f: F)     Vec<Box<dyn Fn>>      Box<dyn Fn>
       (monomorph, fast)     (1 type chung)        (return + trait obj)
       
                                   │
                                   ▼
                  ┌──────────────────────────────────────┐
                  │ Cần return closure?                  │
                  └────────────────┬─────────────────────┘
                                   │
            ┌──────────────────────┼──────────────────────┐
            ▼                      ▼                      ▼
       1 type cố định        Branch trả khác       Cần dyn dispatch
            │                      │                      │
            ▼                      ▼                      ▼
       impl Fn               Box<dyn Fn>           Box<dyn Fn>
       (stack, 0-cost)       (heap, dyn)           
       
                                   │
                                   ▼
                  ┌──────────────────────────────────────┐
                  │ Closure cho thread?                  │
                  └────────────────┬─────────────────────┘
                                   │
                                   ▼
                          thread::spawn(move || ...)
                          
                          - move: ép capture by value
                          - 'static: closure không vay local
                          - Send: captured vars Send
```

---

## 22. Common errors visual

### Error 1: Closure outlive function

```
   ┌────────────────────────────────────────────────┐
   │ fn make() -> impl Fn() {                       │
   │     let x = vec![1,2];                         │
   │     || println!("{:?}", x)        ❌            │
   │ }                                              │
   │                                                │
   │ Visualize:                                     │
   │                                                │
   │ make scope:                                    │
   │ ┌───────────────────┐                          │
   │ │ x ──► [1,2]       │                          │
   │ │ closure (vay &x) ──┼──► return!              │
   │ └───────────────────┘                          │
   │       ↓ x DROP                                 │
   │                                                │
   │       closure ngoài đời, vay &x DANGLING!      │
   │                                                │
   │ FIX: move keyword                              │
   │                                                │
   │ fn make() -> impl Fn() {                       │
   │     let x = vec![1,2];                         │
   │     move || println!("{:?}", x)   ✓            │
   │ }                                              │
   │                                                │
   │ ┌───────────────────┐                          │
   │ │ closure {         │                          │
   │ │   x: vec![1,2]    │  ← x lives IN closure    │
   │ │ }                 │                          │
   │ └───────────────────┘                          │
   │                                                │
   └────────────────────────────────────────────────┘
```

### Error 2: FnOnce called twice

```
   ┌────────────────────────────────────────────────┐
   │ let s = String::from("hi");                    │
   │ let cl = move || drop(s);                      │
   │ cl();   ✓ first call, drops s                  │
   │ cl();   ❌ ERROR: cl is FnOnce, can't reuse    │
   │                                                │
   │ Lý do:                                         │
   │                                                │
   │ Call 1:                                        │
   │ ┌──────────────┐                               │
   │ │ cl { s }     │ → drop(s) → s freed           │
   │ └──────────────┘                               │
   │                                                │
   │ Call 2:                                        │
   │ ┌──────────────┐                               │
   │ │ cl { s? }    │ ← s đã free, sai!             │
   │ └──────────────┘                               │
   │                                                │
   │ → Compiler CẤM gọi lần 2                       │
   └────────────────────────────────────────────────┘
```

### Error 3: Borrow conflict

```
   ┌────────────────────────────────────────────────┐
   │ let v = vec![1,2,3];                           │
   │ let cl = || println!("{:?}", v);   // & v      │
   │ v.push(4);                          ❌ &mut v   │
   │ cl();                                          │
   │                                                │
   │ Timeline:                                      │
   │                                                │
   │ v alive ─────────────────────────────►         │
   │ cl alive    ─────────────────────────►         │
   │             ↑                ↑                 │
   │             cl captures &v   uses cl           │
   │                                                │
   │              ↓                                  │
   │              v.push(4)  ← cần &mut             │
   │                          ❌ xung đột với &v    │
   │                                                │
   │ FIX: dùng cl xong rồi mới push                 │
   │                                                │
   │ cl();             ← cl last use                │
   │ v.push(4);        ✓ cl đã chết (NLL)           │
   └────────────────────────────────────────────────┘
```

---

## 23. Mind map

```
                              CLOSURE
                                 │
       ┌─────────────────────────┼─────────────────────────┐
       ▼                         ▼                         ▼
   SYNTAX                    SEMANTICS                  USAGE
                                                            
   |args| body              capture env                pass/return
   (anonymous)              by &/&mut/move             generic/dyn
                            move keyword               struct field
                                                       
                                ┌────┴────┐
                                ▼         ▼
                              Fn      Compiler
                             FnMut    auto-generates
                             FnOnce   struct + impl
                             
                             Fn ⊆ FnMut ⊆ FnOnce
                             
                             
       ┌─────────────────────────────────────────────────┐
       │                                                 │
       │   CLOSURE = STRUCT + TRAIT IMPL                 │
       │                                                 │
       │   struct __Closure_N {                          │
       │       field_1, field_2, ... (captured vars)     │
       │   }                                             │
       │   impl Fn/FnMut/FnOnce for __Closure_N { ... }  │
       │                                                 │
       │   Mỗi closure literal = UNIQUE type             │
       │   Size = sum of captured vars                   │
       │                                                 │
       └─────────────────────────────────────────────────┘
       
       
       ┌─────────────────────────────────────────────────┐
       │                                                 │
       │   PATTERNS                                      │
       │                                                 │
       │   Iterator combinators (map, filter, fold)      │
       │   Higher-order functions (compose, currying)    │
       │   Callbacks (event-driven)                      │
       │   Stateful generators (counter, RNG)            │
       │   Lazy evaluation                               │
       │   Decorators (logging, retry)                   │
       │                                                 │
       └─────────────────────────────────────────────────┘
       
       
       ┌─────────────────────────────────────────────────┐
       │                                                 │
       │   QUAN HỆ                                       │
       │                                                 │
       │   Closure & TRAIT      → Fn/FnMut/FnOnce trait  │
       │   Closure & GENERIC    → F: Fn bound           │
       │   Closure & OWNERSHIP  → capture mode            │
       │   Closure & MEMORY     → struct ẩn size variable │
       │                                                 │
       └─────────────────────────────────────────────────┘
```

---

## Tổng kết — 10 ý cốt lõi visual

```
   ╔══════════════════════════════════════════════════════════╗
   ║                                                          ║
   ║  1. Closure = function + captured environment             ║
   ║                                                          ║
   ║  2. Cú pháp: |args| body                                 ║
   ║                                                          ║
   ║  3. 3 capture modes:                                     ║
   ║     by &     → Fn                                        ║
   ║     by &mut  → FnMut                                     ║
   ║     by move  → FnOnce (consume)                          ║
   ║                                                          ║
   ║  4. Compiler chọn mode YẾU NHẤT đủ dùng                  ║
   ║                                                          ║
   ║  5. move keyword: ép capture by value                    ║
   ║     (cần khi: thread, return closure, 'static)           ║
   ║                                                          ║
   ║  6. Closure = struct ẩn + impl Fn/FnMut/FnOnce           ║
   ║     Mỗi literal = UNIQUE type                            ║
   ║     Size = sum of captured vars                          ║
   ║                                                          ║
   ║  7. Fn ⊆ FnMut ⊆ FnOnce (subtype chain)                  ║
   ║     Hàm nhận FnOnce → linh hoạt nhất                     ║
   ║                                                          ║
   ║  8. Pass closure:                                        ║
   ║     <F: Fn>(f: F)   → generic, fast, monomorph           ║
   ║     &dyn Fn         → dynamic, 1 binary, ~2-3ns extra    ║
   ║                                                          ║
   ║  9. Return closure:                                      ║
   ║     impl Fn         → 1 type, stack, 0-cost              ║
   ║     Box<dyn Fn>     → multiple types qua branch, heap    ║
   ║                                                          ║
   ║ 10. Send/Sync tự suy từ captured vars                    ║
   ║                                                          ║
   ╚══════════════════════════════════════════════════════════╝
```

---

> Đọc song song `closure.md` (lý thuyết) để hiểu sâu, file này để có hình ảnh.
>
> Chủ đề tiếp theo: **Async/Await** — Future trait, polling, Pin, executor (tokio), state machine generation, async closure.
