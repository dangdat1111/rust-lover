# Async/Await Rust — Minh Hoạ Trực Quan

> Companion visual cho [async.md](./async.md). Cùng đọc song song.

---

## 1. Bức tranh lớn — Async Rust gồm những gì?

```
                          ASYNC RUST UNIVERSE
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   ┌───────────┐    ┌────────────┐    ┌──────────────┐  │
       │   │ async fn  │ ── │  Future    │ ── │   .await     │  │
       │   │ async{}   │    │   trait    │    │              │  │
       │   └───────────┘    └────────────┘    └──────────────┘  │
       │       │                  │                  │          │
       │       │ compiler         │ Pin<&mut Self>   │          │
       │       │ generates        │ poll(cx) ->      │          │
       │       │                  │   Poll<T>        │          │
       │       ▼                  ▼                  ▼          │
       │   ┌────────────────────────────────────────────────┐   │
       │   │           STATE MACHINE (enum)                 │   │
       │   │  Start → Awaiting1 → Awaiting2 → ... → Done    │   │
       │   └────────────────────────────────────────────────┘   │
       │                       │                                │
       │      ┌────────────────┴─────────────────┐              │
       │      ▼                                  ▼              │
       │  ┌─────────┐                       ┌──────────┐        │
       │  │ Executor│ ◄──── wake() ─────────│ Reactor  │        │
       │  │ (tokio) │                       │ epoll    │        │
       │  │ poll()  │ ───── register ──────►│ io_uring │        │
       │  └─────────┘                       └──────────┘        │
       │       │                                                │
       │       ▼ schedule on                                    │
       │  ┌─────────────────────────────────┐                   │
       │  │ Worker threads (M:N scheduling) │                   │
       │  └─────────────────────────────────┘                   │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Tại sao cần Async? — Vấn đề I/O blocking

### Thread-per-request (model cũ)

```
   1 connection = 1 OS thread = ~2-8 MB stack

   10000 connections cần 20-80 GB RAM!

   ┌──── Thread 1 ────┐   "REQ A" ──[wait DB 100ms]── "done"
   │ Stack 8MB        │
   └──────────────────┘

   ┌──── Thread 2 ────┐   "REQ B" ──[wait DB 100ms]── "done"
   │ Stack 8MB        │
   └──────────────────┘

   ...

   ┌──── Thread 10K ──┐   "REQ Z" ──[wait DB 100ms]── "done"
   │ Stack 8MB        │
   └──────────────────┘

   ↑
   OS scheduler context-switch giữa hàng nghìn threads → tốn CPU
   Kernel struggle, latency tăng
```

### Async model: 1 thread, nhiều tasks

```
                              ┌──────────────────┐
                              │  Worker Thread   │
                              │                  │
   [REQ A → poll → Pending]   │  REQ A           │
   [REQ B → poll → Pending]   │  ┌─ poll ─┐ ─┐   │
   [REQ C → poll → Pending]   │  │       │  ▼   │
            │                 │  REQ B   │ poll │
            │ Events come     │  ┌─ poll ─┐ │   │
            ▼ from reactor    │  │       │  ▼   │
                              │  REQ C   │ poll │
   wake(REQ B) → poll → Ready │  ...     │      │
                              │  swap fast       │
                              └──────────────────┘

   Hàng nghìn tasks chia thời gian trên ÍT thread
   Memory: chỉ size state machine của mỗi task (~hundred bytes)
```

---

## 3. Concurrency vs Parallelism — Khác nhau

```
   CONCURRENCY                          PARALLELISM
   (cấu trúc code)                      (thực thi trên hardware)

   Task A: ─█─█─█─█─█─                  Task A: █████████ (core 0)
   Task B: ──█─█─█─█─                   Task B: █████████ (core 1)
   Task C: █─█─█─█──█                   Task C: █████████ (core 2)

   Xen kẽ trên 1 thread                 Đồng thời trên nhiều core
                                        
   Tokio current_thread:                Tokio multi_thread:
   ┌──────────────┐                     ┌─────┐ ┌─────┐ ┌─────┐
   │   1 thread   │                     │ T0  │ │ T1  │ │ T2  │
   │  1000 tasks  │                     │ tasks│ │tasks│ │tasks│
   └──────────────┘                     └─────┘ └─────┘ └─────┘
   Concurrent only                      Concurrent + Parallel
