# Performance Rust — Minh Hoạ Trực Quan

> Companion visual cho [performance.md](./performance.md). Đọc song song.

---

## 1. Bức tranh lớn — Performance Universe

```
                       PERFORMANCE TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   "ĐO TRƯỚC, ĐOÁN SAU"                                 │
       │                                                        │
       │   ┌─────────────┐    ┌──────────────┐    ┌──────────┐  │
       │   │ BENCHMARK   │    │ PROFILE      │    │ OPTIMIZE │  │
       │   │ ─────────   │    │ ───────      │    │ ─────────│  │
       │   │ Đo tốc độ   │    │ Tìm bottleneck│   │ Apply fix│  │
       │   │             │    │              │    │          │  │
       │   │ criterion   │    │ perf         │    │ algorithm│  │
       │   │ hyperfine   │    │ flamegraph   │    │ refactor │  │
       │   │             │    │ samply       │    │ micro-op │  │
       │   └─────────────┘    │ heaptrack    │    └──────────┘  │
       │                      │ tokio-console │                  │
       │                      └──────────────┘                   │
       │                                                        │
       │   ┌────────────────────────────────────────────────┐   │
       │   │              4 CẤP performance work            │   │
       │   │                                                │   │
       │   │   Cấp 1: Algorithmic    (biggest win)          │   │
       │   │   Cấp 2: Architectural                          │   │
       │   │   Cấp 3: Implementation                         │   │
       │   │   Cấp 4: Micro-opt      (last resort)          │   │
       │   └────────────────────────────────────────────────┘   │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Performance Pyramid

```
                      /\
                     /  \    Algorithmic
                    /    \   O(n²) → O(n log n)
                   /______\  Đổi data structure
                  /        \
                 /          \  Architectural
                /            \  Async/Batch/Cache decisions
               /______________\
              /                \  Implementation
             /                  \  Rust idioms, with_capacity,
            /                    \  iter chains, &str over String
           /______________________\
          /                        \
         /        Micro-opt          \  unsafe, SIMD, branchless
        /  (rare, profile first)      \  Only for hot loops
       /________________________________\
      
      ┌──────────────────────────────────────┐
      │ CORRECTNESS                          │
      │ Foundation: code đúng trước, nhanh sau│
      └──────────────────────────────────────┘
   
   📌 Đi từ trên xuống. Đừng nhảy thẳng micro-opt.
```

---

## 3. Hardware Reality — Cost hierarchy

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  Operation                  Cost          Ratio to L1        │
   │  ──────────────────         ──────        ──────────         │
   │                                                              │
   │  Register access            <1 ns         <1x                │
   │  L1 cache hit               ~1 ns         1x                 │
   │  Branch (predicted)         ~1 ns         1x                 │
   │  Branch (mispredict)        ~5-10 ns      5-10x              │
   │  L2 cache hit               ~3-4 ns       4x                 │
   │  L3 cache hit               ~10-30 ns     30x                │
   │  Main memory (RAM)          ~100 ns       100x               │
   │  Allocation (heap)          ~100ns-1µs    100-1000x          │
   │  Mutex (uncontended)        ~10-20 ns     20x                │
   │  Mutex (contended)          ~1-100 µs     1000-100000x       │
   │  Syscall (light)            ~100 ns       100x               │
   │  Thread context switch      ~1-10 µs      1000-10000x        │
   │  SSD read                   ~100 µs       100000x            │
   │  Network RTT (LAN)          ~ms           1000000x           │
   │  Network RTT (Internet)     ~10-100 ms    10-100M x          │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   📌 Memory != uniform. Cache hierarchy quan trọng hơn raw ops.
```

---

## 4. Cache Line visualization

```
   CPU đọc CACHE LINE (64 byte trên x86), không phải 1 byte
   ───────────────────────────────────────────────────────
   
   Memory:
   ┌──────────────────────────────────────────────────────────┐
   │ Cache line 0 (byte 0..63)                                │
   ├──────────────────────────────────────────────────────────┤
   │ Cache line 1 (byte 64..127)                              │
   ├──────────────────────────────────────────────────────────┤
   │ Cache line 2 (byte 128..191)                             │
   └──────────────────────────────────────────────────────────┘
   
   Access byte 5: ──► load nguyên cache line 0 vào L1
   Access byte 10: ─► L1 hit (free)
   Access byte 70: ─► load cache line 1 (cache miss)
   
   
   ✅ TỐT — Sequential access (cache-friendly):
   ───────────────────────────────────────────
   
   Vec<i32> data = [0,1,2,3,4,...,1000];
   
   for x in &data {
       process(x);   // sequential → 1 cache line cover 16 i32
   }
   // Cache miss rate: ~6% (only when crossing lines)
   
   
   ❌ XẤU — Random access (cache-unfriendly):
   ─────────────────────────────────────────
   
   for i in random_indices {
       process(&data[i]);   // jump around → cache miss mỗi lần
   }
   // Cache miss rate: ~100% — 30-100x slower!
```

