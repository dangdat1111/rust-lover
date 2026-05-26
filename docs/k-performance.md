# Performance trong Rust — Deep Dive

> Tài liệu thứ 11 trong bộ Rust nền tảng. Đọc trước:
> - [memory-model.md](./memory-model.md) — stack/heap/cache hierarchy
> - [ownership-borrowing.md](./ownership-borrowing.md) — alloc avoidance
> - [smart-pointers.md](./smart-pointers.md) — Arc/Mutex cost
> - [async.md](./async.md) — async overhead
>
> Rust nổi tiếng nhanh — nhưng "nhanh tự động" là **MYTH**. Code Rust idiomatic có thể nhanh
> như C++, nhưng cũng có thể chậm hơn Python nếu viết sai.
>
> Senior performance work = **đo trước, đoán sau**. Tài liệu này dạy bạn:
> - Cách suy nghĩ về performance (mental model)
> - Tools: criterion, perf, flamegraph, samply, heaptrack
> - Common bottlenecks và cách fix
> - Compiler optimization (LTO, PGO, codegen units)
> - Allocator alternatives
> - Antipatterns

---

# Mục lục

- [Tầng 1: Triết lý performance — Đo, đừng đoán](#tầng-1-triết-lý-performance--đo-đừng-đoán)
- [Tầng 2: Mental model — CPU, Cache, Memory](#tầng-2-mental-model--cpu-cache-memory)
- [Tầng 3: Benchmark hay Profile?](#tầng-3-benchmark-hay-profile)
- [Tầng 4: criterion — Micro-benchmark đúng cách](#tầng-4-criterion--micro-benchmark-đúng-cách)
- [Tầng 5: perf — Sampling profiler Linux](#tầng-5-perf--sampling-profiler-linux)
- [Tầng 6: Flamegraph — Visualize CPU profile](#tầng-6-flamegraph--visualize-cpu-profile)
- [Tầng 7: samply, hyperfine, và bạn bè](#tầng-7-samply-hyperfine-và-bạn-bè)
- [Tầng 8: Memory profiling — heaptrack, dhat, valgrind](#tầng-8-memory-profiling--heaptrack-dhat-valgrind)
- [Tầng 9: Compiler optimization — LTO, codegen units, PGO](#tầng-9-compiler-optimization--lto-codegen-units-pgo)
- [Tầng 10: Allocator alternatives — jemalloc, mimalloc](#tầng-10-allocator-alternatives--jemalloc-mimalloc)
- [Tầng 11: Common bottlenecks và cách fix](#tầng-11-common-bottlenecks-và-cách-fix)
- [Tầng 12: Optimization patterns — Senior techniques](#tầng-12-optimization-patterns--senior-techniques)
- [Tầng 13: Antipatterns — Premature optimization và bạn bè](#tầng-13-antipatterns--premature-optimization-và-bạn-bè)

---

# Tầng 1: Triết lý performance — Đo, đừng đoán

## 1.1 Quy tắc đầu tiên

> **"Đừng tin trực giác của mình. Đo."**

Lý do:
- CPU hiện đại pipeline, branch predict, cache hierarchy → behavior không trực quan
- Compiler optimize aggressive → code "nặng" có thể nhanh hơn code "nhẹ"
- Hardware (Intel/AMD/ARM) khác biệt
- Workload thật khác micro-bench

Senior **không bao giờ** optimize trước khi profile.

## 1.2 Quy tắc 80/20

Trong code, **80% thời gian** chạy ở **20% code**. Tối ưu 20% kia có ROI cực cao. Tối ưu phần khác → waste effort.

→ **Profile trước** để biết 20% là gì.

## 1.3 4 cấp performance work

```
Cấp 1: Algorithmic
  O(n²) → O(n log n)? Đổi data structure?
  → Biggest wins, nhưng cần hiểu bài toán
  
Cấp 2: Architectural
  Async hay sync? Batch hay stream? Cache hay re-compute?
  → Refactor lớn, big impact
  
Cấp 3: Implementation
  Vec::with_capacity, &str thay String, iter thay loop
  → Idiomatic Rust, đa số case tự tốt
  
Cấp 4: Micro-optimization
  unsafe pointer arith, SIMD, branchless code
  → Last resort, chỉ cho hot loop đã profile
```

**Quy tắc**: làm từ Cấp 1 xuống. Đừng nhảy thẳng Cấp 4.

## 1.4 Performance metrics

Đo cái gì?

| Metric | Đơn vị | Khi quan tâm |
|--------|--------|--------------|
| **Latency** (P50, P95, P99) | ms, µs | Web API, real-time |
| **Throughput** | requests/s, ops/s | Backend, batch processing |
| **CPU usage** | % cores | Server cost |
| **Memory** | RSS, peak heap | Long-running services |
| **Cache miss rate** | % | Hot loops |
| **Allocations** | count, bytes | Async/GC-aware code |

Latency vs throughput trade-off thường có:
- Batching → throughput ↑, latency ↑
- Concurrency → both ↑ đến tới limit
- Caching → latency ↓ (hit), memory ↑

## 1.5 Performance pyramid

```
                  /\
                 /  \    Algorithmic (rare changes)
                /____\
               /      \   Architectural
              /________\
             /          \  Implementation (Rust idioms)
            /____________\
           /              \  Micro-optimization
          /________________\  (rare, profile first)
         /                  \
        /     Foundation     \
       /  (correct, then fast)\
       --------------------
```

Correctness trước, sau đó performance. Bug nhanh không có giá trị.

---

# Tầng 2: Mental model — CPU, Cache, Memory

## 2.1 Hardware reality: 1 syscall = 1000+ instruction

```
   Operation                       Approx cost
   ───────────────────             ──────────────
   Register access                 < 1 ns
   L1 cache hit                    ~1 ns
   Branch (predicted)              ~1 ns
   Branch (mispredict)             ~5-10 ns
   L2 cache hit                    ~3-4 ns
   L3 cache hit                    ~10-30 ns
   Main memory (RAM)               ~100 ns
   SSD read                        ~100 µs (10000x cache!)
   Network round-trip              ~ms (1M× cache!)
   Syscall (lightweight)           ~100 ns
   Mutex lock (uncontended)        ~10-20 ns
   Mutex lock (contended)          ~1-100 µs
   Allocation (heap)               ~100 ns - 1 µs
   Thread context switch           ~1-10 µs
```

Memory access **không phải uniform**. Cache hierarchy quyết định performance nhiều hơn raw instruction count.

## 2.2 Cache line — Unit nhỏ nhất CPU đọc

CPU không đọc 1 byte. CPU đọc **cache line** = 64 byte (x86, ARM thường).

```
Memory:
[byte 0..63]   ← 1 cache line
[byte 64..127] ← 1 cache line
[byte 128..191]
...
```

Truy cập 1 byte → load nguyên 64 byte. Nếu sau đó dùng byte gần kề → cache hit (free).

→ Locality (truy cập gần nhau) cực kỳ quan trọng.

## 2.3 Branch prediction

CPU pipeline 10-20 stage. Khi gặp `if`, không thể chờ — phải đoán nhánh và speculative execute.

```rust
for x in data {
    if x > 0 {          // predictable nếu data đa số > 0
        process(x);
    }
}
```

- Predictable branch: ~1ns
- Mispredict: ~5-10ns (pipeline flush)

Branchless code (vd `data.iter().filter(...).sum()`) đôi khi nhanh hơn `if` rõ ràng vì compiler vectorize được.

## 2.4 SIMD và vectorization

CPU x86 có SSE/AVX, ARM có NEON — instructions xử lý 4/8/16 element cùng lúc.

```rust
let v: Vec<i32> = (0..1_000_000).collect();
let sum: i64 = v.iter().map(|&x| x as i64).sum();
// Compiler có thể vectorize → 8x faster
```

Compiler auto-vectorize khi:
- Loop simple (no early exit)
- Data contiguous
- Type primitive

`#[cfg(target_feature = "avx2")]` để explicit SIMD instructions.

## 2.5 False sharing — Cache line ping-pong

```rust
struct Counters {
    a: AtomicU64,
    b: AtomicU64,
}
// a và b cùng cache line!
```

Thread 1 update `a`, thread 2 update `b` → cache line "bounce" giữa cores → 10-100x slower.

Fix: padding:
```rust
#[repr(align(64))]
struct Counter { val: AtomicU64 }

struct Counters {
    a: Counter,  // 64-byte aligned, riêng cache line
    b: Counter,
}
```

## 2.6 Heap allocation cost

```rust
let v: Vec<i32> = Vec::new();
for i in 0..1000 {
    v.push(i);  // realloc khi capacity hết → memcpy + alloc
}
```

Mỗi `push` triggering realloc → copy toàn bộ data sang block mới. Vec dùng growth factor 2x → ~log2(N) reallocs.

Fix:
```rust
let mut v: Vec<i32> = Vec::with_capacity(1000);
for i in 0..1000 { v.push(i); }  // không realloc
```

`with_capacity` cực kỳ quan trọng cho hot path.

## 2.7 Indirection cost

```rust
// 1 level indirection
struct Direct { data: [i32; 100] }  // contiguous

// 2 levels
struct Indirect { data: Vec<i32> }  // pointer → heap

// 3 levels
struct VeryIndirect { data: Box<Vec<Box<i32>>> }  // 3 hops
```

Mỗi level pointer = cache miss potential. Prefer direct/contiguous data.

---

# Tầng 3: Benchmark hay Profile?

## 3.1 Khác biệt

```
   BENCHMARK                        PROFILE
   ─────────                        ───────
   Đo TỐC ĐỘ                       Tìm BOTTLENECK
   "Code này nhanh đến đâu?"       "Phần nào tốn nhất?"
   
   Cố định input                    Workload thật
   Loop nhiều lần                  Run thường
   Statistical analysis             Sampling/instrumentation
   
   criterion, hyperfine             perf, flamegraph,
                                    samply, dhat
```

## 3.2 Khi nào benchmark?

- So sánh 2 implementations
- Track regression (CI)
- Measure improvement after optimization
- Hot path / library API

## 3.3 Khi nào profile?

- "App chậm" — không biết tại sao
- Tìm hot function
- Memory leak / excessive alloc
- Lock contention

## 3.4 Workflow của senior

```
   1. Profile app trong workload thật
                  ↓
   2. Identify hot path (top 20% time)
                  ↓
   3. Benchmark hot path (criterion)
                  ↓
   4. Optimize, benchmark lại
                  ↓
   5. Profile lại — đã giảm chưa?
                  ↓
   6. Lặp lại với hot path mới
```

Đừng skip step 1. Đừng optimize không có baseline.

---

# Tầng 4: criterion — Micro-benchmark đúng cách

## 4.1 Tại sao không dùng `#[bench]` built-in?

Rust có `#[bench]` nhưng:
- Yêu cầu nightly
- Statistical analysis nghèo nàn
- Không track regression
- Không noise filtering

→ Dùng `criterion` (community standard).

## 4.2 Setup criterion

```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_bench"
harness = false
```

```rust
// benches/my_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn bench_fib(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| fib(black_box(20)))
    });
}

criterion_group!(benches, bench_fib);
criterion_main!(benches);
```

Run:
```bash
cargo bench
```

Output:
```
fib 20    time:   [21.234 µs 21.342 µs 21.467 µs]
          change: [-0.5% +0.2% +1.1%] (p = 0.65 > 0.05)
          No change in performance detected.
```

## 4.3 `black_box` — Ngăn compiler optimize đi

```rust
b.iter(|| fib(20))           // ❌ compiler có thể constant fold = 6765
b.iter(|| fib(black_box(20)))  // ✅ ngăn optimize away
```

`black_box(x)` nói "compiler đừng giả định x = constant" — buộc compute thật.

Cũng dùng với output:
```rust
b.iter(|| {
    let r = expensive_op(20);
    black_box(r);  // ngăn compiler "this result is unused, skip"
})
```

## 4.4 Benchmark với input thay đổi

```rust
fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let v: Vec<i32> = (0..size).rev().collect();
            b.iter_batched(
                || v.clone(),                   // setup (không count)
                |mut v| v.sort(),               // measure this
                BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}
```

`iter_batched`: setup outside measurement. Đo chỉ phần thực sự muốn.

## 4.5 Compare implementations

```rust
fn bench_two(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_concat");
    
    group.bench_function("format!", |b| {
        b.iter(|| format!("{}{}", black_box("hello"), black_box("world")))
    });
    
    group.bench_function("String::push_str", |b| {
        b.iter(|| {
            let mut s = String::with_capacity(10);
            s.push_str("hello");
            s.push_str("world");
            s
        })
    });
    
    group.finish();
}
```

Cùng group → HTML report compare side-by-side.

## 4.6 HTML report

```bash
cargo bench
# → target/criterion/report/index.html
```

Report có:
- Time histogram
- Throughput chart
- Comparison với previous run
- Outliers detection

Mở file để analyze trực quan.

## 4.7 Tracking regression trong CI

```bash
# Baseline:
cargo bench -- --save-baseline main

# After change:
cargo bench -- --baseline main
# → criterion báo "% change" so với baseline
```

Setup CI: chạy bench mỗi PR, fail nếu regression > threshold.

## 4.8 Bài học kinh điển khi viết bench

### Lỗi 1: Benchmark trivial code

```rust
b.iter(|| 2 + 3);  // compiler: constant, return 5
```

Bench đo overhead `criterion`, không phải code. Dùng `black_box`.

### Lỗi 2: Bench inside debug build

```bash
cargo bench      # OK — auto release mode
cargo build      # ⚠️ debug mode, KHÔNG bench
```

`cargo bench` mặc định release. `cargo run --release` cho profile.

### Lỗi 3: System noise

Bench laptop với:
- Spotify chạy nền → noise
- CPU throttle vì nóng → variance
- Browser eat RAM → swap

Production: dedicated CI runner, disable turboboost, set CPU governor performance.

### Lỗi 4: Bench too short

```rust
b.iter(|| simple_op());  // 1ns op
```

Criterion auto repeat đủ để có statistical signal. Nhưng nếu op quá nhỏ, noise > signal.

→ `iter_batched` với batch size = nhiều thousand.

## 4.9 Criterion analysis: outliers

```
fib 20    time:   [21.2 µs 21.3 µs 21.5 µs]
Found 10 outliers among 100 measurements (10.00%)
  4 (4.00%) high mild
  6 (6.00%) high severe
```

Outliers nhiều → system noise. Run lại trên môi trường ổn định.

---

# Tầng 5: perf — Sampling profiler Linux

## 5.1 perf là gì?

`perf` = Linux profiler builtin. **Sampling**: lấy mẫu PC (program counter) ~1000 lần/giây → biết function nào đang chạy.

Lợi:
- Low overhead (~1-2%)
- Không cần modify code
- System-wide hoặc per-process
- Hardware counter (cache miss, branches)

Nhược:
- Linux only (macOS dùng `Instruments` hoặc `samply`)
- Cần debug symbols để readable
- Output dạng text raw — cần tool để visualize

## 5.2 Setup symbols

```toml
# Cargo.toml — bật debug info trong release
[profile.release]
debug = true
```

Hoặc `[profile.release-with-debug]` profile riêng:

```toml
[profile.release-with-debug]
inherits = "release"
debug = true
```

Build:
```bash
cargo build --profile release-with-debug
```

Không có debug info → perf hiện hex address, không readable.

## 5.3 Record + report

```bash
# 1. Record (start your app, profile, stop)
sudo perf record --call-graph=dwarf -F 997 ./target/release/myapp

# 2. Report (interactive)
sudo perf report
```

Output:
```
   23.45%  myapp  myapp  [.] my_app::parse_json
   18.32%  myapp  libc.so [.] memcpy
   12.10%  myapp  myapp  [.] my_app::hash_data
   ...
```

Top function = 23.45% time → optimize đây.

## 5.4 Tùy chọn quan trọng

```bash
perf record --call-graph=dwarf   # Stack trace dùng DWARF info
                                  # alternatives: fp (frame pointer, fast but inaccurate), lbr (Intel Last Branch Record)
perf record -F 997               # Sample 997 Hz (odd để avoid aliasing)
perf record -g                   # Enable call graph
perf record -p <PID>             # Attach to running process
perf record --pid <PID> -- sleep 30  # Profile 30s
```

## 5.5 Hardware counters

`perf stat` show hardware counters:

```bash
perf stat ./target/release/myapp

 Performance counter stats:
       1,234,567,890   instructions
         456,789,012   cycles
              0.270    IPC (instructions per cycle)
              12,345   cache-misses
              23,456   cache-references
              52.69%   miss rate
              98,765   branch-misses
```

Insights:
- **IPC < 1**: CPU stall (cache miss, mispredict, dependency)
- **High cache-miss**: data locality kém → restructure
- **Branch-miss**: unpredictable branches → branchless

```bash
perf stat -e cycles,instructions,cache-misses,branch-misses ./myapp
```

## 5.6 Top - Real-time

```bash
sudo perf top
# Hiển thị live top function consuming CPU
```

Như `top` nhưng theo function, không process.

## 5.7 Phép thuật `perf script` + flamegraph

```bash
perf record --call-graph=dwarf ./myapp
perf script | inferno-collapse-perf | inferno-flamegraph > flame.svg
```

Sẽ giải thích chi tiết Tầng 6.

## 5.8 Permission

`perf` cần permission. 2 cách:

```bash
# 1. Run as root (đơn giản):
sudo perf record ...

# 2. Set kernel.perf_event_paranoid = 0 (cho user):
sudo sysctl -w kernel.perf_event_paranoid=0
# Sau đó perf chạy không cần sudo
```

Lưu ý: production thường ko cho perf access — phải debug local hoặc staging.

---

# Tầng 6: Flamegraph — Visualize CPU profile

## 6.1 Flamegraph là gì?

**Flamegraph** = visualization SVG của profile data. Mỗi function = 1 box; box rộng = nhiều thời gian; box lồng = call stack.

```
   width = % CPU time
   ─────────────────────────
   
   [────────────main────────────────────]   ← top
       [───parse_request────] [──response──]
         [─tokenize─][─lex─]   [─serialize─]
              [─slow─]              [─io─]
```

Đọc:
- **Trục X**: % CPU time (KHÔNG phải time axis, là tổng % tại mỗi function)
- **Trục Y**: call stack depth
- **Top of stack**: function đang execute thực sự
- **Width**: function tốn nhiều CPU
- **Color**: thường random (chỉ cosmetic)

## 6.2 Tại sao flamegraph powerful?

- Visual: thấy ngay bottleneck (box rộng nhất)
- Hierarchical: thấy ai gọi ai
- Click vào box để zoom in
- SVG → open trong browser, share dễ

## 6.3 cargo-flamegraph

Cài:
```bash
cargo install flamegraph
```

Yêu cầu (Linux):
```bash
# Ubuntu/Debian
sudo apt install linux-perf

# Permission cho perf
sudo sysctl -w kernel.perf_event_paranoid=0
sudo sysctl -w kernel.kptr_restrict=0
```

Cargo.toml:
```toml
[profile.release]
debug = true
```

Run:
```bash
cargo flamegraph --bin myapp -- arg1 arg2
# → flamegraph.svg
```

Mở SVG trong browser:
```bash
firefox flamegraph.svg
```

## 6.4 Cách đọc flamegraph

```
       ┌────────────────────────────────────────┐
       │ main                              50%  │  ← rộng = nhiều
       └─────────────┬──────────────────────────┘
       ┌──────────┐ ┌──────────┐ ┌───────────┐
       │ parse 20%│ │ work 30%│ │ output 5% │   ← cùng level = sibling
       └──────────┘ └──────────┘ └───────────┘
                       │
                  ┌─────────────────┐
                  │ inner_loop 25%  │  ← bottleneck!
                  └─────────────────┘
                       │
                  ┌─────────────────┐
                  │ alloc 20%       │
                  └─────────────────┘
```

→ `inner_loop` calling `alloc` đang ăn 20% CPU. Tối ưu alloc → big win.

## 6.5 Common patterns trên flamegraph

### Pattern 1: Tall narrow stack
```
┌──┐
│a │
├──┤
│b │
├──┤
│c │
├──┤
│d │
└──┘
```
Deep recursion / many function calls. Có thể inline được không?

### Pattern 2: Wide plateau
```
┌──────────────────────┐
│      hot_function    │  ← rộng
└──────────────────────┘
```
1 function ăn nhiều CPU. Focus tối ưu đây.

### Pattern 3: Multiple peaks
```
┌────┐  ┌────┐  ┌────┐
│ a  │  │ b  │  │ c  │
└────┘  └────┘  └────┘
```
CPU spread đều nhiều function. Đã optimized, hoặc bottleneck ở memory/IO không hiện ở CPU.

### Pattern 4: Allocator dominates
```
┌──────────────────────┐
│ malloc / free / ...  │  ← 30%+ alloc?
└──────────────────────┘
```
→ Cần giảm allocation (pool, with_capacity, reuse Vec).

## 6.6 Flamegraph cho async code

Async code có call stack ngắn (executor → poll → user code). Flamegraph không thấy được logical flow.

Solutions:
- `tokio-console` — runtime-level tasks view
- `tracing-flame` — instrumentation-based flamegraph
- `samply` — newer profiler với better async support

## 6.7 Inverted (icicle) flamegraph

```bash
cargo flamegraph --reverse
```

Đảo ngược: top = caller, bottom = callee. Đôi khi dễ thấy "function nhỏ được gọi nhiều" pattern.

---

# Tầng 7: samply, hyperfine, và bạn bè

## 7.1 samply — Profiler hiện đại

```bash
cargo install samply
samply record ./target/release/myapp
# → mở firefox profiler UI tự động
```

Lợi:
- Cross-platform (macOS, Linux, Windows)
- UI Firefox Profiler (rất tốt cho async)
- Không cần `perf` permission
- Inline view, source view

Đang dần thay thế `cargo-flamegraph` cho nhiều người.

## 7.2 hyperfine — Benchmark CLI

```bash
cargo install hyperfine

# Compare 2 binaries:
hyperfine './app_v1 input.txt' './app_v2 input.txt'

# Warmup runs:
hyperfine --warmup 3 './app input.txt'

# Multiple commands:
hyperfine 'sort file' 'sort -n file' 'sort -u file'
```

Output:
```
Benchmark 1: ./app_v1 input.txt
  Time (mean ± σ):     123.4 ms ±  2.1 ms
  
Benchmark 2: ./app_v2 input.txt
  Time (mean ± σ):      98.7 ms ±  1.8 ms

Summary
  './app_v2 input.txt' ran 1.25 ± 0.03 times faster than './app_v1 input.txt'
```

Tốt hơn `time`: statistical analysis, comparison, warmup.

## 7.3 cargo-bloat — Binary size analyzer

```bash
cargo install cargo-bloat
cargo bloat --release
cargo bloat --release --crates
```

Output:
```
File  .text    Size       Crate Name
1.5%   8.2%   142.7KiB    [Unknown]      __libc_csu_init
1.2%   6.5%   112.4KiB    serde_json     ...
...
```

Hiểu function/crate nào tốn binary size. Dùng để giảm binary cho embedded, distribute.

## 7.4 cargo-asm — Xem assembly output

```bash
cargo install cargo-show-asm
cargo asm myapp::my_function
```

Xem compiler sinh assembly gì. Cho người tò mò:
- Compiler đã vectorize chưa?
- Inline đã work chưa?
- Branchless thật chưa?

## 7.5 tokio-console — Async task profiler

```bash
cargo install tokio-console

# Trong app:
console_subscriber::init();
```

Connect:
```bash
tokio-console
```

UI realtime:
- Tasks alive
- Polled count, busy time
- Idle/waiting tasks
- Resource usage

Cho async app, **bắt buộc**. CPU profiler không thấy được task-level info.

## 7.6 cargo-llvm-cov — Coverage

```bash
cargo install cargo-llvm-cov
cargo llvm-cov
cargo llvm-cov --html  # HTML report
```

Code coverage % với LLVM source-based coverage. Tốt hơn line-based.

---

# Tầng 8: Memory profiling — heaptrack, dhat, valgrind

## 8.1 Tại sao memory profile?

CPU profile không thấy:
- Memory leak (RSS grow over time)
- Excessive allocations
- Large allocations đột ngột
- Fragmentation

→ Cần memory profiler riêng.

## 8.2 heaptrack — Best on Linux

```bash
sudo apt install heaptrack heaptrack-gui

heaptrack ./target/release/myapp
# → heaptrack.myapp.12345.gz

heaptrack_gui heaptrack.myapp.12345.gz
```

GUI rất tốt:
- Top allocators (function alloc nhiều nhất)
- Memory usage over time
- Leak detection
- Histogram of allocation sizes

## 8.3 dhat (Rust crate)

Dhat là profiler from Valgrind, có Rust binding:

```toml
[dependencies]
dhat = "0.3"

[features]
dhat-heap = []
```

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    // your code
}
```

Run:
```bash
cargo run --release --features dhat-heap
# → dhat-heap.json
```

Upload to https://nnethercote.github.io/dh_view/dh_view.html.

Insight: peak heap, allocation count, byte total.

## 8.4 valgrind massif

Slowest nhưng accurate:
```bash
valgrind --tool=massif ./target/release/myapp
ms_print massif.out.<pid>
```

Output text-based heap usage timeline. Slow (~10-20x overhead) — chỉ dùng cho short test.

## 8.5 Counting allocations

Quick check số allocation trong code:

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAlloc;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn main() {
    let before = ALLOCS.load(Ordering::Relaxed);
    expensive_op();
    let after = ALLOCS.load(Ordering::Relaxed);
    println!("Allocations: {}", after - before);
}
```

Diagnostic nhanh — không cần tool ngoài.

## 8.6 Memory leak detection

Long-running Rust services có thể leak nếu:
- Cycle Rc/Arc (không phá bằng Weak)
- Global static collection (lưu mãi)
- `Box::leak` lặp
- C library leak qua FFI

Tools:
- `valgrind --leak-check=full` (slow)
- `heaptrack` GUI có "leaked" tab
- `LSan` (LeakSanitizer): `RUSTFLAGS="-Z sanitizer=leak" cargo build` (nightly)

---

# Tầng 9: Compiler optimization — LTO, codegen units, PGO

## 9.1 Cargo profiles

```toml
[profile.dev]
opt-level = 0      # No optimization
debug = true
overflow-checks = true

[profile.release]
opt-level = 3      # Max optimization
debug = false
lto = false
codegen-units = 16
overflow-checks = false
```

Mặc định `cargo build` = dev, `cargo build --release` = release.

## 9.2 opt-level

```toml
opt-level = 0   # Không optimize (debug)
opt-level = 1   # Basic
opt-level = 2   # Tốt cho size + speed
opt-level = 3   # Max speed (release default)
opt-level = "s" # Optimize for size
opt-level = "z" # Optimize for size aggressive (no loop vectorize)
```

`opt-level = 3` mặc định cho release. Embedded/WASM thường dùng `"s"` hoặc `"z"`.

## 9.3 LTO — Link Time Optimization

LTO cho phép compiler optimize **xuyên crate boundary**:
- Inline function từ crate khác
- Dead code elimination
- Constant propagation

```toml
[profile.release]
lto = false        # Default: no LTO
lto = "thin"       # Fast LTO, gần tốt như fat
lto = "fat"        # Best optimization, slow compile
lto = true         # = "fat"
lto = "off"        # = false
```

Trade-off:
- `lto = "thin"`: +20-30% compile time, +5-15% perf, **recommended cho production**
- `lto = "fat"`: +2-5x compile time, +5-20% perf, dùng cho final release

## 9.4 codegen-units

```toml
[profile.release]
codegen-units = 16   # Default — parallel compile
codegen-units = 1    # Sequential — better optimization, slower compile
```

Nhiều codegen units = parallel compile fast, nhưng giới hạn cross-unit optimization.

Production release thường:
```toml
[profile.release]
lto = "fat"
codegen-units = 1
```

→ Tối đa optimization, không quan tâm compile time.

## 9.5 PGO — Profile Guided Optimization

PGO: chạy app trên realistic workload → record profile → recompile dùng profile để guide optimization.

```bash
# Step 1: Build with instrumentation
RUSTFLAGS="-Cprofile-generate=/tmp/pgo" cargo build --release

# Step 2: Run app on realistic workload
./target/release/myapp typical_input.txt

# Step 3: Merge profiles
llvm-profdata merge -o /tmp/pgo/merged.profdata /tmp/pgo

# Step 4: Build with PGO
RUSTFLAGS="-Cprofile-use=/tmp/pgo/merged.profdata" cargo build --release
```

Wins: +5-15% performance trên CPU-bound code. Worth trying cho production binary.

## 9.6 Cargo profile for production

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
debug = false
panic = "abort"      # Skip unwinding, smaller binary, slightly faster
strip = true         # Strip symbols
overflow-checks = false
```

`panic = "abort"` → process abort khi panic thay vì unwinding. Skip generate unwind tables → smaller binary, slightly faster.

⚠️ Code unit test cần unwinding để catch panic. Test với `panic = "unwind"`.

## 9.7 Target-specific optimization

```bash
# Compile cho CPU hiện tại (max optimization):
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Vs portable binary (default):
cargo build --release   # works on any x86_64
```

`target-cpu=native` cho phép compiler dùng latest instructions (AVX2, AVX-512) → +10-30% perf trên CPU mới. Nhưng binary không chạy được trên CPU cũ.

Production deploy: thường dùng `target-cpu=x86-64-v3` (Haswell+, ~95% CPUs sau 2014).

## 9.8 inline hints

```rust
#[inline]           // Suggest inline (compiler decide)
#[inline(always)]   // Force inline (use sparingly)
#[inline(never)]    // Block inline
```

Compiler thường tự inline tốt. Manual hint chỉ cho hot path đặc biệt — overuse `inline(always)` có thể làm binary phình.

---

# Tầng 10: Allocator alternatives — jemalloc, mimalloc

## 10.1 Default Rust allocator

Rust mặc định dùng **system allocator**:
- Linux: `malloc`/`free` từ glibc
- macOS: jemalloc (legacy version)
- Windows: HeapAlloc

Glibc malloc OK nhưng không phải fastest. Nhiều ngôn ngữ/runtime dùng custom allocator.

## 10.2 jemalloc

```toml
[dependencies]
jemallocator = "0.5"
```

```rust
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

jemalloc:
- Designed for multi-thread heavy workload
- Better fragmentation handling
- Used by Redis, Firefox, Facebook services
- +5-30% perf cho multi-threaded server

## 10.3 mimalloc

```toml
[dependencies]
mimalloc = "0.1"
```

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

Microsoft's allocator. Thường được cho là **nhanh nhất** trong micro-benchmarks. Try this khi need max perf.

## 10.4 So sánh thực tế

```
Workload: web server với nhiều concurrent requests
─────────────────────────────────────────────────

glibc malloc:    baseline
jemalloc:        -10 to -20% latency, +15% throughput  
mimalloc:        -15 to -25% latency, +20% throughput
```

**Đo bằng workload thật, không micro-bench**.

## 10.5 Allocator-aware code

```rust
// Reduce alloc count:
let mut v = Vec::with_capacity(expected_size);   // Pre-allocate
let mut s = String::with_capacity(expected_len);

// Reuse buffer:
let mut buf = Vec::new();
for item in items {
    buf.clear();   // Reuse capacity
    process_into(&mut buf, item);
}

// Pool pattern:
let pool = ObjectPool::new();
let obj = pool.acquire();
// ... use obj
// obj returned automatically on drop
```

Giảm alloc count còn quan trọng hơn đổi allocator.

## 10.6 Arena allocator — bumpalo

Cho workload alloc nhiều nhỏ + drop cùng lúc:

```rust
use bumpalo::Bump;

let arena = Bump::new();
let s1 = arena.alloc("hello");      // alloc trong arena
let s2 = arena.alloc(vec![1, 2, 3]); // không free riêng lẻ
drop(arena);   // free tất cả 1 phát
```

Use case: parser, AST, request processing (per-request arena).

---

# Tầng 11: Common bottlenecks và cách fix

## 11.1 ❌ Allocations trong hot loop

```rust
for line in lines {
    let parts: Vec<&str> = line.split(',').collect();  // alloc Vec mỗi iter!
    process(&parts);
}
```

Fix:
```rust
let mut parts: Vec<&str> = Vec::with_capacity(10);
for line in lines {
    parts.clear();
    parts.extend(line.split(','));   // reuse Vec
    process(&parts);
}

// Hoặc lazy iterator:
for line in lines {
    for part in line.split(',') {     // không alloc Vec
        process_part(part);
    }
}
```

## 11.2 ❌ String concatenation với `+`

```rust
let mut s = String::new();
for i in 0..1000 {
    s = s + &format!("{},", i);   // mỗi vòng alloc String mới!
}
```

Fix:
```rust
let mut s = String::with_capacity(5000);
for i in 0..1000 {
    use std::fmt::Write;
    write!(s, "{},", i).unwrap();   // append in-place
}
```

## 11.3 ❌ Clone không cần thiết

```rust
let big_string = String::from("...giant data...");
process(big_string.clone());   // clone big_string!
process(big_string.clone());
```

Fix:
```rust
process(&big_string);   // borrow
process(&big_string);
```

Or move ownership to last call:
```rust
process(&big_string);
process(big_string);   // last call moves
```

## 11.4 ❌ Box<dyn Trait> trong hot loop

```rust
for _ in 0..1_000_000 {
    let h: Box<dyn Handler> = Box::new(MyHandler);  // heap alloc!
    h.handle();
}
```

Fix: use generic / static dispatch:
```rust
fn process<H: Handler>(h: H) {
    for _ in 0..1_000_000 { h.handle(); }
}
```

## 11.5 ❌ HashMap thay BTreeMap với small data

```rust
let m: HashMap<i32, i32> = (0..10).map(|i| (i, i*2)).collect();
```

HashMap có constant overhead (alloc + hash). Cho N < ~16, `BTreeMap` hoặc thậm chí `Vec<(K, V)> + linear search` nhanh hơn (cache-friendly).

## 11.6 ❌ Unnecessary indirection

```rust
struct Bad {
    data: Box<Vec<Box<i32>>>,   // 3 levels indirect
}
```

Mỗi access = 3 cache miss. Fix:
```rust
struct Good {
    data: Vec<i32>,   // 1 alloc, contiguous
}
```

## 11.7 ❌ Mutex contention

```rust
let counter = Arc::new(Mutex::new(0));
for _ in 0..100 {
    let c = Arc::clone(&counter);
    thread::spawn(move || {
        for _ in 0..1_000_000 {
            *c.lock().unwrap() += 1;   // contention!
        }
    });
}
```

Fix: AtomicUsize:
```rust
let counter = Arc::new(AtomicUsize::new(0));
// Atomic fetch_add, no lock
counter.fetch_add(1, Ordering::Relaxed);
```

Or shard:
```rust
let counters: Vec<AtomicUsize> = (0..NUM_THREADS).map(|_| AtomicUsize::new(0)).collect();
// Each thread updates its own counter
// Sum at end
```

## 11.8 ❌ Sync I/O in async

```rust
async fn handler() {
    let data = std::fs::read_to_string("big.txt").unwrap();   // blocks!
    process(&data).await;
}
```

Fix:
```rust
async fn handler() {
    let data = tokio::fs::read_to_string("big.txt").await.unwrap();
    process(&data).await;
}
```

## 11.9 ❌ Vec<Vec<T>> khi T size cố định

```rust
let matrix: Vec<Vec<f64>> = vec![vec![0.0; 100]; 100];  // nested vec
```

Cache-unfriendly. Mỗi inner Vec là alloc riêng. Fix:
```rust
let matrix: Vec<f64> = vec![0.0; 100 * 100];
// access matrix[i * 100 + j]

// Or use ndarray crate:
let matrix = ndarray::Array2::<f64>::zeros((100, 100));
```

## 11.10 ❌ Đọc string char-by-char

```rust
for c in s.chars() {
    if c == ' ' { count += 1; }
}
```

`chars()` decode UTF-8 mỗi byte. Cho ASCII:
```rust
for &b in s.as_bytes() {
    if b == b' ' { count += 1; }
}
```

10x nhanh hơn cho big string.

---

# Tầng 12: Optimization patterns — Senior techniques

## 12.1 Pattern: Cache locality

```rust
// ❌ Stride access
for i in 0..1000 {
    for j in 0..1000 {
        sum += matrix[j][i];   // jump cache lines
    }
}

// ✅ Sequential
for i in 0..1000 {
    for j in 0..1000 {
        sum += matrix[i][j];   // contiguous
    }
}
```

Hoán đổi loop order → 5-10x speedup.

## 12.2 Pattern: Batch operations

```rust
// ❌ One per call
for item in items {
    db.insert(item).await?;
}

// ✅ Bulk insert
db.insert_batch(&items).await?;
```

Mỗi DB call có overhead network + parse. Batch giảm overhead.

## 12.3 Pattern: SIMD với portable_simd (nightly) hoặc crate

```rust
// Manual loop
let sum: f32 = vec.iter().sum();

// With wide crate (auto-vectorize hint):
use std::simd::*;
let chunks = vec.array_chunks::<8>();
let sum = chunks.fold(f32x8::splat(0.0), |acc, c| acc + f32x8::from_array(*c));
let total: f32 = sum.reduce_sum();
```

8x parallel per CPU op. Cho float math, big speedup.

## 12.4 Pattern: Memoization

```rust
let mut cache = HashMap::new();
fn fib(n: u64, cache: &mut HashMap<u64, u64>) -> u64 {
    if let Some(&v) = cache.get(&n) { return v; }
    let r = if n < 2 { n } else { fib(n-1, cache) + fib(n-2, cache) };
    cache.insert(n, r);
    r
}
```

Recursive với big subtree repeat → memoize. From O(2^n) → O(n).

## 12.5 Pattern: Lazy evaluation

```rust
fn expensive_status() -> String { /* expensive */ }

// ❌ Always compute
log::info!("Status: {}", expensive_status());

// ✅ Skip if not logged:
log::info!("Status: {}", lazy(|| expensive_status()));
```

`format!` macro lazy by default — nhưng nếu pass `f(x)` thì f đã eval. Wrap trong closure để delay.

## 12.6 Pattern: Inline data + small_vec

```rust
use smallvec::SmallVec;

// Stack-allocate up to 8 items, fallback to heap:
let mut v: SmallVec<[i32; 8]> = SmallVec::new();
v.push(1); v.push(2);   // stack
v.extend(0..20);         // grow to heap
```

`SmallVec` zero-alloc cho common case (small data). Used in `rustc`, `serde`.

## 12.7 Pattern: Custom hasher

```rust
use std::collections::HashMap;
use ahash::AHasher;
use std::hash::BuildHasherDefault;

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;
```

Default `HashMap` dùng `SipHash` (DoS-resistant). For non-public-facing data, `ahash`/`fxhash` 2-5x nhanh hơn.

`HashMap<String, String>` -> `FxHashMap<String, String>` typical 30-50% improvement.

## 12.8 Pattern: Const generics for small fixed array

```rust
fn process_n<const N: usize>(arr: [u8; N]) {
    // compiler có thể fully unroll loop
}

process_n::<16>([0; 16]);
```

Generic over size → compiler specialize per size → unroll, vectorize.

## 12.9 Pattern: Profile-guided thuần API

```rust
// Branch hint:
if hot_path_likely {
    fast_case();
} else {
    cold_case();
}
```

In nightly: `core::intrinsics::likely(cond)`. Trên stable, compiler thường tự đoán đúng.

## 12.10 Pattern: Compile time work

```rust
const TABLE: [u32; 256] = generate_table();   // const fn at compile time
```

const fn evaluate tại compile → no runtime cost. Move work to compile when possible.

---

# Tầng 13: Antipatterns — Premature optimization và bạn bè

## 13.1 ❌ Premature optimization

> "Premature optimization is the root of all evil" — Donald Knuth

Đừng:
- Optimize trước khi có working code
- Optimize trước khi profile
- Optimize phần không nóng

Hệ quả: code khó đọc, không nhanh hơn, có khi chậm hơn.

## 13.2 ❌ Tin micro-benchmark naively

Micro-benchmark có cô lập rất khác workload thật:
- Cache "warmed up" 100% (real: cold cache)
- No system noise
- Same data
- Predictable branches

Real workload có thể behave hoàn toàn khác. Always **profile real app** before/after.

## 13.3 ❌ Optimize "thấy đẹp" mà không đo

```rust
// "for loop chậm, dùng iter chain mới idiomatic và nhanh"
data.iter().map(...).filter(...).sum()

// vs
let mut sum = 0;
for x in data {
    if cond(x) { sum += transform(x); }
}
```

Idiomatic không tự động nhanh hơn. Đo cụ thể trên workload.

## 13.4 ❌ unsafe để "tăng tốc"

```rust
unsafe { *ptr = value; }   // bypass bounds check
```

Modern Rust compile bounds check thường được optimized away. `unsafe` chỉ wins khi:
- Profile rõ ràng cho thấy bounds check là bottleneck
- Có thể prove safety

Bug từ `unsafe` = UB, debug ngàn lần đắt hơn 1% performance gain.

## 13.5 ❌ Quá nhiều `#[inline(always)]`

```rust
#[inline(always)]
fn helper() { /* ... */ }
```

Forced inline → binary phình → instruction cache miss → có thể CHẬM hơn.

Để compiler decide unless có evidence.

## 13.6 ❌ Tự implement Mutex/Channel

```rust
struct MyMutex { /* tự design */ }
```

99% trường hợp `std::sync::Mutex` hoặc `parking_lot::Mutex` đã optimize tốt. Tự viết → bug + chậm.

Exception: thực sự cần fine-tune cho hot path đã profile + có lock-free expertise.

## 13.7 ❌ Optimize before fixing memory layout

```rust
struct Bad {
    a: u8,        // 1 byte
    b: u64,       // 8 byte (alignment → 7 byte padding before b)
    c: u8,        // 1 byte
    d: u64,       // 8 byte (more padding)
}
// size = 32 byte do padding!
```

Fix:
```rust
struct Good {
    b: u64,
    d: u64,
    a: u8,
    c: u8,
    // size = 24 byte
}
```

Compiler reorder field từ Rust 1.x — nhưng `#[repr(C)]` thì không. Aware về layout cho hot struct.

## 13.8 ❌ Pre-allocate quá lớn

```rust
let mut v = Vec::with_capacity(1_000_000);   // alloc 4MB
for x in 0..10 { v.push(x); }   // chỉ dùng 40 byte!
```

Reserve over-estimate → waste memory + alloc cost. Predict thật.

## 13.9 ❌ JSON serialize/deserialize trong hot path

JSON parse expensive — chuỗi → DOM-like → struct. Khi hot:
- Cache parsed result
- Dùng binary format (bincode, MessagePack)
- Hoặc parse lazy (don't decode unused fields)

## 13.10 ❌ println! debug trong hot loop

```rust
for x in big_data {
    println!("processing {:?}", x);   // I/O syscall + format → SLOW
    process(x);
}
```

Log có cost. Use:
- `log::trace!` / `tracing::trace!` with level filter
- Conditional compile: `#[cfg(debug_assertions)]`
- Sample logging (every Nth iteration)

---

# Tổng kết — 12 nguyên tắc senior

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Profile trước, optimize sau. Đừng đoán.                      │
│                                                                 │
│ 2. 80/20 rule — focus on hot 20% code path.                     │
│                                                                 │
│ 3. Algorithmic > Architectural > Implementation > Micro-opt.    │
│    Đi từ cấp trên xuống.                                        │
│                                                                 │
│ 4. Benchmark micro với criterion + black_box.                   │
│                                                                 │
│ 5. Profile prod-like workload với perf/samply.                  │
│                                                                 │
│ 6. Flamegraph để visualize bottleneck.                          │
│                                                                 │
│ 7. Memory profile riêng (heaptrack, dhat).                      │
│                                                                 │
│ 8. Production release: lto="fat", codegen-units=1.              │
│                                                                 │
│ 9. Cân nhắc jemalloc/mimalloc cho threaded workloads.           │
│                                                                 │
│ 10. Reduce allocations trước khi đổi allocator.                 │
│                                                                 │
│ 11. Cache locality matter. Layout > algorithm đôi khi.          │
│                                                                 │
│ 12. Async cần tokio-console — perf không thấy task-level.       │
└─────────────────────────────────────────────────────────────────┘
```

---

# Workflow tổng hợp cho 1 performance task

```
1. Define metric & target
   ────────────────────────
   "Reduce P99 latency from 500ms to 100ms"
   "Increase throughput from 1k to 10k req/s"
   
2. Establish baseline
   ───────────────────
   hyperfine ./app
   tokio-console
   prometheus metrics
   
3. Profile
   ────────
   cargo flamegraph --bin myapp
   perf record + perf report
   heaptrack ./myapp
   
4. Identify hot path
   ─────────────────
   Top 3 functions trong flamegraph
   Memory peak source
   Lock contention hotspots
   
5. Hypothesis + experiment
   ───────────────────────
   "Reducing alloc here gives 30%"
   → cargo bench specific hot fn
   
6. Implement + verify
   ──────────────────
   criterion compare
   Profile again → bottleneck moved?
   
7. Repeat
   ──────
   New bottleneck appears
   Until target met
```

---

# Liên kết về memory model

Performance ↔ memory model:

| Performance concept | Memory layer involved |
|---------------------|----------------------|
| Cache hit/miss | L1/L2/L3 cache hierarchy |
| Branch prediction | CPU pipeline |
| SIMD | Vector registers |
| False sharing | Cache line coherence protocol |
| Allocation cost | Heap, OS pages, brk/mmap syscall |
| Mutex lock | Atomic ops + maybe futex syscall |
| Async overhead | State machine size, polling |
| Page fault | OS virtual memory |

→ Performance Rust **là** câu chuyện về CPU + memory hierarchy.

---

# Crates và tools (Senior toolkit)

| Tool | Mục đích |
|------|----------|
| `criterion` | Micro-benchmark with stats |
| `hyperfine` | CLI benchmark / compare |
| `perf` | Linux sampling profiler |
| `flamegraph` / `cargo-flamegraph` | CPU profile visualization |
| `samply` | Cross-platform profiler with Firefox UI |
| `tokio-console` | Async task profiler |
| `heaptrack` | Memory profiler (Linux) |
| `dhat` | Heap usage analysis |
| `cargo-bloat` | Binary size analyzer |
| `cargo-asm` / `cargo-show-asm` | View assembly |
| `cargo-llvm-cov` | Code coverage |
| `jemallocator` / `mimalloc` | Alternative allocators |
| `bumpalo` | Arena allocator |
| `smallvec` | Inline-stack vec |
| `ahash` / `fxhash` | Fast hasher |
| `rayon` | Data parallelism |

---

# Lộ trình tiếp theo

Bạn đã có 11 chủ đề:

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
11. performance        ← MỚI
```

Còn các topic chuyên sâu:

- **Unsafe Rust** — raw pointer, UnsafeCell, atomic ordering, FFI, soundness
- **Iterator deep dive** — implement Iterator, lazy, parallel với rayon
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Logging & Observability** — tracing nâng cao, OpenTelemetry, structured logs
- **Web framework realistic** — axum project apply 11 chủ đề
- **Database** — sqlx, sea-orm, transaction, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo chủ đề tiếp theo! 🦀⚡