```

---

## 4. Future trait — Trái tim async

```
   ┌──────────────────────────────────────────────────────┐
   │                                                      │
   │   pub trait Future {                                 │
   │       type Output;                                   │
   │                                                      │
   │       fn poll(                                       │
   │           self: Pin<&mut Self>,    ← Pin để self-ref │
   │           cx: &mut Context<'_>,    ← chứa Waker      │
   │       ) -> Poll<Self::Output>;                       │
   │   }                                                  │
   │                                                      │
   │   pub enum Poll<T> {                                 │
   │       Ready(T),     ← xong, đây là kết quả           │
   │       Pending,      ← chưa xong, sẽ wake() khi xong  │
   │   }                                                  │
   │                                                      │
   └──────────────────────────────────────────────────────┘

   Mỗi Future = 1 state machine có thể bị "hỏi" (poll):
       Hỏi: "Mày có giá trị chưa?"
       Trả: Ready(value) hoặc Pending
```

---

## 5. Future LAZY — Không tự chạy

```
   ❌ Suy nghĩ sai (kiểu JavaScript Promise):
   ─────────────────────────────────────────
   let fut = async_work();   ← bắt đầu chạy ngầm
   // làm việc khác
   let r = fut.await;        ← đợi xong


   ✅ Thực tế Rust:
   ─────────────────────────────────────────
   let fut = async_work();   ← chỉ tạo state machine, KHÔNG CHẠY
                              ┌───────────────────────┐
                              │ FutureStateMachine    │
                              │   state: Start        │
                              │   data: ...           │
                              └───────────────────────┘
                                       ↑ "ngủ"
   
   let r = fut.await;        ← BÂY GIỜ executor mới poll() nó
                              poll → poll → ... → Ready


   📌 Hệ quả:
   let fut = async { panic!("?") };
   // không panic, vì fut chưa chạy

   drop(fut);
   // không panic, vì state machine bị drop mà chưa poll
```

---

## 6. async/await → State Machine

```
   Code bạn viết:
   ───────────────────────────────────
   async fn example() -> u32 {
       let x = step1().await;
       let y = step2(x).await;
       x + y
   }

   Compiler generate (xấp xỉ):
   ───────────────────────────────────
   enum ExampleStateMachine {
       Start,
       AwaitingStep1 { fut: Step1Future },
       AwaitingStep2 { x: u32, fut: Step2Future },
       Done,
   }

   State Transition Diagram:
   ─────────────────────────

         ┌──────────┐
         │  Start   │
         └────┬─────┘
              │ first poll: gọi step1(), chuyển state
              ▼
       ┌──────────────────┐
       │ AwaitingStep1    │ ◄──┐
       │   fut: Step1     │    │ inner Pending → return Pending
       └────────┬─────────┘ ───┘
                │ inner Ready(x)
                ▼
       ┌──────────────────┐
       │ AwaitingStep2    │ ◄──┐
       │   x: u32         │    │ inner Pending → return Pending
       │   fut: Step2     │ ───┘
       └────────┬─────────┘
                │ inner Ready(y)
                ▼
         ┌──────────┐
         │  Done    │ return Poll::Ready(x + y)
         └──────────┘
```

---

## 7. Memory layout của async fn

```
   async fn example() {
       let buffer = [0u8; 4096];   // 4KB array
       fetch().await;
       println!("{}", buffer[0]);  // buffer sống qua await
   }


   Compiler generate state machine:
   ───────────────────────────────────────
   
   sizeof(ExampleStateMachine) ≥ 4096 bytes!

   ┌─────────────────────────────────────────────┐
   │ ExampleStateMachine                         │
   │                                             │
   │ ┌─────────────────────────────────────────┐ │
   │ │ Discriminant tag (u8/u16)               │ │ ← state
   │ ├─────────────────────────────────────────┤ │
   │ │ buffer: [u8; 4096]    ← biến local       │ │  4096 bytes
   │ │                       sống qua await    │ │
   │ ├─────────────────────────────────────────┤ │
   │ │ fetch_future: FetchFuture                │ │  inner future
   │ │   ┌─────────────────────────┐           │ │
   │ │   │ ...nested state...      │           │ │
   │ │   └─────────────────────────┘           │ │
   │ └─────────────────────────────────────────┘ │
   └─────────────────────────────────────────────┘
   
   📌 Nguyên tắc:
   sizeof(future) = max(state_x_data) + tag
   biến chỉ trong 1 state thì chỉ nằm trong variant đó
   biến qua nhiều state → "promote" lên field chung