---

## 5. False Sharing — Cache line ping-pong

```
   ❌ FALSE SHARING:
   
   #[repr(C)]
   struct Counters {
       a: AtomicU64,   // offset 0
       b: AtomicU64,   // offset 8
   }
   // Tổng size = 16 byte → CÙNG cache line 64-byte
   
   
   Visualization:
   ──────────────
   
   Cache line 0:
   ┌──────────────────────────────────────────────────┐
   │ [ a: AtomicU64 ][ b: AtomicU64 ][ ... padding ]  │
   └──────────────────────────────────────────────────┘
        ▲                  ▲
        │                  │
        │                  └─── Thread 2 writes b
        └─── Thread 1 writes a
        
   Mỗi write → CPU invalidate cache line on OTHER core
   → "ping-pong" giữa cores → 10-100x slower!
   
   
   ✅ FIX — Pad to cache line:
   ──────────────────────────
   
   #[repr(align(64))]
   struct PaddedCounter { val: AtomicU64 }
   
   struct Counters {
       a: PaddedCounter,    // riêng cache line
       b: PaddedCounter,    // riêng cache line  
   }
   
   Cache line 0:                Cache line 1:
   ┌─────────────────────┐     ┌─────────────────────┐
   │ a (8 byte) + padding│     │ b (8 byte) + padding│
   └─────────────────────┘     └─────────────────────┘
        ▲                            ▲
        │                            │
   Thread 1 only             Thread 2 only
   
   No contention. Each core có local copy.
```

---

## 6. Benchmark vs Profile — Khác biệt

```
   ┌─────────────────────────────┬─────────────────────────────┐
   │ BENCHMARK                   │ PROFILE                     │
   ├─────────────────────────────┼─────────────────────────────┤
   │ "Code này nhanh đến đâu?"   │ "Phần nào tốn nhất?"        │
   │                             │                             │
   │ Cố định input               │ Workload thật               │
   │ Loop nhiều lần              │ Run thường                  │
   │ Statistical analysis        │ Sampling / instrumentation  │
   │                             │                             │
   │ Tool:                       │ Tool:                       │
   │  • criterion                │  • perf                     │
   │  • hyperfine                │  • flamegraph               │
   │                             │  • samply                   │
   │                             │  • heaptrack                │
   │                             │  • tokio-console            │
   │                             │                             │
   │ When:                       │ When:                       │
   │  • Compare 2 impl           │  • "App chậm" — chưa biết   │
   │  • Track regression CI      │  • Tìm hot function         │
   │  • Measure improvement       │  • Memory leak             │
   │  • Hot path / lib API       │  • Lock contention          │
   └─────────────────────────────┴─────────────────────────────┘
   
   
   Workflow senior:
   ────────────────
   
   1. PROFILE app (workload thật) ─► tìm bottleneck
                  ↓
   2. BENCHMARK function hot ─► measure baseline
                  ↓
   3. OPTIMIZE
                  ↓
   4. BENCHMARK lại ─► improved chưa?
                  ↓
   5. PROFILE lại ─► bottleneck moved?
                  ↓
   6. REPEAT
```

---

## 7. criterion — Cấu trúc bench

