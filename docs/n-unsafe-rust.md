# Unsafe Rust — Deep Dive

> Tài liệu thứ 14 trong bộ Rust nền tảng. Đọc trước:
> - [memory-model.md](./memory-model.md) — alignment, padding
> - [ownership-borrowing.md](./ownership-borrowing.md) — borrow rules
> - [smart-pointers.md](./smart-pointers.md) — UnsafeCell foundation
> - [lifetime.md](./lifetime.md) — lifetime + variance
> - [performance.md](./performance.md) — khi nào unsafe có ý nghĩa
>
> **Unsafe Rust** là một ngôn ngữ con (subset) cho phép bạn làm những việc compiler không
> verify được an toàn. Đây là **cửa thoát hiểm** cho:
> - FFI với C/C++
> - Tạo abstraction mới (Vec, Mutex, RefCell đều dùng unsafe nội bộ)
> - Tối ưu hot path (sau khi đo profile)
> - Hardware access, embedded
>
> **Nhưng**: 1 dòng unsafe sai = Undefined Behavior = corrupted memory, security vulnerability,
> crash tùy lúc. Tài liệu này dạy bạn dùng unsafe như senior — **safely wrap unsafe**.
>
> ⚠️ "Unsafe code is contagious — but contained." Wrap unsafe trong safe API.

---

# Mục lục

