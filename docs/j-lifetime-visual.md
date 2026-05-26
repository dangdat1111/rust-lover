# Lifetime Rust — Minh Hoạ Trực Quan

> Companion visual cho [lifetime.md](./lifetime.md). Đọc song song để hiểu sâu.

---

## 1. Bức tranh lớn — Lifetime Universe

```
                       LIFETIME TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   COMPILE-TIME CONCEPT — không tồn tại runtime         │
       │   ─────────────────────────────────────────────         │
       │                                                        │
       │            "Khoảng thời gian giá trị sống"             │
       │                                                        │
       │   ┌────────┐ ┌────────┐ ┌──────────┐ ┌──────────────┐  │
       │   │  'a    │ │ 'static│ │ Elision  │ │  Bounds      │  │
       │   │ names  │ │ special│ │ 3 rules  │ │  T: 'a, 'a:'b│  │
       │   └────────┘ └────────┘ └──────────┘ └──────────────┘  │
       │       │                                                │
       │       ▼                                                │
       │   ┌────────────────────────────────────────────────┐   │
       │   │     BORROW CHECKER                             │   │
       │   │     ────────────────                            │   │
       │   │   Đảm bảo: ref không sống lâu hơn owner        │   │
       │   │   Đảm bảo: 1 mut OR N immut tại 1 thời điểm    │   │
       │   │   NLL: borrow end tại last use                 │   │
       │   │   Polonius (future): chính xác hơn             │   │
       │   └────────────────────────────────────────────────┘   │
       │       │                                                │
       │       ▼                                                │
       │   ┌────────────────────────────────────────────────┐   │
       │   │     ADVANCED TOPICS                            │   │
       │   │   • Subtyping & Variance                       │   │
       │   │   • HRTB (for<'a>)                             │   │
       │   │   • GAT (generic associated types)             │   │
       │   │   • Self-referential + Pin                     │   │
       │   └────────────────────────────────────────────────┘   │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Lifetime là TIMELINE

```
   Code:
   ─────
   let x = 5;
   let r = &x;
   println!("{}", r);
   
   
   Timeline (lifetime visualization):
   ──────────────────────────────────
   
   x:  [────────────────────────]  lifetime của x
       │                        │
       ▼                        ▼
       let x = 5;               (scope end)
   
   r:        [──────────]  lifetime của r (borrow)
             │          │
             ▼          ▼
       let r = &x;   println!
   
   
   Rule: lifetime của r ⊆ lifetime của x (contained inside)
   
   
   ❌ Vi phạm:
   ──────────
   {
       let r;
       {
           let x = 5;       [─────────] x
           r = &x;             [─────] r
       }
       println!("{}", r);          [...] r dùng ở đây
   }
                                      ↑
                              r dùng SAU KHI x chết → DANGLING!
   
   Borrow checker: 'r > 'x → ❌ FAIL
```

---

## 3. `'a` chỉ là TÊN — không tạo lifetime

```
   ❌ HIỂU SAI:
   ────────────
   fn foo<'a>(x: &'a str) -> &'a str { x }
   //      ↑
   //  "'a là lifetime tự tạo, mọi thứ có 'a sẽ tự kéo dài"
   
                   👆 SAI HOÀN TOÀN!
   
   
   ✅ HIỂU ĐÚNG:
   ────────────
   fn foo<'a>(x: &'a str) -> &'a str { x }
   //      ↑
   //  "'a là TÊN của một lifetime đã có sẵn ở caller"
   //  "function nói: output sẽ có cùng tên 'a — sống bằng input"
   
   
   Khi caller gọi:
   ──────────────
   
   let s = String::from("hi");     ← lifetime của s = 'real
   let r = foo(&s);                ← compiler: 'a = 'real
                                   ← r có lifetime 'real
   drop(s);                        ← s chết
   println!("{}", r);              ← ❌ r dangling
   
   
   ┌──────────────────────────────────────────────┐
   │ 'a không kéo dài gì. 'a chỉ MÔ TẢ            │
   │ mối quan hệ giữa các lifetime đã tồn tại.    │
   └──────────────────────────────────────────────┘
```

---

## 4. Cú pháp lifetime — Cheatsheet

