# Observability Rust — Minh Hoạ Trực Quan

> Companion visual cho [observability.md](./observability.md). Đọc song song.

---

## 1. Bức tranh lớn — Observability Universe

```
                       OBSERVABILITY = 3 PILLARS
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │       LOGS              METRICS              TRACES    │
       │   ┌──────────┐      ┌──────────┐         ┌──────────┐  │
       │   │ Discrete │      │ Aggregated│        │ Causal   │  │
       │   │ events    │      │ numbers   │        │ graph    │  │
       │   │           │      │           │        │           │  │
       │   │ "What     │      │ "How      │        │ "Why      │  │
       │   │  happened"│      │  many?"   │        │  slow?"   │  │
       │   └─────┬─────┘      └─────┬─────┘        └─────┬─────┘  │
       │         │                  │                    │        │
       │         └──────────────────┼────────────────────┘        │
       │                            │                              │
       │                            ▼                              │
       │              ┌──────────────────────┐                     │
       │              │  CORRELATED via       │                     │
       │              │  trace_id, request_id │                     │
       │              └──────────────────────┘                     │
       │                                                        │
       │   Tools: tracing, OpenTelemetry, Prometheus, Grafana   │
       └────────────────────────────────────────────────────────┘
```

---

## 2. 3 Pillars — Khác biệt và use case

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │   LOG: discrete event in time                               │
   │   ─────────────────────────────                             │
   │                                                             │
   │   Timeline →                                                │
   │   •          •            •            •                    │
   │   "user 42   "DB query   "payment    "logout"               │
   │    login"    failed"      success"                          │
   │                                                             │
   │   Use: debug specific issue, audit, errors                  │
   │                                                             │
   ├─────────────────────────────────────────────────────────────┤
   │                                                             │
   │   METRIC: number over time                                  │
   │   ────────────────────────                                  │
   │                                                             │
   │   value   ▲                                                 │
   │           │       ╱╲                                        │
   │           │    ╱╲╱  ╲          ← P99 latency               │
   │           │  ╱╲      ╲    ╱╲                                │
   │           │╱           ╲ ╱  ╲                               │
   │           └─────────────────────►  time                     │
   │                                                             │
   │   Use: dashboards, alerts, trends                           │
   │                                                             │
   ├─────────────────────────────────────────────────────────────┤
   │                                                             │
   │   TRACE: causal graph of 1 request                          │
   │   ─────────────────────────────────                         │
   │                                                             │
   │   Trace: req-abc-123                                        │
   │   ├── api_gateway (200ms)                                   │
   │   │   ├── auth (50ms)                                       │
   │   │   └── order_svc (140ms)                                 │
   │   │       ├── db_query (60ms)                               │
   │   │       └── payment (70ms)                                │
   │   │           └── stripe_api (50ms)                         │
   │                                                             │
   │   Use: find bottleneck, distributed debugging               │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 3. So sánh `log` vs `tracing`

```
   log crate:
   ──────────
   
   info!("User {} logged in via {}", user_id, method);
                │
                ▼
   Output: "INFO User 42 logged in via oauth"
   
   ❌ Field embedded trong text
   ❌ Khó parse/query
   ❌ Không có hierarchy
   ❌ Không context propagation
   
   
   tracing crate:
   ──────────────
   
   info!(user_id, method, "User logged in");
                │
                ▼
   Output JSON:
   {
     "message": "User logged in",
     "user_id": 42,
     "method": "oauth",
     "spans": [{"name": "request", "request_id": "abc"}]
   }
   
   ✅ Structured fields
   ✅ Easy to query (Elasticsearch, Loki)
   ✅ Spans hierarchy
   ✅ Async-aware
   
   
   Migration: tracing-log catch log::info!() automatically
```

---

## 4. Spans vs Events

