# Rust Foundations Library — Lộ trình đọc

> Bộ tài liệu deep dive 19 chủ đề Rust, từ cơ bản đến production.
> Mỗi chủ đề có 2 files: **theory** (lý thuyết) + **visual** (minh hoạ ASCII).
> Đọc song song để hiểu sâu nhanh nhất.

---

## 📚 Cách đọc

- **File đánh số letter (a, b, c...)**: thứ tự đề xuất đọc tuần tự
- **File `*.md`**: lý thuyết deep dive
- **File `*-visual.md`**: ASCII visualization companion
- **Đọc song song** 2 file của cùng chủ đề để hiểu tốt nhất

---

## 🗺️ Lộ trình 19 chương

### Phần I: Nền tảng ngôn ngữ (a-e)

| # | Chủ đề | File | Nội dung |
|---|--------|------|----------|
| **a** | Memory Model | [a-memory-model.md](./a-memory-model.md) + [visual](./a-memory-model-visual.md) | Stack/heap, cache hierarchy, virtual memory, atomics |
| **b** | Ownership & Borrowing | [b-ownership-borrowing.md](./b-ownership-borrowing.md) + [visual](./b-ownership-borrowing-visual.md) | 3 quy tắc sở hữu, NLL, Polonius, interior mutability |
| **c** | Trait | [c-trait.md](./c-trait.md) + [visual](./c-trait-visual.md) | Static/dynamic dispatch, vtable, GAT, HRTB, marker traits |
| **d** | Generic | [d-generic.md](./d-generic.md) + [visual](./d-generic-visual.md) | Type/lifetime/const generics, monomorphization, variance |
| **e** | Closure | [e-closure.md](./e-closure.md) + [visual](./e-closure-visual.md) | Capture modes, Fn/FnMut/FnOnce, anonymous struct |

### Phần II: Đồng thời & xử lý lỗi (f-h)

| # | Chủ đề | File | Nội dung |
|---|--------|------|----------|
| **f** | Async/Await | [f-async.md](./f-async.md) + [visual](./f-async-visual.md) | Future trait, state machine, Pin, Waker, executor, tokio |
| **g** | Error Handling | [g-error-handling.md](./g-error-handling.md) + [visual](./g-error-handling-visual.md) | Result, ?, thiserror, anyhow, error context, source chain |
| **h** | Macros | [h-macros.md](./h-macros.md) + [visual](./h-macros-visual.md) | macro_rules!, proc-macros (derive/attribute/function-like) |

### Phần III: Memory & Lifetime nâng cao (i-j)

| # | Chủ đề | File | Nội dung |
|---|--------|------|----------|
| **i** | Smart Pointers | [i-smart-pointers.md](./i-smart-pointers.md) + [visual](./i-smart-pointers-visual.md) | Box/Rc/Arc/Cell/RefCell/Mutex/RwLock/Weak/Cow |
| **j** | Lifetime | [j-lifetime.md](./j-lifetime.md) + [visual](./j-lifetime-visual.md) | `'a`/`'static`, elision, variance, HRTB, NLL, Polonius |

### Phần IV: Production essentials (k-o)

| # | Chủ đề | File | Nội dung |
|---|--------|------|----------|
| **k** | Performance | [k-performance.md](./k-performance.md) + [visual](./k-performance-visual.md) | criterion, perf, flamegraph, LTO, PGO, jemalloc/mimalloc |
| **l** | Observability | [l-observability.md](./l-observability.md) + [visual](./l-observability-visual.md) | tracing, OpenTelemetry, metrics, Prometheus, 4 Golden Signals |
| **m** | Iterator | [m-iterator.md](./m-iterator.md) + [visual](./m-iterator-visual.md) | Iterator trait, lazy, 70+ methods, rayon parallel, Stream |
| **n** | Unsafe Rust | [n-unsafe-rust.md](./n-unsafe-rust.md) + [visual](./n-unsafe-rust-visual.md) | Raw pointer, UnsafeCell, atomic ordering, FFI, miri |
| **o** | Testing | [o-testing.md](./o-testing.md) + [visual](./o-testing-visual.md) | Unit, integration, proptest, criterion, mockall, fuzz |

### Phần V: Ứng dụng thực tế (p-s)

| # | Chủ đề | File | Nội dung |
|---|--------|------|----------|
| **p** | Embedded Rust | [p-embedded-rust.md](./p-embedded-rust.md) + [visual](./p-embedded-rust-visual.md) | no_std, HAL/PAC, RTIC, Embassy, DMA, real-time |
| **q** | Axum Web Project | [q-axum-project.md](./q-axum-project.md) + [visual](./q-axum-project-visual.md) | Routing, extractors, middleware, error, auth, deploy |
| **r** | Database Deep Dive | [r-database.md](./r-database.md) + [visual](./r-database-visual.md) | sqlx, transactions, isolation, sharding, multi-tenancy |
| **s** | Tauri Desktop/Mobile | [s-tauri.md](./s-tauri.md) + [visual](./s-tauri-visual.md) | IPC, commands, events, capabilities, packaging, updater |
| **t** | WASM (WebAssembly) | [t-wasm.md](./t-wasm.md) + [visual](./t-wasm-visual.md) | wasm-bindgen, wasm-pack, WASI, edge computing, frameworks |

