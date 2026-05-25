# Memory Model — TOÀN BỘ qua HÌNH VẼ

> File này là **companion visual** cho `memory-model.md`. Mọi khái niệm đều được vẽ ra. Đọc từ trên xuống dưới — mỗi phần xây trên phần trước.

---

## Mục lục hình minh họa

1. [Bức tranh lớn: Máy tính nhìn từ trên xuống](#1-bức-tranh-lớn)
2. [Hệ phân cấp bộ nhớ (Memory Hierarchy)](#2-hệ-phân-cấp-bộ-nhớ)
3. [Virtual Memory — Ảo hóa địa chỉ](#3-virtual-memory)
4. [Process Address Space — 6 vùng](#4-process-address-space)
5. [Stack — cách hoạt động](#5-stack)
6. [Heap — cách hoạt động](#6-heap)
7. [Stack vs Heap — so sánh trực quan](#7-stack-vs-heap)
8. [Bộ nhớ của các kiểu Rust](#8-bộ-nhớ-các-kiểu-rust)
9. [String / &str / Vec — fat pointer](#9-fat-pointer)
10. [Box / Rc / Arc](#10-box-rc-arc)
11. [Alignment & Padding — căn lề bộ nhớ](#11-alignment--padding)
12. [Niche Optimization — Option nén](#12-niche-optimization)
13. [CPU Cache — L1/L2/L3](#13-cpu-cache)
14. [Cache Line & False Sharing](#14-cache-line--false-sharing)
15. [Cache Associativity](#15-cache-associativity)
16. [Atomic & CAS](#16-atomic--cas)
17. [Memory Ordering — Reordering](#17-memory-ordering)
18. [Async State Machine](#18-async-state-machine)
19. [Lock-Free: Treiber Stack](#19-treiber-stack)
20. [ABA Problem](#20-aba-problem)
21. [NUMA](#21-numa)
22. [Allocators — glibc/jemalloc/mimalloc](#22-allocators)
23. [Memory-Mapped Files (mmap)](#23-mmap)

---

## 1. Bức tranh lớn

Khi bạn chạy `cargo run` → một **process** ra đời. Process này có:

```
┌─────────────────────────────────────────────────────────┐
│                       MÁY TÍNH                          │
│                                                         │
│   ┌──────────┐         ┌─────────────────────────────┐  │
│   │   CPU    │◄───────►│         RAM (4-32 GB)       │  │
│   │          │         │                             │  │
│   │  ┌────┐  │         │   ┌───────────────────┐     │  │
│   │  │ L1 │  │         │   │  Process của bạn  │     │  │
│   │  ├────┤  │         │   │  (Stack + Heap)   │     │  │
│   │  │ L2 │  │         │   └───────────────────┘     │  │
│   │  ├────┤  │         │   ┌───────────────────┐     │  │
│   │  │ L3 │  │         │   │  Process khác     │     │  │
│   │  └────┘  │         │   └───────────────────┘     │  │
│   └──────────┘         └─────────────────────────────┘  │
│                                  ▲                      │
│                                  │ (swap khi RAM hết)   │
│                                  ▼                      │
│                        ┌─────────────────────┐          │
│                        │   Disk (SSD/HDD)    │          │
│                        └─────────────────────┘          │
└─────────────────────────────────────────────────────────┘
```

**Ý chính**: CPU **chỉ làm việc** trên thanh ghi (register) bên trong nó. Mọi dữ liệu **phải được kéo từ RAM về CPU**. Càng xa CPU → càng chậm.

---

## 2. Hệ phân cấp bộ nhớ

Tốc độ thực tế khi CPU lấy 1 byte dữ liệu:

```
        TỐC ĐỘ                                DUNG LƯỢNG
        ▲                                      ▲
        │                                      │
 0.5 ns │  ┌──────────────┐ Register           │  vài chục bytes
        │  └──────────────┘                    │
        │                                      │
 1 ns   │  ┌──────────────┐ L1 Cache           │  32 KB
        │  └──────────────┘                    │
        │                                      │
 4 ns   │  ┌──────────────┐ L2 Cache           │  256 KB
        │  └──────────────┘                    │
        │                                      │
 12 ns  │  ┌──────────────┐ L3 Cache           │  8 MB
        │  └──────────────┘                    │
        │                                      │
 100 ns │  ┌──────────────┐ RAM (DRAM)         │  16 GB
        │  └──────────────┘                    │
        │                                      │
 100 µs │  ┌──────────────┐ SSD                │  500 GB
        │  └──────────────┘                    │
        │                                      │
 10 ms  │  ┌──────────────┐ HDD                │  4 TB
        │  └──────────────┘                    │
        ▼                                      ▼

       Càng nhanh ←──── Trade-off ────→ Càng nhiều
```

**Hình dung số học**: nếu L1 = 1 giây, thì:
- L2 = 4 giây
- L3 = 12 giây
- RAM = **100 giây** (~2 phút)
- SSD = **100,000 giây** (~28 giờ)
- HDD = **10,000,000 giây** (~115 ngày)

→ Đây là lý do **cache miss** là kẻ thù của hiệu năng.

---

## 3. Virtual Memory

Mỗi process **NGHĨ** rằng nó sở hữu toàn bộ RAM. Sự thật: OS lừa nó.

```
   Process A NGHĨ                Process B NGHĨ
   ─────────────                 ─────────────
   0x7FFF... ┌───┐               0x7FFF... ┌───┐
             │   │                         │   │
             │ . │                         │ . │
             │ . │                         │ . │
   0x0000... └───┘               0x0000... └───┘
   (toàn bộ RAM của tôi!)        (toàn bộ RAM của tôi!)

                       │
                       │  OS + MMU (Memory Management Unit)
                       ▼  dịch địa chỉ ảo → địa chỉ thật

                  RAM THẬT (vật lý)
                  ┌───────────────────┐
                  │ ░░░░  A page 1    │
                  │ ▓▓▓▓  B page 5    │
                  │ ░░░░  A page 2    │
                  │ ▒▒▒▒  Kernel      │
                  │ ▓▓▓▓  B page 1    │
                  │ ░░░░  A page 3    │
                  │ ▓▓▓▓  B page 2    │
                  └───────────────────┘
```

**Bảng dịch (Page Table)**:

```
Process A:                       Process B:
Virtual    →   Physical          Virtual    →   Physical
0x1000     →   0x8000            0x1000     →   0xA000
0x2000     →   0xC000            0x2000     →   0xB000
0x3000     →   0xE000            0x3000     →   0xD000
```

→ Cùng địa chỉ ảo `0x1000` nhưng trỏ về 2 vùng RAM khác nhau → **cô lập process**.

---

## 4. Process Address Space

Khi process chạy, không gian địa chỉ ảo được chia thành 6 vùng:

```
   Địa chỉ CAO (0xFFFF...FFFF)
   ┌──────────────────────────────────┐
   │       KERNEL SPACE               │  ← OS dùng, user không động được
   │       (Linux: 0xFFFF...8000+)    │
   ├──────────────────────────────────┤
   │           STACK                  │  ← Biến local, tham số hàm
   │           ↓ (mọc xuống)          │     (LIFO — Last In First Out)
   │                                  │
   │           [ trống ]              │
   │                                  │
   │           ↑ (mọc lên)            │
   │           HEAP                   │  ← Box, Vec, String, ...
   ├──────────────────────────────────┤
   │       BSS (uninit globals)       │  ← static MUT chưa init = 0
   ├──────────────────────────────────┤
   │       DATA (init globals)        │  ← static có giá trị ban đầu
   ├──────────────────────────────────┤
   │       TEXT (code)                │  ← Mã máy của chương trình
   ├──────────────────────────────────┤
   │       [reserved / null page]     │  ← 0x0 không thể truy cập
   └──────────────────────────────────┘
   Địa chỉ THẤP (0x0000...0000)
```

**Ví dụ thực tế với code Rust**:

```rust
static MAX: i32 = 100;          // DATA segment
static mut COUNTER: i32 = 0;    // BSS segment

fn main() {                     // TEXT segment (code)
    let x = 42;                 // STACK
    let v = vec![1, 2, 3];      // v (3 word) ở STACK,
                                // [1,2,3] ở HEAP
}
```

**Visualize**:

```
STACK:                    HEAP:
┌─────────────┐           ┌─────┬─────┬─────┐
│ x = 42      │           │  1  │  2  │  3  │
├─────────────┤           └─────┴─────┴─────┘
│ v.ptr ──────┼──────────►       ▲
│ v.len = 3   │                  │
│ v.cap = 3   │  (Vec là 3 word ở stack, data ở heap)
└─────────────┘
```

---

## 5. Stack

Stack giống **chồng đĩa**: chỉ thêm/bớt ở đỉnh.

### Mỗi lần gọi hàm = đẩy 1 "frame" mới lên stack

```rust
fn main() {
    let a = 1;
    foo();
}
fn foo() {
    let b = 2;
    bar();
}
fn bar() {
    let c = 3;
}
```

**Diễn biến**:

```
Bước 1: main() chạy
┌─────────────┐ ← stack pointer (đỉnh stack)
│ a = 1       │
│ main frame  │
└─────────────┘

Bước 2: foo() được gọi → đẩy frame mới lên
┌─────────────┐ ← SP
│ b = 2       │
│ foo frame   │
├─────────────┤
│ a = 1       │
│ main frame  │
└─────────────┘

Bước 3: bar() được gọi
┌─────────────┐ ← SP
│ c = 3       │
│ bar frame   │
├─────────────┤
│ b = 2       │
│ foo frame   │
├─────────────┤
│ a = 1       │
│ main frame  │
└─────────────┘

Bước 4: bar() return → bar frame BIẾN MẤT (chỉ giảm SP)
┌─────────────┐ ← SP
│ b = 2       │
│ foo frame   │
├─────────────┤
│ a = 1       │
│ main frame  │
└─────────────┘
```

**Lý do stack nhanh**: chỉ tăng/giảm 1 thanh ghi (Stack Pointer). Không cần tìm chỗ trống.

---

## 6. Heap

Heap là **vùng nhớ tự do** — bạn xin (`malloc`/`Box::new`), bạn trả (`free`/`drop`).

```
Lúc đầu: heap toàn là bộ nhớ trống
┌─────────────────────────────────────────────┐
│             FREE (1 MB)                     │
└─────────────────────────────────────────────┘

Sau khi: let a = Box::new(100i32);        // 4 bytes
        let b = Box::new([0u8; 16]);      // 16 bytes
        let c = Box::new("hello".to_string());  // 24 byte (header) + 5 byte (data)

┌────┬──────────┬───────────────────────────────┐
│ a  │    b     │ c hdr │     c data     │ FREE │
│4 B │   16 B   │ 24 B  │      5 B       │      │
└────┴──────────┴───────────────────────────────┘
  ▲    ▲          ▲
  │    │          │
  ptr  ptr        ptr  ← các biến trên stack trỏ tới đây

Sau khi drop(b):
┌────┬──────────┬───────────────────────────────┐
│ a  │   FREE   │ c hdr │     c data     │ FREE │
│4 B │   16 B   │ 24 B  │      5 B       │      │
└────┴──────────┴───────────────────────────────┘
       ↑ Lỗ trống — gọi là FRAGMENTATION
```

### Allocator quản lý các "lỗ trống" bằng Free List

```
Free List (danh sách các block trống):

  [16 B] ──► [32 B] ──► [128 B] ──► NULL

Khi xin 20 B:
  - Bỏ qua 16 B (quá nhỏ)
  - Lấy 32 B → cắt 20 B để dùng, trả 12 B vào free list

Sau:
  [16 B] ──► [12 B] ──► [128 B] ──► NULL
```

---

## 7. Stack vs Heap

```
                STACK                              HEAP
        ┌───────────────────┐              ┌───────────────────┐
        │  Tốc độ:    ⚡⚡⚡  │              │  Tốc độ:    🐢      │
        │  Quản lý:   Tự động│             │  Quản lý:   Thủ công│
        │  Kích thước:Cố định│             │  Kích thước:Động   │
        │             (compile)│           │             (runtime)│
        │  Lifetime:  Frame  │              │  Lifetime:  Đến khi│
        │             scope  │              │             free   │
        │  Capacity:  Nhỏ    │              │  Capacity:  Lớn    │
        │             (8 MB) │              │             (vài GB)│
        │  Thread:    Riêng  │              │  Thread:    Chia sẻ│
        │             /thread│              │             (cần sync)│
        └───────────────────┘              └───────────────────┘

  Đặt biến lên STACK khi:                Đặt biến lên HEAP khi:
  - Kích thước biết lúc compile          - Kích thước biết lúc runtime
  - Sống trong 1 scope                   - Sống lâu hơn scope
  - Nhỏ (vài bytes → vài KB)             - Lớn (MB → GB)
  - Sao chép (Copy)                      - Chuyển sở hữu (Move)
```

**Ví dụ ranh giới**:

```rust
let a: [i32; 5] = [1,2,3,4,5];     // STACK (size cố định)
let b: Vec<i32> = vec![1,2,3,4,5]; // Vec header ở STACK, data ở HEAP
let c: Box<i32> = Box::new(42);    // c ở STACK, 42 ở HEAP
let d: i32 = 42;                   // STACK
```

```
STACK:                        HEAP:
┌──────────────┐
│ a: [1,2,3,4,5]│  ← 20 byte ở luôn stack
├──────────────┤
│ b.ptr ───────┼──►┌──────────────┐
│ b.len = 5    │   │ 1, 2, 3, 4, 5│
│ b.cap = 5    │   └──────────────┘
├──────────────┤
│ c ───────────┼──►┌──┐
│              │   │42│
├──────────────┤   └──┘
│ d = 42       │
└──────────────┘
```

---

## 8. Bộ nhớ các kiểu Rust

### 8.1. Kiểu nguyên thủy

```
i8       1 byte    ┌─┐
i16      2 bytes   ┌─┬─┐
i32      4 bytes   ┌─┬─┬─┬─┐
i64      8 bytes   ┌─┬─┬─┬─┬─┬─┬─┬─┐
i128    16 bytes   ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐

f32      4 bytes   ┌─┬─┬─┬─┐  (IEEE 754 single)
f64      8 bytes   ┌─┬─┬─┬─┬─┬─┬─┬─┐  (IEEE 754 double)

bool     1 byte    ┌─┐  (chỉ dùng 1 bit, lãng phí 7 bit)

char     4 bytes   ┌─┬─┬─┬─┐  (Unicode scalar value)

usize    8 bytes (trên 64-bit)
isize    8 bytes (trên 64-bit)
```

### 8.2. Tuple

```rust
let t: (i32, f64, bool) = (1, 2.0, true);
```

```
STACK:
┌───────────────────┐
│ i32: 1            │  4 bytes
├─[padding 4 byte]──┤  ← để f64 căn lề 8
│ f64: 2.0          │  8 bytes
├───────────────────┤
│ bool: true        │  1 byte
├─[padding 7 byte]──┤  ← để toàn tuple căn lề 8
└───────────────────┘
Tổng: 24 bytes (không phải 4+8+1=13!)
```

### 8.3. Array

```rust
let arr: [i32; 5] = [10, 20, 30, 40, 50];
```

```
STACK:
┌───┬───┬───┬───┬───┐
│ 10│ 20│ 30│ 40│ 50│  ← 5 × 4 = 20 bytes liên tiếp
└───┴───┴───┴───┴───┘
  ▲
  └─ arr là pointer ẨN tới đây
```

### 8.4. Struct

```rust
struct Point { x: i32, y: i32 }
let p = Point { x: 10, y: 20 };
```

```
STACK:
┌─────────┬─────────┐
│ x = 10  │ y = 20  │  8 bytes (2 × 4)
└─────────┴─────────┘
```

---

## 9. Fat Pointer

`String`, `&str`, `Vec<T>`, `&[T]` đều là **fat pointer** = pointer + metadata.

### 9.1. String

```rust
let s = String::from("hello");
```

```
STACK (24 bytes — fat pointer):
┌──────────────┐
│ ptr ─────────┼──►  HEAP:
├──────────────┤     ┌───┬───┬───┬───┬───┐
│ len = 5      │     │ h │ e │ l │ l │ o │
├──────────────┤     └───┴───┴───┴───┴───┘
│ cap = 5      │     (UTF-8 bytes)
└──────────────┘
```

- **ptr** (8 byte): trỏ tới byte đầu tiên trên heap
- **len** (8 byte): có 5 byte đang dùng
- **cap** (8 byte): tổng dung lượng đã cấp phát

### 9.2. Vec\<T\>

Giống y hệt String, chỉ là T tổng quát:

```rust
let v: Vec<i32> = vec![1, 2, 3];
```

```
STACK (24 bytes):                HEAP:
┌──────────────┐                 ┌────┬────┬────┐
│ ptr ─────────┼────────────────►│ 1  │ 2  │ 3  │
├──────────────┤                 └────┴────┴────┘
│ len = 3      │                 (3 × 4 = 12 bytes)
├──────────────┤
│ cap = 3      │
└──────────────┘
```

### 9.3. &str

`&str` là **slice** — chỉ ptr + len (không cap, vì không sở hữu).

```rust
let s = String::from("hello world");
let slice: &str = &s[0..5];      // "hello"
```

```
STACK:
┌──────────────┐
│ s.ptr ───────┼─►┐
│ s.len = 11   │  │
│ s.cap = 11   │  │
├──────────────┤  │   HEAP:
│ slice.ptr ───┼─►├─►┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
│ slice.len=5  │  │  │ h │ e │ l │ l │ o │ ' '│ w │ o │ r │ l │ d │
└──────────────┘  └─►└───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
                     ▲           ▲
                     │           │
                     │           └─ slice trỏ vào ĐÂY luôn (cùng heap với s)
                     └─ s.ptr
```

→ `&str` **không** sao chép data, chỉ là "cửa sổ" nhìn vào string khác.

---

## 10. Box, Rc, Arc

### 10.1. Box\<T\> — đơn sở hữu

```rust
let b: Box<i32> = Box::new(42);
```

```
STACK:
┌──────────────┐
│ b: ptr ──────┼──►  HEAP:
└──────────────┘     ┌────┐
                     │ 42 │
                     └────┘
   8 bytes              4 bytes
```

### 10.2. Rc\<T\> — đếm tham chiếu (single-thread)

```rust
let a: Rc<String> = Rc::new(String::from("hi"));
let b = Rc::clone(&a);  // không clone data, chỉ tăng count
let c = Rc::clone(&a);
```

```
STACK:                    HEAP:
┌────────┐                ┌─────────────────────┐
│ a ─────┼───────────────►│ strong: 3           │  ← count
├────────┤        ┌──────►│ weak:   0           │
│ b ─────┼────────┘ ┌────►│ ─────────────────── │
├────────┤          │     │ String header       │
│ c ─────┼──────────┘     │   ptr ──┐           │
└────────┘                │   len   │           │
                          │   cap   │           │
                          └─────────┼───────────┘
                                    ▼
                          ┌──────────┐
                          │ "hi"     │
                          └──────────┘
```

Khi `c` drop → strong = 2. Khi cả 3 drop → strong = 0 → free.

### 10.3. Arc\<T\> — Rc nhưng cho multi-thread

```
STACK (thread 1)         HEAP (shared)              STACK (thread 2)
┌────────┐               ┌──────────────────┐       ┌────────┐
│ arc1 ──┼──────────────►│ strong: 2 (ATOMIC)│◄─────┼── arc2 │
└────────┘               │ weak:   0         │      └────────┘
                         │ data: "..."       │
                         └──────────────────┘
```

Khác biệt với Rc: counter dùng **atomic** (`AtomicUsize`) để tăng/giảm an toàn từ nhiều thread.

→ Arc chậm hơn Rc một chút vì atomic ops, nhưng thread-safe.

---

## 11. Alignment & Padding

CPU đọc bộ nhớ theo "khúc" (8/16/32 bytes). Dữ liệu **phải bắt đầu** ở địa chỉ chia hết cho size của nó.

### Quy tắc: type có size N → phải đặt ở địa chỉ chia hết cho N

```rust
struct Bad {
    a: u8,    // 1 byte
    b: u64,   // 8 bytes
    c: u8,    // 1 byte
}
// Bạn nghĩ: 1+8+1 = 10 bytes?
// Thực tế: 24 bytes!
```

```
Địa chỉ:  0    1               8    9              16     24
          ┌────┬───────────────┬────┬────────────────┬────┐
          │ a  │   PADDING     │ b  │   PADDING      │ c  │
          │1 B │     7 B       │8 B │     7 B        │1 B │
          └────┴───────────────┴────┴────────────────┴────┘
            ▲                    ▲                    ▲
            │                    │                    │
        a tại addr 0          b PHẢI tại            sau c phải pad
        (chia hết 1)          addr chia hết 8       để toàn struct
                              → padding 7 B         căn lề 8 (struct
                                                    size = bội của 8)
```

### Sắp xếp lại field từ LỚN → NHỎ → tiết kiệm

```rust
struct Good {
    b: u64,   // 8 bytes
    a: u8,    // 1 byte
    c: u8,    // 1 byte
}
// Size: 16 bytes (tiết kiệm 8 byte!)
```

```
Địa chỉ:  0              8  9  10        16
          ┌──────────────┬──┬──┬─────────┐
          │      b       │ a│ c│ PADDING │
          │    8 B       │1 │1 │   6 B   │
          └──────────────┴──┴──┴─────────┘
```

### Hoặc dùng `#[repr(packed)]` để bỏ padding (CHẬM hơn!)

```rust
#[repr(packed)]
struct Packed { a: u8, b: u64, c: u8 }  // 10 bytes
```

```
┌──┬──────────────┬──┐
│ a│      b       │ c│  ← b ở addr 1 (LỆCH 8!)
└──┴──────────────┴──┘     → CPU phải đọc 2 lần & ghép → chậm
```

---

## 12. Niche Optimization

Rust phát hiện "giá trị bất hợp lệ" để **nén** `Option`.

### Bài toán: `Option<bool>` cần bao nhiêu byte?

`bool` chỉ dùng 2 giá trị: 0 và 1. Còn 254 giá trị "trống".
→ Rust dùng `2` để biểu diễn `None`!

```
Option<bool>: 1 byte
  Some(false)  =  0x00
  Some(true)   =  0x01
  None         =  0x02   ← niche!
```

### `Option<&T>` cũng vậy

Reference `&T` **không bao giờ** là null (`0x0`). Rust dùng `0x0` để biểu diễn `None`.

```
Option<&i32>:  8 bytes (KHÔNG phải 16!)

Some(&x):  ┌─────────────────┐
           │ 0x7FFF...1234   │  ← địa chỉ thật
           └─────────────────┘

None:      ┌─────────────────┐
           │ 0x0000...0000   │  ← niche (sẽ không xung đột!)
           └─────────────────┘
```

So với C, `Option<int*>` cần thêm 1 bool tag → 16 bytes:

```
C struct {bool has_value; int* ptr;}  →  16 bytes (lãng phí 8 byte!)
Rust Option<&i32>                     →   8 bytes
```

---

## 13. CPU Cache

CPU không đọc thẳng từ RAM. Nó kéo dữ liệu qua **3 cấp cache** trước:

```
CPU CORE 0                       CPU CORE 1
┌────────────────┐               ┌────────────────┐
│  Registers     │               │  Registers     │
│       │        │               │       │        │
│  ┌────▼─────┐  │               │  ┌────▼─────┐  │
│  │   L1     │  │  (32 KB,1 ns) │  │   L1     │  │
│  └────┬─────┘  │               │  └────┬─────┘  │
│       │        │               │       │        │
│  ┌────▼─────┐  │               │  ┌────▼─────┐  │
│  │   L2     │  │  (256 KB,4 ns)│  │   L2     │  │
│  └────┬─────┘  │               │  └────┬─────┘  │
└───────┼────────┘               └───────┼────────┘
        │                                │
        └───────────┬────────────────────┘
                    ▼
            ┌──────────────┐
            │     L3       │  (8 MB, 12 ns)
            │   (shared)   │
            └──────┬───────┘
                   ▼
            ┌──────────────┐
            │     RAM      │  (16 GB, 100 ns)
            └──────────────┘
```

### Khi CPU đọc `arr[0]`

```
CPU: "Cho tôi addr 0x1000"
  │
  ▼
L1: "Tôi không có" (cache MISS)
  │
  ▼
L2: "Tôi không có"
  │
  ▼
L3: "Tôi không có"
  │
  ▼
RAM: "Đây, 64 BYTE bắt đầu từ 0x1000"  ← LUÔN trả 1 CACHE LINE (64 byte)
  │
  ▼
RAM trả về 64 byte → copy vào L3 → L2 → L1 → CPU
                                              ▲
                                              │
                                  Lần sau đọc 0x1001, 0x1002...
                                  → CACHE HIT, không phải đi xuống RAM nữa
```

→ Đây là lý do **iteration tuần tự** nhanh hơn random access nhiều lần.

---

## 14. Cache Line & False Sharing

### Cache line = đơn vị chuyển dữ liệu giữa RAM ↔ Cache (64 byte)

```
RAM:                                              Cache:
0x1000 ┌──┬──┬──┬──┬──┬──┬──┬──┐ ← cache line 1   ┌─────────────┐
       │ a│ b│ c│ d│ e│ f│ g│ h│                   │  line 1     │
0x1008 ├──┼──┼──┼──┼──┼──┼──┼──┤                   │  (64 byte)  │
       │  │  │  │  │  │  │  │  │                   └─────────────┘
       ...                                              ▲
0x1040 ┌──┬──┬──┬──┬──┬──┬──┬──┐ ← cache line 2         │
                                          ◄────────────┘
                                          Đọc 1 byte → kéo TOÀN BỘ 64 byte
```

### False Sharing — bẫy đa luồng

```rust
struct Counters {
    a: AtomicU64,  // Thread 1 cập nhật
    b: AtomicU64,  // Thread 2 cập nhật
}
```

```
Cả a và b nằm trong CÙNG 1 cache line 64 byte:

┌──────────────────────────────────────────────┐
│           Cache Line (64 byte)                │
│  ┌──────────┬──────────┬───────────────────┐  │
│  │ a (8 B)  │ b (8 B)  │ ...padding...     │  │
│  └──────────┴──────────┴───────────────────┘  │
└──────────────────────────────────────────────┘

Thread 1 (core 0):                Thread 2 (core 1):
- Ghi vào a                       - Ghi vào b
- Core 0 lấy cache line về L1     - Core 1 muốn cache line này
- Đánh dấu "modified"             - Báo core 0: "đưa tao!"
                                  - Core 0 phải FLUSH về L3
                                  - Core 1 kéo về
- Lại muốn cập nhật a             - Cập nhật b
- Yêu cầu lại từ core 1...        - Lại flush về...
  → PING-PONG vô tận              → PING-PONG vô tận

Hiệu năng: chậm 10-100x so với khi tách riêng cache line!
```

### Khắc phục: padding để 2 biến ở 2 cache line khác nhau

```rust
struct Counters {
    a: AtomicU64,
    _pad: [u8; 56],  // đẩy b sang cache line khác
    b: AtomicU64,
}
```

```
Cache Line 1:                     Cache Line 2:
┌──────────────────────┐          ┌──────────────────────┐
│ a │   padding 56 B   │          │ b │   ...            │
└──────────────────────┘          └──────────────────────┘
   ▲                                  ▲
   Thread 1 dùng                      Thread 2 dùng
   (không ảnh hưởng đến thread 2)
```

---

## 15. Cache Associativity

Cache là **mảng nhỏ** → không thể chứa địa chỉ tùy ý → cần map.

### Direct-Mapped (1-way) — mỗi addr chỉ có 1 chỗ duy nhất

```
Cache (4 slot):
┌────┬────┬────┬────┐
│ S0 │ S1 │ S2 │ S3 │
└────┴────┴────┴────┘

Addr → slot:  hash = addr % 4

addr 0   → S0
addr 4   → S0   ← CONFLICT với addr 0!
addr 8   → S0   ← CONFLICT!
addr 12  → S0   ← CONFLICT!

Nếu vòng lặp truy cập 0, 4, 0, 4, 0, 4... → liên tục evict → CHẬM
```

### Set-Associative (4-way) — mỗi addr có 4 chỗ chọn

```
Cache (4 set × 4 way):

       Way 0   Way 1   Way 2   Way 3
Set 0  ┌────┬─────┬─────┬─────┐
       │    │     │     │     │  ← 4 slot cho 1 set
Set 1  ├────┼─────┼─────┼─────┤
       │    │     │     │     │
       ...

Addr → set:  hash = (addr / 64) % num_sets

→ Cùng 1 set có thể chứa 4 line khác nhau → ít conflict hơn
```

### Bẫy: Power-of-2 strides

```rust
// 1024 = power of 2 → có thể xung đột set
let big: [[f64; 1024]; 1024] = ...;
for j in 0..1024 {
    sum += big[j][0];  // mỗi truy cập cách nhau 1024*8 = 8192 B
}                       // → cùng 1 set → conflict!
```

**Fix**: pad lên 1025 (số lẻ) → các stride khác nhau → trải đều các set.

---

## 16. Atomic & CAS

### Vấn đề: 2 thread cùng tăng 1 biến

```rust
let mut counter = 0;
// Thread 1: counter += 1;
// Thread 2: counter += 1;
```

Trên CPU, `counter += 1` thực ra là **3 lệnh**:

```
1. LOAD  counter  →  register   (đọc giá trị từ RAM)
2. ADD   register, 1            (cộng 1)
3. STORE register →  counter    (ghi lại vào RAM)
```

### Race condition

```
counter = 0 ban đầu

Thread 1                    Thread 2
─────────                    ─────────
LOAD counter (= 0)
                            LOAD counter (= 0)
ADD 1 (=1)
                            ADD 1 (=1)
STORE counter (= 1)
                            STORE counter (= 1)

Kết quả: counter = 1 (đáng lẽ phải = 2!)
```

### Giải pháp: Atomic — lock cache line trong khi đọc-sửa-ghi

```rust
use std::sync::atomic::{AtomicI64, Ordering};
let counter = AtomicI64::new(0);
counter.fetch_add(1, Ordering::SeqCst);
```

Lệnh CPU: `LOCK XADD` → **atomic** (không thể bị chia cắt)

```
Thread 1                    Thread 2
─────────                    ─────────
🔒 LOCK cache line
LOAD counter (= 0)
ADD 1                       ⏸ Chờ...
STORE counter (= 1)
🔓 UNLOCK
                            🔒 LOCK
                            LOAD counter (= 1)
                            ADD 1
                            STORE counter (= 2)
                            🔓 UNLOCK

Kết quả: counter = 2 ✓
```

### CAS (Compare-And-Swap) — vũ khí của lock-free

`CAS(addr, expected, new)`:
- **Nếu** `*addr == expected` → ghi `new` → trả `true`
- **Nếu không** → không làm gì → trả `false`

```
┌──────────────────────────────────────────────────────────┐
│                  CAS atomically:                          │
│                                                           │
│      if (*addr == expected) {                             │
│          *addr = new;                                     │
│          return true;                                     │
│      } else {                                             │
│          return false;                                    │
│      }                                                    │
│                                                           │
│  Toàn bộ trong 1 cycle, không bị chia cắt!                │
└──────────────────────────────────────────────────────────┘
```

### Pattern: retry loop

```
loop {
    let cur = counter.load();           // 1. Đọc giá trị hiện tại
    let new = cur + 1;                  // 2. Tính giá trị mới
    if counter.cas(cur, new) {          // 3. Thử cập nhật
        break;                          //    Thành công → thoát
    }                                   //    Thất bại → quay lại 1
}
```

```
Thread A:                    Thread B:
cur = 5                      cur = 5
new = 6                      new = 6
CAS(5, 6) ✓                  CAS(5, 6) ✗  (vì counter giờ = 6, không = 5)
                             cur = 6 (load lại)
                             new = 7
                             CAS(6, 7) ✓
```

---

## 17. Memory Ordering

CPU và compiler **tự ý sắp xếp lại** lệnh để tối ưu! Điều này gây bug trong đa luồng.

### Ví dụ bug

```rust
// Thread 1:                       // Thread 2:
data = 42;                         while !ready {}
ready = true;                      println!("{}", data);
```

**Bạn nghĩ**: thread 2 sẽ in ra `42`.

**Thực tế**: CPU có thể đảo `data = 42` và `ready = true`!

```
Thực thi đảo ngược của thread 1:
  ready = true;      ← thread 2 thấy ready=true ngay!
  data  = 42;        ← nhưng data vẫn = 0 lúc đó

→ Thread 2 in ra 0 hoặc rác.
```

### Sơ đồ Memory Ordering

```
┌─────────────────────────────────────────────────────────────┐
│                    Relaxed                                  │
│  - Chỉ atomic, không ràng buộc thứ tự                       │
│  - Nhanh nhất                                               │
│  - Dùng: counter đếm thuần túy                              │
│                                                             │
│  Thread 1: write A (relaxed)   |   Thread 2: read A         │
│           write B (relaxed)    |              read B        │
│                                                             │
│  Có thể thấy: A trước B, B trước A, không A, không B...    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Acquire / Release                        │
│  - Release: tất cả store TRƯỚC nó phải hiện thực hóa        │
│  - Acquire: tất cả load SAU nó thấy được các store đó       │
│  - Cặp đôi: 1 thread Release, 1 thread Acquire             │
│                                                             │
│  Thread 1:                  Thread 2:                       │
│  data = 42;                 while !ready.load(Acquire) {}   │
│  ready.store(true,Release); // ↑ Sau dòng này, data CHẮC=42 │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    SeqCst (mạnh nhất)                       │
│  - Tất cả thread thấy CÙNG 1 thứ tự toàn cục                │
│  - Như có 1 "đồng hồ thế giới"                              │
│  - Chậm nhất                                                │
│  - Dùng khi không chắc → an toàn                            │
└─────────────────────────────────────────────────────────────┘
```

### Visualize Acquire-Release

```
Thread 1:                          Thread 2:

  data = 42;                       
  data2 = 99;                      
       ▼                           
       ▼  ─────────────────►       │
  ready.store(true, Release);      │ (Acquire BLOCKS đến khi
                                   │  ready = true)
                                   ▼
                                   while !ready.load(Acquire) {}
                                                ◄─────────────
                                   // Bây giờ data = 42 GUARANTEED
                                   // và data2 = 99 GUARANTEED
                                   println!("{} {}", data, data2);

      Mọi write TRƯỚC Release  ═════►  Mọi read SAU Acquire
```

---

## 18. Async State Machine

`async fn` được compiler biến thành **enum state machine**.

```rust
async fn task() {
    let a = step1().await;
    let b = step2(a).await;
    println!("{}", b);
}
```

Compiler tạo:

```rust
enum TaskState {
    Start,
    AfterStep1 { a: i32, fut: Step2Future },
    Done,
}
```

### Diễn biến

```
Bước 1: tạo Future
┌──────────────────────────┐
│ TaskState::Start         │
└──────────────────────────┘

Bước 2: executor poll lần 1
  → bắt đầu step1()
  → step1 chưa xong (vd: chờ IO)
  → state machine LƯU LẠI vị trí:

┌──────────────────────────┐
│ TaskState::AfterStep1 {  │
│   a: <pending>           │
│   fut: <step1 future>    │
│ }                        │
└──────────────────────────┘

Bước 3: IO xong → executor poll lần 2
  → resume từ AfterStep1
  → lấy a, gọi step2(a)
  → step2 cũng chưa xong → lưu vị trí
  → ...

Bước cuối:
┌──────────────────────────┐
│ TaskState::Done          │
└──────────────────────────┘
```

### Stack-less coroutine — lý do async Rust khác Go

```
GOROUTINE (Go):                    ASYNC TASK (Rust):
┌────────────────┐                 ┌────────────────┐
│ Stack riêng    │                 │ Enum state     │
│ 8 KB → 1 MB    │                 │ ~vài chục byte │
│ (mọc động)     │                 │ (cố định)      │
└────────────────┘                 └────────────────┘
  Tốn RAM lớn                       Tiết kiệm RAM cực kỳ
  Switch tốn                        Switch = pop enum
  Stack copy                        Không stack copy
```

→ 1 triệu task async Rust = vài chục MB. 1 triệu goroutine = vài GB.

### Pin — neo state machine

Một số future có **con trỏ tự tham chiếu** (trỏ vào field khác của chính nó):

```
self_ref future:
┌──────────────────────────┐
│ data: "hello"            │ ◄──┐
│ ptr: ─────────────────────────┘ (trỏ vào data!)
└──────────────────────────┘
```

Nếu future **bị di chuyển** sang địa chỉ khác → `ptr` trỏ vào địa chỉ cũ → DANGLING!

→ `Pin<&mut T>` cấm di chuyển → đảm bảo `ptr` luôn hợp lệ.

---

## 19. Treiber Stack

Stack lock-free dùng CAS — không cần Mutex.

### Cấu trúc

```
head ──► [val=3] ──► [val=2] ──► [val=1] ──► null
         (Node A)    (Node B)    (Node C)
```

### Push 4

```
Bước 1: Tạo node mới, next = head hiện tại
┌──────────┐
│ val = 4  │
│ next ────┼──► (trỏ vào head cũ = A)
└──────────┘
new_node

head ──► [3] ──► [2] ──► [1] ──► null
  ▲
  │
new_node.next

Bước 2: CAS(head, A, new_node)
  - Nếu head VẪN = A → đổi head thành new_node
  - Nếu head đã thay đổi → RETRY

Sau khi CAS thành công:
head ──► [4] ──► [3] ──► [2] ──► [1] ──► null
```

### Pop

```
Bước 1: đọc head = A
Bước 2: lấy A.next = B
Bước 3: CAS(head, A, B)
  - Thành công → trả về A
  - Thất bại → retry

Trước:
head ──► [3] ──► [2] ──► [1] ──► null
         (A)

Sau:
head ──► [2] ──► [1] ──► null
         (B)
A bị "tách ra" → free
```

---

## 20. ABA Problem

Cái bẫy cổ điển khi dùng CAS với pointer.

### Kịch bản

```
Trạng thái ban đầu:
head ──► [A] ──► [B] ──► null


Thread 1: muốn pop
  - Đọc head = ptr_A
  - Đọc A.next = ptr_B
  - Sắp CAS(head, ptr_A, ptr_B)... ZZZ (bị OS cướp CPU)
  

Trong lúc đó, Thread 2 cực kỳ tích cực:
  - Pop A:    head → B
  - Pop B:    head → null
  - Free A
  - malloc 1 node mới → TÌNH CỜ allocator trả lại địa chỉ của A
  - Đặt val = X, push lên: head → A (cùng địa chỉ) → null


Trạng thái sau:
head ──► [A'] ──► null
         (cùng địa chỉ A, nhưng KHÁC dữ liệu!)


Thread 1 thức dậy:
  - CAS(head, ptr_A, ptr_B)
  - head VẪN = ptr_A → CAS THÀNH CÔNG! (sai lầm!)
  - head ──► ptr_B → nhưng ptr_B đã FREE rồi!
  - DANGLING POINTER → crash hoặc lỗi nghiêm trọng
```

```
Tóm tắt vấn đề:

  Thời điểm T1   T2          T3 (thread 1 wake)
  head = A      head = A     head = A
  thread 1      thread 2     thread 1
  đọc A         pop A, B,    CAS thấy A
  ngủ thiếp     malloc về A  THÀNH CÔNG (sai!)
                
       ▲             ▲              ▲
       │             │              │
   Đọc giá trị   Giá trị đổi    "Giá trị không
   A             A→B→A          đổi!" (sai!)
                 (nhưng A
                  khác A ban đầu)
```

### Giải pháp 1: Tagged Pointer (counter)

```
Thay vì lưu chỉ pointer, lưu (pointer, tag):

  head = (ptr_A, version = 5)


Mỗi lần thay đổi, tag++:
  Pop A:  head = (ptr_B, 6)
  Pop B:  head = (null,  7)
  Push A: head = (ptr_A, 8)


Thread 1 CAS với (ptr_A, 5):
  head hiện tại = (ptr_A, 8)
  CAS THẤT BẠI vì tag khác → an toàn!
```

### Giải pháp 2: Hazard Pointer / Epoch (crossbeam)

```
Trước khi đọc, thread 1 "đăng ký" rằng nó đang dùng A:

  Hazard Pointer Table:
  ┌──────────────────────────┐
  │ Thread 1 đang giữ: ptr_A │ ← đăng ký
  │ Thread 2 đang giữ: -     │
  └──────────────────────────┘

Khi thread 2 muốn free A:
  - Kiểm tra HP table
  - "Thread 1 đang giữ A → KHÔNG free, đợi đã"
  - Đặt vào "retired list", free sau
```

---

## 21. NUMA

Trên server lớn, có nhiều **socket** CPU, mỗi socket có RAM riêng.

```
┌─────────────────────────────────────────────────────────────┐
│                       Mainboard                              │
│                                                              │
│  ┌──────────────────┐         ┌──────────────────┐          │
│  │   Socket 0       │◄───────►│   Socket 1       │          │
│  │  ┌────┐┌────┐    │  QPI    │  ┌────┐┌────┐    │          │
│  │  │Core││Core│    │  Link   │  │Core││Core│    │          │
│  │  │ 0  ││ 1  │    │ (chậm)  │  │ 2  ││ 3  │    │          │
│  │  └────┘└────┘    │         │  └────┘└────┘    │          │
│  │       │           │         │       │          │          │
│  │       ▼           │         │       ▼          │          │
│  │  ┌──────────┐    │         │  ┌──────────┐   │          │
│  │  │ Local    │    │         │  │ Local    │   │          │
│  │  │ RAM 0    │    │         │  │ RAM 1    │   │          │
│  │  │ (60ns)   │    │         │  │ (60ns)   │   │          │
│  │  └──────────┘    │         │  └──────────┘   │          │
│  └──────────────────┘         └──────────────────┘          │
└─────────────────────────────────────────────────────────────┘

Truy cập local:    Core 0 → RAM 0  =  60 ns   ⚡
Truy cập remote:   Core 0 → RAM 1  = 200 ns   🐢 (qua QPI)
```

### Bẫy: thread chạy ở Socket 0 nhưng data ở Socket 1

```
Thread chạy core 0 → đọc data → đi qua QPI → Socket 1 → trả về
                                              ▲
                                       Chậm hơn 3-4 lần!
```

### Fix: NUMA-aware allocation

```rust
// numactl --cpunodebind=0 --membind=0 ./my_program
// → Thread & data đều ở Socket 0
```

```
First-touch policy:
  - malloc 1 GB → chưa cấp RAM thật
  - Thread X (chạy Socket 1) ghi vào → Linux cấp RAM ở Socket 1
  - Thread X đọc rất nhanh

  → "Người đầu tiên chạm vào memory" quyết định nó nằm ở socket nào
```

---

## 22. Allocators

### glibc malloc (mặc định Linux)

```
Mỗi thread có "arena" riêng (giảm contention):

Thread 1 ──► Arena 1 ──► [bin 16] [bin 32] [bin 64]...
Thread 2 ──► Arena 2 ──► [bin 16] [bin 32] [bin 64]...
Thread 3 ──► Arena 3 ──► [bin 16] [bin 32] [bin 64]...

Vấn đề: fragmentation tệ với pattern alloc/free phức tạp
```

### jemalloc (Facebook, Rust trước đây)

```
Hierarchy 3 tầng:

  Thread Cache (tcache)         ← Cấp 1: per-thread, lock-free
       │
       ▼
  Arena                         ← Cấp 2: per-CPU, ít contention
       │
       ▼
  Global Pool                   ← Cấp 3: shared, có lock

Mỗi cấp có bins theo size class:
  bin 8B, 16B, 24B, 32B, 48B, 64B, 80B, 96B, 112B, 128B...
  (~150 size classes)

→ Phân mảnh thấp, tốt cho server chạy lâu dài
```

### mimalloc (Microsoft)

```
Free list theo "page" 64 KB:

┌──────────────── Page (64 KB) ────────────────┐
│ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐     │
│ │16 B │ │16 B │ │16 B │ │16 B │ │ FREE│ ... │
│ └─────┘ └─────┘ └─────┘ └─────┘ └──┬──┘     │
│                                     │        │
│                                     ▼        │
│                                  free_list   │
└──────────────────────────────────────────────┘

Mỗi page chỉ 1 size class → alloc cực nhanh
Khoảng 7-25% nhanh hơn jemalloc trong nhiều workload
```

### So sánh ngắn gọn

```
                glibc      jemalloc    mimalloc    tcmalloc
                ─────      ────────    ────────    ────────
Mặc định:       ✓          -           -           -
Server lâu dài: ⚠ frag     ✓✓✓         ✓✓          ✓✓
Tốc độ:         ★          ★★          ★★★         ★★★
Multi-thread:   ★★         ★★★         ★★★         ★★★
Memory low:     ★          ★★          ★★★         ★★
Profiling:      -          ✓ (hữu ích) -           -

Khuyến nghị cho Big Data:
  - Server 24/7      → jemalloc
  - Tốc độ cao nhất  → mimalloc
  - Google ecosystem → tcmalloc
```

---

## 23. mmap

Memory-Mapped Files — ánh xạ file vào không gian địa chỉ ảo, **không** copy vào RAM.

### Đọc file thông thường (copy 2 lần!)

```
File (disk) ──► Kernel buffer (RAM) ──► User buffer (RAM)
                       ▲                       ▲
                       │                       │
                read(): copy 1            copy 2
```

### mmap (chỉ "trỏ" tới file)

```
   Process Virtual Address Space            DISK
   ┌──────────────────────────┐            ┌─────────────┐
   │     ...                  │            │             │
   │  HEAP                    │            │  huge_file  │
   │  ────                    │            │  (1 TB)     │
   │  ┌──────────────────┐    │            │             │
   │  │ MMAP REGION      │◄───┼────────────┤             │
   │  │ (1 TB virtual)   │    │  on-demand │             │
   │  └──────────────────┘    │   paging   │             │
   │  STACK                   │            │             │
   └──────────────────────────┘            └─────────────┘

   Process thấy: 1 TB byte trong RAM
   Sự thật:      OS chỉ kéo PAGE (4 KB) cần thiết về RAM khi truy cập
```

### Demand paging

```
Bước 1: mmap file 1 TB
   → Không tốn 1 byte RAM nào! Chỉ tạo entries trong page table.

Bước 2: Truy cập byte tại offset 500_000_000_000
   → Page fault!
   → OS đọc 4 KB từ disk vào RAM
   → Map vào virtual address
   → Code tiếp tục

Bước 3: Truy cập byte gần đó (offset 500_000_000_100)
   → Cùng page 4 KB → đã có trong RAM → siêu nhanh

Bước 4: Memory đầy
   → OS evict những page không dùng → ghi về disk (nếu dirty)
```

### Khi nào dùng mmap?

```
DATA > RAM (vd: 1 TB file, RAM 32 GB):  ✓ mmap rất tốt
RANDOM ACCESS pattern:                  ✓ mmap tốt
SHARED giữa nhiều process:              ✓ mmap tốt (cùng physical page)
SEQUENTIAL scan 1 lần:                  ⚠ read() có khi tốt hơn
WRITE nhiều:                            ⚠ cẩn thận msync/page dirty
```

---

## Tổng kết — Bản đồ tư duy

```
                     MEMORY MODEL
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
     LAYOUT           HIỆU NĂNG         ĐỒNG THỜI
        │                 │                 │
    ┌───┴───┐         ┌───┴───┐         ┌───┴───┐
    ▼       ▼         ▼       ▼         ▼       ▼
  Stack   Heap      Cache  Alignment  Atomic   Lock-free
   │       │         │        │         │         │
   ▼       ▼         ▼        ▼         ▼         ▼
 Frame  Allocator  L1/L2/L3 Padding  Ordering  Treiber
        Free List  Line                         ABA
        Bins       Assoc                        NUMA
                   False
                   Sharing

       ASYNC                  BIG DATA
         │                       │
     ┌───┴───┐               ┌───┴───┐
     ▼       ▼               ▼       ▼
  State    Pin             jemalloc  mmap
  machine                  mimalloc  Arena
                                     Zero-copy
```

---

## Kết: Hãy nhớ 4 nguyên lý

```
1. CPU LƯỜI → cache, prefetch, reorder
   → Code thân thiện cache = code nhanh

2. RAM CHẬM → mọi tối ưu xoay quanh "đừng đi xuống RAM"
   → Sequential access > Random access

3. THREAD CỘNG TÁC RỦI RO → atomic, ordering, false sharing
   → Càng chia sẻ ít càng nhanh

4. BỘ NHỚ HỮU HẠN → allocator, mmap, arena
   → Biết khi nào cần đổi chiến lược
```

> Mỗi hình vẽ ở đây phản ánh 1 trade-off. Không có "đúng tuyệt đối" — chỉ có "phù hợp với bài toán của bạn".

---

*File companion cho `memory-model.md`. Đọc 2 file song song để vừa hiểu lý thuyết vừa thấy trực quan.*
