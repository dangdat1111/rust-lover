# SIMD Rust — Minh Hoạ Trực Quan

> Companion visual cho [z-simd.md](./z-simd.md). Đọc song song.

---

## 1. Bức tranh lớn — SIMD Universe

```
                         SIMD TRONG RUST
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │   "1 LỆNH — NHIỀU DỮ LIỆU"   (data parallelism trong 1 core) │
   │                                                              │
   │   ┌────────────┐   ┌─────────────┐   ┌────────────────────┐  │
   │   │ DATA LAYOUT│   │ AUTO-VEC    │   │ TƯỜNG MINH          │  │
   │   │ ──────────│   │ ─────────   │   │ ───────────         │  │
   │   │ AoS → SoA  │ → │ compiler tự │ → │ std::simd (port.)  │  │
   │   │ contiguous │   │ sinh SIMD   │   │ core::arch (intr.) │  │
   │   │ align      │   │ target-cpu  │   │ + runtime dispatch │  │
   │   └────────────┘   └─────────────┘   └────────────────────┘  │
   │      điều kiện cần    rẻ & di động      last resort, unsafe   │
   │                                                              │
   │   ┌──────────────────────────────────────────────────────┐  │
   │   │  NGUYÊN TẮC VÀNG                                      │  │
   │   │  đo → SoA → auto-vec → std::simd → core::arch        │  │
   │   │  luôn fallback scalar · luôn đọc asm · luôn bọc unsafe│  │
   │   └──────────────────────────────────────────────────────┘  │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 2. SISD vs SIMD — một lệnh làm bao nhiêu?

```
  SISD (scalar)                      SIMD (vector, AVX2 = 8 lane f32)

  a0 ┐                               ┌ a0 a1 a2 a3 a4 a5 a6 a7 ┐
     │ ADD → c0   (lệnh 1)           │                          │ 1 lệnh
  b0 ┘                               │ a + b                    │ VADDPS
  a1 ┐                               │                          │
     │ ADD → c1   (lệnh 2)           └ b0 b1 b2 b3 b4 b5 b6 b7 ┘
  b1 ┘                                 = c0 c1 c2 c3 c4 c5 c6 c7
  a2 ┐
     │ ADD → c2   (lệnh 3)           ┌────────────────────────────┐
  b2 ┘                               │ 8 phần tử  → 1 lệnh        │
  ...                                │ throughput lý thuyết ×8    │
  → 8 phần tử = 8 lệnh               └────────────────────────────┘
```

---

## 3. Thang độ rộng thanh ghi (x86_64)

```
   bit:  0                                                      512
         ├──────────────────────────────────────────────────────┤

   XMM   ████████████████                                            128-bit  SSE
         │  4 × f32     │                                            (baseline x86_64)

   YMM   ████████████████████████████████                           256-bit  AVX/AVX2
         │        8 × f32         │                                  (XMM = nửa dưới)

   ZMM   ████████████████████████████████████████████████████████   512-bit  AVX-512
         │                16 × f32                │                  (YMM = nửa dưới)

   ──────────────────────────────────────────────────────────────
   aarch64:  Vn  ████████████████   128-bit NEON  (4 × f32, baseline)
   wasm:         ████████████████   128-bit simd128 (4 × f32)

   số lane = width / sizeof(T)    →  f32 (4B):  SSE=4  AVX2=8  AVX512=16
                                     f64 (8B):  SSE=2  AVX2=4  AVX512=8
                                     i8  (1B):  SSE=16 AVX2=32 AVX512=64
```

---

## 4. AoS vs SoA — vì sao layout quyết định tất cả

```
 ❌ AoS — Array of Structs                  ✅ SoA — Struct of Arrays
 struct Particle{ x,y,z, vx,vy,vz }         struct World{ x:Vec, y:Vec, ... }

 bộ nhớ (muốn cộng các .x):                 bộ nhớ (muốn cộng các .x):
 ┌──┬──┬──┬───┬───┬───┬──┬──┬──┐            ┌──┬──┬──┬──┬──┬──┬──┬──┐
 │x0│y0│z0│vx0│vy0│vz0│x1│y1│z1│...         │x0│x1│x2│x3│x4│x5│x6│x7│...
 └▲─┴──┴──┴───┴───┴───┴▲─┴──┴──┘            └▲────────────────────▲┘
  │   stride 24B        │                    └──── 1 vector load ──┘
  └── x0 ──────────────┘x1                   x0..x7 liền kề → loadu 1 phát

 SIMD muốn x0,x1,..x7 nhưng chúng           SIMD load thẳng [x0..x8] vào ymm
 cách nhau 24B → phải gather → CHẬM         → 1 lệnh VMULPS/VFMADD → ×8
 (compiler thường BỎ vectorize)             (compiler TỰ vectorize được)

 ┌─────────────────────────────────────────────────────────────────┐
 │ 📌 Hot loop quét 1 field qua nhiều phần tử  →  PHẢI SoA          │
 │    Game ECS · ảnh/âm thanh/DSP · ML · vật lý  đều dùng SoA       │
 └─────────────────────────────────────────────────────────────────┘
