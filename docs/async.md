# Async/Await trong Rust — Từ Bản Chất Đến Nâng Cao

> Tài liệu thứ 6 trong bộ Rust nền tảng. Khuyến nghị đọc trước:
> - [memory-model.md](./memory-model.md)
> - [ownership-borrowing.md](./ownership-borrowing.md)
> - [trait.md](./trait.md)
> - [generic.md](./generic.md)
> - [closure.md](./closure.md)
>
> Đây là chủ đề khó nhất trong Rust. Nó kết hợp **tất cả** các khái niệm trước:
> ownership, borrow, lifetime, trait, generic, closure, memory layout, smart pointer (`Pin`, `Arc`).
> Đừng vội. Hiểu sâu mỗi tầng trước khi đi tầng tiếp theo.
>
> Tài liệu này dùng `tokio` làm runtime ví dụ (phổ biến nhất). Nhưng nguyên lý áp dụng cho mọi executor (async-std, smol, embassy, glommio...).

---

# Mục lục

- [Tầng 1: Tại sao cần Async?](#tầng-1-tại-sao-cần-async)
- [Tầng 2: Concurrency vs Parallelism vs Asynchrony](#tầng-2-concurrency-vs-parallelism-vs-asynchrony)
- [Tầng 3: Future trait — Trái tim của Async Rust](#tầng-3-future-trait--trái-tim-của-async-rust)
- [Tầng 4: async/await desugar — Compiler sinh State Machine](#tầng-4-asyncawait-desugar--compiler-sinh-state-machine)
- [Tầng 5: Pin — Vì sao Future cần Pin?](#tầng-5-pin--vì-sao-future-cần-pin)
- [Tầng 6: Waker — Cơ chế đánh thức Future](#tầng-6-waker--cơ-chế-đánh-thức-future)
- [Tầng 7: Executor & Reactor — Kiến trúc Runtime](#tầng-7-executor--reactor--kiến-trúc-runtime)
- [Tầng 8: Task, Spawn, Send + 'static](#tầng-8-task-spawn-send--static)
- [Tầng 9: Patterns nâng cao — select!, join!, Stream, Cancellation](#tầng-9-patterns-nâng-cao--select-join-stream-cancellation)
- [Tầng 10: Common Pitfalls — Lỗi thường gặp](#tầng-10-common-pitfalls--lỗi-thường-gặp)

---

# Tầng 1: Tại sao cần Async?

## 1.1 Vấn đề: I/O blocking

Hãy nghĩ về một web server đơn giản. Mỗi request có 2 giai đoạn:

```
[CPU work] → [I/O wait: DB query / HTTP call / file read] → [CPU work]
   1ms              100ms                                       1ms
```

99% thời gian là **chờ I/O**. CPU đang **rảnh** nhưng thread bị "khóa".

### Cách 1: One thread per request (model truyền thống — Apache, PHP-FPM)

```
Thread 1: ──── REQ A ────[wait 100ms]──── REQ A end
Thread 2: ──── REQ B ────[wait 100ms]──── REQ B end
Thread 3: ──── REQ C ────[wait 100ms]──── REQ C end
...
Thread 10000: ──── REQ Z ────[wait 100ms]──── REQ Z end
```

**Vấn đề**:
- Mỗi OS thread tốn ~2-8 MB stack (Linux mặc định) → 10000 threads = 20-80 GB RAM
- Context switch giữa threads tốn CPU (~1-10 microseconds/switch)
- Kernel scheduler không scale tốt với hàng chục nghìn threads
- C10K problem (1999): không thể phục vụ 10000 kết nối đồng thời với model này

### Cách 2: Event loop + callbacks (Node.js, JavaScript browser)

```
Event Loop:
  ┌─────────────────────────────────────┐
  │  REQ A: callback đăng ký khi DB done │
  │  REQ B: callback đăng ký khi DB done │
  │  REQ C: callback đăng ký khi DB done │
  └─────────────────────────────────────┘
         ↓ khi I/O event xảy ra
  Execute callback tương ứng
```

**Ưu điểm**: 1 thread phục vụ 10000+ connections. Tiết kiệm RAM.

**Nhược điểm**: **Callback hell**.

```javascript
db.query(sql, (err, rows) => {
  if (err) return cb(err);
  http.fetch(url, (err, data) => {
    if (err) return cb(err);
    file.write(path, data, (err) => {
      if (err) return cb(err);
      // ... lồng nhau vô tận
    });
  });
});
```

Code khó đọc, khó debug, error handling phải lặp lại.

### Cách 3: async/await — sync code, async execution

Mục tiêu: viết **giống code đồng bộ** nhưng chạy như event loop.

```rust
async fn handle_request() -> Result<String> {
    let rows = db.query(sql).await?;       // dừng ở đây, không block thread
    let data = http.fetch(url).await?;     // dừng ở đây, không block thread
    file.write(path, &data).await?;        // dừng ở đây, không block thread
    Ok(format!("Done: {} rows", rows.len()))
}
```

Đọc như code đồng bộ. Thực thi: **dừng tại `await`, nhường thread cho task khác**, khi I/O ready thì tiếp tục.

## 1.2 Async không phải multi-threading

**Quan trọng** — phải phân biệt rõ:

| Khái niệm | Bản chất |
|-----------|----------|
| **Thread** | OS thread, có stack riêng, scheduler kernel quản lý |
| **Async task** | "Co-routine" trong userspace, executor quản lý, **không có stack riêng** |
| **Parallelism** | Chạy thật sự đồng thời trên nhiều CPU core |
| **Concurrency** | Quản lý nhiều việc xen kẽ trên 1 (hoặc vài) CPU |

Async **chính** là tool để làm **concurrency** (không nhất thiết parallelism). Một single-thread executor (như `tokio::runtime::Builder::new_current_thread()`) cũng có thể chạy hàng nghìn async tasks.

Để có parallelism trong async, bạn cần **multi-thread executor** (`tokio::runtime::Builder::new_multi_thread()`), khi đó các tasks được phân phối qua các threads.

## 1.3 So sánh với các ngôn ngữ khác

| Ngôn ngữ | Mô hình async |
|----------|---------------|
| **C/C++** | Manual: epoll, libuv, hoặc fibers (Boost.Coroutine) |
| **Go** | Goroutines: stackful coroutines, M:N scheduler, runtime tự lo. ~2KB stack ban đầu, growable. |
| **JavaScript (Node)** | Event loop + Promise + async/await. Single-thread (worker threads riêng). |
| **Python** | `asyncio`: coroutines (stackless), event loop. Có GIL → không parallel CPU. |
| **C#** | Task + async/await. ThreadPool quản lý. State machine sinh bởi compiler (giống Rust). |
| **Rust** | Stackless coroutines, state machine compile-time, **runtime tách rời** (chọn tokio, async-std, smol, embassy...). Zero-cost. |

**Triết lý Rust**: Không build executor vào ngôn ngữ. Chỉ định nghĩa **interface** (trait `Future`). Bạn chọn runtime phù hợp use case.

**Trade-off của Rust**:
- Ưu: cực kỳ hiệu quả (no GC, no green thread overhead), zero-cost abstraction, có thể chạy trên embedded (no_std)
- Nhược: phức tạp hơn (Pin, Send + 'static, lifetime của task), runtime fragmentation

## 1.4 Khi nào dùng async, khi nào dùng thread?

**Dùng async khi:**
- Nhiều task I/O-bound (network, file, DB)
- Cần xử lý nhiều kết nối đồng thời (10K+)
- Cần timeout/cancellation tinh vi
- Cần composable I/O patterns (select, race, join)

**Dùng thread khi:**
- Task CPU-bound (tính toán nặng)
- Số lượng ít (< vài trăm)
- Không cần I/O composability phức tạp
- Cần tối đa tốc độ một task riêng lẻ

**Đừng**: chạy CPU-bound trong async task — sẽ **block** executor thread, đánh chìm tất cả task khác trên thread đó.

---

# Tầng 2: Concurrency vs Parallelism vs Asynchrony

## 2.1 Định nghĩa chính xác

**Concurrency**: cấu trúc chương trình thành nhiều "tasks độc lập" có thể tiến triển. **Không nhất thiết chạy đồng thời**.

**Parallelism**: thực thi đồng thời nhiều việc trên nhiều CPU core. **Đây là kỹ thuật chạy**.

**Asynchrony**: model lập trình không-block — gửi yêu cầu rồi làm việc khác, không chờ.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│   CONCURRENCY là "structure"        PARALLELISM là "execution"│
│   ─────────────────────────         ────────────────────────  │
│   How you organize code             How code runs on hardware│
│                                                              │
│   Async/await, generators           Threads on multi-core     │
│   Coroutines                        SIMD                      │
│   Event loops                       GPU                       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Một chương trình **concurrent** có thể chạy:
- Trên 1 core (interleaved — không parallel)
- Trên N core (parallel)

Một chương trình **parallel** thường cũng concurrent (nhưng không bắt buộc — SIMD chẳng hạn).

## 2.2 Ví dụ minh hoạ

### Concurrent nhưng không parallel

Single-thread tokio runtime, chạy 1000 tasks:

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut handles = vec![];
    for i in 0..1000 {
        handles.push(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            i
        }));
    }
    for h in handles { h.await.unwrap(); }
}
```

1000 tasks **xen kẽ** trên 1 thread. Sau ~1 giây tất cả xong (vì khi `.await sleep`, task dừng và nhường thread cho task khác).

### Parallel nhưng không concurrent (theo nghĩa async)

```rust
use rayon::prelude::*;

fn main() {
    let sum: i64 = (0..1_000_000).into_par_iter().sum();
}
```

Tính toán đồng thời trên nhiều core (parallel) nhưng không có async (không có coroutine).

### Cả hai

Multi-thread tokio:

```rust
#[tokio::main]  // mặc định multi-thread
async fn main() {
    // 1000 tasks chia đều trên N worker threads
}
```

## 2.3 M:N scheduling — Vũ trụ của async runtime

Multi-thread executor như tokio dùng mô hình **M tasks chạy trên N threads** (M >> N).

```
Tasks (M):    [T1] [T2] [T3] [T4] [T5] [T6] [T7] ... [T10000]
                ↓
Schedule:     [Work-stealing queue]
                ↓
Threads (N):  [Worker 1]  [Worker 2]  [Worker 3]  [Worker 4]
                ↓             ↓            ↓           ↓
CPU Cores:    [Core 0]    [Core 1]    [Core 2]    [Core 3]
```

Mỗi worker thread có **local queue**. Khi local queue trống, thread "steal" task từ thread khác (work-stealing). Đây là kỹ thuật scheduler hiệu quả nhất hiện tại.

---

# Tầng 3: Future trait — Trái tim của Async Rust

## 3.1 Định nghĩa Future

Trong Rust, mọi giá trị async đều là một **Future**. `Future` là một **trait** trong `std::future`:

```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

Bản chất: một Future là **một state machine** mà bạn có thể "poll" để hỏi:
- "Bạn có giá trị chưa?" → `Poll::Ready(T)` (xong)
- "Chưa, đang chờ I/O" → `Poll::Pending` (sẽ tự thông báo khi xong qua `Waker`)

## 3.2 Future là lazy — không tự chạy

**ĐÂY LÀ ĐIỂM QUAN TRỌNG NHẤT** mà người mới hay nhầm:

```rust
async fn hello() {
    println!("Hello");
}

fn main() {
    let fut = hello();  // KHÔNG IN GÌ. fut chỉ là một state machine "ngủ"
    // chương trình kết thúc, không in "Hello"
}
```

Khác với JavaScript Promise (eager — chạy ngay khi tạo), **Rust Future là lazy — chỉ chạy khi được poll**.

Để poll, bạn cần **executor**. `tokio::main` macro tạo executor và poll Future trả về:

```rust
#[tokio::main]
async fn main() {
    hello().await;  // Bây giờ in "Hello"
}
```

## 3.3 Tự implement một Future thủ công

Để hiểu sâu, ta tự viết một Future "đếm 3 lần rồi xong":

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct CountToThree {
    count: u32,
}

impl Future for CountToThree {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("poll called, count = {}", self.count);
        self.count += 1;
        if self.count >= 3 {
            Poll::Ready(format!("Done after {} polls", self.count))
        } else {
            // Báo executor: tôi chưa xong, hãy poll lại sau
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[tokio::main]
async fn main() {
    let fut = CountToThree { count: 0 };
    let result = fut.await;
    println!("{}", result);
}
```

Output:
```
poll called, count = 0
poll called, count = 1
poll called, count = 2
Done after 3 polls
```

**Lưu ý quan trọng**: nếu trả `Poll::Pending` mà **không gọi `cx.waker().wake()`**, executor sẽ KHÔNG bao giờ poll lại → task treo vĩnh viễn. Đây là **contract** của Future.

## 3.4 Một Future thực tế: Timer

Timer thực sự không nên dùng busy-poll như trên. Phải dùng OS timer (timerfd, kqueue...) và gọi `waker` khi expired:

```rust
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Delay {
    when: std::time::Instant,
    waker: Arc<Mutex<Option<std::task::Waker>>>,
}

impl Future for Delay {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if std::time::Instant::now() >= self.when {
            return Poll::Ready(());
        }

        // Lưu waker để thread riêng đánh thức
        let mut guard = self.waker.lock().unwrap();
        *guard = Some(cx.waker().clone());

        // Spawn thread chờ rồi đánh thức (đây là demo; thực tế dùng I/O reactor)
        let when = self.when;
        let waker = self.waker.clone();
        thread::spawn(move || {
            let now = std::time::Instant::now();
            if now < when {
                thread::sleep(when - now);
            }
            if let Some(w) = waker.lock().unwrap().take() {
                w.wake();
            }
        });
        Poll::Pending
    }
}
```

Đây là mô hình thật sự: Future **đăng ký waker**, một bên khác (thread / reactor / event source) **gọi waker.wake()** khi sự kiện xảy ra, executor poll lại Future, lần này nó `Poll::Ready`.

## 3.5 Future là 1 trait, nhưng có 2 loại implementation

1. **Tự viết bằng tay**: implement `Future` trait trực tiếp (như ở trên — hiếm dùng, chỉ khi viết primitive)
2. **`async fn` / `async { }` block**: compiler tự generate code implement `Future`

```rust
async fn foo() -> i32 { 42 }
// tương đương (xấp xỉ):
fn foo() -> impl Future<Output = i32> {
    async { 42 }
}
```

Tầng 4 sẽ giải mã chính xác compiler sinh gì.

---

# Tầng 4: async/await desugar — Compiler sinh State Machine

## 4.1 Quy tắc vàng: async block = State Machine

Khi bạn viết:

```rust
async fn example() -> u32 {
    let x = step1().await;
    let y = step2(x).await;
    x + y
}
```

Compiler **không** tạo "thread giả" hay "callback chain". Nó tạo một **enum state machine** kiểu:

```rust
enum ExampleStateMachine {
    Start,                                    // chưa bắt đầu
    AwaitingStep1 { fut: Step1Future },      // đang chờ step1
    AwaitingStep2 { x: u32, fut: Step2Future }, // đang chờ step2
    Done,                                    // đã xong
}
```

Mỗi `.await` = **một state transition**. Khi state machine bị poll:
- Nếu đang ở `Start`: gọi `step1()`, chuyển sang `AwaitingStep1`, poll inner future
- Nếu inner `Poll::Pending` → return `Poll::Pending`
- Nếu inner `Poll::Ready(x)` → chuyển sang `AwaitingStep2`, gọi `step2(x)`, poll tiếp
- Tiếp tục cho đến `Done` → return `Poll::Ready(x + y)`

## 4.2 Sơ đồ chuyển trạng thái

```
       poll()             poll() inner  Pending
Start ─────────► AwaitingStep1 ◄──────────────┐
                       │                      │
                       │ inner Ready(x)       │
                       ▼                      │
                AwaitingStep2 ◄───────────────┘
                       │
                       │ inner Ready(y)
                       ▼
                     Done (return x+y)
```

Tại mỗi `Pending`, state machine **lưu lại toàn bộ biến local** đang sống vào struct của state hiện tại. Lần poll sau, lấy ra dùng tiếp.

## 4.3 Memory layout của async fn

Đây là điểm khiến async Rust **khác hoàn toàn** Go/JS:

- **Go**: mỗi goroutine có stack riêng (~2 KB ban đầu, grow). Lưu biến trên stack.
- **JS**: V8 lưu trạng thái trên heap khi `await`.
- **Rust**: **không có stack riêng**. State machine = một **struct** (enum + biến), kích thước **biết tại compile time**, có thể đặt **bất cứ đâu**: stack của caller, heap (`Box<dyn Future>`), trong struct lớn hơn...

Kích thước của một async function = **kích thước của state machine** = **tổng tất cả biến local sống qua mỗi `.await` + tag enum**.

```rust
async fn big() {
    let buffer = [0u8; 4096];  // 4 KB array trên "stack" của future
    foo().await;
    println!("{}", buffer[0]);  // buffer sống qua await
}
// sizeof::<ReturnedFuture>() >= 4096 bytes
```

Đây là lý do tại sao mọi người khuyên: **đừng để biến lớn sống qua `.await`** — sẽ làm phình kích thước Future.

## 4.4 Cargo expand — xem code thật compiler sinh

Cài: `cargo install cargo-expand`. Chạy `cargo expand` để xem desugared code (rất verbose, không hoàn toàn giống state machine thực nhưng giúp hiểu).

Một ví dụ đơn giản:

```rust
async fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Sau expand (đại khái):

```rust
fn add(a: i32, b: i32) -> impl Future<Output = i32> {
    AddFuture { a, b, state: AddState::Start }
}

struct AddFuture { a: i32, b: i32, state: AddState }
enum AddState { Start, Done }

impl Future for AddFuture {
    type Output = i32;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<i32> {
        match self.state {
            AddState::Start => {
                self.state = AddState::Done;
                Poll::Ready(self.a + self.b)
            }
            AddState::Done => panic!("polled after Ready"),
        }
    }
}
```

Tất cả không có heap, không có alloc, không runtime cost — **zero-cost abstraction**.

## 4.5 Nested async — Future trong Future

```rust
async fn outer() {
    inner().await;
}

async fn inner() {
    other().await;
}
```

State machine `outer` chứa state machine `inner` chứa state machine `other`. **Lồng nhau như Russian doll**.

```
sizeof(Outer) = sizeof(state_tag) + sizeof(Inner)
sizeof(Inner) = sizeof(state_tag) + sizeof(Other)
```

Đây là lý do "deep async call chain" có thể có Future cực lớn. Để giảm: `Box::pin(inner())` — đẩy `inner` lên heap, Future cha chỉ chứa con trỏ.

## 4.6 Borrow qua .await

Vì state machine **lưu biến local** trong struct, biến đang **borrow** một biến khác trong cùng struct → tạo **self-referential struct**!

```rust
async fn foo() {
    let s = String::from("hello");
    let r = &s;            // r borrow s
    other().await;          // <-- ở đây cả s và r phải sống qua await
    println!("{}", r);
}
```

State machine:
```rust
struct FooFuture {
    s: String,
    r: &/* ??? */ String,  // borrow CHÍNH struct này → self-ref
    state: FooState,
}
```

Đây chính là lý do `Future` **phải `Pin`**. Tầng 5 sẽ giải thích chi tiết.

---

# Tầng 5: Pin — Vì sao Future cần Pin?

## 5.1 Vấn đề Self-Referential Struct

Hãy nghĩ về memory:

```
Trên heap (hoặc stack):
┌─────────────────────────────────┐
│ FooFuture                       │
│   s: "hello"      <───────┐     │
│   r: &s    ───────────────┘     │  ← con trỏ trỏ vào field cùng struct
│   state: Awaiting               │
└─────────────────────────────────┘
```

`r` trỏ vào `s` — nhưng `s` là field của cùng struct! Nếu struct bị **MOVE** (copy bytes sang địa chỉ mới), `r` vẫn trỏ về địa chỉ cũ → **dangling pointer**!

```
Sau khi move struct sang địa chỉ mới:
┌─────────────────────────────────┐
│ FooFuture (đã free)             │
│   s: ??? (đã free)              │
│   r: &(địa chỉ cũ) ── DANGLING! │
└─────────────────────────────────┘

┌─────────────────────────────────┐
│ FooFuture (địa chỉ mới)         │
│   s: "hello"  (đã copy)         │
│   r: &(địa chỉ CŨ — sai!)       │
│   state: Awaiting               │
└─────────────────────────────────┘
```

Trong Rust thông thường, **mọi struct đều move được** (semantics mặc định). Nhưng với self-referential, move = unsoundness.

## 5.2 Giải pháp: Pin<P>

`Pin<P>` là một **wrapper** xung quanh pointer `P` (như `&mut T`, `Box<T>`, `Arc<T>`...), đảm bảo:

> Giá trị `T` mà `P` trỏ tới **sẽ không bị move ra khỏi vị trí hiện tại** (trừ khi `T: Unpin`).

```rust
pub struct Pin<P> { pointer: P }
```

Cho phép gì:
- Đọc/ghi field bình thường qua `Pin::deref`
- Nhưng **không** thể lấy `&mut T` hoặc `T` ra mà có thể swap/replace giá trị

## 5.3 Unpin — Đa số type vẫn move được

`Unpin` là một **auto trait** (marker trait, tự suy diễn) — đánh dấu "type này SAFE để move dù đang trong Pin".

Hầu hết primitive (`i32`, `String`, `Vec<T>`, `bool`, ...) đều `Unpin`. Bạn vẫn có thể swap chúng trong Pin.

Chỉ vài type **không** Unpin:
- Future do `async fn`/`async {}` sinh ra (có thể self-ref)
- `PhantomPinned` (zero-size marker để **opt-out** Unpin manually)
- Type chứa `!Unpin` field

```rust
use std::marker::PhantomPinned;

struct MyFuture {
    data: String,
    self_ref: *const String,  // raw pointer trỏ vào self.data
    _pinned: PhantomPinned,   // opt-out Unpin
}
```

## 5.4 Tại sao Future::poll nhận Pin<&mut Self>?

Nhìn lại signature:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
```

`self: Pin<&mut Self>` đảm bảo trong suốt thời gian poll, future **không bị move**. Điều này hợp lý vì self-ref bên trong (nếu có) sẽ luôn valid.

Người gọi (executor) phải **pin future** trước khi poll lần đầu tiên. Sau khi pin, không thể move nữa.

## 5.5 Cách Pin trong thực tế

### Pin lên stack với `pin!` macro

```rust
use std::pin::pin;

let fut = some_async_fn();
let mut fut = pin!(fut);  // fut bây giờ là Pin<&mut Future>
fut.as_mut().poll(cx);
```

### Pin lên heap với Box::pin

```rust
let fut = Box::pin(some_async_fn());  // Pin<Box<Future>>
fut.as_mut().poll(cx);
```

`Box::pin` là cách phổ biến nhất. Đẩy future vào heap, sau đó tự động Pin.

## 5.6 Khi nào bạn phải nghĩ về Pin?

**Code thường ngày (90%)**: KHÔNG cần nghĩ về Pin. `.await` lo hết. Compiler tự `pin!` các future ngầm.

**Khi nào cần Pin**:
- Tự implement `Future` (như ví dụ Delay ở Tầng 3)
- Tự implement `Stream` (Tầng 9)
- Lưu trữ Future trong collection (`Vec<Pin<Box<dyn Future>>>`)
- Tự viết executor
- Code interop với async crate khác

## 5.7 Pin projection — Truy cập field bên trong Pin

Nếu bạn có `Pin<&mut MyFuture>` và muốn truy cập field bên trong:

```rust
struct MyFuture {
    inner: SomeFuture,    // field này cũng cần Pin
    flag: bool,           // field này không cần
}

impl Future for MyFuture {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<u32> {
        // SAFETY: ta giữ pin invariant manually
        let this = unsafe { self.get_unchecked_mut() };
        // Pin field inner:
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        inner.poll(cx)
    }
}
```

Crate `pin-project` (rất phổ biến) generate code này tự động và safe:

```rust
use pin_project::pin_project;

#[pin_project]
struct MyFuture {
    #[pin] inner: SomeFuture,
    flag: bool,
}

impl Future for MyFuture {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<u32> {
        let this = self.project();
        this.inner.poll(cx)  // this.inner đã là Pin<&mut SomeFuture>
    }
}
```

---

# Tầng 6: Waker — Cơ chế đánh thức Future

## 6.1 Vấn đề: Khi nào executor biết nên poll lại?

Khi Future trả `Pending`, executor có 2 lựa chọn:

1. **Busy loop**: cứ poll liên tục → tốn 100% CPU vô ích.
2. **Sleep + wait for signal**: executor "ngủ" cho đến khi có sự kiện đánh thức.

Cách 2 là đúng. Nhưng làm sao executor biết phải đánh thức task **nào**? Hàng nghìn task đang `Pending`...

→ Cần một **callback** gắn vào mỗi lần `poll`. Đó là **Waker**.

## 6.2 Context và Waker

```rust
pub struct Context<'a> { /* contains a Waker */ }

impl Context<'_> {
    pub fn waker(&self) -> &Waker;
}
```

`Waker` là **handle** mà future giữ. Khi I/O ready (gói tin đến, timer expired, channel có item...), một bên khác gọi `waker.wake()` → executor được báo "task X cần poll lại".

```rust
// Trong poll của future
if io_not_ready() {
    let waker = cx.waker().clone();
    register_io_callback(move || {
        waker.wake();  // gọi sau khi I/O sẵn sàng
    });
    return Poll::Pending;
}
```

## 6.3 Waker internals

```rust
pub struct Waker {
    waker: RawWaker,
}

pub struct RawWaker {
    data: *const (),
    vtable: &'static RawWakerVTable,
}

pub struct RawWakerVTable {
    clone: unsafe fn(*const ()) -> RawWaker,
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
}
```

Waker = `(data pointer, vtable)`. Vtable cung cấp clone/wake/drop tự custom. Đây là cách `tokio` cài đặt waker rất hiệu quả: `data` = pointer trỏ vào task node trong scheduler, `wake` = đẩy task vào ready queue.

```
Task node trong scheduler:
┌─────────────────────────┐
│ TaskNode                │
│   future: Pin<Box<...>> │ ◄────┐
│   state: Idle/Running/. │      │
│   waker: Waker          │      │
└─────────────────────────┘      │
         ▲                       │
         │ data pointer          │ wake() → push vào ready queue
         │                       │
   ┌─────────────┐               │
   │ Waker       │───────────────┘
   │  data: ptr  │
   │  vtable: ── │── { clone, wake, wake_by_ref, drop }
   └─────────────┘
```

## 6.4 Waker đi cùng task qua các lần poll

Future không tự giữ Waker giữa các lần poll. Mỗi lần poll, executor truyền `cx` mới. **Future phải `clone()` waker** từ context nếu muốn lưu lại để đánh thức sau:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    let my_waker = cx.waker().clone();  // ← lưu để gọi sau
    register_io(my_waker);
    Poll::Pending
}
```

## 6.5 wake() vs wake_by_ref()

- `wake()` consume Waker (move).
- `wake_by_ref()` chỉ borrow — Waker vẫn dùng được sau.

Khi không cần giữ Waker, dùng `wake_by_ref` tránh clone không cần thiết.

## 6.6 Spurious wake — Đôi khi bị đánh thức "vô cớ"

Future contract cho phép: executor **có thể** poll lại dù chưa có ai gọi waker. Đây là **spurious wake**.

→ Future implementation **phải tự kiểm tra trạng thái** mỗi lần poll, không giả định "vì tôi được poll nên I/O đã ready".

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    // PHẢI check lại, không phụ thuộc vào việc waker được wake hay không
    if !io_is_ready() {
        // Đăng ký lại waker
        register_io(cx.waker().clone());
        return Poll::Pending;
    }
    Poll::Ready(read_io())
}
```

## 6.7 So sánh với JavaScript Promise

JS Promise dùng microtask queue và resolve/reject callbacks. Không có khái niệm "Waker tách rời". Promise là **eager** — chạy ngay, resolve khi xong.

Rust Future tách rời 3 thành phần:
1. **Future state machine** — trạng thái
2. **Executor** — quản lý task, gọi poll
3. **Reactor** — nguồn event (I/O, timer), gọi waker khi event ready

Tách rời để: thay reactor (epoll, io_uring, kqueue, IOCP) mà không thay executor. Modular cao.

---

# Tầng 7: Executor & Reactor — Kiến trúc Runtime

## 7.1 Hai thành phần của một runtime

Mọi async runtime (`tokio`, `async-std`, `smol`, `embassy`) đều có 2 thành phần cốt lõi:

```
┌─────────────────────────────────────────┐
│           ASYNC RUNTIME                 │
│                                         │
│  ┌──────────────┐    ┌────────────────┐ │
│  │  EXECUTOR    │    │   REACTOR      │ │
│  │              │    │                │ │
│  │ Chạy tasks   │◄──►│ Watch I/O      │ │
│  │ Poll futures │    │ Timer          │ │
│  │ Schedule     │    │ Signal         │ │
│  │ ready queue  │    │ Wake tasks     │ │
│  └──────────────┘    └────────────────┘ │
└─────────────────────────────────────────┘
```

**Executor**: quản lý tasks, gọi `Future::poll`, đưa vào/lấy ra khỏi ready queue.

**Reactor**: nguồn của events. Dùng OS API:
- Linux: `epoll`, `io_uring`
- macOS/BSD: `kqueue`
- Windows: `IOCP`

Khi event ready (socket có data, timer expired...), reactor gọi `waker.wake()` của task chờ event đó.

## 7.2 Vòng lặp executor (simplified pseudo-code)

```rust
fn executor_loop() {
    loop {
        // 1. Lấy task sẵn sàng từ ready queue
        while let Some(task) = ready_queue.pop() {
            // 2. Poll task
            let cx = Context::from_waker(&task.waker);
            match task.future.as_mut().poll(&mut cx) {
                Poll::Ready(_) => {
                    // task xong, drop
                }
                Poll::Pending => {
                    // task tự đăng ký waker với reactor, sẽ được wake sau
                }
            }
        }

        // 3. Khi không còn task nào ready, gọi reactor block đợi I/O
        let events = reactor.poll();  // syscall epoll_wait / io_uring_enter

        // 4. Mỗi event → wake task tương ứng
        for ev in events {
            ev.waker.wake();  // đẩy task vào ready queue
        }
    }
}
```

Đây là **trái tim của tokio**. Mọi optimization khác là tinh chỉnh trên model này.

## 7.3 Multi-thread executor — Work stealing

Single-thread executor như trên dễ hiểu nhưng chỉ chạy trên 1 core. Tokio mặc định **multi-thread** với **work-stealing**:

```
┌─ Worker 1 ─┐  ┌─ Worker 2 ─┐  ┌─ Worker 3 ─┐  ┌─ Worker 4 ─┐
│ local queue│  │ local queue│  │ local queue│  │ local queue│
│ [T1 T2 T3] │  │ [T4 T5]    │  │ []         │  │ [T6 T7 T8] │
└────────────┘  └────────────┘  └────────────┘  └────────────┘
                                      │
                                      │ Worker 3 empty → steal!
                                      ▼
                              Pick from another worker's queue
                                      │
                                      ▼
                              ┌─ Worker 3 ─┐
                              │ [T7]       │  ← stole from W4
                              └────────────┘

           ┌────────────────────────────┐
           │  Global queue (overflow)   │
           │  [T100 T101 T102 ...]      │
           └────────────────────────────┘
                    ▲
                    │ When local queue full
                    │ When spawned from non-worker thread
```

Mỗi worker:
1. Ưu tiên lấy task từ local queue (cache-friendly, no contention)
2. Nếu trống → steal từ worker khác
3. Nếu hết → pull từ global queue
4. Nếu vẫn hết → park (sleep), chờ reactor wake

Đây là cách **tokio scale lên 64+ cores** rất hiệu quả.

## 7.4 Tokio runtime — Tạo và quản lý

Cách phổ biến nhất:

```rust
#[tokio::main]
async fn main() {
    // ...
}
```

Đây là **macro** mở rộng thành:

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // ...
        });
}
```

`block_on` là cách "chui từ sync vào async": chạy executor cho đến khi future cho trước xong.

### Tự cấu hình runtime

```rust
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)                  // số worker thread
    .thread_name("my-worker")
    .thread_stack_size(2 * 1024 * 1024) // 2MB
    .enable_io()
    .enable_time()
    .build()
    .unwrap();

rt.block_on(async {
    // ...
});
```

### Single-thread runtime

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // chạy hoàn toàn trên 1 thread
}
```

Lợi: không cần `Send` cho task, ít synchronization overhead, nhỏ gọn.

## 7.5 Khi runtime gặp blocking code

Nếu một task `sleep(Duration::from_secs(5))` (sync std::thread::sleep) hoặc làm CPU-bound nặng, **toàn bộ worker thread** bị block. Tasks khác trên cùng worker bị treo!

```
Worker thread: ─[Task A ──── thread::sleep(5s) ────]─[Task B] ...
                                                     ↑
                                       Task B chỉ chạy sau 5s!
```

**Giải pháp**:
- Dùng async version: `tokio::time::sleep` thay vì `std::thread::sleep`
- Với CPU work: `tokio::task::spawn_blocking(|| { ... })` — chuyển vào blocking thread pool riêng

```rust
let result = tokio::task::spawn_blocking(|| {
    expensive_cpu_computation()
}).await.unwrap();
```

`spawn_blocking` chạy closure trên **blocking thread pool** (mặc định 512 thread), không động đến worker threads.

## 7.6 Reactor — Cơ chế I/O notification

Reactor lưu một map từ **file descriptor / OS handle** → **list of wakers**.

```rust
// Pseudo-code
struct Reactor {
    epoll_fd: RawFd,
    wakers: HashMap<RawFd, Vec<Waker>>,
}

impl Reactor {
    fn register(&mut self, fd: RawFd, interest: Interest, waker: Waker) {
        epoll_ctl(self.epoll_fd, ADD, fd, interest);
        self.wakers.entry(fd).or_default().push(waker);
    }

    fn poll_events(&mut self, timeout: Duration) {
        let events = epoll_wait(self.epoll_fd, timeout);
        for ev in events {
            for waker in self.wakers.remove(&ev.fd).unwrap_or_default() {
                waker.wake();
            }
        }
    }
}
```

Một I/O operation như `socket.read().await`:
1. Cố read non-blocking — nếu có data → return Ready
2. Nếu `EWOULDBLOCK` → register waker với reactor, return Pending
3. Reactor sau đó nhận `EPOLLIN` event → wake → poll lại → bước 1 thành công

## 7.7 io_uring — Tương lai của async I/O

Trên Linux gần đây, `io_uring` cung cấp **completion-based** thay vì readiness-based như epoll.

- **epoll**: "Socket sẵn sàng để đọc" → app gọi `read()` syscall → có data
- **io_uring**: "Đọc giùm tôi vào buffer này, báo lại khi xong" → app chờ completion → có data sẵn

io_uring giảm số syscall, hỗ trợ batch, performance cao hơn. Tokio đang thử nghiệm `tokio-uring` crate. Lâu dài có thể là default.

---

# Tầng 8: Task, Spawn, Send + 'static

## 8.1 Task là gì?

**Task** = đơn vị thực thi độc lập trong runtime. Mỗi task chứa 1 Future top-level. Tasks có thể chạy đồng thời (concurrent), được executor lên lịch.

So sánh:
- **Thread**: do OS quản lý, có stack riêng, ~MB RAM mỗi cái
- **Task**: do runtime quản lý, không stack riêng, chỉ tốn bằng kích thước Future

→ Tạo hàng triệu task khả thi. Tạo hàng triệu thread thì không.

## 8.2 spawn — Tạo task mới

```rust
let handle = tokio::spawn(async {
    println!("Hello from task");
    42
});

let result: i32 = handle.await.unwrap();
```

`spawn` trả về `JoinHandle<T>`, là một Future. `await` nó để chờ task xong và lấy kết quả.

### So sánh với .await trực tiếp

```rust
// Sequential:
async fn sequential() {
    let a = task_a().await;  // 1s
    let b = task_b().await;  // 1s
    // total: 2s
}

// Concurrent (cả hai cùng chạy):
async fn concurrent() {
    let ha = tokio::spawn(task_a());
    let hb = tokio::spawn(task_b());
    let a = ha.await.unwrap();
    let b = hb.await.unwrap();
    // total: ~1s (cả hai chạy song song)
}

// Hoặc dùng join!:
async fn with_join() {
    let (a, b) = tokio::join!(task_a(), task_b());
    // total: ~1s
}
```

## 8.3 Tại sao spawn yêu cầu Send + 'static?

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
```

Hai trait bound này gây không ít đau khổ cho người mới. Hiểu vì sao chúng cần:

### `Send`

Multi-thread runtime có thể **di chuyển task giữa các worker threads** (work-stealing). Khi đó, future bị move sang thread khác → phải `Send`.

```rust
let rc = Rc::new(42);  // Rc !Send
tokio::spawn(async move {
    println!("{}", rc);  // ERROR: Rc không Send
});
```

Sửa: dùng `Arc<T>` (Send) thay vì `Rc<T>` (!Send), hoặc dùng single-thread runtime (`current_thread`).

### `'static`

Task có thể chạy **lâu hơn scope tạo nó**. Sau khi `spawn`, future vào một heap struct trong runtime, có thể sống đến hết chương trình.

```rust
fn caller() {
    let s = String::from("hello");
    tokio::spawn(async {
        println!("{}", s);  // ERROR: s không 'static, sẽ drop khi caller return
    });
    // caller return → s drop → task có thể vẫn đang chạy → use after free
}
```

Sửa: **move** vào task:

```rust
fn caller() {
    let s = String::from("hello");
    tokio::spawn(async move {  // ← move s
        println!("{}", s);  // s đi cùng task, OK
    });
}
```

Hoặc dùng `tokio::task::spawn_local` trong `LocalSet` (không cần Send), nhưng task không thể leo qua thread.

### `Sync` không bắt buộc

Tại sao `Sync` không cần? Vì Future là `&mut self` khi poll — chỉ 1 thread access tại một thời điểm. Không có concurrent access tới future state.

## 8.4 JoinHandle — Chờ task xong

```rust
let h = tokio::spawn(async { 42 });

// .await: chờ và lấy kết quả
let val = h.await.unwrap();  // unwrap vì có thể panic/cancel

// abort: huỷ task
h.abort();

// is_finished: kiểm tra
if h.is_finished() { ... }
```

`JoinHandle::abort()` gửi tín hiệu cancel. Task sẽ bị drop ở `.await` point gần nhất (cooperative).

## 8.5 spawn_blocking — Cho CPU/sync code

```rust
let result = tokio::task::spawn_blocking(|| {
    // sync code, có thể block thread
    std::thread::sleep(Duration::from_secs(5));
    expensive_computation()
}).await.unwrap();
```

Chạy closure trên **blocking thread pool** riêng (không phải worker thread). Khi closure xong, kết quả về future.

**Khi nào dùng**:
- Đọc file lớn sync
- CPU-bound nặng (CPU phải burn)
- Gọi library cũ không hỗ trợ async (vd C bindings)

## 8.6 spawn_local — Task không Send

```rust
let local = tokio::task::LocalSet::new();
local.run_until(async {
    let rc = Rc::new(42);  // !Send
    tokio::task::spawn_local(async move {
        println!("{}", rc);  // OK trong LocalSet
    });
}).await;
```

`LocalSet` giữ tasks trên 1 thread, không cần Send. Phù hợp khi bạn có data structure `!Send` (Rc, RefCell, ...).

## 8.7 Task local memory layout

Khi spawn:
```rust
tokio::spawn(future);
```

Tokio:
1. `Box::pin(future)` → heap allocate
2. Tạo `Task` struct chứa: future box, state (idle/running/done), waker, queue link
3. Đẩy vào worker queue

```
Heap:
┌─────────────────────────────┐
│ Task struct                 │
│   future: Pin<Box<dyn ..>>  │
│   state: AtomicU8           │
│   join_waker: Waker         │
│   ref_count: AtomicUsize    │
└─────────────────────────────┘
       ▲
       │ Arc<Task> trỏ vào
       │
┌──────┴──────────┐
│ JoinHandle      │  ← bạn giữ cái này ở phía caller
└─────────────────┘
```

Tokio dùng nhiều optimization: small task pool, inline waker, NUMA awareness... nhưng concept là vậy.

## 8.8 Task cancellation — Cooperative

Rust async cancellation **cooperative**: gọi `abort()` không kill task ngay lập tức. Task được "đánh dấu cancel", và sẽ bị drop tại `.await` point tiếp theo.

```rust
let h = tokio::spawn(async {
    loop {
        let _ = compute_heavy();  // KHÔNG có .await → không thể cancel ở đây
    }
});

h.abort();  // task vẫn chạy loop vô tận!
```

Tasks **không có `.await`** không thể bị cancel. Đây là điểm khác Go (goroutine có preemption).

**Best practice**: chèn `tokio::task::yield_now().await` định kỳ nếu có vòng lặp CPU nặng, hoặc dùng `spawn_blocking` cho CPU work.

## 8.9 Drop khi task chạy dở — Destructor được gọi không?

Khi task bị cancel, **state hiện tại của future bị drop**. Drop chạy bình thường → mọi destructor (Drop impl) chạy.

```rust
struct MustCleanup;
impl Drop for MustCleanup {
    fn drop(&mut self) { println!("cleanup!"); }
}

let h = tokio::spawn(async {
    let _x = MustCleanup;
    tokio::time::sleep(Duration::from_secs(10)).await;
});

tokio::time::sleep(Duration::from_secs(1)).await;
h.abort();
// In ra "cleanup!" khi task drop
```

Đây là điểm mạnh của Rust: RAII trong async cũng hoạt động.

---

# Tầng 9: Patterns nâng cao — select!, join!, Stream, Cancellation

## 9.1 join! — Đợi nhiều futures cùng lúc

```rust
let (a, b, c) = tokio::join!(fetch_a(), fetch_b(), fetch_c());
```

`join!` chạy tất cả future **trong cùng task** (concurrent, không parallel trừ khi mỗi future có nội bộ spawn). Tất cả phải xong mới trả về.

So với `spawn`:
- `join!`: cùng task, không cần Send + 'static, nhưng phải xong tất cả
- `spawn` x3 rồi `.await`: 3 task riêng, mỗi cái Send + 'static, có thể chạy parallel trên multi-thread

**Lưu ý**: `join!` chạy concurrent nhưng vẫn **1 task** → nếu 1 future CPU-heavy không await, các future khác bị block.

### try_join!

```rust
let (a, b) = tokio::try_join!(fetch_a(), fetch_b())?;
```

Giống `join!` nhưng nếu 1 future trả `Err`, các future kia bị cancel, error được propagate ngay.

## 9.2 select! — Đợi cái nào xong trước

```rust
tokio::select! {
    val = fetch_a() => {
        println!("a: {:?}", val);
    }
    val = fetch_b() => {
        println!("b: {:?}", val);
    }
    _ = tokio::time::sleep(Duration::from_secs(5)) => {
        println!("timeout!");
    }
}
```

`select!`:
- Poll tất cả future
- Cái nào xong trước → chạy branch tương ứng
- Các future còn lại bị **drop ngay** (cancel)

Đây là tool **siêu mạnh** cho:
- Timeout: race với `sleep`
- Race nhiều I/O sources
- Cancellation signal

### Cancel safety

Khi `select!` chọn 1 nhánh, các future khác bị drop **giữa chừng**. Vấn đề: trạng thái đang dở có sao không?

Một future **cancel-safe** nếu drop nó giữa chừng vẫn an toàn. Ví dụ:
- `tokio::time::sleep(d)` → cancel-safe
- `socket.read(&mut buf)` → **không** cancel-safe (data có thể đã đọc 1 phần vào buf rồi mất)
- `mpsc::Receiver::recv()` → cancel-safe (channel design cho cancel)

Trước khi dùng future trong `select!`, check tài liệu cancel safety.

## 9.3 Timeout

```rust
use tokio::time::{timeout, Duration};

let result = timeout(Duration::from_secs(5), fetch()).await;
match result {
    Ok(value) => { /* xong trong 5s */ }
    Err(_elapsed) => { /* quá thời gian */ }
}
```

`timeout` về cơ bản là `select!` giữa future và sleep.

## 9.4 Stream — Iterator async

`Stream` là **async version của Iterator**:

```rust
use futures::stream::{Stream, StreamExt};

pub trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
}
```

Trong khi Iterator trả `Option<T>` qua `next()`, Stream trả `Poll<Option<T>>` qua `poll_next()`.

### Dùng Stream

```rust
use futures::stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;

let interval = tokio::time::interval(Duration::from_secs(1));
let mut stream = IntervalStream::new(interval).take(5);

while let Some(_tick) = stream.next().await {
    println!("tick");
}
```

### Stream combinators

Giống Iterator: `map`, `filter`, `fold`, `take`, `skip`, `for_each`, `collect`...

```rust
let sum: i32 = stream::iter(0..10)
    .map(|x| x * 2)
    .filter(|x| futures::future::ready(x % 3 == 0))
    .fold(0, |acc, x| async move { acc + x })
    .await;
```

### async_stream — Generator syntax

Crate `async-stream` cho cú pháp generator:

```rust
use async_stream::stream;

let s = stream! {
    for i in 0..3 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        yield i;
    }
};
```

## 9.5 Channel — Giao tiếp giữa tasks

Async tasks không nên share state qua `Mutex` (sẽ có deadlock potential). Cách idiomatic: **channels**.

### mpsc — Multi-producer, single-consumer

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel::<i32>(100);  // 100 buffer slots

tokio::spawn(async move {
    for i in 0..10 {
        tx.send(i).await.unwrap();
    }
});

while let Some(val) = rx.recv().await {
    println!("got {}", val);
}
```

### oneshot — Single-use, single-consumer

```rust
use tokio::sync::oneshot;

let (tx, rx) = oneshot::channel::<i32>();

tokio::spawn(async move {
    tx.send(42).unwrap();
});

let val = rx.await.unwrap();
```

Phổ biến cho "request/response" pattern.

### broadcast — Multi-producer, multi-consumer (clone messages)

```rust
let (tx, _) = tokio::sync::broadcast::channel(16);
let mut rx1 = tx.subscribe();
let mut rx2 = tx.subscribe();

tx.send(1).unwrap();

assert_eq!(rx1.recv().await.unwrap(), 1);
assert_eq!(rx2.recv().await.unwrap(), 1);  // cả hai nhận
```

### watch — Last value cache

```rust
let (tx, mut rx) = tokio::sync::watch::channel("initial");
tx.send("updated").unwrap();
rx.changed().await.unwrap();
println!("{}", *rx.borrow());  // "updated"
```

Phù hợp config reload, state propagation.

## 9.6 async sync primitives — Mutex, RwLock, Semaphore

### tokio::sync::Mutex

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

let data = Arc::new(Mutex::new(0i32));

let data_clone = Arc::clone(&data);
tokio::spawn(async move {
    let mut guard = data_clone.lock().await;  // .await! không block
    *guard += 1;
});
```

Khác `std::sync::Mutex`: `.lock().await` không block thread. Khi lock taken, task yield, lock available → task được wake.

**Quy tắc**: nếu chỉ giữ lock ngắn (vài lệnh, không await), `std::sync::Mutex` thực ra **nhanh hơn** và OK. Chỉ dùng `tokio::sync::Mutex` khi cần **giữ lock qua `.await`**.

### Semaphore

```rust
use tokio::sync::Semaphore;
let sem = Arc::new(Semaphore::new(10));  // max 10 concurrent

for _ in 0..100 {
    let permit = sem.clone().acquire_owned().await.unwrap();
    tokio::spawn(async move {
        // làm việc
        drop(permit);  // (tự động khi out of scope)
    });
}
```

Giới hạn concurrency. Phổ biến cho rate-limiting outbound requests.

## 9.7 async fn trong trait (Rust 1.75+)

Trước 1.75, trait không hỗ trợ `async fn` native, phải dùng `async_trait` macro (Box<dyn Future>).

Nay (1.75+):

```rust
trait Database {
    async fn query(&self, sql: &str) -> Vec<Row>;
}

struct Postgres;
impl Database for Postgres {
    async fn query(&self, sql: &str) -> Vec<Row> {
        // ...
    }
}
```

Caveat:
- Trả về `impl Future` ẩn → chưa hoàn toàn dyn-compatible (chưa dùng được `dyn Database`)
- Workaround cho dyn: `#[async_trait]` macro hoặc box future thủ công

## 9.8 Async Closure (Rust 1.85+)

```rust
let f = async |x: i32| -> i32 { x * 2 };
let result = f(21).await;
```

Tạo closure trả về Future. Tương đương:

```rust
let f = |x: i32| async move { x * 2 };
```

Async closure mở khả năng higher-order async functions sạch hơn (ví dụ trong stream combinators).

## 9.9 Pattern phức tạp: Graceful Shutdown

```rust
use tokio::sync::broadcast;

async fn run() {
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    for i in 0..10 {
        let mut shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            tokio::select! {
                _ = work(i) => {}
                _ = shutdown_rx.recv() => {
                    println!("task {} cleanup", i);
                }
            }
        });
    }

    // Đợi Ctrl+C
    tokio::signal::ctrl_c().await.unwrap();
    println!("Shutting down...");
    let _ = shutdown_tx.send(());

    // Đợi tasks cleanup (có thể join all handles)
    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

Đây là mẫu chuẩn cho long-running services.

---

# Tầng 10: Common Pitfalls — Lỗi thường gặp

## 10.1 Blocking trong async

**Lỗi**:
```rust
async fn bad() {
    std::thread::sleep(Duration::from_secs(5));  // BLOCK thread!
}
```

**Đúng**:
```rust
async fn good() {
    tokio::time::sleep(Duration::from_secs(5)).await;
}
```

**Tổng quát**: trong async function, ĐỪNG dùng:
- `std::thread::sleep` → dùng `tokio::time::sleep`
- `std::sync::Mutex` khi giữ lock qua await → dùng `tokio::sync::Mutex`
- `std::fs::read` → dùng `tokio::fs::read`
- `reqwest::blocking::get` → dùng `reqwest::get` (async)

## 10.2 Holding MutexGuard across await

```rust
let mutex = std::sync::Mutex::new(0);
async fn bad() {
    let guard = mutex.lock().unwrap();
    other_async().await;  // ← lock held qua await → !Send → spawn fail
    drop(guard);
}
```

**Vấn đề**: `MutexGuard` không Send. Future chứa nó cũng không Send → không spawn được trên multi-thread runtime.

**Đúng**: drop guard trước await:
```rust
async fn good() {
    let value = {
        let guard = mutex.lock().unwrap();
        *guard  // copy ra
    };
    other_async().await;
}
```

## 10.3 Borrow conflict trong select!

```rust
let mut buf = vec![0u8; 1024];
loop {
    tokio::select! {
        result = socket.read(&mut buf) => { ... }
        _ = signal => { break; }
    }
}
```

Nếu `signal` triggers, `socket.read` bị cancel — nhưng `buf` đã được mượn mut. Lần loop sau, có thể buf đã bị partial-write. → **không cancel-safe**.

Solution: dùng buffered framing layer (vd `tokio::codec`), hoặc explicit framing.

## 10.4 Spawn trong loop không bound

```rust
loop {
    let conn = listener.accept().await.unwrap();
    tokio::spawn(handle(conn));  // có thể spawn vô tận!
}
```

DoS: nếu attacker mở 1 triệu kết nối, runtime spawn 1 triệu tasks → OOM.

**Fix**: Semaphore limit:
```rust
let sem = Arc::new(Semaphore::new(1000));
loop {
    let permit = sem.clone().acquire_owned().await.unwrap();
    let conn = listener.accept().await.unwrap();
    tokio::spawn(async move {
        handle(conn).await;
        drop(permit);
    });
}
```

## 10.5 Forgetting to await

```rust
async fn bad() {
    fetch_data();  // ← lỗi: không có .await, Future bị drop mà không chạy
}
```

Compiler thường warn `must_use`. Đừng phớt lờ:

```
warning: unused implementer of `Future` that must be used
```

## 10.6 Detached task — Spawn rồi quên

```rust
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.spawn(async { /* ... */ });
    // không block_on, không await → task có chạy không tuỳ runtime
}
```

Spawn rồi không giữ JoinHandle → "detached task". Tokio multi-thread vẫn chạy nó cho đến khi runtime drop.

Best practice: hoặc `await` handle, hoặc dùng `tokio_util::task::TaskTracker` quản lý.

## 10.7 .await trong locking critical section

```rust
let _g = mutex.lock().await;
let result = expensive_async().await;  // lock held trong toàn bộ thời gian
```

Lock contention sẽ rất tệ. Tách nhỏ:
```rust
let snapshot = {
    let g = mutex.lock().await;
    g.clone()
};
let result = expensive_async_using(snapshot).await;
```

## 10.8 Recursive async fn

```rust
async fn recurse(n: u32) -> u32 {
    if n == 0 { return 0; }
    recurse(n - 1).await + 1  // ← lỗi: kích thước Future vô hạn
}
```

State machine của `recurse` chứa Future của `recurse(n-1)` → đệ quy kích thước.

**Fix**: Box future:
```rust
fn recurse(n: u32) -> Pin<Box<dyn Future<Output = u32> + Send>> {
    Box::pin(async move {
        if n == 0 { return 0; }
        recurse(n - 1).await + 1
    })
}
```

Hoặc dùng `async-recursion` crate.

## 10.9 Tokio runtime nested

```rust
#[tokio::main]
async fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();  // ← lỗi
    rt.block_on(async { ... });  // panic: Cannot start runtime from within runtime
}
```

Không tạo runtime trong runtime. Dùng `Handle::current()` để tham chiếu runtime hiện tại nếu cần.

## 10.10 Future không Send vì biến local

```rust
async fn bad() {
    let rc = Rc::new(5);
    other().await;
    println!("{}", rc);  // ← rc sống qua await
}
// future bao biến rc → không Send
```

Compiler chỉ ra dòng cụ thể. Fix: dùng `Arc` thay `Rc`, hoặc `drop(rc)` trước await.

---

# Tóm tắt — 10 ý cốt lõi của Async Rust

1. **Future = lazy state machine**. Không tự chạy — phải có executor poll.
2. **async/await là syntactic sugar** cho state machine. Compiler generate enum + impl Future.
3. **Memory layout**: future là 1 struct, size biết compile time, lưu mọi biến local sống qua await.
4. **Pin** bảo vệ self-referential struct (future chứa borrow vào chính mình) khỏi bị move.
5. **Waker** là callback để future "đăng ký" với executor "hãy poll lại tôi khi sự kiện X xảy ra".
6. **Executor** (tokio) chạy poll loop trên N worker threads, work-stealing để load balance.
7. **Reactor** dùng OS API (epoll/kqueue/IOCP/io_uring) để watch I/O và gọi waker khi ready.
8. **Task = spawn(future)** — đơn vị độc lập, cần Send + 'static cho multi-thread runtime.
9. **Cancellation cooperative**: gọi abort/drop chỉ có hiệu lực ở `.await` point tiếp theo.
10. **Pitfall lớn nhất**: blocking syscall hoặc giữ MutexGuard qua await → block thread / không Send.

---

# Liên kết về memory model

Nhìn lại với góc nhìn memory:

| Khái niệm async | Memory layout |
|------------------|---------------|
| `async fn foo() -> T` | Function trả về `impl Future` — một struct stack size cố định |
| `.await` | State transition, lưu biến local vào enum variant |
| `Box::pin(future)` | Heap allocate future, prevent move |
| `tokio::spawn(future)` | Future bị Box::pin và lưu trong task struct trên heap |
| Waker | Pointer + vtable (fat-pointer-like) → trỏ vào task node trong scheduler |
| Reactor | Single instance trên runtime, lưu HashMap<fd, Vec<Waker>> |

Async chính là một biểu hiện cụ thể của: **biến function/closure → struct + impl trait**, kết hợp với **scheduling cooperative**.

---

# Bộ tài liệu Rust hoàn thiện

Sau Async, bạn đã có nền tảng đầy đủ:

```
6 trụ cột Rust:
1. memory-model          — Bộ nhớ
2. ownership-borrowing   — Quyền sở hữu
3. trait                 — Polymorphism
4. generic               — Parametric polymorphism
5. closure               — Function as value
6. async/await           — Concurrency model ← BẠN ĐANG Ở ĐÂY
```

**Chủ đề mở rộng** (nếu muốn đi xa hơn):
- Error handling: `Result`, `?`, `thiserror`, `anyhow`
- Macros: `macro_rules!`, procedural macros
- Unsafe Rust: raw pointer, FFI, atomic ordering
- Builders & type-state pattern (đã đụng ở generic)
- Embedded Rust: no_std, embassy
- Web frameworks: axum, actix, rocket
- Database: sqlx, sea-orm, diesel
- Performance: profiling, criterion bench, perf

Chúc bạn hành trình học Rust tốt đẹp! 🦀