- [Tầng 1: Unsafe là gì, vì sao tồn tại?](#tầng-1-unsafe-là-gì-vì-sao-tồn-tại)
- [Tầng 2: 5 Unsafe Superpowers](#tầng-2-5-unsafe-superpowers)
- [Tầng 3: Raw Pointers — `*const T`, `*mut T`](#tầng-3-raw-pointers--const-t-mut-t)
- [Tầng 4: Undefined Behavior — Danh sách đáng sợ](#tầng-4-undefined-behavior--danh-sách-đáng-sợ)
- [Tầng 5: Aliasing rules — Stacked Borrows / Tree Borrows](#tầng-5-aliasing-rules--stacked-borrows--tree-borrows)
- [Tầng 6: UnsafeCell — Foundation của interior mutability](#tầng-6-unsafecell--foundation-của-interior-mutability)
- [Tầng 7: Atomic Ordering — Memory model deep](#tầng-7-atomic-ordering--memory-model-deep)
- [Tầng 8: Send và Sync — unsafe impl](#tầng-8-send-và-sync--unsafe-impl)
- [Tầng 9: Memory layout — `#[repr(...)]` và alignment](#tầng-9-memory-layout--repr-và-alignment)
- [Tầng 10: FFI — Gọi C / Được gọi từ C](#tầng-10-ffi--gọi-c--được-gọi-từ-c)
- [Tầng 11: MaybeUninit và uninitialized memory](#tầng-11-maybeuninit-và-uninitialized-memory)
- [Tầng 12: mem::transmute và dynamic dispatch hack](#tầng-12-memtransmute-và-dynamic-dispatch-hack)
- [Tầng 13: Safe abstraction patterns](#tầng-13-safe-abstraction-patterns)
- [Tầng 14: Tools — miri, sanitizers, cargo-careful](#tầng-14-tools--miri-sanitizers-cargo-careful)
- [Tầng 15: Antipatterns và soundness bugs](#tầng-15-antipatterns-và-soundness-bugs)

---

# Tầng 1: Unsafe là gì, vì sao tồn tại?

## 1.1 Định nghĩa

```rust
unsafe {
    // Có thể làm 5 việc mà safe code không làm được
}

unsafe fn risky() {
    // Function đánh dấu unsafe — caller phải `unsafe {}` khi gọi
}

unsafe trait Marker {}    // Marker trait đặc biệt (Send, Sync)

unsafe impl Send for MyType {}   // Implement unsafe trait
```

`unsafe` không tắt **borrow checker**. Nó cho phép 5 hành động extra (xem Tầng 2).

## 1.2 Tại sao Rust cần unsafe?

Rust safety model dựa trên:
- **Ownership**: 1 owner mỗi giá trị
- **Borrowing**: 1 mut HOẶC N shared, không lẫn
- **Lifetimes**: reference không sống lâu hơn data

3 quy tắc này **rất mạnh** — nhưng quá rigid cho:

### Không thể với safe Rust:

1. **Double linked list** — node có 2 owner (prev + next)
2. **Self-referential** — struct chứa ref vào field của chính nó
3. **FFI** — call vào C, không có lifetime info
4. **Hardware access** — memory-mapped I/O không phải owned memory
5. **Concurrent data structures lock-free** — cần atomic raw access
6. **Performance trick** — đôi khi compiler bounds check không elide được

Đây là **chính những** vấn đề `unsafe` giải quyết.

## 1.3 Triết lý senior

```
┌────────────────────────────────────────────────────────────┐
│ "Unsafe is contagious, but contained."                     │
│                                                            │
│ • Unsafe code có thể violate invariants ANYWHERE          │
│ • Nhưng dùng đúng → wrap thành SAFE API                   │
│ • Caller của safe API không bao giờ thấy unsafe           │
└────────────────────────────────────────────────────────────┘

Ví dụ:
─────
Vec<T> nội bộ dùng raw pointer + unsafe. Nhưng API public 100% safe:
  v.push(x)      ← safe, không lỗi memory
  v[0]            ← safe, panic nếu out of bounds (KHÔNG UB)
  v.iter()        ← safe

Người dùng KHÔNG cần biết Vec dùng unsafe internally.
```

## 1.4 Khi nào dùng unsafe?

✅ **Có lý do**:
- FFI với C/C++ library (không có Rust equivalent)
- Build foundational abstraction (Vec, Mutex, RefCell, channels)
- Embedded: hardware register access
- Performance critical, đã profile, đã try safe alternatives
- Bypass bounds check sau khi prove safety

❌ **Không lý do**:
- "Tôi nghĩ nó sẽ nhanh hơn" (chưa profile)
- "Để tránh borrow checker complaint" (restructure code)
- "Để skip bounds check" (compiler thường elide rồi)
- "Vì C/C++ làm thế" (Rust idiom khác)

## 1.5 Cost của unsafe

```
   Risk: high  ←─── 1 bug = UB, security vulnerability, crash
   
   Audit cost: high — mỗi unsafe block cần human verification
   
   Compile cost: zero — unsafe không add overhead
   
   Runtime cost: zero — same machine code as safe equivalent
```

→ Pay **audit cost** để get raw access. Trade-off rất rõ.

---

# Tầng 2: 5 Unsafe Superpowers

Chỉ 5 việc bạn có thể làm trong `unsafe {}` mà safe Rust không cho:

## 2.1 Dereference raw pointer

```rust
let x = 5;
let p: *const i32 = &x;

let val = unsafe { *p };   // Dereference raw pointer
```

Safe code chỉ deref `&T` hoặc `&mut T`. Raw pointer `*const T` / `*mut T` cần unsafe.

## 2.2 Call unsafe function

```rust
unsafe fn dangerous() {}

unsafe {
    dangerous();   // Call unsafe fn
}
```

Functions declared `unsafe fn` → caller phải `unsafe {}`.

Examples: `Vec::set_len`, `String::from_utf8_unchecked`, `mem::transmute`, FFI functions.

## 2.3 Access / Modify mutable static

```rust
static mut COUNTER: u32 = 0;

unsafe {
    COUNTER += 1;
    println!("{}", COUNTER);
}
```

Mutable static = data race nếu multi-thread. Safe code không cho.

(Avoid: use `AtomicU32` instead.)

## 2.4 Implement unsafe trait

```rust
unsafe trait MyMarker {}

unsafe impl MyMarker for MyType {}
```

`unsafe trait` = "implementor phải uphold invariants". Examples: `Send`, `Sync`, `GlobalAlloc`.

## 2.5 Access fields of `union`

```rust
union MyUnion {
    f1: u32,
    f2: f32,
}

let u = MyUnion { f1: 1 };
let v = unsafe { u.f1 };   // Access union field
```

Union = C-style union, no Rust tag. Unsafe vì compiler không track active variant.

## 2.6 Borrow checker VẪN HOẠT ĐỘNG

```rust
unsafe {
    let mut x = 5;
    let r1 = &mut x;
    let r2 = &mut x;  // ❌ STILL ERROR even in unsafe block
}
```

`unsafe` không tắt borrow checker. Chỉ enable 5 superpowers ở trên.

---

# Tầng 3: Raw Pointers — `*const T`, `*mut T`

## 3.1 2 loại raw pointer

```rust
let x = 5;
let p: *const i32 = &x;       // immutable raw pointer
let p_mut: *mut i32 = &x as *const i32 as *mut i32;   // mutable
```

Khác `&T` / `&mut T`:
- **Không lifetime** — không track
- **Có thể null** (vs reference always valid)
- **Có thể overlap** — 2 `*mut T` cùng địa chỉ OK
- **Tạo Safe** — chỉ deref cần unsafe
- **Không drop check** — bạn quản lý lifecycle

## 3.2 Tạo raw pointer

```rust
// From reference
let x = 5;
let p: *const i32 = &x;
let p: *const i32 = &x as *const i32;

let mut y = 10;
let p_mut: *mut i32 = &mut y;
let p_mut: *mut i32 = &mut y as *mut i32;

// From Box
let b = Box::new(42);
let p = Box::into_raw(b);   // *mut i32, ownership moved out
// Must call Box::from_raw(p) later to free!

// Null
let p: *const i32 = std::ptr::null();
let p_mut: *mut i32 = std::ptr::null_mut();

// From integer (very rare)
let addr: usize = 0xdeadbeef;
let p = addr as *const i32;   // dangerous!
```

## 3.3 Dereference

```rust
let x = 5;
let p: *const i32 = &x;

unsafe {
    let val = *p;       // Read
    println!("{}", val);
}

let mut y = 10;
let p_mut: *mut i32 = &mut y;

unsafe {
    *p_mut = 20;        // Write
}
```

**Lưu ý**: deref unsafe vì:
- Pointer có thể dangling (point to freed memory)
- Pointer có thể null
- Pointer có thể misaligned
- Lifetime của target chưa verify

## 3.4 Pointer arithmetic

```rust
let arr = [1, 2, 3, 4, 5];
let p: *const i32 = arr.as_ptr();

unsafe {
    let second = *p.add(1);       // pointer offset bằng size_of::<T>
    let last = *p.add(4);          // arr[4]
    
    // .offset(n) — i32 (signed)
    let neg = p.add(2).offset(-1);  // arr[1]
}
```

`add(n)`: pointer += n * sizeof(T). Tự lo offset bằng size.

**UB nếu**:
- Pointer out of allocation
- Pointer overflows isize range
- Resulting pointer out of allocation (kể cả không deref)

## 3.5 Read/Write functions

```rust
use std::ptr;

let mut x = 5;
let p = &mut x as *mut i32;

unsafe {
    ptr::write(p, 100);        // *p = 100
    let v = ptr::read(p);      // v = *p (copy)
    
    // Unaligned versions:
    ptr::read_unaligned(p);
    ptr::write_unaligned(p, 200);
    
    // Volatile (for MMIO):
    ptr::read_volatile(p);
    ptr::write_volatile(p, 300);
}
```

`ptr::read` không call destructor of old value. Bạn lo cleanup.

## 3.6 Comparison

```rust
let p1: *const i32 = &5;
let p2: *const i32 = &5;

if p1 == p2 { ... }     // Compare addresses (not values!)
if p1.is_null() { ... }
```

`==` compare địa chỉ. Để compare value: `unsafe { *p1 == *p2 }`.

## 3.7 Cast giữa các loại pointer

```rust
let p: *const u8 = ...;
let p_i32: *const i32 = p as *const i32;       // type cast
let p_mut: *mut u8 = p as *mut u8;             // const → mut
let p_void: *const () = p as *const ();        // erase type

// Reference → pointer:
let r: &i32 = &5;
let p: *const i32 = r;                          // implicit coerce
let p: *const i32 = r as *const i32;            // explicit
```

Pointer cast là zero-cost — chỉ thay đổi type system view.

## 3.8 Pointer vs Box vs Reference — Comparison

| Type | Lifetime track | Ownership | Drop on go-of-scope | Can be null |
|------|----------------|-----------|---------------------|-------------|
| `&T` | ✅ | ❌ | N/A | ❌ |
| `&mut T` | ✅ | ❌ | N/A | ❌ |
| `Box<T>` | N/A | ✅ | ✅ | ❌ |
| `*const T` | ❌ | ❌ | ❌ | ✅ |
| `*mut T` | ❌ | ❌ | ❌ | ✅ |

→ Raw pointer = "C-style pointer". You manage everything.

---

# Tầng 4: Undefined Behavior — Danh sách đáng sợ

## 4.1 UB là gì?

**Undefined Behavior (UB)** = code Rust "không quy định". Compiler có thể:
- Skip checks assuming UB không xảy ra
- Optimize unexpectedly
- Generate code crash, corrupt memory, leak secrets

**Quan trọng**: UB không phải "crash". UB có thể "work fine" trên test, fail tại production. Đây là security nightmare.

## 4.2 Danh sách UB của Rust

Từ Rust Reference / Rustonomicon:

### Memory access
- Dereference dangling pointer (point to freed memory)
- Dereference null pointer
- Dereference misaligned pointer (e.g. `*const u64` at odd address)
- Read uninitialized memory (except `MaybeUninit`)
- Out-of-bounds pointer arithmetic / access

### Aliasing
- 2 `&mut T` to same data (aliasing mutable borrow)
- `&T` and `&mut T` to same data
- Mutate qua `&T` (must go through `UnsafeCell`)

### Type invariants
- Read invalid bit pattern (e.g. `false`/`true` from `bool` other than 0/1)
- Wrong layout for FFI
- Cast pointer to wrong type then deref
- Read `enum` with invalid discriminant
- Send `!Send` type to other thread
- Share `!Sync` type via reference across threads

### Concurrency
- Data race (concurrent unsynchronized access where ≥1 is write)
- Wrong atomic ordering causing race

### Other
- Call function with wrong signature (FFI mismatch)
- Producing invalid `Box`, `Vec`, `String` (e.g. invalid UTF-8 in String)

## 4.3 UB ≠ Crash

```rust
unsafe fn read_invalid_bool() -> bool {
    let x: u8 = 2;
    std::mem::transmute(x)   // bool with bit pattern 2 — UB
}

let b = unsafe { read_invalid_bool() };
if b { /* ... */ }
// Compiler có thể:
// 1. Run "if" branch (treating 2 as truthy)
// 2. Skip "if" (treating 2 as falsy)
// 3. Generate code that crashes
// 4. Generate code that "works" sometimes, crashes others
```

UB doesn't mean immediate crash. Means **anything**.

## 4.4 UB cascade

Once UB happens, **all subsequent behavior** is UB. Compiler đã giả định no UB → optimizations relying on assumption now invalid.

```rust
unsafe { read_uninit(); }   // UB here
let x = 5;
println!("{}", x);          // This line is ALSO UB (technically)
```

→ Tránh **bất cứ giá nào**.

## 4.5 Niche: niche optimization

Rust optimize layout assuming type invariants. Vd `Option<&T>` = `*const T` với null = `None`. Nếu tạo invalid `&T` (e.g. via transmute) → break niche → corrupt `Option`.

→ Cẩn thận với transmute và type casting.

## 4.6 Spec status

Rust **không có** complete formal spec for UB (as of 2025). UB rules đang phát triển:
- Stacked Borrows (current model)
- Tree Borrows (newer, more permissive)
- Polonius (next-gen borrow checker)

→ Code unsafe của bạn có thể "work today, UB tomorrow" if rules change.

Để safe: dùng **miri** để check (Tầng 14).

---

# Tầng 5: Aliasing rules — Stacked Borrows / Tree Borrows

## 5.1 Aliasing là gì?

**Aliasing** = nhiều pointer/reference cùng trỏ vào memory.

Rust nghiêm: at any time, 1 mutable OR N immutable borrow.

Unsafe có thể vi phạm → cần aliasing rules để verify.

## 5.2 Stacked Borrows (current model)

Model formal cho borrow rules. Mỗi memory location có **stack of tags**:

```
Memory address 0x1000:
  Stack: [SharedReadOnly, Unique, ...]
```

Mỗi pointer/reference có 1 tag. Access memory require:
- Tag exist trên stack
- Trên cùng stack (top)
- Compatible với access type (read/write)

### Example flow

```rust
let mut x = 5;
let r1 = &mut x;       // Push Unique tag
let r2 = &mut *r1;     // Push Unique tag (sub-borrow)
*r2 = 10;              // OK: r2 tag on top
*r1 = 20;              // r2 tag popped (no longer needed by *r1)
```

Vi phạm Stacked Borrows = UB.

## 5.3 Two-phase borrows (NLL extension)

```rust
let mut v = vec![1, 2, 3];
v.push(v.len());
// = v.push(v.len())
// 1. Reserve &mut v for push (but not active yet)
// 2. Borrow &v for len() — OK because reservation not active
// 3. After len() returns, activate &mut v for push
```

Đặc biệt cho method call ergonomics.

## 5.4 Tree Borrows (proposed, more permissive)

Tree Borrows ([wip](https://perso.crans.org/vanille/treebor/)) thay stack bằng tree. More patterns OK:

```rust
let mut v = vec![1, 2, 3];
let p = v.as_mut_ptr();
unsafe {
    let r = &mut *p;
    // ... use r ...
    // ... use p ...   // OK with Tree Borrows, sometimes UB with Stacked
}
```

Đang được thử nghiệm. Có thể thay Stacked Borrows trong tương lai.

## 5.5 Quy tắc thực dụng

Đến khi spec stable, tránh:
- Tạo `&mut T` và `*mut T` từ cùng source, dùng cả 2 alternately
- Convert reference qua pointer rồi rebuild reference
- Borrow nested với lifetime overlap

Best practice:
```rust
// ✅ Lấy pointer once, không reborrow reference
let p: *mut T = &mut x as *mut T;
unsafe {
    *p = ...;   // dùng p
}

// ❌ Mix reference và raw pointer
let r = &mut x;
let p = r as *mut T;
unsafe { *p = 5; }
*r = 10;   // alias *p và *r → potentially UB
```

## 5.6 miri sẽ catch

miri (Rust interpreter) runtime check Stacked Borrows. Run code through miri để detect aliasing violation.

```bash
cargo +nightly miri test
```

Slow nhưng catch nhiều UB. Recommended cho mọi unsafe code.

---

# Tầng 6: UnsafeCell — Foundation của interior mutability

## 6.1 Vấn đề

```rust
let x: &i32 = &5;
// Có cách nào mutate *x không? In safe Rust: KHÔNG.
```

Safe Rust: `&T` = không mutate. Borrow checker enforce.

Nhưng `Cell`, `RefCell`, `Mutex` cho phép mutate qua `&self`. Họ làm sao?

→ Tất cả dựa trên `UnsafeCell<T>`.

## 6.2 UnsafeCell definition

```rust
#[repr(transparent)]
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

impl<T: ?Sized> UnsafeCell<T> {
    pub fn get(&self) -> *mut T {
        // SAFETY: cast &self to *mut T is OK ONLY through UnsafeCell
        self as *const _ as *mut _
    }
}
```

`UnsafeCell::get(&self)` returns `*mut T` from `&self`. **Đây là** điểm duy nhất trong Rust khi `&T → *mut T → write` được allowed (qua type system magic).

`#[repr(transparent)]` = same layout as `T`.

## 6.3 Tại sao `UnsafeCell` đặc biệt?

Compiler **biết** `UnsafeCell` và **không** assume immutability của `&UnsafeCell<T>`. Vì vậy:
- Layout: same as T
- Aliasing: opted out of `&T` immutability assumption
- Optimization: compiler không "cache" reads through `&UnsafeCell<T>`

Bạn **không** thể tạo custom type với same magic — chỉ `UnsafeCell` có.

## 6.4 Build Cell on top

```rust
use std::cell::UnsafeCell;

pub struct Cell<T> {
    value: UnsafeCell<T>,
}

impl<T: Copy> Cell<T> {
    pub fn new(v: T) -> Self { Cell { value: UnsafeCell::new(v) } }
    
    pub fn get(&self) -> T {
        // SAFETY: T: Copy, single-thread (Cell !Sync)
        unsafe { *self.value.get() }
    }
    
    pub fn set(&self, v: T) {
        // SAFETY: T: Copy, single-thread
        unsafe { *self.value.get() = v; }
    }
}
```

Cell wrap UnsafeCell + restrict T to Copy. Safe API on top of unsafe core.

## 6.5 Build RefCell

```rust
use std::cell::UnsafeCell;
use std::cell::Cell;

pub struct RefCell<T: ?Sized> {
    flag: Cell<isize>,   // -1=writer, 0=free, N=N readers
    value: UnsafeCell<T>,
}

impl<T> RefCell<T> {
    pub fn borrow(&self) -> Ref<'_, T> {
        let f = self.flag.get();
        if f < 0 { panic!("already borrowed mutably"); }
        self.flag.set(f + 1);
        Ref { flag: &self.flag, val: unsafe { &*self.value.get() } }
    }
    
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        let f = self.flag.get();
        if f != 0 { panic!("already borrowed"); }
        self.flag.set(-1);
        RefMut { flag: &self.flag, val: unsafe { &mut *self.value.get() } }
    }
}
```

Runtime check ở Rust code — compile-time check tắt nhờ UnsafeCell.

## 6.6 UnsafeCell trong concurrency primitives

`Mutex<T>`, `RwLock<T>`, `AtomicXxx` — all use `UnsafeCell<T>` internally:

```rust
pub struct Mutex<T: ?Sized> {
    poison: poison::Flag,
    inner: sys::Mutex,
    data: UnsafeCell<T>,   // ← here
}
```

Sync primitive guarantees ngoài model Rust standard, dùng UnsafeCell để bypass borrow check.

## 6.7 Khi nào tự dùng UnsafeCell?

Hầu hết: KHÔNG. Dùng `Cell<T>`, `RefCell<T>`, `Mutex<T>` đủ.

Custom interior mutability cần khi:
- Build custom sync primitive
- Build custom lock-free data structure
- Embedded: control register access

```rust
struct MyCounter {
    count: UnsafeCell<u64>,
}

unsafe impl Sync for MyCounter {}   // unsafe — phải prove

impl MyCounter {
    pub fn increment(&self) {
        // SAFETY: ... (cần proof)
        unsafe { *self.count.get() += 1; }   // RACE CONDITION potential!
    }
}
```

Pattern trên có race condition — phải dùng atomic. UnsafeCell mỗi mình KHÔNG đủ cho thread safety.

---

# Tầng 7: Atomic Ordering — Memory model deep

## 7.1 Vấn đề: CPU reorder memory operations

```rust
// Thread 1:
x = 1;
y = 2;

// Thread 2:
println!("{} {}", y, x);
// Có thể print "2 0"!? 
// Hoặc "0 1"!?
```

CPU có thể reorder writes for performance. Without sync, observer thread thấy reorder.

## 7.2 Memory ordering trong Rust

```rust
use std::sync::atomic::{AtomicU32, Ordering};

let a = AtomicU32::new(0);
a.store(1, Ordering::Relaxed);
let v = a.load(Ordering::Relaxed);
```

5 orderings:

| Ordering | Guarantee |
|----------|-----------|
| `Relaxed` | Atomicity only. No ordering with other ops. |
| `Acquire` | Used with load. Synchronizes with `Release` store. |
| `Release` | Used with store. Synchronizes with `Acquire` load. |
| `AcqRel` | For RMW (fetch_add). Combines acquire + release. |
| `SeqCst` | Total order across all SeqCst ops. Strongest. |

## 7.3 Relaxed — Just atomicity

```rust
let counter = AtomicU64::new(0);
counter.fetch_add(1, Ordering::Relaxed);
```

Use cho counter where order doesn't matter. **Cheapest** atomic op.

Caveat: không synchronize với other memory operations.

## 7.4 Release / Acquire — Synchronization

Classic pattern: 1 thread "publish" data, others "consume".

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);
static mut DATA: u32 = 0;

// Thread 1 (publisher):
unsafe { DATA = 42; }
READY.store(true, Ordering::Release);   // Release

// Thread 2 (consumer):
while !READY.load(Ordering::Acquire) {}  // Acquire
let v = unsafe { DATA };   // Guaranteed to see 42
```

**Guarantee**:
- Anything **before** Release store in thread 1 happens-before anything **after** matching Acquire load in thread 2.

Acquire-Release pair: foundation of locks, channels.

## 7.5 SeqCst — Total order

```rust
let x = AtomicBool::new(false);
let y = AtomicBool::new(false);

// Thread 1:
x.store(true, Ordering::SeqCst);
let yv = y.load(Ordering::SeqCst);

// Thread 2:
y.store(true, Ordering::SeqCst);
let xv = x.load(Ordering::SeqCst);

// At least one thread sees the other's store.
// With Acquire/Release alone, both might see 0.
```

`SeqCst` đảm bảo **single global order** of all SeqCst ops. Strongest, slowest.

Default ordering của Rust atomic operations là SeqCst (nếu bạn không specify).

## 7.6 fence — Memory barrier

```rust
use std::sync::atomic::fence;

unsafe { DATA = 42; }
fence(Ordering::Release);
READY.store(true, Ordering::Relaxed);
```

`fence` insert memory barrier without specific atomic op. Useful for advanced patterns.

## 7.7 Common patterns

### Pattern 1: Counter (Relaxed OK)
```rust
let count = AtomicU64::new(0);
count.fetch_add(1, Ordering::Relaxed);
let v = count.load(Ordering::Relaxed);
```

### Pattern 2: Spinlock (Acquire/Release)
```rust
struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    fn lock(&self) {
        while self.locked.compare_exchange_weak(
            false, true,
            Ordering::Acquire,   // success ordering
            Ordering::Relaxed,    // failure ordering
        ).is_err() {
            std::hint::spin_loop();
        }
    }
    
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
```

### Pattern 3: Once init (Acquire on read, Release on write)
```rust
let init = AtomicBool::new(false);
let mut value = 0;

// Init (once):
value = compute();
init.store(true, Ordering::Release);

// Read:
if init.load(Ordering::Acquire) {
    println!("{}", value);
}
```

## 7.8 Quy tắc thực dụng

- **Counter / simple flag**: Relaxed
- **Lock / publish data**: Release on store, Acquire on load
- **Complex multi-variable invariant**: SeqCst (safe default)
- **Don't know**: SeqCst, then optimize if profile shows bottleneck

Atomic ordering là **CỰC KỲ KHÓ** — bugs là rare race conditions. Khi nghi ngờ, dùng SeqCst, viết test rigorous.

## 7.9 Tài liệu deep

- [Rust Atomics and Locks](https://marabos.nl/atomics/) — Mara Bos book
- [Rustonomicon — Atomics](https://doc.rust-lang.org/nomicon/atomics.html)
- C++ memory model reference (Rust dùng C++20 model)

---

# Tầng 8: Send và Sync — unsafe impl

## 8.1 Auto-trait Send và Sync

```rust
pub unsafe auto trait Send {}
pub unsafe auto trait Sync {}
```

- **Send**: T có thể move qua thread boundary
- **Sync**: &T có thể share qua thread (=  `T: Send` if `&T: Send`)

**Auto trait**: tự suy ra cho mọi type chứa toàn Send/Sync fields.

## 8.2 Most types Send + Sync

```rust
i32, String, Vec<T>, HashMap<K,V>, Arc<T>, Mutex<T>:  Send + Sync (if components are)
```

## 8.3 !Send types

- `Rc<T>`: counter non-atomic → race
- `*const T`, `*mut T`: raw pointer, manual control
- `MutexGuard<T>` (on some platforms): tied to thread

## 8.4 !Sync types

- `Cell<T>`, `RefCell<T>`: not thread-safe (run-time check non-atomic)
- `Rc<T>`: same as !Send
- Most `!Send` types also `!Sync`

## 8.5 Tự implement Send / Sync (unsafe!)

```rust
struct MyPtr {
    ptr: *mut u32,
}

// Default: *mut → !Send, !Sync
// If MyPtr is actually safe to send/share:
unsafe impl Send for MyPtr {}
unsafe impl Sync for MyPtr {}
```

`unsafe impl` because compiler can't verify thread-safety of raw pointer logic — bạn phải prove.

## 8.6 Khi nào tự impl?

- Custom data structure containing raw pointers (Vec, RawWaker)
- FFI types where C side guarantees safety
- Marker types for typed APIs

Examples:
- `std::rc::Rc<T>`: explicitly `impl !Send for Rc<T>`
- `std::sync::Arc<T>`: `unsafe impl Send + Sync if T: Send + Sync`
- Custom queue with internal locking: `unsafe impl Send + Sync`

## 8.7 PhantomData để control trait

```rust
use std::marker::PhantomData;

struct MyType<T> {
    ptr: *mut T,
    _phantom: PhantomData<T>,   // tell compiler we "own" T
}

// Without PhantomData, MyType<T> would always be Send + Sync
// regardless of T (because raw ptr is opt-out from auto-trait derivation)
// With PhantomData<T>, MyType<T>: Send iff T: Send (same for Sync)
```

`PhantomData<T>` carries T's trait through type system without actually storing T.

---

# Tầng 9: Memory layout — `#[repr(...)]` và alignment

## 9.1 Default layout

```rust
struct Point {
    x: u8,
    y: u32,
}

println!("{}", std::mem::size_of::<Point>());  // 8 (with padding)
```

Rust default = `repr(Rust)`:
- Fields có thể được **reorder** để minimize padding
- Layout không guaranteed across compiler versions
- **KHÔNG** dùng cho FFI

## 9.2 #[repr(C)] — C-compatible layout

```rust
#[repr(C)]
struct Point {
    x: u8,   // offset 0
    y: u32,  // offset 4 (3-byte padding before)
}
// Size: 8 bytes, alignment 4
```

`repr(C)`:
- Fields in declaration order
- Padding inserted to satisfy alignment
- Same as C struct
- **Use for FFI**

## 9.3 Padding & Alignment

```rust
#[repr(C)]
struct Bad {
    a: u8,   // offset 0, size 1
    b: u64,  // offset 8 (7-byte padding!), size 8
    c: u8,   // offset 16, size 1 (7-byte trailing padding)
}
// Total: 24 bytes

#[repr(C)]
struct Good {
    b: u64,  // offset 0
    a: u8,   // offset 8
    c: u8,   // offset 9
    // 6-byte trailing padding
}
// Total: 16 bytes
```

Order fields **descending alignment** để minimize padding (cho `repr(C)`).

`repr(Rust)` tự làm điều này. `repr(C)` follow declaration order — bạn lo padding.

## 9.4 #[repr(packed)] — No padding

```rust
#[repr(C, packed)]
struct Packed {
    a: u8,
    b: u64,
}
// Size: 9 bytes, alignment 1 — no padding!
```

`packed`:
- No padding
- Alignment = 1
- **Reading misaligned field = UB on some CPUs**

```rust
let p = Packed { a: 1, b: 100 };
let b = p.b;   // ⚠️ Read misaligned u64 — UB on ARM, slow on x86

// Safe:
let b = unsafe { std::ptr::read_unaligned(&p.b) };
```

Use carefully — chủ yếu cho FFI matching C `__attribute__((packed))`.

## 9.5 #[repr(transparent)] — Same as inner

```rust
#[repr(transparent)]
struct Wrapper(InnerType);
// Identical layout to InnerType
// Can transmute between them safely (if same type)
```

Used by `UnsafeCell`, `ManuallyDrop`, newtype patterns.

## 9.6 enum layout

```rust
enum E {
    A,        // discriminant 0
    B(u32),   // discriminant 1 + 4 bytes
    C(u8),    // discriminant 2 + 1 byte
}
// Size: max(variant) + tag
// Rust may use niche (e.g. Option<&T> is 8 bytes thanks to non-null pointer)

#[repr(u8)]
enum E2 { A = 0, B = 1, C = 2 }
// Force u8 discriminant

#[repr(C)]
enum E3 { ... }
// C-compatible (int discriminant + data)
```

## 9.7 alignment query

```rust
use std::mem::{size_of, align_of};

println!("{}", size_of::<u64>());        // 8
println!("{}", align_of::<u64>());       // 8

println!("{}", size_of::<Point>());      // depends on repr
println!("{}", align_of::<Point>());     // = max(field alignments)
```

## 9.8 #[repr(align(N))] — Force alignment

```rust
#[repr(align(64))]
struct CacheLineAligned {
    data: AtomicU64,
}
// Aligned to 64 bytes — avoid false sharing (see performance.md)
```

## 9.9 offset_of

```rust
#[repr(C)]
struct S { a: u32, b: u64 }

let off = std::mem::offset_of!(S, b);   // 8
```

Useful for FFI, manual layout calculation.

---

# Tầng 10: FFI — Gọi C / Được gọi từ C

## 10.1 Calling C from Rust

```rust
// Declare extern fn
extern "C" {
    fn abs(x: i32) -> i32;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

fn main() {
    let v = unsafe { abs(-5) };
    println!("{}", v);   // 5
}
```

`extern "C"` = use C calling convention.

For Rust function callable from C:
```rust
#[no_mangle]
pub extern "C" fn rust_function(x: i32) -> i32 {
    x * 2
}
```

`#[no_mangle]` = compiler không mangle name (else C linker can't find).

## 10.2 Linking to C library

```toml
# Cargo.toml
[package]
build = "build.rs"
```

```rust
// build.rs
fn main() {
    println!("cargo:rustc-link-lib=mylib");      // link -lmylib
    println!("cargo:rustc-link-search=/path");   // search dir
}
```

For automated bindings: `bindgen` crate.

```rust
// build.rs
fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .unwrap();
    bindings.write_to_file("src/bindings.rs").unwrap();
}
```

## 10.3 Types compatible với C

| Rust | C |
|------|---|
| `i32` / `i64` / `u32` / etc. | `int32_t`, `int64_t`, etc. |
| `usize` / `isize` | `size_t`, `ssize_t` |
| `f32` / `f64` | `float`, `double` |
| `bool` | `_Bool` |
| `*const T` / `*mut T` | `const T*` / `T*` |
| `Option<&T>` | `T*` (nullable) |
| `extern "C" fn() -> i32` | function pointer |
| `[T; N]` | `T[N]` |
| `#[repr(C)] struct` | `struct` |
| `#[repr(C)] enum` | `enum` |

NOT compatible:
- `Box<T>`, `Vec<T>`, `String` — Rust-specific layout
- `&T`, `&mut T` — reference (use raw pointer)
- `Result<T, E>`, `Option<T>` non-pointer

## 10.4 Strings cross FFI

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    let s = CString::new("hello").unwrap();    // null-terminated
    let len = unsafe { strlen(s.as_ptr()) };
    println!("{}", len);   // 5
}

// Rust string from C:
unsafe fn from_c_string(p: *const c_char) -> String {
    CStr::from_ptr(p).to_string_lossy().to_string()
}
```

`CString` for Rust → C, `CStr` for C → Rust. Both null-terminated.

## 10.5 Common patterns

### Pattern 1: Opaque pointer

```rust
// Rust:
pub struct Engine { /* internal */ }

#[no_mangle]
pub extern "C" fn engine_new() -> *mut Engine {
    Box::into_raw(Box::new(Engine { /* ... */ }))
}

#[no_mangle]
pub extern "C" fn engine_free(p: *mut Engine) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(p)); }
    }
}

#[no_mangle]
pub extern "C" fn engine_run(p: *mut Engine) {
    let engine = unsafe { &mut *p };
    engine.run();
}
```

C side sees `Engine*` opaque pointer.

### Pattern 2: Callback (function pointer)

```rust
extern "C" {
    fn register_callback(cb: extern "C" fn(i32) -> i32);
}

extern "C" fn my_callback(x: i32) -> i32 {
    x * 2
}

unsafe { register_callback(my_callback); }
```

## 10.6 Panic across FFI = UB

```rust
#[no_mangle]
pub extern "C" fn safe_api() -> i32 {
    panic!("oops");   // ⚠️ UB if propagates to C!
}
```

C ABI doesn't know about Rust panic. Must catch:

```rust
#[no_mangle]
pub extern "C" fn safe_api() -> i32 {
    std::panic::catch_unwind(|| {
        risky_rust_code();
        0
    }).unwrap_or(-1)
}
```

`catch_unwind` returns `Result<T, Box<dyn Any>>`. Convert panic to error code.

## 10.7 Lifetime in FFI

Lifetimes are **erased** at FFI boundary. Rust can't enforce safety. **You** lo:
- Pass pointer to C: Rust must keep data alive
- Return pointer from C: lifecycle là contract

```rust
// Rust gives C a pointer:
let buf = vec![0u8; 100];
let p = buf.as_ptr();
unsafe {
    c_function(p, buf.len());
}
// buf must stay alive during C call
```

## 10.8 cbindgen — Generate C header from Rust

```toml
[build-dependencies]
cbindgen = "0.27"
```

```rust
// build.rs
fn main() {
    cbindgen::Builder::new()
        .with_crate(env!("CARGO_MANIFEST_DIR"))
        .generate()
        .unwrap()
        .write_to_file("bindings.h");
}
```

Generates `bindings.h` from Rust `extern "C"` declarations. C code can `#include` it.

---

# Tầng 11: MaybeUninit và uninitialized memory

## 11.1 Vấn đề

```rust
let x: u32;
println!("{}", x);   // ❌ ERROR: use of uninitialized
```

Reading uninit memory = UB. But sometimes need to:
- Allocate buffer rồi fill lazily
- FFI where C side initializes

Old way (`mem::uninitialized`): **deprecated, almost always UB**.

New way: `MaybeUninit<T>`.

## 11.2 MaybeUninit

```rust
use std::mem::MaybeUninit;

let mut x: MaybeUninit<u32> = MaybeUninit::uninit();   // uninit but safe
x.write(42);                                            // initialize
let val = unsafe { x.assume_init() };                   // read (UB if not init)
```

`MaybeUninit<T>`:
- Same layout as T
- Can hold uninit value safely (no UB to construct)
- `assume_init()` is unsafe (caller swears it's initialized)
- No drop on go-of-scope (you manage)

## 11.3 Initialize array element-by-element

```rust
use std::mem::MaybeUninit;

let arr: [MaybeUninit<String>; 100] = unsafe {
    MaybeUninit::uninit().assume_init()   // array of uninit
};
let mut arr = arr;

for i in 0..100 {
    arr[i].write(format!("item {}", i));
}

// All initialized:
let arr: [String; 100] = unsafe {
    std::mem::transmute(arr)   // SAFE because all initialized
};
```

Common pattern for initializing array safely.

## 11.4 Buffer with FFI

```rust
extern "C" {
    fn read_data(buf: *mut u8, size: usize) -> usize;
}

fn read_n(n: usize) -> Vec<u8> {
    let mut buf: Vec<MaybeUninit<u8>> = Vec::with_capacity(n);
    unsafe { buf.set_len(n); }   // expose uninit memory
    
    let written = unsafe {
        read_data(buf.as_mut_ptr() as *mut u8, n)
    };
    
    // Assume first `written` bytes initialized
    let buf: Vec<u8> = unsafe {
        let mut buf = std::mem::ManuallyDrop::new(buf);
        Vec::from_raw_parts(buf.as_mut_ptr() as *mut u8, written, n)
    };
    
    buf
}
```

Pattern for "C fills buffer, we know length after".

## 11.5 Why not just `mem::zeroed`?

```rust
let x: String = unsafe { mem::zeroed() };
// String { len: 0, cap: 0, ptr: null }
// Looks OK but:
// - String invariant: ptr != null (even for empty string)
// - Drop would try to free null → UB
```

`mem::zeroed`: only safe for types where all-zero bits is valid (most primitive types, raw pointers). NOT safe for types with invariants (`String`, `Box`, `Vec`, references).

Prefer `MaybeUninit` for clarity.

## 11.6 Don't drop uninit!

```rust
let mut x: MaybeUninit<String> = MaybeUninit::uninit();
// x goes out of scope without init:
// ✅ OK — MaybeUninit doesn't run T's Drop
```

vs:
```rust
let x: String = unsafe { mem::uninitialized() };   // DEPRECATED
// x: actually uninitialized
// goes out of scope → Drop runs on garbage → UB
```

`MaybeUninit` safer because no automatic Drop.

---

# Tầng 12: mem::transmute và dynamic dispatch hack

## 12.1 transmute — Reinterpret bits

```rust
use std::mem;

let x: u32 = 0x12345678;
let bytes: [u8; 4] = unsafe { mem::transmute(x) };
// bytes = [0x78, 0x56, 0x34, 0x12] (little-endian)
```

`transmute<A, B>(a: A) -> B`: reinterpret bits of `a` as type `B`. **UB if invalid bit pattern**.

Requirements:
- `size_of::<A>() == size_of::<B>()`
- Bit pattern of A must be valid for B

## 12.2 Examples

```rust
// Float bits:
let f: f32 = 1.5;
let bits: u32 = unsafe { mem::transmute(f) };
// bits = 0x3FC00000

// Function pointer dance (rare, for FFI):
let f: fn() = some_fn;
let p: *const () = unsafe { mem::transmute(f) };

// Lifetime cast (DANGEROUS):
fn make_static<'a>(x: &'a str) -> &'static str {
    unsafe { mem::transmute(x) }   // ⚠️ promises that 'a outlives 'static
}
```

## 12.3 Tránh transmute when possible

Better alternatives:
```rust
// Bit cast:
let bits: u32 = f.to_bits();   // safe, no transmute
let f: f32 = f32::from_bits(bits);

// Array slicing:
let n: u32 = 0x12345678;
let bytes = n.to_le_bytes();   // safe
let bytes: [u8; 4] = unsafe { mem::transmute(n) };  // same thing but unsafe

// Pointer cast:
let p: *const A = ...;
let p: *const B = p as *const B;   // safe cast (deref still unsafe)
```

## 12.4 transmute_copy

```rust
let bytes = [1u8, 2, 3, 4];
let n: u32 = unsafe { mem::transmute_copy(&bytes) };
```

`transmute_copy<A, B>(&A) -> B`: copy bytes. Allowed if `size_of::<A>() >= size_of::<B>()`.

Useful when types same size but compiler doesn't know.

## 12.5 trait object internal hack

```rust
// dyn Trait is fat pointer: (data_ptr, vtable_ptr)
let t: Box<dyn Display> = Box::new(42);
let (data, vtable): (*const (), *const ()) = unsafe { mem::transmute(t.as_ref()) };
// data = pointer to 42
// vtable = vtable for Display impl on i32
```

Sometimes useful for reflection. **Very unsafe** — vtable layout not guaranteed stable.

## 12.6 Don't do this

```rust
// ❌ "Hack" lifetime extension
let s = String::from("hi");
let r: &'static str = unsafe { mem::transmute(s.as_str()) };
drop(s);
println!("{}", r);   // UB!

// ❌ "Hack" interior mutability without UnsafeCell
let x = 5;
let r: &mut i32 = unsafe { mem::transmute(&x) };
*r = 10;   // UB! Mutating through what's actually shared ref
```

transmute = nuclear option. Use sparingly.

---

# Tầng 13: Safe abstraction patterns

## 13.1 Pattern: Wrap unsafe in safe API

```rust
pub struct SafePtr<T> {
    inner: *mut T,
}

impl<T> SafePtr<T> {
    pub fn new(v: T) -> Self {
        let b = Box::new(v);
        SafePtr { inner: Box::into_raw(b) }
    }
    
    pub fn get(&self) -> &T {
        // SAFETY: inner is valid for life of SafePtr (constructor invariant)
        unsafe { &*self.inner }
    }
    
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: &mut self → exclusive access
        unsafe { &mut *self.inner }
    }
}

impl<T> Drop for SafePtr<T> {
    fn drop(&mut self) {
        // SAFETY: inner was Box::into_raw'd, so Box::from_raw is valid
        unsafe { drop(Box::from_raw(self.inner)); }
    }
}
```

Internal `unsafe`, external safe API. User never touches raw pointer.

## 13.2 Pattern: Document SAFETY invariants

```rust
/// # Safety
/// - `ptr` must point to a valid initialized `T`
/// - `ptr` must not be deallocated for at least 'a
pub unsafe fn from_ptr<'a, T>(ptr: *const T) -> &'a T {
    unsafe { &*ptr }
}
```

Document **exactly** what caller must guarantee. Comment inside unsafe block:

```rust
fn safe_function() {
    unsafe {
        // SAFETY: ptr was just created from Box and not freed
        let r = &*ptr;
    }
}
```

`// SAFETY: ...` comment is convention. Tools like clippy check.

## 13.3 Pattern: Newtype + invariant

```rust
pub struct NonZero<T>(T);

impl<T: PartialEq + Default> NonZero<T> {
    pub fn new(v: T) -> Option<Self> {
        if v == T::default() { None } else { Some(NonZero(v)) }
    }
    
    pub fn get(self) -> T { self.0 }
}

// Now anywhere `NonZero<T>` appears, invariant holds.
```

Invariant in constructor → safe API everywhere.

## 13.4 Pattern: Phantom types for state

```rust
use std::marker::PhantomData;

pub struct Db<State> {
    handle: u64,
    _state: PhantomData<State>,
}

pub struct Open;
pub struct Closed;

impl Db<Closed> {
    pub fn open() -> Db<Open> { /* ... */ }
}

impl Db<Open> {
    pub fn query(&self) { /* ... */ }
    pub fn close(self) -> Db<Closed> { /* ... */ }
}
```

Type system enforces state transitions. Wrong state → compile error.

## 13.5 Pattern: Drop guard for invariants

```rust
pub struct ResourceGuard {
    resource: u64,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        unsafe { release_resource(self.resource); }
    }
}

// User can't forget to release
let g = ResourceGuard { resource: acquire() };
// ... use ...
// g drops automatically
```

RAII pattern — wraps unsafe cleanup in safe Drop.

## 13.6 Pattern: Lifetime branding

```rust
pub fn with_resource<F, R>(f: F) -> R
where F: for<'a> FnOnce(Resource<'a>) -> R
{
    let r = unsafe { Resource { ... } };
    let result = f(r);
    // r dropped here, lifetime 'a ended
    result
}
```

Lifetime branded → user can't keep `Resource` outside `f`.

## 13.7 Pattern: Type erasure with vtable

```rust
struct VTable {
    drop: unsafe fn(*mut ()),
    method: unsafe fn(*mut (), i32) -> i32,
}

pub struct ErasedType {
    data: *mut (),
    vtable: &'static VTable,
}

// Build dyn-like type manually for control over layout
```

Used by smart pointers, trait objects. Allows static dispatch + dyn-like usage.

---

# Tầng 14: Tools — miri, sanitizers, cargo-careful

## 14.1 miri — Rust interpreter for UB

```bash
rustup +nightly component add miri
cargo +nightly miri test
```

Runs code in interpreter that **detects UB at runtime**:
- Reading uninit memory
- Use after free
- Out-of-bounds access
- Data races (limited)
- Aliasing violations (Stacked Borrows)

```rust
fn buggy() {
    let mut v = vec![1, 2, 3];
    let r = &v[0];
    v.push(4);          // May realloc — invalidate r
    println!("{}", r);  // miri detects!
}
```

Slow (10-100x slower than normal). Use cho test, not production.

## 14.2 cargo-careful

```bash
cargo install cargo-careful
cargo +nightly careful run
cargo +nightly careful test
```

Builds with extra runtime checks (debug assertions in std lib). Catches some UB miri doesn't.

## 14.3 Sanitizers

```bash
# Need nightly + specific target:
RUSTFLAGS="-Z sanitizer=address" cargo +nightly run
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly run
RUSTFLAGS="-Z sanitizer=memory" cargo +nightly run
RUSTFLAGS="-Z sanitizer=leak" cargo +nightly run
```

- **ASan**: address (use-after-free, buffer overflow)
- **TSan**: thread (data race)
- **MSan**: memory (uninit reads)
- **LSan**: leak (memory leak)

Slower runtime but high coverage. Run in CI for unsafe code.

## 14.4 Loom — Test concurrent code

```toml
[dev-dependencies]
loom = "0.7"
```

```rust
#[cfg(loom)]
use loom::sync::Mutex;
#[cfg(not(loom))]
use std::sync::Mutex;

#[test]
#[cfg_attr(loom, ignore)]
fn test() {
    loom::model(|| {
        // Loom runs this with ALL possible thread interleavings
        // Detects races, deadlocks
    });
}
```

```bash
RUSTFLAGS="--cfg loom" cargo test
```

Exhaustive concurrency testing. Find rare races.

## 14.5 cargo-fuzz — Fuzz unsafe code

```bash
cargo install cargo-fuzz
cargo fuzz init
cargo fuzz add my_target
cargo fuzz run my_target
```

Random input → call unsafe API → crash detection. Found many bugs in std lib.

## 14.6 Lints

```rust
#![warn(unsafe_op_in_unsafe_fn)]   // require explicit unsafe in unsafe fn
#![warn(clippy::undocumented_unsafe_blocks)]   // require SAFETY: comment
```

Clippy has many lints for unsafe. Enable strict ones.

## 14.7 Stable miri alternative — Mutation testing

```bash
cargo install cargo-mutants
cargo mutants
```

Mutate code (change `+` to `-`, etc), run tests. If tests pass with mutant → test gap. Find under-tested code.

---

# Tầng 15: Antipatterns và soundness bugs

## 15.1 ❌ Antipattern: unsafe để bypass borrow checker

```rust
let mut v = vec![1, 2, 3];
let r1 = unsafe { &mut *(&mut v[0] as *mut i32) };
let r2 = unsafe { &mut *(&mut v[1] as *mut i32) };
// "Created" 2 mut borrows of v
```

Compile passes nhưng UB. Borrow checker rejected for a reason.

✅ Fix: use `split_at_mut`:
```rust
let (r1, rest) = v.split_at_mut(1);
let r2 = &mut rest[0];
```

## 15.2 ❌ Aliasing two `&mut`

```rust
unsafe {
    let p = &mut x as *mut i32;
    let r1: &mut i32 = &mut *p;
    let r2: &mut i32 = &mut *p;  // 2 &mut to same data — UB
}
```

## 15.3 ❌ Returning ref to dropped data

```rust
fn bad() -> &'static i32 {
    let x = 5;
    unsafe { &*(&x as *const i32) }  // x drops, ref dangles → UB on use
}
```

## 15.4 ❌ Wrong atomic ordering

```rust
let flag = AtomicBool::new(false);
let mut data = 0;

// Thread 1:
data = 42;
flag.store(true, Ordering::Relaxed);   // ❌ Relaxed insufficient

// Thread 2:
while !flag.load(Ordering::Relaxed) {}
println!("{}", data);   // Might see 0 (compiler/CPU reorder)
```

Need Release/Acquire for synchronization.

## 15.5 ❌ Send/Sync wrong

```rust
struct Bad { ptr: *mut u32 }
unsafe impl Send for Bad {}  // ❌ Is it really safe? Need to prove.
unsafe impl Sync for Bad {}
```

If `Bad` shares mutable state without locking → race conditions in safe-looking code.

## 15.6 ❌ Drop after free

```rust
let b = Box::new(42);
let p = Box::into_raw(b);
unsafe {
    drop(Box::from_raw(p));   // free
    let v = *p;                // ❌ use after free → UB
}
```

## 15.7 ❌ Misaligned access

```rust
let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8];
let p = bytes.as_ptr().add(1) as *const u64;
unsafe {
    let v = *p;   // ❌ Read u64 at offset 1 — misaligned, UB on ARM
}
```

✅ Fix: `ptr::read_unaligned`:
```rust
let v = unsafe { ptr::read_unaligned(p) };
```

## 15.8 ❌ Reading uninit

```rust
let x: u32;
let v: u32 = unsafe { mem::transmute(x) };  // ❌ uninit read — UB
```

Use `MaybeUninit`.

## 15.9 ❌ FFI panic propagation

```rust
#[no_mangle]
pub extern "C" fn cb() {
    panic!("oops");   // ❌ UB if C side catches
}
```

Wrap in `catch_unwind`.

## 15.10 ❌ Self-referential without Pin

```rust
struct SelfRef {
    data: String,
    ptr: *const String,
}

let mut s = SelfRef { data: "hi".into(), ptr: std::ptr::null() };
s.ptr = &s.data;
// s moved? ptr dangles!
let s2 = s;   // ❌ ptr inside s2 still points to old location
```

Use `Pin<Box<T>>` to prevent move (see [lifetime.md](./lifetime.md) Tầng 14).

## 15.11 ✅ Best practices summary

```
┌─────────────────────────────────────────────────────────┐
│ 1. Document SAFETY invariants                           │
│ 2. Minimize unsafe block scope                           │
│ 3. Wrap unsafe in safe API                              │
│ 4. Test with miri                                       │
│ 5. Run sanitizers in CI                                 │
│ 6. Use loom for concurrent unsafe                       │
│ 7. Avoid transmute when possible                        │
│ 8. Document invariants for unsafe trait impls           │
│ 9. Prefer existing safe abstractions (Vec, Box, Arc)    │
│ 10. If unsure, ask. Unsafe is hard.                     │
└─────────────────────────────────────────────────────────┘
```

---

# Tổng kết — 12 nguyên tắc senior

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Unsafe enables 5 superpowers. Borrow checker still on.       │
│                                                                 │
│ 2. UB ≠ crash. UB = anything can happen. Avoid at all cost.     │
│                                                                 │
│ 3. Raw pointers don't have lifetime. You manage everything.     │
│                                                                 │
│ 4. UnsafeCell is foundation. Cell/RefCell/Mutex built on it.    │
│                                                                 │
│ 5. Atomic ordering: Relaxed → Acquire/Release → SeqCst.         │
│    When in doubt, SeqCst.                                       │
│                                                                 │
│ 6. Send/Sync auto-impl. unsafe impl ONLY if you can prove.      │
│                                                                 │
│ 7. #[repr(C)] for FFI. Default Rust layout not FFI-stable.      │
│                                                                 │
│ 8. FFI: types compatible, no panic across boundary.             │
│                                                                 │
│ 9. MaybeUninit for uninit memory. mem::uninitialized deprecated.│
│                                                                 │
│ 10. transmute is nuclear. Prefer safer alternatives.            │
│                                                                 │
│ 11. Wrap unsafe in safe API. Document SAFETY invariants.        │
│                                                                 │
│ 12. Test with miri, sanitizers, loom. Audit every unsafe block. │
└─────────────────────────────────────────────────────────────────┘
```

---

# Liên kết về memory model

Unsafe Rust = direct access to memory model concepts:

| Unsafe feature | Memory model |
|----------------|--------------|
| Raw pointer | Direct memory address, no metadata |
| Pointer arithmetic | Byte offset by `size_of::<T>()` |
| Alignment | CPU requires aligned access for non-byte types |
| Padding | Compiler insert to satisfy alignment |
| `#[repr(C)]` | Fixed layout matching C |
| `#[repr(packed)]` | No padding (potential UB on misaligned access) |
| Atomic ordering | CPU memory model (acquire/release semantics) |
| `UnsafeCell` | Opt-out of compiler immutability assumption |
| Cache line | False sharing avoidance with `#[repr(align(64))]` |

---

# Crates senior toolkit

| Crate | Mục đích |
|-------|----------|
| `bytemuck` | Safe transmute between Plain-Old-Data types |
| `zerocopy` | Zero-copy parsing of byte buffers |
| `cstr` | CString utilities |
| `libc` | C standard library bindings |
| `bindgen` | Auto-generate Rust bindings from C |
| `cbindgen` | Auto-generate C headers from Rust |
| `pin-project` | Safe Pin projection |
| `ouroboros` | Safe self-referential struct |
| `crossbeam-utils` | CachePadded, atomic helpers |
| `loom` | Concurrent code testing |

---

# Lộ trình tiếp theo

Bạn đã có 14 chủ đề:

```
1. memory-model
2. ownership-borrowing
3. trait
4. generic
5. closure
6. async
7. error-handling
8. macros
9. smart-pointers
10. lifetime
11. performance
12. observability
13. iterator
14. unsafe-rust         ← MỚI
```

Còn các topic chuyên sâu thực hành:

- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 14 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