```

---

## 5. Tháp tối ưu — đi từ trên xuống, đừng nhảy cóc

```
                  ┌──────────────────────────────┐
       Cấp 0      │ LÀM ĐÚNG  (scalar + test)    │  nền móng
                  └──────────────┬───────────────┘
                                 ▼
       Cấp 1      │ ĐO  criterion + flamegraph   │  xác nhận bottleneck
                                 ▼
       Cấp 2      │ DATA LAYOUT  AoS → SoA       │  điều kiện cần
                                 ▼
       Cấp 3      │ AUTO-VECTORIZE               │  ~80% dừng ở đây
                  │ loop sạch + target-cpu=native│  rẻ · an toàn · di động
                                 ▼
       Cấp 4      │ std::simd / wide (portable)  │  tường minh, vẫn an toàn
                                 ▼
       Cấp 5      │ core::arch intrinsics        │  last resort
                  │ unsafe + target_feature      │  per-arch · vắt kiệt
                  └──────────────────────────────┘

   ▲ càng xuống: nhanh hơn (đôi khi) · phức tạp hơn · kém di động · dễ UB hơn
   📌 mỗi cấp phải JUSTIFY bằng số đo, không bằng cảm giác
```

---

## 6. Auto-vectorization — điều kiện để compiler "chịu" sinh SIMD

```
   VÒNG LẶP "ĐẸP"                          BẬT CỜ
   ┌────────────────────────────┐         RUSTFLAGS="-C target-cpu=native"
   │ ✅ số vòng đếm được         │              │
   │ ✅ không bounds-check       │              ▼
   │    (iterator / chunks_exact)│        ┌──────────────┐
   │ ✅ không loop-carried dep   │        │  LLVM auto-   │
   │ ✅ ít/không branch          │  ───►  │  vectorizer   │ ───► vaddps/vfmadd
   │ ✅ phép toán kết hợp được   │        └──────────────┘      ymm registers
   └────────────────────────────┘

   VÒNG "XẤU" → LLVM bỏ cuộc, sinh scalar:
   ┌────────────────────────────┐
   │ ❌ a[i] có bounds-check     │   ┌────────────────────────────────────┐
   │ ❌ if data[i] > 0 {...}     │   │ float sum tuần tự KHÔNG kết hợp     │
   │ ❌ break/continue theo data │   │ (a+b)+c ≠ a+(b+c) → LLVM không dám  │
   │ ❌ sum += x (1 accumulator) │   │ đổi thứ tự → dùng NHIỀU accumulator │
   └────────────────────────────┘   └────────────────────────────────────┘
```

---

## 7. Horizontal reduction — gộp lanes về scalar

```
   acc (f32x8) sau vòng lặp:
   ┌────┬────┬────┬────┬────┬────┬────┬────┐
   │ s0 │ s1 │ s2 │ s3 │ s4 │ s5 │ s6 │ s7 │
   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
     └─┬──┘    └─┬──┘    └─┬──┘    └─┬──┘      reduce_sum()
       +         +         +         +         (log2(8)=3 bước)
     ┌─┴───────┬─┴───────┬─┴───────┬─┘
     │ s01     │ s23     │ s45 s67 │
     └────┬────┴────┬────┴────┬────┘
          +         +         +
        ┌─┴─────────┴───┐
        │   s0123       │ s4567
        └───────┬───────┘
                +
            ┌───┴───┐
            │ TOTAL │  ← 1 scalar
            └───────┘

   💡 Mẹo ILP: dùng 2–4 acc song song trong loop (giấu latency),
      chỉ reduce 1 lần ở CUỐI:  (acc0+acc1+acc2+acc3).reduce_sum()
