# Trait — TOÀN BỘ qua HÌNH VẼ

> Companion visual cho `trait.md`. Mọi khái niệm được vẽ ra. Đọc tuần tự — mỗi hình xây trên hình trước.

---

## Mục lục

1. [Bức tranh lớn: Trait là gì?](#1-bức-tranh-lớn)
2. [Trait vs Class — so sánh tư duy](#2-trait-vs-class)
3. [Khai báo & implement](#3-khai-báo-impl)
4. [Default method](#4-default-method)
5. [Supertrait](#5-supertrait)
6. [Method vs Associated Function](#6-method-vs-assoc)
7. [Static dispatch — Monomorphization](#7-static-dispatch)
8. [Dynamic dispatch — vtable](#8-dynamic-dispatch)
9. [Fat pointer của trait object](#9-fat-pointer)
10. [Layout vtable](#10-layout-vtable)
11. [So sánh Static vs Dynamic](#11-so-sánh)
12. [Object safety — rule trực quan](#12-object-safety)
13. [Associated type](#13-associated-type)
14. [Generic parameter trên trait](#14-generic-on-trait)
15. [Marker trait — Send/Sync flow](#15-send-sync)
16. [Orphan rule — coherence](#16-orphan-rule)
17. [Newtype pattern](#17-newtype)
18. [Blanket implementation](#18-blanket-impl)
19. [Iterator — chain & lazy](#19-iterator)
20. [Deref coercion chain](#20-deref-coercion)
21. [From → Into auto](#21-from-into)
22. [Bản đồ trait stdlib](#22-stdlib-map)
23. [Decision tree: static hay dynamic?](#23-decision-tree)
24. [Bản đồ tư duy tổng](#24-mind-map)

---

## 1. Bức tranh lớn

```
   ╔═══════════════════════════════════════════════════════════╗
   ║                       TRAIT                                ║
   ║                                                            ║
   ║   = "Khả năng" mà một TYPE có thể có                       ║
   ║                                                            ║
   ║   Hợp đồng:                                                ║
   ║     "Type X tuyên bố có khả năng Y"                        ║
   ║                                                            ║
   ║   Code:                                                    ║
   ║     trait Y {                                              ║
   ║         fn method1(&self) -> ...;                          ║
   ║     }                                                      ║
   ║                                                            ║
   ║     impl Y for X {                                         ║
   ║         fn method1(&self) -> ... { ... }                   ║
   ║     }                                                      ║
   ║                                                            ║
   ╚═══════════════════════════════════════════════════════════╝

   Ví dụ thực:
   ─────────
   
   trait Bark { fn bark(&self); }
        │
        │ "khả năng sủa"
        │
        ├──► impl Bark for Dog { ... }     ✓ Dog có khả năng sủa
        ├──► impl Bark for Wolf { ... }    ✓ Wolf có khả năng sủa
        └──► impl Bark for Cat { ... }     ❌ không có (sai semantic)
```

---

## 2. Trait vs Class

```
   ═══════════════════════════════════════════════════════════
                       JAVA / C++ (OOP)
   ═══════════════════════════════════════════════════════════
   
                   ┌──────────────┐
                   │   Animal     │  ← base class
                   │  (abstract)  │
                   └──────┬───────┘
                          │ extends
                  ┌───────┴───────┐
                  ▼               ▼
            ┌─────────┐     ┌─────────┐
            │  Dog    │     │  Cat    │
            └─────────┘     └─────────┘
   
   Quan hệ: IS-A (kế thừa cứng)
   Field + method từ parent → child
   "Dog IS-A Animal"


   ═══════════════════════════════════════════════════════════
                          RUST (Trait)
   ═══════════════════════════════════════════════════════════
   
   trait Bark   ──┐                ┌── struct Dog
   trait Run    ──┤                │   { name, ... }
   trait Sleep  ──┤                │
                  │  CAPABILITIES  │
                  │     ┌──────────┘
                  │     │
                  ▼     ▼
                 ┌──────────┐
                 │   Dog    │
                 │ impl Bark│  ← "has capability"
                 │ impl Run │
                 │ impl Sleep│
                 └──────────┘
   
   Quan hệ: HAS-CAPABILITY (composition + trait)
   Field tự định nghĩa, capability mix-and-match
```

---

## 3. Khai báo & implement

```rust
trait Greet {
    fn hello(&self) -> String;
}

struct Vietnamese;
struct English;

impl Greet for Vietnamese {
    fn hello(&self) -> String { String::from("Xin chào") }
}

impl Greet for English {
    fn hello(&self) -> String { String::from("Hello") }
}
```

### Sơ đồ

```
                         trait Greet
                         ┌──────────┐
                         │ hello()  │  ← yêu cầu signature
                         └──────────┘
                              ▲
                              │ impl
                ┌─────────────┼─────────────┐
                │             │             │
                │             │             │
          ┌─────┴─────┐ ┌─────┴─────┐
          │Vietnamese │ │  English  │
          │           │ │           │
          │ hello()→  │ │ hello()→  │
          │"Xin chào" │ │"Hello"    │
          └───────────┘ └───────────┘

   Mỗi struct ĐỘC LẬP impl trait theo cách riêng.
```

### Gọi method

```rust
let v = Vietnamese;
println!("{}", v.hello());     // → "Xin chào"

let e = English;
println!("{}", e.hello());     // → "Hello"
```

```
       compile time
   ┌──────────────────┐
   │ v.hello() ───────┼──► resolve to Vietnamese::hello
   │ e.hello() ───────┼──► resolve to English::hello
   └──────────────────┘
   
   Compiler biết v: Vietnamese → direct call (static)
```

---

## 4. Default method

```rust
trait Print {
    fn name(&self) -> String;              // required
    
    fn announce(&self) {                   // default
        println!("Tôi là {}", self.name());
    }
}

struct User { name: String }

impl Print for User {
    fn name(&self) -> String { self.name.clone() }
    // KHÔNG impl announce → dùng default
}
```

### Sơ đồ

```
   trait Print:
   ┌────────────────────────────────────┐
   │ fn name() -> String     [required] │ ← phải impl
   ├────────────────────────────────────┤
   │ fn announce() {                    │ ← có body sẵn
   │     println("Tôi là {}", name())   │
   │ }                                   │
   └────────────────────────────────────┘
                  │
                  │ impl Print for User
                  ▼
   ┌────────────────────────────────────┐
   │ User:                              │
   │   name() → "Alice"                 │
   │   announce() ← inherit default     │
   └────────────────────────────────────┘
   
   user.announce()  →  "Tôi là Alice"
```

### Override

```rust
impl Print for VIP {
    fn name(&self) -> String { ... }
    
    fn announce(&self) {                   // override
        println!("✨ {} ✨", self.name());
    }
}
```

```
   trait Print default                  VIP override
   ─────────────────                    ────────────
   announce: print "Tôi là X"           announce: print "✨ X ✨"
```

---

## 5. Supertrait

```rust
trait Animal {
    fn name(&self) -> String;
}

trait Pet: Animal {        // Pet REQUIRES Animal
    fn owner(&self) -> String;
}
```

### Sơ đồ phân cấp

```
                    ┌─────────────┐
                    │   Animal    │  ← supertrait
                    │             │
                    │  name()     │
                    └─────┬───────┘
                          │ "REQUIRES"
                          │
                    ┌─────┴───────┐
                    │     Pet     │  ← subtrait
                    │             │
                    │  owner()    │
                    └─────────────┘
   
   Type nào impl Pet → phải impl Animal trước:
   
   impl Animal for Dog { fn name() {...} }   ✓ required
   impl Pet    for Dog { fn owner() {...} }  ✓ enabled
   
   Thiếu Animal → ERROR khi impl Pet
```

### Nhiều supertrait

```rust
trait SuperPet: Animal + Display + Clone {
    fn intro(&self) {
        // dùng được name() (từ Animal), Display ({}), clone()
    }
}
```

```
        Animal      Display     Clone
           ▲           ▲          ▲
           │           │          │
           └─────┬─────┴─────┬────┘
                 │           │
                 ▼           ▼
                ┌─────────────┐
                │  SuperPet   │  ← cần CẢ 3
                └─────────────┘
```

---

## 6. Method vs Associated Function

```rust
trait Shape {
    fn area(&self) -> f64;            // METHOD (có self)
    fn unit() -> Self;                 // ASSOCIATED FN (no self)
}
```

### Sơ đồ gọi

```
   Method:                            Associated fn:
   ──────                             ──────────────
   
   instance.method(args)              Type::function(args)
        │                                  │
        ▼                                  ▼
   ┌─────────┐                       ┌──────────┐
   │ self ── │                       │   args   │
   │ args    │                       │          │
   └─────────┘                       └──────────┘
   
   c = Circle { ... }                 c = Circle::unit()
   a = c.area()                       (constructor)
```

---

## 7. Static dispatch — Monomorphization

### Code bạn viết

```rust
fn print<T: Greet>(x: &T) {
    println!("{}", x.hello());
}

let v = Vietnamese;
let e = English;
print(&v);
print(&e);
```

### Compiler sinh ra

```
   ╔══════════════════════════════════════════════════════════════╗
   ║              MONOMORPHIZATION (compile time)                  ║
   ╠══════════════════════════════════════════════════════════════╣
   ║                                                                ║
   ║  Phát hiện: print được gọi với T = Vietnamese, English         ║
   ║                                                                ║
   ║  → CLONE function thành 2 bản:                                 ║
   ║                                                                ║
   ║  fn print__Vietnamese(x: &Vietnamese) {                        ║
   ║      println!("{}", x.hello());     ← inline Vietnamese::hello║
   ║  }                                                             ║
   ║                                                                ║
   ║  fn print__English(x: &English) {                              ║
   ║      println!("{}", x.hello());     ← inline English::hello   ║
   ║  }                                                             ║
   ║                                                                ║
   ║  Caller code đổi thành:                                        ║
   ║      print__Vietnamese(&v);                                    ║
   ║      print__English(&e);                                       ║
   ║                                                                ║
   ╚══════════════════════════════════════════════════════════════╝
```

### Sơ đồ binary

```
   BEFORE monomorphization:           AFTER:
   ──────────                          ─────
                                      
   fn print<T>(x: &T)                  fn print__Vietnamese(x: &V)
        │                              fn print__English(x: &E)
        │ (generic,                    fn print__Dog(x: &D)
        │  chưa thực sự ở              ...
        │  binary)                     (nhiều function, mỗi cái
        │                              optimize cho type cụ thể)
        ▼
   1 placeholder                       N functions trong binary
```

### Cost: code size tăng

```
   Code size:
   ──────────
   
   Generic 1000-line × 10 types = 10,000 lines code thật trong binary
   
   ↑ tăng binary size
   ↓ giảm icache hit nếu binary quá lớn
   
   Trade-off: speed ↑↑↑ vs size ↑↑
```

---

## 8. Dynamic dispatch — vtable

### Code

```rust
let animals: Vec<Box<dyn Greet>> = vec![
    Box::new(Vietnamese),
    Box::new(English),
];

for a in &animals {
    println!("{}", a.hello());
}
```

### Sơ đồ memory

```
   STACK:
   ┌──────────────┐
   │ animals.ptr  │──┐
   │ .len = 2     │  │
   │ .cap = 2     │  │
   └──────────────┘  │
                     ▼
   HEAP (Vec backing):
   ┌──────────────────────────────────────────────────┐
   │  Element 0 (Box<dyn Greet>):                     │
   │    data_ptr ───────► Vietnamese instance        │
   │    vtable_ptr ─────► VTABLE_Vietnamese_Greet    │
   │                                                  │
   │  Element 1 (Box<dyn Greet>):                     │
   │    data_ptr ───────► English instance           │
   │    vtable_ptr ─────► VTABLE_English_Greet       │
   └──────────────────────────────────────────────────┘
   
   TEXT segment (static, in binary):
   ┌──────────────────────────────────────────────────┐
   │ VTABLE_Vietnamese_Greet:                         │
   │   destructor: drop_in_place::<Vietnamese>        │
   │   size: 0                                         │
   │   align: 1                                        │
   │   method[0]: <Vietnamese as Greet>::hello       │ ◄── implementation
   │                                                   │     thực sự
   │ VTABLE_English_Greet:                            │
   │   destructor: drop_in_place::<English>           │
   │   size: 0                                         │
   │   align: 1                                        │
   │   method[0]: <English as Greet>::hello          │
   └──────────────────────────────────────────────────┘
```

### Diễn biến `a.hello()`

```
   for a in &animals:
       │
       ▼
   a = &Box<dyn Greet>
       │
       ▼
   a.hello():
       ┌────────────────────────────────────────────┐
       │ 1. Lấy vtable_ptr từ fat pointer           │
       │    vtable = *(a + 8)                       │
       │                                            │
       │ 2. Lookup method[0] trong vtable           │
       │    hello_fn = vtable.method[0]             │
       │                                            │
       │ 3. Indirect call:                          │
       │    hello_fn(data_ptr)                      │
       └────────────────────────────────────────────┘
       
   → 1 memory load + 1 indirect call
   → ~2-3 ns extra cost
```

---

## 9. Fat pointer

### So sánh pointer thường vs trait object

```
   ┌─────────────────────────────────────────────────────────┐
   │            Box<Vietnamese>  (concrete)                  │
   ├─────────────────────────────────────────────────────────┤
   │                                                         │
   │   STACK:                                                │
   │   ┌──────────────┐                                      │
   │   │ ptr (8 byte) │──► Vietnamese instance              │
   │   └──────────────┘                                      │
   │                                                         │
   │   Compiler đã biết:                                     │
   │   - sizeof(Vietnamese) = 0                              │
   │   - method hello() ở địa chỉ X                          │
   │                                                         │
   └─────────────────────────────────────────────────────────┘
   
   ┌─────────────────────────────────────────────────────────┐
   │            Box<dyn Greet>  (trait object)               │
   ├─────────────────────────────────────────────────────────┤
   │                                                         │
   │   STACK:                                                │
   │   ┌──────────────┐                                      │
   │   │ data_ptr     │──► instance (Vietnamese hoặc Eng)   │
   │   │   (8 byte)   │                                      │
   │   ├──────────────┤                                      │
   │   │ vtable_ptr   │──► vtable trong TEXT                │
   │   │   (8 byte)   │                                      │
   │   └──────────────┘                                      │
   │                                                         │
   │   FAT POINTER: 16 byte                                  │
   │                                                         │
   │   Compiler KHÔNG biết:                                  │
   │   - sizeof(?) — không cố định                           │
   │   - method ở địa chỉ nào — cần lookup                   │
   │                                                         │
   │   Do đó cần thêm vtable_ptr.                            │
   │                                                         │
   └─────────────────────────────────────────────────────────┘
```

### Vì sao 2 pointer?

```
   Câu hỏi: "Box chứa dyn Greet" — Box phải chứa gì?
   
   Khi unbox & call method:
   1. Cần biết DATA (instance ở đâu) → data_ptr
   2. Cần biết METHOD (function ở đâu) → vtable_ptr
   
   → Cả 2 ptr → 16 byte → "fat" pointer
```

---

## 10. Layout vtable

```
                  ╔═══════════════════════════════════╗
                  ║   VTable cho (Type, Trait)       ║
                  ╠═══════════════════════════════════╣
                  ║                                   ║
                  ║   Header (mọi vtable):           ║
                  ║   ┌─────────────────────────┐    ║
                  ║   │ destructor fn ptr       │    ║ ◄ drop khi free
                  ║   ├─────────────────────────┤    ║
                  ║   │ size: usize             │    ║ ◄ sizeof instance
                  ║   ├─────────────────────────┤    ║
                  ║   │ align: usize            │    ║ ◄ alignof instance
                  ║   ├─────────────────────────┤    ║
                  ║                                   ║
                  ║   Methods của trait:              ║
                  ║   ┌─────────────────────────┐    ║
                  ║   │ method_0 fn ptr ────────┼──► impl1
                  ║   ├─────────────────────────┤    ║
                  ║   │ method_1 fn ptr ────────┼──► impl2
                  ║   ├─────────────────────────┤    ║
                  ║   │ method_N fn ptr ────────┼──► implN
                  ║   └─────────────────────────┘    ║
                  ║                                   ║
                  ╚═══════════════════════════════════╝
```

### Ví dụ cụ thể

```rust
trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> String;
    fn age(&self) -> u32;
}
```

```
   VTABLE_Dog_Animal:           VTABLE_Cat_Animal:
   ┌────────────────────┐       ┌────────────────────┐
   │ drop_dog            │      │ drop_cat            │
   │ size: 24            │      │ size: 16            │
   │ align: 8            │      │ align: 8            │
   ├────────────────────┤       ├────────────────────┤
   │ Dog::name           │      │ Cat::name           │
   │ Dog::sound          │      │ Cat::sound          │
   │ Dog::age            │      │ Cat::age            │
   └────────────────────┘       └────────────────────┘
   
   STATIC trong binary, mỗi (Type, Trait) có 1 vtable duy nhất.
```

---

## 11. So sánh Static vs Dynamic

```
   ═════════════════════════════════════════════════════════════
                          STATIC DISPATCH
   ═════════════════════════════════════════════════════════════
   
   fn process<T: Greet>(x: &T)
              ────────
              monomorphize
   
   Compile → multiple copies:
   ┌────────┐  ┌────────┐  ┌────────┐
   │process │  │process │  │process │
   │__Viet  │  │__Eng   │  │__Dog   │  ...
   └────────┘  └────────┘  └────────┘
   
   ⚡ Speed:    ⚡⚡⚡  (inline, optimized per type)
   📦 Binary:   ↑↑   (nhiều bản code)
   🧠 Compile:  🐢   (slow)
   
   
   ═════════════════════════════════════════════════════════════
                         DYNAMIC DISPATCH
   ═════════════════════════════════════════════════════════════
   
   fn process(x: &dyn Greet)
                 ──────────
                 trait object
   
   Compile → 1 copy:
   ┌────────┐
   │process │ ← vtable lookup mỗi call
   └────────┘
   
   ⚡ Speed:    ⚡⚡   (~2-3 ns extra per call)
   📦 Binary:   ↓    (chỉ 1 bản code)
   🧠 Compile:  ⚡    (fast)
   
   Lookup chain:
       call x.hello()
            │
            ▼ load vtable_ptr
            │
            ▼ index method
            │
            ▼ indirect call
            │
            ▼ execute
```

### Benchmark visual

```
   100,000,000 calls trong loop:
   
   Static dispatch:    ███ 0.3s
   Dynamic dispatch:   ██████ 0.6s
   
   → Khác biệt ~2x cho hot loop
   → Cho app thường: không cảm nhận được
```

---

## 12. Object safety — rule trực quan

```
   ═══════════════════════════════════════════════════════════
                       OBJECT-SAFE RULES
   ═══════════════════════════════════════════════════════════
   
   ✅ ĐƯỢC làm dyn:                  ❌ KHÔNG được:
   ────────────                       ─────────────
   
   trait Good {                       trait Bad1 {
       fn m(&self);                       fn m<T>(&self, x: T);
       fn m2(&self) -> i32;          }   //  ↑ generic
   }                                  
                                      trait Bad2 {
   trait WithDefault {                    fn m(&self) -> Self;
       fn m(&self) {...}              }   //  ↑ return Self
   }                                  
                                      trait Bad3 {
                                          fn helper() -> i32;
                                      }   //  ↑ no self / static
```

### Quy tắc visual

```
   ┌─────────────────────────────────────────────────┐
   │  Mỗi method trong trait phải:                   │
   │                                                  │
   │  [ ] Có &self / &mut self / self / Box<self>    │
   │      (NOT static)                                │
   │                                                  │
   │  [ ] Không có generic <T> trên method            │
   │      (Lifetime OK, type generic NO)              │
   │                                                  │
   │  [ ] Return NOT Self                             │
   │      (trừ khi dùng &Self / &mut Self / Box<Self>)│
   │                                                  │
   │  [ ] Không where Self: Sized                     │
   │                                                  │
   │  Nếu MỌI method thoả → trait là OBJECT-SAFE     │
   │  → có thể dùng dyn Trait                        │
   └─────────────────────────────────────────────────┘
```

### Vì sao? Trực quan

```
   Generic method:
   ────────────
   trait Bad { fn m<T>(&self, x: T); }
   
   vtable cần lưu m::<i32>, m::<f64>, m::<String>, ...
   Vô hạn instantiation → KHÔNG ĐỦ chỗ trong vtable
   → CẤM
   
   Return Self:
   ───────────
   trait Bad { fn clone(&self) -> Self; }
   
   Caller có &dyn Trait, không biết Self là gì.
   → KHÔNG biết stack alloc bao nhiêu cho return value
   → CẤM
   
   Static fn:
   ────────
   trait Bad { fn unit() -> Self; }
   
   Gọi qua dyn nào? `(dyn Trait)::unit()` — không có instance
   → KHÔNG hợp lý
   → CẤM (trừ khi có where Self: Sized)
```

---

## 13. Associated type

```rust
trait Container {
    type Item;
    fn get(&self, i: usize) -> Self::Item;
}
```

### Sơ đồ

```
   ┌──────────────────────────┐
   │ trait Container          │
   │ ┌──────────────────────┐ │
   │ │ type Item: ???       │ │  ← placeholder
   │ │ fn get → Self::Item  │ │
   │ └──────────────────────┘ │
   └──────────────────────────┘
              │
              │ impl Container for Vec<i32>
              ▼
   ┌──────────────────────────┐
   │ Vec<i32>                 │
   │ ┌──────────────────────┐ │
   │ │ type Item = i32      │ │  ← fill in
   │ │ fn get → i32          │ │
   │ └──────────────────────┘ │
   └──────────────────────────┘
              │
              │ impl Container for Vec<String>
              ▼
   ┌──────────────────────────┐
   │ Vec<String>              │
   │ ┌──────────────────────┐ │
   │ │ type Item = String   │ │
   │ │ fn get → String      │ │
   │ └──────────────────────┘ │
   └──────────────────────────┘
   
   Mỗi type chỉ impl Container 1 LẦN.
   Item là HỆ QUẢ của type, không phải chọn lúc gọi.
```

---

## 14. Generic parameter trên trait

```rust
trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<String> for i32 {
    fn convert(self) -> String { self.to_string() }
}

impl Convert<f64> for i32 {
    fn convert(self) -> f64 { self as f64 }
}
```

### Sơ đồ

```
                trait Convert<T>
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   Convert<String> Convert<f64>  Convert<...>
        │              │
        │ impl for i32 │ impl for i32
        ▼              ▼
   ┌──────────┐   ┌──────────┐
   │ i32 →    │   │ i32 →    │
   │ String   │   │ f64      │
   └──────────┘   └──────────┘
   
   i32 có NHIỀU impl Convert với T khác nhau.
   Mỗi T = 1 impl khác nhau.
```

### So sánh

```
   ASSOCIATED TYPE                  GENERIC PARAMETER
   ───────────────                  ─────────────────
   
   type Item                        <T>
   
   1 type → 1 impl                  1 type → N impl
   Output PHỤ THUỘC type            Output CHỌN lúc gọi
   
   Iterator                         From
   trait Iterator {                 trait From<T> {
       type Item;                       fn from(t: T) -> Self;
   }                                }
   
   Vec<i32>::iter() → Item=&i32     i32: From<u8>, From<i8>, ...
   (chỉ 1 cách)                     (nhiều)
```

---

## 15. Send/Sync flow

### Send: chuyển ownership giữa thread

```
   Thread A                         Thread B
   ┌──────────────┐                 ┌──────────────┐
   │              │                 │              │
   │   value: T   │ ─── move ─────►│   value: T   │
   │              │                 │              │
   └──────────────┘                 └──────────────┘
   
   Hợp pháp KHI: T: Send
   
   T: Send                          T: NOT Send
   ──────                            ──────────
   i32, String, Vec                  Rc<U>     (count không atomic)
   Box<T> if T: Send                 *mut U    (raw ptr)
   Arc<U> if U: Send + Sync          MutexGuard (lock thread-specific)
```

### Sync: chia sẻ &T giữa thread

```
   Thread A                         Thread B
   ┌──────────────┐                 ┌──────────────┐
   │   &value ────┼─── share ──────┼──► &value    │
   │              │                 │              │
   └──────────────┘                 └──────────────┘
          │                                ▲
          │                                │
          └────► value: T (trên heap) ◄────┘
   
   Hợp pháp KHI: T: Sync
   
   T: Sync                          T: NOT Sync
   ──────                            ──────────
   i32, String                       Cell<U>    (interior mut, không atomic)
   Mutex<U>                          RefCell<U> (counter không atomic)
   AtomicI32                         Rc<U>
   &T if T: Sync
```

### Mối liên hệ Send/Sync

```
   T: Sync   ⟺   &T: Send
   
   Đọc:
   "T có thể chia sẻ giữa thread"
   tương đương
   "&T có thể chuyển giữa thread"
```

### Auto-derivation

```
   struct MyData {
       a: i32,        ← Send + Sync
       b: String,     ← Send + Sync
       c: Vec<i32>,   ← Send + Sync
   }
   
   → MyData TỰ ĐỘNG: Send + Sync
   
   struct MyDataRc {
       a: i32,
       b: Rc<i32>,    ← NOT Send, NOT Sync
   }
   
   → MyDataRc TỰ ĐỘNG: NOT Send, NOT Sync
   (1 field xấu → cả struct xấu)
```

---

## 16. Orphan rule

```
   ═══════════════════════════════════════════════════════════
   QUY TẮC:  Để impl Trait for Type, ÍT NHẤT MỘT phải LOCAL
   ═══════════════════════════════════════════════════════════
   
                Trait local?   Type local?    OK?
                ────────────   ───────────    ───
                    ✓               ✓          ✓
                    ✓               ✗          ✓
                    ✗               ✓          ✓
                    ✗               ✗          ✗ ORPHAN!
```

### Sơ đồ

```
   YOUR CRATE                         EXTERNAL CRATES
   ┌──────────────┐                   ┌──────────────┐
   │ MyStruct     │                   │ Vec, String  │
   │ MyTrait      │                   │ Display      │
   └──────────────┘                   └──────────────┘
   
   ✓ impl MyTrait for MyStruct        (cả 2 local)
   ✓ impl MyTrait for Vec<i32>        (trait local)
   ✓ impl Display for MyStruct        (type local)
   ✗ impl Display for Vec<i32>        (NEITHER local!) ← orphan
```

### Vì sao cần rule này?

```
   Giả sử không có orphan rule:
   
   Crate A:                Crate B:
   ┌─────────────────┐     ┌─────────────────┐
   │ impl Display    │     │ impl Display    │
   │   for Vec<i32>  │     │   for Vec<i32>  │
   │   → "[1,2,3]"   │     │   → "[1, 2, 3]" │
   └─────────────────┘     └─────────────────┘
   
   User crate:
   ┌─────────────────┐
   │ use A;          │
   │ use B;          │
   │ vec.fmt(...) ?  │  ← AMBIGUOUS, impl nào?
   └─────────────────┘
   
   → Đổ vỡ ecosystem. CẤM ngay từ đầu.
```

---

## 17. Newtype pattern

```rust
struct MyVec(Vec<i32>);            // wrap

impl Display for MyVec {           // ✓ MyVec local!
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "...")
    }
}
```

### Sơ đồ

```
   Vấn đề:                        Giải pháp:
   ──────                          ─────────
   
   impl Display for Vec<i32>      struct MyVec(Vec<i32>);
   ❌ orphan                       │
                                  │ wrap
                                  ▼
                                  impl Display for MyVec  ✓
                                  
                                  let v = MyVec(vec![1,2,3]);
                                  println!("{}", v);
```

### Layout: zero-cost wrapper

```
   Vec<i32>:                      MyVec(Vec<i32>):
   ──────                          ────────────────
   
   STACK (24 byte):                STACK (24 byte):
   ┌──────────┐                    ┌──────────┐
   │ ptr      │                    │ ptr      │
   │ len      │                    │ len      │
   │ cap      │                    │ cap      │
   └──────────┘                    └──────────┘
                                   
                                   ↑ y hệt Vec! Không thêm gì.
```

→ Newtype = **zero runtime cost**.

---

## 18. Blanket implementation

```rust
impl<T: Display> ToString for T {
    fn to_string(&self) -> String {
        format!("{}", self)
    }
}
```

### Sơ đồ phổ quát

```
   Universal quantifier:
   ──────────────────
   
   ∀ T mà T: Display:
       T tự động có ToString
   
   Tức là:
   ─────
   
   i32: Display       → i32: ToString
   f64: Display       → f64: ToString
   String: Display    → String: ToString
   MyType: Display    → MyType: ToString    (tự động!)
   ...
   
   Không cần impl thủ công cho từng type.
```

### Ứng dụng

```rust
// Bạn impl Display:
impl Display for User { ... }

// Tự dưng có:
let u = User { ... };
let s: String = u.to_string();     // ← blanket impl cho miễn phí
```

---

## 19. Iterator — chain & lazy

```rust
let v = vec![1, 2, 3, 4, 5];
let r: Vec<i32> = v.iter()
    .map(|x| x * 2)
    .filter(|x| x > &4)
    .collect();
```

### Sơ đồ lazy

```
   ITER STATE MACHINE:
   
   v.iter()      →  Iter (lazy, không tính gì)
       │
       ▼
   .map(|x| x*2) →  Map<Iter, F> (vẫn lazy)
       │
       ▼
   .filter(...)  →  Filter<Map<Iter, F>, P> (vẫn lazy)
       │
       ▼
   .collect()    →  TRIGGER thực hiện!
                    
   Pipeline mỗi item:
   ─────────────────
   
   1 → map → 2 → filter (2 > 4? NO) → discard
   2 → map → 4 → filter (4 > 4? NO) → discard
   3 → map → 6 → filter (6 > 4? YES) → push to Vec
   4 → map → 8 → filter (8 > 4? YES) → push to Vec
   5 → map → 10 → filter (10 > 4? YES) → push to Vec
   
   r = [6, 8, 10]
```

### Quan trọng: KHÔNG có Vec trung gian

```
   Naive (Java/Python style):         Rust idiom:
   ─────────                          ────────
   
   tmp1 = v.map(...)                  Pipeline mỗi item
   tmp2 = tmp1.filter(...)            qua chain rồi mới đến item kế
   result = tmp2.collect()            
   
   Memory: 3 Vec trung gian            Memory: chỉ result Vec
   
   ┌────┐ ┌────┐ ┌────┐                ┌────┐
   │tmp1│ │tmp2│ │ rs │                │ rs │
   └────┘ └────┘ └────┘                └────┘
```

### Zero-cost

```
   Code Rust:                          Code C tương đương:
   ──────────                          ──────────────────
   
   (0..1000)                           int sum = 0;
       .filter(|x| x%2==0)              for (int i = 0; i < 1000; i++) {
       .sum()                                if (i%2 == 0) sum += i;
                                        }
                                        
   Sau optimize:                       Tốc độ:
   y hệt code C bên phải               Y HỆT NHAU (~ns khác biệt)
```

---

## 20. Deref coercion chain

```rust
let s: String = "hi".to_string();
fn need_str(x: &str) {}
need_str(&s);          // &String → &str
```

### Sơ đồ chain

```
   Pass &String đến hàm cần &str:
   
   &String
      │
      │ String: Deref<Target = str>
      ▼
   &str    ✓ match!
```

### Nhiều cấp

```
   let b: Box<Rc<String>> = Box::new(Rc::new("hi".to_string()));
   need_str(&b);
   
   &Box<Rc<String>>
      │
      │ Box: Deref<Target = Rc<String>>
      ▼
   &Rc<String>
      │
      │ Rc: Deref<Target = String>
      ▼
   &String
      │
      │ String: Deref<Target = str>
      ▼
   &str    ✓
   
   Compiler thử mọi cấp đến khi khớp.
```

### Cảnh báo: chỉ cho smart pointer

```
   ✓ Đúng dùng:
   impl Deref for Box<T> { ... }
   impl Deref for Rc<T> { ... }
   impl Deref for Arc<T> { ... }
   impl Deref for MutexGuard<T> { ... }
   
   ❌ Sai dùng:
   impl Deref for User { type Target = String; ... }
   
   → confuse: "user.len() là String::len? Hay User method?"
```

---

## 21. From → Into auto

```rust
impl From<i32> for MyNum {
    fn from(x: i32) -> MyNum { ... }
}

// TỰ ĐỘNG có:
// impl Into<MyNum> for i32 { ... }
```

### Cơ chế

```
   trong std:
   ─────────
   
   impl<T, U: From<T>> Into<U> for T {
       fn into(self) -> U {
           U::from(self)
       }
   }
   
   ↑ Blanket impl
```

### Sơ đồ

```
   Bạn impl:                          Auto sinh:
   ──────────                          ──────────
   
   impl From<i32> for MyNum           impl Into<MyNum> for i32
        │                                  │
        │ (chỉ cần 1 chiều)                │ (chiều ngược tự có)
        ▼                                  ▼
   MyNum::from(5)                     5.into()
        │                                  │
        └───── cùng kết quả ───────────────┘
                  MyNum
```

### Quy tắc đẹp

```
   ┌─────────────────────────────────────┐
   │ LUÔN impl From, KHÔNG impl Into     │
   │                                     │
   │ → 1 dòng code, được cả 2 chiều     │
   │ → Lỗi compile dễ đọc hơn            │
   └─────────────────────────────────────┘
```

---

## 22. Bản đồ trait stdlib

```
                  ┌────────────────────────────────────┐
                  │           STDLIB TRAITS            │
                  └────────────────────────────────────┘
                                  │
       ┌──────────────────────────┼──────────────────────────┐
       ▼                          ▼                          ▼
   CONVERSION                COLLECTION              EQUALITY/ORDER
   ──────────                ──────────              ──────────────
   From/Into                 Iterator                PartialEq
   TryFrom/TryInto           IntoIterator            Eq
   FromStr                   FromIterator            PartialOrd
   ToString                  Extend                  Ord
                                                     Hash
   
       
       ┌──────────────────────────────────────────────┐
       ▼                                              ▼
   FORMATTING                                    SMART POINTER
   ──────────                                    ─────────────
   Display                                       Deref / DerefMut
   Debug                                         Drop
   Write                                         AsRef / AsMut
                                                 Borrow / BorrowMut
   
   
       ┌──────────────────────────────────────────────┐
       ▼                                              ▼
   OPERATORS                                     MARKERS
   ─────────                                     ───────
   Add / Sub / Mul                               Send / Sync
   Neg / Not                                     Copy / Clone
   Index / IndexMut                              Sized / Unpin
   Range                                         
   
   
       ┌──────────────────────────────────────────────┐
       ▼                                              ▼
   ERROR                                         CLOSURE
   ─────                                         ───────
   Error                                         Fn / FnMut / FnOnce
   (in std::error)                               
   
   
                  ┌────────────────────────┐
                  ▼                        ▼
                ASYNC                     I/O
                ─────                     ───
                Future                    Read / Write
                IntoFuture                BufRead
                AsyncRead/Write           Seek
```

---

## 23. Decision tree: static hay dynamic?

```
                  ┌─────────────────────────────────┐
                  │   TÔI CẦN POLYMORPHISM           │
                  └────────────────┬────────────────┘
                                   │
                                   ▼
                  ┌─────────────────────────────────┐
                  │ Tôi có biết TYPE lúc compile?   │
                  └────────────────┬────────────────┘
                                   │
                ┌──────────────────┴──────────────────┐
                │                                     │
              CÓ                                    KHÔNG
                │                                     │
                ▼                                     ▼
   ┌──────────────────────────┐         ┌──────────────────────────┐
   │     STATIC DISPATCH      │         │   Tôi cần GIỮ NHIỀU TYPE │
   │                          │         │   KHÁC NHAU?             │
   │  fn f<T: Trait>(x: T)    │         └────────────┬─────────────┘
   │  fn f(x: impl Trait)     │                      │
   │                          │              ┌───────┴───────┐
   │  - Inline                │              │               │
   │  - Zero-cost             │              CÓ              KHÔNG
   │  - Code bloat            │              │               │
   └──────────────────────────┘              ▼               ▼
                                  ┌────────────────┐  ┌───────────────┐
                                  │  DYNAMIC       │  │ Đo benchmark? │
                                  │                │  │               │
                                  │ Vec<Box<dyn T>>│  │ Static nếu hot│
                                  │ &dyn Trait     │  │ loop          │
                                  │                │  │ Dynamic nếu   │
                                  │ - Vtable       │  │ flex hơn      │
                                  │ - Linh hoạt    │  │               │
                                  │ - Slower ~2x   │  │               │
                                  └────────────────┘  └───────────────┘
```

### Use cases thực

```
   STATIC dispatch dùng cho:        DYNAMIC dispatch dùng cho:
   ─────────────────                ──────────────────
   
   - Iterator chain                 - GUI widget tree
   - Math/SIMD functions            - Plugin system
   - Generic data structures        - Heterogeneous collections
   - Hot loops                      - Error trait (Box<dyn Error>)
   - Small functions                - Strategy pattern
                                    - Callback registry
```

---

## 24. Mind Map — Bản đồ tư duy tổng

```
                                  TRAIT
                                    │
       ┌────────────────────────────┼────────────────────────────┐
       ▼                            ▼                            ▼
   DEFINE                       USE                           PATTERN
                                                                
   ┌────────┐              ┌─────┴─────┐                  ┌────┴────┐
   ▼        ▼              ▼           ▼                  ▼         ▼
  trait   impl          STATIC      DYNAMIC            Common    Advanced
  ───     ────          ──────      ───────            ──────    ────────
  required for ──┐      <T:Trait>   dyn Trait          From      GAT
  default    Type│      impl Trait  &dyn               Iterator  Auto traits
  supertrait     │      monomorph   vtable             Display   HRTB
  assoc type     │      0-cost      ~2-3 ns extra      Deref     Marker
                 │      inline      fat ptr 16B        Drop      (Send/Sync)
                 │      code bloat  obj-safe           Eq/Hash   Object-safe
                 ▼
              ┌─────────────────────────────────────────────────┐
              │              COHERENCE                          │
              │  Orphan rule      → newtype workaround          │
              │  Blanket impl     → universal coverage          │
              │  No overlap       → 1 impl per (Trait, Type)    │
              └─────────────────────────────────────────────────┘
```

### Quan hệ Trait với Memory Model

```
   ┌─────────────────────────────────────────────────────┐
   │                    Trait                            │
   │                                                     │
   │   STATIC DISPATCH                                   │
   │   ────────────                                      │
   │   - Sinh nhiều function trong TEXT segment          │
   │   - Inline → đẩy code vào caller frame              │
   │   - Code bloat ↑                                    │
   │                                                     │
   │   DYNAMIC DISPATCH                                  │
   │   ────────────                                      │
   │   - Fat pointer (16 byte) trên stack                │
   │   - data_ptr → instance (heap thường)               │
   │   - vtable_ptr → vtable static trong TEXT           │
   │   - 1 indirect call → khả năng cache miss icache   │
   │                                                     │
   │   AUTO TRAITS (Send/Sync)                           │
   │   ──────────                                        │
   │   - Compile-time contract về memory ownership       │
   │   - Không có runtime cost                           │
   │   - Tương đương Rc → !Send, Arc → Send (vì atomic) │
   │                                                     │
   └─────────────────────────────────────────────────────┘
```

---

## Tổng kết — 10 ý cốt lõi

```
   ╔════════════════════════════════════════════════════════════╗
   ║                                                            ║
   ║  1. Trait = contract về CAPABILITY, không phải class       ║
   ║                                                            ║
   ║  2. Type không inherit type, chỉ có trait (composition)    ║
   ║                                                            ║
   ║  3. Static dispatch = monomorphization (compile-time)      ║
   ║     → nhanh, code lớn                                      ║
   ║                                                            ║
   ║  4. Dynamic dispatch = vtable (runtime)                    ║
   ║     → linh hoạt, code nhỏ, +2-3ns/call                     ║
   ║                                                            ║
   ║  5. dyn Trait = fat pointer (data + vtable), 16 byte       ║
   ║                                                            ║
   ║  6. Object-safe trait mới dùng được dạng dyn               ║
   ║     (no generic method, no return Self)                    ║
   ║                                                            ║
   ║  7. Orphan rule: Trait hoặc Type phải LOCAL                ║
   ║     → workaround: newtype pattern (zero-cost)              ║
   ║                                                            ║
   ║  8. Associated type: 1 type, 1 impl, 1 Item                ║
   ║     Generic param: 1 type, N impl, N output                ║
   ║                                                            ║
   ║  9. Default method giúp trait scale                        ║
   ║     (Iterator: 1 required + 70 default)                    ║
   ║                                                            ║
   ║ 10. Marker trait (Send/Sync/Copy) = auto-derived contract  ║
   ║     compiler kiểm tra, zero runtime cost                   ║
   ║                                                            ║
   ╚════════════════════════════════════════════════════════════╝
```

---

> Đọc song song `trait.md` (lý thuyết) để hiểu nguyên lý, file này để có hình ảnh trong đầu.
>
> Chủ đề tiếp theo: **Generic** — xây trên trait bounds, hiểu thêm về monomorphization, where clauses, lifetimes trong generic, phantom types, const generics.