```
   ┌──────────────────────────────────────────────────────────┐
   │ // benches/my_bench.rs                                   │
   │                                                          │
   │ use criterion::{black_box, criterion_group,              │
   │                 criterion_main, Criterion};              │
   │                                                          │
   │ fn fib(n: u64) -> u64 {                                  │
   │     if n < 2 { n } else { fib(n-1) + fib(n-2) }          │
   │ }                                                        │
   │                                                          │
   │ fn bench_fib(c: &mut Criterion) {                        │
   │     c.bench_function("fib 20", |b| {                     │
   │         b.iter(|| fib(black_box(20)))                    │
   │     });                                                  │
   │ }                                                        │
   │                                                          │
   │ criterion_group!(benches, bench_fib);                    │
   │ criterion_main!(benches);                                │
   └──────────────────────────────────────────────────────────┘
                              │
                              ▼  cargo bench
   ┌──────────────────────────────────────────────────────────┐
   │ Run output:                                              │
   │                                                          │
   │ fib 20    time:   [21.234 µs 21.342 µs 21.467 µs]        │
   │           change: [-0.5% +0.2% +1.1%]                    │
   │           Found 5 outliers among 100 measurements        │
   │                                                          │
   │ Statistical analysis:                                    │
   │ • mean ± std                                             │
   │ • outlier detection                                      │
   │ • regression comparison                                  │
   │                                                          │
   │ → HTML report: target/criterion/report/index.html       │
   └──────────────────────────────────────────────────────────┘
   
   
   black_box() — RẤT QUAN TRỌNG:
   ──────────────────────────────
   
   ❌ Without black_box:
   b.iter(|| fib(20))
   //         ↑
   //   Compiler: "constant input, constant output → return 6765"
   //   → KHÔNG đo gì cả!
   
   ✅ With black_box:
   b.iter(|| fib(black_box(20)))
   //              ↑
   //   "Hide value from optimizer"
   //   → force compute thật
```

---

## 8. perf — Sampling profiler

```
   ┌─────────────────────────────────────────────────────────┐
   │                  perf workflow                          │
   │                                                         │
   │   1. Build với debug info:                              │
   │      [profile.release]                                  │
   │      debug = true                                       │
   │                                                         │
   │      cargo build --release                              │
   │                                                         │
   │                       │                                 │
   │                       ▼                                 │
   │   2. Record:                                            │
   │      sudo perf record --call-graph=dwarf -F 997 ./app   │
   │                                                         │
   │                       │ Sample 997 Hz                   │
   │                       │ Capture stack trace             │
   │                       ▼                                 │
   │   3. Report:                                            │
   │      sudo perf report                                   │
   │                                                         │
   │      Output:                                            │
   │      ┌─────────────────────────────────────┐            │
   │      │ 23.45%  myapp  [.] parse_json       │  ← top!   │
   │      │ 18.32%  myapp  [.] hash_data        │            │
   │      │ 12.10%  libc   [.] memcpy           │            │
   │      └─────────────────────────────────────┘            │
   │                                                         │
   └─────────────────────────────────────────────────────────┘
   
   
   Sampling vs instrumentation:
   ─────────────────────────────
   
   ┌────────────────────┬──────────────────────┐
   │ SAMPLING (perf)    │ INSTRUMENTATION      │
   ├────────────────────┼──────────────────────┤
   │ Ngắt CPU N Hz      │ Insert tracing code  │
   │ Capture PC + stack │ Mỗi function entry   │
   │                    │ / exit               │
   │ Low overhead (~1%) │ Higher overhead      │
   │ Statistical        │ Exact counts         │
   │ Miss short fns     │ Slow short fns       │
   │                    │                      │
   │ Best for: hot path │ Best for: count fns  │
   └────────────────────┴──────────────────────┘
   
   
   Hardware counters (perf stat):
   ──────────────────────────────
   
   perf stat ./app
   
   Performance counter stats:
        1,234,567,890   instructions
          456,789,012   cycles
                0.27   IPC          ← < 1 = CPU stall!
               12,345   cache-misses ← high = data locality bad
               23,456   cache-references
              52.69%    miss rate    ← needs locality fix
               98,765   branch-misses
```

---

## 9. Flamegraph — Đọc và phân tích