```
   ┌────────────────────────────────────────────────────────────┐
   │                                                            │
   │   Event = point in time                                    │
   │   Span  = duration (start + end)                           │
   │                                                            │
   │   Timeline ──────────────────────────────────►            │
   │                                                            │
   │   Event:        ●          ●           ●                   │
   │              info!()    info!()      error!()              │
   │                                                            │
   │   Span:    ┌──────────────────────────────┐                │
   │            │       active                   │                │
   │            └──────────────────────────────┘                │
   │            start                          end              │
   │                                                            │
   │   Events thường NẰM trong span                             │
   │   Span = context cho events                                │
   └────────────────────────────────────────────────────────────┘
   
   
   Hierarchical spans:
   ───────────────────
   
   span "request"
     ├── event: "start"
     ├── span "auth"
     │     ├── event: "validating"
     │     └── event: "success"
     ├── span "db_query"
     │     └── event: "query took 50ms"
     └── event: "response sent"
   
   
   3 cách tạo span:
   ────────────────
   
   1. Manual:
      let span = info_span!("op", user_id = 42);
      let _enter = span.enter();
      do_work();
      // _enter drop → span exit
   
   2. in_scope:
      info_span!("op").in_scope(|| do_work());
   
   3. #[instrument] (recommended):
      #[instrument]
      fn op() { ... }
```

---

## 5. tracing-subscriber Layer architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                  YOUR APP                                │
   │   tracing::info!(user_id, "logged in")                   │
   │                       │                                  │
   │                       ▼                                  │
   │              Global Dispatcher                           │
   │                       │                                  │
   │                       ▼                                  │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │           Registry (Subscriber)                 │    │
   │   │                                                 │    │
   │   │   ┌─────────────────────────────────────┐       │    │
   │   │   │  Filter Layer (EnvFilter)            │       │    │
   │   │   │  "info,hyper=warn"                   │       │    │
   │   │   └────────────────┬────────────────────┘       │    │
   │   │                    ▼ (events that pass filter)  │    │
   │   │   ┌─────────────────────────────────────┐       │    │
   │   │   │  Layer 1: fmt::layer (stdout JSON)  │ ──────┼────┼──► stderr
   │   │   └─────────────────────────────────────┘       │    │
   │   │                    │                            │    │
   │   │                    ▼                            │    │
   │   │   ┌─────────────────────────────────────┐       │    │
   │   │   │  Layer 2: opentelemetry::layer       │ ──────┼────┼──► OTLP
   │   │   └─────────────────────────────────────┘       │    │
   │   │                    │                            │    │
   │   │                    ▼                            │    │
   │   │   ┌─────────────────────────────────────┐       │    │
   │   │   │  Layer 3: custom counter             │ ──────┼────┼──► metric
   │   │   └─────────────────────────────────────┘       │    │
   │   └─────────────────────────────────────────────────┘    │
   └──────────────────────────────────────────────────────────┘
   
   
   Code:
   ─────
   tracing_subscriber::registry()
       .with(EnvFilter::new("info"))
       .with(fmt::layer().json())
       .with(opentelemetry_layer)
       .init();
```

---

## 6. #[instrument] macro — Auto-span

```
   ┌──────────────────────────────────────────────────────────┐
   │ Code bạn viết:                                           │
   │                                                          │
   │   #[tracing::instrument(                                 │
   │       skip(self, big_data),                              │
   │       fields(user_id = user.id),                         │
   │       err,                                               │
   │   )]                                                     │
   │   async fn process(&self,                                │
   │                    user: &User,                          │
   │                    big_data: &[u8])                      │
   │       -> Result<()> {                                    │
   │       info!("processing");                               │
   │       step_1().await?;                                   │
   │       step_2().await?;                                   │
   │       Ok(())                                             │
   │   }                                                      │
   └──────────────────────────────────────────────────────────┘
                              │
                              ▼ Expanded behavior
   ┌──────────────────────────────────────────────────────────┐
   │ Equivalent to:                                           │
   │                                                          │
   │   async fn process(&self, user: &User, big_data: &[u8])  │
   │       -> Result<()> {                                    │
   │       let span = info_span!(                             │
   │           "process",                                     │
   │           user = ..., // user borrowed via Debug         │
   │           user_id = user.id,                             │
   │           // self skipped                                │
   │           // big_data skipped                            │
   │       );                                                 │
   │       async {                                            │
   │           info!("processing");                           │
   │           step_1().await?;                               │
   │           step_2().await?;                               │
   │           Ok(())                                         │
   │       }                                                  │
   │       .instrument(span)                                  │
   │       .await                                             │
   │       .inspect_err(|e| error!(?e))                      │
   │   }                                                      │
   └──────────────────────────────────────────────────────────┘
   
   
   Common options:
   ───────────────
   
   ┌────────────────────┬─────────────────────────────────────┐
   │ Option             │ Effect                              │
   ├────────────────────┼─────────────────────────────────────┤
   │ name = "..."       │ Override span name                  │
   │ level = "debug"    │ Span level                          │
   │ skip(arg)          │ Don't record arg                    │
   │ skip_all           │ Skip all args                       │
   │ fields(k = v)      │ Extra fields                        │
   │ err                │ Log error if return Err             │
   │ ret                │ Log return value                    │
   │ target = "..."     │ Custom target                       │
   └────────────────────┴─────────────────────────────────────┘
