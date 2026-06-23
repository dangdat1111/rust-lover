# Smart Pointers trong Rust — Deep Dive

> Tài liệu thứ 9 trong bộ Rust nền tảng. Đọc sau khi đã quen:
> - [memory-model.md](./memory-model.md) — stack/heap
> - [ownership-borrowing.md](./ownership-borrowing.md) — quy tắc sở hữu
> - [trait.md](./trait.md) — Deref, Drop là traits
> - [async.md](./async.md) — Mutex async khác sync
>
> **Smart pointers** là các type sở hữu dữ liệu trên heap (hoặc với "siêu năng lực" khác) và quản lý lifetime tự động qua `Drop`.
> Chúng là **vũ khí mở rộng** của Rust ownership system — cho phép shared ownership, interior mutability, thread-safe access.
> Tài liệu này đào sâu memory layout, trade-off, và khi nào dùng cái nào.

---

# Mục lục

- [Tầng 1: Smart Pointer là gì?](#tầng-1-smart-pointer-là-gì)
- [Tầng 2: Box<T> — Heap allocation đơn giản nhất](#tầng-2-boxt--heap-allocation-đơn-giản-nhất)
- [Tầng 3: Rc<T> — Reference counting single-thread](#tầng-3-rct--reference-counting-single-thread)
- [Tầng 4: Arc<T> — Atomic Rc cho multi-thread](#tầng-4-arct--atomic-rc-cho-multi-thread)
- [Tầng 5: Weak<T> — Phá vỡ cycle](#tầng-5-weakt--phá-vỡ-cycle)
- [Tầng 6: Interior Mutability — Bẻ cong quy tắc borrow](#tầng-6-interior-mutability--bẻ-cong-quy-tắc-borrow)
- [Tầng 7: Cell<T> và RefCell<T>](#tầng-7-cellt-và-refcellt)
- [Tầng 8: Mutex<T> — Thread-safe mutation](#tầng-8-mutext--thread-safe-mutation)
- [Tầng 9: RwLock<T> — Many readers, one writer](#tầng-9-rwlockt--many-readers-one-writer)
- [Tầng 10: Combinations — Rc<RefCell<T>>, Arc<Mutex<T>>](#tầng-10-combinations--rcrefcellt-arcmutext)
- [Tầng 11: Smart pointers nâng cao — Cow, OnceCell, LazyLock](#tầng-11-smart-pointers-nâng-cao--cow-oncecell-lazylock)
- [Tầng 12: Async smart pointers — tokio::sync](#tầng-12-async-smart-pointers--tokiosync)
- [Tầng 13: Decision tree — Khi nào dùng cái nào](#tầng-13-decision-tree--khi-nào-dùng-cái-nào)

---

# Tầng 1: Smart Pointer là gì?

## 1.1 Định nghĩa

**Smart pointer** = type **giống pointer** (có thể deref ra dữ liệu) nhưng **thông minh hơn**:

- Sở hữu (own) dữ liệu — gọi `Drop` khi out of scope
- Có thêm "siêu năng lực": shared ownership, interior mutability, thread safety, lazy init, ...

So với **reference** (`&T`, `&mut T`):

| Aspect | Reference | Smart Pointer |
|--------|-----------|---------------|
| Sở hữu dữ liệu? | ❌ Không | ✅ Có (thường) |
| Auto Drop? | ❌ | ✅ |
| Trỏ vào heap? | Có thể | Hầu hết là có |
| Trait đặc biệt | `Deref`, `Copy` | `Deref`, `Drop`, ... |

## 1.2 Tại sao gọi là "smart"?

Vì chúng có **logic** chứ không chỉ là raw address:

- `Box<T>`: alloc heap khi tạo, dealloc khi drop
- `Rc<T>`: tăng counter khi clone, giảm khi drop, dealloc khi counter = 0
- `Mutex<T>`: lock khi access, unlock khi guard drop
- `Cow<T>`: clone-on-write — chỉ alloc khi muốn modify

## 1.3 Deref và DerefMut — Bí mật cú pháp

Smart pointer "trông giống reference" nhờ trait `Deref`:

```rust
pub trait Deref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}
```

Khi bạn viết `*box_x` hoặc `box_x.method()`, compiler tự gọi `deref()` để get `&T`.

```rust
let b = Box::new(5);
let v = *b;          // *b = *(b.deref()) = *(&5) = 5
let s = b.to_string(); // b.to_string() = (b.deref()).to_string()
```

→ Smart pointer **transparently** acts như reference.

## 1.4 Drop trait — Cleanup tự động

```rust
pub trait Drop {
    fn drop(&mut self);
}
```

Khi value out of scope, compiler chèn `drop()` call → resource cleanup tự động (RAII).

```rust
struct MyResource;
impl Drop for MyResource {
    fn drop(&mut self) { println!("cleaning up"); }
}

fn main() {
    let _r = MyResource;
}  // Tự động in "cleaning up"
```

## 1.5 Tổng quan các smart pointers cốt lõi

```
                       SMART POINTERS RUST
                              │
       ┌──────────────────────┼──────────────────────┐
       │                      │                      │
   OWNERSHIP            INTERIOR MUTABILITY      OTHER
       │                      │                      │
   Box<T>                Cell<T> (Copy)         Cow<T>
   Rc<T>                 RefCell<T>             OnceCell<T>
   Arc<T>                Mutex<T>               LazyLock<T>
   Weak<T>               RwLock<T>              Pin<P>
                         atomic types
                         tokio::sync::*
```

---

# Tầng 2: Box<T> — Heap allocation đơn giản nhất

## 2.1 Box<T> là gì?

`Box<T>` là cách **đơn giản nhất** để alloc trên heap. Không sharing, không reference counting — chỉ owned heap pointer.

```rust
let b = Box::new(5);     // alloc 4 byte trên heap, lưu 5
println!("{}", b);        // 5
// b drops at end → heap memory freed
```

## 2.2 Memory layout

```
Stack:                  Heap:
┌──────────────┐       ┌──────────────┐
│ b: Box<i32>  │       │      5       │
│   ptr ───────┼──────►│              │
│              │       │              │
└──────────────┘       └──────────────┘
   8 byte                 4 byte
```

`Box<T>` trên stack chỉ là **1 pointer** (8 byte trên 64-bit). Dữ liệu thực ở heap.

## 2.3 Use cases

### Use case 1: Type có size không biết tại compile time

```rust
trait Animal { fn speak(&self); }

fn get_animal(kind: u8) -> Box<dyn Animal> {  // dyn → size không biết
    if kind == 0 { Box::new(Dog) } else { Box::new(Cat) }
}
```

`dyn Trait` không sized → phải Box.

### Use case 2: Recursive type

```rust
// ❌ ERROR: size vô hạn
enum List {
    Cons(i32, List),  // recursive
    Nil,
}

// ✅ OK
enum List {
    Cons(i32, Box<List>),  // Box có size cố định
    Nil,
}
```

Vì `Box<List>` có size cố định (1 pointer), enum có size cố định.

### Use case 3: Move large value rẻ hơn

```rust
struct BigThing { data: [u8; 1_000_000] }

fn process(t: BigThing) { ... }       // move = copy 1MB
fn process_boxed(t: Box<BigThing>) { ... } // move = copy 8 byte

let x = BigThing { ... };
process(x);                            // copy 1MB
process_boxed(Box::new(x));            // alloc heap rồi copy 8 byte
```

### Use case 4: Trait object như field

```rust
struct Animals {
    list: Vec<Box<dyn Animal>>,
}
```

Vec phải có size cố định cho mỗi phần tử — `dyn Animal` không sized → Box.

## 2.4 Box::new là alloc syscall không?

Không trực tiếp. `Box::new` gọi allocator (mặc định: system allocator → `malloc`/`free` trên POSIX, `HeapAlloc` trên Windows).

```rust
let b = Box::new(5);
// Đại loại:
// 1. allocator.alloc(Layout::new::<i32>()) → ptr
// 2. *ptr = 5
// 3. b = Box { ptr }
```

Cost: 1 syscall (cho lần alloc) + 1 init. Với type lớn hoặc alloc tần suất cao, cân nhắc:
- `Vec::with_capacity` để preallocate
- `bumpalo` arena allocator
- `jemalloc`/`mimalloc` global allocator

## 2.5 Box::leak — Cho phép memory leak có chủ đích

```rust
let b = Box::new("hello".to_string());
let s: &'static mut String = Box::leak(b);
// s sống mãi đến hết chương trình, không được drop
```

Tại sao dùng? Khi cần `&'static T` từ heap data — phổ biến cho global config:

```rust
fn make_static_config() -> &'static Config {
    Box::leak(Box::new(Config::load()))
}
```

Cẩn thận: **memory không free**. Chỉ dùng khi value sống đến end of program.

## 2.6 Box performance

- Alloc cost: 1 lần khi `Box::new`
- Deref: cùng cost như deref pointer thường (1 load)
- Drop: 1 lần free
- Sizeof: 8 byte (1 pointer trên 64-bit)

Trong hot path nên minimize Box alloc nếu type nhỏ.

## 2.7 So sánh với `unique_ptr` (C++)

`Box<T>` ≈ `std::unique_ptr<T>` C++:
- Single ownership
- Move = transfer ownership
- Auto dealloc on drop

Khác:
- C++ raw pointer + smart pointer cùng tồn tại → dễ bug
- Rust borrow checker enforce — không có shared mutable raw pointer

---

# Tầng 3: Rc<T> — Reference counting single-thread

## 3.1 Vấn đề Box không giải được

```rust
let b = Box::new(5);
let b2 = b;      // move
let b3 = b;      // ❌ ERROR: b already moved
```

Box là single-owner. Khi cần **multiple owners** (vd: graph node có nhiều parent), cần `Rc<T>`.

## 3.2 Rc<T> = Reference Counted

```rust
use std::rc::Rc;

let a = Rc::new(5);
let b = Rc::clone(&a);   // KHÔNG copy data, chỉ tăng counter
let c = Rc::clone(&a);

println!("count = {}", Rc::strong_count(&a));  // 3
```

3 biến cùng "sở hữu" giá trị `5`. Khi tất cả drop, mới free heap.

## 3.3 Memory layout

```
Stack:                          Heap:
┌──────────────┐               ┌──────────────────┐
│ a: Rc<i32>   │               │ RcBox<i32> {     │
│   ptr ───────┼──────────────►│   strong: 3      │
└──────────────┘               │   weak:   1      │
┌──────────────┐               │   data:   5      │
│ b: Rc<i32>   │   ┌──────────►│ }                │
│   ptr ───────┼───┘           └──────────────────┘
└──────────────┘
┌──────────────┐
│ c: Rc<i32>   │   ┌──────────►(cùng heap)
│   ptr ───────┼───┘
└──────────────┘
```

Heap block chứa:
- `strong: usize` — số strong references
- `weak: usize` — số weak references (xem Tầng 5)
- `data: T` — dữ liệu

Khi `Rc::clone()`: `strong += 1`. Khi `Drop`: `strong -= 1`, nếu = 0 → drop `T`, nếu cả weak = 0 → free heap block.

## 3.4 Rc<T> là immutable

```rust
let r = Rc::new(5);
*r += 1;  // ❌ ERROR: cannot borrow as mutable
```

`Rc<T>` chỉ cho phép **shared** (`&T`) — không mutate. Lý do: nếu cho mutate, 2 owner có thể đồng thời modify → race trong single-thread (nested borrow).

Để mutate, combine với `Cell` hoặc `RefCell`:

```rust
let r = Rc::new(RefCell::new(5));
*r.borrow_mut() += 1;
```

## 3.5 Cycle problem — Memory leak!

```rust
use std::rc::Rc;
use std::cell::RefCell;

struct Node {
    value: i32,
    next: RefCell<Option<Rc<Node>>>,
}

let a = Rc::new(Node { value: 1, next: RefCell::new(None) });
let b = Rc::new(Node { value: 2, next: RefCell::new(Some(Rc::clone(&a))) });
*a.next.borrow_mut() = Some(Rc::clone(&b));

// Bây giờ a → b → a → b → ... (cycle)
// strong_count của a = 2, b = 2
// Khi a, b drop khỏi stack → strong = 1 (vẫn không 0)
// → Memory không bao giờ free → LEAK!
```

→ Đây là điểm yếu lớn của reference counting (cả Rust và C++/Swift). Giải pháp: **Weak<T>** (Tầng 5).

## 3.6 Rc::clone vs .clone()

```rust
let a = Rc::new(5);
let b = Rc::clone(&a);  // ✅ Idiomatic
let b = a.clone();       // ✅ Tương đương nhưng kém rõ
```

Convention: dùng `Rc::clone(&a)` để rõ "đây không phải deep clone, chỉ tăng counter".

## 3.7 Rc<T> KHÔNG Send + Sync

```rust
let r = Rc::new(5);
thread::spawn(move || println!("{}", r));   // ❌ ERROR: Rc not Send
```

Lý do: counter không atomic — thread khác tăng/giảm cùng lúc → race. Cho thread-safe, dùng `Arc<T>` (Tầng 4).

## 3.8 Rc<T> performance

- Clone: 1 atomic-free increment (nhanh hơn Arc rất nhiều)
- Drop: 1 decrement + check zero
- Deref: 1 load (rẻ)
- Sizeof: 8 byte (pointer)
- Overhead heap: 16 byte (2 usize counter) + data

→ Nhanh hơn Arc, dùng khi single-thread.

## 3.9 Use cases Rc<T>

- Graph/tree với multiple parents
- Cache/lookup shared data (vd: AST nodes)
- GUI scene graph (parent-child reference)
- Shared config (read-only) trong single-thread

## 3.10 Cơ chế `clone()` ở mức bộ nhớ — non-atomic

Đây là điểm dễ hiểu lầm nhất: `Rc::clone` **KHÔNG** đụng tới `data: T`. Nó chỉ làm đúng 2 việc trên control block (`RcBox`):

1. Đọc `strong`, cộng 1, ghi lại — bằng phép `+1` **thường** (non-atomic), vì counter là `Cell<usize>`.
2. Trả về một `Rc` mới chứa **cùng một con trỏ** tới control block.

```rust
// Mô phỏng (đơn giản hoá) từ std:
impl<T> Clone for Rc<T> {
    fn clone(&self) -> Rc<T> {
        let n = self.inner().strong.get();   // strong: Cell<usize> → KHÔNG atomic
        self.inner().strong.set(n + 1);
        Rc { ptr: self.ptr }                 // copy con trỏ, KHÔNG copy T
    }
}
```

**Vì sao `+1` thường là an toàn?** Vì `Rc<T>` là `!Send + !Sync` (Tầng 3.7) → compiler **cấm** nó vượt biên thread → không bao giờ có 2 thread cùng sờ vào `strong` cùng lúc → không thể data race trên counter. Đây chính là lý do Rc nhanh: nó "mua" tốc độ bằng cách **từ bỏ multi-thread**.

Sơ đồ thay đổi bộ nhớ khi clone:

```
Trước clone:                       Sau `let b = Rc::clone(&a);`
 a ─┐                                a ─┐
    ▼                                   ├─► RcBox{ strong:2, weak:1, data:[..] }
 RcBox{ strong:1, weak:1, data }     b ─┘     (CÙNG block — data KHÔNG nhân đôi)
```

→ Clone là **O(1)**, không phụ thuộc size của `T`. Clone một `Rc<[u8; 1_000_000]>` cũng chỉ là tăng 1 counter + copy 8 byte con trỏ — không hề copy 1 MB. Đây là khác biệt bản chất với `.clone()` của một kiểu thường (deep copy).

Cặp đối xứng khi `Drop`: `strong -= 1` (cũng non-atomic); nếu `strong == 0` → gọi destructor của `T` rồi (nếu `weak == 0`) free luôn block.

---

# Tầng 4: Arc<T> — Atomic Rc cho multi-thread

## 4.1 Arc<T> = Atomic Reference Counted

Cùng API như `Rc<T>`, nhưng counter là **atomic** → safe cho multi-thread.

```rust
use std::sync::Arc;
use std::thread;

let a = Arc::new(vec![1, 2, 3]);
let mut handles = vec![];
for _ in 0..10 {
    let a = Arc::clone(&a);
    handles.push(thread::spawn(move || {
        println!("{:?}", a);
    }));
}
for h in handles { h.join().unwrap(); }
```

10 thread share `Vec` qua Arc — không clone Vec, chỉ tăng counter.

## 4.2 Memory layout (giống Rc)

```
Heap:
┌──────────────────────────┐
│ ArcInner<T> {            │
│   strong: AtomicUsize    │ ← atomic!
│   weak:   AtomicUsize    │ ← atomic!
│   data:   T              │
│ }                        │
└──────────────────────────┘
```

## 4.3 Arc<T> là immutable

Giống Rc, Arc chỉ shared. Để mutate qua nhiều thread → `Arc<Mutex<T>>` hoặc `Arc<RwLock<T>>`.

## 4.4 Performance: Arc vs Rc

| Op | Rc | Arc |
|----|-----|-----|
| Clone | ~1ns (non-atomic inc) | ~10-20ns (atomic inc) |
| Drop | ~1ns | ~10-20ns + maybe drop |
| Deref | ~1ns (same) | ~1ns (same) |

Atomic ops đắt hơn 5-20× non-atomic do CPU cache coherence. Nhưng so với syscall/network, vẫn rẻ. **Đừng overoptimize bằng Rc khi cần Arc**.

## 4.5 Arc<T> implements Send + Sync khi T: Send + Sync

```rust
fn requires_send<T: Send>(_: T) {}
requires_send(Arc::new(5));      // ✅ i32: Send + Sync → Arc<i32>: Send + Sync
requires_send(Arc::new(RefCell::new(5)));  // ❌ RefCell !Sync → Arc<RefCell> !Send
```

→ Khi share giữa thread, phải đảm bảo T Send + Sync. RefCell không Sync — phải Mutex/RwLock.

## 4.6 Pattern: Arc<T> immutable shared config

```rust
struct Config { url: String, timeout: u64 }

let cfg = Arc::new(Config::load());

// Mỗi task/thread chỉ cần clone Arc (rẻ), không cần clone Config
for _ in 0..1000 {
    let cfg = Arc::clone(&cfg);
    tokio::spawn(async move {
        connect(&cfg.url).await;
    });
}
```

Khi config **immutable**, Arc thuần là pattern siêu phổ biến.

## 4.7 Khi nào dùng Rc, khi nào Arc?

```
                Có cần share giữa các thread?
                          │
                  ┌───────┴────────┐
                 YES               NO
                  │                 │
                  ▼                 ▼
                Arc<T>            Rc<T>
                                  (faster)
```

**Quy tắc thực dụng**: nếu bạn không chắc, dùng Arc. Performance penalty nhỏ thường không đáng so với refactor sau này.

## 4.8 Cơ chế `Arc::clone` ở mức bộ nhớ — atomic increment

Giống Rc, `Arc::clone` chỉ copy con trỏ + tăng `strong`, **không copy `data`**. Khác biệt **duy nhất nhưng cốt tử**: `strong` là `AtomicUsize`, nên phép tăng phải là **atomic read-modify-write** (`fetch_add`):

```rust
// Mô phỏng từ std (lược phần overflow guard — xem 4.12):
impl<T> Clone for Arc<T> {
    fn clone(&self) -> Arc<T> {
        let old = self.inner().strong.fetch_add(1, Ordering::Relaxed);
        // ... check overflow ...
        Arc { ptr: self.ptr }
    }
}
```

`fetch_add` đảm bảo: nếu 100 thread cùng `clone()` một lúc, counter tăng **đúng** 100, không mất update. Một phép `+1` thường (`load` → `+1` → `store`) sẽ bị **lost update** khi 2 core xen kẽ → count sai → cuối cùng là double-free hoặc memory leak. Đó là lý do `Rc` không thể dùng cho multi-thread, còn `Arc` tồn tại.

## 4.9 Vì sao `clone` chỉ cần `Relaxed`?

`Ordering::Relaxed` = "đảm bảo **atomicity** của riêng phép này, KHÔNG đồng bộ hoá bất kỳ vùng nhớ nào khác". Nghe có vẻ liều — nhưng với `clone` nó **vừa đủ và đúng**:

- Để gọi được `self.clone()`, bạn **đã đang giữ một `Arc`** rồi → `strong ≥ 1` → object **chắc chắn còn sống**, không thread nào free nó ngay lúc này.
- `clone` không **đọc/ghi `data`**, cũng không "công bố" hay "thu nhận" dữ liệu nào — chỉ làm counter `+1`.
- Quan hệ happens-before cần để thấy `data` hợp lệ **đã được thiết lập từ trước** bởi chính cái `Arc` bạn đang cầm.

→ Không cần `Acquire`/`Release` → tiết kiệm memory barrier → clone rẻ nhất có thể mà vẫn đúng.

> 🧠 Trực giác: *"Tôi tăng số người tham chiếu — nhưng vì chính tôi đã là một người tham chiếu, chẳng ai phá được nhà trong lúc tôi đang đếm."*

## 4.10 `drop` của Arc — `Release` rồi `Acquire`

Đây là nửa khó. Khi một `Arc` bị drop:

```rust
impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // 1) Giảm strong bằng Release
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;                    // chưa phải owner cuối → xong, KHÔNG free
        }
        // 2) Mình là owner cuối (vừa đưa count 1 → 0)
        std::sync::atomic::fence(Ordering::Acquire);   // hàng rào trước khi huỷ
        // 3) drop data T + giải phóng heap
        unsafe { self.drop_slow(); }
    }
}
```

Vì sao cặp `Release` (lúc giảm) + `Acquire` (trước khi free) là **bắt buộc**:

- **`Release` ở mỗi lần giảm:** mọi thao tác ghi mà thread này từng làm lên `data` (qua `Arc`) phải hoàn tất & "đẩy ra" **trước** khi nó buông quyền sở hữu. Nó *công bố*: "tôi đã xong việc với dữ liệu".
- **`Acquire` ở thread giảm cuối cùng:** thread đưa count về 0 phải **nhìn thấy toàn bộ** các ghi của *mọi* thread khác từng giữ Arc, **trước khi** chạy destructor của `T` và `free` bộ nhớ. `Acquire` *thu nhận* tất cả các `Release` kia.

Thiếu cặp này, CPU/compiler được phép sắp xếp lại lệnh sao cho `free` chạy **trước** khi một thread khác kịp hoàn tất ghi lên `data` → **use-after-free / double-free**. Đây không phải lý thuyết suông — đó chính là loại bug mà memory model sinh ra để chặn.

> 💡 Vì sao dùng `fence(Acquire)` **riêng** thay vì `fetch_sub(1, AcqRel)`? Để chỉ trả phí `Acquire` ở **đúng lần cuối**. Các lần giảm không-phải-cuối chỉ cần `Release` (rẻ hơn). Lại là một ví dụ "chọn ordering tối thiểu vừa đủ" — đúng tinh thần [a-memory-model.md](./a-memory-model.md) (mục *Atomic trong thực tế: Arc đếm reference*).

## 4.11 Bức tranh happens-before tổng thể

```
Thread A (ghi rồi buông):          Thread B (owner cuối, sắp free):
  ghi vào data .........
  drop Arc:
    strong.fetch_sub(Release) ──┐
                                │  synchronizes-with
                                └────► fence(Acquire)
                                          │ happens-before
                                          ▼
                                       destructor(data)  ← THẤY ghi của A ✓
                                       free(heap)         ← an toàn ✓
```

Chuỗi happens-before hoàn chỉnh:

```
use data (A) → decrement Release (A) → [synchronizes-with] → fence Acquire (B) → drop & free (B)
```

→ Đảm bảo **không thread nào còn đang dùng `data` khi nó bị huỷ**, và thread huỷ **thấy trạng thái mới nhất**. Đây là toàn bộ lý do `Arc` an toàn để chia sẻ giữa các thread — không phải nhờ "phép màu", mà nhờ chọn đúng ordering.

## 4.12 Hai chi tiết "khó nhằn" còn lại

**1. Overflow guard.** Nếu có `~usize::MAX` lần clone (vd `mem::forget` lặp), counter sẽ tràn về 0 → free nhầm khi vẫn còn `Arc` đang sống → UB. std chặn bằng cách kiểm tra `old` rồi **`abort()`** (không `panic`, vì panic có thể bị bắt và để lại counter ở trạng thái hỏng):

```rust
let old = self.inner().strong.fetch_add(1, Ordering::Relaxed);
if old > MAX_REFCOUNT {           // ~ isize::MAX
    std::process::abort();
}
```

**2. Clone của `Weak`.** `Weak::clone` tăng `weak` (cũng `Relaxed`), không đụng `strong`. `Arc::downgrade` tăng `weak`; còn `Weak::upgrade` phải `compare_exchange` trên `strong` (CAS) để **chỉ tăng nếu `strong > 0`** — nếu object đã chết (`strong == 0`) thì trả `None`. Đây là chỗ duy nhất trong vòng đời count cần CAS thay vì `fetch_add` thuần, vì nó vừa phải kiểm tra điều kiện vừa phải tăng một cách atomic.

| Thao tác | Counter | Ordering | Ghi chú |
|----------|---------|----------|---------|
| `Arc::clone` | `strong += 1` | `Relaxed` | đã giữ ref → object còn sống |
| `Arc::drop` (không cuối) | `strong -= 1` | `Release` | công bố ghi của mình |
| `Arc::drop` (cuối, →0) | `+ fence` | `Acquire` | thu nhận trước khi free |
| `Weak::clone` | `weak += 1` | `Relaxed` | không cản drop của data |
| `Weak::upgrade` | `strong` CAS | `Acquire`/`Relaxed` | chỉ +1 nếu `> 0`, else `None` |
| overflow | — | — | `old > MAX_REFCOUNT` → `process::abort()` |

---

# Tầng 5: Weak<T> — Phá vỡ cycle

## 5.1 Vấn đề: Cycle leak

```rust
// a ─→ b ─→ a (cycle)
let a = Rc::new(Node { next: RefCell::new(None) });
let b = Rc::new(Node { next: RefCell::new(Some(Rc::clone(&a))) });
*a.next.borrow_mut() = Some(Rc::clone(&b));
// → memory leak khi a, b ra khỏi scope
```

## 5.2 Weak<T> — Reference không tăng strong count

```rust
use std::rc::{Rc, Weak};

let a = Rc::new(5);
let weak = Rc::downgrade(&a);  // tạo Weak từ Rc

println!("strong = {}", Rc::strong_count(&a));  // 1
println!("weak   = {}", Rc::weak_count(&a));    // 1
```

`Weak<T>`:
- Không tăng strong count → không ngăn drop
- Tăng weak count → ngăn dealloc heap block (vì còn cần data layout)
- Phải `upgrade()` để dùng → `Option<Rc<T>>` (None nếu data đã drop)

## 5.3 Upgrade pattern

```rust
let a = Rc::new(5);
let weak = Rc::downgrade(&a);

if let Some(strong) = weak.upgrade() {
    println!("still alive: {}", *strong);
} else {
    println!("dropped");
}

drop(a);

assert!(weak.upgrade().is_none());  // a đã drop
```

## 5.4 Phá cycle với Weak

```rust
struct Parent {
    children: RefCell<Vec<Rc<Child>>>,
}

struct Child {
    parent: RefCell<Weak<Parent>>,   // ← Weak, không Rc
    name: String,
}
```

- Parent → Child: strong (Rc) — parent OWNS children
- Child → Parent: weak — child không own parent

Khi parent drop, children list drop, children drop (vì strong = 0). Weak từ child → parent không ngăn drop. ✅ Không leak.

## 5.5 Use cases Weak

- Tree với parent reference (root → child = Rc, child → parent = Weak)
- Observer pattern (subject giữ list Weak<Observer>)
- Cache với "có thể vẫn còn" reference

## 5.6 Memory layout với Weak

```
Heap block luôn còn cho đến khi cả strong=0 VÀ weak=0:

Khi strong = 0:
  → drop data T
  → giải phóng phần data
  → CHƯA giải phóng counter block

Khi weak = 0 (sau strong = 0):
  → giải phóng counter block (16 byte)
  → memory hoàn toàn free
```

Vì vậy Weak vẫn "giữ chân" 16 byte counter ngay cả khi data drop. Đủ để upgrade trả None safely.

---

# Tầng 6: Interior Mutability — Bẻ cong quy tắc borrow

## 6.1 Quy tắc Rust thông thường

```
   AT ANY TIME:
   • 1 mutable reference (&mut T)
   • HOẶC
   • N immutable references (&T)
   
   KHÔNG bao giờ có cả 2 cùng lúc.
```

Compile-time enforce. Strict nhưng không đủ flexible cho mọi case.

## 6.2 Vấn đề: Mutate qua &T

Đôi khi bạn có `&T` (shared) nhưng cần modify field bên trong:

```rust
// Cache: lookup là &self nhưng cần update internal HashMap
struct Cache {
    data: HashMap<String, i32>,
}
impl Cache {
    fn get(&self, key: &str) -> i32 {
        // Cần update access count → nhưng &self không cho mutate
    }
}
```

Giải pháp: **Interior mutability** — mutate qua `&T` (lấy `&mut T` từ bên trong).

## 6.3 Các type interior mutability

| Type | Single/Multi-thread | Runtime check? | Cost |
|------|---------------------|----------------|------|
| `Cell<T>` | Single | Không (only Copy types) | Rất rẻ |
| `RefCell<T>` | Single | Có (panic if violate) | Rẻ |
| `Mutex<T>` | Multi | Có (block other) | Trung bình |
| `RwLock<T>` | Multi | Có (block writer) | Trung bình |
| Atomic types | Multi | Hardware | Tuỳ ordering |
| `OnceCell<T>` | Single | Có (1 lần init) | Rẻ |
| `OnceLock<T>` | Multi | Có (1 lần init) | Trung bình |

## 6.4 UnsafeCell — Nguyên thủy gốc

Tất cả interior mutability dựa trên `std::cell::UnsafeCell<T>` — type duy nhất legal cho phép `&T → &mut T` (qua `unsafe`).

```rust
pub struct UnsafeCell<T: ?Sized> { value: T }

impl<T> UnsafeCell<T> {
    pub fn get(&self) -> *mut T;   // ← magic: từ &self ra *mut T
}
```

Bạn không dùng `UnsafeCell` trực tiếp. Cell, RefCell, Mutex... đều build trên nó.

---

# Tầng 7: Cell<T> và RefCell<T>

## 7.1 Cell<T> — Chỉ cho Copy types

```rust
use std::cell::Cell;

let c = Cell::new(5);
c.set(10);
let v = c.get();    // 10
```

`Cell<T>`:
- `T` phải `Copy` (hoặc `T: ?Sized` với Default)
- `get()` trả COPY của value (không borrow)
- `set()` thay value
- KHÔNG cho `&mut T` trực tiếp

```rust
let c = Cell::new(5);
let r = c.get();
c.set(10);
println!("{}", r);  // vẫn 5 (vì r là COPY)
```

→ Cell cực kỳ rẻ (chỉ load/store) nhưng giới hạn.

## 7.2 Cell::take và replace

```rust
let c = Cell::new(vec![1, 2, 3]);
let v = c.take();          // lấy ra (replace với Default::default())
// c giờ chứa Vec::new()

let old = c.replace(vec![10, 20]);
// c giờ chứa [10, 20], old chứa value cũ
```

`Cell` cho không-Copy nếu T impl Default — qua `take`.

## 7.3 RefCell<T> — Runtime borrow checker

```rust
use std::cell::RefCell;

let r = RefCell::new(vec![1, 2, 3]);

let borrowed: std::cell::Ref<Vec<i32>> = r.borrow();
println!("{:?}", *borrowed);
drop(borrowed);

let mut mutated: std::cell::RefMut<Vec<i32>> = r.borrow_mut();
mutated.push(4);
```

`RefCell<T>`:
- `borrow()` → `Ref<T>` (giống `&T`)
- `borrow_mut()` → `RefMut<T>` (giống `&mut T`)
- Track borrow count tại RUNTIME
- **Panic** nếu vi phạm quy tắc

## 7.4 Khi nào panic?

```rust
let r = RefCell::new(5);
let _a = r.borrow();
let _b = r.borrow_mut();  // ❌ PANIC: already borrowed
```

```rust
let r = RefCell::new(5);
let _a = r.borrow_mut();
let _b = r.borrow_mut();  // ❌ PANIC: already mutably borrowed
```

Phải drop borrow trước khi borrow lại differently.

## 7.5 try_borrow / try_borrow_mut

Để tránh panic, có version trả Result:

```rust
let r = RefCell::new(5);
let _a = r.borrow();

match r.try_borrow_mut() {
    Ok(mut b) => *b += 1,
    Err(_) => eprintln!("already borrowed"),
}
```

Phù hợp khi không chắc borrow status.

## 7.6 Memory layout

```
RefCell<T>:
┌──────────────────────────┐
│ flag: Cell<BorrowFlag>   │  ← isize: 0=untouched, >0=N readers, -1=writer
│ value: UnsafeCell<T>     │
└──────────────────────────┘

Khi borrow():
  if flag >= 0: flag += 1; return Ref
  else: panic

Khi borrow_mut():
  if flag == 0: flag = -1; return RefMut
  else: panic
```

Cost: 1 atomic-free load + check.

## 7.7 Use case kinh điển: Mock object trong test

```rust
trait Messenger {
    fn send(&self, msg: &str);
}

struct MockMessenger {
    sent: RefCell<Vec<String>>,
}

impl Messenger for MockMessenger {
    fn send(&self, msg: &str) {
        // trait method nhận &self nhưng cần modify sent
        self.sent.borrow_mut().push(msg.into());
    }
}
```

Trait method `send(&self, ...)` không cho `&mut self`, nhưng test cần track calls → RefCell.

## 7.8 Cell vs RefCell so sánh

| Aspect | Cell | RefCell |
|--------|------|---------|
| Type bound | T: Copy (hoặc Default cho take) | Any T |
| Borrow ra reference? | ❌ Chỉ get/set value | ✅ borrow/borrow_mut |
| Runtime cost | Rất rẻ (load/store) | Rẻ + flag check |
| Panic? | Không | Có nếu vi phạm |
| Use case | Counter, simple field | Vec, HashMap, complex |

---

# Tầng 8: Mutex<T> — Thread-safe mutation

## 8.1 Vấn đề: Mutate shared data từ nhiều thread

```rust
let data = Arc::new(vec![0; 10]);
for _ in 0..5 {
    let data = Arc::clone(&data);
    thread::spawn(move || {
        data[0] += 1;   // ❌ ERROR: cannot mutate
    });
}
```

Arc cho shared, nhưng immutable. Không thể `&mut` từ Arc thẳng.

## 8.2 Arc<Mutex<T>>

```rust
use std::sync::{Arc, Mutex};

let data = Arc::new(Mutex::new(vec![0; 10]));
let mut handles = vec![];
for _ in 0..5 {
    let data = Arc::clone(&data);
    handles.push(thread::spawn(move || {
        let mut guard = data.lock().unwrap();
        guard[0] += 1;
    }));
}
for h in handles { h.join().unwrap(); }

println!("{:?}", data.lock().unwrap());
```

`Mutex<T>` cung cấp:
- `lock()` → `LockResult<MutexGuard<T>>` (block cho đến khi free)
- `try_lock()` → `TryLockResult<...>` (return ngay)
- `MutexGuard<T>` giống `&mut T`, auto unlock khi drop

## 8.3 Memory layout

```
Mutex<T>:
┌──────────────────────────┐
│ poisoned: AtomicBool     │
│ inner: sys::Mutex (OS)   │  ← futex/pthread_mutex
│ data: UnsafeCell<T>      │
└──────────────────────────┘
```

`Mutex` trên Linux dùng **futex**:
- Uncontended (no waiters): atomic CAS, ~10ns
- Contended: syscall vào kernel, sleep/wakeup, ~µs

## 8.4 Poisoning — Thread chết khi giữ lock

```rust
let m = Arc::new(Mutex::new(0));
let m2 = Arc::clone(&m);

let h = thread::spawn(move || {
    let _g = m2.lock().unwrap();
    panic!("ohno");   // panic trong khi giữ lock
});
h.join().unwrap_err();

// Bây giờ Mutex bị "poisoned":
let result = m.lock();
// Err(PoisonError) — vì state có thể không consistent
```

Lý do: thread panic giữa modify → state có thể inconsistent. Default: error cho lần lock sau.

Recover poisoned mutex:
```rust
let guard = match m.lock() {
    Ok(g) => g,
    Err(poisoned) => poisoned.into_inner(),  // ignore poison
};
```

Tranh cãi: poisoning feature hay anti-feature? Một số crate (`parking_lot::Mutex`) bỏ poisoning để nhanh hơn.

## 8.5 MutexGuard auto unlock

```rust
{
    let mut g = m.lock().unwrap();
    *g += 1;
    // g out of scope → drop → unlock tự động
}
```

RAII pattern — không cần `unlock()` manually. So với pthread (`pthread_mutex_unlock`) — bug nếu quên unlock.

## 8.6 Scope của lock

⚠️ **Antipattern**: giữ lock quá lâu:

```rust
// ❌ TỆ
let g = data.lock().unwrap();
expensive_io().await;     // lock held suốt I/O!
process(&*g);
```

→ Tất cả thread khác bị block. Rút ngắn:

```rust
// ✅ TỐT
let snapshot = {
    let g = data.lock().unwrap();
    g.clone()             // copy ra
};                        // drop g ngay
expensive_io().await;
process(&snapshot);
```

## 8.7 Deadlock — 2 thread chờ nhau

```rust
let a = Arc::new(Mutex::new(0));
let b = Arc::new(Mutex::new(0));

let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
let t1 = thread::spawn(move || {
    let _ga = a1.lock();
    thread::sleep(Duration::from_millis(10));
    let _gb = b1.lock();  // ← chờ b
});

let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
let t2 = thread::spawn(move || {
    let _gb = b2.lock();
    thread::sleep(Duration::from_millis(10));
    let _ga = a2.lock();  // ← chờ a
});

// DEADLOCK! T1 giữ a, chờ b. T2 giữ b, chờ a.
```

Cách tránh:
- **Lock ordering**: luôn lock theo thứ tự fixed (a → b trong mọi nơi)
- `try_lock` + retry
- Tránh lock nhiều mutex cùng lúc

## 8.8 parking_lot crate

`parking_lot::Mutex` là alternative phổ biến:
- Nhanh hơn std (~2-3x)
- Không poisoning
- API đơn giản hơn (lock() không trả Result)
- Hỗ trợ timeout natively

```toml
[dependencies]
parking_lot = "0.12"
```

```rust
use parking_lot::Mutex;

let m = Mutex::new(0);
let mut g = m.lock();   // ← không cần unwrap
*g += 1;
```

Cân nhắc dùng `parking_lot` cho hot paths.

---

# Tầng 9: RwLock<T> — Many readers, one writer

## 9.1 Vấn đề: Mutex serialize cả readers

`Mutex` cho 1 thread access mỗi lúc. Nhưng nếu workload là **đọc nhiều, ghi ít** (cache, config), serialize readers là phí phạm.

## 9.2 RwLock<T> — Read-write lock

```rust
use std::sync::RwLock;

let lock = RwLock::new(5);

// Nhiều readers cùng lúc:
{
    let r1 = lock.read().unwrap();
    let r2 = lock.read().unwrap();
    println!("{} {}", *r1, *r2);
}

// Writer độc quyền:
{
    let mut w = lock.write().unwrap();
    *w = 10;
}
```

Rules:
- N readers cùng lúc HOẶC
- 1 writer (block tất cả readers + writers khác)

## 9.3 Memory layout

```
RwLock<T>:
┌──────────────────────────┐
│ state: AtomicUsize       │  ← bit 0 = writer locked
│                          │     bits 1..31 = reader count
│ inner: sys::RwLock       │
│ data: UnsafeCell<T>      │
└──────────────────────────┘
```

## 9.4 Khi nào dùng RwLock?

Khi:
- **Read >> Write ratio** (vd: 100 reads / 1 write)
- Critical sections dài (readers parallel có ý nghĩa)

Khi KHÔNG nên dùng:
- Write nhiều
- Critical section rất ngắn (overhead RwLock > Mutex)

## 9.5 Writer starvation

```
   Time:  T1 reader → T2 reader → T3 reader → T4 reader → ...
                                           ↑
                                  T5 writer chờ mãi
```

Nếu liên tục có reader đến, writer có thể "đói". Linux implementation thường giảm priority reader khi có writer chờ — nhưng vẫn cần aware.

## 9.6 Mutex vs RwLock benchmark

```
   Workload: 90% reads, 10% writes, 8 threads
   ────────────────────────────────────────
   
   Mutex:        100% serial         → throughput ~X
   RwLock:       readers parallel    → throughput ~5-8X
   
   Workload: 50% reads, 50% writes
   ────────────────────────────────────────
   
   Mutex:        100% serial         → throughput ~X
   RwLock:       contention writer   → throughput ~0.8X (overhead!)
```

**Đo bằng** `criterion` thay vì đoán.

## 9.7 parking_lot::RwLock

Như parking_lot Mutex — nhanh hơn, không poisoning, fairness configurable.

## 9.8 Pattern: Snapshot read

```rust
let cfg = Arc::new(RwLock::new(Config::default()));

// Reader path (hot):
fn get_url(cfg: &RwLock<Config>) -> String {
    let r = cfg.read().unwrap();
    r.url.clone()
}

// Writer path (cold, reload config):
fn reload(cfg: &RwLock<Config>) {
    let new = Config::load();
    let mut w = cfg.write().unwrap();
    *w = new;
}
```

Reader minimal lock time (chỉ clone field cần).

## 9.9 Alternative: ArcSwap

`arc_swap::ArcSwap<T>` — lock-free read, atomic swap write:

```rust
use arc_swap::ArcSwap;
let cfg: ArcSwap<Config> = ArcSwap::new(Arc::new(Config::default()));

// Reader (lock-free, atomic load):
let snap = cfg.load();
println!("{}", snap.url);

// Writer:
cfg.store(Arc::new(Config::load()));
```

Cực nhanh cho read-heavy + atomic config swap. Trade-off: read là snapshot, không thấy update mid-read.

---

# Tầng 10: Combinations — Rc<RefCell<T>>, Arc<Mutex<T>>

## 10.1 Single-thread mutation: Rc<RefCell<T>>

```rust
use std::rc::Rc;
use std::cell::RefCell;

let shared = Rc::new(RefCell::new(vec![1, 2, 3]));

let a = Rc::clone(&shared);
let b = Rc::clone(&shared);

a.borrow_mut().push(4);
b.borrow_mut().push(5);

println!("{:?}", shared.borrow());  // [1, 2, 3, 4, 5]
```

Pattern: **shared mutable state single-thread**. Phổ biến trong GUI, parser AST, graph.

## 10.2 Multi-thread mutation: Arc<Mutex<T>>

```rust
use std::sync::{Arc, Mutex};

let counter = Arc::new(Mutex::new(0));

let handles: Vec<_> = (0..10).map(|_| {
    let c = Arc::clone(&counter);
    thread::spawn(move || {
        for _ in 0..100 {
            *c.lock().unwrap() += 1;
        }
    })
}).collect();

for h in handles { h.join().unwrap(); }
println!("{}", *counter.lock().unwrap());  // 1000
```

Đây là pattern **đếm chia sẻ giữa threads**.

## 10.3 Read-heavy: Arc<RwLock<T>>

```rust
let cache: Arc<RwLock<HashMap<String, String>>> = 
    Arc::new(RwLock::new(HashMap::new()));

// 100 reader threads — đa số read parallel
// 1 writer occasional update
```

## 10.4 Tại sao Arc<T>, không Arc<Mutex<T>> luôn?

Nếu T immutable, **không cần Mutex**. Mutex chỉ cần khi mutate.

```rust
// ✅ Read-only config — không Mutex
let cfg: Arc<Config> = Arc::new(Config::load());

// ✅ Mutable shared state — cần Mutex
let counter: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
```

## 10.5 Cách combine quyết định

```
   Single-thread?
   ────────────
   • Read-only:       Rc<T>
   • Mutable:         Rc<RefCell<T>>  hoặc  Rc<Cell<T>>
   
   Multi-thread?
   ─────────────
   • Read-only:       Arc<T>
   • Mutable:         Arc<Mutex<T>>
   • Read-heavy:      Arc<RwLock<T>>
   • Atomic primitive:Arc<AtomicXxx> hoặc chỉ AtomicXxx + static
```

## 10.6 Nested smart pointers — Khi nào?

```rust
Arc<Mutex<HashMap<String, Arc<User>>>>
```

Đây là pattern thực tế:
- `Arc<...>` — share map giữa threads
- `Mutex<...>` — mutate map (insert/remove)
- `HashMap<String, ...>` — lookup
- `Arc<User>` — user data shared, không clone khi lookup

→ Pattern user store thread-safe trong web server.

## 10.7 Pitfall: Arc<Mutex<T>> cho mọi thứ

Không phải mọi shared mutable cần `Arc<Mutex>`. Có khi:
- Message passing (channel) đơn giản hơn
- `Atomic*` types cho primitive
- `DashMap` (concurrent HashMap) thay `Arc<Mutex<HashMap>>`

```rust
// Thay vì:
let map: Arc<Mutex<HashMap<K, V>>> = Arc::new(Mutex::new(...));

// Cân nhắc:
use dashmap::DashMap;
let map: Arc<DashMap<K, V>> = Arc::new(DashMap::new());
// → lock per bucket, parallel rất tốt
```

---

# Tầng 11: Smart pointers nâng cao — Cow, OnceCell, LazyLock

## 11.1 Cow<T> — Clone on Write

`std::borrow::Cow<T>` = "Clone on Write" — borrowed OR owned, chỉ clone khi cần modify.

```rust
use std::borrow::Cow;

fn process(input: &str) -> Cow<str> {
    if input.contains("bad") {
        Cow::Owned(input.replace("bad", "***"))  // alloc nếu cần
    } else {
        Cow::Borrowed(input)                      // không alloc
    }
}

let s1 = process("hello");      // Borrowed — không alloc
let s2 = process("bad word");   // Owned — alloc
```

Use case: function trả String **chỉ khi cần modify**, ngược lại trả `&str` (zero-cost).

## 11.2 Cow trong serde

```rust
#[derive(Deserialize)]
struct Config<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
}
```

Nếu JSON string không có escape character, serde borrow trực tiếp từ input → zero-alloc. Có escape → owned.

## 11.3 OnceCell<T> và OnceLock<T> — Lazy init

```rust
use std::cell::OnceCell;

let cell: OnceCell<String> = OnceCell::new();
let v: &String = cell.get_or_init(|| {
    println!("initializing");
    "value".to_string()
});
// Lần 2: không print "initializing"
let v2 = cell.get_or_init(|| panic!("not called"));
```

`OnceCell<T>`:
- Khởi tạo **đúng 1 lần** (lazy)
- Sau khi init, là `&T` immutable
- Single-thread (unless OnceLock)

`OnceLock<T>`: thread-safe version (atomic init).

## 11.4 LazyLock<T> — Global lazy static

```rust
use std::sync::LazyLock;

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d+$").unwrap()
});

fn check(s: &str) -> bool {
    REGEX.is_match(s)   // first call: init; sau đó: chỉ deref
}
```

Stable từ Rust 1.80. Trước đó dùng crate `once_cell::sync::Lazy` hoặc `lazy_static!` macro.

## 11.5 Atomic types — Lock-free primitives

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

COUNTER.fetch_add(1, Ordering::Relaxed);
let c = COUNTER.load(Ordering::Relaxed);
```

Available:
- `AtomicBool`, `AtomicI8/16/32/64`, `AtomicU8/16/32/64`
- `AtomicPtr<T>` — atomic pointer
- `AtomicIsize`, `AtomicUsize`

Memory orderings (đơn giản → phức tạp):
- `Relaxed` — chỉ atomicity, không synchronization
- `Acquire`/`Release` — pair cho happens-before
- `AcqRel` — combined
- `SeqCst` — total order, mạnh nhất (chậm nhất)

Đây là deep topic — xem `memory-model.md` Tầng 9-10 hoặc Rust nomicon.

## 11.6 Pin<P> — Pin lên địa chỉ

```rust
use std::pin::Pin;
let pinned: Pin<Box<Future>> = Box::pin(some_async_fn());
```

`Pin<P>` được giải thích sâu trong [async.md Tầng 5](./async.md). Tóm tắt: prevent moving của value mà có self-reference.

## 11.7 Tóm tắt smart pointers nâng cao

```
   ┌─────────────┬──────────────────────────────┐
   │ Pointer     │ Mục đích                     │
   ├─────────────┼──────────────────────────────┤
   │ Cow<T>      │ Borrow OR owned, defer clone │
   │ OnceCell<T> │ Init 1 lần (single-thread)   │
   │ OnceLock<T> │ Init 1 lần (multi-thread)    │
   │ LazyLock<T> │ Global static lazy init      │
   │ Pin<P>      │ Prevent move (self-ref)      │
   │ Atomic*     │ Lock-free primitives         │
   └─────────────┴──────────────────────────────┘
```

---

# Tầng 12: Async smart pointers — tokio::sync

## 12.1 Vấn đề: std Mutex trong async

```rust
async fn bad() {
    let m = std::sync::Mutex::new(0);
    let mut g = m.lock().unwrap();
    other_async().await;   // ❌ MutexGuard !Send → Future !Send → spawn fail
    *g += 1;
}
```

`std::sync::Mutex::lock()`:
- Block thread (không yield)
- Guard không Send (trên Linux)
- Giữ qua `.await` → block executor thread

## 12.2 tokio::sync::Mutex

```rust
use tokio::sync::Mutex;

async fn good() {
    let m = Mutex::new(0);
    let mut g = m.lock().await;    // async lock, yield khi chờ
    other_async().await;           // OK, guard Send
    *g += 1;
}
```

`tokio::sync::Mutex::lock().await`:
- KHÔNG block thread khi waiting
- Task yield cho executor làm việc khác
- Guard Send → safe spawn

## 12.3 Khi nào dùng std vs tokio Mutex?

```
   ┌──────────────────────────────────────────────┐
   │ Giữ lock qua .await?                         │
   │                                              │
   │   NO  → std::sync::Mutex (nhanh hơn)         │
   │   YES → tokio::sync::Mutex                   │
   └──────────────────────────────────────────────┘
```

Khi không await trong lock (chỉ vài operations đơn giản), `std::sync::Mutex` ưu việt hơn:
- Nhanh hơn (~2-10x)
- Đơn giản hơn

Chỉ chuyển sang tokio Mutex khi thực sự cần await trong critical section.

## 12.4 tokio::sync::RwLock

Tương tự, có async version của RwLock.

## 12.5 tokio::sync::Semaphore — Rate limiting

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let sem = Arc::new(Semaphore::new(10));   // max 10 concurrent

for _ in 0..1000 {
    let permit = sem.clone().acquire_owned().await.unwrap();
    tokio::spawn(async move {
        do_work().await;
        drop(permit);  // tự động khi out of scope
    });
}
```

Chạy max 10 task cùng lúc — phổ biến cho rate limit outbound API calls.

## 12.6 tokio::sync::oneshot, mpsc, broadcast, watch

Channels = alternative to shared mutable state. Tránh lock entirely:

```rust
use tokio::sync::mpsc;
let (tx, mut rx) = mpsc::channel::<Msg>(100);

// Producer:
tx.send(msg).await.unwrap();

// Consumer:
while let Some(msg) = rx.recv().await {
    process(msg).await;
}
```

Đã giải thích chi tiết trong [async.md](./async.md). Quy tắc senior: **"share memory by communicating, don't communicate by sharing memory"** (Go motto, áp dụng tốt cho Rust).

## 12.7 std::sync::Mutex trong async: Cách dùng đúng

```rust
async fn correct() {
    let m = Arc::new(std::sync::Mutex::new(0));
    
    let snapshot = {
        let g = m.lock().unwrap();
        g.clone()
    };  // ← drop guard TRƯỚC await
    
    process(snapshot).await;
    
    let new_value = compute().await;
    {
        let mut g = m.lock().unwrap();
        *g = new_value;
    }
}
```

Pattern: lock → snapshot → drop → await → lock → update.

---

# Tầng 13: Decision tree — Khi nào dùng cái nào

## 13.1 Master decision tree

```
                  Cần share dữ liệu?
                          │
                  ┌───────┴────────┐
                 NO                YES
                  │                 │
              ┌───┴───┐             ▼
              │       │       Cần multiple owners?
            Box<T>   T           │
            (heap)   (stack)  ┌──┴──┐
                             NO     YES
                              │      │
                              ▼      ▼
                            Trên 1 thread?
                          (chỉ qua reference)
                                   │
                               ┌───┴───┐
                              YES     NO
                               │      │
                               ▼      ▼
                              Rc<T>  Arc<T>
                                       │
                                       
                  Cần mutate?
                          │
                  ┌───────┴────────┐
                 NO                YES
                  │                 │
              Arc<T>            Single thread?
              Rc<T>                  │
                                 ┌───┴────┐
                                YES      NO
                                 │        │
                              Copy T?    Read>>Write?
                                 │           │
                             ┌───┴───┐   ┌───┴────┐
                            YES     NO  YES      NO
                             │       │   │        │
                            Cell    RefCell RwLock  Mutex
                                     <T>     <T>    <T>
                                     
                                  (Multi-thread):
                                  Arc<RwLock<T>>
                                  Arc<Mutex<T>>
```

## 13.2 Quick reference card

```
   Scenario                       Smart pointer
   ────────────────────────────   ─────────────────────────────
   Heap alloc                     Box<T>
   Recursive type                 Box<T>
   Trait object                   Box<dyn Trait>
   
   Shared single-thread (RO)      Rc<T>
   Shared single-thread (RW)      Rc<RefCell<T>>  or  Rc<Cell<T>>
   
   Shared multi-thread (RO)       Arc<T>
   Shared multi-thread (RW)       Arc<Mutex<T>>
   Shared MT, read-heavy          Arc<RwLock<T>>
   
   Borrow OR owned                Cow<'a, str> / Cow<'a, [T]>
   Lazy init (1 thread)           OnceCell<T>
   Lazy init (multi thread)       OnceLock<T> / LazyLock<T>
   Atomic primitive               AtomicXxx
   
   Async lock                     tokio::sync::Mutex<T>
   Async RwLock                   tokio::sync::RwLock<T>
   Rate limit                     Arc<Semaphore>
   Message passing                tokio::sync::mpsc / oneshot
```

## 13.3 Performance heuristics

```
   ┌──────────────────────────────────────────────────────┐
   │ Operation              Approx cost (nanoseconds)     │
   ├──────────────────────────────────────────────────────┤
   │ Box deref              ~1ns                          │
   │ Rc clone               ~1ns                          │
   │ Arc clone              ~10-20ns (atomic)             │
   │ Cell get/set           ~1ns                          │
   │ RefCell borrow         ~2-3ns                        │
   │ Mutex lock (uncontend) ~10-20ns                      │
   │ Mutex lock (contend)   ~µs (syscall)                 │
   │ RwLock read uncontend  ~20-30ns                      │
   │ Atomic load Relaxed    ~1ns                          │
   │ Atomic CAS             ~5-10ns                       │
   └──────────────────────────────────────────────────────┘
   
   📌 Nhanh hay chậm phụ thuộc rất nhiều vào:
      • Cache locality
      • Contention level
      • CPU architecture
   ⟹ Đo bằng `criterion`, không đoán.
```

## 13.4 Antipatterns checklist

```
   ❌ Arc<Mutex<T>> mọi nơi, kể cả T immutable
      → Arc<T> đủ
   
   ❌ Rc khi có thể chỉ cần borrow
      → &T qua function arg
   
   ❌ RefCell trên struct lớn — bug runtime panic
      → restructure data, hoặc smaller RefCell field
   
   ❌ std::sync::Mutex trong async, hold qua .await
      → tokio::sync::Mutex hoặc drop guard trước await
   
   ❌ Tự implement Mutex từ AtomicUsize
      → dùng std hoặc parking_lot, đã optimized
   
   ❌ Arc::clone trong loop hot path
      → consider lifetime + reference
   
   ❌ Mutex giữ qua I/O (read file, network)
      → snapshot pattern
   
   ❌ Cycle Rc/Arc không Weak
      → memory leak
   
   ❌ Lazy + LazyLock global state nhiều quá
      → dependency injection thường tốt hơn
```

## 13.5 Mental model — Senior cách nghĩ

Khi gặp shared state:

1. **Hỏi: có cần share không?** Nhiều khi pass `&T` đủ.
2. **Hỏi: có cần mutate không?** Nếu không, không cần lock.
3. **Hỏi: single-thread hay multi?** Quyết Rc/Arc, RefCell/Mutex.
4. **Hỏi: contention level?** Nếu cao → cân nhắc lock-free (channel, atomic, ArcSwap).
5. **Hỏi: lock duration?** Ngắn → Mutex. Dài + read-heavy → RwLock.
6. **Đo, không đoán** — `cargo bench` với realistic workload.

---

# Tổng kết — 10 nguyên tắc senior

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Smart pointer = ownership tool, không phải GC giả.       │
│                                                             │
│ 2. Box cho single-owner heap. Mặc định.                     │
│                                                             │
│ 3. Rc cho shared single-thread, Arc cho multi-thread.       │
│    Khác biệt là atomic vs non-atomic counter.               │
│                                                             │
│ 4. Cell/RefCell/Mutex/RwLock = interior mutability.         │
│    Bẻ cong quy tắc borrow checker.                          │
│                                                             │
│ 5. Cell: Copy types only. RefCell: any T, runtime check.    │
│                                                             │
│ 6. Mutex serialize. RwLock cho parallel readers.            │
│    Đo trước khi chọn.                                       │
│                                                             │
│ 7. Async: tokio::sync::Mutex CHỈ khi giữ qua await.         │
│    Nếu không, std::sync::Mutex nhanh hơn.                   │
│                                                             │
│ 8. Cycle Rc/Arc + RefCell/Mutex → memory leak.              │
│    Dùng Weak<T> phá cycle.                                  │
│                                                             │
│ 9. Combine: Arc<Mutex<T>>, Rc<RefCell<T>> là idiom.         │
│    Đừng tự reinvent.                                        │
│                                                             │
│ 10. Channel/message passing thường > shared mutable state.  │
│     "Share memory by communicating."                        │
└─────────────────────────────────────────────────────────────┘
```

---

# Liên kết về memory model

Mọi smart pointer có **2 phần**:

```
   Stack: pointer (8 byte trên 64-bit)
              │
              ▼
   Heap:  Control block + data
   
   ┌──────────────────────────┐
   │ Control block (metadata) │  ← strong/weak count, lock state...
   ├──────────────────────────┤
   │ Data T                   │
   └──────────────────────────┘
```

Cost runtime của smart pointer = work với control block:
- `Box`: zero (chỉ deref + drop alloc)
- `Rc`: increment/decrement counter (non-atomic)
- `Arc`: increment/decrement counter (atomic — cache coherence cost)
- `Cell/RefCell`: borrow flag check
- `Mutex/RwLock`: futex syscall khi contend

Trong hot path:
- Atomic ops gây **cache line ping-pong** giữa cores → tránh contention bằng sharding
- Lock contention làm syscall → tránh bằng lock-free structure khi possible
- False sharing: 2 atomic field trên cùng cache line ảnh hưởng lẫn nhau → padding (`#[repr(align(64))]`)

## `clone()` — nơi 2 tầng memory model gặp nhau

`Rc::clone`/`Arc::clone` là ví dụ gọn nhất cho thấy "memory model" thực ra có **2 tầng** và clone chạm cả hai:

```
┌─ Tầng 1: MEMORY LAYOUT (nằm ở đâu) ──────────────────────────────┐
│  clone KHÔNG copy `data: T`. Nó copy 8 byte con trỏ trên stack    │
│  và tăng counter trong control block dùng chung trên heap.        │
│  ⟹ O(1), độc lập với sizeof(T). Đây là "shared ownership".        │
└──────────────────────────────────────────────────────────────────┘
┌─ Tầng 2: CONCURRENCY MEMORY MODEL (atomic ordering) ─────────────┐
│  Rc : counter là Cell<usize> → +1 thường. An toàn vì !Send+!Sync. │
│  Arc: counter là AtomicUsize →                                    │
│        clone → fetch_add(Relaxed)      (4.9: vì sao đủ)            │
│        drop  → fetch_sub(Release) + fence(Acquire)  (4.10–4.11)   │
│  ⟹ Đúng đắn KHÔNG đến từ "atomic" mà từ CHỌN ĐÚNG ORDERING.       │
└──────────────────────────────────────────────────────────────────┘
```

Chốt lại: `Rc/Arc::clone` = "tăng refcount", **không** sao chép dữ liệu — nó cấp thêm *shared ownership* trên cùng một vùng heap. Liên hệ với memory model nằm ở (a) layout có control block nhúng counter cạnh `data`, và (b) với `Arc`, tính đúng đắn phụ thuộc hoàn toàn vào mô hình release/acquire để vừa không data race vừa không đồng bộ thừa.

> Xem thêm góc nhìn từ phía atomic/CPU trong [a-memory-model.md](./a-memory-model.md) — mục *"Atomic trong thực tế: Arc đếm reference"* và Phần V (ordering). Hai tài liệu bổ trợ nhau: ở đây nhìn từ **smart pointer**, bên kia nhìn từ **memory/CPU**.

---

# Crates phổ biến (Senior toolkit)

| Crate | Mục đích |
|-------|----------|
| `parking_lot` | Mutex/RwLock nhanh hơn std |
| `arc_swap` | Lock-free atomic swap Arc |
| `dashmap` | Concurrent HashMap (lock per bucket) |
| `crossbeam` | Channels, queues lock-free |
| `rayon` | Data parallelism, Arc-based |
| `once_cell` | Old version of OnceCell, OnceLock |
| `lazy_static` | Macro cho global lazy (deprecated, dùng LazyLock) |
| `mimalloc` / `jemallocator` | Global allocator nhanh hơn system |

---

# Lộ trình tiếp theo

Bạn đã đầy đủ ngữ vựng smart pointers. Các chủ đề sâu hơn:

- **Unsafe Rust** — raw pointer, UnsafeCell deep, atomic ordering, FFI
- **Testing patterns** — bench với criterion để đo smart pointer cost
- **Iterator deep dive** — iterator combinators, parallel với rayon (dùng Arc internally)
- **Web framework realistic** — apply Arc<Mutex<...>> / Arc<RwLock<...>> trong axum
- **Database** — connection pool (Arc<Pool>), sqlx + smart pointers

Báo chủ đề tiếp theo bạn muốn đi sâu! 🦀⚡
