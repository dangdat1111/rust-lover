# Unsafe Rust — Minh Hoạ Trực Quan

> Companion visual cho [unsafe-rust.md](./unsafe-rust.md). Đọc song song.

---

## 1. Bức tranh lớn — Unsafe Rust Universe

```
                          UNSAFE RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   "Cửa thoát hiểm" — 5 SUPERPOWERS                     │
       │                                                        │
       │   ┌─────────────────────────────────────────────────┐  │
       │   │ 1. Deref raw pointer                            │  │
       │   │ 2. Call unsafe fn                               │  │
       │   │ 3. Access mut static                            │  │
       │   │ 4. Impl unsafe trait                            │  │
       │   │ 5. Access union field                           │  │
       │   └─────────────────────────────────────────────────┘  │
       │                                                        │
       │   Borrow checker VẪN HOẠT ĐỘNG                          │
       │                                                        │
       │   ┌──────────────────┐    ┌──────────────────┐         │
       │   │ Use cases:        │    │ Risks:           │        │
       │   │  • FFI            │    │  • UB            │        │
       │   │  • Build abstr.   │    │  • Security      │        │
       │   │  • Hot path opt   │    │  • Crash random  │        │
       │   │  • Hardware       │    │  • Hard debug    │        │
       │   │  • Embedded       │    │                  │        │
       │   └──────────────────┘    └──────────────────┘         │
       │                                                        │
       │   Triết lý:                                            │
       │   "Unsafe is contagious, but contained"                │
       │   Wrap unsafe in safe API → user never sees unsafe     │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. 5 Superpowers

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  1. DEREFERENCE RAW POINTER                                  │
   │     ────────────────────────                                 │
   │     let p: *const i32 = &x;                                  │
   │     unsafe { let v = *p; }      ← cần unsafe                 │
   │                                                              │
   ├──────────────────────────────────────────────────────────────┤
   │                                                              │
   │  2. CALL UNSAFE FUNCTION                                     │
   │     ──────────────────                                       │
   │     unsafe fn dangerous() {}                                 │
   │     unsafe { dangerous(); }     ← cần unsafe                 │
   │                                                              │
   │     Examples: Vec::set_len, String::from_utf8_unchecked,     │
   │               mem::transmute, FFI extern functions           │
   │                                                              │
   ├──────────────────────────────────────────────────────────────┤
   │                                                              │
   │  3. ACCESS MUTABLE STATIC                                    │
   │     ─────────────────────                                    │
   │     static mut COUNTER: u32 = 0;                             │
   │     unsafe { COUNTER += 1; }                                 │
   │                                                              │
   │     (Avoid — use AtomicU32 instead)                          │
   │                                                              │
   ├──────────────────────────────────────────────────────────────┤
   │                                                              │
   │  4. IMPLEMENT UNSAFE TRAIT                                   │
   │     ───────────────────                                      │
   │     unsafe impl Send for MyType {}                           │
   │     unsafe impl Sync for MyType {}                           │
   │     unsafe impl GlobalAlloc for MyAlloc {}                   │
   │                                                              │
   ├──────────────────────────────────────────────────────────────┤
   │                                                              │
   │  5. ACCESS UNION FIELD                                       │
   │     ─────────────────                                        │
   │     union U { f1: u32, f2: f32 }                             │
   │     let u = U { f1: 1 };                                     │
   │     unsafe { let v = u.f1; }                                 │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   📌 unsafe KHÔNG tắt borrow checker. Còn lại đầy đủ.
```

---

## 3. Raw Pointer vs Reference

```
   ┌─────────────────────────────────────────────────────────────┐
   │  Aspect              │ &T / &mut T   │ *const T / *mut T   │
   ├──────────────────────┼───────────────┼─────────────────────┤
   │  Lifetime track      │ ✅ Yes         │ ❌ No                │
   │  Can be null         │ ❌ Never       │ ✅ Yes               │
   │  Can alias mutable   │ ❌ No          │ ✅ Yes (UB careful) │
   │  Deref needs unsafe  │ ❌ No          │ ✅ Yes               │
   │  Alignment guaranteed│ ✅ Yes         │ ❌ No                │
   │  Validity guaranteed │ ✅ Yes         │ ❌ No                │
   │  Cast freely         │ ❌ Limited     │ ✅ Yes               │
   │  Arithmetic          │ ❌ No          │ ✅ .add(n), .offset()│
   └──────────────────────┴───────────────┴─────────────────────┘
   
   
   Diagram:
   ────────
   
   &T          → safe abstraction        →  managed
                ┌──────────┐
                │  &T      │ ─► T (lifetime-tracked)
                │ (8 byte) │
                └──────────┘
   
   *const T    → C-style pointer          →  manual
                ┌──────────┐
                │ *const T │ ─► ??? (no guarantee)
                │ (8 byte) │
                └──────────┘
                Could be: null, dangling, misaligned, valid
                You verify before deref.
```