```
   ┌──────────────────────────────┬──────────────────────────────┐
   │ CÚ PHÁP                      │ NGHĨA                        │
   ├──────────────────────────────┼──────────────────────────────┤
   │ &i32                         │ Reference, elided lifetime   │
   │ &'a i32                      │ Reference, lifetime 'a       │
   │ &'static i32                 │ Reference, lifetime mãi mãi  │
   │ &'a mut i32                  │ Mut ref, lifetime 'a         │
   │ &'_  T                       │ Anonymous lifetime           │
   │                              │                              │
   │ fn foo<'a>(...)              │ Declare 'a param              │
   │ struct S<'a> { ... }         │ Struct với lifetime          │
   │ impl<'a> S<'a> { ... }       │ Impl block                   │
   │ trait T<'a> { ... }          │ Trait                        │
   │                              │                              │
   │ <T: 'a>                      │ T outlives 'a                │
   │ <'a: 'b>                     │ 'a outlives 'b               │
   │ T: 'static                   │ T không borrow ngắn          │
   │                              │                              │
   │ for<'a> Fn(&'a str)          │ HRTB — for all 'a            │
   │ &'a Foo<'b>                  │ Ref 'a, vào Foo có lifetime 'b│
   └──────────────────────────────┴──────────────────────────────┘
   
   
   📌 Vị trí:
   ──────────
   Lifetime trước type:    fn foo<'a, T>(...)
   Lifetime trước mut:     &'a mut T  (không &mut 'a T)
```

---

## 5. Lifetime Elision — 3 Quy tắc

```
   ┌────────────────────────────────────────────────────────────┐
   │ Rule 1: Mỗi input reference có lifetime riêng              │
   │ ─────────────────────────────────────────────              │
   │                                                            │
   │ fn foo(x: &str, y: &str)                                   │
   │ ↓ compiler tự thêm:                                        │
   │ fn foo<'a, 'b>(x: &'a str, y: &'b str)                     │
   │                                                            │
   ├────────────────────────────────────────────────────────────┤
   │ Rule 2: ĐÚNG 1 input ref → output cùng lifetime            │
   │ ────────────────────────────────────────────────           │
   │                                                            │
   │ fn first(s: &str) -> &str                                  │
   │ ↓                                                          │
   │ fn first<'a>(s: &'a str) -> &'a str                        │
   │                                                            │
   ├────────────────────────────────────────────────────────────┤
   │ Rule 3: &self/&mut self → output lifetime của self         │
   │ ───────────────────────────────────────────────────        │
   │                                                            │
   │ impl Foo { fn get(&self, k: &str) -> &str }                │
   │ ↓                                                          │
   │ fn get<'a, 'b>(&'a self, k: &'b str) -> &'a str            │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
   
   
   Cây quyết định elision:
   ───────────────────────
   
            Function có reference?
                    │
              ┌─────┴─────┐
             No           Yes
              │            │
              ▼            ▼
             OK         Output có ref?
                            │
                       ┌────┴────┐
                      No        Yes
                       │         │
                       ▼         ▼
                       OK     Có &self?
                                  │
                             ┌────┴────┐
                            Yes       No
                             │         │
                             ▼         ▼
                          Use self  Đúng 1 input ref?
                          lifetime       │
                                    ┌────┴────┐
                                   Yes       No
                                    │         │
                                    ▼         ▼
                              Use input    PHẢI VIẾT
                              lifetime     EXPLICIT
```

---

## 6. Khi elision FAIL — Phải viết explicit

```
   ❌ Elision không xử lý:
   ───────────────────────
   
   fn longer(x: &str, y: &str) -> &str { ... }
   //   2 input ref ('a, 'b independent)
   //   1 output ref — lifetime nào? 'a? 'b?
   //   Compiler không biết → ERROR
   
   
   ✅ 3 cách viết explicit:
   ────────────────────────
   
   Option 1: Cả 2 cùng 'a
   ──────────────────────
   fn longer<'a>(x: &'a str, y: &'a str) -> &'a str { ... }
   //  'a = lifetime ngắn nhất giữa x và y
   //  output ≤ min(x, y)
   
   
   Option 2: Output gắn với x
   ──────────────────────────
   fn longer<'a, 'b>(x: &'a str, y: &'b str) -> &'a str { ... }
   //  output có cùng lifetime với x
   //  y có thể ngắn hơn — không ảnh hưởng output
   
   
   Option 3: Output gắn với y
   ──────────────────────────
   fn longer<'a, 'b>(x: &'a str, y: &'b str) -> &'b str { ... }
   //  ngược lại
   
   
   📌 Chọn theo logic function:
      "Output thực sự mượn từ đâu?"
```

---

## 7. Lifetime trong Struct