```

---

## 8. Lồng async — Russian doll futures

```
   async fn outer() {
       inner().await;
   }
   async fn inner() {
       deep().await;
   }
   async fn deep() {
       leaf().await;
   }

   Memory layout:
   ─────────────────────────────
   ┌───────────────────────────────────────────────────────┐
   │ OuterFuture (tag + ...)                               │
   │  ┌───────────────────────────────────────────────┐    │
   │  │ InnerFuture (tag + ...)                       │    │
   │  │  ┌───────────────────────────────────────┐    │    │
   │  │  │ DeepFuture (tag + ...)                │    │    │
   │  │  │  ┌──────────────────────────────┐     │    │    │
   │  │  │  │ LeafFuture                   │     │    │    │
   │  │  │  └──────────────────────────────┘     │    │    │
   │  │  └───────────────────────────────────────┘    │    │
   │  └───────────────────────────────────────────────┘    │
   └───────────────────────────────────────────────────────┘
        Lồng nhau như búp bê Nga

   Có thể rất to → Box::pin để đẩy lên heap, cha chỉ chứa pointer:

   ┌──────────────────────────────────┐
   │ OuterFuture                      │
   │  inner: Pin<Box<InnerFuture>> ───┼───►  Heap
   │                                  │       ┌──────────────┐
   │                                  │       │ InnerFuture  │
   └──────────────────────────────────┘       └──────────────┘
```

---

## 9. Pin — Vì sao Future cần Pin?

```
   Future chứa biến local + borrow vào chính nó:
   ──────────────────────────────────────────────
   async fn foo() {
       let s = String::from("hello");
       let r = &s;             ← borrow nội bộ
       other().await;
       println!("{}", r);      ← r phải sống qua await
   }

   Compiler state machine:
   ┌────────────────────────────────────────────┐
   │ FooFuture                                  │
   │   s: String      ┌─ self-reference         │
   │   r: *const str ─┘                         │
   │   state: ...                               │
   └────────────────────────────────────────────┘
                ▲
                │ r trỏ vào s — cùng struct!

   Nếu future bị MOVE sang địa chỉ khác:
   ──────────────────────────────────────

   Trước move (địa chỉ 0x1000):
   ┌────────────────────────┐
   │ s: "hello"  ←─┐        │
   │ r: 0x1000     │        │
   └───────────────┘        │

   Sau move (sang 0x2000):
   ┌────────────────────────┐
   │ s: "hello"  (copy)     │ ← địa chỉ 0x2000
   │ r: 0x1000 ❌ DANGLING! │ ← vẫn trỏ về địa chỉ cũ
   └────────────────────────┘

   ⟹ Bug nghiêm trọng nếu cho phép move

   Giải pháp: Pin<P>
   ─────────────────
   Pin<&mut Future> hoặc Pin<Box<Future>> 
   bảo đảm future KHÔNG bị move sau khi đã pin.
```

---

## 10. Pin trong thực tế

```
   ┌─────────────────────────────────────────────────────┐
   │  KHI DÙNG .await NGÀY THƯỜNG: KHÔNG CẦN NGHĨ VỀ PIN │
   │                                                     │
   │  await ngầm pin tất cả                              │
   └─────────────────────────────────────────────────────┘

   KHI CẦN NGHĨ:
   ────────────
   • Tự implement Future trait
   • Lưu Future trong collection
   • Trả về Box<dyn Future>

   Cách pin:
   ─────────

   1. Stack pin (zero alloc):
      ────────────────────────
      use std::pin::pin;
      let mut fut = pin!(my_async());
      fut.as_mut().poll(cx);
      ┌──────────────────────────┐
      │ Stack frame              │
      │  ┌────────────────────┐  │
      │  │ Future bytes here  │  │ ◄── Pin<&mut> trỏ vào đây
      │  └────────────────────┘  │
      └──────────────────────────┘


   2. Heap pin (Box::pin):
      ──────────────────
      let fut = Box::pin(my_async());
      
      Stack             Heap
      ┌──────────┐      ┌────────────────┐
      │ fut: Box │ ───► │ Future bytes   │ ← Pin protect
      └──────────┘      └────────────────┘
      
      Phổ biến nhất; vì Box owned → future không thể move.


   3. Pin projection (truy cập field):
      ─────────────────────────────
      use pin_project::pin_project;
      
      #[pin_project]
      struct MyFuture {
          #[pin] inner: SomeFuture,   ← cần pin
          flag: bool,                 ← không pin
      }
      
      let this = self.project();      ← safe API
      this.inner.poll(cx);            ← this.inner là Pin<&mut SomeFuture>