```

---

## 7. Async tracing problem & fix

```
   ❌ PROBLEM — Span context lost qua await:
   ──────────────────────────────────────────
   
   async fn process() {
       let span = info_span!("processing");
       let _enter = span.enter();
       
       other_async().await;     ← yield point
                    │
                    ▼ Task có thể switch thread
       
       info!("after await");    ← span context có thể lost!
   }
   
   
   ✅ FIX 1 — .instrument() wrap:
   ───────────────────────────────
   
   async fn process() {
       async {
           info!("processing");
           other_async().await;
           info!("after await");
       }
       .instrument(info_span!("processing"))
       .await;
   }
   
   
   ✅ FIX 2 — #[instrument] (recommended):
   ────────────────────────────────────────
   
   #[instrument]
   async fn process() {
       info!("processing");
       other_async().await;
       info!("after await");
   }
   
   
   Spawn task — context không tự inherit:
   ───────────────────────────────────────
   
   ❌ Lost context:
   tokio::spawn(async {
       info!("child");   // không trong parent span
   });
   
   ✅ Explicit propagation:
   let parent = Span::current();
   tokio::spawn(async move {
       let _e = parent.enter();
       info!("child");
   });
   
   ✅ Or instrument:
   tokio::spawn(
       async { info!("child"); }
           .instrument(info_span!("child_task"))
   );
```

---

## 8. tokio-console — Async runtime view

```
   ┌──────────────────────────────────────────────────────────┐
   │ Setup:                                                   │
   │                                                          │
   │   [dependencies]                                         │
   │   console-subscriber = "0.4"                             │
   │   tokio = { version = "1", features = ["tracing"] }      │
   │                                                          │
   │   // main:                                               │
   │   console_subscriber::init();                            │
   │                                                          │
   │   Run app:                                               │
   │   RUSTFLAGS="--cfg tokio_unstable" cargo run             │
   │                                                          │
   │   In another terminal:                                   │
   │   tokio-console                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   UI realtime:
   ────────────
   
   ┌──────────────────────────────────────────────────────────┐
   │ tokio-console                                            │
   │                                                          │
   │   Tasks (5 running, 1 blocked)                          │
   │   ──────────────────────────────                         │
   │   ID  Name              Polled   Busy    Idle   State    │
   │   1   main              1234     5s      ...    running  │
   │   2   handle_request    5678     20ms    100ms  idle     │
   │   3   db_pool::worker   9012     800ms   500ms  idle     │
   │   4   parse_json        100      50ms    0      ⚠️ busy  │
   │   ...                                                    │
   │                                                          │
   │   Resources (mutex, channels):                           │
   │   ───────────────────────────                            │
   │   db_pool::Mutex   contention: 200ms (high!)             │
   │   chan_buffer       full: 30s (slow consumer?)           │
   │                                                          │
   │ → Press d for details on specific task                   │
   └──────────────────────────────────────────────────────────┘
   
   ⟹ Insight không thấy được trong perf/flamegraph:
     • Task scheduling
     • Time idle waiting
     • Lock contention
     • Channel saturation