---

## 4. Pointer arithmetic

```
   let arr = [1, 2, 3, 4, 5];
   let p: *const i32 = arr.as_ptr();
   
   
   Memory layout:
   ──────────────
   ┌─────┬─────┬─────┬─────┬─────┐
   │  1  │  2  │  3  │  4  │  5  │
   └─────┴─────┴─────┴─────┴─────┘
   ▲     ▲     ▲     ▲     ▲
   p     p+1   p+2   p+3   p+4
   |     |     |     |     |
   each step = sizeof(i32) = 4 bytes
   
   
   .add(n):    pointer offset by n * sizeof(T)
   .offset(n): same but signed (allows negative)
   
   
   ⚠️ UB if:
   ──────────
   • Pointer goes out of allocation:
   
       arr  arr.add(5)            arr.add(100)
       │    │   (one-past-end OK)  │  (UB!)
       ▼    ▼                       ▼
   [   1  2  3  4  5  ]?  ?  ?  ?  ?  ?  ?
                     │   ← outside allocation
                     │     even if you don't dereference
   
   • Resulting pointer overflows isize range
```

---

## 5. UB — Undefined Behavior

```
   ┌──────────────────────────────────────────────────────────┐
   │  UB = "Undefined Behavior"                               │
   │                                                          │
   │  Compiler giả định UB KHÔNG xảy ra                       │
   │  → Optimization based on this assumption                 │
   │  → Once UB happens, anything can happen                  │
   └──────────────────────────────────────────────────────────┘
   
   
   UB ≠ Crash:
   ───────────
   
   UB code có thể:
   ┌─────────────────────────────────────────────┐
   │ ✓ Work fine in tests                        │
   │ ✓ Work fine in dev                          │
   │ ✗ Crash randomly in production             │
   │ ✗ Corrupt memory silently                   │
   │ ✗ Leak secrets                              │
   │ ✗ Execute attacker's code                  │
   │ ✗ Behave differently each run               │
   │ ✗ "Time travel": UB at line N affects line 1│
   └─────────────────────────────────────────────┘
   
   
   Common UB:
   ──────────
   
   ┌────────────────────────────────────────────┐
   │ MEMORY:                                    │
   │  • Dangling pointer deref                  │
   │  • Null pointer deref                      │
   │  • Misaligned pointer deref                │
   │  • Uninit memory read                      │
   │  • Out-of-bounds                           │
   ├────────────────────────────────────────────┤
   │ ALIASING:                                  │
   │  • 2 &mut to same data                     │
   │  • &T + &mut T same data                   │
   │  • Mutate via &T (without UnsafeCell)      │
   ├────────────────────────────────────────────┤
   │ TYPE:                                      │
   │  • Invalid bit pattern (bool ≠ 0/1)        │
   │  • Wrong layout FFI                        │
   │  • Type-confused pointer                   │
   ├────────────────────────────────────────────┤
   │ CONCURRENCY:                               │
   │  • Data race                               │
   │  • Wrong atomic ordering                   │
   ├────────────────────────────────────────────┤
   │ OTHER:                                     │
   │  • Panic across FFI                        │
   │  • Invalid Box/Vec/String                  │
   └────────────────────────────────────────────┘
```

---

## 6. Aliasing rules — Stacked Borrows

```
   ┌──────────────────────────────────────────────────────────┐
   │  Memory location 0x1000:                                 │
   │                                                          │
   │   Stack of tags (LIFO):                                  │
   │   ┌───────────────────┐                                  │
   │   │ Tag #3 (Unique)   │ ← top                            │
   │   ├───────────────────┤                                  │
   │   │ Tag #2 (Unique)   │                                  │
   │   ├───────────────────┤                                  │
   │   │ Tag #1 (Shared)   │                                  │
   │   └───────────────────┘                                  │
   │                                                          │
   │   Each &T / &mut T has a tag                             │
   │   Access requires tag to be VALID on stack               │
   └──────────────────────────────────────────────────────────┘
   
   
   Example flow:
   ─────────────
   
   let mut x = 5;        Stack: []
   
   let r1 = &mut x;       Stack: [#1 Unique]
   
   let r2 = &mut *r1;     Stack: [#1 Unique, #2 Unique]
                                  (#2 is sub-borrow of #1)
   
   *r2 = 10;              OK — #2 on top
   
   *r1 = 20;              Pop #2 first (no longer needed)
                          Stack: [#1 Unique]
                          OK — #1 on top
   
   
   ❌ Violation:
   ──────────────
   
   let r2 = &mut *r1;     Stack: [#1, #2]
   *r1 = 30;              ⚠️ #1 not on top, #2 is
                          Compiler may optimize as if #1 unique
                          → reading via #2 next can see stale value
                          → UB
```