```

---

## 11. Unpin — Type vẫn move được

```
   ┌─────────────────────────────────────┐
   │   Unpin                             │
   │   ─────                             │
   │   Auto-trait. Đánh dấu              │
   │   "type này SAFE để move khi pin"   │
   └─────────────────────────────────────┘

       Type             Unpin?
       ─────            ──────
       i32              ✅ Yes
       String           ✅ Yes  
       Vec<T>           ✅ Yes
       HashMap          ✅ Yes
       Box<T>           ✅ Yes
       Mutex<T>         ✅ Yes

       async fn / async {} → returned Future:   ❌ NO (have to assume self-ref)
       PhantomPinned                            ❌ NO (manual opt-out)

   Nếu T: Unpin thì Pin<&mut T> ≈ &mut T.
   Pin chỉ "có ý nghĩa" với !Unpin types.
```

---

## 12. Waker — Cơ chế đánh thức

```
   FUTURE ────► (poll trả Pending) ─────► EXECUTOR
                       │                       │
                       │ "khoan tôi chưa xong" │
                       │ "tôi đăng ký waker"   │ "tôi sẽ ngủ"
                       ▼                       │
              REACTOR / EVENT SOURCE           │
              (epoll, timer, channel...)       │
                       │                       │
                       │ I/O ready / timer fire│
                       │ → gọi waker.wake()    │
                       ▼                       │
                  push task vào ready queue ───┤
                                               ▼
                                           "OK, poll lại"
                                               │
                                               ▼
                                          Future poll → Ready(T)


   Waker structure:
   ────────────────
   ┌──────────────────────────────────┐
   │ Waker                            │
   │   data:  *const ()  ────► Task   │
   │   vtable: { clone,wake,drop } ── │── implement
   └──────────────────────────────────┘   tự định nghĩa
                                          (vd: push task vào queue)
```

---

## 13. Vòng đời 1 lần poll

```
   ┌────────────────────────────────────────────────────────────┐
   │                                                            │
   │  1. Executor lấy task ra khỏi ready queue                  │
   │             │                                              │
   │             ▼                                              │
   │  2. Tạo Context với task's Waker                           │
   │             │                                              │
   │             ▼                                              │
   │  3. future.poll(Pin<&mut self>, &mut cx)                   │
   │             │                                              │
   │  ┌──────────┴─────────┐                                    │
   │  ▼                    ▼                                    │
   │ Ready(T)            Pending                                │
   │  │                    │                                    │
   │  ▼                    ▼                                    │
   │ Done, drop      Future đã clone Waker và                   │
   │                 lưu cho một event source                   │
   │                       │                                    │
   │                       ▼                                    │
   │              Executor không re-schedule                    │
   │              Task ngủ cho đến khi waker.wake()             │
   │                       │                                    │
   │                       ▼                                    │
   │             (vài thời gian sau)                            │
   │             Event source thấy I/O ready                    │
   │             gọi waker.wake() ─► push vào ready queue       │
   │                       │                                    │
   │                       ▼                                    │
   │             [Back to step 1]                               │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
```

---

## 14. Executor + Reactor — Kiến trúc tokio

```
   ┌──────────────────────────────────────────────────────────────┐
   │                       TOKIO RUNTIME                          │
   │                                                              │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │                    EXECUTOR                            │  │
   │  │                                                        │  │
   │  │   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │  │
   │  │   │ Worker 0 │ │ Worker 1 │ │ Worker 2 │ │ Worker 3 │  │  │
   │  │   │ ┌──────┐ │ │ ┌──────┐ │ │ ┌──────┐ │ │ ┌──────┐ │  │  │
   │  │   │ │Local │ │ │ │Local │ │ │ │Local │ │ │ │Local │ │  │  │
   │  │   │ │queue │ │ │ │queue │ │ │ │queue │ │ │ │queue │ │  │  │
   │  │   │ └──────┘ │ │ └──────┘ │ │ └──────┘ │ │ └──────┘ │  │  │
   │  │   └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘  │  │
   │  │        │            │            │            │        │  │
   │  │        └────────────┴────────────┴────────────┘        │  │
   │  │                     │                                  │  │
   │  │              ┌──────▼──────┐                           │  │
   │  │              │  Global     │                           │  │
   │  │              │  injection  │                           │  │
   │  │              │  queue      │                           │  │
   │  │              └─────────────┘                           │  │
   │  └────────────────────────────────────────────────────────┘  │
   │                          │                                   │
   │                          │ wake() pushes task                │
   │                          ▼                                   │
   │  ┌────────────────────────────────────────────────────────┐  │
   │  │                     REACTOR                            │  │
   │  │   ┌────────────────────────────────────────────────┐   │  │
   │  │   │  epoll fd / io_uring                           │   │  │
   │  │   │                                                │   │  │
   │  │   │  fd → wakers map                               │   │  │
   │  │   │   42 → [W_taskA, W_taskC]                      │   │  │
   │  │   │   43 → [W_taskB]                               │   │  │
   │  │   │  ...                                           │   │  │
   │  │   └────────────────────────────────────────────────┘   │  │
   │  │            ▲                                           │  │
   │  │            │ epoll_wait() returns events               │  │
   │  └────────────┴───────────────────────────────────────────┘  │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
                                  │
                                  │ system calls
                                  ▼
                       ┌──────────────────────┐
                       │   LINUX KERNEL       │
                       │  (epoll, io_uring)   │
                       └──────────────────────┘