```

---

## 8. Branchless bằng mask — bỏ `if` khỏi vòng nóng

```
   ReLU:  x < 0 ? 0 : x        (làm cho cả 8 lane CÙNG LÚC)

   v        = [ -1.0  2.0  -3.0  4.0  0.5  -0.1  6.0  -7.0 ]
                 │     │     │     │    │     │    │     │
   v.simd_lt(0) → so sánh per-lane → MASK:
   mask     = [  T     F     T     F    F     T    F     T  ]
                 │     │     │     │    │     │    │     │
   mask.select(splat(0.0), v):
              chọn 0.0 ─┐   chọn v ─┐
   kết quả  = [  0.0   2.0   0.0   4.0  0.5   0.0  6.0   0.0 ]

   ┌──────────────────────────────────────────────────────┐
   │ KHÔNG có lệnh nhảy (branch) → CPU không mispredict     │
   │ → vectorize được · pipeline mượt                      │
   └──────────────────────────────────────────────────────┘
```

---

## 9. Xử lý đuôi (tail) — len không chia hết cho lane

```
   data.len() = 19,  lane = 8

   ┌──────────────────┬──────────────────┬───────────┐
   │  chunk 0 (8)     │  chunk 1 (8)     │ remainder │
   │  [0 .. 8)        │  [8 .. 16)       │  [16..19) │
   └────────┬─────────┴────────┬─────────┴─────┬─────┘
            │ SIMD             │ SIMD          │ scalar
            ▼                  ▼               ▼
        f32x8 ops          f32x8 ops      while i<n { ... }

   chunks_exact(8) ──► lặp các mảnh đúng 8     (vectorize)
   .remainder()    ──► slice 3 phần tử cuối    (scalar tail)

   3 chiến lược:  [scalar tail]  [masked load/store]  [pad buffer = 0]
                   đơn giản nhất   1 nhánh, cần mask    kiểm soát alloc
```

---

## 10. target_feature & runtime dispatch — ship 1 binary cho mọi CPU

```
                         pub fn dot(a, b)   ← API an toàn, public
                                │
              ┌─────────────────┼──────────────────┐
              ▼                 ▼                  ▼
   is_x86_feature_detected!  (aarch64)         fallback
        ("avx2","fma")?       NEON baseline      scalar
        │ có        │ không        │                │
        ▼           └──────────────┼────────────────┘
   unsafe dot_avx2                 ▼
   #[target_feature(              dot_scalar  (luôn đúng, mọi CPU)
     enable="avx2,fma")]

   ┌───────────────────────────────────────────────────────────────┐
   │ #[target_feature] làm hàm thành `unsafe`:                      │
   │   gọi trên CPU THIẾU feature = illegal instruction (UB).        │
   │   → chỉ gọi SAU khi is_*_feature_detected! = true.             │
   │   → đó là lúc `unsafe` được "trả nợ".                          │
   │                                                               │
   │ KHÁC: -C target-feature=+avx2 toàn cục → cả binary cần AVX2    │
   │       → crash trên CPU cũ. ĐỪNG ship kiểu này.                │
   └───────────────────────────────────────────────────────────────┘