```
   Struct chứa reference → CẦN lifetime parameter
   ──────────────────────────────────────────────
   
   ❌ ERROR:
   struct Wrap { data: &str }   ← missing lifetime
   
   ✅ OK:
   struct Wrap<'a> { data: &'a str }
   
   
   Memory + Lifetime:
   ──────────────────
   
   ┌────────────────────────────────────┐
   │ Stack frame caller                 │
   │                                    │
   │  s: String  (owner of "hello")     │
   │      │                             │
   │      └──referenced by──┐           │
   │                        │           │
   │  w: Wrap<'_> {         │           │
   │      data: &───────────┘           │
   │  }                                 │
   │                                    │
   │  Lifetime của w ≤ lifetime của s   │
   └────────────────────────────────────┘
   
   
   Khi drop(s) trước khi w còn dùng:
   ─────────────────────────────────
   
   let s = String::from("hi");
   let w = Wrap { data: &s };
   drop(s);                  ← ❌ ERROR: w còn borrow s
   use(&w);
   
   
   Trade-off: Owned vs Borrowed struct
   ────────────────────────────────────
   
   ┌─────────────────────┬─────────────────────┐
   │ Borrowed            │ Owned               │
   ├─────────────────────┼─────────────────────┤
   │ struct W<'a> {      │ struct W {          │
   │   data: &'a str     │   data: String      │
   │ }                   │ }                   │
   ├─────────────────────┼─────────────────────┤
   │ Zero alloc          │ Heap alloc          │
   │ Có lifetime         │ Không lifetime      │
   │ Khó dùng public API │ Dễ dùng public API  │
   │ Lifetime constraint │ Tự do hơn           │
   │                     │                     │
   │ Use: parser,        │ Use: cache, config, │
   │   iterator, short   │   long-lived state  │
   └─────────────────────┴─────────────────────┘
```

---

## 8. Lifetime Bounds — `T: 'a` và `'a: 'b`

```
   T: 'a   ──►  "T outlives 'a"
                "T (nếu có ref) sống ít nhất bằng 'a"
   
   'a: 'b  ──►  "'a outlives 'b"
                "'a sống ít nhất bằng 'b"  ('a ≥ 'b)
   
   
   Visualization:
   ──────────────
   
   'a: ──[────────────────────────────────]── lifetime 'a
   'b: ───────[──────────────]─────────────── lifetime 'b
   
   'a: 'b  ✅  ('a chứa hoặc dài hơn 'b)
   'b: 'a  ❌  ('b ngắn hơn 'a)
   
   
   Subtyping:
   ──────────
   
   'a ≥ 'b means 'a can be "demoted" to 'b
   
   ┌──────────────────────────────────────┐
   │ 'static  ─── longest                 │
   │   ↑                                  │
   │   demote                             │
   │   ↓                                  │
   │ 'a    (longer scope)                 │
   │   ↑                                  │
   │   demote                             │
   │   ↓                                  │
   │ 'b    (shorter scope)                │
   │   ↑                                  │
   │   demote                             │
   │   ↓                                  │
   │ '_    (very short)                   │
   └──────────────────────────────────────┘
   
   
   Practical example:
   ──────────────────
   
   fn from_b_to_a<'a, 'b: 'a>(y: &'b str) -> &'a str { y }
   //                  ↑
   //              'b ≥ 'a (longer can serve as shorter)
   //              y có thể "co" thành &'a str
```

---

## 9. `'static` — 2 nghĩa khác nhau

```
   ┌──────────────────────────────────────────────────────────────┐
   │ NGHĨA 1: &'static T                                          │
   │ ─────────────────                                            │
   │                                                              │
   │ Reference sống đến END OF PROGRAM                            │
   │                                                              │
   │ let s: &'static str = "literal";   ← string literal          │
   │   stored in .rodata (read-only)                              │
   │   không bao giờ drop                                         │
   │                                                              │
   │ Memory:                                                      │
   │ ┌────────────┐                                               │
   │ │ .rodata    │ ← OS load và keep alive cho process          │
   │ │  "literal" │                                               │
   │ └────────────┘                                               │
   │       ▲                                                      │
   │       │                                                      │
   │   &'static str (16 byte: ptr + len)                          │
   └──────────────────────────────────────────────────────────────┘
   
   ┌──────────────────────────────────────────────────────────────┐
   │ NGHĨA 2: T: 'static                                          │
   │ ──────────────────                                           │
   │                                                              │
   │ Type T không chứa reference ngắn                             │
   │ (chứa owned hoặc 'static refs)                               │
   │                                                              │
   │ Ví dụ T: 'static:                                            │
   │   i32, String, Vec<u8>, Box<T>, &'static str                 │
   │                                                              │
   │ NOT T: 'static:                                              │
   │   &'a str, struct Wrapper<'a> { x: &'a str }                 │
   │                                                              │
   │ ⚠️ T: 'static KHÔNG có nghĩa instance sống mãi:              │
   │                                                              │
   │ let s: String = String::from("hi");  ← String: 'static       │
   │ drop(s);                              ← drop sớm OK          │
   │                                                              │
   │ 'static chỉ nói "type tự stand-alone, không phụ              │
   │ thuộc lifetime nào ngắn hơn".                                │
   └──────────────────────────────────────────────────────────────┘
   
   
   So sánh trực quan:
   ──────────────────
   
   ┌─────────────────────┬─────────────────────┐
   │ &'static T          │ T: 'static          │
   ├─────────────────────┼─────────────────────┤
   │ Cụ thể: ref         │ Constraint: type    │
   │ sống mãi            │ không borrow ngắn   │
   ├─────────────────────┼─────────────────────┤
   │ "literal"           │ String              │
   │ Box::leak(x)        │ i32                 │
   │ static FOO: &T = .. │ Vec<u8>             │
   │                     │ &'static str        │
   ├─────────────────────┼─────────────────────┤
   │ Instance sống mãi   │ Instance có thể     │
   │                     │ drop bất cứ lúc nào │
   └─────────────────────┴─────────────────────┘
```