```

---

## 9. OpenTelemetry — Architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │           YOUR RUST APP                         │    │
   │   │                                                 │    │
   │   │   tracing::info_span!("handle_request")         │    │
   │   │            │                                    │    │
   │   │            ▼                                    │    │
   │   │   tracing_opentelemetry::layer                  │    │
   │   │            │                                    │    │
   │   │            ▼                                    │    │
   │   │   opentelemetry_sdk::trace::TracerProvider      │    │
   │   │            │                                    │    │
   │   │            ▼                                    │    │
   │   │   opentelemetry-otlp Exporter                   │    │
   │   │            │                                    │    │
   │   └────────────┼────────────────────────────────────┘    │
   │                │  OTLP/gRPC (port 4317)                  │
   │                ▼                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │  OpenTelemetry Collector (optional)             │    │
   │   │  ─────────────────────────────────              │    │
   │   │  • Batch                                        │    │
   │   │  • Process (filter, transform)                  │    │
   │   │  • Tail sampling                                │    │
   │   │  • Multiple destinations                        │    │
   │   └─────┬──────────────┬──────────────┬─────────────┘    │
   │         │              │              │                  │
   │         ▼              ▼              ▼                  │
   │   ┌──────────┐   ┌──────────┐   ┌──────────────┐         │
   │   │ Jaeger   │   │ Tempo    │   │ Honeycomb /  │         │
   │   │ (local)  │   │ (Grafana)│   │ Datadog /    │         │
   │   │          │   │          │   │ AWS X-Ray    │         │
   │   └──────────┘   └──────────┘   └──────────────┘         │
   │         │              │              │                  │
   │         └──────────────┴──────────────┘                  │
   │                        │                                 │
   │                        ▼                                 │
   │              ┌──────────────────┐                        │
   │              │ Visualize traces │                        │
   │              │ in browser UI    │                        │
   │              └──────────────────┘                        │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. OpenTelemetry Span

```
   Span structure:
   ───────────────
   
   ┌─────────────────────────────────────────┐
   │ Span                                    │
   │                                         │
   │  TraceID:       abc-123-...             │ ← shared across services
   │  SpanID:        def-456-...             │ ← unique per span
   │  ParentSpanID:  111-222-...             │ ← parent in trace tree
   │                                         │
   │  Name:          "GET /users/:id"        │
   │  Kind:          SERVER                   │  (or CLIENT, INTERNAL)
   │  Start time:    1716_000_000_000_000_000│
   │  End time:      1716_000_200_000_000_000│
   │                                         │
   │  Status:        OK                       │
   │                                         │
   │  Attributes:                             │
   │    http.method:       GET                │
   │    http.url:          /users/42          │
   │    http.status_code:  200                │
   │    user.id:           42                 │
   │                                         │
   │  Events (logs in span):                  │
   │    [t=10ms]  "validating auth"           │
   │    [t=50ms]  "fetching from DB"          │
   │    [t=180ms] "rendering response"        │
   │                                         │
   │  Links:                                  │
   │    → some_other_trace                    │
   └─────────────────────────────────────────┘
   
   
   Visualization trong Jaeger UI:
   ──────────────────────────────
   
   Trace: GET /users/42  total: 200ms
   ────────────────────────────────────────────────────
                                                   200ms
   |═══════════════════════════════════════════════|  api_gateway
       |═══════|                                       auth (50ms)
                  |═════════════════════════════════| order_service (140ms)
                       |═══════|                       db_query (60ms)
                                  |════════════════|   payment_service (70ms)
                                       |══════════|    stripe_api (50ms)
   0      50      100      150     200ms