```

---

## 11. SIMD ⟂ Threads — hai trục, nhân nhau

```
                    1 CORE                         N CORES
            ┌────────────────────┐        ┌────────────────────────┐
   SIMD     │  ymm: [■■■■■■■■]    │        │  rayon chia mảng:       │
   (lane)   │  1 lệnh = 8 phần tử│  ×N    │  ┌────┐┌────┐┌────┐┌────┐│
            │                    │  cores │  │core││core││core││core││
            │  gain ×4..16       │        │  │+SIMD││+SIMD││+SIMD││+SIMD│
            └────────────────────┘        │  └────┘└────┘└────┘└────┘│
                                          │  gain ×(#core × #lane)  │
                                          └────────────────────────┘

   par_chunks(N)          → rayon: chia khối cho nhiều core   (MIMD)
        └─ mỗi khối gọi dot() → bên trong tự SIMD             (SIMD)

   ┌─────────────────────────────────────────────────────────┐
   │ SIMD ≠ threads. Kết hợp: rayon chia thô × SIMD trong core│
   └─────────────────────────────────────────────────────────┘
```

---

## 12. Kiểm chứng — đọc asm tìm dấu vết SIMD

```
   cargo asm --release crate::ham        |     godbolt.org (-O3 -C target-cpu=native)

   ┌─────────────────────────────┬───────────────────────────────────┐
   │  ĐÃ vectorize ✅            │  CHƯA vectorize ❌                │
   ├─────────────────────────────┼───────────────────────────────────┤
   │ x86 AVX:  vaddps  ymm0,...  │  addss  xmm0,...   (s = scalar)   │
   │           vfmadd231ps ymm.. │  mulss  xmm0,...                  │
   │ x86 SSE:  addps   xmm0,...  │  từng phần tử 1 lệnh              │
   │ NEON:     fadd v0.4s, ...   │  fadd s0, s1  (1 lane)            │
   │           fmla v0.4s, ...   │                                   │
   └─────────────────────────────┴───────────────────────────────────┘

   p = packed (vector)   ·   s = scalar (1 phần tử)
   ymm/zmm = AVX/AVX-512  ·  .4s = NEON 4×f32  ·  xmm đơn lẻ ≈ scalar

   → rồi criterion đo con số THẬT (nhớ black_box). asm nói "có dùng",
     bench nói "có nhanh hơn không". Cần CẢ HAI.
```

---

## 13. Decision tree — có nên SIMD không & dùng cấp nào?

```
   Đã profile, hot loop này là bottleneck?
   │
   ├─ KHÔNG → DỪNG. Tối ưu chỗ khác (thuật toán/kiến trúc/cache).
   │
   └─ CÓ → Bottleneck là ALU/compute (không phải bandwidth/IO)?
           │
           ├─ KHÔNG → SIMD không cứu. Giảm dữ liệu / sửa cache.
           │
           └─ CÓ → Dữ liệu contiguous & đồng kiểu? (SoA?)
                   │
                   ├─ KHÔNG → đổi layout AoS→SoA TRƯỚC. Rồi quay lại.
                   │
                   └─ CÓ → Vòng ít branch, đếm được?
                           │
                           ├─ branch nặng → branchless mask? nếu không → bỏ SIMD
                           │
                           └─ ok → bật target-cpu=native, ĐỌC ASM
                                   │
                                   ├─ đã auto-vec & đủ nhanh → XONG ✅
                                   │
                                   └─ chưa đủ → cần stable & portable?
                                               │
                                               ├─ có → std::simd(nightly)/wide(stable)
                                               │
                                               └─ cần lệnh đặc thù/vắt kiệt
                                                  → core::arch intrinsics
                                                    + fallback + runtime dispatch
```

---

## 14. Mind map tổng kết

```
                          ┌──────────────┐
                          │  SIMD RUST   │
                          └──────┬───────┘
          ┌──────────────┬───────┼────────┬─────────────────┐
          ▼              ▼        ▼        ▼                 ▼
     ┌─────────┐   ┌─────────┐ ┌──────┐ ┌────────┐    ┌──────────┐
     │ PHẦN    │   │ DATA    │ │ AUTO │ │TƯỜNG   │    │ AN TOÀN  │
     │ CỨNG    │   │ LAYOUT  │ │ VEC  │ │MINH    │    │          │
     ├─────────┤   ├─────────┤ ├──────┤ ├────────┤    ├──────────┤
     │ lane    │   │AoS vs   │ │target│ │std::   │    │detect    │
     │ XMM/YMM │   │  SoA    │ │-cpu  │ │ simd   │    │ feature  │
     │ /ZMM    │   │contiguou│ │chunks│ │wide    │    │fallback  │
     │ SSE/AVX │   │ s       │ │_exact│ │core::  │    │bọc unsafe│
     │ /NEON   │   │ align   │ │đọc asm│ │ arch   │    │test vs   │
     │         │   │         │ │      │ │intrinsic│   │ scalar   │
     └─────────┘   └─────────┘ └──────┘ └────────┘    └──────────┘
          │              │        │        │                 │
          └──────────────┴────────┼────────┴─────────────────┘
                                   ▼
                   ┌───────────────────────────────────┐
                   │ ĐO → SoA → AUTO-VEC → std::simd →  │
                   │ core::arch.  Luôn fallback,        │
                   │ luôn đọc asm, luôn bọc unsafe.     │
                   └───────────────────────────────────┘
```

---

> Quay lại lý thuyết: [z-simd.md](./z-simd.md) · Liên quan: [k-performance.md](./k-performance.md) · [a-memory-model.md](./a-memory-model.md) · [x-data-layout-visual.md](./x-data-layout-visual.md) · [n-unsafe-rust.md](./n-unsafe-rust.md)