---

## 10. `'static` use case visualization

```
   1. tokio::spawn cần F: 'static
   ───────────────────────────────
   
   tokio::spawn(future)  where F: Future + Send + 'static
   
   Lý do:
   ──────
   Task có thể sống lâu hơn spawn site:
   
   fn caller() {
       let s = String::from("hi");
       tokio::spawn(async {
           println!("{}", s);     ← ❌ s là ref ngắn
       });
   }   ← caller return, s drop, task vẫn chạy → UB!
   
   Fix: move owned:
   ────────────────
   fn caller() {
       let s = String::from("hi");
       tokio::spawn(async move {    ← move s vào async block
           println!("{}", s);       ← s đi cùng task → 'static ✅
       });
   }
   
   
   2. Global static với Box::leak
   ──────────────────────────────
   
   fn load_config() -> &'static Config {
       let cfg = Config::from_file();    ← heap-allocated String
       Box::leak(Box::new(cfg))           ← intentional leak
       //  ↑ Box consumed, return &'static
       //    memory NEVER freed (OK vì sống đến end)
   }
   
   Memory:
   ┌────────────────────┐
   │ Heap (leaked)      │
   │  Config { ... }    │ ← lived until program exit
   └────────────────────┘
        ▲
        │
        └─── &'static Config
        
   📌 Pattern: dùng cho global singleton.
              Tránh nếu function gọi nhiều lần (leak từng instance).
```

---

## 11. Subtyping & Variance

```
   Subtype trong lifetime:
   ───────────────────────
   
   'static <: 'a     ('static là subtype của 'a)
   'long   <: 'short ('long là subtype của 'short)
   
   "Longer lifetime can be USED as shorter lifetime"
   (Liskov substitution)
   
   
   Variance — Khi T <: U, Foo<T> có quan hệ gì với Foo<U>?
   ────────────────────────────────────────────────────────
   
   ┌─────────────────┬────────────────────────────────────────┐
   │ Variance        │ Visualization                          │
   ├─────────────────┼────────────────────────────────────────┤
   │ COVARIANT       │ T <: U  →  Foo<T> <: Foo<U>            │
   │                 │ (same direction)                       │
   │                 │ E.g. &'a T over 'a                     │
   ├─────────────────┼────────────────────────────────────────┤
   │ CONTRAVARIANT   │ T <: U  →  Foo<U> <: Foo<T>            │
   │                 │ (reverse direction)                    │
   │                 │ E.g. fn(T) -> U over T input           │
   ├─────────────────┼────────────────────────────────────────┤
   │ INVARIANT       │ T <: U  →  no relation                 │
   │                 │ E.g. &'a mut T over T                  │
   │                 │      Cell<T> over T                    │
   └─────────────────┴────────────────────────────────────────┘
   
   
   Variance của các types phổ biến:
   ────────────────────────────────
   
   ┌────────────────┬────────────────┬────────────────┐
   │ Type           │ Over T         │ Over 'a        │
   ├────────────────┼────────────────┼────────────────┤
   │ &'a T          │ covariant      │ covariant      │
   │ &'a mut T      │ INVARIANT      │ covariant      │
   │ Box<T>         │ covariant      │ —              │
   │ Vec<T>         │ covariant      │ —              │
   │ Cell<T>        │ INVARIANT      │ —              │
   │ *const T       │ covariant      │ —              │
   │ *mut T         │ INVARIANT      │ —              │
   │ fn(T) -> U     │ contra in T    │ covariant      │
   │                │ co in U        │                │
   │ PhantomData<T> │ covariant      │ —              │
   └────────────────┴────────────────┴────────────────┘
```

---

## 12. Tại sao `&mut T` INVARIANT?

```
   Nếu &mut T covariant trong T, sẽ có lỗ hổng:
   ─────────────────────────────────────────────
   
   ┌─────────────────────────────────────────────────────┐
   │ Setup:                                              │
   │   static_str: &'static str = "lit";                 │
   │   r1: &'static mut &'static str = &mut static_str;  │
   │                                                     │
   │ Demote (if covariant):                              │
   │   r2: &'a mut &'a str = r1;     // 'a < 'static     │
   │                                                     │
   │ Overwrite:                                          │
   │   local: String = String::from("local");            │
   │   *r2 = &local;                  // gán ref ngắn    │
   │                                                     │
   │ Result:                                             │
   │   r1 (kiểu &'static mut &'static str) trỏ vào local │
   │   local sẽ drop → r1 dangling → UB                  │
   └─────────────────────────────────────────────────────┘
   
   ⟹ &mut T INVARIANT trong T để ngăn lỗ hổng này.
   
   
   Tương tự cho Cell<T>:
   ─────────────────────
   
   Cell<T> cho phép set qua &Cell — nếu covariant có lỗ hổng tương tự.
   ⟹ Cell<T> INVARIANT trong T.
```