---

## 7. UnsafeCell — Foundation

```
   ┌──────────────────────────────────────────────────────────┐
   │  #[repr(transparent)]                                    │
   │  pub struct UnsafeCell<T: ?Sized> { value: T }           │
   │                                                          │
   │  impl<T: ?Sized> UnsafeCell<T> {                         │
   │      pub fn get(&self) -> *mut T {                       │
   │          // Magic: &self → *mut T                        │
   │          self as *const _ as *mut _                      │
   │      }                                                   │
   │  }                                                       │
   └──────────────────────────────────────────────────────────┘
   
   
   Cấu trúc xây dựng:
   ──────────────────
   
   ┌─────────────────────────────────────────────────────┐
   │                                                     │
   │   YOUR Code (safe)                                  │
   │       │                                             │
   │       ▼                                             │
   │   ┌────────────────────────────────────────────┐    │
   │   │  Cell / RefCell / Mutex / RwLock           │    │
   │   │  (safe API on top)                         │    │
   │   └────────────────────────────────────────────┘    │
   │       │                                             │
   │       ▼ wraps                                       │
   │   ┌────────────────────────────────────────────┐    │
   │   │  UnsafeCell<T>                             │    │
   │   │  (the ONLY way to legally have             │    │
   │   │   &T → *mut T → write in Rust)             │    │
   │   └────────────────────────────────────────────┘    │
   │       │                                             │
   │       ▼                                             │
   │   raw memory                                        │
   │                                                     │
   └─────────────────────────────────────────────────────┘
   
   
   Why magic?
   ──────────
   
   Compiler KNOWS UnsafeCell và:
   • Does NOT assume &UnsafeCell<T> is immutable
   • Does NOT cache reads through &UnsafeCell<T>
   • Opted out of normal aliasing rules
   
   You CANNOT create another type with this property —
   only UnsafeCell has this compiler magic.
```

---

## 8. Atomic Ordering — Memory model

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   STRENGTH ladder:                                       │
   │                                                          │
   │   Relaxed  ←─────────────── weakest                      │
   │      │                                                   │
   │      │ atomicity only                                    │
   │      │ no synchronization with other memory ops          │
   │      │                                                   │
   │   Acquire (load) / Release (store)                       │
   │      │                                                   │
   │      │ pair: matched Release → Acquire creates           │
   │      │ happens-before relation                           │
   │      │                                                   │
   │   AcqRel (RMW operations)                                │
   │      │                                                   │
   │      │ combines acquire + release                        │
   │      │                                                   │
   │   SeqCst ←──────────────── strongest                     │
   │                                                          │
   │      Total order across all SeqCst ops globally          │
   │      Slowest but easiest to reason about                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Release-Acquire pattern (publisher-consumer):
   ──────────────────────────────────────────────
   
   Thread 1 (publisher):           Thread 2 (consumer):
   ─────────────────────           ───────────────────
   
   write DATA = 42;                while !READY.load(Acquire) {}
              │                          │
              │                          │
              ▼ happens-before           │
                                         │
   READY.store(true, Release);           │
              │                          │
              │      synchronizes-with   │
              └──────────────────────────┘
                                         │
                                         ▼
                                    read DATA  ← guaranteed 42!
   
   
   ┌──────────────────────────────────────────────────┐
   │ GUARANTEE: anything before Release happens-before│
   │ anything after the matching Acquire.             │
   └──────────────────────────────────────────────────┘