```

---

## 11. Distributed Tracing — Context propagation

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Client                                                 │
   │     │                                                    │
   │     │ HTTP GET /order                                    │
   │     │ traceparent: 00-abc...-001-01                      │
   │     ▼                                                    │
   │   API Gateway                                            │
   │     │ TraceID: abc...                                    │
   │     │ Span: 001                                          │
   │     │                                                    │
   │     ├──► Call User Service:                              │
   │     │     HTTP request                                   │
   │     │     traceparent: 00-abc...-002-01                  │
   │     │                                                    │
   │     │   User Service                                     │
   │     │     │ TraceID: abc... (same!)                      │
   │     │     │ Span: 002, Parent: 001                       │
   │     │     │                                              │
   │     │     └──► Query DB                                  │
   │     │           traceparent: 00-abc...-003-01            │
   │     │                                                    │
   │     │         DB driver instrumented                     │
   │     │           span: 003, parent: 002                   │
   │     │                                                    │
   │     └──► Call Order Service                              │
   │           traceparent: 00-abc...-004-01                  │
   │                                                          │
   │         Order Service                                    │
   │           span: 004, parent: 001                         │
   │           ├── Call Payment Service (span 005)            │
   │           └── Update DB (span 006)                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   W3C traceparent header format:
   ──────────────────────────────
   
   traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
                ─  ────────────────────────────────  ──────────────── ──
                │  TraceID (16 byte hex)            ParentSpanID    flags
                │                                   (8 byte hex)
                version
   
   
   Tự động qua libraries:
   ──────────────────────
   • axum/tower-http::TraceLayer  → extract khi nhận request
   • reqwest-tracing               → inject khi gửi request
   • opentelemetry::global propagator → manual control
```

---

## 12. Metrics — 3 types

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │   COUNTER — luôn tăng                                       │
   │   ────────────────────                                      │
   │                                                             │
   │     value ▲                                                 │
   │           │                  ┌────────                      │
   │           │             ┌────┘                              │
   │           │        ┌────┘                                   │
   │           │   ┌────┘                                        │
   │           │┌──┘                                             │
   │           └──────────────────────────►  time               │
   │                                                             │
   │   Use: counter!("requests_total").increment(1)             │
   │   Query: rate(requests_total[5m])  → req/sec               │
   │                                                             │
   ├─────────────────────────────────────────────────────────────┤
   │                                                             │
   │   GAUGE — current value (up/down)                           │
   │   ────────────────────────────────                          │
   │                                                             │
   │     value ▲                                                 │
   │           │       ╱╲                                        │
   │           │     ╱╲╱  ╲      ╱╲                              │
   │           │   ╱╲      ╲    ╱  ╲                             │
   │           │╱╲╱          ╲╱      ╲                           │
   │           └──────────────────────────►  time               │
   │                                                             │
   │   Use: gauge!("connections_active").set(42.0)              │
   │                                                             │
   ├─────────────────────────────────────────────────────────────┤
   │                                                             │
   │   HISTOGRAM — distribution                                  │
   │   ─────────────────────────                                 │
   │                                                             │
   │   bucket count ▲                                            │
   │                │      ▓▓▓                                   │
   │                │     ▓▓▓▓▓                                  │
   │                │    ▓▓▓▓▓▓▓                                 │
   │                │   ▓▓▓▓▓▓▓▓▓                                │
   │                │  ▓▓▓▓▓▓▓▓▓▓▓        ▓                      │
   │                │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓     ▓▓▓                     │
   │                └────────────────────────────►  latency      │
   │                  P50           P95            P99           │
   │                  100ms        500ms          2s             │
   │                                                             │
   │   Use: histogram!("request_duration_seconds").record(0.123) │
   │   Query: histogram_quantile(0.99, ...) → P99               │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 13. 4 Golden Signals / RED Method

