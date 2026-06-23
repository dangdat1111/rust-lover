# Smart Pointers Rust — Minh Hoạ Trực Quan

> Companion visual cho [smart-pointers.md](./smart-pointers.md). Đọc song song.

---

## 1. Bức tranh lớn — Smart Pointer Universe

```
                       SMART POINTERS RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   OWNERSHIP                  INTERIOR MUTABILITY       │
       │   ─────────                  ──────────────────────    │
       │                                                        │
       │   Box<T>      single owner   Cell<T>      Copy only    │
       │      │        heap alloc        │         single-thread│
       │      │                          │                      │
       │   Rc<T>       multi-owner    RefCell<T>   any T        │
       │      │        single thread     │         runtime check│
       │      │                          │                      │
       │   Arc<T>      multi-owner    Mutex<T>     thread-safe  │
       │      │        multi-thread      │         mutate       │
       │      │                          │                      │
       │   Weak<T>     non-owning     RwLock<T>    many readers │
       │              break cycle                  one writer   │
       │                                                        │
       │   NÂNG CAO:                                            │
       │   Cow<T>      borrow/owned   atomic types  lock-free   │
       │   OnceCell    init 1 lần     tokio::sync::* async      │
       │   LazyLock    global lazy    Pin<P>        no-move     │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Box<T> — Memory layout

```
   Code:
   ─────
   let b: Box<i32> = Box::new(5);
   
   
   Stack frame                    Heap
   ┌───────────────────┐         ┌──────────┐
   │ b: Box<i32>       │         │   5      │  ← data
   │   ptr ────────────┼────────►│          │
   │ (8 byte)          │         │ (4 byte) │
   └───────────────────┘         └──────────┘
   
   sizeof(Box<T>) = 8 byte (1 pointer)
   sizeof(actual data) tuỳ T
   
   
   Lifecycle:
   ──────────
   Box::new(5)  ──► malloc(4) ──► *ptr = 5 ──► b giữ ptr
       │
       └─ b ra khỏi scope ──► drop ──► free(ptr) ──► heap freed
```

---

## 3. Box<T> — 4 use cases

```
   1. dyn Trait (unsized type)
   ────────────────────────────
   ┌────────────────────────────────┐
   │ fn get(...) -> Box<dyn Animal> │
   │ {                              │
   │   if x { Box::new(Dog) }       │
   │   else { Box::new(Cat) }       │
   │ }                              │
   └────────────────────────────────┘
   sizeof(dyn Animal) = unknown
   → Box<dyn Animal> = fat pointer 16 byte
   
   
   2. Recursive type
   ─────────────────
   ┌──────────────────────────────┐
   │ enum List {                  │
   │   Cons(i32, Box<List>),  ◄──┤
   │   Nil,                       │  Box → size cố định
   │ }                            │  Không có Box → vô hạn
   └──────────────────────────────┘
   
   sizeof(List) = sizeof(tag) + max(
     sizeof(i32) + sizeof(Box) = 4 + 8 = 12,
     0
   ) = 16 (with alignment)
   
   
   3. Move large value rẻ
   ───────────────────────
   struct BigThing { data: [u8; 1_000_000] }   // 1 MB
   
   fn process(t: BigThing)        // move = copy 1MB (slow)
   fn process(t: Box<BigThing>)   // move = copy 8 byte (fast)
   
   
   4. Box<dyn Trait> trong Vec
   ────────────────────────────
   Vec<Box<dyn Animal>>
   ┌──────┬──────┬──────┐
   │ Box1 │ Box2 │ Box3 │  ← Vec items have fixed size (16 byte each)
   └──┬───┴──┬───┴──┬───┘
      ▼      ▼      ▼
     Dog    Cat    Bird  ← actual values on heap
