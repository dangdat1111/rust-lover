# Memory Layout của Data Types — TOÀN BỘ qua HÌNH VẼ

> Bạn đã hiểu stack/heap tổng quát trong [a-memory-model](./a-memory-model.md).
> File này đi **một tầng sâu hơn**: *mỗi kiểu dữ liệu Rust nằm trong bộ nhớ chính xác như thế nào* — từng byte một.
> Mọi con số `size` / `align` trong file này được **verify bằng `rustc 1.95.0`** (chạy `std::mem::size_of` / `align_of`).
> Đọc tuần tự từ trên xuống — mỗi phần xây trên phần trước.

---

## Mục lục hình minh họa

**Phần I — Bản chất (nền tảng)**
1. [Memory layout là gì? Ba con số quyết định tất cả](#1-ba-con-số)
2. [Một biến sống ở đâu: Stack hay Heap](#2-stack-hay-heap)
3. [Số nguyên: byte, two's complement, endianness](#3-số-nguyên)
4. [float / bool / char / unit — những điều bất ngờ](#4-float-bool-char-unit)

**Phần II — Kết hợp & căn lề**
5. [Alignment & Padding — tại sao có lỗ trống](#5-alignment--padding)
6. [Tuple — bản chất là struct ẩn danh](#6-tuple)
7. [Array `[T; N]` — liền mạch](#7-array)
8. [Struct — compiler sắp xếp lại field](#8-struct)
9. [`#[repr(...)]` — điều khiển layout bằng tay](#9-repr)

**Phần III — Con trỏ & dữ liệu trên heap**
10. [Reference & raw pointer — thin pointer](#10-thin-pointer)
11. [Fat pointer — `&[T]`, `&str`, `&dyn Trait`](#11-fat-pointer)
12. [`Box<T>` — sở hữu một ô heap](#12-box)
13. [`Vec<T>` — ba từ máy, tự lớn](#13-vec)
14. [`String` vs `&str` vs `&String`](#14-string)

**Phần IV — Nâng cao**
15. [Enum — discriminant + variant lớn nhất](#15-enum)
16. [`Option<T>` & Niche Optimization](#16-niche)
17. [`Rc` / `Arc` — control block trên heap](#17-rc-arc)
18. [`Cell` / `RefCell` — cờ mượn nằm cạnh data](#18-cell-refcell)
19. [Zero-Sized Types (ZST) & `PhantomData`](#19-zst)
20. [Generic & Monomorphization — layout sinh lúc compile](#20-generic)

**Phần V — Tổng kết**
21. [Bảng tổng hợp size/align](#21-bảng-tổng-hợp)
22. [Cây quyết định: "kiểu này nằm đâu, bao nhiêu byte?"](#22-cây-quyết-định)
23. [Mind map tổng kết](#23-mind-map)

---

<a name="1-ba-con-số"></a>
## 1. Ba con số quyết định tất cả

RAM về bản chất chỉ là **một mảng byte khổng lồ**, mỗi byte có một địa chỉ (address):

```
 địa chỉ:  0      1      2      3      4      5      6      7   ...
         ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
   RAM:  │ byte │ byte │ byte │ byte │ byte │ byte │ byte │ byte │ ...
         └──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘
```

CPU không biết "i32", "String" hay "struct" là gì. Nó chỉ biết **đọc/ghi N byte tại địa chỉ X**.
Vậy nên **"memory layout"** của một kiểu chỉ là câu trả lời cho 3 câu hỏi:

```
┌─────────────────────────────────────────────────────────────┐
│  1. SIZE  (size_of)   → kiểu này chiếm BAO NHIÊU byte?        │
│                                                               │
│  2. ALIGN (align_of)  → địa chỉ bắt đầu phải CHIA HẾT cho?    │
│                          (vd align=4 ⇒ chỉ đặt được ở 0,4,8…) │
│                                                               │
│  3. OFFSET            → trong struct, field nằm ở byte thứ?   │
└─────────────────────────────────────────────────────────────┘
```

**Bản chất triết lý của Rust ở đây:** layout được tính **100% lúc compile** (với kiểu `Sized`).
Compiler biết chính xác mọi thứ → không cần header runtime mô tả kiểu như Java/Python.
Đây là gốc rễ của "zero-cost": một `i32` trong Rust = đúng 4 byte, **không một byte thừa** cho metadata.

```
   Java:  Integer  →  [ header 12-16B ][ giá trị 4B ]  + nằm trên heap, qua con trỏ
   Rust:  i32      →  [ giá trị 4B ]                    + nằm thẳng trên stack
                       ▲
                       └── KHÔNG có header, KHÔNG con trỏ. Chính nó là 4 byte.
```

> Quy ước trong cả file: **1 từ máy (word) = 8 byte** (hệ 64-bit). Ô `[....]` = 1 byte.

---

<a name="2-stack-hay-heap"></a>
## 2. Một biến sống ở đâu: Stack hay Heap

Câu hỏi đầu tiên với BẤT KỲ kiểu nào: phần dữ liệu nằm trên **stack** hay **heap**?

```
        STACK (nhanh, tự dọn)              HEAP (linh hoạt, phải xin/trả)
   ┌───────────────────────────┐      ┌───────────────────────────────┐
   │ • Kích thước biết lúc      │      │ • Kích thước biết lúc RUNTIME  │
   │   compile (Sized)          │      │ • Hoặc cần sống lâu hơn scope  │
   │ • i32, bool, char, f64     │      │ • Nội dung của: Box, Vec,      │
   │ • tuple, array, struct,    │      │   String, Rc, Arc...           │
   │   enum (nếu field Sized)   │      │                                │
   │ • CHÍNH các con trỏ        │─────▶│ • DỮ LIỆU mà con trỏ trỏ tới   │
   │   (Box/Vec/String header)  │ trỏ  │                                │
   └───────────────────────────┘      └───────────────────────────────┘
```

Điểm cực kỳ quan trọng và hay nhầm:

```
   let v: Vec<i32> = vec![10, 20, 30];

   STACK                              HEAP
   ┌──────────────────┐              ┌────┬────┬────┐
   │ v: Vec<i32>      │              │ 10 │ 20 │ 30 │
   │  ┌────────────┐  │   ptr        └────┴────┴────┘
   │  │ ptr ───────┼──┼─────────────▶ ▲
   │  │ cap = 3    │  │
   │  │ len = 3    │  │
   │  └────────────┘  │
   │   (24 byte)      │
   └──────────────────┘

   ⇒ "v" trên stack chỉ là một HEADER 24 byte (3 từ máy).
     Dữ liệu thật (10,20,30) nằm trên heap.
     size_of::<Vec<i32>>() == 24, BẤT KỂ Vec chứa 3 hay 3 triệu phần tử.
```

Đây là mô hình tư duy nền: **"kiểu cấp cao" trong Rust hầu hết là một header nhỏ trên stack trỏ tới dữ liệu trên heap.** Phần còn lại của file chỉ là vẽ ra từng loại header đó.

---

<a name="3-số-nguyên"></a>
## 3. Số nguyên: byte, two's complement, endianness

### 3.1 Họ số nguyên và kích thước

```
 Kiểu     bit   byte   align        Khoảng giá trị
 ─────────────────────────────────────────────────────────────
 i8/u8      8     1      1     i8: -128..127     u8: 0..255
 i16/u16   16     2      2
 i32/u32   32     4      4     ◀── mặc định cho số nguyên
 i64/u64   64     8      8
 i128/u128 128    16     16
 isize     —      8      8     ◀── = "1 từ máy", = kích thước con trỏ
 usize     —      8      8         (8 trên 64-bit, 4 trên 32-bit)
```

`usize` đặc biệt: nó **luôn đủ lớn để chứa một địa chỉ bộ nhớ**. Vì thế index mảng, `len()`, `cap()` đều dùng `usize` — không phải ngẫu nhiên.

### 3.2 Một byte trông như thế nào — `u8 = 217`

```
   217 trong hệ 10  =  1101 1001 (nhị phân)  =  0xD9

         bit:   7   6   5   4   3   2   1   0
              ┌───┬───┬───┬───┬───┬───┬───┬───┐
              │ 1 │ 1 │ 0 │ 1 │ 1 │ 0 │ 0 │ 1 │   ← 1 byte = 8 bit
              └───┴───┴───┴───┴───┴───┴───┴───┘
   trọng số:   128  64  32  16  8   4   2   1
              128 + 64 +     16 + 8 +         1  = 217 ✓
```

### 3.3 Số âm: Two's Complement (bù 2)

CPU không có "dấu trừ". Số âm được mã hóa bằng quy ước **bù 2**: bit cao nhất là "dấu", và `-x` được tính bằng `đảo tất cả bit rồi +1`.

```
   i8 = -1:
        +1   =  0000 0001
        đảo  =  1111 1110
        +1   =  1111 1111   =  0xFF   ⇒  -1 lưu là 1111_1111

   i8 = -128 (cực tiểu):  1000 0000  = 0x80
   i8 = +127 (cực đại):   0111 1111  = 0x7F

        1000 0000 ─────────────────────► 0111 1111
        -128                              +127
          ▲ bit 7 = 1 ⇒ âm        bit 7 = 0 ⇒ dương ▲
```

Vẻ đẹp của bù 2: phép cộng `5 + (-5)` dùng **cùng một mạch cộng** với số dương, kết quả tự tràn về 0. CPU không cần biết số có dấu hay không khi cộng.

### 3.4 Endianness — thứ tự byte trong RAM khi số > 1 byte

`u32 = 0x12345678` (4 byte) được xếp vào RAM theo thứ tự nào?

```
   Giá trị:  0x12345678   (12 = byte cao nhất, 78 = byte thấp nhất)

   địa chỉ:        +0     +1     +2     +3
                ┌──────┬──────┬──────┬──────┐
 Little-Endian: │  78  │  56  │  34  │  12  │  ◀── x86, ARM, RISC-V, đa số
   (byte thấp   └──────┴──────┴──────┴──────┘      "ngược" — byte nhỏ trước
    trước)

                ┌──────┬──────┬──────┬──────┐
 Big-Endian:    │  12  │  34  │  56  │  78  │  ◀── network order, một số mips
   (byte cao    └──────┴──────┴──────┴──────┘      "xuôi" như ta viết
    trước)
```

Điều này **chỉ quan trọng** khi bạn: ghi số ra file/network, hoặc cast con trỏ qua `*const u8`. Trong code Rust thường ngày, `a + b` luôn cho kết quả đúng bất kể endianness — vì CPU lo phần đó. Khi cần kiểm soát, dùng `to_le_bytes()` / `to_be_bytes()`.

---

<a name="4-float-bool-char-unit"></a>
## 4. float / bool / char / unit — những điều bất ngờ

### 4.1 `f32` / `f64` — IEEE 754

Số thực không lưu "phần nguyên . phần thập phân". Nó lưu dạng khoa học nhị phân: `dấu × 1.mantissa × 2^exponent`.

```
   f32 (4 byte = 32 bit):
   ┌─┬────────────┬───────────────────────────┐
   │S│  exponent  │         mantissa          │
   │1│   8 bit    │          23 bit           │
   └─┴────────────┴───────────────────────────┘
    ▲     ▲                  ▲
    │     │                  └ phần định trị (độ chính xác)
    │     └ số mũ (độ lớn, có bias 127)
    └ dấu (0=+, 1=−)

   f64 (8 byte):  S=1 bit, exponent=11 bit, mantissa=52 bit
```

Hệ quả thực tế của layout này: `0.1 + 0.2 != 0.3` — vì `0.1` không biểu diễn chính xác được trong nhị phân, y như `1/3` không viết hết trong hệ 10.

### 4.2 `bool` — 1 byte, không phải 1 bit!

```
   size_of::<bool>() == 1   (KHÔNG phải 1 bit)

   false = 0x00 = 0000 0000
   true  = 0x01 = 0000 0001
                        ▲
   Chỉ 0 và 1 là HỢP LỆ. Các giá trị 2..255 là "niche" (lỗ trống)
   → ghi nhớ điều này, phần 16 (Option) sẽ tận dụng nó!
```

Vì sao 1 byte mà không 1 bit? Vì **địa chỉ nhỏ nhất CPU đánh được là 1 byte** — không có "địa chỉ của 1 bit". Đánh đổi 7 bit để lấy khả năng tham chiếu `&bool`.

### 4.3 `char` — 4 byte, KHÔNG phải 1 byte!

Đây là chỗ dân từ C hay sốc nhất:

```
   size_of::<char>() == 4   (C: char = 1 byte; Rust: char = 4 byte!)

   char Rust = MỘT Unicode Scalar Value, khoảng 0x0..=0x10FFFF
               (trừ vùng surrogate 0xD800..=0xDFFF)

   'A'  = U+0041   →  ┌────┬────┬────┬────┐
                      │ 41 │ 00 │ 00 │ 00 │  (4 byte, little-endian)
                      └────┴────┴────┴────┘
   '🦀' = U+1F980  →  ┌────┬────┬────┬────┐
                      │ 80 │ F9 │ 01 │ 00 │  vẫn vừa trong 4 byte
                      └────┴────┴────┴────┘
```

**Bản chất phân biệt cực quan trọng:**

```
   char           = 1 ký tự Unicode = LUÔN 4 byte (cố định)
   str / String   = chuỗi UTF-8     = ký tự dài 1..4 byte (biến thiên)

   "A🦀" trong String (UTF-8):
   ┌────┬────┬────┬────┬────┐
   │ 41 │ F0 │ 9F │ A6 │ 80 │  ← 'A' tốn 1 byte, '🦀' tốn 4 byte → tổng 5 byte
   └────┴────┴────┴────┴────┘
     'A'  └──── '🦀' ────┘
```

→ Đây là lý do `String` **không cho index bằng `s[0]`**: byte thứ 0 có thể là nửa của một ký tự!

### 4.4 `()` unit — Zero-Sized Type đầu tiên

```
   size_of::<()>() == 0    ← chiếm 0 byte!
   align_of::<()>() == 1

   ()  →  (không có byte nào cả)
```

Một kiểu **chiếm 0 byte** là hợp lệ trong Rust. `()` mang ý nghĩa "không có thông tin gì". `fn` không trả về gì thực ra trả về `()`. Ta đào sâu ZST ở [phần 19](#19-zst).

---

<a name="5-alignment--padding"></a>
## 5. Alignment & Padding — tại sao có lỗ trống

### 5.1 Vì sao tồn tại alignment?

CPU không đọc RAM từng byte một. Nó đọc theo **khối thẳng hàng** (vd 8 byte mỗi lần). Nếu một `u32` (4 byte) nằm vắt qua ranh giới khối, CPU phải đọc **2 lần** rồi ghép lại — chậm, có kiến trúc còn crash.

```
   ĐỌC u32 tại offset 4 (thẳng hàng, align=4):     ✓ 1 lần đọc
   khối:  [0 1 2 3][4 5 6 7]
                    └─u32─┘     nằm gọn trong 1 khối

   ĐỌC u32 tại offset 6 (lệch hàng):               ✗ 2 lần đọc
   khối:  [0 1 2 3][4 5 6 7][8 9 ...]
                        └──u32──┘     vắt qua ranh giới khối!
```

→ **Quy tắc:** một kiểu align=N **phải** bắt đầu ở địa chỉ chia hết cho N. Compiler đảm bảo điều này bằng cách chèn **padding** (byte đệm).

### 5.2 Padding sinh ra như thế nào

```
   struct Mixed { a: u8, b: u32 }   // ngây thơ tưởng = 5 byte

   Nhưng b cần align=4 ⇒ b phải ở offset chia hết cho 4:

   offset:  0    1    2    3    4    5    6    7
          ┌────┬────┬────┬────┬────┬────┬────┬────┐
          │ a  │ ?? │ ?? │ ?? │ b  │ b  │ b  │ b  │
          └────┴────┴────┴────┴────┴────┴────┴────┘
            ▲    └──padding──┘   └──── b (u32) ────┘
            a (1B)  3 byte phí

   size = 8, align = 4   (KHÔNG phải 5!)
```

### 5.3 Tail padding — đệm ở cuối để array hoạt động

```
   struct S { a: u32, b: u8 }

   offset:  0    1    2    3    4    5    6    7
          ┌────┬────┬────┬────┬────┬────┬────┬────┐
          │ a  │ a  │ a  │ a  │ b  │ ?? │ ?? │ ?? │
          └────┴────┴────┴────┴────┴────┴────┴────┘
                                  └── tail padding ──┘
   size = 8 (bội số của align=4), không phải 5.
```

Vì sao đệm ở cuối? Để khi xếp `[S; 2]` thành mảng, phần tử thứ 2 vẫn thẳng hàng (`a` của nó ở offset 8, chia hết cho 4). **Quy tắc vàng: `size` luôn là bội số của `align`.**

---

<a name="6-tuple"></a>
## 6. Tuple — bản chất là struct ẩn danh

Tuple xếp các field cạnh nhau, **tuân theo cùng luật align/padding như struct**:

```
   (i32, u8)                size = 8, align = 4   (đã verify)

   offset:  0    1    2    3    4    5    6    7
          ┌────┬────┬────┬────┬────┬────┬────┬────┐
          │   .0 (i32)        │ .1 │ pad pad pad  │
          └────┴────┴────┴────┴────┴────┴────┴────┘

   (u8, u8, u8)             size = 3, align = 1   (không cần padding)
          ┌────┬────┬────┐
          │ .0 │ .1 │ .2 │
          └────┴────┴────┘

   ()  (empty tuple)        size = 0   ← chính là unit type ở phần 4.4
```

Lưu ý: với tuple, compiler **cũng được phép sắp xếp lại** field như struct (xem phần 8) — thứ tự trong bộ nhớ không nhất thiết khớp `.0 .1 .2`.

---

<a name="7-array"></a>
## 7. Array `[T; N]` — liền mạch

Mảng là **N phần tử cùng kiểu nằm sát nhau, không header**:

```
   [i32; 4]                 size = 16 (= 4 × 4), align = 4

   offset:  0      4      8      12
          ┌──────┬──────┬──────┬──────┐
          │ [0]  │ [1]  │ [2]  │ [3]  │   mỗi ô = i32 (4 byte)
          └──────┴──────┴──────┴──────┘
          địa chỉ phần tử [i] = base + i × size_of::<i32>()
                                      = base + i × 4
```

```
   [u8; 5]                  size = 5, align = 1
          ┌────┬────┬────┬────┬────┐
          │[0] │[1] │[2] │[3] │[4] │
          └────┴────┴────┴────┴────┘
```

**Điểm cốt lõi:** `N` là **một phần của kiểu**, biết lúc compile → array nằm gọn trên **stack**, truy cập `arr[i]` chỉ là một phép nhân + cộng. Đây là cấu trúc dữ liệu thân thiện cache nhất (mọi phần tử liền kề).

So sánh nhanh với "slice" `[T]` (không có N) — đó là kiểu **không Sized**, sẽ gặp ở phần 11.

---

<a name="8-struct"></a>
## 8. Struct — compiler sắp xếp lại field

Đây là phần "wow" của Rust. Mặc định (`repr(Rust)`), compiler được **tự do hoán đổi thứ tự field** để giảm padding!

```
   struct Foo {
       a: u8,    // align 1
       b: u32,   // align 4
       c: u16,   // align 2
   }
```

**Nếu giữ nguyên thứ tự (cách C làm) → tốn 12 byte:**

```
   offset:  0    1    2    3    4    5    6    7    8    9   10   11
          ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
          │ a  │pad │pad │pad │   b (u32)         │  c (u16) │pad │pad │
          └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘
            ▲    └─ 3 phí ─┘                       ▲          └ 2 phí ┘
          a ở 0, b cần align 4 nên nhảy tới 4, c ở 8...   size = 12 😞
```

**Rust mặc định sắp xếp lại: b (lớn nhất) → c → a → tốn 8 byte:**

```
   thứ tự bộ nhớ thật:  b, c, a
   offset:  0    1    2    3    4    5    6    7
          ┌────┬────┬────┬────┬────┬────┬────┬────┐
          │   b (u32)         │  c (u16) │ a  │pad │
          └────┴────┴────┴────┴────┴────┴────┴────┘
                                            ▲    ▲
                                          a (1B) chỉ 1 byte phí
   size = 8 ✓  (đã verify: struct AB → size=8, repr(C) → size=12)
```

> **Bản chất:** vì Rust **không hứa** thứ tự field trong bộ nhớ (khác C), compiler tự do tối ưu. Bạn vẫn viết `foo.a`, `foo.b` bình thường — compiler dịch tên field thành offset thật. Bạn được sự tiện lợi, compiler lo việc nén.

Quy trình compiler dùng (đơn giản hóa): **sắp field theo align giảm dần**, rồi đặt liên tiếp. Nhờ vậy field to (align cao) đứng trước, field nhỏ "lấp khe".

---

<a name="9-repr"></a>
## 9. `#[repr(...)]` — điều khiển layout bằng tay

Khi nào cần can thiệp? Khi nói chuyện với C (FFI), với phần cứng, với network. Bốn `repr` chính:

```
 ┌──────────────────┬──────────────────────────────────────────────┐
 │ #[repr(Rust)]    │ MẶC ĐỊNH. Compiler hoán field tự do, tối ưu   │
 │ (không ghi gì)   │ size. KHÔNG ổn định giữa các bản compile.       │
 ├──────────────────┼──────────────────────────────────────────────┤
 │ #[repr(C)]       │ Giữ NGUYÊN thứ tự khai báo, layout giống C.    │
 │                  │ Dùng cho FFI. Có thể tốn padding hơn.          │
 ├──────────────────┼──────────────────────────────────────────────┤
 │ #[repr(packed)]  │ align = 1, BỎ HẾT padding. size nhỏ nhất       │
 │                  │ nhưng truy cập field có thể chậm/UB nếu &.     │
 ├──────────────────┼──────────────────────────────────────────────┤
 │ #[repr(align(N))]│ ÉP align tối thiểu = N (vd cache-line 64B).    │
 │ #[repr(transparent)] dùng cho newtype 1 field: layout = field đó.│
 └──────────────────┴──────────────────────────────────────────────┘
```

Cùng struct `{ a: u8, b: u32, c: u16 }` qua 3 repr (đã verify):

```
   repr(Rust)    size =  8   ┌b───────┐┌c───┐a▓        (hoán + nén)
   repr(C)       size = 12   a▓▓▓┌b───────┐┌c───┐▓▓     (giữ thứ tự)
   repr(packed)  size =  7   a┌b───────┐┌c───┐          (không padding)
                                 ▲
                  ▓ = padding   packed: b nằm ở offset 1 (lệch hàng!)
                                → đọc b qua &b là UB; phải copy ra trước.
```

**Quy tắc thực dụng:** mặc định cứ để `repr(Rust)` (nhanh + gọn). Chỉ thêm `repr(C)` khi field phải khớp byte với thế giới bên ngoài (struct C, gói tin mạng, thanh ghi thiết bị).

---

<a name="10-thin-pointer"></a>
## 10. Reference & raw pointer — thin pointer

Con trỏ "gầy" (thin) = **đúng 1 từ máy = 8 byte**, chứa một địa chỉ:

```
   &T, &mut T, *const T, *mut T, Box<T>   →  size = 8, align = 8 (với T: Sized)

   let x: i32 = 42;
   let r: &i32 = &x;

   STACK
   ┌──────────────────┐
   │ x: i32   = 42    │ ◀──────────┐
   ├──────────────────┤            │ địa chỉ của x
   │ r: &i32          │            │
   │   = 0x7ffe...0c ─┼────────────┘
   │   (8 byte)       │
   └──────────────────┘

   ⇒ "r" chỉ là một số 8 byte: địa chỉ của x. Không hơn.
```

Tất cả 4 loại con trỏ tới kiểu `Sized` đều **giống hệt nhau trong bộ nhớ** (8 byte địa chỉ). Khác biệt giữa `&` / `&mut` / `*const` / `*mut` chỉ là **luật compile-time** (ai được đọc/ghi), **không phải** khác biệt bộ nhớ runtime.

```
   &T       : đọc, chia sẻ được nhiều       ┐
   &mut T   : đọc+ghi, độc quyền             │ cùng 8 byte runtime,
   *const T : raw, cần unsafe để deref       │ khác nhau ở luật compiler
   *mut T   : raw mutable, cần unsafe        ┘
```

---

<a name="11-fat-pointer"></a>
## 11. Fat pointer — `&[T]`, `&str`, `&dyn Trait`

Khi trỏ tới kiểu **không Sized** (size không biết lúc compile), 8 byte địa chỉ là **không đủ** — cần thêm metadata. Đó là con trỏ "béo" (fat) = **2 từ máy = 16 byte**.

### 11.1 Slice `&[T]` và `&str` — (ptr, len)

```
   &[T]   →  size = 16   (ptr 8 + len 8)        đã verify: &[u8] = 16
   &str   →  size = 16   (ptr 8 + byte-len 8)   đã verify: &str  = 16

   let v = vec![10, 20, 30, 40, 50];
   let s: &[i32] = &v[1..4];        // mượn 3 phần tử giữa

   STACK (s, fat pointer)            HEAP (data của Vec)
   ┌──────────────────┐            ┌────┬────┬────┬────┬────┐
   │ s: &[i32]        │            │ 10 │ 20 │ 30 │ 40 │ 50 │
   │  ┌────────────┐  │  ptr       └────┴────┴────┴────┴────┘
   │  │ ptr ───────┼──┼──────────────────▲
   │  │ len = 3    │  │                   └ trỏ vào phần tử [1]
   │  └────────────┘  │
   │   (16 byte)      │   ⇒ len đi KÈM con trỏ ⇒ biết slice dừng ở đâu
   └──────────────────┘     mà KHÔNG cần đọc gì trên heap.
```

`&str` y hệt, chỉ khác `len` là **số byte UTF-8** (không phải số ký tự):

```
   let s: &str = "A🦀";
   ┌────────────┐         HEAP/static
   │ ptr ───────┼────────▶ ┌────┬────┬────┬────┬────┐
   │ len = 5    │          │ 41 │ F0 │ 9F │ A6 │ 80 │  (5 byte UTF-8)
   └────────────┘          └────┴────┴────┴────┴────┘
                            len=5 dù chỉ 2 "ký tự"
```

### 11.2 Trait object `&dyn Trait` — (ptr, vtable)

```
   &dyn Trait   →  size = 16   (ptr tới data 8 + ptr tới vtable 8)

   let s: &dyn Draw = &circle;

   STACK                          HEAP/stack: data        STATIC: vtable của Circle
   ┌──────────────────┐          ┌──────────────┐        ┌─────────────────────┐
   │ s: &dyn Draw     │   ptr    │ circle:Circle│        │ size, align          │
   │  ┌────────────┐  │ ────────▶│  (các field) │        │ drop_fn              │
   │  │ data ptr ──┼──┼─────────▶└──────────────┘        │ draw()  ─► Circle::… │
   │  │ vtbl ptr ──┼──┼──────────────────────────────────▶│ area()  ─► Circle::… │
   │  └────────────┘  │                                   └─────────────────────┘
   │   (16 byte)      │   ⇒ vtable = "bảng tra hàm" để gọi đúng phiên bản method
   └──────────────────┘     lúc runtime (dynamic dispatch).
```

→ Xem sâu hơn vtable ở [c-trait](./c-trait.md). Điểm cần nhớ tại đây: **không Sized ⇒ con trỏ 16 byte**, phần thừa là metadata (len hoặc vtable).

---

<a name="12-box"></a>
## 12. `Box<T>` — sở hữu một ô heap

`Box<T>` = con trỏ thin (8 byte) **sở hữu** dữ liệu trên heap. Khác `&T` ở chỗ: khi `Box` bị drop, nó **giải phóng** vùng heap.

```
   let b: Box<i32> = Box::new(42);

   STACK                  HEAP
   ┌────────────────┐    ┌──────┐
   │ b: Box<i32>    │    │  42  │
   │  ptr ──────────┼───▶└──────┘
   │  (8 byte)      │     (4 byte, được Box sở hữu)
   └────────────────┘
   khi b ra khỏi scope → tự free ô heap 42.
```

Công dụng layout điển hình: **biến kiểu không-Sized hoặc khổng lồ thành 8 byte trên stack.**

```
   Recursive type — KHÔNG Box thì size vô hạn:

   enum List { Cons(i32, List), Nil }    // ❌ List chứa List chứa List... ∞

   enum List { Cons(i32, Box<List>), Nil } // ✓ Box<List> = 8 byte cố định
                            ▲
   ┌─ Cons ─────────────┐   └ phần đuôi nằm trên heap, stack chỉ giữ 8B
   │ i32 │ Box ─────────┼──▶┌─ Cons ──────────┐
   └─────┴──────────────┘   │ i32 │ Box ──────┼──▶ ... ─▶ Nil
                            └─────┴───────────┘
```

`Box<T>` với `T: Sized` là thin pointer (8B). Nhưng `Box<dyn Trait>` hay `Box<[T]>` là **fat pointer (16B)** — vì nó box một kiểu không-Sized.

---

<a name="13-vec"></a>
## 13. `Vec<T>` — ba từ máy, tự lớn

`Vec<T>` header trên stack = **3 từ máy = 24 byte**: `ptr`, `cap` (sức chứa), `len` (đang dùng).

```
   Vec<T>   →  size = 24   (đã verify: Vec<u8> = 24)

   let mut v = Vec::with_capacity(4);
   v.push(10); v.push(20); v.push(30);   // len=3, cap=4

   STACK                          HEAP (cap=4 ô đã cấp)
   ┌──────────────────┐          ┌────┬────┬────┬─────┐
   │ v: Vec<i32>      │   ptr     │ 10 │ 20 │ 30 │ ??? │
   │  ┌────────────┐  │ ─────────▶└────┴────┴────┴─────┘
   │  │ ptr        │  │            └─ len=3 ─┘   └ ô thừa
   │  │ cap = 4    │  │                            (cap−len)
   │  │ len = 3    │  │
   │  └────────────┘  │   • len = số phần tử ĐANG có
   │   (24 byte)      │   • cap = số ô ĐÃ cấp (để push không phải xin lại liền)
   └──────────────────┘
```

Khi `push` mà `len == cap` → Vec **cấp vùng heap mới to gấp đôi**, copy sang, free vùng cũ:

```
   cap=4, len=4, push thêm:
   cũ:  [a][b][c][d]                (đầy)
   mới: [a][b][c][d][e][ ][ ][ ]    cap=8  ← cấp mới, copy 4 phần tử, e vào
        └────── copy ──────┘
   ptr trong header được cập nhật trỏ sang vùng mới. Vùng cũ được free.
```

→ Đây là vì sao `&v[0]` có thể "hỏng" sau khi push: địa chỉ thay đổi. Borrow checker chặn đúng tình huống này lúc compile.

Thứ tự 3 field (`ptr/cap/len`) **không** thuộc ABI ổn định — đừng dựa vào nó; chỉ cần nhớ tổng = 24 byte.

---

<a name="14-string"></a>
## 14. `String` vs `&str` vs `&String`

`String` về bản chất là **`Vec<u8>` đảm bảo nội dung là UTF-8 hợp lệ**. Cùng header 24 byte.

```
   String  →  size = 24  (ptr + cap + len, y như Vec<u8>)

   let s: String = String::from("Rust");

   STACK                          HEAP
   ┌──────────────────┐          ┌────┬────┬────┬────┐
   │ s: String        │   ptr     │ R  │ u  │ s  │ t  │  (UTF-8 byte)
   │  ┌────────────┐  │ ─────────▶└────┴────┴────┴────┘
   │  │ ptr        │  │            52   75   73   74
   │  │ cap = 4    │  │
   │  │ len = 4    │  │   len/cap tính bằng BYTE, không phải ký tự.
   │  └────────────┘  │
   │   (24 byte)      │
   └──────────────────┘
```

### Bộ ba dễ lẫn — vẽ cạnh nhau

```
   String       : OWNED, có thể lớn lên     header 24B (ptr,cap,len) → heap
   &str         : MƯỢN, chỉ đọc, cố định    fat ptr 16B (ptr,len)    → đâu đó
   &String      : mượn cái header           thin ptr 8B              → String

   let s: String = String::from("Rust");
   let a: &str    = &s;       //  fat 16B, trỏ thẳng vào byte trên heap
   let b: &String = &s;       //  thin 8B, trỏ vào HEADER của s

        b (8B)            s: String (24B header)        HEAP
       ┌──────┐          ┌──────────────────┐          ┌───────────┐
       │ ptr ─┼─────────▶│ ptr ─────────────┼─────────▶│ R u s t   │
       └──────┘          │ cap=4 / len=4    │      ▲    └───────────┘
                         └──────────────────┘      │
       a (16B)           ┌──────┬──────┐            │
                         │ ptr ─┼──────┼────────────┘  (bỏ qua header,
                         │ len=4│      │                trỏ thẳng vào byte)
                         └──────┴──────┘
```

**Bản chất API:** hàm nên nhận `&str` (linh hoạt — nhận được cả `String` lẫn literal), trả về `String` khi cần sở hữu. `&str` là "view chỉ đọc lên byte UTF-8", `String` là "chủ sở hữu buffer co giãn được".

---

<a name="15-enum"></a>
## 15. Enum — discriminant + variant lớn nhất

Enum (kiểu tổng / tagged union) lưu: **một "tag" (discriminant) cho biết đang là variant nào** + **đủ chỗ cho variant lớn nhất**.

```
   enum Shape {
       Circle(f64),            // cần 8 byte (1 f64)
       Rect(f64, f64),         // cần 16 byte (2 f64)  ← LỚN NHẤT
       Point,                  // cần 0 byte
   }
```

```
   Layout = [ tag ][ padding ][ ──── chỗ cho variant lớn nhất ──── ]

   ┌──────┬─────────┬──────────────────┬──────────────────┐
   │ tag  │ padding │      slot 0       │      slot 1       │
   │ 8B*  │         │      (f64)        │      (f64)        │
   └──────┴─────────┴──────────────────┴──────────────────┘
    tag=0 ⇒ Circle: dùng slot 0, bỏ slot 1
    tag=1 ⇒ Rect  : dùng cả slot 0 và slot 1
    tag=2 ⇒ Point : không dùng slot nào

   * tag thường 1 byte về mặt logic, nhưng do align của f64 (=8),
     compiler đệm để slot dữ liệu thẳng hàng. size_of::<Shape>() = 24.

   ⇒ Enum to bằng (variant lớn nhất + tag + padding), KỂ CẢ khi
     đang giữ variant nhỏ. Đây là "chi phí variant lớn".
```

Mẹo tối ưu: nếu một variant **rất to** so với phần còn lại, `Box` nó lại để mọi variant chỉ tốn 8 byte con trỏ:

```
   enum Msg { Small(u8), Huge([u8; 1000]) }       // size ≈ 1001 byte 😱
   enum Msg { Small(u8), Huge(Box<[u8; 1000]>) }  // size ≈ 16 byte ✓
                              ▲ data 1000B dời sang heap
```

`Result<T, E>` cũng là enum hai variant (`Ok(T)` / `Err(E)`) — cùng cơ chế: `size = tag + max(size T, size E)`. Đã verify `Result<i32,u8>` = 8.

---

<a name="16-niche"></a>
## 16. `Option<T>` & Niche Optimization

`Option<T>` = enum `Some(T)` / `None`. Ngây thơ thì cần thêm 1 tag. Nhưng Rust có **phép màu**: nếu `T` có sẵn "lỗ trống" (niche — giá trị bit không bao giờ hợp lệ), Rust **dùng lỗ đó làm tag** → `None` **miễn phí**!

### 16.1 Khi T KHÔNG có niche — cần tag thật

```
   Option<i32>   →  size = 8  (i32=4 + tag 1 + pad 3)   đã verify
   Option<u8>    →  size = 2  (u8=1 + tag 1)             đã verify

   mọi bit pattern của i32 đều hợp lệ ⇒ không có giá trị nào "dư" để làm None
   ⇒ phải thêm 1 byte tag riêng:

   Some(7):  ┌────┬───────────┐        None:  ┌────┬───────────┐
             │ 01 │  7 (i32)  │               │ 00 │  ?? rác   │
             └────┴───────────┘               └────┴───────────┘
              tag                              tag=0 ⇒ bỏ qua data
```

### 16.2 Khi T CÓ niche — None miễn phí (Niche Optimization)

```
   Option<bool>  →  size = 1   (KHÔNG to hơn bool!)        đã verify
   Option<char>  →  size = 4   (= char)                    đã verify
   Option<&T>    →  size = 8   (= &T, KHÔNG 16!)            đã verify
   Option<Box<T>>→  size = 8   (= Box<T>)                  đã verify
   Option<NonZeroU32> → size = 4 (= u32)                   đã verify

   Ví dụ Option<bool>: bool chỉ dùng 0,1 — giá trị 2..255 là lỗ trống.
   Rust gán: None = 2.

       false  = 0x00
       true   = 0x01
       None   = 0x02     ◀── dùng một giá trị "không thể có của bool" làm None!
       ┌────┐
       │ 02 │  = None, vẫn chỉ 1 byte
       └────┘
```

### 16.3 Niche kinh điển: `Option<&T>` dùng địa chỉ NULL

```
   Reference Rust KHÔNG BAO GIỜ null. ⇒ địa chỉ 0 là lỗ trống.
   Rust gán: None = địa chỉ 0.

   Some(&x):  ┌──────────────────┐      None:  ┌──────────────────┐
              │ 0x7ffe...  (≠0)  │             │ 0x0000000000000  │
              └──────────────────┘             └──────────────────┘
               địa chỉ thật                     địa chỉ 0 = None

   ⇒ Option<&T> = 8 byte, GIỐNG HỆT con trỏ trần của C.
     "Con trỏ có thể null" của C = Option<&T> an toàn của Rust,
     CÙNG layout, KHÔNG tốn thêm byte nào. Đây là zero-cost ở mức tuyệt đối.
```

> **Bản chất triết lý:** Rust biến "tính an toàn" thành **miễn phí** bằng cách tận dụng cấu trúc bit sẵn có. `Option<Box<T>>`, `Option<&T>`, `Option<NonNull>` đều = 8 byte. Đây là lý do người ta nói "dùng `Option` đi, không tốn gì đâu" — đúng nghĩa đen.

---

<a name="17-rc-arc"></a>
## 17. `Rc` / `Arc` — control block trên heap

`Rc<T>` (đơn luồng) / `Arc<T>` (đa luồng, atomic) cho phép **nhiều chủ sở hữu**. Header trên stack chỉ là **1 con trỏ (8 byte)**, nhưng vùng heap chứa thêm **bộ đếm tham chiếu**.

```
   Rc<T>, Arc<T>  →  size = 8   (đã verify) — chỉ 1 con trỏ!

   let a = Rc::new(42);
   let b = Rc::clone(&a);   // KHÔNG copy 42, chỉ tăng strong count
   let c = Rc::clone(&a);

   STACK                         HEAP — "RcBox" (control block + data)
   ┌──────────┐                 ┌──────────────────────────────┐
   │ a: ptr ──┼────┐            │ strong = 3   ◀── đếm Rc đang sống │
   ├──────────┤    │            │ weak   = 0   ◀── đếm Weak         │
   │ b: ptr ──┼────┼───────────▶│ ──────────────────────────────│
   ├──────────┤    │            │ value  = 42  ◀── DỮ LIỆU thật    │
   │ c: ptr ──┼────┘            └──────────────────────────────┘
   └──────────┘     cả a,b,c trỏ CÙNG control block

   • Rc::clone → strong += 1  (rẻ, chỉ +1 số đếm, KHÔNG copy value)
   • drop      → strong -= 1; khi strong == 0 → drop value
                 khi cả strong==0 && weak==0 → free luôn control block
```

Khác biệt Rc vs Arc nằm ở **kiểu bộ đếm**, không phải layout:

```
   Rc<T>  : strong/weak = usize thường       → nhanh, KHÔNG thread-safe
   Arc<T> : strong/weak = AtomicUsize        → +1/−1 nguyên tử, gửi qua thread được
            (cùng layout 8B header + control block; chỉ phép tăng/giảm khác)
```

`Weak<T>` trỏ cùng control block nhưng **không giữ value sống** (chỉ tăng `weak`), dùng để cắt vòng tham chiếu (cycle). Chi tiết ở [i-smart-pointers](./i-smart-pointers.md).

---

<a name="18-cell-refcell"></a>
## 18. `Cell` / `RefCell` — cờ mượn nằm cạnh data

Hai kiểu này cho "interior mutability" (sửa qua `&` thay vì `&mut`). Layout của chúng cho thấy **chi phí của việc dời kiểm tra mượn từ compile-time sang runtime**.

```
   Cell<T>     →  size = size_of::<T>()   (KHÔNG thêm byte!)
   ┌──────────────┐
   │  value: T    │   chỉ là một ô T, nhưng cho phép get/set qua &self
   └──────────────┘     (cơ chế: copy ra/ghi vào, không trả &mut → an toàn)

   RefCell<T>  →  size = size_of::<T>() + 8  (thêm 1 cờ borrow)
   ┌──────────────┬──────────────┐
   │  borrow flag │   value: T   │
   │  (isize)     │              │
   └──────────────┴──────────────┘
        ▲
        │  0      = chưa ai mượn
        │  n > 0  = đang có n shared borrow (&)
        │  −1     = đang có 1 mut borrow (&mut)
        └ borrow()/borrow_mut() KIỂM TRA cờ này lúc RUNTIME → panic nếu vi phạm
```

```
   So sánh bản chất:
   ┌──────────────┬────────────────────────┬────────────────────┐
   │              │ kiểm tra borrow ở đâu  │ chi phí layout     │
   ├──────────────┼────────────────────────┼────────────────────┤
   │ &T / &mut T  │ COMPILE-time (miễn phí)│ 0 byte             │
   │ RefCell<T>   │ RUNTIME (có thể panic) │ +8 byte cờ + check │
   └──────────────┴────────────────────────┴────────────────────┘
   ⇒ RefCell = "đánh đổi 8 byte + kiểm tra runtime để lách borrow checker".
     Chỉ dùng khi thật cần. Ghép Rc<RefCell<T>> = chia sẻ + sửa được (single-thread).
```

---

<a name="19-zst"></a>
## 19. Zero-Sized Types (ZST) — kiểu chiếm 0 byte

Rust cho phép kiểu có `size == 0`. Chúng **tồn tại lúc compile** nhưng **biến mất hoàn toàn lúc runtime**.

```
   size = 0 với:
   ┌─────────────────────────────────────────────────────────┐
   │ ()                       unit type                        │
   │ struct Unit;             unit struct (không field)        │
   │ struct Marker;           dùng làm "nhãn" type-state       │
   │ PhantomData<T>           "giả vờ chứa T" cho generic       │
   │ [u8; 0]                  mảng rỗng                         │
   └─────────────────────────────────────────────────────────┘
```

### ZST trong collection — phép màu `HashSet<K>` = `HashMap<K, ()>`

```
   HashSet<String>  thực chất là  HashMap<String, ()>
                                                   ▲
                              value () chiếm 0 byte ⇒ Set "free" phần value.

   Vec<()> với 1 triệu phần tử:
   ┌──────────────────┐
   │ ptr (dangling)   │   KHÔNG cấp heap nào cả!
   │ cap = ...        │   len chỉ là một bộ đếm.
   │ len = 1_000_000  │   Lặp 1 triệu lần "phần tử ()" mà tốn 0 byte data.
   └──────────────────┘
```

### `PhantomData` — đánh dấu mà không tốn chỗ

```
   struct Meters(f64);    // muốn phân biệt Meters vs Feet ở compile-time
   struct Wrapper<T> {
       value: f64,
       _marker: PhantomData<T>,   // 0 byte — chỉ để compiler "nhớ" T là gì
   }
   ⇒ size_of::<Wrapper<Meters>>() == 8 (chỉ riêng f64).
     PhantomData cho ta an toàn kiểu (type-state) với chi phí bộ nhớ = 0.
```

→ ZST là biểu hiện thuần khiết nhất của "zero-cost abstraction": thông tin tồn tại cho compiler kiểm tra, runtime không trả một byte nào.

---

<a name="20-generic"></a>
## 20. Generic & Monomorphization — layout sinh lúc compile

Khi viết `Vec<T>` generic, layout cụ thể **chưa tồn tại** cho tới khi bạn dùng với một `T` cụ thể. Compiler **monomorphize**: với mỗi `T` dùng thật, nó sinh ra một phiên bản code + layout riêng.

```
   Bạn viết MỘT lần:                Compiler sinh ra NHIỀU bản:
   ┌──────────────────┐            ┌──────────────────────────────┐
   │ struct Pair<T> {  │            │ Pair_i32  { a: i32, b: i32 }  │ size 8
   │   a: T, b: T,     │   ────────▶│ Pair_f64  { a: f64, b: f64 }  │ size 16
   │ }                 │   dùng     │ Pair_u8   { a: u8,  b: u8  }  │ size 2
   └──────────────────┘ Pair<i32>, └──────────────────────────────┘
                        Pair<f64>,    mỗi bản có layout RIÊNG, tối ưu riêng
                        Pair<u8>
```

```
   ⇒ Generic trong Rust = ZERO-COST. Pair<i32> nhanh và gọn y như
     khi bạn viết tay struct chỉ chứa i32. Không có "boxing", không
     có con trỏ ẩn, không kiểm tra kiểu lúc runtime (khác Java generics
     xóa kiểu / Python kiểu động).

   Đánh đổi: code phình ra (code bloat) — mỗi T một bản máy.
            Muốn 1 bản dùng chung → dùng dyn Trait (fat pointer, phần 11).
```

```
   Hai con đường, hai layout:
   ┌─────────────────────┬──────────────────────┬─────────────────────┐
   │                     │ static (generic <T>) │ dynamic (dyn Trait) │
   ├─────────────────────┼──────────────────────┼─────────────────────┤
   │ layout              │ riêng mỗi T, gọn      │ chung, qua fat ptr  │
   │ dispatch            │ compile-time (inline) │ runtime (vtable)    │
   │ kích thước nhị phân │ to (nhiều bản)        │ nhỏ (1 bản)         │
   │ tốc độ gọi          │ nhanh nhất            │ thêm 1 lần tra bảng │
   └─────────────────────┴──────────────────────┴─────────────────────┘
```

---

<a name="21-bảng-tổng-hợp"></a>
## 21. Bảng tổng hợp size/align (verify bằng rustc 1.95.0, 64-bit)

```
 ┌────────────────────┬──────┬───────┬──────────────────────────────────┐
 │ Kiểu               │ size │ align │ Ghi chú                          │
 ├────────────────────┼──────┼───────┼──────────────────────────────────┤
 │ ()                 │   0  │   1   │ ZST                              │
 │ bool               │   1  │   1   │ chỉ 0/1 hợp lệ → có niche        │
 │ u8 / i8            │   1  │   1   │                                  │
 │ char               │   4  │   4   │ Unicode scalar, KHÔNG 1 byte     │
 │ i32 / u32 / f32    │   4  │   4   │ mặc định số nguyên = i32         │
 │ i64 / u64 / f64    │   8  │   8   │                                  │
 │ usize / isize      │   8  │   8   │ = kích thước con trỏ             │
 │ i128 / u128        │  16  │  16   │                                  │
 ├────────────────────┼──────┼───────┼──────────────────────────────────┤
 │ &T &mut T *const T │   8  │   8   │ thin pointer (T: Sized)          │
 │ Box<T>             │   8  │   8   │ thin, sở hữu heap (T: Sized)     │
 │ Rc<T> / Arc<T>     │   8  │   8   │ header 1 con trỏ; đếm ở heap     │
 │ &[T] / &str        │  16  │   8   │ FAT: ptr + len                   │
 │ &dyn Trait         │  16  │   8   │ FAT: ptr + vtable                │
 │ Box<dyn Trait>     │  16  │   8   │ FAT, sở hữu                      │
 ├────────────────────┼──────┼───────┼──────────────────────────────────┤
 │ Vec<T>             │  24  │   8   │ ptr + cap + len                  │
 │ String             │  24  │   8   │ = Vec<u8> (UTF-8)                │
 │ [i32; 4]           │  16  │   4   │ N × size, liền mạch              │
 │ (i32, u8)          │   8  │   4   │ như struct, có padding           │
 ├────────────────────┼──────┼───────┼──────────────────────────────────┤
 │ Option<i32>        │   8  │   4   │ cần tag (i32 không niche)        │
 │ Option<u8>         │   2  │   1   │ cần tag                          │
 │ Option<bool>       │   1  │   1   │ NICHE: None miễn phí             │
 │ Option<char>       │   4  │   4   │ NICHE                            │
 │ Option<&T>         │   8  │   8   │ NICHE: None = null               │
 │ Option<Box<T>>     │   8  │   8   │ NICHE                            │
 │ NonZeroU32         │   4  │   4   │ 0 là niche                       │
 │ Option<NonZeroU32> │   4  │   4   │ NICHE: dùng 0 làm None           │
 │ Result<i32, u8>    │   8  │   4   │ enum 2 variant + tag             │
 ├────────────────────┼──────┼───────┼──────────────────────────────────┤
 │ struct{u8,u32,u16} │   8  │   4   │ repr(Rust): hoán + nén           │
 │  cùng, repr(C)     │  12  │   4   │ giữ thứ tự → nhiều padding       │
 │  cùng, repr(packed)│   7  │   1   │ bỏ hết padding                   │
 └────────────────────┴──────┴───────┴──────────────────────────────────┘
```

> Tự kiểm chứng: `println!("{}", std::mem::size_of::<Vec<i32>>());`

---

<a name="22-cây-quyết-định"></a>
## 22. Cây quyết định: "kiểu này nằm đâu, bao nhiêu byte?"

```
   Có kiểu T, muốn biết layout?
                 │
                 ▼
   ┌─────────────────────────────────────┐
   │ T là Sized? (biết size lúc compile) │
   └─────────────────────────────────────┘
        │ KHÔNG                  │ CÓ
        ▼                        ▼
   [T] str dyn Trait      ┌──────────────────────────┐
   → KHÔNG dùng trực      │ T có chứa con trỏ heap?   │
     tiếp được; phải qua  │ (Box/Vec/String/Rc...)    │
     &  hoặc  Box         └──────────────────────────┘
        │                    │ CÓ              │ KHÔNG
        ▼                    ▼                 ▼
   con trỏ tới nó là   header nhỏ trên     nằm THẲNG trên
   FAT (16B):          stack TRỎ tới       stack, size =
   • &[T]/&str: ptr+len heap:              tổng field +
   • &dyn: ptr+vtable   • Box   →  8B      padding (theo
                        • Rc/Arc → 8B      align), compiler
                        • Vec/String→24B   có thể hoán field
                        DATA ở heap.       (nếu repr(Rust)).
```

Ba câu hỏi vàng cho **bất kỳ** kiểu mới gặp:

```
   1. Nó Sized không?           → quyết định thin (8B) hay fat (16B) khi mượn
   2. Data nằm stack hay heap?  → header trên stack, nội dung có thể ở heap
   3. Có niche để nén Option?   → Option<nó> có miễn phí không
```

---

<a name="23-mind-map"></a>
## 23. Mind map tổng kết

```
                          MEMORY LAYOUT của DATA TYPE
                                     │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
    ┌───▼────┐                  ┌────▼─────┐                 ┌────▼─────┐
    │ 3 SỐ   │                  │ STACK    │                 │ HEAP     │
    │ size   │                  │ (Sized,  │                 │ (runtime │
    │ align  │                  │  nhanh)  │                 │  size)   │
    │ offset │                  └────┬─────┘                 └────┬─────┘
    └───┬────┘                       │                            │
        │                  ┌─────────┼─────────┐         data của:│
   tính 100% lúc           │         │         │         Box/Vec/ │
   COMPILE (Sized)     scalar   compound   con trỏ       String/  │
        │              i32 f64  struct     thin 8B:      Rc/Arc   │
        │              bool     enum       &T Box Rc      ────────┘
        │              char 4B  tuple      fat 16B:
        │              () 0B    array      &[T] &str &dyn
        │
   ┌────┴─────────────────────────────────────────────────────┐
   │ NGUYÊN LÝ XUYÊN SUỐT                                       │
   ├───────────────────────────────────────────────────────────┤
   │ • align tồn tại vì CPU đọc theo khối → padding lấp khe     │
   │ • repr(Rust) tự do hoán field để nén; repr(C) giữ thứ tự  │
   │ • Sized → thin 8B; không-Sized → fat 16B (kèm len/vtable) │
   │ • "kiểu cao cấp" = header nhỏ trên stack → data trên heap  │
   │ • Niche: tận dụng bit dư → Option<&T>=8B, None miễn phí    │
   │ • ZST chiếm 0 byte: thông tin compile-time, runtime free   │
   │ • Generic monomorphize: mỗi T một layout riêng, zero-cost  │
   └───────────────────────────────────────────────────────────┘

        TẤT CẢ phục vụ MỘT triết lý:
        "Trừu tượng cấp cao, chi phí bộ nhớ bằng 0 — bạn không
         trả cho thứ bạn không dùng, và thứ bạn dùng không thể
         làm thủ công tốt hơn." (zero-cost abstraction)
```

---

## Đọc tiếp

- [a-memory-model.md](./a-memory-model.md) — stack/heap, cache, virtual memory ở tầng hệ thống
- [b-ownership-borrowing.md](./b-ownership-borrowing.md) — vì sao move/borrow an toàn (luật trên các layout này)
- [c-trait.md](./c-trait.md) — vtable & dynamic dispatch chi tiết
- [i-smart-pointers.md](./i-smart-pointers.md) — Box/Rc/Arc/Cell/RefCell/Cow đầy đủ
- [n-unsafe-rust.md](./n-unsafe-rust.md) — thao tác trực tiếp byte, raw pointer, repr

> 🦀 **Một câu chốt:** trong Rust, *kiểu* chỉ là một hợp đồng về **bao nhiêu byte, căn lề ra sao, byte nào nghĩa gì**. Hiểu layout = hiểu tại sao Rust vừa nhanh như C vừa an toàn.