```
   ┌──────────────────────────────────────────────────────────┐
   │  4 GOLDEN SIGNALS (Google SRE)                           │
   │  ────────────────────────────                            │
   │                                                          │
   │  1. TRAFFIC                                              │
   │     ───────                                              │
   │     counter!("requests_total").increment(1);             │
   │     → How busy?                                          │
   │                                                          │
   │  2. ERRORS                                               │
   │     ──────                                               │
   │     counter!("errors_total", "type" => kind).inc(1);     │
   │     → How many fail?                                     │
   │                                                          │
   │  3. LATENCY                                              │
   │     ────────                                             │
   │     histogram!("duration_seconds").record(elapsed);      │
   │     → How fast?                                          │
   │                                                          │
   │  4. SATURATION                                           │
   │     ──────────                                           │
   │     gauge!("connection_pool_used").set(used);            │
   │     gauge!("cpu_usage_ratio").set(cpu);                  │
   │     → How full?                                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  RED METHOD (simpler for web services)                   │
   │  ────────────────────────────────────                    │
   │                                                          │
   │  R - Rate     (Traffic)                                  │
   │  E - Errors   (Errors)                                   │
   │  D - Duration (Latency)                                  │
   │                                                          │
   │  Skip Saturation if not infrastructure team              │
   ├──────────────────────────────────────────────────────────┤
   │  USE METHOD (for resources)                              │
   │  ──────────────────────────                              │
   │                                                          │
   │  U - Utilization (% in use)                              │
   │  S - Saturation  (queue depth)                           │
   │  E - Errors      (faults)                                │
   │                                                          │
   │  For CPU, memory, disk, network                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. Cardinality trap

```
   ✅ LOW CARDINALITY — OK với metrics
   ──────────────────────────────────
   
   counter!("http_requests_total",
       "method" => "GET",        // 5 values
       "status" => "200",        // 10 values
       "endpoint" => "/users",   // 100 values
   ).increment(1);
   
   Total series: 5 × 10 × 100 = 5,000 series ✅
   
   
   ❌ HIGH CARDINALITY — METRICS DEATH
   ──────────────────────────────────
   
   counter!("http_requests_total",
       "user_id" => user.id.to_string(),    // 1,000,000 users!
   ).increment(1);
   
   Total series: 1,000,000+ series ❌
   → Prometheus melts
   → Memory explodes
   → Query slow / timeout
   
   
   Rule:
   ─────
   ┌──────────────────────────────────────────────┐
   │ Label = enumerable, low-cardinality          │
   │   method, status, endpoint, region, env       │
   │                                              │
   │ NOT a label = unbounded                       │
   │   user_id, request_id, IP, URL with params    │
   │                                              │
   │ High-cardinality data → LOGS or TRACES        │
   └──────────────────────────────────────────────┘
```

---

## 15. Sampling strategies

```
   ┌──────────────────────────────────────────────────────────┐
   │  HEAD SAMPLING — decide at start                         │
   │  ────────────────────────────                            │
   │                                                          │
   │  Sampler::TraceIdRatioBased(0.01)   // keep 1%           │
   │                                                          │
   │  For each request: random() < 0.01 ? keep : drop         │
   │                                                          │
   │  ✅ Cheap (skip work for dropped)                        │
   │  ❌ Might lose interesting traces                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  TAIL SAMPLING — decide at end (in collector)            │
   │  ─────────────────────────────────────────                │
   │                                                          │
   │  All traces sent to collector → buffer                   │
   │  Decision based on:                                      │
   │    • Has error? → keep                                   │
   │    • Slow (>1s)? → keep                                  │
   │    • Random 1% → keep                                    │
   │    • Otherwise → drop                                    │
   │                                                          │
   │  ✅ Keep important traces                                │
   │  ❌ Expensive (buffer all in collector)                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  ADAPTIVE SAMPLING                                       │
   │  ─────────────────                                       │
   │                                                          │
   │  Low traffic    → 100%                                   │
   │  Medium         → 10%                                    │
   │  High           → 1%                                     │
   │  Spike          → 0.1%                                   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Recommendations:
   ────────────────
   
   ┌────────────────┬────────────────────────────────┐
   │ Traffic        │ Strategy                       │
   ├────────────────┼────────────────────────────────┤
   │ < 1k req/s     │ Head sampling 100%             │
   │ 1k-10k req/s   │ Head sampling 10%              │
   │ 10k-100k req/s │ Head 1% + tail (errors 100%)   │
   │ > 100k req/s   │ Head 0.1% + tail (errors 100%) │
   └────────────────┴────────────────────────────────┘
   
   📌 ALWAYS keep error traces 100%
