# Design Patterns trong Rust — Visual Companion

> File minh hoạ ASCII đi kèm [y-design-patterns.md](./y-design-patterns.md).
> Đọc song song. Mỗi sơ đồ ở đây tương ứng một mục lý thuyết.

---

## Mục lục visual

1. [Bản đồ lớn: pattern GoF → Rust](#v1-ban-do-lon)
2. [Vì sao GoF sinh ra từ OOP — bảng đối chiếu](#v2-gof-oop)
3. [4 nguyên tắc nền tảng](#v3-nguyen-tac)
4. [Newtype — zero-cost wrapping](#v4-newtype)
5. [Builder — flow](#v5-builder)
6. [Typestate — state machine ở compile time](#v6-typestate)
7. [RAII — vòng đời drop](#v7-raii)
8. [Strategy — 3 cách dispatch](#v8-strategy)
9. [Static vs Dynamic dispatch — memory](#v9-dispatch-memory)
10. [Observer: classic (sai) vs channel (đúng)](#v10-observer)
11. [State: enum vs typestate](#v11-state)
12. [Decorator / tower middleware — onion](#v12-decorator)
13. [Composite — enum đệ quy](#v13-composite)
14. [Bridge — hai trục độc lập](#v14-bridge)
15. [Actor model — message passing](#v15-actor)
16. [Concurrency patterns — pipeline & fan-out](#v16-concurrency)
17. [Hexagonal architecture](#v17-hexagonal)
18. [Expression problem — enum vs trait](#v18-expression)
19. [Case study: từ nhỏ đến lớn](#v19-case-study)
20. [Antipatterns — hình ảnh](#v20-antipatterns)
21. [Decision tree tổng](#v21-decision-tree)
22. [Mind map tổng kết](#v22-mind-map)

---

<a name="v1-ban-do-lon"></a>
## 1. Bản đồ lớn: pattern GoF → Rust

```
                    23 PATTERN GoF (1994)
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
   │ TAN VÀO     │   │ BIẾN NHẸ HƠN│   │ VẪN CẦN,    │
   │ NGÔN NGỮ    │   │             │   │ VIẾT KIỂU   │
   │ (~30%)      │   │ (~40%)      │   │ RUST (~30%) │
   ├─────────────┤   ├─────────────┤   ├─────────────┤
   │ Iterator    │   │ Strategy    │   │ Builder     │
   │ Null Object │   │  → closure  │   │ Decorator   │
   │  → Option   │   │ Command     │   │ Repository  │
   │ Prototype   │   │  → closure  │   │ Observer    │
   │  → Clone    │   │ Visitor     │   │  → channel  │
   │ Template M. │   │  → match    │   │ Bridge      │
   │  → default  │   │ State       │   │ Composite   │
   │    method   │   │  → enum     │   │  → enum+Box │
   │             │   │ Factory     │   │ Facade      │
   │             │   │  → fn       │   │ Flyweight   │
   └─────────────┘   └─────────────┘   └─────────────┘

          + ĐẶC SẢN RUST (không có trong GoF):
   ┌────────────────────────────────────────────────┐
   │ Newtype · Typestate · RAII guard · Sealed trait │
   │ Extension trait · PhantomData · Interior mut.    │
   │ Entry API · Cow                                  │
   └────────────────────────────────────────────────┘
```

---

<a name="v2-gof-oop"></a>
## 2. Vì sao GoF sinh ra từ OOP — bảng đối chiếu

```
BỆNH CỦA OOP 1994            THUỐC (GoF pattern)      RUST CÓ BỆNH?
────────────────────────────────────────────────────────────────────
Không có sum type        →   Visitor, State,      →   KHÔNG (có enum)
(chỉ class hierarchy)         Composite (class)         match thay thế

Không có closure         →   Strategy, Command    →   KHÔNG (Fn/FnMut)
first-class                   (interface + class)       closure thay thế

Có null                  →   Null Object          →   KHÔNG (Option<T>)

Duyệt lộ cấu trúc        →   Iterator (class)     →   KHÔNG (built-in
                                                        Iterator trait)

Inheritance cứng nhắc    →   Bridge, Decorator,   →   KHÔNG inheritance,
                              Template Method           dùng composition

Tạo object phức tạp      →   Builder, Factory     →   CÓ (thiếu named/
                                                        optional args)
                                                        → Builder ĐẮT GIÁ

Aliasing tự do giả định  →   Observer, Mediator   →   CÓ "ngược": borrow
                                                        checker CẤM →
                                                        dùng channel
```

> Đọc bảng: cột phải = "KHÔNG" nghĩa là pattern đó *tan biến* trong Rust.

---

<a name="v3-nguyen-tac"></a>
## 3. Bốn nguyên tắc nền tảng

```
┌──────────────────────────────────────────────────────────┐
│ 1. MAKE INVALID STATES UNREPRESENTABLE                     │
│                                                            │
│    ❌ struct Conn { is_open: bool, sock: Option<...> }     │
│         open=true + sock=None ← BUG biểu diễn được         │
│                                                            │
│    ✅ enum Conn { Closed, Open(TcpStream) }                │
│         trạng thái sai KHÔNG TỒN TẠI → compiler gác cổng   │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ 2. PARSE, DON'T VALIDATE                                   │
│                                                            │
│    String thô ──parse──► Email (kiểu chặt)                 │
│    sau điểm này, KHÔNG AI kiểm tra lại. Type = bằng chứng. │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ 3. COMPOSITION OVER INHERITANCE (bắt buộc, Rust no inherit)│
│                                                            │
│    trait (hành vi) + struct chứa struct (dữ liệu)          │
│    + generic (tham số hoá) ── KHÔNG có cây kế thừa         │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ 4. TRẢ CHI PHÍ CÓ Ý THỨC                                   │
│                                                            │
│    Box/Rc/Arc/dyn = alloc + pointer chase + dispatch       │
│    Rust khiến chi phí HIỆN trong kiểu. Đo trước, tối ưu sau│
└──────────────────────────────────────────────────────────┘
```

---

<a name="v4-newtype"></a>
## 4. Newtype — zero-cost wrapping

```
NGUY HIỂM: cùng kiểu cơ sở, khác ý nghĩa
   fn transfer(from: u64, to: u64, amount: u64)
                 │         │          │
                 └─────────┴──────────┘  compiler KHÔNG phân biệt
                      transfer(amount, from, to)  ← SAI, vẫn compile 😱

AN TOÀN: bọc Newtype
   struct AccountId(u64);   struct Money(u64);
   fn transfer(from: AccountId, to: AccountId, amount: Money)
                      ▲ sai kiểu → COMPILE ERROR ✅

MEMORY (zero-cost):
   AccountId(u64)        u64
   ┌──────────┐        ┌──────────┐
   │   u64    │   ==   │   u64    │   ← cùng layout, 0 byte overhead
   └──────────┘        └──────────┘
   (single-field struct có repr giống field — xem chương x)

3 CÔNG DỤNG:
   (a) type safety  ── phân biệt giá trị cùng kiểu
   (b) orphan rule  ── impl Display for Wrapper(Vec<..>)
   (c) invariant    ── private field + new() validate
                       └─► chỉ tạo được qua cổng kiểm tra
```

---

<a name="v5-builder"></a>
## 5. Builder — flow

```
VẤN ĐỀ: Rust KHÔNG có named args / optional args
   Server::new("h", 8080, 30, 1024, true, false, None, 4)
                                    ▲▲▲▲ cái nào là cái gì??

BUILDER FLOW:
   ServerBuilder::new("localhost")   ◄── defaults: port=8080, tls=false
            │
            ▼ .port(443)              ┌── mỗi method: mut self → Self
            │                         │   set 1 trường, trả self
            ▼ .tls(true)              │   (chain được)
            │                         │
            ▼ .max_connections(5000)  ┘
            │
            ▼ .build()  ──────────────► Server { ... }  (validate tại đây)

ĐỌC NHƯ VĂN XUÔI, chỉ set cái cần.

KHI NÀO DÙNG:
   nhiều trường optional/default  ──► Builder  ✅
   2-3 trường bắt buộc            ──► Struct{..} / new()  (Builder = thừa)
   tất cả public + có default     ──► ..Default::default()
```

---

<a name="v6-typestate"></a>
## 6. Typestate — state machine ở compile time

```
Request<NoUrl>                Request<HasUrl>
┌──────────────┐   .url(u)    ┌──────────────┐
│ new()        │ ───────────► │ .body(b)     │
│              │   (tiêu thụ  │ .send() ✅   │
│ .send() ❌   │    self, đổi │              │
│ (KHÔNG tồn   │    kiểu trả) │              │
│  tại method) │              │              │
└──────────────┘              └──────────────┘

   Request::new().send()
                  ▲
                  └── COMPILE ERROR: NoUrl không có .send()

CƠ CHẾ:
   struct Request<State> { ..., _state: PhantomData<State> }
                                            ▲ zero-sized, 0 byte
   impl Request<NoUrl>  { fn url(self) -> Request<HasUrl> {..} }
   impl Request<HasUrl> { fn send(self) -> .. {..} }
                              ▲ method chỉ tồn tại ở state hợp lệ

CHI PHÍ RUNTIME = 0   |   BẢO ĐẢM = compile-time hoàn toàn

VÍ DỤ THẬT: embedded-hal Pin<Input> vs Pin<Output>
            không write() được lên input pin → compiler chặn
```

---

<a name="v7-raii"></a>
## 7. RAII — vòng đời drop

```
   fn work() {
       let guard = Resource::new();   ◄── ACQUIRE
       │
       │   ... dùng resource ...
       │
       │   (kể cả khi PANIC ở đây ──────┐
       │                                │
   }   ◄── scope kết thúc               │
       │                                │
       └──► guard.drop() TỰ ĐỘNG ◄──────┘
            RELEASE (đóng file / nhả lock / rollback)
            compiler chèn lời gọi — KHÔNG QUÊN ĐƯỢC

MutexGuard ví dụ:
   {
     let g = mutex.lock();   ── lock acquired
     *g += 1;
   } ◄── g drop → unlock tự động (không có "quên unlock")

so với:
   C    : phải nhớ free()/close()       ← dễ quên, leak
   Go   : defer (phải nhớ viết)         ← đỡ hơn, vẫn thủ công
   Java : try-with-resources / finally  ← phải bọc
   Rust : TỰ ĐỘNG theo ownership        ← không thể quên
```

---

<a name="v8-strategy"></a>
## 8. Strategy — 3 cách dispatch

```
BÀI TOÁN: thay thuật toán/hành vi

┌─ CÁCH 1: CLOSURE ──────────────────────────────────────┐
│  fn process(d: &mut [i32], f: impl Fn(&i32,&i32)->Ord)  │
│  process(v, |a,b| a.cmp(b))                             │
│  → nhẹ nhất. Dùng khi: đơn giản, cục bộ, 1 lần          │
└─────────────────────────────────────────────────────────┘
┌─ CÁCH 2: GENERIC <C: Trait> ───────────────────────────┐
│  struct Archiver<C: Compress> { strategy: C }           │
│  → monomorphize, ZERO-COST, không vtable                │
│  → Dùng khi: phức tạp, biết lúc compile, hot path       │
└─────────────────────────────────────────────────────────┘
┌─ CÁCH 3: Box<dyn Trait> ───────────────────────────────┐
│  struct Archiver { strategy: Box<dyn Compress> }        │
│  → đổi được lúc RUNTIME, vtable lookup, 1 bản code       │
│  → Dùng khi: chọn lúc runtime / lưu nhiều kiểu chung    │
└─────────────────────────────────────────────────────────┘

   đơn giản ──────────────────────────────► phức tạp/linh hoạt
   closure        generic              Box<dyn>
   (0 cost)      (0 cost,             (vtable,
                  code phình)          1 bản code)
```

---

<a name="v9-dispatch-memory"></a>
## 9. Static vs Dynamic dispatch — memory

```
STATIC (generic, monomorphize):
   process::<Gzip>(d)   process::<Zstd>(d)
        │                    │
        ▼                    ▼
   [code Gzip]          [code Zstd]   ← compiler sinh 2 bản
   gọi trực tiếp        gọi trực tiếp  ← inline được, NHANH
   + nhanh nhất                          - binary phình

DYNAMIC (Box<dyn>):
   Box<dyn Compress>
   ┌────────────┬────────────┐
   │ data ptr   │ vtable ptr │  ← fat pointer (2 words)
   └─────┬──────┴─────┬──────┘
         │            │
         ▼            ▼
      [Gzip data]  [vtable: compress() → addr]
                   ┌──────────────────────────┐
                   │ drop()    → ...           │
                   │ compress()→ Gzip::compress│ ← lookup lúc chạy
                   └──────────────────────────┘
   + 1 bản code, gọn        - pointer chase, không inline
   + linh hoạt runtime      - chậm hơn chút

CHỌN: mặc định generic. dyn khi cần runtime poly / giảm code size.
(chi tiết: chương c — trait, mục vtable & monomorphization)
```

---

<a name="v10-observer"></a>
## 10. Observer: classic (sai) vs channel (đúng)

```
❌ CLASSIC OBSERVER (chống borrow checker):
   ┌──────────┐  giữ &mut    ┌────────────┐
   │ Subject  │ ───────────► │ Observer A │
   │          │ ◄─────────── │ (giữ ref   │
   │ Vec<&mut │   ref ngược  │  ngược)    │
   │  dyn Obs>│              └────────────┘
   └──────────┘
        │  → aliasing + mutation đồng thời + vòng tham chiếu
        ▼
   BORROW CHECKER TỪ CHỐI / phải Rc<RefCell<>> → panic runtime, leak

✅ CHANNEL (Rust-native):
   ┌──────────┐                      ┌────────────┐
   │ Subject  │ ──tx.send(Event)──►  │ Observer A │ (sở hữu state riêng)
   │ (chỉ gửi,│        channel    ┌─► │ Observer B │
   │  không   │ ──────────────────┤  └────────────┘
   │  biết ai │                   └─► (broadcast cho nhiều subscriber)
   │  nghe)   │
   └──────────┘
   → ownership tách bạch, không vòng, thread-safe, dễ test

   crate: std mpsc · tokio broadcast · flume
```

---

<a name="v11-state"></a>
## 11. State: enum vs typestate

```
ENUM + MATCH (runtime, linh hoạt):
   ┌─────┐ next ┌──────┐ next ┌────────┐ next
   │ Red │ ───► │Green │ ───► │ Yellow │ ───► (Red)
   └─────┘      └──────┘      └────────┘
   enum TrafficLight { Red, Green, Yellow }
   fn next(self) -> Self { match self {...} }
   ✓ đổi runtime  ✓ serialize  ✓ lưu  ✓ exhaustive match
   ✗ gọi sai chỉ bắt được lúc runtime

TYPESTATE (compile-time, cứng):
   Locked ──unlock()──► Unlocked ──lock()──► Locked
   (kiểu)              (kiểu khác)
   ✓ gọi sai = COMPILE ERROR  ✓ zero-cost
   ✗ khó serialize  ✗ phình impl block

QUYẾT ĐỊNH:
   nhiều state, đổi nhiều, cần lưu  ──► enum
   bug đắt, flow cố định, bắt sớm   ──► typestate
```

---

<a name="v12-decorator"></a>
## 12. Decorator / tower middleware — onion

```
       Request đi vào
            │
   ┌────────▼─────────────────────────┐
   │  TracingLayer (log span)          │ ◄── decorator ngoài cùng
   │  ┌──────────────────────────────┐ │
   │  │  TimeoutLayer                │ │ ◄── decorator
   │  │  ┌─────────────────────────┐ │ │
   │  │  │  RateLimitLayer         │ │ │ ◄── decorator
   │  │  │  ┌────────────────────┐ │ │ │
   │  │  │  │  AuthLayer         │ │ │ │ ◄── decorator
   │  │  │  │  ┌───────────────┐ │ │ │ │
   │  │  │  │  │ CORE Service  │ │ │ │ │ ◄── logic thật
   │  │  │  │  └───────────────┘ │ │ │ │
   │  │  │  └────────────────────┘ │ │ │
   │  │  └─────────────────────────┘ │ │
   │  └──────────────────────────────┘ │
   └────────┬──────────────────────────┘
            │
       Response đi ra (qua từng lớp ngược lại)

   ServiceBuilder::new()
       .layer(TraceLayer)      ┐
       .layer(TimeoutLayer)    │ mỗi layer = 1 Decorator
       .layer(RateLimitLayer)  │ tháo lắp, sắp xếp lại tự do
       .layer(AuthLayer)       ┘
       .service(core)

   = Decorator (chồng hành vi) + Chain of Responsibility (mỗi lớp
     quyết định xử lý hay chuyển tiếp). axum dùng cơ chế này.
```

---

<a name="v13-composite"></a>
## 13. Composite — enum đệ quy

```
CÂY THƯ MỤC:
   enum FileNode {
       File { name, size },
       Dir  { name, children: Vec<FileNode> }  ◄── đệ quy qua Vec
   }
                    Dir "root"
                   /          \
            File "a.txt"    Dir "src"
              (10)          /        \
                     File "main"   File "lib"
                       (50)          (30)

   total_size() = match → File trả size, Dir trả sum(children)
                  → đệ quy, exhaustive, KHÔNG vtable

BIỂU THỨC (cần Box vì self-reference trực tiếp):
   enum Expr { Num(f64), Add(Box<Expr>,Box<Expr>), Mul(..) }
                                ▲ Box: con trỏ → size hữu hạn

         Add
        /   \
      Num    Mul
      (2)   /   \
          Num   Num
          (3)   (4)
   eval = 2 + (3*4) = 14   (Composite + Interpreter hợp nhất)

CHỌN: số dạng node cố định → enum (exhaustive).
      số dạng node MỞ (plugin) → Box<dyn Trait>.
```

---

<a name="v14-bridge"></a>
## 14. Bridge — hai trục độc lập

```
VẤN ĐỀ: bùng nổ tổ hợp khi dùng inheritance
   AlertEmail  AlertSMS  AlertPush
   ReminderEmail  ReminderSMS  ReminderPush   ← 2×3 = 6 class 😱
   (thêm 1 loại HOẶC 1 kênh → nhân lên)

BRIDGE: tách 2 trục
   TRỤC ABSTRACTION          TRỤC IMPLEMENTATION
   (loại notification)       (kênh gửi)
   ┌──────────┐              ┌──────────┐
   │ Alert    │──giữ────────►│ Channel  │ (trait)
   │ Reminder │   Box<dyn>   ├──────────┤
   │ Digest   │              │ Email    │
   └──────────┘              │ SMS      │
                             │ Push     │
                             │ Slack    │
                             └──────────┘
   thêm loại  → +1 struct (độc lập)
   thêm kênh  → +1 impl Channel (độc lập)
   → 2+4 = 6 thứ thay vì 2×4 = 8, và KHÔNG nhân lên

   struct Alert { channel: Box<dyn Channel> }  ← cây cầu
```

---

<a name="v15-actor"></a>
## 15. Actor model — message passing

```
        ┌──────────────────────────────────┐
        │  ACTOR (1 task sở hữu state)       │
        │  ┌──────────────────────────────┐ │
        │  │ count: u64  ◄── KHÔNG ai      │ │
        │  │             chạm trực tiếp     │ │
        │  └──────────────────────────────┘ │
        │  loop { match rx.recv() {          │
        │     Incr => count += 1             │
        │     Get{resp} => resp.send(count)  │
        │  }}                                │
        └────────────▲─────────────▲─────────┘
                     │             │
              mpsc channel    (xử lý TUẦN TỰ
                     │         → không lock,
   ┌─────────────────┴───┐      không data race)
   │ Handle (clone được) │
   │  .incr() → send Incr│
   │  .get()  → send Get + oneshot trả lời
   └─────────┬───────────┘
             │
   ┌─────────┴──────────┐
   │ task A   task B  ...│  ← nhiều client cùng nói chuyện
   └────────────────────┘

REQUEST-RESPONSE qua oneshot:
   client ──Get{resp: tx}──► actor
   client ◄────rx.await───── actor (gửi kèm "địa chỉ trả lời")

so với Arc<Mutex>: không lock contention, không deadlock,
ranh giới rõ. Đây là Observer/Mediator LÀM ĐÚNG trong Rust.
```

---

<a name="v16-concurrency"></a>
## 16. Concurrency patterns — pipeline & fan-out

```
WORKER POOL (job đồng nhất):
   [Queue] ──┬──► Worker 1 ─┐
             ├──► Worker 2 ─┤──► (kết quả)
             ├──► Worker 3 ─┤
             └──► Worker N ─┘
   → rayon (CPU-bound) / tokio task (IO-bound)

PIPELINE (nhiều giai đoạn):
   [Source]─ch1─►[Parse]─ch2─►[Transform]─ch3─►[Sink]
      task        task          task           task
   → mỗi stage 1 task, nối bằng channel
   → backpressure tự nhiên (channel đầy = chặn)

FAN-OUT / FAN-IN:
              ┌─► Worker ─┐
   [Source] ──┼─► Worker ─┼──► [Collector]
       (fan-out)─► Worker ─┘    (fan-in)
   → chia việc tăng throughput, gom kết quả về 1 nơi
   → FuturesUnordered (async) / channel chung
```

---

<a name="v17-hexagonal"></a>
## 17. Hexagonal architecture

```
   ADAPTERS (driving — gọi VÀO domain)
   ┌───────────┬───────────┬───────────┐
   │ HTTP/axum │   CLI     │   gRPC    │
   └─────┬─────┴─────┬─────┴─────┬─────┘
         │ gọi PORT (trait)      │
   ┌─────▼───────────▼───────────▼─────┐
   │                                    │
   │           DOMAIN (core)            │
   │   ┌────────────────────────────┐   │
   │   │ business logic THUẦN        │   │
   │   │ KHÔNG import sqlx/axum/...   │   │
   │   │                             │   │
   │   │ định nghĩa PORT (trait):    │   │
   │   │  - trait OrderRepository    │   │
   │   │  - trait PaymentGateway     │   │
   │   └────────────────────────────┘   │
   │                                    │
   └─────┬───────────┬───────────┬─────┘
         │ gọi PORT (trait)      │
   ┌─────▼─────┬─────▼─────┬─────▼─────┐
   │ Postgres  │  Redis    │   S3      │
   │ Repo      │  Cache    │  Storage  │
   └───────────┴───────────┴───────────┘
   ADAPTERS (driven — BỊ domain gọi)

   PORT   = trait do domain định nghĩa
   ADAPTER= impl cụ thể (PgRepo, StripeGateway)
   DOMAIN = phụ thuộc ABSTRACTION, không phụ thuộc infra
   → test domain không cần DB; đổi infra không đụng logic
```

---

<a name="v18-expression"></a>
## 18. Expression problem — enum vs trait

```
                  Thêm KIỂU mới?    Thêm OPERATION mới?
   ──────────────────────────────────────────────────────
   enum + match    KHÓ              DỄ
   (sửa enum +     (compiler nhắc   (chỉ thêm 1 hàm
    mọi match)      mọi match)       match mới)

   trait object    DỄ               KHÓ
   (Box<dyn>)      (chỉ impl trait  (sửa trait → mọi
                    cho kiểu mới)    impl phải cập nhật)

   CHỌN THEO TRỤC NÀO HAY ĐỔI HƠN:
   ┌─────────────────────────────────────────────┐
   │  thêm operation nhiều (eval/print/optimize)  │
   │       → enum + match (Visitor tan biến)      │
   │                                              │
   │  thêm kiểu nhiều (plugin, shape mới)         │
   │       → trait object                         │
   └─────────────────────────────────────────────┘

   serde = Visitor THẬT (streaming, kiểu chưa biết trước)
```

---

<a name="v19-case-study"></a>
## 19. Case study: URL shortener từ nhỏ đến lớn

```
STAGE 0 ── script 30 dòng
   HashMap + vài hàm
   PATTERN: KHÔNG CÓ ◄── và đó là ĐÚNG
   │  áp lực: (chưa có)
   ▼
STAGE 1 ── module nhỏ
   PATTERN: Newtype (ShortCode/LongUrl) + Builder (config)
   │  áp lực: nhầm kiểu string, config rối
   ▼
STAGE 2 ── library
   PATTERN: Repository (Storage trait) + Strategy (CodeGenerator)
   │         + DI qua generic
   │  áp lực: nhiều cách lưu (mem/redis), nhiều cách sinh mã, cần test
   ▼
STAGE 3 ── web service
   PATTERN: Hexagonal + DI runtime (Arc<dyn>) + Decorator (tower)
   │  áp lực: nhiều client, auth/ratelimit/log, tách domain để test
   ▼
STAGE 4 ── distributed
   PATTERN: Actor + Event-driven + Plugin + (Event Sourcing)
      áp lực: scale, nhiều team, audit, analytics realtime

═══════════════════════════════════════════════════════════
QUY LUẬT VÀNG:
   pattern = PHẢN ỨNG với áp lực, KHÔNG phải điểm khởi đầu
   đi từ trên xuống, thêm pattern khi áp lực xuất hiện
   nhét event-sourcing vào script = over-engineering
═══════════════════════════════════════════════════════════
```

```
   ĐỘ PHỨC TẠP
   ▲
   │                                        ╭─ Event Sourcing
   │                                   ╭────╯  Plugin
   │                              ╭────╯       Actor
   │                         ╭────╯  Hexagonal
   │                    ╭────╯       Decorator
   │               ╭────╯  Repository
   │          ╭────╯       Strategy
   │     ╭────╯  Newtype/Builder
   │ ────╯  (không pattern)
   └────────────────────────────────────────────► QUY MÔ HỆ THỐNG
     script   module  library   web      distributed
```

---

<a name="v20-antipatterns"></a>
## 20. Antipatterns — hình ảnh

```
❌ #1: Rc<RefCell<>> KHẮP NƠI ("Java bằng Rust")
   Node = Rc<RefCell<NodeData>>
   ┌────┐◄──┐  vòng tham chiếu → leak
   │ A  │   │  borrow_mut khi đang borrow → PANIC runtime
   └─┬──┘   │  mất bảo đảm compile-time
     └──────┘
   ✅ FIX: arena (Vec + index id) / message passing / sở hữu 1 chiều

❌ #2: OVER-ENGINEERING
   AbstractFactoryStrategyBuilder cho app in "Hello"
   ✅ FIX: rule of three — chờ lần lặp thứ 3 mới trừu tượng hóa

❌ #3: DỊCH 1:1 PATTERN JAVA
   classic Observer / static mut Singleton / Deref giả kế thừa
   ✅ FIX: hỏi "bài toán GỐC là gì?" → channel / OnceLock / composition

❌ #4: GOD TRAIT (20 method)
   ✅ FIX: tách trait nhỏ, compose qua supertrait

❌ #5: Deref GIẢ KẾ THỪA
   impl Deref for Dog { type Target = Animal }
   ✅ FIX: composition + delegate tường minh

❌ #6: PREMATURE dyn / generic
   Box<dyn> khi kiểu biết lúc compile; generic cho 1 impl duy nhất
   ✅ FIX: kiểu cụ thể tới khi THẬT SỰ cần đa hình

MẪU SỐ CHUNG: áp giải pháp khi CHƯA hiểu bài toán,
              hoặc CHỐNG lại grain của Rust.
```

---

<a name="v21-decision-tree"></a>
## 21. Decision tree tổng

```
                    ┌─────────────────────────┐
                    │ Bài toán THẬT là gì?    │
                    └────────────┬────────────┘
                                 │
   ┌─────────────────────────────┼─────────────────────────────┐
   ▼                             ▼                             ▼
 TẠO OBJECT               CẤU TRÚC / GHÉP              HÀNH VI / GIAO TIẾP
   │                             │                             │
   ├ nhiều optional?             ├ thêm hành vi chồng?         ├ thay thuật toán?
   │  → Builder                  │  → Decorator/tower          │  ├ đơn giản → closure
   │                             │                             │  ├ compile → generic
   ├ trình tự bắt buộc?          ├ cây/đệ quy?                 │  └ runtime → Box<dyn>
   │  → Typestate                │  → enum + Box               │
   │                             │                             ├ 1 trong N dạng?
   ├ chọn impl runtime?          ├ 2 trục độc lập?             │  → enum + match
   │  → Factory + Box<dyn>       │  → Bridge                   │
   │                             │                             ├ component báo nhau?
   ├ global đọc-1-lần?           ├ tách DB/HTTP?               │  → channel/event bus
   │  → OnceLock (cẩn thận!)     │  → Repository → Hexagonal   │
   │                             │                             ├ dọn tài nguyên?
   └ copy object?                └ giấu hệ con phức tạp?       │  → RAII (Drop)
      → Clone                       → Facade (module)         │
                                                              ├ chia sẻ state?
   PHÂN BIỆT GIÁ TRỊ / BẤT BIẾN?                              │  ├ đơn giản → Arc<Mutex>
      → Newtype (+ private field)                             │  └ hot → Actor
                                                              │
                                                              └ undo/snapshot?
                                                                 → Command(enum)/Clone

   ╔══════════════════════════════════════════════════╗
   ║ NGHI NGỜ? → ĐỪNG dùng pattern. Chờ áp lực thứ 2.  ║
   ╚══════════════════════════════════════════════════╝
```

---

<a name="v22-mind-map"></a>
## 22. Mind map tổng kết

```
                        DESIGN PATTERNS TRONG RUST
                                   │
        ┌──────────────────────────┼──────────────────────────┐
        │                          │                          │
    TRIẾT LÝ                   IDIOMS NỀN TẢNG            PATTERN GoF
        │                          │                          │
   ┌────┴────┐              ┌──────┼──────┐         ┌─────────┼─────────┐
   │ GoF từ  │              │Newtype      │      Creational Structural Behavioral
   │ OOP     │              │Builder      │         │         │         │
   │         │              │Typestate    │      Builder    Adapter   Strategy
   │ Rust    │              │RAII         │      Factory    Decorator →closure
   │ đổi hình│              │Default      │      Singleton  Facade    Observer
   │ dạng    │              └─────────────┘      →OnceLock  Composite →channel
   │ pattern │                                   Prototype  Proxy     State
   └─────────┘                                   →Clone     Flyweight →enum
        │                                                   Bridge    Visitor
   ┌────┴─────────────┐                                              →match
   │ 4 NGUYÊN TẮC:    │
   │ 1 invalid states │         ┌──────────────────────────────────┐
   │   unrepresentable│         │ ĐẶC SẢN RUST (không có GoF):       │
   │ 2 parse not      │         │ Sealed trait · Extension trait    │
   │   validate       │         │ PhantomData · Interior mutability  │
   │ 3 composition    │         │ Entry API · Cow                    │
   │ 4 chi phí có ý   │         └──────────────────────────────────┘
   │   thức           │
   └──────────────────┘         ┌──────────────────────────────────┐
        │                       │ CONCURRENCY:                       │
   ┌────┴─────────────┐         │ Actor · Arc<Mutex> · Worker pool   │
   │ KIẾN TRÚC LỚN:   │         │ Pipeline · Fan-out/in              │
   │ DI (trait)       │         └──────────────────────────────────┘
   │ Repository       │
   │ Hexagonal        │         ┌──────────────────────────────────┐
   │ Plugin           │         │ TỪ NHỎ ĐẾN LỚN:                    │
   │ Event-driven     │         │ script→module→lib→web→distributed  │
   │ CQRS/EventSource │         │ pattern = phản ứng với ÁP LỰC      │
   │ DDD              │         └──────────────────────────────────┘
   └──────────────────┘

   ╔════════════════════════════════════════════════════════════╗
   ║ CÂU THẦN CHÚ: Đừng hỏi "pattern nào hợp?". Hỏi "bài toán   ║
   ║ thật là gì, cách nhẹ nhất Rust giải nó?". Pattern sẽ tự lộ ║
   ║ ra — hoặc tự biến mất.                                      ║
   ╚════════════════════════════════════════════════════════════╝
```

🦀 Đọc kèm theory: [y-design-patterns.md](./y-design-patterns.md)