---

## 13. NLL — Non-Lexical Lifetimes

```
   TRƯỚC NLL (lexical):
   ────────────────────
   
   fn main() {
       let mut v = vec![1, 2, 3];
       let r = &v[0];          ←─┐
       println!("{}", r);        │ Borrow alive
       v.push(4);                │ đến hết }
       //   ↑ ERROR cũ           │
   }                              ←─┘ Borrow end (lexical)
   
   
   VỚI NLL (current):
   ──────────────────
   
   fn main() {
       let mut v = vec![1, 2, 3];
       let r = &v[0];          ←─┐ Borrow alive
       println!("{}", r);        │ đến LAST USE
                                 │
       v.push(4);              ←─┘ Borrow đã end (r không dùng nữa)
       //   ↑ ✅ OK
   }
   
   
   Cách NLL track:
   ───────────────
   
              Control Flow Graph (CFG)
                       │
                       ▼
              Phân tích last use của mỗi borrow
                       │
                       ▼
              Borrow end tại "last use" trên mọi path
   
   
   Path-sensitive:
   ───────────────
   
   if cond {
       use(r);     ← last use trên path TRUE
   }
   // Trên path FALSE: r đã end ngay từ assignment
   
   v.push(4);      ← OK trên FALSE, error trên TRUE? NLL: chỉ check thực sự
```

---

## 14. NLL chưa hoàn hảo → Polonius

```
   ❌ NLL FAIL nhưng đúng logic:
   ─────────────────────────────
   
   fn first_or_insert(
       map: &mut HashMap<i32, String>, k: i32
   ) -> &String {
       if let Some(v) = map.get(&k) {
           return v;                       ← return trên branch này
       }
       map.insert(k, String::new());       ← ❌ NLL: get vẫn borrow
       map.get(&k).unwrap()
   }
   
   NLL nghĩ: get borrow extend tới cuối function (vì trên branch TRUE)
   Thực tế: branch TRUE return — không cần insert sau
   
   
   ✅ Polonius (future) giải quyết:
   ────────────────────────────────
   Phân tích "borrow chỉ extend nếu KHÔNG return" — branch TRUE return
   nên borrow không extend qua sau if. Insert OK.
   
   
   Hiện tại workaround:
   ────────────────────
   fn first_or_insert(
       map: &mut HashMap<i32, String>, k: i32
   ) -> &String {
       map.entry(k).or_insert_with(String::new)
       //  ↑ entry API thiết kế để tránh borrow conflict
   }
   
   ────────────────────────────────────────────────
   
   Trạng thái Polonius:
   ────────────────────
   • Đang phát triển (nightly)
   • Datalog-based
   • Hoàn thành chưa rõ
   • Try nightly:
        RUSTFLAGS="-Z polonius" cargo +nightly build
```

---

## 15. Two-Phase Borrows

```
   Code thường ngày:
   ─────────────────
   let mut v = vec![1, 2, 3];
   v.push(v.len());     ← bình thường conflict
   
   Phân tích:
   ──────────
   v.push(...) cần &mut v
   v.len()     cần &v
   
   Cùng lúc &mut v + &v? Vi phạm rule!
   
   
   Two-phase borrow (NLL feature):
   ───────────────────────────────
   
   1. Reservation phase (mut borrow tạo nhưng "passive"):
      ┌────────────────────────────────────┐
      │ &mut v reserved (chưa activate)    │
      └────────────────────────────────────┘
                       │
                       ▼
   2. v.len() cần &v — OK vì &mut v chưa active:
      ┌────────────────────────────────────┐
      │ &v alive (immutable)               │
      │ &mut v reserved (still passive)    │
      └────────────────────────────────────┘
                       │
                       ▼
   3. &v drop khi len() return:
      ┌────────────────────────────────────┐
      │ &mut v activate                    │
      │ (giờ exclusive access)             │
      └────────────────────────────────────┘
                       │
                       ▼
   4. push(...) dùng &mut v:
      ┌────────────────────────────────────┐
      │ push thực thi                      │
      └────────────────────────────────────┘
```

---

## 16. HRTB — `for<'a>`