```

---

## 15. Work-stealing — Cân bằng tải

```
   Trước work-stealing:

   Worker 0: [T1 T2 T3 T4 T5]   ←─ busy
   Worker 1: [T6 T7]            ← bình thường
   Worker 2: []                 ← rỗng, ngủ
   Worker 3: []                 ← rỗng, ngủ
   
   Tổng utilization: chỉ 50%


   Sau work-stealing:

   Worker 0: [T1 T2 T3]         ← bình thường
   Worker 1: [T6 T7]            ← bình thường
   Worker 2: [T4]               ← stole from W0
   Worker 3: [T5]               ← stole from W0
   
   Tổng utilization: 100% 🎉


   Cơ chế steal:
   ─────────────
   • Worker rỗng nhìn local queue worker khác
   • Lấy 1/2 số task từ tail của queue đó
   • Push vào local queue mình
   • Race-free vì queue dùng atomic CAS

   ┌────────────────┐         ┌────────────────┐
   │ Worker 0       │         │ Worker 3       │
   │ ┌────────────┐ │  steal  │ ┌────────────┐ │
   │ │ T1 T2 T3 ─⊕── halve──► │ ─⊕─ T3       │ │
   │ │            │ │         │ │            │ │
   │ └────────────┘ │         │ └────────────┘ │
   │ pop from head  │         │ pop from head  │
   └────────────────┘         └────────────────┘
```

---

## 16. spawn — Tạo task

```
   tokio::spawn(future)
        │
        ▼
   ┌─────────────────────────────────────┐
   │ 1. Box::pin(future) on heap         │
   │                                     │
   │    Heap: ┌──────────────┐           │
   │          │ Future bytes │ ← pinned  │
   │          └──────────────┘           │
   │                                     │
   │ 2. Wrap in Task struct              │
   │                                     │
   │    ┌────────────────────────┐       │
   │    │ Task                   │       │
   │    │   future: Pin<Box<…>>  │       │
   │    │   state: AtomicU8      │       │
   │    │   waker: Waker         │       │
   │    │   ref_count: Atomic    │       │
   │    └────────────────────────┘       │
   │                                     │
   │ 3. Push into worker's local queue   │
   │                                     │
   │ 4. Return JoinHandle to caller      │
   └─────────────────────────────────────┘
        │
        ▼
   Caller giữ JoinHandle:
        │
        │ .await để chờ + lấy kết quả
        │ .abort() để cancel
        ▼
   Future bị poll trên worker → khi Ready → drop
```

---

## 17. Send + 'static — Tại sao spawn cần

```
   tokio::spawn<F>(f: F)
   where F: Future + Send + 'static
                       ▲       ▲
                       │       │
                       │       └── Task có thể sống lâu hơn caller frame
                       │           → không capture biến local ngắn hạn
                       │
                       └── Multi-thread runtime move task giữa các worker
                           → future bị move giữa các thread → cần Send


   Lỗi điển hình:

   ❌ Rc trong async block:
   ─────────────────────────
   let rc = Rc::new(5);           // Rc !Send
   tokio::spawn(async move {       // ← lỗi: future containing Rc !Send
       println!("{}", rc);
   });
   
   Fix: Arc thay Rc, hoặc spawn_local trong LocalSet.


   ❌ Reference vào biến local:
   ──────────────────────────
   let s = String::from("hi");
   tokio::spawn(async {            // ← lỗi: &s sống không 'static
       println!("{}", s);
   });
   
   Fix: move vào async block:
   ──────────────────────────
   tokio::spawn(async move {       // OK: s di chuyển vào task
       println!("{}", s);
   });
