# Observability trong Rust — Logging, Tracing, Metrics

> Tài liệu thứ 12 trong bộ Rust nền tảng. Đọc trước:
> - [async.md](./async.md) — async + tracing có quirk riêng
> - [error-handling.md](./error-handling.md) — error log đúng cách
> - [performance.md](./performance.md) — tracing có cost, cần aware
>
> **Observability** là khả năng hiểu **what's happening inside** một hệ thống production
> chỉ qua output (logs, metrics, traces) — KHÔNG cần ssh vào server hoặc debug locally.
>
> Code chạy production = box đen. Observability là **đèn pin** để soi vào đó.
>
> Tài liệu này dạy bạn:
> - 3 pillars: Logs, Metrics, Traces
> - `tracing` crate — chuẩn de-facto của Rust
> - OpenTelemetry integration
> - Metrics với Prometheus
> - Distributed tracing
> - Patterns và antipatterns của senior

---

# Mục lục

- [Tầng 1: Tại sao Observability quan trọng?](#tầng-1-tại-sao-observability-quan-trọng)
- [Tầng 2: 3 Pillars — Logs, Metrics, Traces](#tầng-2-3-pillars--logs-metrics-traces)
- [Tầng 3: log crate — Foundation đơn giản](#tầng-3-log-crate--foundation-đơn-giản)
- [Tầng 4: tracing — Cấu trúc của thế hệ mới](#tầng-4-tracing--cấu-trúc-của-thế-hệ-mới)
- [Tầng 5: Spans vs Events — Hiểu cho đúng](#tầng-5-spans-vs-events--hiểu-cho-đúng)
- [Tầng 6: Subscriber và Layer architecture](#tầng-6-subscriber-và-layer-architecture)
- [Tầng 7: Structured Logging — Không phải string format](#tầng-7-structured-logging--không-phải-string-format)
- [Tầng 8: #[instrument] macro — Auto-tracing functions](#tầng-8-instrument-macro--auto-tracing-functions)
- [Tầng 9: Filtering, Levels, Targets](#tầng-9-filtering-levels-targets)
- [Tầng 10: Async tracing — Quirks và best practices](#tầng-10-async-tracing--quirks-và-best-practices)
- [Tầng 11: OpenTelemetry — Standard cho distributed tracing](#tầng-11-opentelemetry--standard-cho-distributed-tracing)
- [Tầng 12: Metrics — metrics crate và Prometheus](#tầng-12-metrics--metrics-crate-và-prometheus)
- [Tầng 13: Distributed Tracing — Context propagation](#tầng-13-distributed-tracing--context-propagation)
- [Tầng 14: Sampling — Khi telemetry quá nhiều](#tầng-14-sampling--khi-telemetry-quá-nhiều)
- [Tầng 15: Patterns và Antipatterns](#tầng-15-patterns-và-antipatterns)

---

# Tầng 1: Tại sao Observability quan trọng?

## 1.1 Vấn đề thực tế

Bạn deploy 1 web service. Một tuần sau, user phàn nàn:
- "API chậm sau 3 giờ chiều"
- "Random 500 errors"
- "Cập nhật user thỉnh thoảng không lưu"

**Không có observability**, bạn:
- SSH vào server, đọc log raw, grep từng chuỗi
- Thử reproduce locally — không lặp lại được
- Đoán mò, deploy "fix", lặp đi lặp lại
- User khó chịu, mất tin tưởng

**Có observability** đúng:
- Open dashboard → thấy P99 latency tăng 5x sau 3PM
- Drill down → user_service → DB query "find_user" tăng 100x
- Click trace → query plan đổi vì index missing
- Fix, deploy, verify ngay trên dashboard

## 1.2 Observability ≠ Monitoring

| Aspect | Monitoring | Observability |
|--------|-----------|---------------|
| Câu hỏi trả lời | "Cái gì đang sai?" | "Tại sao đang sai?" |
| Yêu cầu | Biết trước cái gì cần check | Khám phá tự do |
| Data type | Predefined metrics | High-cardinality data |
| Use case | Known failures (CPU, disk) | Unknown unknowns |

Monitoring = subset của observability. Observability cho phép **debug** issue chưa từng gặp.

## 1.3 3 câu hỏi observability trả lời

Charity Majors (Honeycomb) định nghĩa:
1. **What's broken?** — alerts
2. **Why is it broken?** — drill down, correlate signals
3. **What's normal?** — baseline để compare

Để trả lời cả 3, bạn cần **3 pillars**.

## 1.4 Cost của observability

Có cost:
- **Compute**: collect, send, parse telemetry
- **Storage**: logs/traces/metrics chiếm GB-TB
- **Network**: ship data ra hệ thống collect
- **Cognitive**: dashboard maintenance, alert tuning

Senior balance: đo đủ để debug nhanh, không đo quá nhiều thành noise.

---

# Tầng 2: 3 Pillars — Logs, Metrics, Traces

## 2.1 Logs

**Logs** = event chuỗi thời gian, mỗi cái mô tả 1 sự kiện cụ thể.

```
2024-05-26T10:30:45.123Z INFO  user_login user_id=42 method=oauth
2024-05-26T10:30:45.456Z ERROR db_query error=timeout query="SELECT ..."
```

Đặc điểm:
- **Discrete events**
- High verbosity (mỗi action 1 log)
- High cardinality (mỗi log có thể unique)
- Storage expensive (text)

Tốt cho:
- Debug specific user issue
- Audit trail (security, compliance)
- Stack trace, error details

Xấu cho:
- Aggregation across services (chậm)
- Real-time alerts (delay)

## 2.2 Metrics

**Metrics** = numeric measurement theo thời gian, đã aggregate.

```
http_requests_total{method="GET", status="200"} = 12345
http_request_duration_p99_ms = 234
```

Đặc điểm:
- **Aggregated** numbers (count, gauge, histogram)
- Low cardinality (limit số labels)
- Fixed dimensions
- Cheap to store (numeric)

Tốt cho:
- Dashboards (Grafana)
- Alerts (P99 > threshold)
- Trends (week-over-week)
- Capacity planning

Xấu cho:
- Per-request debugging (lost detail)
- Investigating outliers (aggregated away)

## 2.3 Traces

**Traces** = call graph của 1 request đi qua nhiều services.

```
Trace: req-12345
├── Span: api_gateway (200ms)
│   ├── Span: auth_service (50ms)
│   └── Span: order_service (140ms)
│       ├── Span: db_query (60ms)
│       └── Span: payment_service (70ms)
│           └── Span: stripe_api (50ms)
```

Đặc điểm:
- **Causal relationships** (parent → child)
- Hierarchical
- High cardinality (1 trace = 1 request)
- Bridges logs + metrics

Tốt cho:
- Tìm bottleneck across services
- Root cause analysis distributed system
- Latency debugging
- Service dependency map

## 2.4 Khi nào dùng cái nào?

```
   Câu hỏi                         Pillar
   ────────────────────────        ──────
   "1000 errors hôm nay?"          Metrics (counter)
   "User X gặp lỗi gì lúc 3PM?"    Logs
   "Request abc-123 chậm vì sao?"  Traces
   "P99 latency tuần này?"         Metrics
   "Stack trace exception?"        Logs (với context từ Traces)
   "Service A call B mấy lần?"     Traces / Metrics
```

→ **Phải có cả 3**. Senior team không bỏ qua pillar nào.

## 2.5 Correlation — Sức mạnh thật của observability

3 pillars **độc lập** thì có giới hạn. **Correlated** mới powerful.

Workflow:
1. Alert fires: "P99 latency > 500ms" (Metrics)
2. Drill down to time range, find affected requests (Traces)
3. Open trace, find slow span (Traces)
4. Read logs of that span (Logs)
5. Root cause: DB connection pool exhausted

→ Mỗi pillar có **trace_id** chung để link.

---

# Tầng 3: log crate — Foundation đơn giản

## 3.1 log crate là gì?

`log` là **facade crate** — định nghĩa API logging chuẩn, không implement.

```toml
[dependencies]
log = "0.4"
env_logger = "0.11"   # implementation
```

```rust
use log::{info, warn, error, debug, trace};

fn main() {
    env_logger::init();
    
    info!("Starting application");
    warn!("Config file not found, using defaults");
    error!("Failed to connect: {}", err);
}
```

Run với log level:
```bash
RUST_LOG=info ./myapp
RUST_LOG=debug,hyper=warn ./myapp   # debug all, hyper=warn
```

## 3.2 5 levels

```
ERROR  ─── Critical, action needed
WARN   ─── Unusual but handled
INFO   ─── Normal operations milestones
DEBUG  ─── Detail useful for debugging
TRACE  ─── Very verbose, every step
```

Convention production: INFO trở lên. DEBUG/TRACE chỉ khi debugging.

## 3.3 Implementations

`log` crate là facade. Phải pick implementation:

| Crate | Output | Use case |
|-------|--------|----------|
| `env_logger` | stderr, configurable | CLI tools, simple servers |
| `simple_logger` | stderr, minimal | Quick tools |
| `pretty_env_logger` | Colored stderr | Local dev |
| `flexi_logger` | Files, rotation | Production services |
| `tracing-log` | Forward to tracing | Migration path |

## 3.4 Hạn chế của log crate

```rust
info!("User {} logged in via {}", user_id, method);
```

Output: `INFO User 42 logged in via oauth`

**Vấn đề**:
- Field embedded trong text → khó parse, không structured
- Không có context propagation (request_id?)
- Không thấy hierarchy / spans
- Khó query/aggregate logs (cần regex)

→ Production cần **structured logging** → dùng `tracing`.

## 3.5 Khi nào dùng log crate?

- CLI tool đơn giản
- Library code (provide log, không force tracing on user)
- Migration path: legacy code dùng log, mới dùng tracing

Library Rust often nên dùng `log` crate (lighter) — application pick implementation.

---

# Tầng 4: tracing — Cấu trúc của thế hệ mới

## 4.1 tracing là gì?

`tracing` = framework observability từ Tokio team. Built cho async-first.

Hơn `log`:
- **Structured fields** (key-value, không string)
- **Spans** (hierarchical context)
- **Async-aware** (track logical task)
- **Composable** (multiple subscribers / layers)
- **Forward log!()** seamless

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

```rust
use tracing::{info, warn, error, debug, instrument};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("Starting");
    
    let span = tracing::info_span!("processing", user_id = 42);
    let _guard = span.enter();
    info!("Inside span");
}
```

Output:
```
2024-05-26T10:00:00Z  INFO myapp: Starting
2024-05-26T10:00:01Z  INFO processing{user_id=42}: myapp: Inside span
```

`processing{user_id=42}:` = current span context.

## 4.2 Event = single log line

```rust
info!(user_id = 42, action = "login", "User logged in");
```

Output:
```
INFO myapp: User logged in user_id=42 action=login
```

Fields **structured**, không phải interpolation:
- `user_id = 42` → field `user_id` với value `42`
- `"User logged in"` → message

Output backend chọn format: text, JSON, OpenTelemetry...

## 4.3 So sánh với log crate

```rust
// log crate:
info!("user {} action {}", user_id, action);
// Output: text "user 42 action login"

// tracing:
info!(user_id, action, "User action");
// Output: structured { user_id: 42, action: "login", message: "User action" }
```

Structured fields → query/filter trong log system (Elasticsearch, Loki, Datadog).

## 4.4 tracing → log bridge

Code library cũ dùng `log::info!`. tracing có thể catch:

```rust
tracing_subscriber::fmt()
    .with_env_filter("info")
    .with_target(true)
    .init();

// log!() calls automatically forwarded to tracing!
log::info!("from log crate");  // → tracing event
```

→ Migration smooth.

---

# Tầng 5: Spans vs Events — Hiểu cho đúng

## 5.1 Khác biệt cốt lõi

```
   Event    = 1 thời điểm (point in time)
   Span     = 1 khoảng (duration with start + end)
   
   Timeline:
   ─────────────────────────────────────────►
   
   Event:        •         •           •
                (log)     (log)        (log)
   
   Span:    ┌───────────────────────┐
            │       active           │
            └───────────────────────┘
            start                    end
```

## 5.2 Khi nào dùng cái nào?

```rust
// Event — "1 chuyện xảy ra"
info!("User logged in");
error!(?err, "Failed");

// Span — "Một logic block đang chạy"
let span = info_span!("process_request", request_id = "abc");
let _enter = span.enter();
// ... do work ...
// span auto exit when _enter dropped
```

Events thường nằm **bên trong** spans. Span = context cho events.

## 5.3 Cú pháp span

### Cách 1: Manual

```rust
let span = tracing::info_span!("operation", user_id = 42);
let _enter = span.enter();   // bắt đầu span

do_work();   // events trong đây ngầm có span context

// _enter drop → span exit
```

### Cách 2: in_scope

```rust
tracing::info_span!("operation", user_id = 42).in_scope(|| {
    do_work();
});
```

### Cách 3: #[instrument] (preferred)

```rust
#[tracing::instrument]
fn process(user_id: u32) {
    info!("processing");
}
```

`#[instrument]` tự tạo span với function name + args. Đẹp nhất.

## 5.4 Recording fields sau khi tạo

```rust
let span = info_span!("process", user_id = tracing::field::Empty);
let _enter = span.enter();

let user = fetch_user().await;
span.record("user_id", user.id);  // điền sau
```

Useful khi field value chưa biết tại span creation.

## 5.5 Hierarchical structure

```rust
let outer = info_span!("outer");
let _e1 = outer.enter();

let inner = info_span!("inner");
let _e2 = inner.enter();

info!("event in nested span");
```

Output thấy hierarchy:
```
outer:inner: myapp: event in nested span
```

Span parent tự suy ra từ current context. Forms tree.

## 5.6 Async spans

```rust
async fn process() {
    let span = info_span!("processing");
    let _enter = span.enter();
    
    other_async().await;   // ⚠️ span context có thể bị lost!
}
```

Problem: khi yield, span guard có thể không follow.

Fix: dùng `.instrument()`:
```rust
async fn process() {
    async {
        info!("processing");
        other_async().await;
    }.instrument(info_span!("processing")).await;
}
```

Hoặc `#[instrument]` (recommended).

---

# Tầng 6: Subscriber và Layer architecture

## 6.1 Subscriber là gì?

`Subscriber` = trait implement bởi **anh ta nhận events/spans** và làm gì đó (in ra, gửi đi).

```
   App code
     │
     │ tracing::info!(...)
     ▼
   Dispatch (global)
     │
     ▼
   Subscriber  ── implements: on_event, on_enter, ...
```

1 process có **1** global subscriber. Subscriber tự handle.

## 6.2 tracing_subscriber::fmt — Print to stderr

```rust
tracing_subscriber::fmt()
    .with_target(true)        // include target (crate::module)
    .with_thread_ids(true)    // include thread ID
    .with_thread_names(true)
    .with_file(true)
    .with_line_number(true)
    .with_env_filter("info,hyper=warn")
    .init();
```

Output:
```
2024-05-26T10:00:00Z INFO ThreadId(1) myapp::main: src/main.rs:42: User action
```

## 6.3 Layer architecture

Subscriber phức tạp khi muốn nhiều output (file + stdout + OpenTelemetry).

`tracing-subscriber::Layer`: composable.

```rust
use tracing_subscriber::prelude::*;

let stdout_layer = tracing_subscriber::fmt::layer()
    .with_writer(std::io::stdout);

let file_layer = tracing_subscriber::fmt::layer()
    .with_writer(std::fs::File::create("app.log").unwrap())
    .json();   // JSON format

let filter = tracing_subscriber::EnvFilter::from_default_env();

tracing_subscriber::registry()
    .with(filter)
    .with(stdout_layer)
    .with(file_layer)
    .init();
```

Cú pháp:
- `registry()` — empty subscriber
- `.with(layer)` — attach layer
- Layers chain — events đi qua tất cả

## 6.4 JSON output (production)

```rust
let json_layer = tracing_subscriber::fmt::layer().json();
```

Output:
```json
{
  "timestamp": "2024-05-26T10:00:00Z",
  "level": "INFO",
  "target": "myapp",
  "fields": {
    "user_id": 42,
    "message": "User action"
  },
  "spans": [
    { "name": "process_request", "request_id": "abc" }
  ]
}
```

→ Parse bởi Elasticsearch, Loki, Datadog. Production luôn dùng JSON.

## 6.5 Built-in formatters

`tracing-subscriber::fmt::format` modules:
- `compact` — single line, terse
- `full` — multi-line readable (default dev)
- `json` — JSON (production)
- `pretty` — colored multi-line (dev)

```rust
let layer = tracing_subscriber::fmt::layer()
    .compact();   // hoặc .pretty(), .json()
```

## 6.6 Custom Layer

```rust
use tracing_subscriber::Layer;

struct MyLayer;

impl<S> Layer<S> for MyLayer where S: tracing::Subscriber {
    fn on_event(&self, event: &tracing::Event, _ctx: tracing_subscriber::layer::Context<S>) {
        println!("Custom layer saw: {:?}", event.metadata().target());
    }
}
```

Useful: count events, send to external system, custom filter.

## 6.7 tracing-bunyan-formatter

Bunyan format = JSON optimized for analysis:

```toml
[dependencies]
tracing-bunyan-formatter = "0.3"
```

```rust
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};

tracing_subscriber::registry()
    .with(JsonStorageLayer)
    .with(BunyanFormattingLayer::new(
        "myapp".into(),
        std::io::stdout,
    ))
    .init();
```

Compatible với `bunyan` CLI tool để pretty-print local.

---

# Tầng 7: Structured Logging — Không phải string format

## 7.1 Triết lý structured

```rust
// ❌ String interpolation (legacy)
info!("User {} updated profile, age {}", user_id, age);

// ✅ Structured fields
info!(user_id, age, "User updated profile");
```

Output JSON:
```json
{ "message": "User updated profile", "user_id": 42, "age": 30 }
```

Trong log system: query `user_id:42 AND age:[18 TO 65]` — không cần regex.

## 7.2 Field syntax

```rust
info!(user_id = 42, "Login");           // explicit
info!(user_id, "Login");                // shorthand if var name = field
info!(?err, "Failed");                  // Debug format
info!(%user.name, "Login");             // Display format
info!(user.id = user.id, "Login");      // dotted name (nested)
```

`?` = Debug, `%` = Display. Useful khi value không primitive.

## 7.3 Span fields cũng structured

```rust
let span = info_span!("request", user_id = 42, request_id = "abc");
```

Mọi event trong span tự inherit fields:
```json
{
  "message": "processing",
  "spans": [{ "name": "request", "user_id": 42, "request_id": "abc" }]
}
```

## 7.4 Type của field

```rust
info!(
    int_field = 42_i64,
    bool_field = true,
    string_field = "hello",
    float_field = 3.14,
    debug_field = ?some_struct,
    display_field = %name,
);
```

Primitive types preserve. Complex types → Debug/Display string.

## 7.5 Sensitive data — Don't log!

⚠️ **NEVER**:
```rust
info!(password = user.password, "Login");   // ❌ password in logs!
```

Pattern senior:
```rust
struct User {
    id: u64,
    #[allow(dead_code)]
    password: String,   // never log
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("password", &"<redacted>")
            .finish()
    }
}
```

Hoặc `secrecy` crate:
```rust
use secrecy::{Secret, ExposeSecret};

struct User {
    password: Secret<String>,   // type ngăn log accidentally
}
```

## 7.6 High vs Low cardinality

- **Low cardinality** (OK): `level`, `service`, `region` (vài giá trị)
- **High cardinality** (be careful): `user_id`, `request_id` (millions of values)

Logs OK với high cardinality. **Metrics** thì không (Tầng 12).

---

# Tầng 8: #[instrument] macro — Auto-tracing functions

## 8.1 Cú pháp cơ bản

```rust
#[tracing::instrument]
fn process(user_id: u32, action: &str) {
    info!("processing");
}
```

Tự tạo span `process` với fields `user_id`, `action`. Events trong function tự nested.

## 8.2 Options

```rust
#[instrument(
    name = "custom_name",       // override span name
    level = "debug",            // span level
    skip(big_arg),              // không log big_arg
    skip_all,                   // skip tất cả args
    fields(extra = 42),         // thêm fields
    err,                        // log error nếu return Err
    ret,                        // log return value
)]
async fn process(user_id: u32, big_arg: BigStruct) -> Result<()> {
    Ok(())
}
```

## 8.3 `skip` — Tránh log large data

```rust
#[instrument(skip(data))]
fn process(user_id: u32, data: &[u8]) {   // data có thể GB
    // user_id logged, data không
}
```

Quan trọng: log binary data lớn → tốn memory, network, storage.

## 8.4 `err` — Auto log error

```rust
#[instrument(err)]
async fn process() -> Result<(), MyError> {
    risky_op()?;
    Ok(())
}
```

Nếu trả `Err`, tự log:
```
ERROR process: MyError { ... }
```

Đỡ phải `.map_err(|e| { error!("..."); e })`.

## 8.5 `ret` — Log return value

```rust
#[instrument(ret)]
fn classify(value: i32) -> Category {
    // ...
}
// Tự log return value
```

⚠️ Lưu ý: nếu return type lớn (Vec<T>), log có thể nặng.

## 8.6 Async functions

`#[instrument]` work tự nhiên với async fn — span follows await points.

```rust
#[instrument]
async fn fetch_user(id: u32) -> User {
    let raw = db.query(id).await;   // span context follows
    parse_user(raw)
}
```

## 8.7 Methods và Self

```rust
impl Service {
    #[instrument(skip(self))]   // skip self để tránh log struct lớn
    pub async fn handle(&self, req: Request) -> Response {
        // ...
    }
}
```

`skip(self)` rất phổ biến — self thường có nhiều field không liên quan.

## 8.8 Combine `target` cho filter

```rust
#[instrument(target = "myapp::business")]
fn process() { ... }
```

→ Filter `RUST_LOG=myapp::business=debug` để chỉ thấy business logs.

---

# Tầng 9: Filtering, Levels, Targets

## 9.1 EnvFilter — Cấu hình từ env

```rust
use tracing_subscriber::EnvFilter;

let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info"));

tracing_subscriber::fmt()
    .with_env_filter(filter)
    .init();
```

Set env:
```bash
RUST_LOG=info
RUST_LOG=debug,hyper=warn,sqlx=info
RUST_LOG=myapp::auth=trace,info
```

Syntax `target=level,target=level`. Default level cho rest.

## 9.2 Target = module path

Mặc định `target` của event = module path (`myapp::auth::login`).

```bash
RUST_LOG=myapp::auth=debug   # chỉ debug auth module
```

Override:
```rust
info!(target: "billing", "payment processed");
```

## 9.3 Per-span level

```rust
#[instrument(level = "debug")]
fn detailed_work() {
    debug!("step 1");
}

#[instrument(level = "trace")]
fn very_detailed() {
    trace!("step 1.1");
}
```

Production: filter ở info level → detailed_work + very_detailed bị skip → no overhead.

## 9.4 Dynamic filter reload

```rust
use tracing_subscriber::{reload, EnvFilter, Layer};

let (filter, reload_handle) = reload::Layer::new(
    EnvFilter::new("info")
);

tracing_subscriber::registry()
    .with(filter)
    .with(tracing_subscriber::fmt::layer())
    .init();

// Later:
reload_handle.modify(|f| *f = EnvFilter::new("debug")).unwrap();
```

→ Change log level runtime via HTTP endpoint (đẹp cho debugging prod).

## 9.5 Performance impact của filtering

`tracing` filter aggressive — `debug!()` ở info filter có **near-zero cost** (chỉ atomic check + return).

```rust
debug!("expensive {}", expensive_compute());
```

Even at info filter, `expensive_compute()` STILL evaluated! Vì argument eval trước macro.

Fix: explicit lazy:
```rust
if tracing::enabled!(tracing::Level::DEBUG) {
    debug!("expensive {}", expensive_compute());
}
```

Hoặc dùng span field record với `Empty`:
```rust
let span = debug_span!("expensive", result = field::Empty);
let _e = span.enter();
if tracing::span::Span::current().is_some() {
    span.record("result", &expensive_compute());
}
```

Trong practice: hot loop tránh log expensive args.

## 9.6 LevelFilter vs EnvFilter

```rust
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;

// LevelFilter — simple
let filter = LevelFilter::INFO;

// EnvFilter — flexible, per-target
let filter = EnvFilter::new("info,hyper=warn");
```

LevelFilter chỉ global level. EnvFilter có per-target — production default.

---

# Tầng 10: Async tracing — Quirks và best practices

## 10.1 Vấn đề: span context lost qua await

```rust
async fn process() {
    let span = info_span!("processing");
    let _enter = span.enter();   // ⚠️ guard
    
    other_async().await;          // yield point
    
    info!("after await");         // có còn trong span không?
}
```

`_enter` là RAII guard. Khi `await` yield, future suspend nhưng `_enter` vẫn alive trong frame.

Vấn đề thực: nếu task switch thread, thread khác poll future → span context có thể không follow đúng.

## 10.2 Fix 1: `.instrument()`

```rust
use tracing::Instrument;

async fn process() {
    async {
        info!("processing");
        other_async().await;
        info!("after await");
    }
    .instrument(info_span!("processing"))
    .await;
}
```

`.instrument()` đảm bảo span follow async block khắp polls.

## 10.3 Fix 2: `#[instrument]` (recommended)

```rust
#[instrument]
async fn process() {
    info!("processing");
    other_async().await;
    info!("after await");
}
```

Tương đương `.instrument()` tự động. Đây là cách idiomatic.

## 10.4 Span context propagation across `spawn`

```rust
let outer_span = info_span!("parent");
let _e = outer_span.enter();

tokio::spawn(async {
    info!("child");   // ⚠️ NOT in parent span
});
```

Spawn = new task, new context. Span không tự inherit.

Fix:
```rust
let outer_span = info_span!("parent");
let _e = outer_span.enter();

tokio::spawn(async {
    info!("child");
}.instrument(info_span!("child")));   // explicit
```

Hoặc current span:
```rust
let parent = tracing::Span::current();
tokio::spawn(async move {
    let _e = parent.enter();
    info!("child in parent span");
});
```

## 10.5 tokio-console — Async runtime profiler

```toml
[dependencies]
console-subscriber = "0.4"
tokio = { version = "1", features = ["tracing"] }
```

```rust
console_subscriber::init();
```

Run:
```bash
RUSTFLAGS="--cfg tokio_unstable" cargo run
```

Trong terminal khác:
```bash
cargo install tokio-console
tokio-console
```

UI realtime:
- Tasks alive
- Busy/idle time per task
- Resources (mutex contention, sleep, channels)

CPU profiler **không thấy** task-level info. tokio-console **bắt buộc** cho async debugging.

## 10.6 Tracing in spawned tasks

```rust
async fn handle_request(req: Request) {
    let request_id = generate_id();
    let span = info_span!("request", request_id = %request_id);
    
    async move {
        let user = fetch_user().await;
        process_request(user, req).await;
    }
    .instrument(span)
    .await;
}
```

Mỗi request 1 span — tất cả events từ async work nested under.

---

# Tầng 11: OpenTelemetry — Standard cho distributed tracing

## 11.1 OpenTelemetry là gì?

**OpenTelemetry** (OTel) = OPEN standard cho telemetry data. Vendor-neutral:
- **API**: cách bạn instrument code
- **SDK**: implementations
- **Protocol** (OTLP): wire format
- **Backends**: Jaeger, Tempo, Honeycomb, Datadog, AWS X-Ray all support OTLP

→ Code 1 lần, switch backend dễ.

## 11.2 OTel concepts

| Concept | Mô tả |
|---------|-------|
| **Trace** | Cây spans cho 1 request |
| **Span** | Đơn vị work (start, end, attributes, events) |
| **TraceID** | UUID xác định 1 trace |
| **SpanID** | UUID xác định 1 span |
| **Parent SpanID** | Span cha trong cây |
| **Attributes** | Key-value gắn span |
| **Events** | Time-stamped log trong span |
| **Baggage** | Propagated metadata across services |
| **Resource** | Service-level info (name, version, host) |
| **Exporter** | Send data ra backend |
| **Sampler** | Quyết định lưu trace nào |

## 11.3 Setup tracing + OpenTelemetry

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-opentelemetry = "0.27"
opentelemetry = "0.26"
opentelemetry_sdk = { version = "0.26", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.26", features = ["grpc-tonic"] }
```

```rust
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;

fn init_telemetry() {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()
        .unwrap();
    
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "myapp"),
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]))
        .build();
    
    let tracer = provider.tracer("myapp");
    
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer();
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();
}

#[tokio::main]
async fn main() {
    init_telemetry();
    // ...
}
```

Bây giờ:
- `tracing::info_span!` → OTel span tự động
- Spans ship qua OTLP gRPC ra collector
- Jaeger UI thấy được

## 11.3.1 Setup minimal — Jaeger all-in-one cho dev

```bash
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

App ship spans qua port 4317 (OTLP gRPC). Mở browser http://localhost:16686 để xem traces.

## 11.4 Span attributes

```rust
use tracing::field;

#[instrument(fields(http.method = field::Empty, http.url = field::Empty))]
async fn handle(req: Request) {
    let span = tracing::Span::current();
    span.record("http.method", req.method().as_str());
    span.record("http.url", req.uri().to_string().as_str());
    
    process(req).await;
}
```

OTel có **semantic conventions** — keys tiêu chuẩn:
- `http.method`, `http.status_code`, `http.url`
- `db.system`, `db.statement`
- `messaging.system`, `messaging.destination`

Dùng để backends recognize và group.

## 11.5 Events inside spans

```rust
let span = info_span!("operation");
let _enter = span.enter();

info!("step 1");   // event 1
do_work();
info!("step 2");   // event 2
```

Events log trong span → trong Jaeger UI thấy timeline.

## 11.6 Exporter options

| Exporter | Backend |
|----------|---------|
| `opentelemetry-otlp` (gRPC) | Jaeger, Tempo, OTel Collector |
| `opentelemetry-otlp` (HTTP) | Same but HTTP/JSON |
| `opentelemetry-jaeger` | Jaeger native protocol (deprecated, use OTLP) |
| `opentelemetry-zipkin` | Zipkin |
| `opentelemetry-aws` | AWS X-Ray |
| `opentelemetry-stdout` | Debug (print spans) |

OTLP recommended — universal.

---

# Tầng 12: Metrics — metrics crate và Prometheus

## 12.1 metrics crate

```toml
[dependencies]
metrics = "0.24"
metrics-exporter-prometheus = "0.16"
```

```rust
use metrics::{counter, gauge, histogram};

// Counter — luôn tăng
counter!("http_requests_total", "method" => "GET", "status" => "200").increment(1);

// Gauge — số hiện tại (lên xuống)
gauge!("connections_active").set(42.0);

// Histogram — distribution
histogram!("request_duration_seconds").record(0.123);
```

## 12.2 Prometheus exporter

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

fn init_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9000))
        .install()
        .expect("failed to install");
}
```

App exposes `http://localhost:9000/metrics`:
```
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 12345

# HELP connections_active Current connections
# TYPE connections_active gauge
connections_active 42

# HELP request_duration_seconds Request duration
# TYPE request_duration_seconds histogram
request_duration_seconds_bucket{le="0.1"} 100
request_duration_seconds_bucket{le="0.5"} 5000
request_duration_seconds_bucket{le="1.0"} 12000
request_duration_seconds_count 12345
request_duration_seconds_sum 1234.5
```

Prometheus server scrape endpoint định kỳ.

## 12.3 3 loại metrics chính

### Counter — monotonic increasing

```rust
counter!("events_total").increment(1);
counter!("bytes_received_total").increment(req.body().len() as u64);
```

Use case: events count, bytes, errors.

Rate (operations per second) tính bằng `rate()` trong PromQL.

### Gauge — current value

```rust
gauge!("memory_bytes").set(get_memory_usage());
gauge!("queue_length").increment(1.0);
gauge!("queue_length").decrement(1.0);
```

Use case: current connections, queue depth, memory usage.

### Histogram — distribution

```rust
histogram!("http_request_duration_seconds").record(elapsed);
```

Use case: latency, response size.

Histogram → có thể tính P50, P95, P99 trong Prometheus:
```promql
histogram_quantile(0.99, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))
```

## 12.4 Labels (dimensions)

```rust
counter!("http_requests_total",
    "method" => "GET",
    "endpoint" => "/users",
    "status" => "200"
).increment(1);
```

Labels create separate time series. Cardinality concerns:
- 4 methods × 100 endpoints × 5 status codes = 2000 series
- OK in Prometheus
- 4 methods × 100 endpoints × 1M user_ids = 400M series — **explode**

**Never** label by high-cardinality field (user_id, request_id, IP). Use trace/log for that.

## 12.5 Common metrics naming

Convention từ Prometheus:
- Suffix `_total` cho counter
- Suffix `_seconds` cho duration
- Suffix `_bytes` cho size
- Suffix `_ratio` cho percentage (0-1)

```rust
counter!("http_requests_total");
histogram!("http_request_duration_seconds");
gauge!("memory_usage_bytes");
gauge!("cpu_usage_ratio");
```

## 12.6 4 Golden Signals (Google SRE)

Service health phải có 4 metrics:

1. **Traffic** — requests per second
   ```rust
   counter!("requests_total").increment(1);
   ```

2. **Errors** — error rate
   ```rust
   counter!("errors_total", "type" => err.kind()).increment(1);
   ```

3. **Latency** — P50, P95, P99
   ```rust
   histogram!("request_duration_seconds").record(elapsed);
   ```

4. **Saturation** — resource utilization
   ```rust
   gauge!("connection_pool_used").set(pool.used() as f64);
   gauge!("cpu_usage_ratio").set(get_cpu() / 100.0);
   ```

→ Mỗi service expose 4 này = good baseline.

## 12.7 RED Method — 3 metrics đơn giản hơn

For request-driven services:
- **R**ate (requests/sec)
- **E**rrors (failures/sec)
- **D**uration (latency)

Đơn giản hơn 4 Golden Signals, đủ cho most web services.

## 12.8 USE Method — Cho resources

For CPU, memory, disk:
- **U**tilization (%)
- **S**aturation (queue depth)
- **E**rrors (faults)

→ Combine: RED for services + USE for infrastructure.

---

# Tầng 13: Distributed Tracing — Context propagation

## 13.1 Vấn đề: 1 request đi qua N services

```
Client → API Gateway → User Service → DB
                    ↘ Order Service → Payment Service
                                    ↘ Stripe API
```

Mỗi service log riêng. Làm sao biết các log/spans thuộc cùng 1 request?

→ **Trace context propagation**.

## 13.2 W3C Trace Context standard

HTTP header propagate:
```
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
             ┬  ┬                                ┬                ┬
             │  │                                │                └ flags
             │  │                                └ parent span ID
             │  └ trace ID (16 byte)
             └ version
```

Server receive request:
1. Parse `traceparent` → extract trace_id + parent_span_id
2. Create new span as child of parent
3. Outgoing requests: pass new `traceparent` with own span_id

OpenTelemetry SDK does this automatically with proper propagators.

## 13.3 Setup propagation

```rust
use opentelemetry::global;
use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;

fn init() {
    let propagator = TextMapCompositePropagator::new(vec![
        Box::new(TraceContextPropagator::new()),
        // có thể thêm baggage propagator
    ]);
    global::set_text_map_propagator(propagator);
    // ... init tracer như Tầng 11
}
```

## 13.4 HTTP client — Inject context

```rust
use opentelemetry::global;
use tracing::Span;

async fn call_external(url: &str) -> reqwest::Response {
    let span = Span::current();
    let cx = span.context();
    
    let mut req = reqwest::Client::new().get(url);
    
    // Inject trace context into headers
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut()));
    });
    
    req.send().await.unwrap()
}
```

Crate `reqwest-tracing` simplifies:
```rust
use reqwest_middleware::ClientBuilder;
use reqwest_tracing::TracingMiddleware;

let client = ClientBuilder::new(reqwest::Client::new())
    .with(TracingMiddleware::default())
    .build();
// Tự inject traceparent header
```

## 13.5 HTTP server — Extract context

Với axum:
```rust
use axum::{Router, middleware};
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/", get(handler))
    .layer(TraceLayer::new_for_http());
```

`TraceLayer` tự extract `traceparent` từ request, create span. Mọi events trong handler nested under.

## 13.6 Baggage — Metadata propagation

Beyond trace context, baggage carry app-specific data:
```
baggage: user_id=42, region=us-east-1
```

```rust
use opentelemetry::baggage::BaggageExt;

let cx = Context::current().with_baggage(vec![
    KeyValue::new("user_id", 42i64),
]);
```

Downstream services có thể read baggage để route, log, decide.

⚠️ Baggage propagates qua **mọi** service → don't put secrets, PII.

## 13.7 Trace in logs — Correlation

Logs phải có `trace_id` để correlate với traces:

```rust
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[instrument]
async fn handler() {
    let trace_id = tracing::Span::current().context().span().span_context().trace_id();
    info!(%trace_id, "handling request");
}
```

→ Logs có `trace_id` field. Click log → open trace in Jaeger/Tempo.

Better: dùng `tracing-opentelemetry` để tự inject `trace_id` vào log JSON.

---

# Tầng 14: Sampling — Khi telemetry quá nhiều

## 14.1 Vấn đề: 1B requests/day = 1B traces?

Big service: millions/billions requests. Sending all traces:
- Storage: petabytes
- Network bandwidth
- Backend cost
- Mostly redundant (95% traces look same)

→ **Sample**: chỉ keep subset.

## 14.2 Sampling strategies

### Head sampling — Decide at start

```rust
use opentelemetry_sdk::trace::Sampler;

let provider = TracerProvider::builder()
    .with_sampler(Sampler::TraceIdRatioBased(0.01))   // 1% sample
    .build();
```

Pros: Cheap (skip work for unsampled).
Cons: Lose interesting traces (errors, slow).

### Tail sampling — Decide at end

OTel Collector can buffer traces, then sample based on:
- Trace has error → keep
- Latency > threshold → keep
- Random 1% otherwise

Pros: Keep important traces.
Cons: Expensive (buffer all traces in collector).

### Adaptive sampling

Adjust rate based on traffic:
- Low traffic → 100%
- High traffic → 1%

### Custom samplers

```rust
struct ErrorAndSlowSampler {
    base: Sampler,
}

impl Sampler for ErrorAndSlowSampler {
    fn should_sample(...) -> SamplingResult {
        // logic
    }
}
```

## 14.3 Sampling recommendations

```
Traffic       Strategy             Comment
─────────     ──────────           ───────
< 1k/s        100%                 Sample everything
1k-10k/s      10%                  Random sample
10k-100k/s    1%                   Add tail sampling for errors
> 100k/s      0.1% + tail          Necessary
```

Always tail-sample errors at 100% — that's what you need.

## 14.4 Sampling logs

Logs cũng có thể quá nhiều. Pattern:

```rust
// Every Nth iteration
if iter_count % 1000 == 0 {
    info!("Progress: {}", iter_count);
}

// Time-based throttle
static LAST_LOG: AtomicU64 = AtomicU64::new(0);
let now = epoch_secs();
if now - LAST_LOG.load(Ordering::Relaxed) > 60 {
    LAST_LOG.store(now, Ordering::Relaxed);
    info!("Heartbeat");
}
```

Hoặc crate `tracing_subscriber::filter::Targets` với fine-grained rate limit.

## 14.5 Metrics aggregation built-in

Metrics đã aggregate — không cần sample. Trade-off: lost individual values.

Histograms keep distribution shape → P50/P99 OK without sampling.

---

# Tầng 15: Patterns và Antipatterns

## 15.1 ✅ Pattern: Request ID propagation

```rust
use axum::extract::Request;
use uuid::Uuid;

async fn middleware(req: Request, next: Next) -> Response {
    let request_id = req.headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    let span = tracing::info_span!("request", request_id = %request_id);
    next.run(req).instrument(span).await
}
```

Mỗi request 1 ID → grep logs by ID khi user complain.

## 15.2 ✅ Pattern: 4 Golden Signals dashboard

Mỗi service expose:
```rust
// In handler:
let start = Instant::now();
let result = handle(req).await;
let elapsed = start.elapsed();

counter!("requests_total", "endpoint" => endpoint).increment(1);
histogram!("request_duration_seconds", "endpoint" => endpoint)
    .record(elapsed.as_secs_f64());

if result.is_err() {
    counter!("errors_total", "endpoint" => endpoint).increment(1);
}
```

Grafana dashboard: traffic, errors, latency. Setup once, alerts forever.

## 15.3 ✅ Pattern: Layered initialization

```rust
fn init_observability() {
    init_metrics();
    init_tracing_with_otel();
    init_panic_hook();
}

fn init_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "panic!");
        default(info);
    }));
}
```

Panic → log với full info trước khi exit.

## 15.4 ✅ Pattern: Per-environment config

```rust
#[derive(serde::Deserialize)]
struct ObservabilityConfig {
    log_level: String,
    log_format: String,        // "json" or "text"
    otel_endpoint: Option<String>,
    sample_rate: f64,
}

// dev:  log_level=debug, format=text, no otel
// prod: log_level=info, format=json, otel=..., sample=0.01
```

## 15.5 ✅ Pattern: Health check endpoint

```rust
async fn health() -> impl IntoResponse {
    // Check critical dependencies
    let db_ok = check_db().await;
    let redis_ok = check_redis().await;
    
    if db_ok && redis_ok {
        (StatusCode::OK, "OK")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Degraded")
    }
}
```

Kubernetes/load balancer probe → restart if unhealthy.

## 15.6 ✅ Pattern: Log error path vs success differently

```rust
async fn handle(req: Request) -> Result<Response, Error> {
    match process(req).await {
        Ok(resp) => {
            debug!("ok");   // debug only
            Ok(resp)
        }
        Err(e) => {
            error!(?e, "Failed");   // ERROR with full info
            Err(e)
        }
    }
}
```

Error log có full context. Success log thưa thớt (track via metrics).

## 15.7 ❌ Antipattern: Log everything verbose

```rust
fn process(items: &[Item]) {
    info!("Processing {} items", items.len());
    for item in items {
        info!("Processing item {:?}", item);   // ❌ huge log volume
        process_one(item);
    }
}
```

Per-item log → log volume explodes. Use:
- `debug!` level (filtered out in prod)
- Aggregate metric: `counter!("items_processed_total").increment(items.len())`

## 15.8 ❌ Antipattern: Log secrets / PII

```rust
info!(user = ?user, "Login");   // ❌ might include password, email, SSN
```

Audit logs for secrets. Use Debug impl that redacts, or `secrecy` crate.

## 15.9 ❌ Antipattern: High-cardinality metric labels

```rust
counter!("requests_total", "user_id" => user.id.to_string()).increment(1);
//                          ^^^^^^^^^^
// 1M users → 1M time series → Prometheus melts
```

Metrics labels low-cardinality only. User-level data → logs/traces.

## 15.10 ❌ Antipattern: print/println cho logging

```rust
println!("Server started");   // ❌
eprintln!("Error: {}", e);    // ❌
```

Issues:
- No log level / filtering
- No structured
- Hard to redirect
- No correlation

Always use `tracing::info!` / `error!`.

## 15.11 ❌ Antipattern: Bỏ qua error trong log layer

```rust
let _ = tracing_subscriber::fmt::layer().init();   // ignore error
```

Logging init fail = app blind. Should panic to surface:
```rust
tracing_subscriber::fmt::init();   // panic if fail
```

## 15.12 ❌ Antipattern: Span every function

```rust
#[instrument] fn trivial_helper(x: i32) -> i32 { x + 1 }
```

Span has overhead (~100-300ns). Triv helpers don't need. Instrument at **logical boundaries** (request handler, async task, expensive op).

---

# Tổng kết — 12 nguyên tắc senior

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Observability ≠ monitoring. Phải có cả 3 pillars.             │
│                                                                  │
│ 2. tracing > log crate. Structured + spans + async-aware.        │
│                                                                  │
│ 3. JSON output cho production. Text cho dev.                     │
│                                                                  │
│ 4. #[instrument] với skip(self), skip(big_args).                 │
│                                                                  │
│ 5. EnvFilter per-target — `myapp=debug,hyper=warn`.              │
│                                                                  │
│ 6. Async: `.instrument()` hoặc `#[instrument]`, không guard.     │
│                                                                  │
│ 7. tokio-console cho async runtime profiling.                    │
│                                                                  │
│ 8. OpenTelemetry standard cho distributed tracing.               │
│                                                                  │
│ 9. 4 Golden Signals (RED) cho mỗi service.                       │
│                                                                  │
│ 10. Metrics labels low-cardinality. User_id → logs/traces.       │
│                                                                  │
│ 11. Sample aggressively. Tail-sample errors 100%.                │
│                                                                  │
│ 12. NEVER log secrets / PII. Redact qua Debug impl.              │
└──────────────────────────────────────────────────────────────────┘
```

---

# Stack production hoàn chỉnh

```
┌────────────────────────────────────────────────────────┐
│                  YOUR RUST APP                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │ tracing::info!(...)                              │  │
│  │ tracing::info_span!(...)                         │  │
│  │ metrics::counter!(...)                           │  │
│  └─────────────┬────────────────────────────────────┘  │
│                │                                       │
│  ┌─────────────▼─────────────────┐                     │
│  │ tracing_subscriber             │                     │
│  │   ├─ fmt::layer (JSON)        │ → stdout/stderr    │
│  │   ├─ opentelemetry::layer     │ ─┐                  │
│  │   └─ env_filter               │  │                  │
│  └────────────────────────────────┘  │                  │
│                                      │                  │
│  ┌────────────────────────────────┐  │                  │
│  │ metrics-exporter-prometheus    │  │                  │
│  │   /metrics endpoint            │  │                  │
│  └────────────────────────────────┘  │                  │
└──────────────────────────────────────┼──────────────────┘
                                       │
                ┌──────────────────────┼──────────────────┐
                │                      │                  │
                ▼                      ▼                  ▼
        ┌──────────────┐       ┌──────────────┐    ┌──────────────┐
        │  Promtail /  │       │  OpenTelemetry│    │  Prometheus  │
        │  Vector /    │       │  Collector    │    │  scraper     │
        │  Fluentd     │       │  (OTLP)       │    │              │
        └──────┬───────┘       └───────┬───────┘    └──────┬───────┘
               │                       │                    │
               ▼                       ▼                    ▼
        ┌──────────────┐       ┌──────────────┐    ┌──────────────┐
        │ Loki/        │       │ Tempo/       │    │ Prometheus/  │
        │ Elasticsearch│       │ Jaeger/      │    │ Mimir/       │
        │              │       │ Honeycomb    │    │ Cortex       │
        └──────┬───────┘       └───────┬──────┘    └──────┬───────┘
               │                       │                   │
               └───────────────────────┴───────────────────┘
                                       │
                                       ▼
                              ┌──────────────────┐
                              │     Grafana       │
                              │   (dashboards     │
                              │    + alerts)      │
                              └──────────────────┘
```

---

# Crates senior toolkit

| Crate | Mục đích |
|-------|----------|
| `tracing` | Core API |
| `tracing-subscriber` | Subscribers, layers, filtering |
| `tracing-opentelemetry` | OTel integration |
| `tracing-bunyan-formatter` | Bunyan JSON format |
| `tracing-log` | Forward log → tracing |
| `tracing-error` | SpanTrace cho errors |
| `tracing-actix-web` | Actix HTTP layer |
| `tower-http` | TraceLayer for tower (axum) |
| `console-subscriber` | tokio-console support |
| `opentelemetry` | OTel core |
| `opentelemetry-otlp` | OTLP exporter |
| `metrics` | Metrics facade |
| `metrics-exporter-prometheus` | Prometheus exporter |
| `secrecy` | Secret type to avoid logging |
| `reqwest-tracing` | HTTP client middleware |

---

# Lộ trình tiếp theo

Bạn đã có 12 chủ đề:

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
12. observability      ← MỚI
```

Còn các topic chuyên sâu:

- **Unsafe Rust** — raw pointer, UnsafeCell, atomic ordering, FFI
- **Iterator deep dive** — implement, lazy, rayon parallel
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 12 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