```
   ┌────────────────────────────────────────────────────────────┐
   │                  FLAMEGRAPH                                │
   │                                                            │
   │  Trục Y: call stack depth                                  │
   │  Trục X: % CPU time (KHÔNG phải timeline)                  │
   │  Width: function tốn nhiều CPU                             │
   │  Top of stack: function actually executing                 │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
   
   
   Example flamegraph:
   ──────────────────
   
        ┌──────┐
        │alloc │  ← 20% (top hot leaf)
        ├──────┴────┐
        │ vec_grow  │  ← 25%
        ├───────────┴────┐
        │  parse_inner   │  ← 30%
        ├────────────────┴────┐
        │   parse_request     │  ← 50%
        ├─────────────────────┴────┐
        │      handle_request      │  ← 60%
   ┌────┴─────────────┐    ┌──────┴────────────┐
   │    main          │    │   compute (30%)   │
   │   (60%)          │    │                   │
   └──────────────────┘    └───────────────────┘
   ────────────────────────────────────────────►
                       % CPU time
   
   
   Đọc:
   ─────
   • main → handle_request → parse_request → parse_inner → vec_grow → alloc
   • Box rộng nhất = ăn nhiều CPU nhất
   • Top stack box = function THỰC SỰ chạy (alloc ở đây)
   • Mỗi bước xuống = caller
   
   Insight: vec_grow → alloc đang ăn ~20% → tối ưu alloc.
   
   
   Common patterns:
   ────────────────
   
   ┌──┐
   │a │       ┌──────────────────────┐    ┌────┐ ┌────┐ ┌────┐
   ├──┤       │   hot_function       │    │ a  │ │ b  │ │ c  │
   │b │       └──────────────────────┘    └────┘ └────┘ └────┘
   ├──┤
   │c │       Wide plateau                 Multiple peaks
   ├──┤       1 function dominates         Spread out
   │d │       → focus tối ưu               → maybe memory/IO bound
   └──┘
   Tall narrow
   Deep recursion
   → inline?
   
   
   Allocator pattern (RED FLAG):
   ──────────────────────────────
   
   ┌────────────────────────────────────────┐
   │   malloc / free / __rust_realloc       │  ← 30%+ alloc?
   └────────────────────────────────────────┘
   
   → Cần giảm allocations:
     • Vec::with_capacity
     • Reuse buffers
     • Avoid clone
     • Object pool
```

---

## 10. Memory profiling — heaptrack

```
   ┌──────────────────────────────────────────────────────────┐
   │   heaptrack workflow                                     │
   │                                                          │
   │   heaptrack ./target/release/myapp                       │
   │            │                                             │
   │            ▼ (records all malloc/free)                   │
   │   heaptrack.myapp.12345.gz                               │
   │            │                                             │
   │            ▼                                             │
   │   heaptrack_gui heaptrack.myapp.12345.gz                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   GUI tabs:
   ─────────
   
   ┌────────────────────────────────────────────────────┐
   │ Summary    │ Top of memory usage:                  │
   │ Flamegraph │  ─────────────────                    │
   │ Top-Down   │   45 MB  parse_json::Parser::new       │
   │ Bottom-Up  │   23 MB  HashMap::resize              │
   │ Allocations│   12 MB  String::with_capacity        │
   │ Sizes      │   ...                                 │
   │ Caller     │                                       │
   │ Leaks ✓    │ Allocations count:                    │
   │            │  ─────────────────                    │
   │            │   1,234,567  small (<256 byte)        │
   │            │   45,678     medium                   │
   │            │   123        large                    │
   │            │                                       │
   │            │ Peak: 67 MB at line 234               │
   └────────────────────────────────────────────────────┘
   
   
   Memory usage over time:
   ────────────────────────
   
   MB   ▲
   100  │           ┌─────┐
        │      ┌────┘     └──┐
    50  │  ┌───┘             └──── leak? growing?
        │ ┌┘                            │
     0  │┘                              │
        └────────────────────────────► time
                                        ↑
                            Leak: grows over time, not freed
                            Spike: temporary peak (parse?)
```

---

## 11. Compiler optimization — Cargo profiles

```
   ┌──────────────────────────────────────────────────────────┐
   │                  Cargo.toml profiles                     │
   │                                                          │
   │   [profile.dev]                                          │
   │   opt-level = 0       ← no opt, fast compile             │
   │   debug = true                                           │
   │   ────────                                               │
   │                                                          │
   │   [profile.release]                                      │
   │   opt-level = 3       ← max opt                          │
   │   debug = false                                          │
   │   lto = false         ← default — no LTO                 │
   │   codegen-units = 16  ← parallel compile                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Production-grade release:
   ─────────────────────────
   
   ┌──────────────────────────────────────────────────────────┐
   │   [profile.release]                                      │
   │   opt-level = 3                                          │
   │   lto = "fat"          ← +5-20% perf, slow compile      │
   │   codegen-units = 1    ← single unit = max optimization  │
   │   debug = false                                          │
   │   panic = "abort"      ← smaller binary                  │
   │   strip = true         ← strip symbols                   │
   └──────────────────────────────────────────────────────────┘
   
   
   Compile time vs Performance:
   ────────────────────────────
   
   Profile          Compile time    Run time
   ──────           ─────────────   ────────
   dev              ████ fast        ░░░░ slow
   release default  ██████          █████
   release LTO      ███████████      ██████ +5-15%
   release LTO=fat  ██████████████   ███████ +10-20%
                    + codegen=1
   PGO              ████████████████ ████████ +5-15% more
   
   📌 Mỗi level: trade compile time để gain run time
```

