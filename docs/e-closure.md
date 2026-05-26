# Closure — Từ Bản Chất Đến Nâng Cao

> Closure là **function được nâng cấp**: nó có thể "nhớ" biến từ scope bao quanh. Closure là cách Rust làm functional programming, higher-order function, iterator chain, callback. Hiểu closure = hiểu cách Rust hợp nhất ownership + trait + generic vào 1 khái niệm duy nhất.

---

## Mục lục

**Tầng 1 — Vì sao Closure tồn tại?**
1. [Bài toán: function nhớ context](#1-bài-toán)
2. [Closure trong các ngôn ngữ khác](#2-closure-khác)
3. [Closure trong Rust: triết lý](#3-closure-rust)

**Tầng 2 — Closure cơ bản**
4. [Cú pháp closure](#4-cú-pháp)
5. [Type inference của closure](#5-type-inference)
6. [Closure không phải function](#6-không-phải-function)

**Tầng 3 — Capture Environment**
7. [3 cách capture](#7-3-cách-capture)
8. [Compiler tự chọn capture](#8-compiler-chọn)
9. [Keyword `move`](#9-move-keyword)
10. [Capture chi tiết: từng field](#10-capture-field)

**Tầng 4 — Fn / FnMut / FnOnce**
11. [3 trait hierarchy](#11-3-trait-hierarchy)
12. [FnOnce — consume](#12-fnonce)
13. [FnMut — mutate](#13-fnmut)
14. [Fn — read-only](#14-fn)
15. [Quan hệ Fn ⊆ FnMut ⊆ FnOnce](#15-quan-hệ-3-trait)

**Tầng 5 — Closure là gì thực sự?**
16. [Closure = struct ẩn + trait impl](#16-closure-là-struct)
17. [Mỗi closure có TYPE riêng](#17-unique-type)
18. [Size của closure](#18-size-closure)
19. [Closure trong memory](#19-closure-memory)

**Tầng 6 — Truyền và Trả về Closure**
20. [Pass closure: generic vs dyn](#20-pass-closure)
21. [Return closure: impl Fn vs Box dyn Fn](#21-return-closure)
22. [Closure trong struct field](#22-closure-trong-struct)
23. [Higher-order functions](#23-higher-order)

**Tầng 7 — Closure & Ownership**
24. [Closure & borrowing rules](#24-borrowing)
25. [Closure & lifetime](#25-lifetime)
26. [Move closure cho thread](#26-move-thread)
27. [Closure & Send/Sync](#27-send-sync)

**Tầng 8 — Patterns**
28. [Iterator combinators](#28-iterator-combinators)
29. [Callback pattern](#29-callback)
30. [Builder với closure](#30-builder-closure)
31. [Lazy evaluation](#31-lazy-evaluation)
32. [Decorator / middleware](#32-decorator)

**Tầng 9 — Closure nâng cao**
33. [Closure & async](#33-async-closure)
34. [Closure recursion (Y combinator)](#34-recursion)
35. [Closure tự tham chiếu state](#35-stateful)
36. [Type erasure cho closure](#36-type-erasure)

**Tầng 10 — Bẫy thường gặp**
37. [Common closure errors](#37-common-errors)

---

# TẦNG 1 — VÌ SAO CLOSURE TỒN TẠI?

## 1. Bài toán

Function thường **không có state**:

```rust
fn add(a: i32, b: i32) -> i32 { a + b }
add(1, 2);   // 3
```

Mỗi lần gọi, function "khởi tạo lại từ đầu" — không nhớ gì.

### Bài toán: cần function nhớ context

```
   Tôi muốn function tăng số theo một bước cố định:
   
   inc_by_3(5)  → 8
   inc_by_3(10) → 13
   
   Bước "3" là context bên ngoài function.
   
   Cách viết:
   ────────
   fn inc_by_3(x: i32) -> i32 { x + 3 }     ← hardcode!
   
   Nhưng nếu tôi muốn inc_by_5, inc_by_7, ...?
   → Phải viết 1 function/case.
```

→ **Cần** "factory" sinh function với context khác nhau.

### Closure giải quyết

```rust
fn make_incrementer(step: i32) -> impl Fn(i32) -> i32 {
    move |x| x + step                  // ← closure nhớ step
}

let inc3 = make_incrementer(3);
let inc5 = make_incrementer(5);

inc3(10);    // 13
inc5(10);    // 15
```

→ `step` là **environment** mà closure capture được.

---

## 2. Closure trong các ngôn ngữ khác

### JavaScript

```javascript
function makeAdder(x) {
    return function(y) { return x + y; }
}
const add5 = makeAdder(5);
add5(3);   // 8
```

→ Dễ dùng, nhưng:
- `x` lưu ở đâu? — heap (GC quản lý)
- Closure = object có hidden field

### C++

```cpp
auto make_adder = [](int x) {
    return [x](int y) { return x + y; };
};
auto add5 = make_adder(5);
add5(3);   // 8
```

→ Cú pháp `[x]` = "capture x". C++ rất control:
- `[x]`: capture by value
- `[&x]`: capture by reference
- `[=]`: capture all by value
- `[&]`: capture all by reference

### Python

```python
def make_adder(x):
    return lambda y: x + y

add5 = make_adder(5)
add5(3)   # 8
```

→ Tự capture by reference, GC quản lý.

### Java (Java 8+)

```java
IntFunction<Integer> makeAdder(int x) {
    return y -> x + y;
}
```

→ Closure phải capture **final** (immutable) variables → hạn chế.

### Rust: kết hợp

```rust
let make_adder = |x: i32| move |y: i32| x + y;
let add5 = make_adder(5);
add5(3);   // 8
```

→ Cú pháp `|args| body`, capture **tự động**, `move` keyword cho explicit move.

---

## 3. Closure trong Rust: triết lý

### 3 nguyên tắc

```
   1. Closure là function + state (captured environment)
   
   2. Capture mode (by ref / by mut ref / by move) 
      ↔ tương ứng với trait impl (Fn / FnMut / FnOnce)
   
   3. Mỗi closure có UNIQUE type (compiler-generated struct)
```

### Đẹp ở đâu?

```
   Closure trong Rust:
   ──────────
   - Zero-cost abstraction (giống function thường)
   - Borrow checker check capture
   - Trait system phân loại tự nhiên
   - Có thể inline hoặc dyn
   - Send/Sync auto-derive theo captured vars
```

### So với function pointer

```rust
fn pure_fn(x: i32) -> i32 { x + 1 }       // function POINTER
let cl = |x: i32| x + 1;                  // closure

// Cả 2 callable:
pure_fn(5);
cl(5);

// Khác: closure có thể capture state, function không.
```

---

# TẦNG 2 — CLOSURE CƠ BẢN

## 4. Cú pháp closure

```rust
// Closure nhỏ nhất:
let say_hi = || println!("hi");
say_hi();   // "hi"

// Có argument:
let add = |a, b| a + b;
add(1, 2);  // 3

// Có type annotation:
let add: fn(i32, i32) -> i32 = |a, b| a + b;
// hoặc:
let add = |a: i32, b: i32| -> i32 { a + b };

// Body nhiều dòng:
let process = |x| {
    let y = x * 2;
    y + 1
};

// Không có argument:
let const_fn = || 42;
const_fn();   // 42
```

### So với function

```
   FUNCTION:                          CLOSURE:
   ─────────                          ────────
   
   fn name(args) -> Ret { body }      |args| body
   
   - Toàn cục                         - Local
   - Không capture                    - Có thể capture
   - Có tên                           - Thường anonymous (let assigns name)
   - Phải khai báo type               - Thường suy ra
```

---

## 5. Type inference của closure

Compiler tự suy ra:

```rust
let cl = |x| x + 1;
cl(5);                  // → suy ra x: i32

let cl = |x| x.to_string();
cl("hi");               // → x: &str
cl(5);                  // ❌ ERROR — closure đã "lock" với &str
```

### Lock type sau dùng lần đầu

```rust
let cl = |x| x;
cl("hi");              // x: &str
cl(5);                 // ❌ — sau dùng &str, không đổi sang i32 được
```

```
   ┌──────────────────────────────────────────────┐
   │ Closure được "polymorphism" CHỈ TRONG        │
   │ phạm vi: chưa dùng lần nào.                  │
   │                                              │
   │ Sau lần dùng đầu → type LOCK.                │
   │                                              │
   │ Vì closure đã được monomorphize.             │
   └──────────────────────────────────────────────┘
```

### Muốn generic? Dùng function

```rust
fn identity<T>(x: T) -> T { x }     // generic over T

let cl: fn(i32) -> i32 = identity;
let cl: fn(&str) -> &str = identity;   // 2 monomorphized versions
```

---

## 6. Closure không phải function

```rust
fn takes_fn(f: fn(i32) -> i32) -> i32 {
    f(5)
}

let cl = |x| x + 1;
takes_fn(cl);          // ✓ OK nếu closure KHÔNG capture

let y = 10;
let cl2 = |x| x + y;   // capture y!
takes_fn(cl2);         // ❌ ERROR — cl2 không phải `fn`
```

```
   error: closures can only be coerced to `fn` types if they do not capture any variables
```

```
   fn (function pointer):       1 pointer (8 byte)
                                không capture
   
   closure:                      struct chứa captured vars
                                 size phụ thuộc captured vars
```

→ **Function pointer ⊆ Closure** (function = closure không capture).

---

# TẦNG 3 — CAPTURE ENVIRONMENT

## 7. 3 cách capture

Closure có thể capture variable theo **3 cách**:

```rust
let x = String::from("hi");

// Cách 1: by REFERENCE (&)
let cl = || println!("{}", x);   // & x

// Cách 2: by MUTABLE REFERENCE (&mut)
let mut x = vec![1, 2];
let mut cl = || x.push(3);       // &mut x

// Cách 3: by VALUE / MOVE
let x = String::from("hi");
let cl = move || println!("{}", x);   // move x in
```

### Tương ứng với trait

```
   Capture mode               Closure implements
   ─────────────              ──────────────────
   by &           ──────────► Fn  (đọc, gọi nhiều lần)
   by &mut        ──────────► FnMut (sửa, gọi nhiều lần qua &mut)
   by move        ──────────► FnOnce (consume, gọi 1 lần)
   
   (Nếu captured là Copy, move không "kill" gốc)
```

---

## 8. Compiler tự chọn capture

Rust **lười nhất có thể**: chọn cách yếu nhất đủ dùng.

```rust
let x = 5;

let cl1 = || println!("{}", x);
// Compiler: chỉ đọc x → capture by & (Fn)

let mut y = vec![1, 2];
let mut cl2 = || y.push(3);
// Compiler: cần sửa y → capture by &mut (FnMut)

let z = String::from("hi");
let cl3 = || drop(z);
// Compiler: consume z → capture by move (FnOnce)
```

### Compiler nâng cấp khi cần

```rust
let mut x = 5;
let mut cl = || { x += 1; println!("{}", x); };
// x được DÙNG cả đọc lẫn ghi → capture by &mut (FnMut)
```

---

## 9. Keyword `move`

`move` ép closure capture **by value/move**, kể cả khi không cần.

```rust
let x = String::from("hi");
let cl = move || println!("{}", x);
// x đã move vào cl
// println!("{}", x);   ❌ x đã move
```

### Khi nào cần `move`?

#### Case 1: thread

```rust
let x = vec![1, 2, 3];
std::thread::spawn(|| {
    println!("{:?}", x);          // ❌
});
```

```
   error: closure may outlive function, but borrows `x`
   help: use `move` keyword
```

Vì thread có thể chạy lâu hơn caller — x phải move vào.

```rust
std::thread::spawn(move || {
    println!("{:?}", x);          // ✓
});
```

#### Case 2: return closure

```rust
fn make_counter() -> impl FnMut() -> i32 {
    let mut count = 0;
    move || { count += 1; count }   // ← move count vào closure
}
```

Không `move` → closure capture &count, nhưng `count` chết khi `make_counter` return → dangling.

#### Case 3: explicit ownership

```rust
let buf = vec![0u8; 1024];
let cl = move || process(buf);    // explicit: cl SỞ HỮU buf
```

---

## 10. Capture chi tiết: từng field

Rust 2021 hỗ trợ **disjoint capture**: capture từng field riêng.

```rust
struct Big {
    a: String,
    b: String,
}

let big = Big { a: "hi".into(), b: "hello".into() };

let cl = || println!("{}", big.a);
// Trước 2021: capture toàn bộ big
// Sau 2021: chỉ capture big.a

println!("{}", big.b);    // ✓ trong Rust 2021 (b chưa bị capture)
```

```
   Trước 2021:                       Sau 2021:
   ──────────                         ────────
   
   cl capture big                    cl capture big.a only
       │                                 │
       ▼                                 ▼
   ┌─────────┐                       ┌─────────┐
   │ a, b    │                       │ a only  │
   └─────────┘                       └─────────┘
                                     b vẫn dùng được!
```

---

# TẦNG 4 — FN / FNMUT / FNONCE

## 11. 3 trait hierarchy

```rust
trait FnOnce<Args> {
    type Output;
    fn call_once(self, args: Args) -> Self::Output;
}

trait FnMut<Args>: FnOnce<Args> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
}

trait Fn<Args>: FnMut<Args> {
    fn call(&self, args: Args) -> Self::Output;
}
```

### Hierarchy

```
   FnOnce  (parent — gọi 1 lần, consume)
     ▲
     │
   FnMut   (gọi nhiều lần qua &mut)
     ▲
     │
   Fn      (gọi nhiều lần qua &)
```

→ `Fn` là subtype của `FnMut`, `FnMut` là subtype của `FnOnce`.

### Hệ quả

```
   Closure implements Fn         → có thể coerce thành FnMut và FnOnce
   Closure implements FnMut      → có thể coerce thành FnOnce
   Closure implements FnOnce     → CHỈ FnOnce
```

---

## 12. FnOnce — consume

```rust
fn call_once<F: FnOnce() -> i32>(f: F) -> i32 {
    f()
}

let s = String::from("hi");
let cl = move || { drop(s); 42 };   // s bị consume
call_once(cl);
// call_once(cl);   ❌ cl chỉ gọi 1 lần
```

### Khi closure là FnOnce?

```
   Closure là FnOnce khi:
   ─────
   - Move OUT của captured variable
     (drop, return value...)
   - Bất cứ thao tác nào "consume" capture
```

### Signature

```rust
fn call_once<F>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()           // gọi đúng 1 lần
}
```

→ Hàm nhận closure bằng **value** (move).

---

## 13. FnMut — mutate

```rust
fn call_many<F: FnMut()>(mut f: F, n: usize) {
    for _ in 0..n {
        f();
    }
}

let mut count = 0;
call_many(|| count += 1, 5);
// count = 5
```

### Khi closure là FnMut?

```
   Closure là FnMut khi:
   ─────
   - Sửa captured variable
   - Không consume (không move out)
```

### Signature

```rust
fn call_many<F>(mut f: F)
where
    F: FnMut(),
{
    f();          // gọi qua &mut self
    f();          // ✓ vẫn gọi được
}
```

---

## 14. Fn — read-only

```rust
fn call_with_5<F: Fn(i32) -> i32>(f: F) -> i32 {
    f(5)
}

let factor = 3;
let cl = |x| x * factor;     // chỉ đọc factor
let r = call_with_5(cl);     // 15
println!("{}", r);
// call_with_5(cl);          // ✓ Fn gọi nhiều lần
```

### Khi closure là Fn?

```
   Closure là Fn khi:
   ──
   - CHỈ đọc captured variable
   - Không sửa, không consume
```

### Signature

```rust
fn call_with_5<F>(f: F) -> i32
where
    F: Fn(i32) -> i32,
{
    f(5);    // gọi qua &self
    f(10);   // ✓ shared ref, không độc quyền
}
```

---

## 15. Quan hệ Fn ⊆ FnMut ⊆ FnOnce

```
                FnOnce
                 ▲
       ┌─────────┤
       │         │
     FnMut       │
       ▲         │
       │         │
       │         │
      Fn ────────┘
   
   Fn  ⊆  FnMut  ⊆  FnOnce
   "yếu nhất"        "mạnh nhất"
```

### Practical: nhận closure càng "yếu" càng linh hoạt

```rust
// Linh hoạt nhất (nhận MỌI closure):
fn flexible<F: FnOnce()>(f: F) { ... }

// Trung bình:
fn moderate<F: FnMut()>(f: F) { ... }

// Ràng buộc nhất (chỉ Fn):
fn strict<F: Fn()>(f: F) { ... }
```

```
   Caller có Fn   → strict ✓  moderate ✓  flexible ✓
   Caller có FnMut → strict ❌  moderate ✓  flexible ✓
   Caller có FnOnce → strict ❌  moderate ❌  flexible ✓
```

### Pattern thực dụng

```rust
// CALLBACK gọi 1 lần (cleanup, completion):
fn on_complete<F: FnOnce()>(f: F) { ... }

// CALLBACK gọi nhiều lần (loop, retry):
fn on_event<F: FnMut(Event)>(f: F) { ... }

// READ-ONLY CALLBACK (multiple thread, đơn giản):
fn on_read<F: Fn(&Data)>(f: F) { ... }
```

---

# TẦNG 5 — CLOSURE LÀ GÌ THỰC SỰ?

## 16. Closure = struct ẩn + trait impl

Bí mật: closure trong Rust thực ra là **struct anonymous** + `impl FnOnce/FnMut/Fn`.

### Bạn viết

```rust
let x = 10;
let factor = 3;
let cl = |y| y * factor + x;
```

### Compiler sinh ra

```rust
// Struct ẩn (anonymous, không có tên thực)
struct __ClosureN {
    factor: i32,    // capture by value (or & nếu compiler chọn)
    x: i32,
}

impl Fn(i32) -> i32 for __ClosureN {
    fn call(&self, args: (i32,)) -> i32 {
        let (y,) = args;
        y * self.factor + self.x
    }
}

// Tạo instance
let cl = __ClosureN { factor: 3, x: 10 };
```

### Lúc gọi

```rust
cl(5);    
// equivalent to:
__ClosureN::call(&cl, (5,));
```

---

## 17. Mỗi closure có UNIQUE type

**Mỗi** closure literal trong source code → **type khác nhau**.

```rust
let cl1 = || 42;
let cl2 = || 42;

// cl1 và cl2 có TYPE KHÁC NHAU
// Dù body giống hệt!

let v: Vec<???> = vec![cl1, cl2];   // ❌ type mismatch
```

### Visualize

```
   Source code:                       Compiler sinh:
   ─────────                         ──────────
   
   let cl1 = || 42;                  struct __Closure_line5;
                                     impl Fn() -> i32 for __Closure_line5 {...}
                                     let cl1: __Closure_line5 = ...;
   
   let cl2 = || 42;                  struct __Closure_line6;
                                     impl Fn() -> i32 for __Closure_line6 {...}
                                     let cl2: __Closure_line6 = ...;
   
   2 type khác nhau!
```

### Cách giải quyết: trait object hoặc generic

```rust
// Generic — vẫn KHÔNG vào 1 Vec (vì khác type):
fn run<F: Fn() -> i32>(f: F) { ... }
run(cl1);  // ✓ monomorph cho __Closure_line5
run(cl2);  // ✓ monomorph cho __Closure_line6

// Trait object — cho phép chung Vec:
let v: Vec<Box<dyn Fn() -> i32>> = vec![
    Box::new(cl1),
    Box::new(cl2),
];
```

---

## 18. Size của closure

Size = size của captured variables (cộng padding/align).

```rust
let cl1 = || println!("hi");
// Không capture → __Closure size = 0!

let x: i32 = 5;
let cl2 = || println!("{}", x);
// Capture &x → __Closure size = 8 (1 ref)

let s = String::from("hi");
let cl3 = move || println!("{}", s);
// Capture s by move → __Closure size = 24 (String header)

let a = 1u8;
let b = 2u64;
let cl4 = move || (a, b);
// Capture a, b by move → __Closure size = 16 (u8 + padding + u64)
```

### So sánh với function pointer

```
   fn pointer:                       Closure size:
   ──────────                         ──────────
   8 byte (1 ptr)                    0..N byte (captured)
   
   ┌──────────┐                      ┌──────────┐
   │ fn ptr   │                      │ field 1  │
   │ (8 B)    │                      │ field 2  │
   └──────────┘                      │  ...     │
                                     └──────────┘
```

---

## 19. Closure trong memory

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

### Layout cụ thể

```
   STACK của main:
   ┌──────────────┐
   │ x = 10       │
   ├──────────────┤
   │ s.ptr ───────┼──► HEAP "hello"
   │ s.len = 5    │
   │ s.cap = 5    │
   ├──────────────┤
   │ cl: __Cl {   │
   │   x: 10      │    ← capture by COPY (i32 is Copy)
   │   s: ────────┼──► HEAP "hello" (MOVED here)
   │   s.len: 5   │
   │   s.cap: 5   │
   │ }            │
   └──────────────┘
   
   Sau move: s ở main vô hiệu, s trong closure sở hữu heap data.
   
   Khi cl drop → drop closure → drop s_in_cl → free heap "hello"
```

---

# TẦNG 6 — TRUYỀN VÀ TRẢ VỀ CLOSURE

## 20. Pass closure: generic vs dyn

### Generic (static dispatch, fast, monomorphize)

```rust
fn run<F>(f: F) where F: Fn() -> i32 {
    let r = f();
    println!("{}", r);
}

let cl = || 42;
run(cl);   // monomorphize cho type của cl
```

### Trait object (dynamic dispatch, dyn Fn)

```rust
fn run(f: &dyn Fn() -> i32) {
    let r = f();
    println!("{}", r);
}

let cl = || 42;
run(&cl);   // pass reference, 1 binary
```

### So sánh

```
   GENERIC                          DYN
   ──────                            ───
   fn run<F: Fn()>(f: F)            fn run(f: &dyn Fn())
   
   ⚡ Fast (inline)                  ⚡ ~2-3 ns extra
   📦 Code bloat                     📦 Small
   Multi-instantiate                 1 version
   
   Closure pass by VALUE              Closure pass by REF
   (move into f)                      (&cl)
```

### Khi cần Box<dyn>

```rust
// Lưu closure vào struct → cần type cụ thể:
struct Handler {
    callback: Box<dyn Fn(Event)>,    // ← box để có size cố định
}
```

---

## 21. Return closure: impl Fn vs Box dyn Fn

### Return generic không được

```rust
fn make_adder<F>() -> F where F: Fn(i32) -> i32 {
    |x| x + 1                              // ❌ ERROR
}
```

```
   error: type parameter `F` is determined by caller
```

Caller không biết type cụ thể của closure trong body → impossible.

### Cách 1: `impl Trait` (Rust 1.26+)

```rust
fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

let add5 = make_adder(5);
add5(3);   // 8
```

→ Compiler tự "chôn" type closure. Caller gọi qua trait method.

### Cách 2: `Box<dyn Trait>` (heap, dynamic)

```rust
fn make_adder(n: i32) -> Box<dyn Fn(i32) -> i32> {
    Box::new(move |x| x + n)
}
```

→ Heap allocation, dyn dispatch.

### Khi nào dùng cái nào?

```
   impl Fn:                           Box<dyn Fn>:
   ───────                             ────────────
   
   - Stack alloc                      - Heap alloc
   - Zero-cost                        - Có cost (vtable)
   - Type ẩn (compiler biết)          - Type-erased
   - Mỗi call site monomorphize       - 1 type chung
   
   Use case:                           Use case:
   - Function nhỏ                      - Lưu vào Vec, struct
   - Inline được                       - Recursive types
   - Performance critical              - Plugin system
```

### Pattern: return closure chứa branch

```rust
// Trả 2 closure khác nhau → KHÔNG dùng impl Fn được
// (vì impl Fn ép 1 type)

fn make_op(plus: bool) -> Box<dyn Fn(i32, i32) -> i32> {
    if plus {
        Box::new(|a, b| a + b)
    } else {
        Box::new(|a, b| a - b)
    }
}
```

```rust
// impl Fn KHÔNG được:
fn make_op(plus: bool) -> impl Fn(i32, i32) -> i32 {
    if plus { |a, b| a + b }              // closure type 1
    else { |a, b| a - b }                 // closure type 2 — KHÁC!
}
// ❌ ERROR: hai branch trả về type KHÁC nhau
```

---

## 22. Closure trong struct field

```rust
struct EventHandler<F: Fn(Event)> {
    callback: F,
}

impl<F: Fn(Event)> EventHandler<F> {
    fn new(f: F) -> Self {
        EventHandler { callback: f }
    }
    
    fn fire(&self, e: Event) {
        (self.callback)(e);
    }
}

let h = EventHandler::new(|e| println!("Got: {:?}", e));
h.fire(Event::Click);
```

### Hoặc dùng dyn

```rust
struct EventHandler {
    callback: Box<dyn Fn(Event)>,
}

impl EventHandler {
    fn new(f: impl Fn(Event) + 'static) -> Self {
        EventHandler { callback: Box::new(f) }
    }
}

let h = EventHandler::new(|e| println!("Got: {:?}", e));
```

### Multiple callbacks

```rust
struct EventBus {
    handlers: Vec<Box<dyn Fn(Event)>>,   // ← heterogeneous closures
}

impl EventBus {
    fn add(&mut self, f: impl Fn(Event) + 'static) {
        self.handlers.push(Box::new(f));
    }
    
    fn fire(&self, e: Event) {
        for h in &self.handlers {
            h(e.clone());
        }
    }
}
```

---

## 23. Higher-order functions

Function nhận function (closure) làm argument hoặc trả về function.

### Iterator combinator — kinh điển

```rust
let v = vec![1, 2, 3, 4, 5];

let sum_of_squares: i32 = v.iter()
    .map(|&x| x * x)
    .filter(|&x| x > 5)
    .sum();
// 16 + 25 = 41
```

### Composition

```rust
fn compose<A, B, C, F, G>(f: F, g: G) -> impl Fn(A) -> C
where
    F: Fn(A) -> B,
    G: Fn(B) -> C,
{
    move |x| g(f(x))
}

let add1 = |x: i32| x + 1;
let double = |x: i32| x * 2;
let pipeline = compose(add1, double);
pipeline(5);   // (5+1)*2 = 12
```

### Currying (1-arg per call)

```rust
fn add(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y
}

let add5 = add(5);
add5(3);   // 8
```

---

# TẦNG 7 — CLOSURE & OWNERSHIP

## 24. Closure & borrowing rules

Closure cũng phải tuân thủ borrow checker.

```rust
let mut v = vec![1, 2, 3];
let cl = || println!("{:?}", v);    // capture by &
v.push(4);                           // ❌ borrow checker!
cl();
```

```
   error: cannot borrow `v` as mutable because it is also borrowed as immutable
```

Vì `cl` capture `&v` → v đang được borrow → không thể `push` (cần &mut).

### Fix: dùng cl trước rồi mới push

```rust
let mut v = vec![1, 2, 3];
let cl = || println!("{:?}", v);
cl();                  // dùng cl
v.push(4);             // ✓ cl không alive nữa (NLL)
```

### Mutable borrow

```rust
let mut v = vec![1, 2, 3];
let mut cl = || v.push(4);    // capture by &mut

// println!("{:?}", v);       // ❌ &v xung đột với &mut cl

cl();
println!("{:?}", v);          // ✓ sau cl xong
```

---

## 25. Closure & lifetime

Closure capture reference → có lifetime.

```rust
fn make_printer<'a>(s: &'a str) -> impl Fn() + 'a {
    move || println!("{}", s)
}

let s = String::from("hi");
let p = make_printer(&s);
p();
drop(s);    // s chết
// p();     // ❌ p capture &s, không dùng được nữa
```

### `'static` closure

```rust
fn make_static() -> impl Fn() + 'static {
    let s = String::from("hi");      // owned
    move || println!("{}", s)
}

let p = make_static();
p();   // ✓ p không phụ thuộc local
```

```
   impl Fn() + 'static:
   ──
   Closure không vay gì có lifetime ngắn hơn 'static
   → Có thể chạy bất cứ đâu (vd thread)
```

### Closure trong thread

```rust
let v = vec![1, 2, 3];
std::thread::spawn(move || {
    println!("{:?}", v);
})
.join()
.unwrap();
```

→ `move` đảm bảo closure SỞ HỮU v → không vay gì → 'static.

---

## 26. Move closure cho thread

```rust
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;

let data = Arc::new(Mutex::new(vec![1, 2, 3]));

let mut handles = vec![];
for i in 0..3 {
    let data = Arc::clone(&data);
    handles.push(thread::spawn(move || {
        let mut d = data.lock().unwrap();
        d.push(i);
    }));
}

for h in handles {
    h.join().unwrap();
}
```

### Phân tích

```
   ┌──────────────────────────────────────────────────┐
   │ Main thread:                                      │
   │   data: Arc<Mutex<Vec<i32>>>                      │
   │                                                   │
   │   for i in 0..3:                                  │
   │       let data = Arc::clone(&data);              │  ← clone Arc
   │       thread::spawn(move || { ... });             │  ← move data, i in
   └──────────────────────────────────────────────────┘
                              │
                              ▼
   ┌──────────────────────────────────────────────────┐
   │ Each thread:                                      │
   │   - Owns its own Arc clone (counter inc)         │
   │   - Owns i (copy)                                │
   │   - Locks Mutex → mutates Vec                    │
   │   - Drops Arc → counter dec                      │
   └──────────────────────────────────────────────────┘
```

---

## 27. Closure & Send/Sync

Closure tự động `Send` nếu **mọi captured variable** là `Send`.

```rust
let s = String::from("hi");           // String: Send
let cl = move || println!("{}", s);
// cl: Send ✓

let rc = Rc::new(42);                  // Rc: NOT Send
let cl2 = move || println!("{}", rc);
// cl2: NOT Send

std::thread::spawn(cl2);              // ❌ cl2 không Send
```

→ Compiler tự động propagate Send/Sync.

---

# TẦNG 8 — PATTERNS

## 28. Iterator combinators

```rust
let words = vec!["hello", "world", "rust"];

// Map: transform mỗi item
let upper: Vec<String> = words.iter()
    .map(|s| s.to_uppercase())
    .collect();

// Filter: chọn theo điều kiện
let long: Vec<&&str> = words.iter()
    .filter(|s| s.len() > 4)
    .collect();

// Reduce/fold: tổng hợp
let total_chars: usize = words.iter()
    .map(|s| s.len())
    .sum();

// Find first match
let first_long = words.iter().find(|s| s.len() > 4);

// Any/all
let has_long = words.iter().any(|s| s.len() > 5);
let all_short = words.iter().all(|s| s.len() < 10);
```

### Lazy

```rust
let processed = (1..1_000_000)
    .map(|x| x * 2)                  // chưa chạy
    .filter(|&x| x % 3 == 0)         // chưa chạy
    .take(5)                          // chưa chạy
    .collect::<Vec<i32>>();           // ⚡ trigger, chỉ tính đến khi đủ 5
```

→ Chỉ tính 5 phần tử đầu thoả → không duyệt 1 triệu.

---

## 29. Callback pattern

```rust
struct Button {
    on_click: Box<dyn Fn() + 'static>,
}

impl Button {
    fn new<F: Fn() + 'static>(f: F) -> Self {
        Button { on_click: Box::new(f) }
    }
    
    fn click(&self) {
        (self.on_click)();
    }
}

let counter = Arc::new(Mutex::new(0));
let counter_clone = Arc::clone(&counter);
let btn = Button::new(move || {
    let mut c = counter_clone.lock().unwrap();
    *c += 1;
    println!("Clicked {} times", c);
});

btn.click();
btn.click();
```

---

## 30. Builder với closure

```rust
struct ServerBuilder {
    host: String,
    middleware: Vec<Box<dyn Fn(Request) -> Request>>,
}

impl ServerBuilder {
    fn middleware<F: Fn(Request) -> Request + 'static>(mut self, f: F) -> Self {
        self.middleware.push(Box::new(f));
        self
    }
}

let server = ServerBuilder::new()
    .middleware(|req| { /* log */ req })
    .middleware(|req| { /* auth */ req })
    .build();
```

---

## 31. Lazy evaluation

```rust
struct Lazy<T, F: FnOnce() -> T> {
    init: Option<F>,
    value: Option<T>,
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    fn new(f: F) -> Self {
        Lazy { init: Some(f), value: None }
    }
    
    fn get(&mut self) -> &T {
        if self.value.is_none() {
            let f = self.init.take().unwrap();
            self.value = Some(f());
        }
        self.value.as_ref().unwrap()
    }
}

let mut expensive = Lazy::new(|| {
    println!("computing...");
    42
});

println!("before access");
println!("{}", expensive.get());     // → "computing..." → 42
println!("{}", expensive.get());     // → 42 (cached)
```

(std::sync::OnceLock cho phép pattern này thread-safe.)

---

## 32. Decorator / middleware

```rust
fn logged<F: Fn(i32) -> i32>(f: F) -> impl Fn(i32) -> i32 {
    move |x| {
        let r = f(x);
        println!("called with {}, returned {}", x, r);
        r
    }
}

let add1 = |x| x + 1;
let logged_add1 = logged(add1);
logged_add1(5);    // → "called with 5, returned 6"
```

---

# TẦNG 9 — CLOSURE NÂNG CAO

## 33. Closure & async

```rust
async fn process(items: Vec<i32>) {
    let futures: Vec<_> = items.iter()
        .map(|&x| async move {
            tokio::time::sleep(...).await;
            x * 2
        })
        .collect();
    
    let results = futures::future::join_all(futures).await;
}
```

### Async closure (nightly)

```rust
#![feature(async_closure)]
let cl = async move |x: i32| -> i32 {
    tokio::time::sleep(...).await;
    x * 2
};
```

Trên stable: dùng `|x| async move { ... }` (closure trả về async block).

---

## 34. Closure recursion (Y combinator)

Closure không thể tự gọi chính nó vì... không có tên!

```rust
let fact = |n: u64| -> u64 {
    if n == 0 { 1 } else { n * fact(n - 1) }  // ❌ fact chưa tồn tại
};
```

### Workaround: Y combinator

```rust
fn fix<T, F>(f: F) -> impl Fn(T) -> T
where
    F: Fn(&dyn Fn(T) -> T, T) -> T + 'static,
    T: 'static,
{
    move |x| {
        // Recursion through trait object
        let f_ref = &f;
        let self_ref: &dyn Fn(T) -> T = ???;  // KHÓ!
        ...
    }
}
```

Phức tạp. Thực tế: dùng function thường khi cần recursion.

### Workaround đơn giản: Rc<RefCell<Box<dyn Fn>>>

```rust
use std::cell::RefCell;
use std::rc::Rc;

let fact: Rc<RefCell<Box<dyn Fn(u64) -> u64>>> = 
    Rc::new(RefCell::new(Box::new(|_| 0)));

let fact_clone = Rc::clone(&fact);
*fact.borrow_mut() = Box::new(move |n: u64| -> u64 {
    if n == 0 { 1 } else { n * (fact_clone.borrow())(n - 1) }
});

let r = (fact.borrow())(5);    // 120
```

Phức tạp → đơn giản dùng `fn`.

---

## 35. Closure tự tham chiếu state

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

→ Closure giữ state internal. `FnMut` vì sửa state.

### Multi-state

```rust
fn make_stateful() -> impl FnMut(i32) -> (i32, i32) {
    let mut sum = 0;
    let mut count = 0;
    move |x| {
        sum += x;
        count += 1;
        (sum, count)
    }
}

let mut f = make_stateful();
println!("{:?}", f(5));    // (5, 1)
println!("{:?}", f(3));    // (8, 2)
println!("{:?}", f(7));    // (15, 3)
```

---

## 36. Type erasure cho closure

```rust
// Lưu N closure khác type vào 1 collection
let closures: Vec<Box<dyn Fn() -> i32>> = vec![
    Box::new(|| 1),
    Box::new(|| 2 + 2),
    Box::new(|| { println!("side effect"); 42 }),
];

for c in &closures {
    println!("{}", c());
}
```

→ Box<dyn Fn> "xoá" type → đồng nhất hoá.

### Trade-off

```
   Type erasure (Box<dyn Fn>):       Generic (impl Fn):
   ────────────                       ─────────
   - Heap alloc                       - Stack
   - Vtable lookup                    - Inline
   - Heterogeneous collection         - Homogeneous
   - 1 binary                         - Monomorphize
   - Linh hoạt runtime                - Static
```

---

# TẦNG 10 — BẪY THƯỜNG GẶP

## 37. Common errors

### Error 1: `closure may outlive function`

```rust
fn make() -> impl Fn() {
    let x = vec![1, 2];
    || println!("{:?}", x)              // ❌ x vay, x chết khi make return
}
```

**Fix**: `move`

```rust
fn make() -> impl Fn() {
    let x = vec![1, 2];
    move || println!("{:?}", x)         // ✓
}
```

### Error 2: `closure called twice but is FnOnce`

```rust
let s = String::from("hi");
let cl = move || drop(s);     // FnOnce (consume s)
cl();
cl();                          // ❌ — chỉ 1 lần
```

### Error 3: `cannot borrow as mutable... already borrowed as immutable`

```rust
let v = vec![1, 2];
let cl = || println!("{:?}", v);
v.push(3);                              // ❌ v đang borrow
cl();
```

### Error 4: `expected closure that implements Fn, found one that implements FnMut`

```rust
fn run<F: Fn()>(f: F) { f(); }

let mut x = 0;
let cl = || { x += 1; };           // FnMut
run(cl);                            // ❌ — cần Fn
```

**Fix**: dùng FnMut bound:
```rust
fn run<F: FnMut()>(mut f: F) { f(); }
```

### Error 5: type mismatch giữa 2 closure

```rust
let a = |x: i32| x + 1;
let b = |x: i32| x + 1;
let arr = [a, b];                  // ❌ a và b khác type!
```

**Fix**: Box<dyn Fn>:
```rust
let arr: [Box<dyn Fn(i32) -> i32>; 2] = [Box::new(a), Box::new(b)];
```

### Error 6: closure borrow whole struct

```rust
struct S { a: String, b: String }
let s = S { a: "hi".into(), b: "bye".into() };

let cl = || println!("{}", s.a);
println!("{}", s.b);              // Rust 2018: ❌, Rust 2021: ✓
```

Rust 2021 (disjoint capture) sửa.

---

# KẾT LUẬN

## Bản đồ tư duy

```
                           CLOSURE
                              │
       ┌──────────────────────┼──────────────────────┐
       ▼                      ▼                      ▼
   SYNTAX                 CAPTURE              TRAITS
                                                    
   |args| body            3 cách:               Fn ⊆ FnMut ⊆ FnOnce
   move keyword           - by &                                
   inference              - by &mut             ┌────────┐
                          - by move             │ &self  │ Fn
                                                │ &mut s │ FnMut
                          compiler chọn          │ self   │ FnOnce
                          tự động                └────────┘
   
   ┌────────────────────────────────────────────────────┐
   │              CLOSURE = STRUCT ẨN                   │
   │                                                    │
   │   struct __Closure {                               │
   │       captured_var1, var2, ...                     │
   │   }                                                │
   │   impl Fn/FnMut/FnOnce for __Closure { ... }       │
   │                                                    │
   │   Mỗi closure literal = UNIQUE type                │
   │   Size = total size captured vars                  │
   └────────────────────────────────────────────────────┘
   
   ┌────────────────────────────────────────────────────┐
   │              PASS / RETURN                         │
   │                                                    │
   │   PASS:    fn f<F: Fn>(f: F)     ← generic         │
   │            fn f(f: &dyn Fn)       ← dyn            │
   │                                                    │
   │   RETURN:  impl Fn                ← static         │
   │            Box<dyn Fn>            ← dynamic        │
   └────────────────────────────────────────────────────┘
   
   ┌────────────────────────────────────────────────────┐
   │              PATTERNS                              │
   │                                                    │
   │   Iterator combinators                             │
   │   Callbacks                                        │
   │   Higher-order functions                           │
   │   Lazy evaluation                                  │
   │   Decorator / middleware                           │
   │   Stateful generators                              │
   └────────────────────────────────────────────────────┘
```

## 10 nguyên lý

```
1. Closure = function + captured environment
2. Capture mode: by &, by &mut, by move
   → Compiler tự chọn "yếu nhất đủ dùng"
3. move keyword: ép capture by value
4. 3 trait hierarchy:
   Fn   (&self)     → đọc, gọi nhiều lần
   FnMut(&mut self) → sửa, gọi nhiều lần
   FnOnce(self)     → consume, gọi 1 lần
5. Fn ⊆ FnMut ⊆ FnOnce (subtype chain)
6. Closure = struct ẩn + impl trait
7. Mỗi closure có UNIQUE type
8. Pass: <F: Fn> (gen) hoặc &dyn Fn (dyn)
9. Return: impl Fn (1 type) hoặc Box<dyn Fn> (đa hình)
10. Send/Sync tự suy từ captured vars
```

## Liên hệ với 3 khái niệm khác

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  Closure & OWNERSHIP                                     │
│  ─────────                                               │
│  - Capture by & → borrow                                 │
│  - Capture by &mut → exclusive borrow                    │
│  - Capture by move → take ownership                      │
│  → Borrow checker check capture                          │
│                                                          │
│  Closure & TRAIT                                         │
│  ──────────                                              │
│  - Fn/FnMut/FnOnce là trait                              │
│  - Closure auto-impl 1 trong 3                           │
│  - Pass closure = trait bound                            │
│                                                          │
│  Closure & GENERIC                                       │
│  ──────────                                              │
│  - fn f<F: Fn>() — generic over closure type             │
│  - impl Fn — opaque return type                          │
│  - Mỗi closure literal monomorphize riêng                │
│                                                          │
│  Closure & MEMORY                                        │
│  ──────────                                              │
│  - Closure size = sum of captured (+ padding)            │
│  - Captured ref → closure có lifetime                    │
│  - Box<dyn Fn> → heap alloc + vtable                     │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## Lộ trình tiếp theo

Sau Trait + Generic + Closure → sẵn sàng cho:
- **Async/Await** — Future là trait, async fn → impl Future, closure async
- **Error handling** — `?` operator, From trait
- **Iterator advanced** — combinators sâu hơn
- **Smart pointers** — Cow, ManuallyDrop, MaybeUninit

Đọc song song `closure-visual.md`.

> Closure là cách Rust hợp nhất function + state + capability. Hiểu closure = hiểu cách Rust nhìn nhận "code as data".