```

---

## 9. Atomic patterns

```
   ┌─────────────────────────────────────────────────────────┐
   │  Pattern 1: COUNTER (Relaxed is enough)                 │
   │  ─────────────────────────────────────                  │
   │                                                         │
   │  let count = AtomicU64::new(0);                         │
   │  count.fetch_add(1, Ordering::Relaxed);                 │
   │                                                         │
   │  Why Relaxed? Order of counter updates doesn't          │
   │  affect correctness — just the final value matters.     │
   │                                                         │
   ├─────────────────────────────────────────────────────────┤
   │  Pattern 2: SPINLOCK (Acquire/Release)                  │
   │  ─────────────────────────────────────                  │
   │                                                         │
   │  fn lock(&self) {                                       │
   │      while self.locked.compare_exchange_weak(           │
   │          false, true,                                   │
   │          Ordering::Acquire,    ← gain lock              │
   │          Ordering::Relaxed,                             │
   │      ).is_err() { spin_loop(); }                        │
   │  }                                                      │
   │                                                         │
   │  fn unlock(&self) {                                     │
   │      self.locked.store(false, Ordering::Release);       │
   │      //                          ↑ release lock         │
   │  }                                                      │
   │                                                         │
   ├─────────────────────────────────────────────────────────┤
   │  Pattern 3: ONCE INIT                                   │
   │  ──────────────────                                     │
   │                                                         │
   │  // Init:                                               │
   │  value = compute();                                     │
   │  init.store(true, Ordering::Release);                   │
   │                                                         │
   │  // Read:                                               │
   │  if init.load(Ordering::Acquire) {                      │
   │      println!("{}", value);    ← guaranteed init       │
   │  }                                                      │
   └─────────────────────────────────────────────────────────┘
   
   
   Default: Khi không chắc, dùng SeqCst.
   ────────────────────────────────────
   
   ┌────────────────────────────────────────────────────┐
   │ Pros of SeqCst:                                    │
   │  • Easiest to reason about                         │
   │  • Hardest to misuse                               │
   │  • Total order globally                            │
   │                                                    │
   │ Cons:                                              │
   │  • Slowest (memory barrier instructions)           │
   │  • Sometimes "too strong" — overkill               │
   │                                                    │
   │ Optimize to Acquire/Release/Relaxed AFTER profile  │
   │ shows bottleneck.                                  │
   └────────────────────────────────────────────────────┘
```

---

## 10. Send / Sync — Thread safety

```
   ┌──────────────────────────────────────────────────────────┐
   │  SEND: T can be moved across thread boundary             │
   │                                                          │
   │  let v = vec![1, 2, 3];                                  │
   │  thread::spawn(move || println!("{:?}", v));             │
   │           //  ↑ move requires Vec<i32>: Send            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  SYNC: &T can be shared across threads                   │
   │                                                          │
   │  let m = Mutex::new(5);                                  │
   │  thread::scope(|s| {                                     │
   │      s.spawn(|| println!("{}", m.lock().unwrap()));       │
   │      s.spawn(|| println!("{}", m.lock().unwrap()));      │
   │  });                                                     │
   │  // Requires Mutex<i32>: Sync                            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  Auto-trait: tự suy ra cho mọi type chứa toàn Send/Sync │
   └──────────────────────────────────────────────────────────┘
   
   
   Types and their thread-safety:
   ──────────────────────────────
   
   ┌──────────────┬──────┬──────┐
   │ Type         │ Send │ Sync │
   ├──────────────┼──────┼──────┤
   │ i32          │ ✅   │ ✅   │
   │ String       │ ✅   │ ✅   │
   │ Vec<T>       │ if T │ if T │
   │ Arc<T>       │ if T │ if T │
   │ Mutex<T>     │ if T │ if T │
   │ &T           │ if T │ if T │
   ├──────────────┼──────┼──────┤
   │ Rc<T>        │ ❌   │ ❌   │  ← non-atomic count
   │ Cell<T>      │ if T │ ❌   │  ← interior mut, no sync
   │ RefCell<T>   │ if T │ ❌   │  ← runtime check non-atomic
   │ *const T     │ ❌   │ ❌   │  ← raw pointer, opt-out
   │ *mut T       │ ❌   │ ❌   │
   └──────────────┴──────┴──────┘
   
   
   unsafe impl — Override:
   ───────────────────────
   
   struct MyPtr { ptr: *mut u32 }   // default !Send !Sync
   
   unsafe impl Send for MyPtr {}    // ← phải PROVE safe
   unsafe impl Sync for MyPtr {}    //   nếu không → UB
```

---

## 11. Memory Layout — `#[repr]`

