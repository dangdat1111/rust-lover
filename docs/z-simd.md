# SIMD trong Rust — Deep Dive

> Tài liệu bổ sung (chương **z**) trong bộ Rust nền tảng. Đọc trước:
> - [a-memory-model.md](./a-memory-model.md) — cache hierarchy, alignment, virtual memory
> - [x-data-layout-visual.md](./x-data-layout-visual.md) — size/align/offset từng byte, padding, ZST
> - [k-performance.md](./k-performance.md) — đo trước đoán sau, criterion, flamegraph
> - [m-iterator.md](./m-iterator.md) — iterator lazy, `chunks_exact`, rayon
> - [n-unsafe-rust.md](./n-unsafe-rust.md) — raw pointer, `target_feature`, UB
>
> SIMD (**S**ingle **I**nstruction, **M**ultiple **D**ata) = 1 lệnh CPU xử lý nhiều phần tử cùng lúc.
> Đây là **Cấp 4 — micro-optimization** trong tháp performance: chỉ chạm tới sau khi đã
> *làm đúng → đo → tối ưu thuật toán/kiến trúc*. Nhưng khi đúng chỗ, nó cho 4×–16× tốc độ.
>
> Tài liệu này dạy theo **nguyên tắc vàng**:
> > **Đo trước → để compiler auto-vectorize → `std::simd` portable → mới đến `core::arch` intrinsics.**
>
> Bạn sẽ học:
> - SIMD là gì, phần cứng đứng sau (lanes, registers, ISA)
> - Data-Oriented Design (AoS vs SoA) — *điều kiện cần* để SIMD ăn tiền
> - Auto-vectorization: viết code để compiler tự sinh SIMD
> - Cách **kiểm chứng** đã vectorize hay chưa (asm + benchmark)
> - `std::simd` (portable) và `core::arch` intrinsics (AVX2/NEON)
> - An toàn, `target_feature`, runtime dispatch
> - Patterns thực tế, hệ sinh thái crate, antipatterns

---

# Mục lục