---

## 12. LTO comparison

```
   No LTO:
   ───────
                                    
   crate A          crate B          crate C
   ┌──────┐         ┌──────┐         ┌──────┐
   │ fn_a │────────▶│ fn_b │────────▶│ fn_c │
   └──────┘         └──────┘         └──────┘
   
   Mỗi crate compile riêng → optimize chỉ trong crate
   Function call cross-crate KHÔNG inline được
   
   
   Thin LTO:
   ─────────
   
   ┌────────────────────────────────────────┐
   │  ALL crates linked together            │
   │  ┌──────┐  ┌──────┐  ┌──────┐          │
   │  │ fn_a │──│ fn_b │──│ fn_c │          │
   │  └──────┘  └──────┘  └──────┘          │
   │      ▲                                 │
   │      │ optimizer thấy all              │
   │      │ inline cross-crate function     │
   └────────────────────────────────────────┘
   
   +5-15% perf, +20-30% compile time
   
   
   Fat LTO:
   ────────
   
   Tương tự thin nhưng aggressive hơn:
   - Inline cross-crate aggressively
   - More dead code elimination
   - +10-20% perf, +200-500% compile time
   
   📌 Thin LTO recommended cho production, Fat LTO cho final release
```

---

## 13. PGO — Profile-Guided Optimization

```
   ┌─────────────────────────────────────────────────────────┐
   │ Step 1: Build với instrumentation                       │
   │ ─────────────────────────────────                       │
   │                                                         │
   │ RUSTFLAGS="-Cprofile-generate=/tmp/pgo" \              │
   │   cargo build --release                                 │
   │                                                         │
   │           │                                             │
   │           ▼                                             │
   │ Step 2: Run trên realistic workload                     │
   │ ────────────────────────────────────                    │
   │                                                         │
   │ ./target/release/myapp typical_input.txt                │
   │                                                         │
   │           │                                             │
   │           ▼ Records:                                    │
   │           • Branch direction taken                      │
   │           • Function call frequency                     │
   │           • Hot/cold paths                              │
   │                                                         │
   │ Step 3: Merge profiles                                  │
   │ ────────────────────                                    │
   │                                                         │
   │ llvm-profdata merge -o /tmp/pgo/merged.profdata \      │
   │   /tmp/pgo                                              │
   │                                                         │
   │           │                                             │
   │           ▼                                             │
   │ Step 4: Build PGO-optimized                             │
   │ ──────────────────────────                              │
   │                                                         │
   │ RUSTFLAGS="-Cprofile-use=/tmp/pgo/merged.profdata" \   │
   │   cargo build --release                                 │
   │                                                         │
   │   → Compiler optimize cho actual usage pattern          │
   │   → +5-15% perf trên CPU-bound code                     │
   └─────────────────────────────────────────────────────────┘
   
   
   Benefits visualization:
   ───────────────────────
   
   Without PGO:
   ────────────
   if condition {        ← compiler guess: 50/50
       hot_branch();
   } else {
       cold_branch();
   }
   
   With PGO (after profile shows true 95% of time):
   ──────────────────────────────────────────────
   compiler arranges code so hot_branch is fall-through
   cold_branch in separate code section
   → better instruction cache utilization
   → branch predictor warm with right info
```

---

## 14. Allocator comparison