```
   ┌────────────────────────────────────────────────────────────┐
   │  Default: #[repr(Rust)]                                    │
   │  ────────────────────                                      │
   │                                                            │
   │  struct Point { x: u8, y: u32 }                            │
   │                                                            │
   │  Compiler reorders fields:                                 │
   │                                                            │
   │  Memory:  [y: u32 (4B)][x: u8 (1B)][padding 3B]            │
   │             ↑                                              │
   │  Total: 8 bytes, no waste                                  │
   │                                                            │
   │  ⚠️ Layout NOT guaranteed — varies by compiler version    │
   │  ⚠️ DO NOT use for FFI                                    │
   └────────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────────┐
   │  #[repr(C)]                                                │
   │  ──────────                                                │
   │                                                            │
   │  #[repr(C)]                                                │
   │  struct Point { x: u8, y: u32 }                            │
   │                                                            │
   │  Memory:  [x: u8 (1B)][padding 3B][y: u32 (4B)]            │
   │            ↑                                               │
   │  Total: 8 bytes (with padding before y for alignment)      │
   │                                                            │
   │  ✅ Layout guaranteed                                      │
   │  ✅ Same as C struct                                       │
   │  ✅ Use for FFI                                            │
   └────────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────────┐
   │  #[repr(C, packed)]                                        │
   │  ──────────────────                                        │
   │                                                            │
   │  #[repr(C, packed)]                                        │
   │  struct Point { x: u8, y: u32 }                            │
   │                                                            │
   │  Memory:  [x: u8 (1B)][y: u32 (4B)]                        │
   │            ↑           ↑                                   │
   │  Total: 5 bytes, NO padding                                │
   │  Alignment: 1                                              │
   │                                                            │
   │  ⚠️ Reading y at offset 1 = MISALIGNED                    │
   │     → UB on ARM, slow on x86                              │
   │  ✅ Match C __attribute__((packed))                       │
   │                                                            │
   │  Read safely: ptr::read_unaligned(&p.y)                    │
   └────────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────────┐
   │  #[repr(align(64))] — Force alignment                      │
   │  ─────────────────────────────                             │
   │                                                            │
   │  #[repr(align(64))]                                        │
   │  struct CacheLineAligned {                                 │
   │      counter: AtomicU64,                                   │
   │  }                                                         │
   │                                                            │
   │  Memory: aligned to 64-byte boundary                       │
   │  Use: avoid false sharing                                  │
   └────────────────────────────────────────────────────────────┘
```

---

## 12. Padding visualization

```
   #[repr(C)]
   struct Bad {
       a: u8,    // size 1, alignment 1
       b: u64,   // size 8, alignment 8
       c: u8,    // size 1, alignment 1
   }
   
   Memory layout:
   ──────────────
   
   offset:  0  1  2  3  4  5  6  7  8  ...   15 16 17 18 19 20 21 22 23
            ┌──┬──┬──┬──┬──┬──┬──┬──┬───────────────┬──┬──┬──┬──┬──┬──┬──┐
            │a │P │P │P │P │P │P │P │       b       │c │P │P │P │P │P │P │
            └──┴──┴──┴──┴──┴──┴──┴──┴───────────────┴──┴──┴──┴──┴──┴──┴──┘
                  ↑                                       ↑
            7 bytes padding to align b              7 bytes trailing pad
            
   Total: 24 bytes (with 14 bytes of padding)
   
   
   ✅ Reorder for min padding:
   ───────────────────────────
   
   #[repr(C)]
   struct Good {
       b: u64,   // largest first
       a: u8,
       c: u8,
       // 6 bytes trailing
   }
   
   offset:  0  ...  7  8  9 10 11 12 13 14 15
            ┌───────────┬──┬──┬──┬──┬──┬──┬──┬──┐
            │     b     │a │c │P │P │P │P │P │P │
            └───────────┴──┴──┴──┴──┴──┴──┴──┴──┘
   
   Total: 16 bytes  (saved 8 bytes!)
   
   📌 #[repr(Rust)] does this reordering AUTOMATICALLY
       #[repr(C)] follows DECLARATION ORDER — bạn lo padding
```

---

## 13. FFI flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   RUST CALLING C:                                        │
   │                                                          │
   │   ┌──────────────────┐                                   │
   │   │ Rust code        │                                   │
   │   │                  │                                   │
   │   │  extern "C" {    │                                   │
   │   │    fn abs(x:i32) │                                   │
   │   │      -> i32;     │                                   │
   │   │  }               │                                   │
   │   │                  │                                   │
   │   │  unsafe { abs(-5) }─────┐                            │
   │   └──────────────────┘     │                             │
   │                            │ C ABI call                  │
   │                            ▼                             │
   │                       ┌──────────────────┐               │
   │                       │ libc / mylib.so  │               │
   │                       │ (C compiled)     │               │
   │                       │                  │               │
   │                       │ int abs(int x)   │               │
   │                       │ { return x>=0    │               │
   │                       │   ? x : -x; }    │               │
   │                       └──────────────────┘               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   C CALLING RUST:                                        │
   │                                                          │
   │   ┌──────────────────┐                                   │
   │   │ Rust lib         │                                   │
   │   │                  │                                   │
   │   │  #[no_mangle]    │                                   │
   │   │  pub extern "C"  │                                   │
   │   │  fn rust_fn(x:   │                                   │
   │   │     i32) -> i32  │                                   │
   │   │  { x * 2 }       │                                   │
   │   └──────────┬───────┘                                   │
   │              │ compiled to libmyrust.so                  │
   │              │ symbol: rust_fn (not mangled)             │
   │              ▼                                           │
   │   ┌──────────────────┐                                   │
   │   │ C code           │                                   │
   │   │                  │                                   │
   │   │  extern int      │                                   │
   │   │  rust_fn(int);   │                                   │
   │   │                  │                                   │
   │   │  int y =         │                                   │
   │   │    rust_fn(5);   │                                   │
   │   └──────────────────┘                                   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. FFI types compatibility