```

---

## 16. Production stack diagram

```
   ┌──────────────────────────────────────────────────────────┐
   │                  YOUR RUST APP                           │
   │                                                          │
   │  tracing::info!()                                        │
   │  metrics::counter!()                                     │
   │                  │                                       │
   │                  ▼                                       │
   │  ┌──────────────────────────────────────────────────┐    │
   │  │ tracing_subscriber                               │    │
   │  │   ├─ EnvFilter                                   │    │
   │  │   ├─ fmt::layer (JSON to stdout)                 │    │
   │  │   └─ opentelemetry::layer (OTLP export)          │    │
   │  └──────────────────────────────────────────────────┘    │
   │                                                          │
   │  ┌──────────────────────────────────────────────────┐    │
   │  │ metrics-exporter-prometheus                      │    │
   │  │   Serves :9000/metrics                           │    │
   │  └──────────────────────────────────────────────────┘    │
   │                                                          │
   └──┬─────────────────────────┬─────────────────────────┬───┘
      │ stdout JSON              │ OTLP gRPC               │ /metrics scrape
      ▼                          ▼                         ▼
   ┌──────────┐            ┌──────────────┐         ┌──────────────┐
   │ Vector / │            │ OpenTelemetry │         │ Prometheus   │
   │ Fluentd  │            │ Collector     │         │ (scrape ts)  │
   │ Promtail │            │               │         │              │
   └────┬─────┘            └──┬─────┬──────┘         └──────┬───────┘
        │                     │     │                       │
        ▼                     ▼     ▼                       ▼
   ┌──────────┐         ┌─────────┐ ┌──────────┐    ┌──────────────┐
   │ Loki /   │         │ Tempo / │ │ Jaeger / │    │ Prometheus / │
   │ Elastic  │         │ Honey-  │ │ Datadog  │    │ Mimir        │
   │ search   │         │ comb    │ │          │    │              │
   └────┬─────┘         └────┬────┘ └────┬─────┘    └──────┬───────┘
        │                    │           │                 │
        └────────────────────┴───────────┴─────────────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │     Grafana       │
                         │                   │
                         │  • Dashboards     │
                         │  • Alerts         │
                         │  • Correlation    │
                         └──────────────────┘
```

---

## 17. Workflow correlate signals

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  1. ALERT fires                                          │
   │     ────────────                                         │
   │     "P99 latency > 500ms" (Prometheus → Alertmanager)    │
   │              │                                           │
   │              ▼                                           │
   │  2. CHECK METRICS dashboard                              │
   │     ──────────────────────                               │
   │     Grafana: which endpoint? when?                       │
   │     Find: /api/orders, started 14:32                     │
   │              │                                           │
   │              ▼                                           │
   │  3. FIND SLOW TRACE                                      │
   │     ─────────────                                        │
   │     Grafana → Tempo: filter by duration > 1s             │
   │     OR: click "View trace" from metric panel             │
   │     Get trace_id = abc-xyz                               │
   │              │                                           │
   │              ▼                                           │
   │  4. OPEN TRACE in Jaeger/Tempo                           │
   │     ──────────────────────────                           │
   │     See call graph:                                      │
   │       /api/orders 1.2s                                   │
   │         ├── auth 50ms                                    │
   │         └── db_query 1.1s  ← bottleneck!                 │
   │              │                                           │
   │              ▼                                           │
   │  5. CHECK LOGS for that span                             │
   │     ───────────────────────                              │
   │     Loki/ES: query trace_id=abc-xyz                      │
   │     See:                                                 │
   │       "DB query: SELECT * FROM orders WHERE..."          │
   │       "Query plan changed: full scan"                    │
   │              │                                           │
   │              ▼                                           │
   │  6. ROOT CAUSE FOUND                                     │
   │     ────────────────                                     │
   │     Index dropped during migration                       │
   │     → Recreate index → resolved                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   ⟹ 3 pillars + correlation = nhanh debug từ "alert" tới "fix".
```

---

## 18. Antipatterns visualization

```
   ❌ 1. Log everything verbose in hot loop
   ────────────────────────────────────────
   
   for item in 1_000_000_items {
       info!("Processing {:?}", item);   // 1M log lines!
       process(item);
   }
   
   Result: log storage explodes, performance dies
   
   ✅ FIX: aggregate metric
   counter!("items_processed_total").increment(items.len() as u64);
   for item in items { process(item); }
   
   
   ❌ 2. Log secrets / PII
   ───────────────────────
   
   info!(user = ?user, "Login");
   //          ↑
   //   Debug includes: password_hash, ssn, credit_card, email
   //   → leaked to logs, kept forever, compliance violation
   
   ✅ FIX: redact in Debug impl, or use secrecy::Secret<T>
   
   
   ❌ 3. High-cardinality metric labels
   ────────────────────────────────────
   
   counter!("requests", "user_id" => uid).increment(1);
                                ↑
                       1M users → 1M time series
                       → Prometheus OOM
   
   ✅ FIX: user_id → log/trace, not metric
   counter!("requests", "endpoint" => endpoint).increment(1);
   
   
   ❌ 4. println! for production
   ─────────────────────────────
   
   println!("Server started");
   
   Issues: no level, no filter, no structured, no correlation
   
   ✅ FIX: tracing::info!("Server started", port = 8080);
   
   
   ❌ 5. Span every trivial fn
   ───────────────────────────
   
   #[instrument] fn add(a: i32, b: i32) -> i32 { a + b }
   //  ↑ ~100-300ns overhead per call. Wasteful for trivial fn.
   
   ✅ FIX: only instrument logical boundaries
   #[instrument] async fn handle_request(...)
   #[instrument] async fn db_query(...)
```