```
   ┌──────────────────────────────────────────────────────────┐
   │  Multi-threaded server workload                          │
   │                                                          │
   │   Allocator    Latency    Throughput   Notes             │
   │   ─────────    ───────    ──────────   ─────             │
   │   glibc malloc  baseline   baseline    Default Linux    │
   │   jemalloc      -10-20%    +15%        Less fragment    │
   │   mimalloc      -15-25%    +20%        Often fastest    │
   │   bumpalo       —          —           Arena, special   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Sử dụng:
   ────────
   
   ┌────────────────────────────────────────────────────┐
   │ [dependencies]                                     │
   │ mimalloc = "0.1"                                   │
   │                                                    │
   │ // main.rs                                         │
   │ #[global_allocator]                                │
   │ static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;│
   │                                                    │
   │ fn main() {                                        │
   │     // mọi Box, Vec, String giờ dùng mimalloc      │
   │ }                                                  │
   └────────────────────────────────────────────────────┘
   
   
   Cách hoạt động:
   ───────────────
   
   App code             Global allocator           Underlying
   ───────              ────────────────           ─────────
   
   Box::new(x)  ────►   __rust_alloc()     ────►   sys_brk / mmap
   Vec::push    ────►   GlobalAlloc::alloc ────►   kernel
   ...                                              
                                                    
   Khi swap mimalloc:                              
                        mimalloc::MiMalloc         
                        ── manage internal pools   
                        ── reduce syscalls         
                        ── thread-local caches     
```

---

## 15. Common bottlenecks visualization

```
   ❌ 1. Alloc trong hot loop
   ──────────────────────────
   
   for line in lines {
       let parts: Vec<&str> = line.split(',').collect();   ← alloc!
       process(&parts);
   }
   
   ⟹ Flamegraph: __rust_alloc / Vec::reserve top of stack
   
   ✅ FIX:
   ────────
   let mut parts: Vec<&str> = Vec::with_capacity(10);
   for line in lines {
       parts.clear();                          ← reuse
       parts.extend(line.split(','));
       process(&parts);
   }
   
   
   ❌ 2. Clone không cần thiết
   ───────────────────────────
   
   let s = String::from("...big data...");
   process(s.clone());   ← clone big_string!
   process(s.clone());
   process(s);            ← OK chỉ ở cuối
   
   ✅ FIX: borrow
   ──────────────
   process(&s);
   process(&s);
   process(s);
   
   
   ❌ 3. Cache-unfriendly access pattern
   ─────────────────────────────────────
   
   // matrix[row][col] với row-major storage
   for i in 0..1000 {
       for j in 0..1000 {
           sum += matrix[j][i];   ← jump cache lines
       }
   }
   
   ✅ FIX: swap loops
   ──────────────────
   for i in 0..1000 {
       for j in 0..1000 {
           sum += matrix[i][j];   ← sequential
       }
   }
   → 5-10x faster
   
   
   ❌ 4. Vec<Vec<T>> nested
   ────────────────────────
   
   let matrix: Vec<Vec<f64>> = ...;   ← each inner Vec separate alloc
   
   ✅ FIX: flat array
   ──────────────────
   let matrix: Vec<f64> = vec![0.0; W * H];
   matrix[i * W + j]
   
   
   ❌ 5. Mutex contention
   ──────────────────────
   
   let counter = Arc::new(Mutex::new(0));
   // 100 threads contending → 1-100µs per lock
   
   ✅ FIX: atomic
   ──────────────
   let counter = Arc::new(AtomicUsize::new(0));
   counter.fetch_add(1, Ordering::Relaxed);   ← ~5-10ns, no lock
```

---

## 16. Optimization patterns visualization

```
   ✅ Pattern 1: Cache locality
   ────────────────────────────
   
   Sequential good        Random bad
   ────────────────       ────────────
   ┌─┬─┬─┬─┬─┬─┐         ┌─┬─┬─┬─┬─┬─┐
   │1│2│3│4│5│6│         │1│ │ │ │ │ │
   └─┴─┴─┴─┴─┴─┘         └─┴─┴─┴─┴─┴─┘
   prefetched              ▲   ▲   ▲
                          jumps random
   
   
   ✅ Pattern 2: SmallVec — inline small data
   ──────────────────────────────────────────
   
   Normal Vec:           SmallVec<[T; 8]>:
   ┌─────────┐           ┌─────────────────┐
   │ Vec     │           │ SmallVec        │
   │  ptr───┐│           │  inline: [_;8]  │ ← stack
   │  len   ││           │   OR             │
   │  cap   ││           │  ptr/len/cap    │ ← heap (overflow)
   └────────┼┘           └─────────────────┘
            ▼ heap
   ┌──────────┐
   │ T,T,T,...│
   └──────────┘
   
   Pattern small case (<8): no alloc
   
   
   ✅ Pattern 3: Object pool
   ─────────────────────────
   
   Without pool:           With pool:
   ──────────────         ──────────
   for req in requests {  let pool = ObjectPool::new();
       let buf = Vec::new(); for req in requests {
       // alloc!              let buf = pool.acquire();
       process(&mut buf);     // reuse!
       drop(buf);             process(&mut buf);
       // free!                drop(buf); // return to pool
   }                       }
   
   N allocs                 1 alloc (per pool size)
   N frees                  0 frees
   
   
   ✅ Pattern 4: Arena allocator
   ─────────────────────────────
   
   use bumpalo::Bump;
   let arena = Bump::new();
   
   for _ in 0..1000 {
       let s = arena.alloc(MyStruct { ... });   ← bump pointer
       process(s);
   }
   drop(arena);   ← free TẤT CẢ 1 phát
   
   ┌────────────────────────────────────────────┐
   │ Bump pointer alloc:                        │
   │                                            │
   │  arena memory: [used][used][used][free...] │
   │                                ▲           │
   │                          bump pointer      │
   │                                            │
   │  alloc: just bump pointer (very fast!)     │
   │  no individual free                        │
   └────────────────────────────────────────────┘
```