---

## 🎯 Đề xuất lộ trình đọc theo level

### Beginner (mới học Rust)
**Đọc thứ tự**: a → b → c → d → e → f → g
- Tập trung hiểu sâu mỗi chương trước khi đi tiếp
- Làm exercises trên https://doc.rust-lang.org/book/

### Intermediate (đã quen Rust cơ bản)
**Đọc thứ tự**: review (a-g) → h → i → j → m → o
- Đào sâu vào memory, lifetime, smart pointers
- Bắt đầu test code production-grade

### Advanced (sẵn sàng production)
**Đọc thứ tự**: k → l → n → q → r
- Performance, observability, unsafe
- Build production web service end-to-end

### Specialist (theo domain)
- **Embedded developer**: p (sau khi nắm a-j)
- **Web/Backend dev**: q + r (sau khi nắm a-o)
- **Desktop/Mobile dev**: s (sau khi nắm a-o)
- **Browser/Edge dev**: t (sau khi nắm a-o, đặc biệt f, n)

---

## 📊 Thống kê

- **20 chủ đề** × 2 files = **40 files**
- **~63,000 dòng** Markdown
- **~6.5 MB** tài liệu
- **~13 giờ đọc** end-to-end
- **~45-65 giờ** để thực sự thấm hiểu

---

## 🚀 Sau khi đọc xong

Bạn có khả năng build:

- 🌐 **Production web services** (axum + sqlx)
- 🗄️ **Database-heavy systems** (sharding, multi-tenancy)
- 🔌 **Embedded firmware** (no_std + embassy)
- 🖥️ **Desktop applications** (Tauri)
- 📱 **Mobile apps** (Tauri v2)
- 🌍 **Browser apps & edge compute** (WASM + Yew/Leptos/Cloudflare Workers)
- 🚀 **High-performance services** (profiled, optimized)
- 🧪 **Well-tested codebases** (unit + integration + e2e + property + fuzz)
- 🔍 **Observable production code** (3 pillars correlated)
- ⚙️ **System programming** (unsafe, FFI, atomic ordering)
- 📊 **Concurrent data processing** (rayon, tokio, channels)
- 🎮 **Game engines, OS kernels, browsers** (foundations cover all)

---

## 🦀 Mỗi chương có structure chuẩn

```
Mỗi *-theory.md có 12-16 Tầng:
  Tầng 1-2:   Nền tảng (tại sao, là gì)
  Tầng 3-6:   Core mechanics (cú pháp, basics)
  Tầng 7-10:  Advanced (deep patterns)
  Tầng 11-13: Production patterns
  Tầng 14-15: Antipatterns + Senior wisdom
  Cuối:       12 nguyên tắc senior + toolkit + crates


Mỗi *-visual.md có 18-26 phần:
  - ASCII diagrams cho mọi concept
  - Memory layout visualization
  - Flow charts cho workflows
  - Comparison tables
  - Decision trees
  - Antipatterns visualization
  - Mind map tổng kết
```

---

## 🎓 Insights cốt lõi xuyên suốt 19 chương

```
1. Memory safety không cost runtime — Rust foundation
2. Zero-cost abstraction — high-level code, low-level speed
3. Type-state pattern — compile-time guarantees
4. Ownership = single owner with borrowing rules
5. Lifetime = compile-time scope tracking
6. Async = stackless coroutine, executor-driven
7. Error path là API thật — test, log, document
8. Profile trước, optimize sau — đo, đừng đoán
9. 3 pillars observability: logs + metrics + traces
10. Test BEHAVIOR, not implementation
11. Unsafe contained — wrap trong safe API
12. Compile-time SQL safety (sqlx)
13. Tower service composition (axum middleware)
14. RTIC priority ceiling for embedded real-time
15. Embassy async embedded — multitasking trên 32KB RAM
16. Tauri: Rust trusted, WebView untrusted
17. Defense in depth security
18. Production checklist: TLS, health, metrics, secrets
19. Continuous learning — Rust evolves rapidly
```

---

## Bước tiếp theo — Áp dụng vào project thực tế

1. **Build side project** — small but production-quality, apply tất cả 19 chapters
2. **Contribute to open source** — sqlx, axum, tokio, embedded-hal, tauri
3. **Read mature Rust code** — rustc, tokio, axum, sqlx, serde
4. **Follow Rust news** — This Week in Rust, Rust blog
5. **Conferences & talks** — RustConf, EuroRust, RustFest
6. **Write & share** — blog posts, OSS libraries, help others

🦀 **Chúc bạn senior Rust journey thành công!**
