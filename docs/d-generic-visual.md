# Generic — TOÀN BỘ qua HÌNH VẼ

> Companion visual cho `generic.md`. Mỗi khái niệm có hình minh hoạ ASCII. Đọc tuần tự từ đầu.

---

## Mục lục

1. [Bức tranh lớn: Generic là gì?](#1-bức-tranh-lớn)
2. [3 loại Parameter](#2-ba-loại-parameter)
3. [Generic Function — Sơ đồ instantiation](#3-generic-function)
4. [Generic Struct — Layout](#4-generic-struct)
5. [Generic Enum — Option layout](#5-generic-enum)
6. [Generic Method & Impl Block](#6-generic-method-impl)
7. [Trait Bound — sơ đồ filter](#7-trait-bound)
8. [Multiple Bounds & Where](#8-multiple-bounds-where)
9. [Monomorphization — visual](#9-monomorphization)
10. [Code bloat trade-off](#10-code-bloat)
11. [Generic Struct memory layout](#11-generic-memory)
12. [Lifetime as Generic Parameter](#12-lifetime-as-generic)
13. [Lifetime Bound T: 'a](#13-lifetime-bound)
14. [PhantomData — zero-size marker](#14-phantomdata)
15. [PhantomData use cases](#15-phantomdata-usecases)
16. [Const Generics](#16-const-generics)
17. [Variance — 3 loại](#17-variance-3-loại)
18. [Variance subtype diagram](#18-variance-subtype)
19. [Why Cell is invariant](#19-cell-invariant)
20. [Turbofish ::<>](#20-turbofish)
21. [Type-state Builder](#21-type-state)
22. [Conditional Implementation](#22-conditional-impl)
23. [Mind map](#23-mind-map)

---

## 1. Bức tranh lớn

```
   ╔════════════════════════════════════════════════════════╗
   ║                       GENERIC                          ║
   ║                                                        ║
   ║   = "Viết code 1 lần, dùng cho mọi type"               ║
   ║                                                        ║
   ║   Tư duy toán học:                                     ║
   ║                                                        ║
   ║     "Cho mọi T, hàm này làm việc với T..."             ║
   ║                                                        ║
   ║                ↓                                       ║
   ║                                                        ║
   ║     fn process<T>(x: T) -> T { ... }                   ║
   ║                                                        ║
   ║   Compiler:                                            ║
   ║     - Compile time: thấy gọi với i32, f64, String,...  ║
   ║     - SINH RA: 3 version tối ưu cho từng type          ║
   ║     - Runtime: nhanh như C                             ║
   ║                                                        ║
   ╚════════════════════════════════════════════════════════╝
```

### So sánh với các ngôn ngữ

```
                   Tốc độ    Type-safe    Lỗi rõ ràng   Box primitive?
                   ──────    ─────────    ───────────   ──────────────
   C macros        ⚡⚡⚡       ❌           ❌            -
   C void*         ⚡         ❌           -             -
   C++ template    ⚡⚡⚡       ✓ (duck)     ❌            ❌
   Java generic    ⚡         ✓            ✓             ✓ (slow)
   Rust generic    ⚡⚡⚡       ✓            ✓             ❌ (zero-cost)
```

---

## 2. Ba loại Parameter

```
              ┌──────────────────────────────────┐
              │       GENERIC PARAMETERS         │
              └──────────────────────────────────┘
                              │
       ┌──────────────────────┼──────────────────────┐
       ▼                      ▼                      ▼
   
   ┌──────────────┐    ┌──────────────┐     ┌──────────────┐
   │     TYPE     │    │   LIFETIME   │     │    CONST     │
   │     <T>      │    │     <'a>     │     │ <const N>    │
   ├──────────────┤    ├──────────────┤     ├──────────────┤
   │              │    │              │     │              │
   │ - struct V<T>│    │ - struct &'a │     │ - [T; N]     │
   │ - fn f<T>()  │    │ - fn f<'a>() │     │ - Matrix<R,C>│
   │ - enum E<T>  │    │ - reference  │     │ - compile    │
   │              │    │   tuổi thọ   │     │   const      │
   │              │    │              │     │              │
   │ Phổ biến     │    │ Phổ biến nhì │     │ Mới (1.51+)  │
   │ nhất         │    │              │     │              │
   └──────────────┘    └──────────────┘     └──────────────┘

   Kết hợp cả 3:
   ──────────────
   struct Buffer<'a, T, const N: usize> {
       slice: &'a mut [T; N],
              ────    ─  ─
              'a       T  N
   }
```

---

## 3. Generic Function

```rust
fn identity<T>(x: T) -> T { x }
```

### Sơ đồ instantiation

```
   Code bạn viết:
   ─────────────
   
   fn identity<T>(x: T) -> T {
       x
   }
                  ▲
                  │ template
                  │
   ┌──────────────┴──────────────┐
   │                              │
   call identity(5)         call identity("hi")
   call identity(3.14)
   
   COMPILER MONOMORPHIZE:
   
   fn identity__i32(x: i32) -> i32 { x }
   fn identity__f64(x: f64) -> f64 { x }
   fn identity__strslice(x: &str) -> &str { x }
   
   Mỗi version optimize riêng → ASM tốt nhất cho từng type
```

### Inference vs Turbofish

```
   ┌─────────────────────────────────────────────┐
   │ Compiler suy luận T:                        │
   │   let a = identity(5);                      │
   │              ───────                        │
   │   5: i32 → T = i32 (tự suy)                 │
   └─────────────────────────────────────────────┘
   
   ┌─────────────────────────────────────────────┐
   │ Khi suy không được → turbofish:             │
   │   let n = "42".parse::<i32>().unwrap();     │
   │                  ───────                    │
   │                  T = i32 explicit           │
   └─────────────────────────────────────────────┘
```

---

## 4. Generic Struct

```rust
struct Point<T> { x: T, y: T }
```

### Layout sau monomorph

```
   struct Point<T> { x: T, y: T }
                ▲
                │ instantiate
                │
   ┌────────────┼────────────┐
   ▼            ▼            ▼
   
   Point<i32>:                Point<f64>:
   ┌────────┬────────┐        ┌────────┬────────┐
   │  i32   │  i32   │        │  f64   │  f64   │
   │  4 B   │  4 B   │        │  8 B   │  8 B   │
   └────────┴────────┘        └────────┴────────┘
   = 8 byte                   = 16 byte
   
   
   Point<String>:
   ┌──────┬──────┬──────┬──────┬──────┬──────┐
   │ ptr  │ len  │ cap  │ ptr  │ len  │ cap  │
   │      x (String)   │      y (String)     │
   └──────┴──────┴──────┴──────┴──────┴──────┘
   = 48 byte (2 × 24-byte String header)
```

→ **Mỗi instantiation có layout khác nhau!**

### So sánh với Java generic

```
   Java: List<Integer>          Rust: Vec<i32>
   ────────────────────         ──────────────
   
   Internal: Object[]            Internal: *mut i32
                 │                            │
                 ▼                            ▼
            ┌──────────────┐           ┌──────────────┐
            │ pointer to   │           │ inline       │
            │ Integer ─────┼──► [42]   │ data: [42]   │
            │ Integer ─────┼──► [43]   │       [43]   │
            └──────────────┘           └──────────────┘
            Heap: 2 box per int        Stack-friendly,
            Cache-unfriendly           in-place
```

---

## 5. Generic Enum

```rust
enum Option<T> {
    Some(T),
    None,
}
```

### Layout Option<i32>

```
   Option<i32>:
   ┌─────┬──────────┐
   │ tag │  data    │
   │ 4 B │  4 B     │
   └─────┴──────────┘
   = 8 byte (alignment)
   
   tag = 0:                       tag = 1:
   ┌─────┬──────────┐              ┌─────┬──────────┐
   │  0  │   42     │              │  1  │   X X    │
   └─────┴──────────┘              └─────┴──────────┘
   Some(42)                        None (data không dùng)
```

### Niche optimization với Option<&T>

```
   Option<&i32>:
   ┌──────────────────────┐
   │ ptr (8 byte)         │
   └──────────────────────┘
   = 8 byte (KHÔNG có tag!)
   
   Vì sao? Vì:
     &i32 KHÔNG bao giờ là 0x0
     → Rust dùng 0x0 để encode None
     
   Some(&x):  ┌──────────────────────┐
              │ 0x7FFF...1234         │
              └──────────────────────┘
   
   None:      ┌──────────────────────┐
              │ 0x0000...0000         │  ← niche
              └──────────────────────┘
```

### Result<T, E>

```
   Result<i32, String>:
   ┌─────┬─────────────────────────┐
   │ tag │ data (max(i32, String)) │
   │ 4 B │       24 B               │
   └─────┴─────────────────────────┘
   = 32 byte (alignment)
   
   tag = 0: Ok(i32)
   tag = 1: Err(String)
```

---

## 6. Generic Method & Impl Block

```rust
impl<T> Point<T> {
    fn new(x: T, y: T) -> Self { Point { x, y } }
}
```

### Đọc cú pháp

```
   impl<T> Point<T> {
        ─       ─
        │       │
        │       └─ Tôi đang impl cho Point<T>
        │
        └─ KHAI BÁO: trong scope này, T là parameter
   
   Hai chỗ này KHÔNG là 1 type — chỉ là cùng tên:
   - Bên trái <T>: khai báo
   - Bên phải Point<T>: sử dụng
```

### Impl chỉ cho 1 instantiation

```
   impl Point<f64> {           ← KHÔNG có <T>
       fn distance(&self) -> f64 { ... }
   }
   
   ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
   │ Point<i32>     │  │ Point<f64>     │  │ Point<String>  │
   │                │  │                │  │                │
   │ new()    ✓     │  │ new()    ✓     │  │ new()    ✓     │
   │ distance() ❌  │  │ distance() ✓   │  │ distance() ❌  │
   └────────────────┘  └────────────────┘  └────────────────┘
                          │
                          │ ONLY Point<f64> có method này
```

---

## 7. Trait Bound

```rust
fn largest<T: PartialOrd>(v: &[T]) -> T { ... }
```

### Sơ đồ filter

```
                  Tất cả types
                       │
                       ▼
         ┌─────────────────────────┐
         │  Filter: T: PartialOrd  │
         └─────────┬───────────────┘
                   │
                   ▼
              ┌────────────┐
              │ i32  ✓     │
              │ f64  ✓     │
              │ String ✓   │
              │ MyType ✓   │   ← chỉ nếu impl PartialOrd
              │ ────────── │
              │ MyTypeX ❌ │   ← không impl → reject
              └────────────┘
```

### Trait bound = capability filter

```
   ┌────────────────────────────────────────────┐
   │  fn largest<T: PartialOrd>(v: &[T]) -> T   │
   │                ──────────                  │
   │                                            │
   │   "T phải có khả năng SO SÁNH"             │
   │                                            │
   │   Hàm dùng > và < bên trong:               │
   │     item > largest                         │
   │              ▲                             │
   │              │ cần T: PartialOrd           │
   └────────────────────────────────────────────┘
```

### Bound = explicit contract

```
   C++ template (duck typing):
   ──────────────
   template<typename T>
   T largest(...) {
       if (a > b) ...   ← Compiler không biết T cần gì
                          → Lỗi DEEP khi instantiate
   }
   
   Rust generic (explicit):
   ────────────────────
   fn largest<T: PartialOrd>(...) {
                ──────────
                ↑ CONTRACT rõ ràng
       if a > b ...       ← Compiler biết, type-check ngay
   }
```

---

## 8. Multiple Bounds & Where

### Inline bounds

```rust
fn process<T: Clone + Debug + PartialEq>(x: T) { ... }
//             ─────   ─────   ─────────
//             T phải có CẢ 3
```

### Where clause (dễ đọc hơn)

```rust
fn process<T, U>(x: U)
where
    T: Clone + Debug + PartialEq,
    U: Iterator<Item = T> + ExactSizeIterator,
{ ... }
```

### Visual

```
                  bound matrix
   ┌────────────────────────────────────────┐
   │            T trait     U trait          │
   │  ┌──────┬──────────┬──────────┬─────┐  │
   │  │  T   │ Clone    │ Debug    │ Eq  │  │
   │  ├──────┼──────────┼──────────┼─────┤  │
   │  │      │   ✓      │   ✓      │  ✓  │  │
   │  ├──────┴──────────┴──────────┴─────┤  │
   │  │  U: Iterator<Item = T> + ExactSize│  │
   │  └────────────────────────────────────┘ │
   └─────────────────────────────────────────┘
```

### Where cho phép bound mà inline không làm được

```
   ┌─────────────────────────────────┐
   │  fn foo<T>() where             │
   │      Vec<T>: Clone,             │   ← bound TRÊN type
   │                                 │     không phải param trực tiếp
   │      <T as Iterator>::Item:     │
   │           Display,              │   ← bound trên associated type
   │  { ... }                        │
   └─────────────────────────────────┘
```

---

## 9. Monomorphization

### Quá trình bước-bước

```
   ╔════════════════════════════════════════════════════════╗
   ║                  COMPILATION PROCESS                   ║
   ╠════════════════════════════════════════════════════════╣
   ║                                                        ║
   ║  Step 1: Parse generic code                            ║
   ║  ──────                                                ║
   ║  fn double<T: Add<Output=T> + Copy>(x: T) -> T {       ║
   ║      x + x                                             ║
   ║  }                                                     ║
   ║                                                        ║
   ║  ↓                                                     ║
   ║                                                        ║
   ║  Step 2: Scan call sites                               ║
   ║  ──────                                                ║
   ║  double(5)      → T = i32                              ║
   ║  double(3.14)   → T = f64                              ║
   ║  double(1u8)    → T = u8                               ║
   ║                                                        ║
   ║  ↓                                                     ║
   ║                                                        ║
   ║  Step 3: Generate monomorphic versions                 ║
   ║  ──────                                                ║
   ║  fn double__i32(x: i32) -> i32 {                       ║
   ║      x + x       ← dùng ADD instruction                ║
   ║  }                                                     ║
   ║                                                        ║
   ║  fn double__f64(x: f64) -> f64 {                       ║
   ║      x + x       ← dùng FADD instruction               ║
   ║  }                                                     ║
   ║                                                        ║
   ║  fn double__u8(x: u8) -> u8 {                          ║
   ║      x + x       ← ADD với 8-bit                       ║
   ║  }                                                     ║
   ║                                                        ║
   ║  ↓                                                     ║
   ║                                                        ║
   ║  Step 4: Replace call sites                            ║
   ║  ──────                                                ║
   ║  double(5)    →  double__i32(5)                        ║
   ║  double(3.14) →  double__f64(3.14)                     ║
   ║  double(1u8)  →  double__u8(1)                         ║
   ║                                                        ║
   ║  ↓                                                     ║
   ║                                                        ║
   ║  Step 5: Optimize (inline, constant fold, ...)         ║
   ║  ──────                                                ║
   ║  double__i32(5) inline → just 10                       ║
   ║  (function disappears!)                                ║
   ║                                                        ║
   ╚════════════════════════════════════════════════════════╝
```

### Final binary

```
   TEXT segment (after optimize):
   ┌──────────────────────────────────────┐
   │ main:                                 │
   │   mov eax, 10        ; double(5) = 10 │
   │   movsd xmm0, 6.28   ; double(3.14)   │
   │   mov al, 2          ; double(1u8)    │
   │                                       │
   │ (no double() function visible)        │
   └──────────────────────────────────────┘
```

---

## 10. Code Bloat Trade-off

```
   ╔════════════════════════════════════════════════════════╗
   ║              CODE SIZE vs SPEED                        ║
   ╠════════════════════════════════════════════════════════╣
   ║                                                        ║
   ║   GENERIC                          DYN TRAIT           ║
   ║   (static dispatch)                (dynamic dispatch)  ║
   ║                                                        ║
   ║   Code in binary:                  Code in binary:     ║
   ║   ┌─────┐ ┌─────┐                  ┌─────┐             ║
   ║   │fn__1│ │fn__2│                  │ fn  │             ║
   ║   ├─────┤ ├─────┤                  └─────┘             ║
   ║   │fn__3│ │fn__4│                                       ║
   ║   ├─────┤ ├─────┤                  + vtables for       ║
   ║   │fn__5│ │fn__6│                    each type         ║
   ║   └─────┘ └─────┘                                       ║
   ║                                                        ║
   ║   ↕ N versions                     ↕ 1 version         ║
   ║                                                        ║
   ║   Speed:                           Speed:              ║
   ║   ⚡⚡⚡⚡⚡                            ⚡⚡⚡                ║
   ║   (inline, optimized               (vtable lookup,     ║
   ║    per type)                        ~2-3 ns extra)     ║
   ║                                                        ║
   ║   Binary size:                     Binary size:        ║
   ║   📦📦📦📦📦                        📦                  ║
   ║                                                        ║
   ║   Compile time:                    Compile time:       ║
   ║   🐢🐢🐢                            🚀                  ║
   ║                                                        ║
   ║   I-cache pressure:                I-cache pressure:   ║
   ║   ⚠ có thể đẩy code ra cache       ✓ code nhỏ ở cache  ║
   ║                                                        ║
   ╚════════════════════════════════════════════════════════╝
```

### Quy tắc thực dụng

```
   ┌────────────────────────────────────────────────┐
   │  Mặc định:  GENERIC                            │
   │                                                │
   │  Chuyển sang DYN khi:                          │
   │  - Cần collection of mixed types               │
   │  - Plugin / dynamic registration               │
   │  - Generic code lớn → binary bloat thực sự     │
   │  - Compile time quá lâu                        │
   │                                                │
   │  Hybrid: outer generic, inner non-generic     │
   │  fn print<T: Display>(x: T) {                  │
   │      print_impl(x.to_string())  ← non-gen     │
   │  }                                             │
   └────────────────────────────────────────────────┘
```

---

## 11. Generic Struct Memory Layout

### Pair<T, U>

```rust
struct Pair<T, U> { first: T, second: U }
```

```
   Pair<i32, i32>:                  Pair<u8, u64>:
   ┌──────┬──────┐                  ┌──┬───────┬───────┐
   │ i32  │ i32  │                  │u8│padding│  u64  │
   │ 4 B  │ 4 B  │                  │1B│  7 B  │  8 B  │
   └──────┴──────┘                  └──┴───────┴───────┘
   = 8 byte                         = 16 byte
   
   Padding: vì u64 phải align 8.
```

### Vec<T>

```
   Vec<T>:
   
   STACK (24 byte, MỌI T):
   ┌──────────────┐
   │ ptr (8 B) ───┼──► HEAP (alloc động):
   │ len (8 B)    │    ┌───┬───┬───┬...┐
   │ cap (8 B)    │    │ T │ T │ T │   │
   └──────────────┘    └───┴───┴───┴...┘
                       Mỗi cell = sizeof(T) byte
   
   Vec<i32>: cells 4 byte
   Vec<u8>:  cells 1 byte
   Vec<String>: cells 24 byte (header)
   Vec<[i32; 100]>: cells 400 byte
```

---

## 12. Lifetime as Generic Parameter

```rust
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str { ... }
```

### Sơ đồ subtyping

```
   Lifetime giống như "kích thước":
   
   'static  ──────────────────────────────────────────►
   'a            ─────────►
   'b                  ────►
   
   'static ⊇ 'a ⊇ 'b
   
   Subtyping (mạnh hơn ⊆ yếu hơn):
   'static ⊆ 'a ⊆ 'b
   "có thể dùng" ↑
```

### Instantiation theo lifetime

```
   fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str
   
   Call site 1:
   ───────────
   let x: &'long str = ...;
   let y: &'long str = ...;
   longest(x, y);          → 'a = 'long
   
   Call site 2:
   ───────────
   let x: &'short str = ...;
   let y: &'short str = ...;
   longest(x, y);          → 'a = 'short
   
   Hỗn hợp:
   ────────
   let x: &'long str = ...;
   let y: &'short str = ...;
   longest(x, y);          → 'a = 'short (intersection)
                              = min('long, 'short)
```

---

## 13. Lifetime Bound T: 'a

```rust
struct Wrapper<'a, T: 'a> {
    inner: &'a T,
}
```

### Đọc T: 'a

```
   T: 'a   →  "T sống ít nhất 'a"
   
   Tại sao cần?
   ─────────
   
   &'a T  →  reference với lifetime 'a, tới T
              ▲
              │ T phải còn sống suốt 'a
              │ → T: 'a
   
   Visual:
   
   'a:    ────────────────────►
   T:     ──────────────────────►  ← T: 'a (T sống ÍT NHẤT bằng 'a)
   &'a T: ────────────────────►    ← OK
```

### `T: 'static`

```
   T: 'static  →  T không vay gì có lifetime ngắn hơn
   
   Examples:
   ────────
   i32: 'static       ✓ (owned)
   String: 'static    ✓ (owned)
   Vec<i32>: 'static  ✓ (owned, T owned)
   &'static str: 'static  ✓ (static literal)
   
   &'a str: NOT 'static (vay 'a)
   Vec<&'a str>: NOT 'static
```

---

## 14. PhantomData — zero-size marker

```rust
struct MyTag<T> {
    value: i32,
    _marker: PhantomData<T>,
}
```

### Layout

```
   MyTag<T>:
   ┌──────────────┬───┐
   │ value: i32   │ ∅ │  ← PhantomData<T>: ZERO byte!
   │    4 B       │ 0 │
   └──────────────┴───┘
   = 4 byte (PhantomData không chiếm chỗ)
   
   Layout giống nhau dù T = i32, String, hay anything.
```

### Vì sao phải có PhantomData?

```
   struct MyTag<T> {
       value: i32,                ← T không xuất hiện ở field
   }
   
   ❌ ERROR: parameter T is never used
   
   Lý do compiler cấm:
   ─────────────────
   - Variance của MyTag<T> không xác định được
   - Drop semantics: T có cần drop không?
   - Send/Sync: phụ thuộc T không?
   
   → Bắt buộc dùng PhantomData để "trỏ" T mặc dù không lưu data.
```

---

## 15. PhantomData use cases

### Use case 1: Type-state (compile-time state machine)

```rust
struct Connection<State> {
    socket: TcpStream,
    _state: PhantomData<State>,
}

struct Disconnected;
struct Connected;
struct Authenticated;
```

```
   Visual state machine:
   ──────────────
   
   ┌─────────────────────────┐
   │ Connection<Disconnected> │
   └─────────────┬───────────┘
                 │ .connect()
                 ▼
   ┌─────────────────────────┐
   │ Connection<Connected>    │
   └─────────────┬───────────┘
                 │ .login(user, pass)
                 ▼
   ┌─────────────────────────┐
   │ Connection<Authenticated>│
   │                          │
   │ .query(sql)  ✓ Available │
   └──────────────────────────┘
   
   Compile error nếu gọi .query() trên Disconnected!
```

### Use case 2: Variance control

```rust
struct CovInT<T>     { _: PhantomData<T> }          // covariant
struct ContraInT<T>  { _: PhantomData<fn(T)> }      // contravariant
struct InvarInT<T>   { _: PhantomData<*mut T> }     // invariant
```

```
   ┌────────────────────────────────────────────┐
   │ Variance được DECIDE bởi PhantomData kiểu: │
   │                                            │
   │  PhantomData<T>         → covariant in T   │
   │  PhantomData<&T>        → covariant in T   │
   │  PhantomData<Box<T>>    → covariant in T   │
   │                                            │
   │  PhantomData<fn(T)>     → contravariant    │
   │                                            │
   │  PhantomData<&mut T>    → invariant        │
   │  PhantomData<*mut T>    → invariant        │
   │  PhantomData<Cell<T>>   → invariant        │
   │  PhantomData<fn(T) -> T>→ invariant        │
   └────────────────────────────────────────────┘
```

### Use case 3: Lifetime marker

```rust
struct CWrapper<'a, T> {
    ptr: *const T,                       // raw, không có lifetime
    _marker: PhantomData<&'a T>,         // giả vờ chứa &'a T
}
```

```
   Bên trong: *const T (raw, không gắn lifetime)
   Bên ngoài: compiler check như có &'a T
   
   → Lifetime check vẫn hoạt động
   → Nhưng API có thể dùng raw pointer flexibility
```

---

## 16. Const Generics

```rust
fn first<T, const N: usize>(arr: [T; N]) -> T where T: Copy {
    arr[0]
}
```

### Visual

```
   fn first<T, const N: usize>(arr: [T; N]) -> T
            ─       ────────       ─────
            type    const param    size là parameter
            param
   
   Call sites:
   ──────────
   first([1, 2, 3])           → T = i32, N = 3
   first([1.0, 2.0])           → T = f64, N = 2
   first(["a"; 100])           → T = &str, N = 100
   
   Compiler sinh:
   first__i32__3
   first__f64__2
   first__&str__100
```

### Generic Matrix

```rust
struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}
```

```
   Matrix<3, 4>:
   ┌──────┬──────┬──────┬──────┐  ┐
   │ 0,0  │ 0,1  │ 0,2  │ 0,3  │  │
   ├──────┼──────┼──────┼──────┤  │
   │ 1,0  │ 1,1  │ 1,2  │ 1,3  │  ├ R=3 rows
   ├──────┼──────┼──────┼──────┤  │
   │ 2,0  │ 2,1  │ 2,2  │ 2,3  │  │
   └──────┴──────┴──────┴──────┘  ┘
       └─── C=4 columns ───┘
   
   Size compile-time biết: 3 × 4 × 8 = 96 byte
   → Stack allocate được!
   → No heap, no indirection
```

### So với Vec<Vec<f64>>

```
   Matrix<3, 4>:                     Vec<Vec<f64>>:
   ─────────────                     ───────────────
   
   STACK (96 byte):                  STACK (24 byte):
   ┌──────────────┐                  ┌──────────────┐
   │ inline data  │                  │ ptr ─────────┼──► HEAP:
   │ 3×4 = 12 f64 │                  │ len = 3      │    ┌────────┐
   └──────────────┘                  │ cap = 3      │    │ Vec #1 ─┼─►HEAP
                                     └──────────────┘    │ Vec #2 ─┼─►HEAP
                                                          │ Vec #3 ─┼─►HEAP
                                                          └────────┘
                                                          + 3 heap allocs
   
   Total:                            Total:
   - 1 stack alloc                   - 4 heap allocs
   - Cache friendly                  - Cache unfriendly
   - Size cố định                    - Linh hoạt size
```

---

## 17. Variance — 3 loại

```
   ╔══════════════════════════════════════════════════════════╗
   ║                  VARIANCE TYPES                          ║
   ╠══════════════════════════════════════════════════════════╣
   ║                                                          ║
   ║  COVARIANT (thuận chiều)                                 ║
   ║  ─────────                                               ║
   ║  Nếu S ⊆ T (S là subtype của T):                        ║
   ║     F<S> ⊆ F<T>                                         ║
   ║                                                          ║
   ║  Examples:                                               ║
   ║    &T, Box<T>, Vec<T> (read-only access)                ║
   ║                                                          ║
   ║                                                          ║
   ║  CONTRAVARIANT (ngược chiều)                            ║
   ║  ──────────────                                          ║
   ║  Nếu S ⊆ T:                                             ║
   ║     F<S> ⊇ F<T>                                         ║
   ║                                                          ║
   ║  Examples:                                               ║
   ║    fn(T)  — argument position                            ║
   ║                                                          ║
   ║                                                          ║
   ║  INVARIANT (không liên quan)                            ║
   ║  ──────────                                              ║
   ║  Không có quan hệ giữa F<S> và F<T>                     ║
   ║                                                          ║
   ║  Examples:                                               ║
   ║    &mut T, Cell<T>, Mutex<T>, *mut T                    ║
   ║                                                          ║
   ╚══════════════════════════════════════════════════════════╝
```

---

## 18. Variance Subtype Diagram

### Lifetime subtype: 'static ⊆ 'a

```
   'static (sống mãi)  ───────────────────────────────────►
   'a      (sống 'a)        ───────────────────►
   
   'static ⊆ 'a   ("static có thể THAY THẾ 'a")
   
   Vì sao? "static sống lâu hơn → đáp ứng yêu cầu của 'a"
```

### Covariant: Box<&'static T> ⊆ Box<&'a T>

```
   Box<&'static i32>  ⊆  Box<&'a i32>
   ─────────────────       ─────────
   "mạnh hơn"               "yếu hơn"
   (chứa ref vĩnh viễn)     (chứa ref 'a)
   
   → Có thể pass Box<&'static i32> ở chỗ cần Box<&'a i32>
   
   Visual:
   
                       ╔══════════════════════╗
                       ║ cần Box<&'a i32>     ║
                       ╚══════════════════════╝
                                ▲
                                │ pass được
                                │
   ┌──────────────────────────────────────┐
   │ có Box<&'static i32>                  │
   └──────────────────────────────────────┘
```

### Contravariant: fn(&'a T) ⊆ fn(&'static T)

```
   fn(&'a T)   ⊆   fn(&'static T)
   ────────         ───────────
   "mạnh hơn"        "yếu hơn"
   
   Vì sao?
   ──────
   fn(&'a T) = "hàm xử lý reference với MỌI lifetime"
              → có thể xử lý cả static (vì static là 'a đặc biệt)
   
   fn(&'static T) = "hàm CHỈ xử lý reference vĩnh viễn"
                  → không xử lý được ref ngắn
   
   → fn(&'a T) "mạnh hơn" → có thể dùng thay fn(&'static T)
```

---

## 19. Why Cell is invariant

```
   Cell<T>: invariant in T
```

### Giả sử Cell là covariant

```
   Bước 1: 'static ⊆ 'a → Cell<&'static T> ⊆ Cell<&'a T>
   
   Bước 2:
   let cell: Cell<&'static str> = Cell::new("hello");
   let cell_ref: &Cell<&'a str> = &cell;    ← cov upcast
   
   Bước 3 (NGUY HIỂM):
   let short_str: String = ...;
   cell_ref.set(&short_str);              ← ghi 'a vào 'static!
   
   Sau khi short_str chết:
   cell.get()  → &'static str dangling!
   
   ❌ CRASH / undefined behavior
```

### Visual

```
   Nếu Cell covariant:
   
   ┌───────────────────────────────────┐
   │ cell: Cell<&'static str>          │
   │   value: "hello" (static)         │
   └────────────┬──────────────────────┘
                │ upcast covariant
                ▼
   ┌───────────────────────────────────┐
   │ &Cell<&'a str>                     │
   │   .set(short_lived_ref)            │
   └────────────┬──────────────────────┘
                │
                ▼
   ┌───────────────────────────────────┐
   │ cell: Cell<&'static str>          │
   │   value: ngắn (đáng lẽ static)    │
   └───────────────────────────────────┘
                │
                ▼ short_lived chết
   ┌───────────────────────────────────┐
   │ cell.get() → dangling ref!        │  ❌
   └───────────────────────────────────┘
   
   → Rust CẤM cov cho Cell → invariant
```

---

## 20. Turbofish ::<>

```rust
let n = "42".parse::<i32>().unwrap();
//              ────────
//              turbofish
```

### Hình dạng "cá"

```
       ┌──────┐
       │      │
   ::< T >    ← "con cá bơi"
       │      │
       └──────┘
       fins!
```

### Khi nào cần?

```
   Case 1: parse return generic
   ──────
   "42".parse()    → type không xác định
   "42".parse::<i32>()   → ✓ explicit
   
   Case 2: collect generic
   ──────
   (0..10).collect()         → ❌ ambiguous
   (0..10).collect::<Vec<_>>()  → ✓
   
   Case 3: từ context không suy ra
   ──────
   let v = identity::<i32>(5);    // explicit T = i32
```

### Lý do design

```
   Tại sao ::<> mà không <T>?
   ──────
   foo<i32>(x)
   
   Cú pháp này AMBIGUOUS:
   foo < i32 > (x)
       ─       ─
       (so sánh < với i32, sau đó > với (x)?)
   
   → Rust thêm :: để parser biết "đây là generic"
   foo::<i32>(x)   ← rõ ràng
```

---

## 21. Type-state Builder

```rust
struct Builder<State> {
    name: Option<String>,
    age: Option<u32>,
    _state: PhantomData<State>,
}

struct Empty;
struct WithName;
struct WithAll;
```

### State machine visualization

```
   ┌──────────────────┐
   │ Builder<Empty>   │
   │                  │
   │ .new()           │
   │ .name(s) → ─────┼──►┌──────────────────┐
   │                  │   │ Builder<WithName>│
   │ ❌ .build()       │   │                  │
   └──────────────────┘   │ .name(s)  ← override
                          │ .age(n)  → ─────┼──►┌──────────────────┐
                          │                  │   │ Builder<WithAll> │
                          │ ❌ .build()       │   │                  │
                          └──────────────────┘   │ .age(n)  ← override
                                                 │ .build()  ✓      │
                                                 │  → User          │
                                                 └──────────────────┘
   
   Compile error nếu skip step!
   Builder::new().build()  → ❌ Empty không có build
```

### So sánh với runtime check

```
   RUNTIME CHECK:                  COMPILE-TIME (Type-state):
   ──────────                       ─────────────────────
   
   builder.name = Some(s);          builder = builder.name(s);
   builder.build();                  ← state machine prevents
       ↓                                if !name.is_some() { ... }
       runtime panic if !name           never reached if illegal
       
   ❌ Lỗi muộn                       ✓ Lỗi sớm
```

---

## 22. Conditional Implementation

```rust
struct Wrapper<T> { value: T }

impl<T> Wrapper<T> {
    fn new(v: T) -> Self { ... }
}

impl<T: Display> Wrapper<T> {        // chỉ khi T: Display
    fn print(&self) {
        println!("{}", self.value);
    }
}
```

### Visual

```
                    Wrapper<T> capability matrix
                    ────────────────────────────
   
   ┌───────────────────────┬───────┬───────┐
   │                       │ new() │print()│
   ├───────────────────────┼───────┼───────┤
   │ Wrapper<i32>          │   ✓   │   ✓   │  ← i32: Display
   │ Wrapper<String>       │   ✓   │   ✓   │  ← String: Display
   │ Wrapper<f64>          │   ✓   │   ✓   │  ← f64: Display
   │ Wrapper<Vec<i32>>     │   ✓   │   ❌  │  ← Vec không Display
   │ Wrapper<MyType>       │   ✓   │   ?   │  ← depends
   └───────────────────────┴───────┴───────┘
   
   Method available CHỈ khi type satisfy bound
   → "Trait-based method specialization"
```

### Blanket + conditional

```rust
// std::string::ToString:
impl<T: Display + ?Sized> ToString for T {
    fn to_string(&self) -> String { ... }
}
```

```
   Visual:
   ──────
   
   Universe of types
        │
        │ filter T: Display
        ▼
   ┌────────────────────────┐
   │ {T: Display}           │
   │                        │
   │ Auto get: ToString     │
   │                        │
   │ i32, f64, String,      │
   │ User (if Display),     │
   │ ...                    │
   └────────────────────────┘
```

---

## 23. Mind map

```
                              GENERIC
                                 │
       ┌─────────────────────────┼─────────────────────────┐
       ▼                         ▼                         ▼
   PARAMETERS                 BOUNDS                    MEMORY
                                                          
   ┌────┴────┐             ┌───┴───┐                  ┌───┴───┐
   ▼         ▼             ▼       ▼                  ▼       ▼
  3 loại:               trait     where           mono-     code
  - <T>                 bound     clause          morph     bloat
  - <'a>                                          (compile  trade
  - <const N>                                     -time)    -off
   
   
       ┌─────────────────────────────────────────────────┐
       ▼                                                 ▼
   PATTERNS                                          PITFALLS
                                                          
   ┌──────────────────┐                          ┌────────────┐
   │ PhantomData      │                          │ Code bloat │
   │ Type-state       │                          │ Inference  │
   │ Turbofish ::<>   │                          │ Variance   │
   │ Builder          │                          │ Lifetime   │
   │ Newtype          │                          │ mismatch   │
   │ Const generics   │                          └────────────┘
   └──────────────────┘
   
   
       ┌─────────────────────────────────────────────────┐
       ▼                                                 ▼
   VARIANCE                                          INTEGRATION
                                                          
   ┌────────────┐                                   ┌───────────┐
   │ Covariant  │                                   │ Trait     │
   │ Contra     │                                   │ Lifetime  │
   │ Invariant  │                                   │ Memory    │
   │            │                                   │ Iterator  │
   │ Box<T>     │                                   └───────────┘
   │ fn(T)      │
   │ Cell<T>    │
   └────────────┘
```

### Quan hệ với Trait

```
   ┌─────────────────────────────────────────────────────┐
   │                                                     │
   │   Trait Bound = constraint trên Generic            │
   │                                                     │
   │   fn foo<T: Trait>(x: T)                           │
   │              ─────                                  │
   │              ↑ Trait từ trait.md                   │
   │   ↑                                                 │
   │   Generic từ generic.md                            │
   │                                                     │
   │   Hai khái niệm gắn chặt:                          │
   │   - Generic: parameter là type                     │
   │   - Trait bound: contract trên type đó             │
   │                                                     │
   └─────────────────────────────────────────────────────┘
```

---

## Tổng kết — 10 ý cốt lõi visual

```
   ╔══════════════════════════════════════════════════════════╗
   ║                                                          ║
   ║  1. Generic = parametric polymorphism                    ║
   ║     fn f<T>(x: T) → mỗi T có 1 instantiation             ║
   ║                                                          ║
   ║  2. 3 loại param: <T>, <'a>, <const N>                   ║
   ║                                                          ║
   ║  3. Trait bound RÕ RÀNG: <T: Trait>                      ║
   ║     (không như C++ duck typing)                          ║
   ║                                                          ║
   ║  4. Monomorphization:                                    ║
   ║     compile-time clone code → 0-cost runtime             ║
   ║                                                          ║
   ║  5. Trade-off: speed ↑↑↑, binary size ↑↑                 ║
   ║                                                          ║
   ║  6. Lifetime cũng là generic param (subtyping)           ║
   ║                                                          ║
   ║  7. PhantomData<T>: zero-byte marker                     ║
   ║     ← cần khi T không xuất hiện trong field              ║
   ║                                                          ║
   ║  8. Const generic <const N: usize>:                      ║
   ║     compile-time value (vd size of array)                ║
   ║                                                          ║
   ║  9. Variance (3 loại):                                   ║
   ║     covariant: Box<T>, &T                                ║
   ║     contravariant: fn(T)                                 ║
   ║     invariant: Cell<T>, &mut T                           ║
   ║                                                          ║
   ║ 10. Turbofish ::<T> = explicit type instantiation        ║
   ║                                                          ║
   ╚══════════════════════════════════════════════════════════╝
```

---

> Đọc song song `generic.md` (lý thuyết) để hiểu sâu, file này để có hình ảnh trong đầu.
>
> Chủ đề tiếp theo: **Closure** — function-as-value, Fn/FnMut/FnOnce traits, capture environment, move closures, async closure.