---

## 17. Workflow tổng hợp

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │   1. DEFINE METRIC & TARGET                                 │
   │      "Reduce P99 latency 500ms → 100ms"                     │
   │                              │                              │
   │                              ▼                              │
   │   2. ESTABLISH BASELINE                                     │
   │      hyperfine ./app                                        │
   │      Prometheus metrics                                     │
   │                              │                              │
   │                              ▼                              │
   │   3. PROFILE                                                │
   │      cargo flamegraph --bin myapp                           │
   │      perf record + perf report                              │
   │      heaptrack ./myapp                                      │
   │                              │                              │
   │                              ▼                              │
   │   4. IDENTIFY HOT PATH                                      │
   │      Top 3 fns trong flamegraph                             │
   │      Allocator > 10%?                                       │
   │      Lock contention?                                       │
   │                              │                              │
   │                              ▼                              │
   │   5. HYPOTHESIZE                                            │
   │      "Reducing alloc here saves 30%"                        │
   │                              │                              │
   │                              ▼                              │
   │   6. BENCHMARK (criterion)                                  │
   │      Verify hypothesis on isolated fn                       │
   │                              │                              │
   │                              ▼                              │
   │   7. IMPLEMENT FIX                                          │
   │                              │                              │
   │                              ▼                              │
   │   8. VERIFY                                                 │
   │      • criterion compare baseline                           │
   │      • Profile lại → bottleneck moved?                      │
   │                              │                              │
   │                              ▼                              │
   │   9. REPEAT (new bottleneck appears)                        │
   │      Until target met                                       │
   │                                                             │
   │   📌 Target met ≠ done — set new target, continue           │
   └─────────────────────────────────────────────────────────────┘