```

---

## 4. Rc<T> — Reference counting

```
   Code:
   ─────
   let a = Rc::new(5);
   let b = Rc::clone(&a);
   let c = Rc::clone(&a);
   
   
   Stack:                          Heap:
   ┌──────────────┐               ┌──────────────────────┐
   │ a: Rc<i32>   │               │ RcBox {              │
   │   ptr ───────┼──────────────►│   strong: 3   ◄──┐   │
   └──────────────┘               │   weak:   1      │   │
   ┌──────────────┐               │   data:   5      │   │
   │ b: Rc<i32>   │   ┌──────────►│ }                │   │
   │   ptr ───────┼───┘           └──────────────────┴───┘
   └──────────────┘
   ┌──────────────┐
   │ c: Rc<i32>   │   ┌──────────►(same heap block)
   │   ptr ───────┼───┘
   └──────────────┘
   
   Tất cả 3 trỏ vào 1 block.
   strong=3 → drop a, b, c lần lượt → counter 3→2→1→0 → free.
   
   
   Sau Rc::clone:
   ───────────────
   strong += 1  (non-atomic increment, ~1ns)
   
   Sau drop:
   ──────────
   strong -= 1
   if strong == 0:
       drop data (call T's Drop)
       if weak == 0: dealloc block
```

---

## 5. Rc<T> immutable + Cycle leak

```
   ❌ Cycle problem:
   
   struct Node { next: RefCell<Option<Rc<Node>>> }
   
   let a = Rc::new(Node { next: RefCell::new(None) });
   let b = Rc::new(Node { next: RefCell::new(Some(Rc::clone(&a))) });
   *a.next.borrow_mut() = Some(Rc::clone(&b));
   
   
   Memory state:
   ─────────────
   
   ┌─────────────────────┐       ┌─────────────────────┐
   │ Node a              │       │ Node b              │
   │   strong: 2     ──┐ │       │   strong: 2  ◄──┐  │
   │   next: Some ─────┼─┼──────►│   next: Some───┼──┘
   │   ...             │ │       │   ...           │
   └───────────────────┴─┘       └─────────────────┘
        ▲                                  ▲
        │                                  │
        │ (when a/b drop from stack:       │
        │  strong=2 → 1, still > 0)        │
        │                                  │
        └─────  CYCLE — never reach 0 ─────┘
   
   ⟹ Heap blocks NEVER freed = LEAK
   
   
   ✅ Fix with Weak (Tầng 5)
```

---

## 6. Arc<T> — Atomic version

```
   Khác biệt cấu trúc với Rc:
   ──────────────────────────
   
   Rc<T>:                              Arc<T>:
   ┌────────────────┐                  ┌────────────────────┐
   │ ArcBox {       │                  │ ArcInner {         │
   │   strong:usize │                  │   strong:Atomic Usize│
   │   weak:  usize │                  │   weak:  Atomic Usize│
   │   data:  T     │                  │   data:  T         │
   │ }              │                  │ }                  │
   └────────────────┘                  └────────────────────┘
   
   
   Clone behavior:
   ────────────────
   
   Rc:  strong += 1            ← single instruction (~1ns)
   Arc: strong.fetch_add(1, Relaxed)  ← atomic instruction (~10-20ns)
                ↑
       Cache coherence: all CPU cores see update
       → LOCK BUS / invalidate cache line
       → ~5-20x slower than Rc
   
   
   Multi-thread example:
   ─────────────────────
   
        Thread 1                Thread 2                Thread 3
           │                        │                      │
           ├─Arc::clone(&data)──┐   │                      │
           │                    │   ├─Arc::clone(&data)──┐ │
           │                    │   │                    │ │
           │                    ▼   ▼                    ▼ ▼
                            ┌─────────────────────────────────┐
                            │ ArcInner { strong: 3, data: ... }│
                            └─────────────────────────────────┘
                            ALL 3 threads can read concurrently
   
   ⟹ Arc<T> implements Send + Sync khi T: Send + Sync
```

### Arc::clone / Arc::drop — Memory ordering (vì sao đúng)

```
   ┌────────────────────────────────────────────────────────────────┐
   │ CLONE — chỉ cần Relaxed                                         │
   ├────────────────────────────────────────────────────────────────┤
   │   self.strong.fetch_add(1, Relaxed)                            │
   │                                                                │
   │   "Tôi đã đang giữ 1 Arc → object CÒN SỐNG → không ai free     │
   │    được lúc này. Tôi chỉ ĐẾM, không đọc/ghi data."             │
   │   ⟹ không cần đồng bộ hoá vùng nhớ nào → memory barrier = 0    │
   └────────────────────────────────────────────────────────────────┘

   ┌────────────────────────────────────────────────────────────────┐
   │ DROP — Release khi giảm, Acquire trước khi free                │
   ├────────────────────────────────────────────────────────────────┤
   │   if strong.fetch_sub(1, Release) != 1 { return }  ← chưa cuối  │
   │   fence(Acquire)                                   ← là cuối    │
   │   drop(data); free(heap)                                        │
   └────────────────────────────────────────────────────────────────┘


   Vì sao cặp Release/Acquire là BẮT BUỘC:
   ───────────────────────────────────────

   Thread A                          Thread B (giảm cuối → 0)
   ─────────                         ───────────────────────
   ghi data .........
   fetch_sub(Release) ──────┐
                            │  synchronizes-with
                            └───────► fence(Acquire)
                                          │ happens-before
                                          ▼
                                      destructor(data)  ← THẤY ghi của A ✓
                                      free(heap)         ← an toàn ✓

   Thiếu Release/Acquire → CPU có thể free TRƯỚC khi A ghi xong
                          → use-after-free / double-free 💥


   Toàn cảnh ordering trong vòng đời refcount:
   ───────────────────────────────────────────
   Arc::clone      strong.fetch_add(1, Relaxed)       ← rẻ nhất
   Arc::drop       strong.fetch_sub(1, Release)        ← công bố
                   + fence(Acquire) nếu về 0           ← thu nhận
   Weak::clone     weak.fetch_add(1, Relaxed)
   Weak::upgrade   strong.compare_exchange(...)        ← CAS: chỉ +1 nếu > 0
   overflow guard  old > MAX_REFCOUNT → process::abort()


   Rc vs Arc — cùng layout, khác counter:
   ──────────────────────────────────────
   Rc::clone   strong += 1            (Cell<usize>,  non-atomic, ~1ns)
   Arc::clone  strong.fetch_add(...)  (AtomicUsize,  atomic,    ~10-20ns)
        cả hai: copy 8B con trỏ, KHÔNG copy data → O(1) theo sizeof(T)
```

---

## 7. Rc vs Arc — Khi nào dùng cái nào?

```
                    Cần share giữa các THREAD?
                              │
                       ┌──────┴──────┐
                      YES            NO
                       │              │
                       ▼              ▼
                     Arc<T>         Rc<T>
                                    (1.5-3x nhanh hơn)
   
   
   ┌─────────────────┬──────────┬──────────┐
   │ Aspect          │ Rc       │ Arc      │
   ├─────────────────┼──────────┼──────────┤
   │ Thread safe?    │ ❌        │ ✅        │
   │ Send + Sync?    │ ❌        │ ✅ (nếu T) │
   │ Counter type    │ usize    │ Atomic   │
   │ Clone cost      │ ~1ns     │ ~10-20ns │
   │ Drop cost       │ ~1ns     │ ~10-20ns │
   │ Deref cost      │ ~1ns     │ ~1ns     │
   │ Use case        │ AST,GUI  │ Web srv, │
   │                 │ parser   │ threads  │
   └─────────────────┴──────────┴──────────┘
   
   📌 Quy tắc thực dụng: NẾU KHÔNG CHẮC, DÙNG Arc.
      Performance penalty nhỏ < cost refactor sau này.
```

---

## 8. Weak<T> — Phá vỡ cycle

```
   ✅ Fix với Weak:
   
   struct Parent { children: RefCell<Vec<Rc<Child>>> }
   struct Child  { parent: RefCell<Weak<Parent>>, name: String }
   
   
   Memory:
   ───────
   
   ┌────────────────────────────┐
   │ Parent                     │
   │   strong: 1                │
   │   weak: 2                  │  ← children có 2 Weak trỏ về
   │   children: [Rc, Rc] ──┐   │
   └───────────────────────┼─┬─┘
                           │ │
        ┌──────────────────┘ │
        │                    │
        ▼                    ▼
   ┌──────────┐         ┌──────────┐
   │ Child 1  │         │ Child 2  │
   │ strong:1 │         │ strong:1 │
   │ parent:  │         │ parent:  │
   │  Weak ──►(non-owning)        │
   └──────────┘         └──────────┘
   
   Khi Parent ra khỏi scope:
   ─────────────────────────
   parent.strong → 0
   → drop Parent (drop children list)
   → mỗi Rc<Child> strong → 0
   → drop Child
   → Weak<Parent> không ngăn drop ✅
   
   Heap fully freed. KHÔNG LEAK.
   
   
   Weak::upgrade flow:
   ───────────────────
   
   weak.upgrade() ──►  Option<Rc<T>>
                       │
                ┌──────┴───────┐
                │              │
            strong > 0       strong = 0
            (still alive)    (already dropped)
                │              │
                ▼              ▼
            Some(Rc<T>)     None
            (tăng strong)
```

---

## 9. Memory block lifecycle với Weak

```
   ┌─────────────────────────────────────────────────────────┐
   │                  Lifecycle                              │
   │                                                         │
   │   strong > 0   weak ≥ 0                                 │
   │   ┌─────────────────────┐                               │
   │   │ Block alive,        │                               │
   │   │ data accessible     │                               │
   │   └──────────┬──────────┘                               │
   │              │ drop strong=0                            │
   │              ▼                                          │
   │   strong = 0  weak > 0                                  │
   │   ┌─────────────────────┐                               │
   │   │ Data dropped        │                               │
   │   │ but counters alive  │ ← Weak::upgrade returns None  │
   │   └──────────┬──────────┘                               │
   │              │ drop weak=0                              │
   │              ▼                                          │
   │   ┌─────────────────────┐                               │
   │   │ Block fully freed   │                               │
   │   └─────────────────────┘                               │
   └─────────────────────────────────────────────────────────┘
```

---

## 10. Interior Mutability — Bẻ cong quy tắc

```
   ┌─────────────────────────────────────────────────────┐
   │ QUY TẮC BÌNH THƯỜNG (compile-time check):           │
   │                                                     │
   │   1 &mut T  OR  N &T   tại bất kỳ thời điểm        │
   │                                                     │
   │   Không có cách nào mutate qua &T                   │
   └─────────────────────────────────────────────────────┘
   
                              ↓
              Đôi khi bạn có &T (vd: &self method)
              nhưng cần mutate field bên trong
                              ↓
              
   ┌─────────────────────────────────────────────────────┐
   │ INTERIOR MUTABILITY:                                │
   │                                                     │
   │ &T  ─────► .borrow_mut() ─────► &mut Inner          │
   │       (qua UnsafeCell + runtime/atomic check)       │
   │                                                     │
   │ Compile-time relaxed, runtime check thay thế        │
   └─────────────────────────────────────────────────────┘
   
   
   Các type chọn lựa:
   ──────────────────
   
   ┌────────────┬─────────────┬─────────────┬──────────┐
   │ Type       │ Thread?     │ Cost        │ T bound  │
   ├────────────┼─────────────┼─────────────┼──────────┤
   │ Cell       │ Single      │ Rất rẻ      │ Copy     │
   │ RefCell    │ Single      │ Rẻ          │ Any      │
   │ Mutex      │ Multi       │ ~10-20ns    │ Any      │
   │ RwLock     │ Multi       │ ~20-30ns    │ Any      │
   │ Atomic*    │ Multi       │ ~1-10ns     │ Primitive│
   │ OnceCell   │ Single      │ Rẻ          │ Any      │
   │ OnceLock   │ Multi       │ Trung bình   │ Any      │
   └────────────┴─────────────┴─────────────┴──────────┘
```

---

## 11. Cell<T> vs RefCell<T>

```
   Cell<T>:  Get COPY, không borrow
   ──────────────────────────────────
   
   let c = Cell::new(5);
   
   c.set(10);
   let v = c.get();     ← COPY (i32 Copy)
   // v và c riêng biệt, modify một không affect cái kia
   
   
   Memory:
   ┌────────────────┐
   │ Cell<i32>      │
   │   value: 5     │  ← UnsafeCell bên trong
   └────────────────┘
   
   API:
   • get() → T (COPY)
   • set(v: T)
   • take() → T (replace with Default)
   • replace(v: T) → T (old)
   • swap(other) — swap với cell khác
   
   T phải Copy hoặc dùng take/replace cho non-Copy
   
   
   ───────────────────────────────────────────────────────
   
   
   RefCell<T>: Borrow ra reference (track runtime)
   ────────────────────────────────────────────────
   
   let r = RefCell::new(vec![1, 2, 3]);
   
   {
     let b = r.borrow();           ← Ref<T> = &T
     println!("{:?}", *b);
   }  ← drop b
   
   {
     let mut b = r.borrow_mut();   ← RefMut<T> = &mut T
     b.push(4);
   }  ← drop b
   
   
   Memory:
   ┌────────────────────┐
   │ RefCell<Vec<i32>>  │
   │   flag: -1/0/N     │  ← BorrowFlag: -1=writer, 0=free, N=readers
   │   value: Vec       │  ← UnsafeCell
   └────────────────────┘
   
   BORROW RULES (runtime):
   ───────────────────────
   borrow():
     if flag >= 0: flag += 1; return Ref
     else:        PANIC ("already borrowed mutably")
   
   borrow_mut():
     if flag == 0: flag = -1; return RefMut
     else:         PANIC ("already borrowed")
```

---

## 12. RefCell PANIC scenarios

```
   ❌ Scenario 1: Mutable + Immutable cùng lúc
   ─────────────────────────────────────────
   let r = RefCell::new(5);
   let _a = r.borrow();
   let _b = r.borrow_mut();   ← PANIC
   
   
   ❌ Scenario 2: Hai Mutable cùng lúc
   ─────────────────────────────────
   let r = RefCell::new(5);
   let _a = r.borrow_mut();
   let _b = r.borrow_mut();   ← PANIC
   
   
   ✅ Để tránh panic, dùng try_borrow:
   ──────────────────────────────────
   match r.try_borrow_mut() {
       Ok(mut b) => *b += 1,
       Err(_) => eprintln!("already borrowed"),
   }
   
   
   ✅ Hoặc scope explicit:
   ─────────────────────
   {
     let mut b = r.borrow_mut();
     *b += 1;
   }  ← drop b
   {
     let b = r.borrow();    ← OK
   }
```

---

## 13. Mutex<T> — Multi-thread mutation

```
   Code:
   ─────
   let data = Arc::new(Mutex::new(0));
   
   thread::spawn({
       let data = Arc::clone(&data);
       move || {
           let mut g = data.lock().unwrap();
           *g += 1;
       }   ← g drop → unlock tự động
   });
   
   
   Memory:
   ───────
   Stack:                Heap:
   ┌──────────────┐     ┌─────────────────────────┐
   │ data: Arc    │     │ ArcInner {              │
   │   ptr ───────┼────►│   strong: N             │
   └──────────────┘     │   weak: 1               │
                        │   data: Mutex<i32> {    │
                        │     poisoned: AtomicBool│
                        │     sys: pthread_mutex_t│
                        │     value: UnsafeCell<i32>│
                        │   }                     │
                        │ }                       │
                        └─────────────────────────┘
   
   
   Lock flow (Linux):
   ──────────────────
   
   lock():
     ┌─────────────────────────────────────┐
     │ Atomic CAS thử lấy lock             │
     └─────────┬───────────────────────────┘
               │
        ┌──────┴──────┐
       Success     Fail (held by other)
        │              │
        ▼              ▼
       ~10ns       futex syscall
       return      → thread sleep
                   → kernel wake when lock free
                   → ~µs
        │              │
        └──────┬───────┘
               ▼
          MutexGuard
   
   
   Guard drop:
   ───────────
   atomic unlock + maybe futex wake_one
```

---

## 14. Mutex Poisoning

```
   Thread 1                           Thread 2
   ─────────                          ─────────
       │                                  │
       ├─ lock() = Ok(g) ──┐              │
       │                   ▼              │
       │              modify state        │
       │              halfway done        │
       │                   │              │
       ├─ panic! ──────────┘              │
       │                                  │
       │ (Mutex marked POISONED)          │
       │                                  ├─ lock() = Err(PoisonError)
       │                                  │
   
   
   Recover:
   ────────
   match m.lock() {
       Ok(g) => g,
       Err(poisoned) => {
           // accept state có thể inconsistent
           poisoned.into_inner()
       }
   }
   
   ⚠️ Tranh cãi: poisoning có thực sự hữu ích?
      parking_lot::Mutex bỏ poisoning → nhanh + đơn giản hơn.
```

---

## 15. RwLock<T> — Many readers, one writer

```
   3 trạng thái:
   ─────────────
   
   ┌─────────────────────────────────┐
   │ FREE                            │
   │  → read.lock() OK               │
   │  → write.lock() OK              │
   └─────────────────────────────────┘
   
   ┌─────────────────────────────────┐
   │ READING (N readers)             │
   │  → read.lock() OK (N+1)         │
   │  → write.lock() BLOCK           │
   └─────────────────────────────────┘
   
   ┌─────────────────────────────────┐
   │ WRITING (1 writer)              │
   │  → read.lock() BLOCK            │
   │  → write.lock() BLOCK           │
   └─────────────────────────────────┘
   
   
   Visualization:
   ──────────────
   
   Time →
   
   Read1  ████████░░░░░░░░░░░░██████████░░░░░░
   Read2  ░░████████░░░░░░░░░░██████░░░░░░░░░░
   Read3  ░░░░░░░░██░░░░░░░░░░░░░░░░██████░░░░
   Write  ░░░░░░░░░░██████░░░░░░░░░░░░░░██░░░░
           ↑           ↑     ↑          ↑
           |           |     |          |
       3 reads     1 write   3 reads    1 write
        parallel    exclusive parallel  exclusive
   
   
   Mutex vs RwLock benchmark:
   ──────────────────────────
   
   Workload: 90% read, 10% write, 8 threads
                                  ╔═════════════╗
   Mutex   ████████░░░░░░░░░░░    ║ ~X tps     ║
   RwLock  ████████████████████   ║ ~5X-8X tps ║  ← much better
                                  ╚═════════════╝
   
   Workload: 50% read, 50% write
                                  ╔═════════════╗
   Mutex   ████████████░░░░░░░    ║ ~X tps     ║
   RwLock  █████████████░░░░░░    ║ ~0.8X tps  ║  ← OVERHEAD, worse!
                                  ╚═════════════╝
   
   📌 Đo bằng benchmark cho workload cụ thể.
```

---

## 16. Combinations — Sơ đồ matrix

```
   ┌─────────────────────────────────────────────────────────────┐
   │                  COMBINATIONS                               │
   │                                                             │
   │              Single-thread       Multi-thread               │
   │   ┌──────────────────────────────────────────────────────┐  │
   │   │                                                      │  │
   │   │ Immutable │  Rc<T>           │  Arc<T>              │  │
   │   │ shared    │                  │                       │  │
   │   ├───────────┼──────────────────┼───────────────────────┤  │
   │   │ Mutable   │  Rc<RefCell<T>>  │  Arc<Mutex<T>>        │  │
   │   │ shared    │  Rc<Cell<T>>     │  Arc<RwLock<T>>       │  │
   │   │           │  (Copy only)     │                       │  │
   │   ├───────────┼──────────────────┼───────────────────────┤  │
   │   │ Lazy init │  OnceCell<T>     │  OnceLock<T>          │  │
   │   │           │                  │  LazyLock<T>          │  │
   │   ├───────────┼──────────────────┼───────────────────────┤  │
   │   │ Async     │  N/A             │  Arc<tokio::sync::    │  │
   │   │ mutate    │                  │       Mutex<T>>       │  │
   │   │           │                  │  Arc<tokio::sync::    │  │
   │   │           │                  │       RwLock<T>>      │  │
   │   └──────────────────────────────────────────────────────┘  │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 17. Arc<Mutex<HashMap<...>>> deep visualization

```
   Pattern: User session store thread-safe
   
   Code:
   ─────
   type Sessions = Arc<Mutex<HashMap<String, Arc<User>>>>;
   
   let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
   
   
   Memory layout (web server với 4 worker threads):
   ───────────────────────────────────────────────
   
   Worker 1     Worker 2     Worker 3     Worker 4
      │            │            │            │
      └─Arc clone──┴───Arc clone┴───Arc clone┴────┐
                                                  ▼
   ┌───────────────────────────────────────────────────┐
   │ Heap: ArcInner {                                  │
   │   strong: 5  ◄── (4 workers + main)               │
   │   weak: 1                                         │
   │   data: Mutex<HashMap<String, Arc<User>>> {       │
   │     poisoned: false,                              │
   │     value: HashMap {                              │
   │       "session_1" => Arc<User> ─────┐             │
   │       "session_2" => Arc<User> ────┐│             │
   │     }                              ││             │
   │   }                                ││             │
   │ }                                  ││             │
   └────────────────────────────────────┼┼─────────────┘
                                        ▼▼
                                ┌─────────────┐
                                │ Arc<User>   │
                                │   strong: N │  ← multiple workers
                                │   data: ...  │     share user ref
                                └─────────────┘
   
   Worker handles request:
   ───────────────────────
   1. Arc::clone(&sessions)           ← rẻ (10ns)
   2. sessions.lock().unwrap()         ← lock map
   3. let user = sessions.get("id")?   ← Arc<User> clone
   4. drop guard                       ← unlock
   5. process(user).await              ← work with user (no lock!)
   
   📌 Key: lock map ngắn, user data ngoài lock.
```

---

## 18. tokio::sync::Mutex vs std::sync::Mutex

```
   std::sync::Mutex                    tokio::sync::Mutex
   ────────────────                    ──────────────────
   
   m.lock()                            m.lock().await
       │                                   │
       ▼                                   ▼
   ┌──────────────┐                   ┌──────────────┐
   │ Block thread │                   │ Yield to     │
   │ (futex sleep)│                   │ executor     │
   │              │                   │ Task suspends│
   └──────────────┘                   └──────────────┘
       ▲                                   ▲
       │                                   │
   Lock free                          Lock free
       │                                   │
       ▼                                   ▼
   Resume thread                      Wake task,
                                      poll resume
   
   
   ┌────────────────────────────────────────────────────────┐
   │ ❌ std::sync::Mutex giữ lock qua .await:               │
   │                                                        │
   │ let g = m.lock().unwrap();                             │
   │ other.await;        ← thread block! các task khác chờ │
   │ *g += 1;            ← bad design                      │
   │                                                        │
   │ ⟹ Guard !Send (Linux) → Future !Send → spawn fail     │
   └────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────┐
   │ ✅ Option 1: std Mutex, drop trước await:              │
   │                                                        │
   │ let snapshot = {                                       │
   │     let g = m.lock().unwrap();                         │
   │     g.clone()                                          │
   │ };                  ← drop g                          │
   │ other.await;        ← OK                              │
   │                                                        │
   │ ⟹ Nhanh, không overhead async                          │
   └────────────────────────────────────────────────────────┘
   
   
   ┌────────────────────────────────────────────────────────┐
   │ ✅ Option 2: tokio Mutex, can hold across await:       │
   │                                                        │
   │ let mut g = m.lock().await;                            │
   │ other.await;        ← OK, task yield khi lock contend │
   │ *g += 1;                                              │
   │                                                        │
   │ ⟹ Linh hoạt nhưng chậm hơn std Mutex                  │
   └────────────────────────────────────────────────────────┘
   
   
   Quy tắc:
   ────────
   ┌─────────────────────────────────────────────┐
   │ Cần giữ lock qua .await?                    │
   │                                             │
   │   NO  → std::sync::Mutex   (nhanh hơn)      │
   │   YES → tokio::sync::Mutex                  │
   └─────────────────────────────────────────────┘
```

---

## 19. Performance comparison chart

```
   Operation               Cost (nanoseconds, approximate)
   ────────────────────    ───────────────────────────────
   
   Box deref               ▏ ~1ns
   Rc clone                ▏ ~1ns
   Cell get                ▏ ~1ns
   Atomic load Relaxed     ▏ ~1ns
   
   RefCell borrow          ▎ ~2-3ns
   
   Atomic CAS              ▍ ~5-10ns
   
   Arc clone               ▍ ~10-20ns (atomic)
   Mutex lock (free)       ▍ ~10-20ns
   RwLock read (free)      ▌ ~20-30ns
   
   parking_lot::Mutex      ▎ ~3-5ns (faster than std)
   
   ───────────────────────────────────────
   
   Mutex lock (contended)  ████████ ~µs (futex syscall)
   RwLock writer wait      ████████ ~µs
   Network round-trip      ████████████████ ~ms
   
   📌 Cao gấp 1000x giữa các ranges!
   
   
   Khi nào lo về performance?
   ───────────────────────────
   • Hot loop: chạy hàng triệu lần/giây
   • High contention: nhiều thread cùng lock
   • Cache-bound: dùng nhiều Arc → cache line ping-pong
   
   Cách giải:
   ──────────
   • Hot path: tránh Arc nếu được, dùng &T
   • Contention: shard data, lock-free, channel
   • Cache: pad atomic với #[repr(align(64))]
```

---

## 20. Cow<T> — Clone on Write

```
   pub enum Cow<'a, T: ?Sized + ToOwned> {
       Borrowed(&'a T),
       Owned(<T as ToOwned>::Owned),
   }
   
   
   Example:
   ────────
   fn normalize(s: &str) -> Cow<str> {
       if s.chars().all(|c| c.is_ascii_lowercase()) {
           Cow::Borrowed(s)             ← KHÔNG alloc, return reference
       } else {
           Cow::Owned(s.to_lowercase())  ← alloc, return owned String
       }
   }
   
   normalize("hello")       ──► Cow::Borrowed (no alloc!)
   normalize("Hello World") ──► Cow::Owned (alloc String)
   
   
   Memory:
   ───────
   ┌────────────────────────────────┐
   │ Cow<'a, str>                   │
   │                                │
   │  Tag: Borrowed | Owned         │
   │  ┌──────────────────────────┐  │
   │  │ Borrowed: &str (16 byte) │  │
   │  │     OR                   │  │
   │  │ Owned: String (24 byte)  │  │
   │  └──────────────────────────┘  │
   │  ↑ enum chứa max(both) = 24    │
   │                                │
   │  Total ~32 byte                │
   └────────────────────────────────┘
   
   
   Use case kinh điển: serde deserialize
   ──────────────────────────────────────
   #[derive(Deserialize)]
   struct Config<'a> {
       #[serde(borrow)]
       name: Cow<'a, str>,
   }
   
   JSON "no escape" → Borrowed (zero alloc)
   JSON "with \"escape\"" → Owned (must alloc)
```

---

## 21. OnceCell / LazyLock — Lazy global

```
   OnceCell<T>:                       LazyLock<T>:
   ────────────                       ────────────
   Manual init                        Auto init on first access
   
   ┌──────────────┐                   ┌──────────────────┐
   │ OnceCell<T>  │                   │ LazyLock<T>      │
   │   slot: Option<T>                │   slot: OnceLock │
   │              │                   │   init: F        │
   └──────────────┘                   └──────────────────┘
   
   Use:                               Use:
   let c = OnceCell::new();           static R: LazyLock<Regex> =
   let v = c.get_or_init(|| ...);       LazyLock::new(|| Regex::new(...));
                                      
                                      R.is_match("123")   ← auto init lần đầu
   
   
   Memory lifecycle:
   ─────────────────
   
   Before init:  ┌──────────────┐
                 │ slot: None   │   ← chưa init
                 └──────────────┘
   
   After get_or_init: ┌──────────────┐
                       │ slot: Some(T)│  ← init xong, value cached
                       └──────────────┘
   
   Subsequent access: chỉ load Some(T), ~1ns
   
   
   ┌──────────────────────────────────────────────────┐
   │ Use cases:                                       │
   │                                                  │
   │ • Global Regex (compile expensive)               │
   │ • Global config loaded from file                 │
   │ • Database connection pool                       │
   │ • Lazy lookup table                              │
   │                                                  │
   │ Lưu ý: thay vì lazy_static! (macro cũ),          │
   │        nay dùng LazyLock (stable từ 1.80)        │
   └──────────────────────────────────────────────────┘
```

---

## 22. Decision tree — Senior workflow

```
                  Cần share state?
                          │
                     ┌────┴────┐
                    NO         YES
                     │          │
                     ▼          ▼
                   T thường   Multiple owners?
                                │
                          ┌─────┴─────┐
                         NO          YES
                          │           │
                          ▼           ▼
                    pass &T       Multi-thread?
                                       │
                                  ┌────┴────┐
                                 YES        NO
                                  │          │
                                  ▼          ▼
                                Arc<T>    Rc<T>
                                  │
                                  ▼
                              Mutate?
                                  │
                            ┌─────┴─────┐
                           NO          YES
                            │           │
                            ▼           ▼
                          Arc<T>    Read >> Write?
                                        │
                                   ┌────┴────┐
                                  YES        NO
                                   │          │
                                   ▼          ▼
                                Arc<RwLock<T>> Arc<Mutex<T>>
                                                    │
                                                    ▼
                                              Giữ qua .await?
                                                    │
                                              ┌─────┴─────┐
                                             NO          YES
                                              │           │
                                              ▼           ▼
                                          std::Mutex tokio::Mutex
   
   
   ┌──────────────────────────────────────────────────────────────┐
   │ NÂNG CAO — Alternatives:                                     │
   │                                                              │
   │ • atomic types: cho primitive (counter, flag)                │
   │ • channels (mpsc/oneshot): message passing thay shared state │
   │ • DashMap: lock per bucket cho HashMap concurrent            │
   │ • ArcSwap: lock-free atomic swap config                      │
   │ • parking_lot: Mutex/RwLock nhanh hơn std                    │
   └──────────────────────────────────────────────────────────────┘
```

---

## 23. Antipatterns visualization

```
   ❌ 1. Arc<Mutex<T>> nhưng T immutable
   ───────────────────────────────────────
   let cfg: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::load()));
                       ↑
                       Lock cho mỗi read! Phí phạm.
   
   ✅ Arc<T> đủ:
   let cfg: Arc<Config> = Arc::new(Config::load());
   
   
   ❌ 2. Lock giữ qua I/O
   ───────────────────────
   let g = m.lock();
   client.send(&*g).await;     ← lock held trong network call!
   process(&*g);
   
   ✅ Snapshot pattern:
   let snapshot = m.lock().clone();
   drop(snapshot);                  // hold ngắn
   client.send(&snapshot).await;
   
   
   ❌ 3. Cycle Rc/Arc + RefCell/Mutex không Weak
   ──────────────────────────────────────────────
   parent.children = vec![Rc::clone(&child)];
   child.parent = Some(Rc::clone(&parent));    ← cycle!
   
   ✅ Child holds Weak:
   child.parent = Rc::downgrade(&parent);
   
   
   ❌ 4. RefCell cho field nhỏ trong struct lớn
   ─────────────────────────────────────────────
   struct App {
       state: RefCell<HugeState>,   ← bug khả năng cao
   }
   
   ✅ Restructure data, hoặc smaller cells:
   struct App {
       counter: Cell<u64>,           ← chỉ nhỏ
       state: HugeState,             ← &mut bình thường
   }
   
   
   ❌ 5. Arc::clone trong hot loop
   ──────────────────────────────
   for item in items.iter() {
       let arc = Arc::clone(&shared);  ← 10ns × N items
       process(arc, item);
   }
   
   ✅ Pass reference:
   for item in items.iter() {
       process(&shared, item);
   }
```

---

## 24. Memory layout summary

```
   ┌──────────────────────────────────────────────────────────────┐
   │                  SMART POINTER SIZES                         │
   │                                                              │
   │   Type              Stack size  Heap overhead per instance   │
   │   ────              ──────────  ──────────────────────────   │
   │   &T                8 byte      0                            │
   │   &mut T            8 byte      0                            │
   │   &dyn Trait        16 byte     0 (data already exists)      │
   │   Box<T>            8 byte      0 (just data)                │
   │   Box<dyn Trait>    16 byte     0                            │
   │                                                              │
   │   Rc<T>             8 byte      16 byte (2 usize counters)   │
   │   Arc<T>            8 byte      16 byte (2 atomic usize)     │
   │   Weak<T>           8 byte      shared with Rc/Arc           │
   │                                                              │
   │   Cell<T>           sizeof(T)   0                            │
   │   RefCell<T>        sizeof(T)+8 0  (8 = borrow flag)         │
   │                                                              │
   │   Mutex<T>          sizeof(T)+? 0  (? = pthread_mutex_t,     │
   │                                       ~40 byte on Linux)     │
   │   RwLock<T>         sizeof(T)+? 0  (? ~56 byte on Linux)     │
   │                                                              │
   │   Cow<'a, T>        max(&T,T)   0/sizeof(T) tuỳ variant      │
   │                                                              │
   │   atomic types      sizeof(T)   0                            │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 25. Mind map cuối

```
                          SMART POINTERS
                                │
        ┌────────────┬──────────┼──────────┬────────────┐
        ▼            ▼          ▼          ▼            ▼
    OWNERSHIP   INTERIOR    LOCK-FREE   NÂNG CAO   ASYNC
                MUTABILITY  /ATOMIC
        │            │          │          │            │
      Box        Cell        atomic     Cow         tokio::sync
      Rc         RefCell     types      OnceCell      ::Mutex
      Arc        Mutex                  LazyLock      ::RwLock
      Weak       RwLock                 Pin           ::Semaphore
                                                      ::channels
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. Smart pointer = ownership tool   │
                │     không phải GC giả                │
                │                                      │
                │  2. Cost matter: từ ~1ns (Box, Rc)   │
                │     đến ~µs (contended Mutex)        │
                │                                      │
                │  3. Single vs multi-thread tách rõ  │
                │     Rc/RefCell  vs  Arc/Mutex        │
                │                                      │
                │  4. Cycle = leak. Weak = fix.        │
                │                                      │
                │  5. Lock duration < I/O duration     │
                │     ALWAYS                           │
                │                                      │
                │  6. async lock chỉ khi cần           │
                │     std::Mutex nhanh hơn nhiều       │
                │                                      │
                │  7. Channel/message thường > lock    │
                │                                      │
                │  8. Đo bằng criterion, không đoán    │
                └──────────────────────────────────────┘
```

---

## 26. Bộ tài liệu Rust giờ có 9 chủ đề

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   1. memory-model            — Bộ nhớ                    │
   │   2. ownership-borrowing     — Sở hữu                    │
   │   3. trait                   — Polymorphism             │
   │   4. generic                 — Parametric polymorphism  │
   │   5. closure                 — Function as value        │
   │   6. async                   — Concurrency              │
   │   7. error-handling          — Error handling           │
   │   8. macros                  — Macros                   │
   │   9. smart-pointers          — Smart pointers            │
   │      smart-pointers-visual     ← VỪA HOÀN THÀNH          │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 18 files, ~1 MB tài liệu                         │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust senior đã đầy đủ                    │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau smart pointers, có thể đi tiếp các nhánh chuyên sâu:

- **Unsafe Rust** — raw pointer, UnsafeCell deep, atomic ordering, FFI với C/C++
- **Iterator deep dive** — implement Iterator trait, lazy evaluation, rayon parallel
- **Testing patterns** — unit, integration, proptest, criterion bench (đo smart pointer cost)
- **Logging & Observability** — tracing nâng cao, OpenTelemetry, metrics
- **Web framework realistic** — apply tất cả 9 chủ đề vào axum project
- **Database** — sqlx, connection pool với Arc, transaction patterns

Báo cái nào muốn đào sâu! 🦀⚡