---

## 19. Tools matrix

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │  GOAL                       │ TOOL                          │
   │  ──────                     │ ─────                         │
   │                                                             │
   │  Library log (facade)       │ log crate                     │
   │  Structured logging         │ tracing + tracing-subscriber  │
   │  Print to stdout JSON       │ fmt::layer().json()           │
   │  Bunyan format              │ tracing-bunyan-formatter      │
   │                                                             │
   │  Filter by level/target     │ EnvFilter                     │
   │  Auto-span function         │ #[instrument]                 │
   │  Async profiling            │ tokio-console                 │
   │                                                             │
   │  Distributed tracing        │ tracing-opentelemetry + OTLP  │
   │  Local trace UI             │ Jaeger all-in-one (Docker)    │
   │  Production trace backend   │ Tempo, Honeycomb, Datadog     │
   │  Trace HTTP client          │ reqwest-tracing               │
   │                                                             │
   │  Metrics facade             │ metrics                       │
   │  Prometheus export          │ metrics-exporter-prometheus   │
   │  Statsd/InfluxDB            │ metrics-exporter-statsd/influx│
   │                                                             │
   │  Dashboards                 │ Grafana                       │
   │  Alerting                   │ Alertmanager                  │
   │  Log aggregation            │ Loki / Elasticsearch          │
   │  Log shipping               │ Vector / Promtail / Fluentd   │
   │                                                             │
   │  Secret redaction           │ secrecy crate                 │
   │  Panic capture              │ Custom panic hook             │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 20. Mind map cuối

```
                          OBSERVABILITY
                                │
        ┌─────────────┬─────────┼──────────┬─────────────┐
        ▼             ▼         ▼          ▼             ▼
      LOGS         TRACING    METRICS    TRACES      DISTRIBUTED
                              (numbers)  (causal)
        │             │         │          │             │
   log crate      tracing    metrics    OpenTelemetry  W3C context
   env_logger     subscriber Prometheus Jaeger         baggage
                  instrument exporter   Tempo          propagation
                  spans                 OTLP
                  filters
                  
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. 3 pillars: logs + metrics + traces│
                │                                      │
                │  2. Correlate via trace_id           │
                │                                      │
                │  3. tracing > log (structured + spans)│
                │                                      │
                │  4. #[instrument] với skip(self)     │
                │                                      │
                │  5. JSON production, text dev        │
                │                                      │
                │  6. 4 Golden Signals / RED method    │
                │                                      │
                │  7. Metrics labels low-cardinality   │
                │                                      │
                │  8. NEVER log secrets / PII          │
                │                                      │
                │  9. Sample aggressively (errors 100%)│
                │                                      │
                │  10. tokio-console cho async         │
                │                                      │
                │  11. OpenTelemetry standard          │
                │                                      │
                │  12. Test observability config       │
                │      như test code                   │
                └──────────────────────────────────────┘
```

---

## 21. Bộ tài liệu Rust giờ có 12 chủ đề

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
   │      observability-visual    ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 24 files, ~1.35 MB MD                            │
   │                                                          │
   │   🦀 Bộ kỹ năng production observability đầy đủ         │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

- **Unsafe Rust** — raw pointer, UnsafeCell deep, atomic ordering, FFI, soundness
- **Iterator deep dive** — implement, lazy, rayon parallel
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 12 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