```
   ┌──────────────────────────┬──────────────────────────────┐
   │  Rust                    │  C                           │
   ├──────────────────────────┼──────────────────────────────┤
   │  i8, i16, i32, i64       │  int8_t, int16_t, ...        │
   │  u8, u16, u32, u64       │  uint8_t, uint16_t, ...      │
   │  f32, f64                │  float, double               │
   │  usize, isize            │  size_t, ssize_t             │
   │  bool                    │  _Bool                       │
   │  char                    │  uint32_t (32-bit Unicode)   │
   │                          │                              │
   │  *const T, *mut T        │  const T*, T*                │
   │  Option<&T>              │  T*  (null = None)           │
   │  Option<NonNull<T>>      │  T*                          │
   │                          │                              │
   │  extern "C" fn() -> i32  │  function pointer            │
   │                          │                              │
   │  [T; N]                  │  T[N]                        │
   │  #[repr(C)] struct       │  struct                      │
   │  #[repr(C)] enum         │  enum                        │
   │                          │                              │
   │  CString                 │  char* (null-terminated)     │
   │  CStr                    │  const char*                 │
   ├──────────────────────────┼──────────────────────────────┤
   │  ❌ NOT compatible:      │                              │
   │  Box<T>, Vec<T>, String  │  (Rust-specific layout)      │
   │  &T, &mut T              │  (use raw pointer)           │
   │  Option<T> non-pointer   │  (Rust tagged union)         │
   │  Result<T, E>            │  (Rust-specific)             │
   └──────────────────────────┴──────────────────────────────┘
```

---

## 15. MaybeUninit — Uninitialized memory

```
   ┌────────────────────────────────────────────────────────┐
   │  Old way (DEPRECATED, dangerous):                      │
   │  ────────────────────────                              │
   │  let x: u32 = unsafe { mem::uninitialized() };         │
   │  // x has garbage value                                │
   │  println!("{}", x);  // ❌ UB if u32 invariants matter │
   │  // Goes out of scope → Drop runs on garbage → UB      │
   └────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────┐
   │  Modern way: MaybeUninit                               │
   │  ────────────────────────                              │
   │                                                        │
   │  use std::mem::MaybeUninit;                            │
   │                                                        │
   │  let mut x: MaybeUninit<u32> = MaybeUninit::uninit();  │
   │                  │                                     │
   │                  │ • Same layout as u32                │
   │                  │ • Holds uninit value SAFELY         │
   │                  │ • No Drop on go-of-scope            │
   │                  ▼                                     │
   │                                                        │
   │  x.write(42);    ← initialize                          │
   │                                                        │
   │  let v = unsafe {                                      │
   │      x.assume_init()                                   │
   │      //  ↑                                             │
   │      //  You promise it's initialized                  │
   │  };                                                    │
   │                                                        │
   │  println!("{}", v);   // 42                            │
   │                                                        │
   └────────────────────────────────────────────────────────┘
   
   
   Pattern: array init element-by-element
   ──────────────────────────────────────
   
   let arr: [MaybeUninit<String>; 100] = unsafe {
       MaybeUninit::uninit().assume_init()
   };
   let mut arr = arr;
   
   for i in 0..100 {
       arr[i].write(format!("item {}", i));
   }
   
   // Now ALL initialized:
   let arr: [String; 100] = unsafe {
       std::mem::transmute(arr)
   };
   
   // Safety: we initialized every element
```

---

