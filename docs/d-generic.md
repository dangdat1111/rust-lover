# Generic — Từ Bản Chất Đến Nâng Cao

> Generic là cách Rust "viết code 1 lần, dùng cho mọi type". Hiểu generic = hiểu Vec, Option, Result, HashMap... và cách Rust giữ được zero-cost abstraction. File này đi từ vấn đề căn bản đến những góc tinh tế nhất (variance, GAT, const generics).

---

## Mục lục

**Tầng 1 — Vì sao Generic tồn tại?**
1. [Bài toán: code lặp cho từng type](#1-bài-toán)
2. [Các cách giải quyết qua lịch sử](#2-các-cách-giải-quyết)
3. [Generic là gì — định nghĩa cốt lõi](#3-generic-là-gì)

**Tầng 2 — Generic cơ bản**
4. [Generic function](#4-generic-function)
5. [Generic struct](#5-generic-struct)
6. [Generic enum](#6-generic-enum)
7. [Generic method và impl block](#7-generic-method)
8. [Multiple type parameters](#8-multiple-params)

**Tầng 3 — Trait Bounds**
9. [Trait bound — yêu cầu khả năng](#9-trait-bound)
10. [Multiple bounds với +](#10-multiple-bounds)
11. [where clauses sâu](#11-where-clauses)
12. [Conditional implementation](#12-conditional-impl)

**Tầng 4 — Monomorphization & Memory**
13. [Monomorphization — kỹ thuật bên trong](#13-monomorphization)
14. [Code bloat trade-off](#14-code-bloat)
15. [Generic struct trong memory](#15-generic-memory)
16. [Khi monomorph thất bại](#16-monomorph-failure)

**Tầng 5 — Lifetime trong Generic**
17. [Lifetime parameter là generic parameter](#17-lifetime-as-generic)
18. [Bound lifetime với type](#18-lifetime-bound)
19. [HRTB nâng cao](#19-hrtb-advanced)

**Tầng 6 — PhantomData**
20. [Vấn đề: type parameter không dùng](#20-vấn-đề-phantom)
21. [PhantomData là gì](#21-phantomdata-là-gì)
22. [Use cases của PhantomData](#22-phantom-usecases)

**Tầng 7 — Const Generics**
23. [Bài toán: array kích thước biết lúc compile](#23-const-bài-toán)
24. [Const generics cơ bản](#24-const-generics-basic)
25. [Const generics nâng cao](#25-const-generics-advanced)

**Tầng 8 — Variance**
26. [Variance là gì](#26-variance-là-gì)
27. [Covariance, Contravariance, Invariance](#27-3-loại-variance)
28. [Variance trong Rust thực tế](#28-variance-rust)

**Tầng 9 — Patterns nâng cao**
29. [Turbofish ::<>](#29-turbofish)
30. [Default type parameters](#30-default-type-params)
31. [Generic specialization (nightly)](#31-specialization)
32. [Type-state với generic](#32-type-state-generic)
33. [Newtype + generic](#33-newtype-generic)
34. [Builder pattern](#34-builder)

**Tầng 10 — Bẫy thường gặp**
35. [Lỗi generic phổ biến](#35-common-errors)

---

# TẦNG 1 — VÌ SAO GENERIC TỒN TẠI?

## 1. Bài toán

Trước khi có generic, viết container cho mỗi type là **địa ngục**:

```c
// C: 1 container per type
struct IntVec { int* data; int len; };
struct FloatVec { float* data; int len; };
struct StringVec { char** data; int len; };

void int_vec_push(IntVec* v, int x) { ... }
void float_vec_push(FloatVec* v, float x) { ... }
void string_vec_push(StringVec* v, char* x) { ... }
// ... lặp đến khi chết
```

→ Code lặp, dễ bug, không scale.

### Trong toán học

Bạn không viết:
```
- "Hàm cộng 2 số nguyên"
- "Hàm cộng 2 số thực"
- "Hàm cộng 2 số phức"
```

Bạn viết:
```
- "Hàm cộng 2 số" (với "số" hiểu trừu tượng)
```

→ Generic = áp dụng tư duy này vào programming.

---

## 2. Các cách giải quyết qua lịch sử

### Cách 1: Macro (C, preprocessor)

```c
#define DEFINE_VEC(T) \
    struct Vec_##T { T* data; int len; }; \
    void push_##T(struct Vec_##T* v, T x) { ... }

DEFINE_VEC(int)
DEFINE_VEC(float)
```

```
ƯU:  Sinh code cho mỗi type
NHƯỢC:
  - Không type-check trong macro
  - Lỗi compile khủng khiếp
  - Không IDE-friendly
  - Không thể compose
```

### Cách 2: Void pointer (C)

```c
struct Vec { void** data; int len; };
void vec_push(struct Vec* v, void* x) { ... }
```

```
ƯU:  1 implementation
NHƯỢC:
  - Không type-safe (push int → pop string? compiler không biết)
  - Mỗi phần tử là pointer → cache-unfriendly
  - Không in-place
```

### Cách 3: Template (C++)

```cpp
template<typename T>
class Vector { T* data; size_t len; ... };
```

```
ƯU:  Type-safe, type-checked
NHƯỢC:
  - Lỗi compile DEEP (template instantiation từng tầng)
  - "Duck typing": chỉ biết T sai khi thử compile
  - SFINAE phức tạp
```

### Cách 4: Generic + Type Erasure (Java, C#)

```java
class List<T> {
    Object[] data;     // BÊN TRONG là Object[]
    void add(T x) { ... }
}
```

```
ƯU:  1 binary
NHƯỢC:
  - Type Erasure: lúc runtime, List<Integer> và List<String> y hệt
  - Phải box primitive (int → Integer) → tốn RAM
  - List<int> không có (chỉ List<Integer>)
  - Mất performance vì boxing
```

### Cách 5: Rust generic = Template C++ + Trait bound

```rust
struct Vec<T> { ... }

impl<T> Vec<T> {
    fn push(&mut self, x: T) { ... }
}

fn sum<T: Add<Output = T>>(v: Vec<T>) -> T { ... }
```

```
ƯU:
  - Type-safe (như C++ template)
  - Trait bound rõ ràng (NOT duck typing như C++)
  - Lỗi compile dễ đọc
  - Monomorphize → tốc độ C
  - Const generics (Rust 1.51+)
NHƯỢC:
  - Học khó hơn Java generic
  - Code bloat (vì monomorph)
```

---

## 3. Generic là gì

### Định nghĩa chính xác

> **Generic** là khả năng viết code với **parameter là type** (hoặc lifetime, hoặc const value). Compiler sẽ "fill in" parameter cụ thể khi gọi → sinh ra phiên bản tối ưu cho từng case.

### 3 loại parameter

```
   Rust có 3 loại parameter trong generic:
   
   1. TYPE parameter           : <T>
   2. LIFETIME parameter       : <'a>
   3. CONST parameter          : <const N: usize>
```

### Ví dụ kết hợp cả 3

```rust
struct Buffer<'a, T, const N: usize> {
    slice: &'a mut [T; N],
}
```

Đọc:
- `'a`: lifetime của reference
- `T`: kiểu phần tử
- `N`: số phần tử (compile-time constant)

---

# TẦNG 2 — GENERIC CƠ BẢN

## 4. Generic function

```rust
fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(5);          // T = i32
    let b = identity("hi");       // T = &str
    let c = identity(vec![1, 2]); // T = Vec<i32>
}
```

### Đọc syntax

```
   fn identity<T>(x: T) -> T
              ───  ───    ─
               │    │     └─ return type là T
               │    └─ argument type là T
               └─ khai báo type parameter T
```

### Compiler suy luận T

Đa số case bạn không cần chỉ định T:

```rust
let a = identity(5);
//      ──────── ─
//              └─ compiler thấy 5: i32 → T = i32
```

### Chỉ định T thủ công (turbofish)

```rust
let a = identity::<i32>(5);
//              ───────
//              "turbofish"
```

Khi nào cần? Khi compiler không suy ra được:

```rust
let s = "42".parse::<i32>().unwrap();
//              ───────
//              Nếu không có, compiler không biết parse thành type gì
```

---

## 5. Generic struct

```rust
struct Point<T> {
    x: T,
    y: T,
}

let p1 = Point { x: 5, y: 10 };          // Point<i32>
let p2 = Point { x: 1.0, y: 2.0 };       // Point<f64>
```

### Multiple type parameters

```rust
struct Pair<T, U> {
    first: T,
    second: U,
}

let p = Pair { first: 1, second: "hi" };   // Pair<i32, &str>
```

### Bản chất

```
   struct Point<T> {
       x: T,
       y: T,
   }
   
   sau monomorphize với T = i32:
   
   struct Point__i32 {
       x: i32,        ← inline thẳng vào struct
       y: i32,
   }
   
   Size = 8 byte (2 × i32), không có overhead
```

→ **NOT** như Java (List<Integer> chứa Object[] với boxing).

---

## 6. Generic enum

```rust
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

→ **2 enum được dùng nhiều nhất** trong Rust đều là generic.

### Option<T> memory

```rust
let x: Option<i32> = Some(5);
```

```
   sau monomorphize:
   
   enum Option__i32 {
       Some(i32),
       None,
   }
   
   Layout (với niche optimization có thể là khác):
   ┌─────┬──────────┐
   │ tag │ i32 data │
   │ 4 B │   4 B    │
   └─────┴──────────┘
   = 8 byte
   
   tag = 0: Some, data = giá trị
   tag = 1: None, data = không dùng
```

### Niche optimization với Option<&T>

```rust
let x: Option<&i32> = Some(&5);
```

```
   &i32 KHÔNG bao giờ null (0x0).
   → Rust dùng 0x0 để biểu diễn None.
   → Option<&i32> = 8 byte (không cần tag!)
```

(Đã giải thích trong `memory-model.md`.)

---

## 7. Generic method và impl block

```rust
struct Point<T> { x: T, y: T }

impl<T> Point<T> {
    fn new(x: T, y: T) -> Self {
        Point { x, y }
    }
    
    fn x(&self) -> &T {
        &self.x
    }
}
```

### Đọc syntax `impl<T> Point<T>`

```
   impl<T> Point<T>
        ─       ─
        │       │
        │       └─ "Point<T>" — type ta đang impl cho
        │
        └─ "Khai báo: tôi sẽ dùng type parameter T"
   
   → Bắt buộc phải có <T> sau impl
```

### Impl cho 1 type cụ thể

```rust
impl Point<f64> {                 // ← KHÔNG có <T>!
    fn distance_from_origin(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

let p = Point::<f64>::new(3.0, 4.0);
println!("{}", p.distance_from_origin());   // ✓
                                            
let p2 = Point::<i32>::new(3, 4);
// p2.distance_from_origin();  ❌ method only on Point<f64>
```

→ Cực kỳ hữu ích: thêm method **chỉ cho 1 instantiation**.

---

## 8. Multiple type parameters

```rust
fn longest<T: Ord, U: ToString>(v: &[T], prefix: U) -> String {
    let max = v.iter().max().unwrap();
    format!("{}: {:?}", prefix.to_string(), max)
}

let v = vec![3, 1, 4, 1, 5];
let s = longest(&v, "max");
```

### Quy ước đặt tên

```
   T, U, V       — type parameter chung
   K, V          — key, value
   E             — error
   F             — closure/function
   I             — iterator
   'a, 'b        — lifetime
   N, M          — const generic
   Item          — associated type (snake_case_camelcase)
```

→ Như toán học, viết ngắn cho dễ đọc.

---

# TẦNG 3 — TRAIT BOUNDS

## 9. Trait bound

`<T>` không có ràng buộc = "T có thể là **bất cứ gì**".

```rust
fn largest<T>(v: &[T]) -> T {
    let mut largest = v[0];
    for &item in &v[1..] {
        if item > largest {       // ❌ T có operator > không?
            largest = item;
        }
    }
    largest
}
```

```
   error: binary operation `>` cannot be applied to type `T`
   help: consider restricting type parameter `T`
```

→ Phải thêm **trait bound**:

```rust
fn largest<T: PartialOrd>(v: &[T]) -> T { ... }
```

Đọc: "T phải implement PartialOrd."

### Lý do trait bound an toàn hơn C++ template

```
   C++ template (duck typing):
   ────────────
   template<typename T>
   T largest(vector<T>& v) {
       T largest = v[0];
       for (auto& item : v) {
           if (item > largest) ...
       }
   }
   
   T phải có operator> — nhưng KHÔNG ĐƯỢC khai báo!
   → Lỗi compile khi instantiate, ở DEEP CALL STACK
   → Khó debug
   
   Rust generic (explicit):
   ─────────
   fn largest<T: PartialOrd>(v: &[T]) -> T { ... }
   
   T phải có PartialOrd — KHAI BÁO RÕ RÀNG
   → Lỗi compile ngay tại call site nếu T sai
   → Dễ debug
```

---

## 10. Multiple bounds với +

```rust
fn process<T: Clone + Debug + PartialEq>(x: T) {
    let copy = x.clone();
    println!("{:?}", copy);
    if x == copy { ... }
}
```

Đọc: "T phải implement Clone VÀ Debug VÀ PartialEq."

### Cú pháp khác: `+` cho lifetime

```rust
fn foo<'a, T: Display + 'a>(x: &'a T) { ... }
//                       ───
//                       T phải sống ít nhất 'a
```

---

## 11. where clauses sâu

Khi bound nhiều → tách ra `where`:

```rust
// Khó đọc:
fn process<T: Clone + Debug + PartialEq, U: Iterator<Item = T> + ExactSizeIterator>(x: U) { ... }

// Dễ đọc:
fn process<T, U>(x: U)
where
    T: Clone + Debug + PartialEq,
    U: Iterator<Item = T> + ExactSizeIterator,
{ ... }
```

### where cho phép bound mà inline không làm được

```rust
// Inline KHÔNG được:
fn foo<T>(x: T) where Vec<T>: Clone { ... }
//                    ─────────────
//                    Bound TRÊN type không phải parameter của fn

// Cần where!
```

```rust
// Generic bound trên associated type
fn foo<I>(i: I)
where
    I: Iterator,
    I::Item: Display,         // ← bound trên associated type
{
    for item in i {
        println!("{}", item);
    }
}
```

---

## 12. Conditional implementation

Thêm method **chỉ khi** T thoả 1 trait nào đó:

```rust
struct Wrapper<T> { value: T }

impl<T> Wrapper<T> {
    fn new(v: T) -> Self { Wrapper { value: v } }
}

impl<T: Display> Wrapper<T> {        // ← chỉ khi T: Display
    fn print(&self) {
        println!("{}", self.value);
    }
}

let w1 = Wrapper::new(5);
w1.print();                          // ✓ i32 có Display

struct NoDisplay;
let w2 = Wrapper::new(NoDisplay);
// w2.print();                       // ❌ NoDisplay không có Display
```

### Blanket impl + conditional

```rust
// Đây là trong std:
impl<T: Display + ?Sized> ToString for T {
    fn to_string(&self) -> String { ... }
}
```

→ MỌI type có `Display` tự động có `to_string()`.

---

# TẦNG 4 — MONOMORPHIZATION & MEMORY

## 13. Monomorphization — kỹ thuật bên trong

```rust
fn double<T: std::ops::Add<Output = T> + Copy>(x: T) -> T {
    x + x
}

fn main() {
    let a = double(5);
    let b = double(3.14);
}
```

### Compiler làm gì?

```
   Bước 1: Type Inference
   ──────────────────────
   double(5)    → T = i32
   double(3.14) → T = f64
   
   Bước 2: Monomorphize — clone function
   ──────────────────────────────────
   
   fn double__i32(x: i32) -> i32 {
       x + x
   }
   
   fn double__f64(x: f64) -> f64 {
       x + x
   }
   
   Bước 3: Replace call site
   ────────────────────────
   double(5)    → double__i32(5)
   double(3.14) → double__f64(3.14)
   
   Bước 4: Optimize từng version
   ─────────────────────────────
   - double__i32: dùng instruction ADD
   - double__f64: dùng instruction FADD (floating-point add)
   - Cả 2 đều INLINE nếu nhỏ
```

### Sau optimize: gần như "biến mất"

```
   Code Rust:                     Assembly thực tế:
   ──────────                     ────────────────
   
   fn main() {                    main:
       let a = double(5);             mov eax, 10        ; constant 5+5
       let b = double(3.14);          movsd xmm0, 6.28   ; constant 3.14+3.14
   }                                  ret
                                  
                                  ← double() đã BIẾN MẤT!
                                    (vì constant folding)
```

→ "Zero-cost abstraction" có thật.

---

## 14. Code bloat trade-off

```
   Trade-off:
   ──────────
   
   ⚡ Speed:        +++ (mỗi version optimize cho 1 type)
   📦 Binary size: +++ (mỗi version = 1 bản code)
   🧠 Compile:     +++ (compile lâu hơn)
```

### Khi nào code bloat thành vấn đề?

```
   Vec<T> trong std:
   ─────
   - 30+ methods (push, pop, insert, ...)
   - User code: dùng Vec<i32>, Vec<String>, Vec<Custom1>, ...
   
   → 30 × 10 = 300 method instantiations trong binary
   → Có thể tăng binary 1-10 MB
```

### Khi nào CẦN giảm bloat?

```
   - Embedded (binary size critical)
   - WASM (binary nhỏ → load nhanh)
   - Large libraries (vd: serde sinh ra MB code)
```

### Giải pháp 1: Inline outline pattern

```rust
// Generic outer (instantiate cho mỗi T):
fn print<T: Display>(x: T) {
    print_impl(x.to_string())
}

// Non-generic inner (1 lần trong binary):
fn print_impl(s: String) {
    println!("{}", s);
}
```

→ Outer chỉ là wrapper nhỏ → bloat thấp.

### Giải pháp 2: dyn Trait thay generic

```rust
// Generic — bloat
fn print<T: Display>(x: &T) { ... }

// Dyn — 1 bản, ~2ns extra per call
fn print(x: &dyn Display) { ... }
```

→ Xem chi tiết trong `trait.md`.

---

## 15. Generic struct trong memory

```rust
struct Pair<T, U> {
    first: T,
    second: U,
}
```

### Layout sau monomorph

```rust
let p1 = Pair { first: 1u8, second: 2u64 };
```

```
   Pair<u8, u64>:
   
   ┌──────┬─────────────┬──────────────┐
   │  u8  │  padding    │     u64      │
   │ 1 B  │    7 B      │     8 B      │
   └──────┴─────────────┴──────────────┘
   = 16 byte (8-aligned)
   
   ⚠ padding! Field order matters.
```

### Reorder để tiết kiệm

```rust
struct PairGood<T, U> {
    second: U,                  // 8 byte
    first: T,                   // 1 byte
}
// = 8 + 1 + padding = 16 byte (vẫn vậy do alignment)
```

Rust **tự reorder fields** mặc định (trừ khi có `#[repr(C)]`) để tối ưu size. Nhưng generic struct không tối ưu được mọi case.

### Generic struct lớn

```rust
struct LargePair<T> {
    a: T,
    b: T,
    c: T,
    d: T,
}
```

```
   LargePair<i32>:    16 byte
   LargePair<String>: 96 byte (24 × 4)
   LargePair<u8>:      4 byte
   
   Mỗi instantiation = struct layout khác.
```

---

## 16. Khi monomorph thất bại

Không phải lúc nào compiler cũng monomorph được.

### Case: dyn Trait không monomorph

```rust
fn process<T: Trait>(x: &T) { ... }     // monomorph
fn process(x: &dyn Trait) { ... }       // KHÔNG monomorph (1 bản code)
```

### Case: infinite type

```rust
fn recursive<T>(x: T) {
    if x.is_done() { return; }
    recursive(box_it(x));               // ❌ vô hạn instantiation
}
```

Mỗi call tạo type mới → vô hạn → compile fail (hoặc stack overflow ở compile).

### Case: hidden recursive

```rust
struct Node<T> {
    value: T,
    children: Vec<Node<T>>,
}
```

Đây OK vì Vec lưu pointer, không recursive size.

```rust
struct BadNode<T> {
    value: T,
    children: [Node<T>; 10],            // ❌ inline 10 Node<T>
}
```

Đây cũng OK vì compile-time size có thể tính.

```rust
struct VeryBad<T> {
    next: VeryBad<T>,                   // ❌ recursive infinite
}
```

→ ERROR vì size = size + something.

---

# TẦNG 5 — LIFETIME TRONG GENERIC

## 17. Lifetime parameter là generic parameter

Lifetime cũng là 1 loại generic parameter:

```rust
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str { ... }
//          ──
//          Lifetime parameter (giống type parameter)
```

### Kết hợp lifetime + type

```rust
fn process<'a, T: Display>(x: &'a T) -> String {
    format!("{}", x)
}
```

Đọc:
- `'a`: lifetime parameter
- `T`: type parameter, có bound Display
- `&'a T`: reference với lifetime 'a tới T

### Thứ tự khai báo

```
   Quy ước: 
   ─────────
   <'a, 'b, T, U, V, const N: usize>
    ────  ──────  ─────────────────
    lifetime  type      const
   
   Đặt theo thứ tự: lifetime trước, type sau, const cuối.
```

---

## 18. Bound lifetime với type

```rust
struct Wrapper<'a, T: 'a> {
    inner: &'a T,
}
```

Đọc `T: 'a`: "T sống ít nhất `'a`".

### Vì sao cần?

```rust
struct Bad<'a, T> {
    inner: &'a T,         // ← &'a T yêu cầu T sống ít nhất 'a
}
```

```
   warning: T phải sống ít nhất 'a (vì &'a T)
   help: thêm bound T: 'a
```

### `T: 'static` — thường gặp

```rust
fn spawn<F: FnOnce() + Send + 'static>(f: F) { ... }
//                              ─────
//                              F không được vay gì có lifetime ngắn
```

Đọc `'static`: "không vay gì sống ngắn" = "owned" hoặc "vay `'static`".

---

## 19. HRTB nâng cao

Higher-Ranked Trait Bound = "for all lifetimes":

```rust
fn apply<F>(f: F)
where
    F: for<'a> Fn(&'a str) -> &'a str,    // ← HRTB
{ ... }
```

### Vì sao cần?

```rust
fn apply<F>(f: F) where F: Fn(&str) -> &str { ... }
```

Compiler ngầm thêm HRTB nếu không có lifetime cụ thể. Nhưng đôi khi explicit cần thiết:

```rust
struct Processor<F> 
where 
    F: for<'a> Fn(&'a [i32]) -> &'a i32 
{
    func: F,
}
```

Đọc: "F là closure work với **mọi** lifetime `'a`".

### Khi nào KHÔNG dùng HRTB?

```rust
// Closure chỉ work với 1 lifetime cụ thể:
fn make<'a>(s: &'a str) -> impl Fn(&'a str) -> &'a str + 'a {
    move |x| s
}
```

→ Đây là Fn với 1 lifetime, không HRTB.

---

# TẦNG 6 — PHANTOMDATA

## 20. Vấn đề: type parameter không dùng

```rust
struct MyTag<T> {
    value: i32,
    // T không được dùng!
}
```

```
   error[E0392]: parameter `T` is never used
   help: consider removing `T`, or using PhantomData
```

### Vì sao Rust cấm?

Vì T ảnh hưởng đến **variance** và **drop semantics**. Nếu T không xuất hiện trong field, compiler không biết:
- T có ảnh hưởng đến lifetime không?
- T có cần drop không?
- Variance như thế nào?

→ Phải dùng `PhantomData<T>` để "giả vờ" chứa T.

---

## 21. PhantomData là gì

```rust
use std::marker::PhantomData;

struct PhantomData<T>;
```

```
   Đặc tính:
   - SIZE = 0 byte (zero-sized type)
   - Không có data
   - Chỉ để báo compiler: "tôi liên quan đến T"
```

### Cú pháp

```rust
struct MyTag<T> {
    value: i32,
    _marker: PhantomData<T>,
}

impl<T> MyTag<T> {
    fn new() -> Self {
        MyTag { value: 0, _marker: PhantomData }
    }
}
```

### Layout

```
   struct MyTag<T> {
       value: i32,           ← 4 byte
       _marker: PhantomData<T>,  ← 0 byte
   }
   
   Size = 4 byte (không thay đổi dù T là gì)
```

---

## 22. Use cases của PhantomData

### Use case 1: Type-state pattern

```rust
struct Connection<State> {
    socket: TcpStream,
    _state: PhantomData<State>,
}

struct Disconnected;
struct Connected;
struct Authenticated;

impl Connection<Disconnected> {
    fn connect(self) -> Connection<Connected> { ... }
}

impl Connection<Connected> {
    fn login(self, ...) -> Connection<Authenticated> { ... }
}

impl Connection<Authenticated> {
    fn query(&self, sql: &str) -> Rows { ... }
}
```

→ Compile-time state machine. Không thể gọi `.query()` khi chưa `.login()`.

### Use case 2: Lifetime marker cho FFI

```rust
struct CWrapper<'a, T> {
    ptr: *const T,                       // raw pointer, không có lifetime
    _marker: PhantomData<&'a T>,         // giả vờ chứa &'a T
}
```

→ Compiler check lifetime đúng như có `&'a T` thật.

### Use case 3: Variance control

```rust
// Covariant in T:
struct Cov<T>(*const T);
// nhưng *const T không cho compiler biết
struct CovExplicit<T> {
    _marker: PhantomData<T>,
}

// Contravariant in T:
struct Contra<T> {
    _marker: PhantomData<fn(T)>,
}

// Invariant in T:
struct Invar<T> {
    _marker: PhantomData<fn(T) -> T>,
}
```

(Sẽ giải thích variance trong Tầng 8.)

### Use case 4: Send/Sync control

```rust
struct ThreadLocal<T> {
    value: T,
    _not_send: PhantomData<*const ()>,   // *const () NOT Send/Sync
}
// → ThreadLocal<T> không Send dù T Send
```

---

# TẦNG 7 — CONST GENERICS

## 23. Bài toán: array kích thước biết lúc compile

Trước Rust 1.51 (2021), không thể viết generic theo size:

```rust
// Trước 1.51 — KHÔNG được:
fn sum<T, const N: usize>(arr: [T; N]) -> T { ... }
```

→ Phải dùng `&[T]` (slice) → mất type-info về size.

### Vấn đề thực tế

```rust
fn multiply_matrix_3x3(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] { ... }
fn multiply_matrix_4x4(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] { ... }
// ...
```

→ Lặp code. Cần generic theo N.

---

## 24. Const generics cơ bản

```rust
fn first<T, const N: usize>(arr: [T; N]) -> T 
where T: Copy 
{
    arr[0]
}

let a = first([1, 2, 3]);          // N = 3
let b = first([1.0, 2.0]);          // N = 2
```

### Generic struct với const

```rust
struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn new() -> Self {
        Matrix { data: [[0.0; C]; R] }
    }
}

let m: Matrix<3, 4> = Matrix::new();
```

### Size compile-time

```rust
fn array_size<T, const N: usize>(_: &[T; N]) -> usize { N }

let a = [1, 2, 3];
let s = array_size(&a);    // s = 3 (compile-time constant)
```

---

## 25. Const generics nâng cao

### Generic Default

```rust
struct Buffer<const SIZE: usize = 256> {
    data: [u8; SIZE],
}

let b1: Buffer = Buffer { data: [0; 256] };       // default SIZE = 256
let b2: Buffer<1024> = Buffer { data: [0; 1024] };
```

### Const evaluation trong bound (nightly)

```rust
fn split<const N: usize>(arr: [i32; N]) -> ([i32; N/2], [i32; N/2])
where 
    [(); N/2]:                  // ← bound that N/2 is valid
{
    // ...
}
```

→ Một số tính toán trên const generic vẫn cần feature flag.

### Const generic bounds (limitations)

```rust
// Tự nhiên dùng được:
fn foo<const N: usize>(x: [i32; N]) { ... }

// Bound dùng được:
fn bar<const N: usize>(x: [i32; N]) where [i32; N+1]: { ... }
//                                        ────────────
//                                        nightly only

// Khá hạn chế hiện tại — Rust đang mở rộng dần.
```

---

# TẦNG 8 — VARIANCE

## 24. Variance là gì

Đây là chủ đề **khó nhất** của generic. Hiểu được = bạn ở top 5% Rust dev.

### Bài toán

Nếu `Dog` là `Animal` (qua trait), thì:
- `Vec<Dog>` có là `Vec<Animal>` không?
- `&Dog` có là `&Animal` không?
- `fn(Dog)` có là `fn(Animal)` không?

→ Tùy! Phụ thuộc vào **variance**.

### Lifetime variance

Câu hỏi dễ hiểu hơn:
- `&'static T` (sống mãi) có thể dùng nơi cần `&'a T` (sống `'a`) không?
- → CÓ. Vì `'static ⊇ 'a` (sống lâu hơn → có thể dùng cho yêu cầu ngắn).

```
   'static  ⊇  'a   (subtyping: 'static "là" subtype của 'a)
   
   Nếu F<X>:
   - covariant in X    : 'static ⊆ 'a → F<&'static T> ⊆ F<&'a T>
   - contravariant in X: ngược lại
   - invariant in X    : không quan hệ
```

---

## 27. 3 loại variance

### Covariant — "thuận chiều"

```rust
struct Owner<T>(T);
```

`Owner<T>` covariant in T → "T mạnh hơn → Owner<T> mạnh hơn".

```
   &'static T  ⊆  &'a T  (subtyping)
              ⇓
   Owner<&'static T>  ⊆  Owner<&'a T>
```

→ Có thể pass `Owner<&'static i32>` ở chỗ cần `Owner<&'a i32>`.

### Contravariant — "ngược chiều"

```rust
struct Callback<T>(fn(T));
```

`Callback<T>` contravariant in T → "T mạnh hơn → Callback<T> YẾU hơn".

```
   fn(&'a T)  ⊆  fn(&'static T)    (function args đảo ngược)
              ⇓
   Callback<&'a T>  ⊆  Callback<&'static T>
```

Tại sao? Vì:
```
   Hàm nhận &'a T   = "tôi xử lý mọi reference, kể cả ngắn"
   Hàm nhận &'static = "tôi chỉ xử lý reference vĩnh viễn"
   
   → Hàm nhận &'a "mạnh hơn" (xử lý nhiều hơn)
   → Có thể dùng thay cho hàm nhận &'static
```

### Invariant — "không liên quan"

```rust
struct Cell<T>(UnsafeCell<T>);
```

`Cell<T>` invariant in T → KHÔNG có quan hệ.

```
   Vì sao? Cell cho phép cả READ và WRITE.
   
   Nếu Cell<&'static T> ⊆ Cell<&'a T>:
   - Tôi có Cell<&'a> (ngắn).
   - Lấy ra &'a (ok).
   - Nhưng cũng có thể GHI vào (qua &mut Cell):
     ghi &'a (giá trị ngắn) vào nơi đáng lẽ chứa &'static
     → ngắn ghi vào dài → UNSAFE!
   
   → Cell phải invariant để chặn.
```

### Quy tắc

```
   READ ONLY (immutable):                  COVARIANT
     &T, &'a T, Box<T>, Rc<T>, Vec<T>
   
   WRITE / READ-WRITE:                     INVARIANT
     &mut T, Cell<T>, RefCell<T>,
     Mutex<T>, *mut T
   
   FUNCTION ARGUMENT POSITION:             CONTRAVARIANT
     fn(T)
   
   FUNCTION RETURN POSITION:               COVARIANT
     fn() -> T
```

---

## 28. Variance trong Rust thực tế

### Ví dụ ảnh hưởng thực

```rust
fn last(v: &Vec<&str>) -> &str { ... }

let owned = String::from("hi");
let v: Vec<&str> = vec![&owned];
let r: &str = last(&v);
```

OK vì `&Vec` covariant → có thể pass `Vec<&'a str>` ở chỗ cần `Vec<&'b str>` với `'a ⊆ 'b`.

### Ví dụ ngược: `&mut Vec` invariant

```rust
fn replace(v: &mut Vec<&'static str>, s: &'static str) {
    v[0] = s;
}

let mut v: Vec<&str> = vec!["hello"];
// replace(&mut v, ...);              ❌ Vec<&'a str> ≠ Vec<&'static str>
```

→ Vec qua &mut là invariant → không thay đổi lifetime.

### PhantomData để control variance

```rust
struct CovInT<T> {
    _t: PhantomData<T>,         // covariant
}

struct ContraInT<T> {
    _t: PhantomData<fn(T)>,     // contravariant (function arg)
}

struct InvarInT<T> {
    _t: PhantomData<*mut T>,    // invariant (raw mut pointer)
}
```

---

# TẦNG 9 — PATTERNS NÂNG CAO

## 29. Turbofish `::<>`

Khi compiler không suy ra được type:

```rust
let v = vec![1, 2, 3];
let s: i32 = v.iter().sum::<i32>();
//                       ────────
//                       turbofish: chỉ định T = i32 cho sum<T>
```

### Vì sao tên "turbofish"?

```
   ::<>     ← trông giống con cá
   ::<i32>
```

### Khi nào cần?

```rust
// Cần khi return type ambiguous:
let n: i32 = "42".parse().unwrap();             // ✓ context có type
let n = "42".parse::<i32>().unwrap();            // ✓ turbofish

// Khi collect ambiguous:
let v: Vec<i32> = (0..10).collect();             // ✓ context
let v = (0..10).collect::<Vec<i32>>();           // ✓ turbofish
```

---

## 30. Default type parameters

```rust
trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
```

`Rhs = Self` = default. Khi impl không chỉ định:

```rust
impl Add for i32 {                  // ← Rhs = Self = i32 (default)
    type Output = i32;
    fn add(self, rhs: i32) -> i32 { self + rhs }
}

// Hoặc khác type:
impl Add<f64> for i32 {              // Rhs = f64
    type Output = f64;
    fn add(self, rhs: f64) -> f64 { self as f64 + rhs }
}
```

### Default cho struct

```rust
struct Wrapper<T = String> {
    value: T,
}

let w1: Wrapper = Wrapper { value: "hi".into() };   // default String
let w2: Wrapper<i32> = Wrapper { value: 42 };
```

---

## 31. Generic specialization (nightly)

Trong nightly, có thể "đặc biệt hoá" impl cho type cụ thể:

```rust
#![feature(min_specialization)]

trait Print {
    fn print(&self);
}

impl<T> Print for T {                  // general
    default fn print(&self) {
        println!("anything");
    }
}

impl Print for i32 {                   // specialized for i32
    fn print(&self) {
        println!("int: {}", self);
    }
}
```

```rust
5i32.print();           // "int: 5"
"hi".print();           // "anything"
```

→ Vẫn chưa stable vì có nhiều case khó với coherence.

---

## 32. Type-state với generic

```rust
struct Builder<State> {
    name: Option<String>,
    age: Option<u32>,
    _state: PhantomData<State>,
}

struct Empty;
struct WithName;
struct WithAll;

impl Builder<Empty> {
    fn new() -> Self {
        Builder { name: None, age: None, _state: PhantomData }
    }
    
    fn name(self, n: String) -> Builder<WithName> {
        Builder { 
            name: Some(n), 
            age: self.age, 
            _state: PhantomData 
        }
    }
}

impl Builder<WithName> {
    fn age(self, a: u32) -> Builder<WithAll> {
        Builder { 
            name: self.name, 
            age: Some(a), 
            _state: PhantomData 
        }
    }
}

impl Builder<WithAll> {
    fn build(self) -> User {
        User { name: self.name.unwrap(), age: self.age.unwrap() }
    }
}
```

### Sử dụng

```rust
let user = Builder::new()
    .name("Alice".into())   // → Builder<WithName>
    .age(30)                 // → Builder<WithAll>
    .build();                 // ✓ chỉ available ở WithAll
```

Compile error nếu bỏ qua bước:

```rust
let user = Builder::new()
    .build();                 // ❌ Empty không có build()
```

---

## 33. Newtype + generic

```rust
struct Meters(f64);
struct Feet(f64);

// Type-safe units
fn distance(m: Meters) -> Meters { ... }

let d = distance(Meters(100.0));        // ✓
// let d = distance(Feet(100.0));       // ❌ type mismatch
```

### Generic newtype

```rust
struct Vec3<T>([T; 3]);

impl<T: Copy + Mul<Output = T> + Add<Output = T>> Vec3<T> {
    fn dot(&self, other: &Vec3<T>) -> T {
        self.0[0] * other.0[0] + self.0[1] * other.0[1] + self.0[2] * other.0[2]
    }
}

let a = Vec3([1.0, 2.0, 3.0]);
let b = Vec3([4.0, 5.0, 6.0]);
println!("{}", a.dot(&b));
```

→ Newtype + generic = type-safe + linh hoạt.

---

## 34. Builder pattern

```rust
struct ServerBuilder {
    host: String,
    port: u16,
    workers: usize,
}

impl ServerBuilder {
    fn new() -> Self {
        ServerBuilder { host: "localhost".into(), port: 8080, workers: 4 }
    }
    
    fn host(mut self, h: impl Into<String>) -> Self {
        self.host = h.into();
        self
    }
    
    fn port(mut self, p: u16) -> Self {
        self.port = p;
        self
    }
    
    fn workers(mut self, w: usize) -> Self {
        self.workers = w;
        self
    }
    
    fn build(self) -> Server {
        Server { host: self.host, port: self.port, workers: self.workers }
    }
}
```

### Sử dụng

```rust
let server = ServerBuilder::new()
    .host("example.com")
    .port(443)
    .workers(8)
    .build();
```

### Generic version (chain-friendly)

`impl Into<String>` cho phép pass `String`, `&str`, `Cow<str>`, ... → flexibility.

---

# TẦNG 10 — BẪY THƯỜNG GẶP

## 35. Common errors

### Error 1: missing bound

```
error[E0277]: the trait bound `T: PartialOrd` is not satisfied
```

**Fix**: thêm bound `<T: PartialOrd>`.

### Error 2: type inference failed

```
error[E0282]: type annotations needed
```

**Fix**: dùng turbofish hoặc explicit type:

```rust
let v: Vec<i32> = ...;
// hoặc
let v = ....collect::<Vec<i32>>();
```

### Error 3: unconstrained type parameter

```rust
fn foo<T>() {}             // ❌ T không xuất hiện trong args/return

error: type parameter `T` is not constrained
```

**Fix**: dùng PhantomData hoặc bỏ T.

### Error 4: conflicting implementations

```rust
impl<T> Print for T { ... }
impl<T: Clone> Print for T { ... }    // ❌ overlap với blanket
```

### Error 5: cannot infer lifetime

```rust
struct Foo<'a, T> {
    data: &'a T,
}

impl<T> Foo<'_, T> {            // ❌ thiếu lifetime
    ...
}
```

**Fix**: `impl<'a, T> Foo<'a, T>`.

### Error 6: lifetime may not live long enough

```rust
fn foo<T>(x: &T) -> &T { x }
//      ──── ──   ──
//      compiler tự lifetime elision:
//      fn foo<'a, T>(x: &'a T) -> &'a T

// OK!
```

Nếu phức tạp → explicit lifetime.

---

# KẾT LUẬN

## Bản đồ tư duy

```
                           GENERIC
                              │
       ┌──────────────────────┼──────────────────────┐
       ▼                      ▼                      ▼
   PARAMETERS              BOUNDS                MEMORY
   (3 loại)                                       
                                                 
   ┌────┼────┐         ┌─────┴─────┐         ┌─────┴─────┐
   ▼    ▼    ▼         ▼           ▼         ▼           ▼
 type lifetime const  trait      where    monomorph   variance
 <T>   <'a> <const>   bound      clause   (compile)   (cov/contra
                                          → multiple   /invar)
                                          binary
                                          copies)
   
   ┌────────────────────────────────────────────────────┐
   │                  PATTERNS                          │
   │   PhantomData      (zero-size marker)              │
   │   Type-state       (compile-time state machine)    │
   │   Newtype          (zero-cost wrapper)             │
   │   Builder          (fluent API)                    │
   │   Turbofish ::<>   (explicit type)                 │
   └────────────────────────────────────────────────────┘
```

## 10 nguyên lý ghi nhớ

```
1. Generic = parametric polymorphism (1 code, N types)
2. Có 3 loại param: type, lifetime, const
3. Trait bound RÕ RÀNG (không như C++ duck typing)
4. Monomorphization = compile-time clone code
   → zero-cost runtime, code bloat compile-time
5. Lifetime cũng là generic parameter (subtyping)
6. PhantomData = zero-size marker cho type T không dùng
7. Const generic = parameter là compile-time value
8. Variance: covariant (Box<T>), contravariant (fn(T)),
   invariant (Cell<T>, &mut T)
9. Turbofish ::<>= explicit type khi compiler không suy ra
10. where clause cho phép bound phức tạp hơn inline
```

## Liên hệ Memory Model & Trait

```
Generic       ←→ Trait bound (constraint capability)
Monomorph     ←→ Multiple instantiation trong TEXT segment
Generic struct ←→ Layout phụ thuộc T (size + padding)
PhantomData    ←→ Zero-size, ảnh hưởng variance + Send/Sync
Const generic  ←→ Array size, type-level math
Variance       ←→ Lifetime subtyping → ai có thể "thay thế" ai
```

---

## Lộ trình tiếp theo

Hiểu sâu Generic + Trait → bạn đã có nền tảng cho:
- **Closure** (Fn/FnMut/FnOnce là trait, closure type là generic)
- **Async** (Future là trait, async fn return impl Future)
- **Error handling** (Result<T, E> là generic enum)
- **Iterator** (đã có ở trait.md, sâu hơn cần)

Đọc song song `generic-visual.md` để thấy mọi khái niệm bằng hình ảnh.

> Generic là cách Rust nói: "đừng viết code 2 lần, hãy parametrize nó". Hiểu generic = hiểu ngôn ngữ trừu tượng của Rust.