```

---

## 18. join! vs select! vs spawn — Khác biệt

```
   ────────────────────────────────────────────────────
                 join!                  select!
   ────────────────────────────────────────────────────
   Behavior:    đợi TẤT CẢ          đợi 1 cái xong trước
                xong, lấy 
                tuple kết quả        cái còn lại bị DROP

   ────────────────────────────────────────────────────
   Use case:    fetch song song      timeout, race,
                kết hợp kết quả      cancellation

   ────────────────────────────────────────────────────
   Cancel:      không cancel         cancel khi 1 cái 
                                     xong trước

   ────────────────────────────────────────────────────
   Send + 'static? Không cần (cùng task)
   ────────────────────────────────────────────────────


   spawn:        chạy task riêng
   ─────         ─────────────
                 yêu cầu Send + 'static
                 có thể parallel (multi-thread runtime)
                 cần JoinHandle để chờ
                 abort() để cancel


   Trực quan:

   join!(A, B, C):  ──[A─┐
                    ──[B─┼─→ all done → tuple (a,b,c)
                    ──[C─┘

   select!(A,B,C):  ──[A─┐
                    ──[B─┼─→ first done → run branch, cancel others
                    ──[C─┘

   3x spawn:      A: [══════]  worker 0
                  B:    [══════]  worker 1
                  C:       [══════]  worker 2
                  ↑ parallel thật
```

---

## 19. Cancellation — Cooperative

```
   Rust async cancellation = COOPERATIVE
   ─────────────────────────────────────

                handle.abort()
                       │
                       ▼ đánh dấu task = cancelled
            Task tiếp tục chạy như bình thường
                       │
                       ▼ đến .await tiếp theo
            ┌──────────────────────────┐
            │   .await checkpoint      │
            │                          │
            │   Tokio kiểm tra:        │
            │   "task cancelled?"      │
            │   → drop future         │
            │   → destructor chạy      │
            └──────────────────────────┘


   Code không có .await không thể cancel:
   ──────────────────────────────────────

   ❌ Không bao giờ cancel được:
   tokio::spawn(async {
       loop {
           heavy_compute();    // ← không có .await
       }
   });
   handle.abort();             // ← vô tác dụng


   ✅ Cancellable:
   tokio::spawn(async {
       loop {
           heavy_compute();
           tokio::task::yield_now().await;  // ← cancel point
       }
   });


   Khác Go (preemptive):
   Go scheduler có thể CƯỚP goroutine ra giữa chừng (kể cả CPU code).
   Rust không — bạn phải tự "đưa cơ hội" qua .await.
```

---

## 20. Stream — Iterator của thế giới async

```
   Iterator (sync):              Stream (async):
   ──────────────────            ──────────────────
   trait Iterator {              trait Stream {
       type Item;                    type Item;
       fn next(&mut self)            fn poll_next(
          -> Option<Self::Item>;        self: Pin<&mut Self>,
   }                                    cx: &mut Context
                                    ) -> Poll<Option<Self::Item>>;
                                 }


   Iterator next() chỉ trả Some/None
   Stream poll_next trả Poll<Option>:
       Poll::Ready(Some(item))   ← có item
       Poll::Ready(None)         ← stream kết thúc
       Poll::Pending             ← chưa có, sẽ wake


   Sử dụng:
   ─────────
   use futures::stream::StreamExt;
   
   while let Some(item) = stream.next().await {
       println!("got {}", item);
   }
   
   ─ Tương đương while let với iter, nhưng có .await


   Stream combinator chain:
   ────────────────────────
   stream
     .map(|x| x * 2)
     .filter(|x| ready(x > 5))
     .take(10)
     .for_each(|x| async move { println!("{}", x); })
     .await;
```

---

## 21. Channel — Giao tiếp giữa tasks

```
   mpsc — multi producer, single consumer:
   ───────────────────────────────────────
   let (tx, mut rx) = mpsc::channel(100);
   
   ┌──────────┐ tx.clone()  ┌──────────────────┐
   │ Task A   │────────────►│                  │
   │  tx ─┐   │             │     Channel      │
   └──────┘   │             │   (buffer 100)   │   ┌──────────┐
              │             │   [_,_,_,_,_,_]  │──►│  Task R  │
   ┌──────────┴────┐        │                  │   │   rx     │
   │ Task B        │───────►│                  │   └──────────┘
   │  tx ─┐        │        └──────────────────┘
   └──────┘        │
                   │
   ┌───────────────┴┐
   │ Task C         │───────►
   └────────────────┘

   tx.send(val).await: chờ buffer có slot
   rx.recv().await:   chờ có message


   oneshot — single use:
   ─────────────────────
   let (tx, rx) = oneshot::channel();
   
   tokio::spawn(async move { tx.send(42).unwrap(); });
   let val = rx.await.unwrap();
   
   Use case: request → response


   broadcast — N producer, M consumer (clone messages):
   ─────────────────────────────────────────────────
   let (tx, _) = broadcast::channel(16);
   let mut r1 = tx.subscribe();
   let mut r2 = tx.subscribe();
   
   tx.send(1);
   // cả r1.recv() và r2.recv() nhận được 1


   watch — last-value cache:
   ─────────────────────────
   let (tx, mut rx) = watch::channel("a");
   tx.send("b");
   rx.changed().await;
   *rx.borrow();   // "b"
```

---

## 22. async Mutex vs sync Mutex

```
   ┌─────────────────────────────────────────────────────┐
   │ std::sync::Mutex                                    │
   │ ───────────────                                     │
   │ • lock()    → MutexGuard (sync, BLOCK thread)       │
   │ • Nhanh khi không contention                        │
   │ • Guard !Send (Linux) → future chứa guard !Send     │
   │ • DÙNG: lock ngắn, không await trong lock           │
   └─────────────────────────────────────────────────────┘
   
   ┌─────────────────────────────────────────────────────┐
   │ tokio::sync::Mutex                                  │
   │ ─────────────────                                   │
   │ • lock().await → MutexGuard (async)                 │
   │ • Khi lock taken: task yield, không block thread    │
   │ • Guard Send → future Send                          │
   │ • DÙNG: cần giữ lock qua .await                     │
   └─────────────────────────────────────────────────────┘

   ❌ Lỗi điển hình:
   ──────────────────
   let m = std::sync::Mutex::new(0);
   async fn bad() {
       let g = m.lock().unwrap();      // ← std MutexGuard !Send
       other().await;                   // ← guard sống qua await
       drop(g);
   }
   // → Future không Send → spawn fail trên multi-thread runtime


   ✅ Sửa 1 — drop trước await:
   ──────────────────────────
   async fn good_v1() {
       let value = {
           let g = m.lock().unwrap();
           *g  // copy ra
       };
       other().await;
   }


   ✅ Sửa 2 — dùng tokio Mutex:
   ─────────────────────────
   let m = tokio::sync::Mutex::new(0);
   async fn good_v2() {
       let g = m.lock().await;     // ← async lock
       other().await;               // OK
       drop(g);
   }
```

---

## 23. Sơ đồ quyết định — Async hay Thread?

```
                    Cần concurrent processing?
                              │
              ┌───────────────┴───────────────┐
              │                               │
             Yes                              No
              │                               │
              ▼                          [chương trình thường]
       Loại công việc?
              │
   ┌──────────┼────────────┬──────────────┐
   │          │            │              │
 I/O bound  CPU bound   Mixed         Embedded
   │          │            │              │
   ▼          ▼            ▼              ▼
 ASYNC     THREAD       ASYNC          ASYNC
 (tokio)   (rayon /    + spawn_       (embassy /
           std::thread) blocking      smol)
                       cho CPU


   Số connections / tasks?
   ───────────────────────
   < 100         → Cả 2 đều OK
   100 - 10K     → Async ưu thế
   10K - 1M+     → Async bắt buộc


   Cần parallel CPU thật sự?
   ─────────────────────────
   Có → multi-thread runtime hoặc rayon
   Không → current-thread runtime đủ
```

---

## 24. Common Pitfalls — 6 lỗi điển hình

```
   ┌──────────────────────────────────────────────────────────┐
   │ 1. Blocking syscall trong async                          │
   │                                                          │
   │    ❌ std::thread::sleep(...)                            │
   │    ❌ std::fs::read(...)                                 │
   │    ❌ blocking HTTP client (reqwest::blocking)           │
   │                                                          │
   │    ✅ tokio::time::sleep                                 │
   │    ✅ tokio::fs::read                                    │
   │    ✅ reqwest::get (async)                               │
   │    ✅ spawn_blocking cho code không thể chuyển           │
   └──────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────┐
   │ 2. Holding MutexGuard qua await                          │
   │                                                          │
   │    let g = mutex.lock();                                 │
   │    fetch().await;          ← lock vẫn giữ, BAD          │
   │                                                          │
   │    Hậu quả: future !Send (std), hoặc deadlock potential  │
   │    Fix: drop guard trước await, hoặc tokio::sync::Mutex  │
   └──────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────┐
   │ 3. Quên .await                                           │
   │                                                          │
   │    fetch();   ← compile warning: must_use Future         │
   │                                                          │
   │    Future bị drop ngay → KHÔNG CHẠY                      │
   │    Fix: thêm .await                                      │
   └──────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────┐
   │ 4. Recursive async không Box                             │
   │                                                          │
   │    async fn recurse(n) -> ... {                          │
   │        recurse(n - 1).await + 1   ← size vô hạn!         │
   │    }                                                     │
   │                                                          │
   │    Fix: return Pin<Box<dyn Future>> hoặc                 │
   │         async-recursion crate                            │
   └──────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────┐
   │ 5. Spawn unbounded                                       │
   │                                                          │
   │    loop {                                                │
   │        let conn = accept().await;                        │
   │        tokio::spawn(handle(conn));  ← DoS risk!         │
   │    }                                                     │
   │                                                          │
   │    Fix: dùng Semaphore giới hạn concurrency              │
   └──────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────┐
   │ 6. Cancel-unsafe future trong select!                    │
   │                                                          │
   │    select! {                                             │
   │        x = socket.read(&mut buf) => {...}                │
   │        _ = signal => break,                              │
   │    }                                                     │
   │    ↑ nếu signal triggers, read bị cancel mid-way,        │
   │      buf có thể partial-write                            │
   │                                                          │
   │    Fix: kiểm tra cancel safety của future trước khi select│
   │         dùng framing layer (codec)                       │
   └──────────────────────────────────────────────────────────┘
```

---

## 25. Mind Map cuối — Async Rust tổng hợp

```
                              ASYNC RUST
                                  │
        ┌─────────────┬───────────┼───────────┬───────────────┐
        │             │           │           │               │
     SYNTAX        FUTURE      RUNTIME    PATTERNS         PITFALLS
        │             │           │           │               │
     async fn      Future trait  tokio     join! select!    blocking
     async {}      poll/Poll    executor   timeout          MutexGuard
     .await        Pin          reactor    Stream           recursion
     async closure Unpin        spawn      channel          forget await
                   Waker        worker     async Mutex      unbounded
                   Context      work-      cancellation     spawn
                                stealing                    cancel safe


                              STATE MACHINE
                              ─────────────
                     ┌──────────────────────────┐
                     │ enum Variants:           │
                     │   Start                  │
                     │   Awaiting_n { locals }  │
                     │   Done                   │
                     └──────────────────────────┘
                              compiler tự sinh


                            MEMORY MODEL
                            ────────────
                       Future = struct stack-size
                       Có thể: stack, Box, Arc<Mutex<>>
                       Pin → không thể move
                       Borrow qua await → self-ref struct
```

---

## 26. Bộ tài liệu Rust trọn vẹn

```
   ┌──────────────────────────────────────────────────────────┐
   │              RUST FOUNDATIONS LIBRARY                    │
   │  ─────────────────────────────────────────────────────   │
   │                                                          │
   │   1. memory-model           — Bộ nhớ                     │
   │      memory-model-visual                                 │
   │                                                          │
   │   2. ownership-borrowing    — Quyền sở hữu              │
   │      ownership-borrowing-visual                          │
   │                                                          │
   │   3. trait                  — Polymorphism              │
   │      trait-visual                                        │
   │                                                          │
   │   4. generic                — Parametric polymorphism   │
   │      generic-visual                                      │
   │                                                          │
   │   5. closure                — Function as value         │
   │      closure-visual                                      │
   │                                                          │
   │   6. async                  — Concurrency model         │
   │      async-visual            ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ─────────────────────────────────────────────────────   │
   │                                                          │
   │   Tổng: 12 files, ~700 KB Markdown                       │
   │                                                          │
   │   Bạn đã có nền tảng cốt lõi của Rust 🦀                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề mở rộng (gợi ý)

Sau khi vững 6 trụ cột, có thể đi sâu các nhánh thực hành:

- **Error handling**: Result, ?, thiserror, anyhow
- **Macros**: macro_rules!, procedural macros (derive, attribute, function)
- **Unsafe Rust**: raw pointer, FFI với C/C++, atomic ordering nâng cao
- **Type-state pattern**: builder, session type (đã đụng ở generic)
- **Web frameworks**: axum, actix-web, rocket — apply async vào HTTP server
- **Database async**: sqlx, sea-orm
- **Performance**: criterion bench, perf, flamegraph, profile-guided optimization
- **Embedded Rust**: no_std, embassy (async cho microcontroller!)
- **WASM**: Rust + WebAssembly cho browser/server
- **GUI**: egui, iced, tauri

Học vui! 🦀⚡