## 16. Safe abstraction pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   YOUR LIBRARY                                           │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ Public API (100% safe Rust)                    │     │
   │   │                                                │     │
   │   │  fn safe_op(&self, x: i32) -> i32              │     │
   │   │  fn safe_get(&self) -> &T                      │     │
   │   │  fn safe_iter(&self) -> impl Iterator          │     │
   │   │                                                │     │
   │   │  User CANNOT cause UB through this API         │     │
   │   └────────────────┬───────────────────────────────┘     │
   │                    │                                     │
   │                    ▼ implementation                      │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ INTERNAL (unsafe)                              │     │
   │   │                                                │     │
   │   │  unsafe {                                       │    │
   │   │      // SAFETY: ptr is valid because we got    │     │
   │   │      // it from Box::into_raw and haven't      │     │
   │   │      // freed it.                              │     │
   │   │      let r = &*self.ptr;                        │    │
   │   │  }                                              │    │
   │   │                                                │     │
   │   │  // Internal invariants enforced by:            │    │
   │   │  // 1. Constructor (set initial state)          │    │
   │   │  // 2. Drop (cleanup)                           │    │
   │   │  // 3. Mutability rules of &self / &mut self    │    │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Example — SafePtr:
   ──────────────────
   
   pub struct SafePtr<T> {
       inner: *mut T,
   }
   
   impl<T> SafePtr<T> {
       pub fn new(v: T) -> Self {              // safe constructor
           SafePtr { inner: Box::into_raw(Box::new(v)) }
       }
       
       pub fn get(&self) -> &T {                // safe API
           unsafe { &*self.inner }              // unsafe inside
           // SAFETY: inner valid by constructor invariant
       }
   }
   
   impl<T> Drop for SafePtr<T> {                // safe cleanup
       fn drop(&mut self) {
           unsafe { drop(Box::from_raw(self.inner)); }
       }
   }
```

---

## 17. Document SAFETY

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   FOR UNSAFE FN:                                         │
   │   /// # Safety                                           │
   │   ///                                                    │
   │   /// - `ptr` must point to valid `T`                    │
   │   /// - `ptr` must not be deallocated during 'a          │
   │   /// - No other `&mut` to the same data exists          │
   │   pub unsafe fn from_ptr<'a, T>(ptr: *const T)           │
   │       -> &'a T {                                         │
   │       unsafe { &*ptr }                                   │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   INSIDE UNSAFE BLOCK:                                   │
   │                                                          │
   │   fn safe_function(&self) -> &T {                        │
   │       unsafe {                                           │
   │           // SAFETY: self.ptr was obtained from          │
   │           // Box::into_raw in constructor, and is        │
   │           // valid for self's entire lifetime.           │
   │           &*self.ptr                                     │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   │   ⟹ "// SAFETY: ..." comment là CONVENTION              │
   │     Clippy có lint check                                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   FOR UNSAFE TRAIT IMPL:                                 │
   │                                                          │
   │   // SAFETY: MyPtr only holds pointers to data           │
   │   // allocated on heap via Box, with no thread-local     │
   │   // state. Sending across threads is safe because       │
   │   // we don't share mutable state through shared refs.   │
   │   unsafe impl Send for MyPtr {}                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. Tools workflow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   UNSAFE CODE WORKFLOW                                   │
   │                                                          │
   │   1. Write unsafe code with SAFETY comments              │
   │              │                                           │
   │              ▼                                           │
   │   2. Run unit tests (basic check)                        │
   │              │                                           │
   │              ▼                                           │
   │   3. miri:                                               │
   │      cargo +nightly miri test                            │
   │      → Detects UB at runtime (slow but thorough)         │
   │              │                                           │
   │              ▼                                           │
   │   4. Sanitizers:                                         │
   │      RUSTFLAGS="-Z sanitizer=address" cargo +nightly run │
   │      RUSTFLAGS="-Z sanitizer=thread" ...                 │
   │      → AddressSanitizer, ThreadSanitizer                 │
   │              │                                           │
   │              ▼                                           │
   │   5. loom (concurrent):                                  │
   │      RUSTFLAGS="--cfg loom" cargo test                   │
   │      → All thread interleavings                          │
   │              │                                           │
   │              ▼                                           │
   │   6. cargo-fuzz (random inputs):                         │
   │      cargo fuzz run target                               │
   │      → Random fuzzing                                    │
   │              │                                           │
   │              ▼                                           │
   │   7. Clippy strict:                                      │
   │      #![warn(clippy::undocumented_unsafe_blocks)]        │
   │      → Enforce SAFETY comments                           │
   │              │                                           │
   │              ▼                                           │
   │   8. Peer review                                         │
   │      Every unsafe block = human audit                    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   📌 Test với miri là NHẤT THIẾT cho unsafe code.
      Catch nhiều UB compiler không thấy được.
```

---

## 19. Antipatterns visualized