```
   Vấn đề:
   ───────
   
   fn apply<F>(f: F) where F: Fn(&str) -> &str {
       let s = String::from("hi");
       let r = f(&s);
       println!("{}", r);
   }
   
   Closure F nhận &str — lifetime nào? Quyết tại định nghĩa hay caller?
   
   
   Trả lời: HRTB
   ─────────────
   
   fn apply<F>(f: F) where F: for<'a> Fn(&'a str) -> &'a str {
                              ↑
                          "for all 'a"
   
   F phải work với MỌI lifetime 'a — caller pass closure
   universally applicable.
   
   
   Visualization:
   ──────────────
   
   ┌──────────────────────────────────────────────────┐
   │ Without HRTB:                                    │
   │ F: Fn(&'a str) -> &'a str    ← 'a fixed         │
   │                                                  │
   │     F works ONLY for one specific 'a            │
   │                                                  │
   │ With HRTB:                                       │
   │ F: for<'a> Fn(&'a str) -> &'a str               │
   │                                                  │
   │     F works for ALL 'a                          │
   │     (∀ 'a, F can accept &'a str)                │
   └──────────────────────────────────────────────────┘
   
   
   Khi tự explicit cần HRTB:
   ─────────────────────────
   
   fn iter_pairs<F>(f: F) 
   where F: for<'a> Fn(&'a [u8]) -> &'a [u8]
   {
       let v = vec![0u8; 10];
       let slice = f(&v);     ← compiler suy ra 'a = lifetime của v
   }
   
   
   Compiler tự thêm `for<'a>` cho Fn/FnMut/FnOnce:
   ───────────────────────────────────────────────
   
   fn apply<F: Fn(&str) -> &str>(f: F)
   // = for<'a> Fn(&'a str) -> &'a str  (tự động)
```

---

## 17. Lifetime trong async

```
   async fn process(data: &str) {
       other_async().await;       ← yield point
       println!("{}", data);
   }
   
   
   Compiler sinh state machine:
   ────────────────────────────
   
   struct ProcessFuture<'a> {
       data: &'a str,             ← captured borrow
       inner: SomeOtherFuture,
       state: enum { Start, Awaiting, Done },
   }
   
   ┌──────────────────────────────────────┐
   │ Lifetime của Future == lifetime của  │
   │ borrow nó chứa (= 'a)                │
   │                                      │
   │ Future không thể sống lâu hơn data   │
   └──────────────────────────────────────┘
   
   
   Spawn yêu cầu 'static:
   ──────────────────────
   
   ❌ Spawn future giữ borrow ngắn:
   
   let s = String::from("hi");
   tokio::spawn(process(&s));         ← Future chứa &s ('a)
                                      ❌ 'a không 'static
   
   
   ✅ Move owned data:
   
   let s = String::from("hi");
   tokio::spawn(async move {          ← move s vào block
       println!("{}", s);              ← Future tự own s
   });                                 ← Future: 'static ✅
   
   
   Common error: borrow conflict trong async
   ─────────────────────────────────────────
   
   impl Cache {
       async fn get_or_fetch(&mut self, key: &str) -> String {
           if let Some(v) = self.data.get(key) {
               return v.clone();      ← borrow của data alive
           }
           let new = fetch_remote(key).await;
                                      ↑
                                  await yield → borrow vẫn alive
                                  trong state machine
           self.data.insert(key.into(), new.clone());
                  ↑ ❌ ERROR: conflict with .get() borrow
           new
       }
   }
   
   Fix:
   ────
   if self.data.contains_key(key) {
       return self.data.get(key).unwrap().clone();
   }
   let new = fetch_remote(key).await;
   self.data.insert(key.into(), new.clone());
   new
```

---

## 18. Self-Referential Struct + Pin

```
   Vấn đề:
   ───────
   
   struct Node {
       data: String,
       r: &??? String,    ← muốn trỏ vào self.data
   }
   
   Lifetime nào cho r? Không có 'self trong Rust.
   
   
   Memory layout (nếu cho phép):
   ─────────────────────────────
   
   ┌──────────────────────────┐
   │ Node                     │
   │   data: "hello"   ←──┐   │
   │   r: &data ──────────┘   │  ← self-reference
   └──────────────────────────┘
   
   
   Nếu MOVE struct:
   ────────────────
   
   Trước (địa chỉ 0x1000):
   ┌────────────────────┐
   │ data: "hello"  ←─┐ │
   │ r: 0x1000        │ │
   └──────────────────┘
   
   Sau move (sang 0x2000):
   ┌────────────────────┐
   │ data: "hello"      │ ← địa chỉ 0x2000 (copied)
   │ r: 0x1000 ❌       │ ← vẫn trỏ về 0x1000 (cũ) → DANGLING
   └────────────────────┘
   
   
   Giải pháp: Pin<P> + unsafe
   ──────────────────────────
   
   use std::pin::Pin;
   use std::marker::PhantomPinned;
   
   struct SelfRef {
       data: String,
       r: *const String,             ← raw pointer
       _pin: PhantomPinned,          ← opt-out Unpin
   }
   
   let pinned: Pin<Box<SelfRef>> = Box::pin(SelfRef {
       data: "hi".into(),
       r: std::ptr::null(),
       _pin: PhantomPinned,
   });
   
   // Pin<Box<...>>  bảo đảm memory không move
   
   
   Hoặc dùng crate ouroboros (safe abstraction):
   ─────────────────────────────────────────────
   
   #[ouroboros::self_referencing]
   struct SelfRef {
       data: String,
       #[borrows(data)]
       r: &'this String,    ← magic lifetime 'this
   }
```

---

## 19. Patterns đẹp

```
   ✅ Pattern 1: Borrowed iterator (zero alloc)
   ────────────────────────────────────────────
   
   struct Lines<'a> { source: &'a str, pos: usize }
   
   impl<'a> Iterator for Lines<'a> {
       type Item = &'a str;
       fn next(&mut self) -> Option<&'a str> {
           if self.pos >= self.source.len() { return None; }
           let end = self.source[self.pos..].find('\n')
               .map(|i| self.pos + i)
               .unwrap_or(self.source.len());
           let line = &self.source[self.pos..end];
           self.pos = end + 1;
           Some(line)
       }
   }
   
   ⟹ Mỗi `next()` trả &str view vào source — KHÔNG ALLOC
   
   
   ✅ Pattern 2: Cow để defer decision
   ──────────────────────────────────
   
   fn normalize(s: &str) -> Cow<str> {
       if s.contains(' ') {
           Cow::Owned(s.replace(' ', "_"))   ← alloc nếu cần
       } else {
           Cow::Borrowed(s)                  ← không alloc
       }
   }
   
   
   ✅ Pattern 3: Hide lifetime trong public API
   ────────────────────────────────────────────
   
   // ❌ Lifetime exposed:
   pub struct Service<'a> { config: &'a Config }
   
   // ✅ Lifetime hidden:
   pub struct Service { config: Arc<Config> }
   //  ↑ Arc owned, user không phải lo lifetime
   
   
   ✅ Pattern 4: Static config singleton
   ─────────────────────────────────────
   
   use std::sync::LazyLock;
   
   static CONFIG: LazyLock<Config> = LazyLock::new(|| Config::load());
   
   pub fn config() -> &'static Config { &CONFIG }