```

---

## 18. Antipatterns visualization

```
   ❌ 1. Premature optimization
   ────────────────────────────
   
   "Tôi sẽ unsafe ngay từ đầu để nhanh"
        │
        ▼
   Code chưa work → unsafe bug
        │
        ▼
   Debug ngàn lần đắt
   
   ✅ Working code → measure → bottleneck → optimize
   
   
   ❌ 2. Micro-bench naive
   ───────────────────────
   
   bench cô lập:
   ┌──────────────┐
   │ fn micro     │
   │  fully warmed│  ← 50ns
   │  cache       │
   └──────────────┘
        │
        ▼
   "Wow! Nó rất nhanh, deploy!"
        │
        ▼
   Real workload:
   ┌──────────────────┐
   │ Cold cache       │
   │ Memory pressure  │
   │ Lock contention  │
   │ → 5000ns         │ ← 100x slower!
   └──────────────────┘
   
   ✅ Profile real workload trước khi tin micro-bench
   
   
   ❌ 3. unsafe để "tăng tốc"
   ──────────────────────────
   
   unsafe { *ptr.offset(idx) }   ← bypass bounds check
        │
        ▼
   Modern compiler thường optimize bounds check away
   anyway → unsafe doesn't help much
        │
        ▼
   Bug từ unsafe = Undefined Behavior
   → Debug rất khó, security risk
   
   ✅ Profile first. unsafe only if necessary AND can prove safety.
   
   
   ❌ 4. Quá nhiều #[inline(always)]
   ─────────────────────────────────
   
   #[inline(always)]
   fn helper() { /* 100 lines */ }
   
   Caller 1 ─┐
   Caller 2 ─┼─► Inline body 100 lines × 100 callers
   ...      ─┤
   Caller 100┘ → Binary phình → ICache miss → SLOWER
   
   ✅ Let compiler decide (#[inline] hint, not always)
   
   
   ❌ 5. println! / log trong hot loop
   ───────────────────────────────────
   
   for x in big_data {
       println!("processing {:?}", x);    ← syscall + format
       process(x);                         ← actual work
   }
   
   Profile:
   ┌──────────────────────────────────────┐
   │  println / format!  80% time         │
   ├──────────────────────────────────────┤
   │  process(x)         20% time         │  ← optimizing here? No.
   └──────────────────────────────────────┘
   
   ✅ Remove logs from hot path. Use tracing với level filter.
```

---

## 19. Tools matrix

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │  GOAL                  │ TOOL                                │
   │  ────                  │ ────                                │
   │                                                             │
   │  Micro-benchmark       │ criterion                           │
   │  CLI compare/time      │ hyperfine                           │
   │                                                             │
   │  CPU profile (Linux)   │ perf record + flamegraph            │
   │  CPU profile (cross)   │ samply                              │
   │  Visual hot path       │ cargo-flamegraph                    │
   │                                                             │
   │  Memory profile        │ heaptrack (Linux)                   │
   │  Heap usage analysis   │ dhat                                │
   │  Memory leak           │ valgrind --leak-check                │
   │                                                             │
   │  Async tasks           │ tokio-console                       │
   │                                                             │
   │  Binary size           │ cargo-bloat                         │
   │  Assembly view         │ cargo-asm / cargo-show-asm          │
   │                                                             │
   │  Test coverage         │ cargo-llvm-cov                      │
   │                                                             │
   │  Faster allocator      │ jemallocator, mimalloc              │
   │  Arena                 │ bumpalo                             │
   │  Small data            │ smallvec, tinyvec                   │
   │  Faster hash           │ ahash, fxhash                       │
   │  Data parallel         │ rayon                               │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 20. Mind map cuối

```
                              PERFORMANCE
                                    │
        ┌─────────────┬─────────────┼─────────────┬──────────────┐
        ▼             ▼             ▼             ▼              ▼
   PHILOSOPHY     TOOLS         OPTIMIZATION    COMPILER      ANTIPATTERNS
        │             │             │             │              │
   Đo trước,      criterion     Algorithmic    LTO            Premature opt
   đoán sau       perf          Architectural   codegen-units  Trust micro-bench
   80/20          flamegraph    Implementation  PGO            unsafe "for speed"
   4 cấp work     samply        Micro-opt      target-cpu     Quá nhiều inline
                  heaptrack                    panic=abort    Log in hot path
                  tokio-console
                  hyperfine
                  
   ┌────────────────────────────────────────────────────────┐
   │  CORE INSIGHTS cho SENIOR                              │
   │  ───────────────────────────                           │
   │                                                        │
   │  1. Profile trước, optimize sau                        │
   │                                                        │
   │  2. Algorithmic > micro-optimization                   │
   │                                                        │
   │  3. Cache locality matter — sequential > random        │
   │                                                        │
   │  4. Alloc trong hot path là enemy chính                │
   │                                                        │
   │  5. Mutex contention nhanh thành µs killer             │
   │                                                        │
   │  6. LTO + codegen=1 cho production binary             │
   │                                                        │
   │  7. Async cần tokio-console, không phải perf           │
   │                                                        │
   │  8. Memory profile riêng (heaptrack, dhat)             │
   │                                                        │
   │  9. Benchmark workload thật, không cô lập              │
   │                                                        │
   │  10. Đo improvement, không tin "thấy đẹp"              │
   └────────────────────────────────────────────────────────┘
```

---

## 21. Bộ tài liệu Rust giờ có 11 chủ đề

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
   │      performance-visual      ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 22 files, ~1.2 MB MD                             │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust production-ready                   │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau performance, có thể đào sâu các nhánh thực hành:

- **Unsafe Rust** — raw pointer, atomic ordering, FFI, soundness contracts
- **Iterator deep dive** — implement, lazy, parallel với rayon
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Logging & Observability** — tracing nâng cao, OpenTelemetry, distributed tracing
- **Web framework realistic** — axum project apply 11 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