- [Tầng 1: SIMD là gì — SISD vs SIMD](#tầng-1-simd-là-gì--sisd-vs-simd)
- [Tầng 2: Phần cứng — Lanes, Registers, ISA](#tầng-2-phần-cứng--lanes-registers-isa)
- [Tầng 3: Data-Oriented Design — AoS vs SoA](#tầng-3-data-oriented-design--aos-vs-soa)
- [Tầng 4: Tháp tối ưu SIMD — Thứ tự ưu tiên](#tầng-4-tháp-tối-ưu-simd--thứ-tự-ưu-tiên)
- [Tầng 5: Auto-vectorization — Để compiler làm](#tầng-5-auto-vectorization--để-compiler-làm)
- [Tầng 6: Kiểm chứng — Đã vectorize chưa?](#tầng-6-kiểm-chứng--đã-vectorize-chưa)
- [Tầng 7: std::simd — Portable SIMD](#tầng-7-stdsimd--portable-simd)
- [Tầng 8: core::arch — Intrinsics & target_feature](#tầng-8-corearch--intrinsics--target_feature)
- [Tầng 9: Runtime dispatch & multiversioning](#tầng-9-runtime-dispatch--multiversioning)
- [Tầng 10: An toàn, alignment & UB](#tầng-10-an-toàn-alignment--ub)
- [Tầng 11: Patterns thực tế — reduction, mask, tail](#tầng-11-patterns-thực-tế--reduction-mask-tail)
- [Tầng 12: Hệ sinh thái crate](#tầng-12-hệ-sinh-thái-crate)
- [Tầng 13: SIMD vs đa luồng (rayon) — đừng nhầm](#tầng-13-simd-vs-đa-luồng-rayon--đừng-nhầm)
- [Tầng 14: Antipatterns + 10 nguyên tắc senior](#tầng-14-antipatterns--10-nguyên-tắc-senior)

---

# Tầng 1: SIMD là gì — SISD vs SIMD

## 1.1 Trực giác

CPU thường (scalar / **SISD** — *Single Instruction, Single Data*) làm 1 phép tính trên 1 cặp số mỗi lệnh:

```
a[0] + b[0] = c[0]   ← 1 lệnh ADD
a[1] + b[1] = c[1]   ← 1 lệnh ADD
a[2] + b[2] = c[2]   ← 1 lệnh ADD
a[3] + b[3] = c[3]   ← 1 lệnh ADD
                       → 4 lệnh cho 4 phần tử
```

CPU SIMD có thanh ghi rộng chứa **nhiều phần tử** (lanes) và lệnh xử lý tất cả cùng lúc:

```
[a0 a1 a2 a3] + [b0 b1 b2 b3] = [c0 c1 c2 c3]   ← 1 lệnh ADDPS
                                                  → 1 lệnh cho 4 phần tử
```

→ Cùng tần số CPU, throughput lý thuyết ×4 (SSE), ×8 (AVX2), ×16 (AVX-512) cho `f32`.

## 1.2 Phân loại Flynn (để định vị SIMD)

| Loại | Nghĩa | Ví dụ |
|------|-------|-------|
| **SISD** | 1 lệnh, 1 dữ liệu | code scalar bình thường |
| **SIMD** | 1 lệnh, nhiều dữ liệu | SSE/AVX/NEON — **chương này** |
| **MIMD** | nhiều lệnh, nhiều dữ liệu | đa luồng (threads, rayon) — [Tầng 13](#tầng-13-simd-vs-đa-luồng-rayon--đừng-nhầm) |
| **SIMT** | biến thể SIMD trên GPU | CUDA, wgpu compute |

**Mấu chốt**: SIMD = song song hoá *trong 1 core*, trên *1 lệnh*. Nó **trực giao** với đa luồng — bạn có thể (và nên) kết hợp cả hai.

## 1.3 Khi nào SIMD ăn tiền

SIMD thắng đậm khi:
- **Cùng một phép tính** lặp trên **mảng lớn** đồng nhất kiểu (data parallelism).
- Dữ liệu **liền kề trong bộ nhớ** (contiguous) → load cả vector 1 phát.
- **Ít nhánh** (`if`) phụ thuộc dữ liệu trong vòng nóng.

SIMD *không* giúp khi: logic rẽ nhánh nhiều, dữ liệu phân tán (pointer-chasing), kích thước nhỏ, hoặc bottleneck là I/O / cache miss chứ không phải ALU.

---

# Tầng 2: Phần cứng — Lanes, Registers, ISA

## 2.1 Từ vựng

- **Lane**: 1 "ô" trong thanh ghi vector chứa 1 phần tử. Vector `f32x8` có 8 lanes.
- **Vector width**: độ rộng thanh ghi tính bằng bit. 128 / 256 / 512.
- **ISA / instruction set**: tập lệnh SIMD của kiến trúc (SSE, AVX2, NEON...).
- **Số lanes = width / sizeof(phần tử)**. AVX2 (256-bit): 8×`f32`, 4×`f64`, 8×`i32`, 16×`i16`, 32×`i8`.

## 2.2 Bản đồ ISA

| Kiến trúc | ISA | Width | f32 lanes | Ghi chú |
|-----------|-----|-------|-----------|---------|
| x86_64 | **SSE2** | 128-bit | 4 | baseline, *luôn* có trên x86_64 |
| x86_64 | **AVX** | 256-bit | 8 | ~2011+ |
| x86_64 | **AVX2** | 256-bit | 8 | + integer 256-bit, FMA thường đi kèm |
| x86_64 | **AVX-512** | 512-bit | 16 | server/HEDT, có mask register, hơi kén |
| aarch64 | **NEON** | 128-bit | 4 | *luôn* có trên aarch64 (Apple Silicon, mobile) |
| aarch64 | **SVE/SVE2** | scalable | thay đổi | width do CPU quyết định runtime |
| wasm | **simd128** | 128-bit | 4 | WebAssembly SIMD |

> 💡 SSE2 trên x86_64 và NEON trên aarch64 là **baseline** — compiler được phép dùng tự do mà không cần khai báo. AVX2/AVX-512 thì **không** mặc định bật (vì không phải CPU nào cũng có) → phải `target-cpu`/`target-feature` (Tầng 5, 8).

## 2.3 Thanh ghi vật lý (x86_64)

```
XMM0..15   128-bit  (SSE)
YMM0..15   256-bit  (AVX/AVX2)   — XMM là nửa dưới của YMM
ZMM0..31   512-bit  (AVX-512)    — YMM là nửa dưới của ZMM
```

Số lượng thanh ghi hữu hạn → SIMD không phải "free", còn cạnh tranh register với code khác.

---

# Tầng 3: Data-Oriented Design — AoS vs SoA

> **SIMD chỉ ăn tiền nếu dữ liệu xếp đúng.** Đây là phần quan trọng nhất mà người mới hay bỏ qua: bạn phải thiết kế *layout dữ liệu* cho SIMD, không phải gọi vài intrinsic là xong.

## 3.1 AoS — Array of Structs (cách "tự nhiên" của OOP)

```rust
struct Particle {
    x: f32, y: f32, z: f32,     // vị trí
    vx: f32, vy: f32, vz: f32,  // vận tốc
}
let world: Vec<Particle> = vec![/* ... */];

// Muốn cộng tất cả x với dt*vx:
for p in &mut world {
    p.x += dt * p.vx;   // x và vx nằm xen kẽ y,z,vy,vz...
}
```

Bộ nhớ: `x0 y0 z0 vx0 vy0 vz0 | x1 y1 z1 vx1 vy1 vz1 | ...`

Để SIMD cộng `x0,x1,x2,x3`, CPU phải **gather** chúng từ những ô cách nhau 24 byte → không load thẳng được 1 vector. SIMD gần như vô dụng.

## 3.2 SoA — Struct of Arrays (cách "thân thiện SIMD")

```rust
struct World {
    x: Vec<f32>,  y: Vec<f32>,  z: Vec<f32>,
    vx: Vec<f32>, vy: Vec<f32>, vz: Vec<f32>,
}

// Giờ x liền kề nhau:
for i in 0..world.x.len() {
    world.x[i] += dt * world.vx[i];   // x[0..8] load 1 phát → AVX2 ×8
}
```

Bộ nhớ: `x0 x1 x2 x3 x4 ... | y0 y1 y2 ... | vx0 vx1 ...`

→ `x[0..8]` liền kề → 1 lệnh `loadu` lấy cả 8, 1 lệnh `fmadd`, xong. Compiler **tự** vectorize được.

## 3.3 So sánh

| | AoS | SoA |
|--|-----|-----|
| Truy cập 1 object đủ field | ✅ tốt (1 cache line) | ❌ chạm nhiều mảng |
| Xử lý 1 field qua nhiều object | ❌ stride lớn | ✅ contiguous, SIMD-friendly |
| Auto-vectorization | hiếm khi | thường được |
| Hợp với | logic OOP, ít object | hot loop số học, ECS, DSP |

> 📌 Quy tắc: **hot loop nào quét 1-vài field qua nhiều phần tử → SoA.** Game engine (ECS), xử lý ảnh/âm thanh, ML, vật lý đều dùng SoA. Xem thêm padding/align ở [x-data-layout-visual.md](./x-data-layout-visual.md).

## 3.4 Alignment

SIMD load nhanh nhất khi địa chỉ thẳng hàng theo width (16B cho SSE, 32B cho AVX). `Vec<f32>` thường đã đủ align cho phần tử, nhưng để chắc:

```rust
#[repr(align(32))]              // ép align 32 byte cho AVX
struct Aligned([f32; 8]);
```

Thực tế hiện đại: lệnh **unaligned load** (`_mm256_loadu_ps`) gần như nhanh bằng aligned → cứ dùng `loadu`, đừng tự hành xác vì alignment trừ khi đo thấy khác biệt.

---

# Tầng 4: Tháp tối ưu SIMD — Thứ tự ưu tiên

```
Cấp 0: LÀM ĐÚNG          code scalar rõ ràng, có test
   │   → chưa đúng thì SIMD nhanh cũng vô nghĩa
   ▼
Cấp 1: ĐO                criterion + flamegraph (k-performance)
   │   → xác nhận hot loop NÀY thật sự là bottleneck
   ▼
Cấp 2: DATA LAYOUT        AoS → SoA, contiguous, ít branch
   │   → điều kiện cần; thiếu bước này SIMD không ăn
   ▼
Cấp 3: AUTO-VECTORIZE     viết loop sạch + target-cpu=native
   │   → ~80% trường hợp đủ nhanh, KHÔNG cần unsafe
   ▼
Cấp 4: std::simd          portable, an toàn, đa nền tảng (nightly)
   │   → khi auto-vec không đủ, vẫn muốn an toàn
   ▼
Cấp 5: core::arch         intrinsics AVX2/NEON, unsafe, last resort
       → vắt kiệt hiệu năng, chấp nhận unsafe + per-arch code
```

**Đừng nhảy thẳng Cấp 5.** Mỗi cấp xuống là tăng độ phức tạp & rủi ro UB, giảm tính di động. Đa số bài toán dừng ở Cấp 3.

---

# Tầng 5: Auto-vectorization — Để compiler làm

LLVM (backend của rustc) tự sinh SIMD cho vòng lặp đủ "đẹp". Đây là cách **rẻ nhất, an toàn nhất, di động nhất**.

## 5.1 Bật cờ target-cpu

Mặc định rustc build cho CPU baseline (chỉ SSE2 trên x86_64) → không dùng AVX2. Bật:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Hoặc cố định trong `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]   # build cho CPU của MÁY build
```

> ⚠️ `target-cpu=native` build binary chỉ chạy trên CPU tương đương trở lên. Distribute binary cho máy lạ → dùng `target-feature` cụ thể + runtime dispatch (Tầng 9), đừng `native`.

## 5.2 Viết loop "thân thiện vectorize"

```rust
// ✅ Iterator + zip: không bounds-check, biên rõ → LLVM vectorize tốt
pub fn add_assign(a: &mut [f32], b: &[f32]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += *y;
    }
}
```

Điều kiện để compiler vectorize:
- **Số vòng đếm được**, không phụ thuộc dữ liệu giữa các vòng (no loop-carried dependency phức tạp).
- **Không có bounds-check** trong thân (dùng iterator hoặc `chunks_exact` thay vì `a[i]`).
- **Ít/không nhánh** phụ thuộc dữ liệu.
- Phép toán **kết hợp được** (với float cần `-ffast-math`-like; xem 5.4).

## 5.3 `chunks_exact` — bỏ bounds-check, lộ rõ vector width

```rust
pub fn sum(data: &[f32]) -> f32 {
    let mut acc = [0.0f32; 8];                 // 8 accumulator song song
    let mut chunks = data.chunks_exact(8);
    for c in &mut chunks {                      // mỗi c là &[f32] đúng 8 phần tử
        for i in 0..8 { acc[i] += c[i]; }       // LLVM nhận ra → 1 lệnh add vector
    }
    let tail: f32 = chunks.remainder().iter().sum();  // phần dư < 8
    acc.iter().sum::<f32>() + tail
}
```

`chunks_exact(8)` đảm bảo mỗi mảnh đúng 8 phần tử → loop con bị unroll & vectorize, `remainder()` xử lý đuôi.

## 5.4 Bẫy số thực: thứ tự cộng

Cộng `f32` **không kết hợp** (`(a+b)+c ≠ a+(b+c)` do làm tròn). LLVM **không** được tự đổi thứ tự cộng float → khó vectorize reduction nếu bạn viết `sum += data[i]` tuần tự.

Cách xử lý:
- Dùng **nhiều accumulator** như 5.3 (chấp nhận kết quả khác bit so với cộng tuần tự — thường ok).
- Hoặc `std::simd` reduction (Tầng 7) — bạn *chủ động* chọn cộng song song.

> Với **số nguyên**, phép cộng kết hợp → LLVM vectorize reduction thoải mái.

---

# Tầng 6: Kiểm chứng — Đã vectorize chưa?

**Đừng tin, hãy đọc asm + đo.** Đây là kỹ năng phân biệt người biết SIMD thật với người "nghĩ là mình dùng SIMD".

## 6.1 Đọc assembly

```bash
cargo install cargo-show-asm        # 1 lần
cargo asm --release my_crate::add_assign   # xem asm hàm cụ thể
```

Tìm dấu hiệu SIMD:
- x86 AVX: lệnh `vaddps`, `vmulps`, `vfmadd...`, thanh ghi `ymm0..15`.
- x86 SSE: `addps`, `mulps`, thanh ghi `xmm`.
- aarch64 NEON: `fadd v0.4s`, `fmla`, thanh ghi `v0..31`.

Thấy `addss`/`mulss` (có chữ **s** = scalar single) hoặc chỉ `xmm` từng phần tử → **chưa** vectorize.

Hoặc dán code lên [godbolt.org](https://godbolt.org) (Compiler Explorer) với flag `-C opt-level=3 -C target-cpu=native`.

## 6.2 Benchmark — bằng chứng cuối cùng

```rust
// benches/simd.rs — dùng criterion (xem k-performance.md)
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench(c: &mut Criterion) {
    let a = vec![1.0f32; 1 << 16];
    let b = vec![2.0f32; 1 << 16];
    c.bench_function("dot", |bn| bn.iter(|| dot(black_box(&a), black_box(&b))));
}
criterion_group!(g, bench);
criterion_main!(g);
```

`black_box` chặn compiler optimize mất phép tính. So sánh trước/sau khi bật `target-cpu=native` → con số thật.

> 📌 Quy luật: nếu bench *không* nhanh hơn sau khi "thêm SIMD", thì hoặc (a) chưa vectorize thật, (b) bottleneck là memory bandwidth chứ không phải ALU, (c) dữ liệu quá nhỏ. Đọc asm để biết là (a) hay không.

---

# Tầng 7: std::simd — Portable SIMD

`std::simd` (module `core::simd`) cho phép viết SIMD **tường minh** mà vẫn **di động** (tự map sang SSE/AVX/NEON/wasm). Hiện còn **nightly** sau cờ `#![feature(portable_simd)]`.

```rust
#![feature(portable_simd)]
use std::simd::{f32x8, Simd};
use std::simd::num::SimdFloat;   // cho reduce_sum

fn dot_portable(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let lanes = 8;
    let chunks = a.len() / lanes;

    let mut acc = f32x8::splat(0.0);            // [0,0,0,0,0,0,0,0]
    for i in 0..chunks {
        let va = f32x8::from_slice(&a[i * lanes..]);
        let vb = f32x8::from_slice(&b[i * lanes..]);
        acc += va * vb;                          // SIMD multiply-add per lane
    }
    let mut sum = acc.reduce_sum();              // cộng dồn 8 lanes → scalar

    for i in chunks * lanes..a.len() {           // tail
        sum += a[i] * b[i];
    }
    sum
}
```

## 7.1 API cốt lõi

| API | Ý nghĩa |
|-----|---------|
| `Simd<f32, 8>` / alias `f32x8` | vector 8 lane f32 |
| `Simd::splat(x)` | mọi lane = x |
| `Simd::from_slice(&s)` | load 8 phần tử đầu của slice |
| `v.to_array()` / `v.copy_to_slice(&mut s)` | store ra |
| `a + b`, `a * b`, `a.max(b)` | toán tử per-lane |
| `v.reduce_sum()` / `reduce_max()` | horizontal reduction |
| `a.simd_lt(b)` → `Mask` | so sánh per-lane ra mask |
| `mask.select(a, b)` | chọn lane theo mask (branchless `if`) |
| `simd_swizzle!(v, [..])` | hoán vị lanes |

## 7.2 Ưu / nhược

- ✅ **An toàn** (không `unsafe`), **di động** (1 code chạy mọi kiến trúc), đọc dễ.
- ✅ Chọn được số lane → tự điều khiển, không phụ thuộc compiler "có chịu vectorize hay không".
- ❌ Còn **nightly**, API có thể đổi (vd `reduce_sum` nằm trong trait `SimdFloat`).
- ❌ Không chạm được lệnh "đặc sản" của 1 ISA (vd shuffle phức tạp AVX-512) → khi đó mới xuống `core::arch`.

> Nếu cần stable hôm nay mà vẫn portable: dùng crate [`wide`](https://crates.io/crates/wide) (Tầng 12) — API tương tự, chạy trên stable.

---

# Tầng 8: core::arch — Intrinsics & target_feature

Khi cần vắt kiệt hoặc dùng lệnh đặc thù: gọi thẳng intrinsic trong `std::arch`. Đây là `unsafe` và **per-arch**.

## 8.1 Dot product với AVX2 + FMA

```rust
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]   // hàm này GIẢ ĐỊNH CPU có avx2+fma
unsafe fn dot_avx2(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    debug_assert_eq!(a.len(), b.len());

    let n = a.len();
    let mut acc = _mm256_setzero_ps();       // ymm = [0;8]
    let mut i = 0;
    while i + 8 <= n {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));   // load 8 f32 (unaligned ok)
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        acc = _mm256_fmadd_ps(va, vb, acc);            // acc = va*vb + acc
        i += 8;
    }
    // horizontal sum của 8 lane
    let mut tmp = [0.0f32; 8];
    _mm256_storeu_ps(tmp.as_mut_ptr(), acc);
    let mut sum: f32 = tmp.iter().sum();

    while i < n {                            // tail scalar
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}
```

## 8.2 `#[target_feature(enable = ...)]` — vì sao hàm thành `unsafe`

- Cho phép compiler dùng lệnh AVX2 **trong hàm này** dù binary build ở baseline.
- Hàm trở thành `unsafe` vì: **gọi nó trên CPU KHÔNG có AVX2 = UB / illegal instruction**. Trách nhiệm của *người gọi* là đảm bảo CPU hỗ trợ (xem Tầng 9).
- Khác `-C target-feature=+avx2` toàn cục (ép cả binary cần AVX2). `#[target_feature]` cục bộ → an toàn để ship binary đa dạng CPU + runtime dispatch.

## 8.3 NEON (aarch64) — tương đương

```rust
#[cfg(target_arch = "aarch64")]
fn dot_neon(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::aarch64::*;
    // NEON là baseline trên aarch64 → KHÔNG cần target_feature, và intrinsic vẫn unsafe.
    unsafe {
        let n = a.len();
        let mut acc = vdupq_n_f32(0.0);        // [0;4]
        let mut i = 0;
        while i + 4 <= n {
            let va = vld1q_f32(a.as_ptr().add(i));
            let vb = vld1q_f32(b.as_ptr().add(i));
            acc = vfmaq_f32(acc, va, vb);      // acc += va*vb
            i += 4;
        }
        let mut sum = vaddvq_f32(acc);          // horizontal sum 4 lane
        while i < n { sum += a[i] * b[i]; i += 1; }
        sum
    }
}
```

→ Thấy vì sao intrinsics **không di động**: phải viết lại cho từng kiến trúc. Đây là cái giá của Cấp 5.

---

# Tầng 9: Runtime dispatch & multiversioning

Ship 1 binary chạy tối ưu trên *mọi* CPU: chọn cài đặt **lúc chạy** dựa trên CPU thật.

```rust
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());

    #[cfg(target_arch = "x86_64")]
    {
        // macro kiểm tra feature LÚC CHẠY (cache lại, rất rẻ)
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return unsafe { dot_avx2(a, b) };   // an toàn vì ĐÃ kiểm tra
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        return dot_neon(a, b);                   // NEON luôn có trên aarch64
    }

    dot_scalar(a, b)                             // fallback portable
}
```

Mẫu hình:
1. Viết **fallback scalar** luôn đúng.
2. Viết 1–N phiên bản `#[target_feature]` cho các ISA.
3. `is_x86_feature_detected!` / `is_aarch64_feature_detected!` chọn lúc chạy.
4. Việc gọi `unsafe fn` chỉ hợp lệ *sau khi* đã detect → đây là chỗ `unsafe` được "trả nợ".

> Crate [`multiversion`](https://crates.io/crates/multiversion) tự sinh các phiên bản + dispatch bằng 1 attribute, đỡ viết tay.

---

# Tầng 10: An toàn, alignment & UB

| Rủi ro | Nguyên nhân | Phòng |
|--------|-------------|-------|
| **Illegal instruction** | gọi `#[target_feature]` fn trên CPU thiếu feature | luôn `is_*_feature_detected!` trước (Tầng 9) |
| **Out-of-bounds load** | `loadu` `add(i)` vượt slice | đảm bảo `i + lanes <= len`; tail scalar |
| **Đọc rác / uninit** | load quá `len` rồi mask | dùng masked load (`std::simd` `Mask`, hoặc pad buffer) |
| **Sai số float** | đổi thứ tự cộng | chấp nhận khác bit, hoặc tránh nếu cần xác định bit-exact |
| **Aliasing UB** | con trỏ ghi/đọc chồng nhau | đừng cho input/output overlap; tôn trọng `&mut` |

Nguyên tắc an toàn senior:
- **Bọc `unsafe` trong API `safe`** (như `dot` ở Tầng 9): người dùng ngoài không thấy `unsafe`.
- `unsafe` block **càng nhỏ càng tốt**, comment rõ *invariant* phải giữ.
- Chạy [**miri**](https://github.com/rust-lang/miri) cho code unsafe non-SIMD; lưu ý miri **không** chạy được intrinsics SIMD → tách logic để test phần bọc.
- So kết quả SIMD với scalar trong test (`proptest`/`quickcheck`) để bắt sai lệch. Xem [n-unsafe-rust.md](./n-unsafe-rust.md) và [o-testing.md](./o-testing.md).

---

# Tầng 11: Patterns thực tế — reduction, mask, tail

## 11.1 Horizontal reduction (sum/max/min)

Cộng dồn nhiều lane về 1 scalar. Dùng **nhiều accumulator** để giấu latency (instruction-level parallelism), rồi reduce cuối:

```rust
// std::simd: 4 accumulator song song, reduce 1 lần ở cuối
let mut acc = [f32x8::splat(0.0); 4];
for (k, chunk) in data.chunks_exact(32).enumerate() {
    let _ = k;
    for j in 0..4 {
        acc[j] += f32x8::from_slice(&chunk[j * 8..]);
    }
}
let total = (acc[0] + acc[1] + acc[2] + acc[3]).reduce_sum();
```

## 11.2 Branchless với mask (thay `if` trong vòng nóng)

```rust
// Thay: for x in v { if *x < 0.0 { *x = 0.0 } }   ← branch khó vectorize
// Bằng select theo mask:
let v = f32x8::from_slice(&data[i..]);
let clamped = v.simd_lt(f32x8::splat(0.0))   // mask: lane nào < 0
    .select(f32x8::splat(0.0), v);            // chọn 0.0 hay v
clamped.copy_to_slice(&mut data[i..]);
```

→ ReLU branchless. Mọi lane xử lý cùng lúc, không rẽ nhánh.

## 11.3 Xử lý đuôi (tail) — bài toán muôn thuở

Mảng `len` không chia hết cho số lane → phần dư. 3 cách:

| Cách | Mô tả | Khi dùng |
|------|-------|----------|
| **Scalar tail** | vòng `while i < n` cho phần dư | đơn giản nhất, mặc định |
| **Masked load/store** | `std::simd` masked ops xử lý nốt 1 vector | tránh code 2 nhánh, cần mask |
| **Pad buffer** | cấp phát bội số lane, fill 0 | khi kiểm soát được cấp phát |

`chunks_exact(N)` + `remainder()` là cách Rust-idiomatic nhất cho scalar tail.

## 11.4 Khi nào KHÔNG nên SIMD hoá

- Vòng có **branch nặng phụ thuộc dữ liệu** (parser, state machine).
- **Gather/scatter** truy cập ngẫu nhiên (index gián tiếp) — chậm, trừ AVX-512 gather.
- Dữ liệu nhỏ (vài chục phần tử) — overhead setup vector ăn hết lợi.
- Bottleneck là **memory bandwidth** → SIMD không cứu được, cần giảm dữ liệu / cải thiện cache (xem [a-memory-model.md](./a-memory-model.md)).

---

# Tầng 12: Hệ sinh thái crate

| Crate | Vai trò | Stable? |
|-------|---------|---------|
| `std::simd` (`core::simd`) | portable SIMD chính chủ | ❌ nightly |
| [`wide`](https://crates.io/crates/wide) | portable SIMD types (`f32x8`...) | ✅ stable |
| [`pulp`](https://crates.io/crates/pulp) | abstraction an toàn + auto dispatch theo arch | ✅ stable |
| [`multiversion`](https://crates.io/crates/multiversion) | tự sinh nhiều phiên bản + runtime dispatch | ✅ stable |
| [`glam`](https://crates.io/crates/glam) | đại số tuyến tính (game/3D) đã SIMD sẵn | ✅ stable |
| [`nalgebra`](https://crates.io/crates/nalgebra) + `simba` | linear algebra tổng quát, SIMD optional | ✅ stable |
| [`ndarray`](https://crates.io/crates/ndarray) | mảng n chiều, BLAS backend | ✅ stable |
| [`faster`](https://crates.io/crates/faster) | iterator-style SIMD (cũ, tham khảo) | ⚠️ ít bảo trì |
| [`rayon`](https://crates.io/crates/rayon) | **đa luồng** data-parallel (kết hợp với SIMD) | ✅ stable |

> 🎯 Lời khuyên thực dụng: cần **stable + portable** → `wide` hoặc `pulp`. Cần **nightly + chính chủ** → `std::simd`. Cần **3D/game** → đừng tự viết, dùng `glam`.

---

# Tầng 13: SIMD vs đa luồng (rayon) — đừng nhầm

Hai trục song song hoá **khác nhau**, **bổ sung** cho nhau:

```
            1 core                       N cores
        ┌──────────────┐          ┌──────────────────────┐
SIMD →  │ 1 lệnh / 8 ô │   ×N  →  │ rayon: chia mảng cho  │
        │ (data trong  │  cores   │ nhiều thread, mỗi     │
        │  1 thread)   │          │ thread lại SIMD bên   │
        └──────────────┘          │ trong → nhân đôi gain │
                                  └──────────────────────┘
```

```rust
use rayon::prelude::*;
// rayon chia khối + mỗi khối tự auto-vectorize → kết hợp MIMD × SIMD
let total: f32 = a.par_chunks(4096)
    .zip(b.par_chunks(4096))
    .map(|(ca, cb)| dot(ca, cb))   // dot() bên trong dùng SIMD (Tầng 9)
    .sum();
```

| | SIMD | rayon / threads |
|--|------|-----------------|
| Phạm vi | trong 1 core, 1 lệnh | nhiều core |
| Tăng tốc tối đa | × số lane (4–16) | × số core |
| Chi phí | gần như 0 setup | spawn/sync thread |
| Rủi ro | UB nếu sai feature | data race nếu chia sai |

→ Bài toán lớn: **rayon chia thô + SIMD trong mỗi mảnh**. Xem rayon ở [m-iterator.md](./m-iterator.md).

---

# Tầng 14: Antipatterns + 10 nguyên tắc senior

## 14.1 Antipatterns

| ❌ Antipattern | Vì sao sai | ✅ Thay bằng |
|---------------|-----------|-------------|
| Viết intrinsics ngay từ đầu | chưa đo, chưa chắc là bottleneck | đo trước (Cấp 1) |
| Giữ AoS rồi than SIMD không nhanh | layout sai, không load vector được | đổi sang SoA (Tầng 3) |
| `target-feature=+avx2` toàn cục rồi ship | crash trên CPU cũ | runtime dispatch (Tầng 9) |
| Tin là đã SIMD mà không đọc asm | thường vẫn scalar | `cargo asm` / godbolt (Tầng 6) |
| `unsafe` intrinsics không có fallback | crash trên CPU thiếu feature | luôn có nhánh scalar |
| Quên xử lý tail | bỏ sót/đọc lố phần dư | `chunks_exact` + remainder |
| SIMD hoá vòng đầy branch | không vectorize được, code rối | branchless mask, hoặc đừng SIMD |
| Bỏ `black_box` khi bench | compiler xoá phép tính → số ảo | `black_box` input/output |

## 14.2 Mười nguyên tắc senior về SIMD

1. **Đo trước, đoán sau.** SIMD là Cấp 4 — đừng chạm trước khi profile xác nhận hot loop.
2. **Layout trước intrinsics.** AoS→SoA cho gain lớn hơn mọi intrinsic, và là điều kiện cần.
3. **Để compiler làm trước.** `target-cpu=native` + loop sạch giải quyết ~80% trường hợp, miễn phí, di động.
4. **Đọc assembly để kiểm chứng.** "Nghĩ là dùng SIMD" ≠ "đang dùng SIMD".
5. **Portable trước, intrinsics sau.** `std::simd`/`wide` an toàn & đa nền tảng; chỉ xuống `core::arch` khi thật cần.
6. **Mọi intrinsics phải có fallback scalar** + runtime dispatch — không bao giờ ship binary crash trên CPU cũ.
7. **Bọc `unsafe` trong API an toàn**, block nhỏ, comment invariant.
8. **Test SIMD đối chiếu scalar** (proptest) — bắt sai lệch logic & làm tròn.
9. **SIMD ⟂ threads.** Kết hợp rayon (chia core) × SIMD (trong core), đừng nhầm là một.
10. **Biết khi nào dừng.** Branch nặng, dữ liệu nhỏ, bound bởi bandwidth → SIMD không cứu; đừng cố.

## 14.3 Toolkit

```
Đo:        criterion, hyperfine, perf, flamegraph        (k-performance.md)
Đọc asm:   cargo-show-asm (cargo asm), godbolt.org
Kiểm tra:  miri (phần unsafe non-SIMD), proptest đối chiếu scalar
Cờ build:  RUSTFLAGS="-C target-cpu=native"  /  .cargo/config.toml
Detect:    is_x86_feature_detected!, is_aarch64_feature_detected!
Crates:    wide, pulp, multiversion, glam, ndarray, rayon
```

---

> **Tóm tắt một câu**: SIMD trong Rust = *thiết kế dữ liệu SoA → để compiler auto-vectorize → kiểm chứng bằng asm/bench → `std::simd` portable nếu chưa đủ → `core::arch` intrinsics + runtime dispatch là cùng đường.* Luôn đo, luôn có fallback, luôn bọc unsafe.

Đọc kèm bản minh hoạ: [z-simd-visual.md](./z-simd-visual.md).