```
   ❌ 1. unsafe để bypass borrow checker
   ──────────────────────────────────────
   
   let mut v = vec![1, 2, 3];
   let r1 = unsafe { &mut *(&mut v[0] as *mut i32) };
   let r2 = unsafe { &mut *(&mut v[1] as *mut i32) };
   // Compile OK but UB (2 &mut to overlapping)
   
   ✅ Use safe API:
   let (r1, rest) = v.split_at_mut(1);
   let r2 = &mut rest[0];
   
   
   ❌ 2. Misaligned read
   ─────────────────────
   
   let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8];
   let p = bytes.as_ptr().add(1) as *const u64;
   let v = unsafe { *p };   // ❌ misaligned u64 read
   
   ✅ Use read_unaligned:
   let v = unsafe { ptr::read_unaligned(p) };
   
   
   ❌ 3. Drop after free
   ─────────────────────
   
   let b = Box::new(42);
   let p = Box::into_raw(b);
   unsafe { drop(Box::from_raw(p)); }   // free
   let v = unsafe { *p };                // ❌ use-after-free
   
   
   ❌ 4. Wrong Send/Sync impl
   ──────────────────────────
   
   struct Bad { ptr: *mut u32 }
   unsafe impl Send for Bad {}   // ❌ proved nothing
   unsafe impl Sync for Bad {}
   
   ⟹ Caller might share Bad across threads without sync
   ⟹ Mutation race in safe-looking code
   
   ✅ Only impl if data structure ACTUALLY safe
   
   
   ❌ 5. Panic across FFI
   ──────────────────────
   
   #[no_mangle]
   pub extern "C" fn cb() {
       panic!("oops");   // ❌ UB across FFI
   }
   
   ✅ Catch unwind:
   #[no_mangle]
   pub extern "C" fn cb() -> i32 {
       std::panic::catch_unwind(|| {
           risky();
           0
       }).unwrap_or(-1)
   }
   
   
   ❌ 6. transmute lifetime extension
   ──────────────────────────────────
   
   let s = String::from("hi");
   let r: &'static str = unsafe { mem::transmute(s.as_str()) };
   drop(s);
   println!("{}", r);   // ❌ UB
   
   ⟹ NEVER use transmute to extend lifetimes
```

---

## 20. Mind map cuối

```
                          UNSAFE RUST
                                │
        ┌───────────┬───────────┼───────────┬─────────────┐
        ▼           ▼           ▼           ▼             ▼
   5 SUPER-     RAW         MEMORY       FFI         TOOLS
   POWERS       POINTERS     MODEL                    
        │           │           │           │             │
   deref raw    *const T    UnsafeCell  extern "C"   miri
   call unsafe  *mut T      atomic       #[repr(C)]   sanitizers
   mut static   alignment   ordering    bindgen      loom
   unsafe trait pointer     Send/Sync   cbindgen     cargo-fuzz
   union        arith       MaybeUninit panic catch_unwind
                                        CString/CStr
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. UB ≠ crash. UB = anything.       │
                │                                      │
                │  2. Wrap unsafe in safe API          │
                │                                      │
                │  3. Document SAFETY invariants       │
                │                                      │
                │  4. UnsafeCell is foundation         │
                │                                      │
                │  5. Atomic ordering: SeqCst default, │
                │     optimize after profile           │
                │                                      │
                │  6. Send/Sync: unsafe impl ONLY if   │
                │     can prove                        │
                │                                      │
                │  7. #[repr(C)] cho FFI               │
                │                                      │
                │  8. MaybeUninit thay uninitialized   │
                │                                      │
                │  9. transmute = nuclear option       │
                │                                      │
                │  10. miri + sanitizers + loom        │
                │      MANDATORY cho unsafe code       │
                │                                      │
                │  11. Avoid panic across FFI          │
                │                                      │
                │  12. Audit every unsafe block        │
                └──────────────────────────────────────┘
```

---

## 21. Bộ tài liệu Rust giờ có 14 chủ đề

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   1. memory-model            — Bộ nhớ                    │
   │   2. ownership-borrowing     — Sở hữu cơ bản            │
   │   3. trait                   — Polymorphism             │
   │   4. generic                 — Parametric polymorphism  │
   │   5. closure                 — Function as value        │
   │   6. async                   — Concurrency              │
   │   7. error-handling          — Error handling           │
   │   8. macros                  — Macros                   │
   │   9. smart-pointers          — Smart pointers            │
   │  10. lifetime                — Lifetime deep dive       │
   │  11. performance             — Profile & optimize       │
   │  12. observability           — Logs/Traces/Metrics      │
   │  13. iterator                — Iterator + Stream + Rayon │
   │  14. unsafe-rust             — Unsafe + FFI + Atomic    │
   │      unsafe-rust-visual      ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust system programming ĐẦY ĐỦ          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau unsafe Rust, có thể đào sâu nhánh thực hành:

- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 14 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time (dùng nhiều unsafe!)

Báo cái nào muốn đào sâu! 🦀⚡