```

---

## 20. Antipatterns

```
   ❌ 1. Lifetime "chỉ để biên dịch"
   ─────────────────────────────────
   
   fn foo<'a>(x: &'a str) -> String {
       x.to_string()
   }
   
   'a không xuất hiện ở output → bỏ:
   fn foo(x: &str) -> String { x.to_string() }
   
   
   ❌ 2. Quá nhiều lifetime params
   ────────────────────────────────
   
   fn complex<'a, 'b, 'c, 'd>(
       a: &'a str, b: &'b str, c: &'c str, d: &'d str
   ) -> &'a str { ... }
   
   Khó đọc. Thường gộp:
   fn complex<'a>(a: &'a str, b: &'a str, c: &'a str, d: &'a str) -> &'a str
   
   
   ❌ 3. Lifetime ép user trong public API
   ───────────────────────────────────────
   
   pub fn parse<'a>(input: &'a str) -> Parser<'a> { ... }
   //  user PHẢI giữ input alive
   
   Tốt hơn:
   pub fn parse(input: &str) -> ParseResult { ... }
   //  return owned ParseResult, user free hand
   
   
   ❌ 4. Box::leak trong hàm gọi nhiều lần
   ─────────────────────────────────────
   
   fn make_msg(n: i32) -> &'static str {
       Box::leak(format!("count {}", n).into_boxed_str())
   }
   
   make_msg(1);  // leak 1 String
   make_msg(2);  // leak nữa!
   ...           // grows forever
   
   Tốt hơn: return String
   
   
   ❌ 5. Self-referential khi không cần
   ────────────────────────────────────
   
   Cố gắng tạo struct chứa data + ref vào data.
   Thường restructure được:
   • Tách 2 struct (owner + view)
   • Index-based (Vec + usize index)
   • Rc/Arc
   • Hoặc dùng ouroboros nếu thực sự cần
```

---

## 21. Decision tree — Lifetime hay không?

```
                  Cần reference?
                       │
                  ┌────┴────┐
                 NO         YES
                  │          │
                  ▼          ▼
              Owned T     Function/struct/method?
              (no lifetime
               needed)        │
                          ┌───┴───┐
                       Function  Struct
                          │       │
                          ▼       ▼
                    Elision    Cần lifetime
                    rules     parameter
                    cover?    (struct chứa ref)
                          │       │
                     ┌────┴───┐   │
                    Yes      No   │
                     │       │    │
                     ▼       ▼    │
                  No anno  PHẢI   │
                  needed   VIẾT   │
                                  │
                                  ▼
                              <'a> với field
                              data: &'a T
   
   
   ┌────────────────────────────────────────────┐
   │ Quy tắc thực dụng (senior):                │
   │                                            │
   │ • Public API: owned types preferred        │
   │ • Internal short-lived: borrowed OK        │
   │ • Iterator/Parser: borrowed (zero alloc)   │
   │ • Async spawned: 'static (move owned)      │
   │ • Async borrowed: lifetime gắn với await   │
   │ • Global config: &'static qua LazyLock     │
   └────────────────────────────────────────────┘
```

---

## 22. Common errors & fixes

```
   ❌ "borrowed value does not live long enough"
   ─────────────────────────────────────────────
   
   let r;
   {
       let x = 5;
       r = &x;          ← borrow x
   }                    ← x drops here
   println!("{}", r);   ← ❌ x lifetime ngắn hơn r
   
   Fix: rearrange scope, hoặc clone:
   ─────────────────────────────────
   let x = 5;
   let r = &x;
   println!("{}", r);
   
   
   ❌ "cannot return reference to local variable"
   ──────────────────────────────────────────────
   
   fn foo() -> &str {
       let s = String::from("hi");
       &s                ← ❌ s drop sau return
   }
   
   Fix: return owned:
   ──────────────────
   fn foo() -> String {
       String::from("hi")
   }
   
   
   ❌ "missing lifetime specifier"
   ────────────────────────────────
   
   fn first(x: &str, y: &str) -> &str { x }
                                  ↑ which input's lifetime?
   
   Fix: explicit:
   ──────────────
   fn first<'a>(x: &'a str, y: &str) -> &'a str { x }
   
   
   ❌ "cannot borrow `x` as mutable, as it is also borrowed as immutable"
   ──────────────────────────────────────────────────────────────────
   
   let mut v = vec![1, 2, 3];
   let r = &v[0];
   v.push(4);          ← ❌ immutable borrow `r` vẫn alive
   println!("{}", r);
   
   Fix: dùng r trước khi push (NLL):
   ─────────────────────────────────
   let mut v = vec![1, 2, 3];
   let r = &v[0];
   println!("{}", r);   ← last use of r
   v.push(4);            ← ✅ OK
   
   
   ❌ "implementation of `Trait` is not general enough"
   ────────────────────────────────────────────────────
   
   Closure không thoả mãn HRTB. Compiler không suy được for<'a>.
   
   Fix: explicit type annotation hoặc helper function với explicit signature:
   ────────────────────────────────────────────────────────────────────
   fn helper<'a>(x: &'a str) -> &'a str { x }
   apply(helper);
```

---

## 23. Mind map cuối — Lifetime tổng hợp

```
                          LIFETIME
                              │
       ┌─────────┬────────────┼────────────┬──────────────┐
       ▼         ▼            ▼            ▼              ▼
   FUNDAMENTAL ELISION     ADVANCED      ASYNC         PATTERNS
       │         │            │            │              │
   'a syntax   3 rules    Variance     state machine  Borrow iter
   'static     Cây quyết  HRTB         self-ref       Cow defer
   Borrow      định       Subtyping    Pin            Hide in API
   checker                Bounds       'static spawn  Static config
   NLL                    GAT
                          Polonius
                          (future)
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. 'a là TÊN, không phải tạo lifetime│
                │                                      │
                │  2. Elision cover 80% case            │
                │                                      │
                │  3. 'static có 2 nghĩa khác           │
                │                                      │
                │  4. Variance quan trọng: &mut INV    │
                │                                      │
                │  5. NLL relaxes lexical scope        │
                │                                      │
                │  6. async giữ borrow → state machine │
                │                                      │
                │  7. Public API: hạn chế lifetime     │
                │                                      │
                │  8. Lifetime = API contract          │
                └──────────────────────────────────────┘
```

---

## 24. Bộ tài liệu Rust giờ có 10 chủ đề

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
   │      lifetime-visual         ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 20 files, ~1.1 MB MD                             │
   │                                                          │
   │   🦀 Hoàn thiện ngữ vựng nền tảng Rust senior            │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau lifetime, có thể đi tiếp:

- **Unsafe Rust** — raw pointer, UnsafeCell, atomic ordering, FFI, soundness contracts
- **Iterator deep dive** — implement Iterator, lazy, rayon parallel
- **Testing patterns** — unit, integration, proptest, criterion bench
- **Logging & Observability** — tracing nâng cao, OpenTelemetry
- **Performance** — profiling, criterion, perf, flamegraph
- **Web framework realistic** — axum project apply 10 chủ đề
- **Database** — sqlx, sea-orm với async + lifetime

Báo cái nào muốn đào sâu! 🦀⚡
