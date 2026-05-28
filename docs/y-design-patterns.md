# Design Patterns trong Rust — Từ Zero Đến Senior

> Design pattern không phải là "code mẫu để copy". Nó là **từ vựng chung** để mô tả giải pháp cho những bài toán lặp đi lặp lại trong thiết kế phần mềm.
>
> File này có một luận điểm xuyên suốt: **Phần lớn các pattern kinh điển (GoF) được phát minh để chữa cháy cho hạn chế của ngôn ngữ OOP class-based (Java/C++). Rust — với ownership, enum, trait, pattern matching — khiến nhiều pattern trở nên không cần thiết, hoặc biến chúng thành thứ khác đẹp hơn.**
>
> Một senior **không thuộc lòng 23 pattern**. Senior hiểu **các lực (forces) tạo ra bài toán**, rồi chọn công cụ nhẹ nhất giải quyết được. Mục tiêu của file này là dạy bạn cách *nghĩ* đó, không phải cách *nhớ*.

---

## Mục lục

**Tầng 1 — Triết lý: vì sao design pattern tồn tại?**
1. [Design pattern là gì — bản chất](#1-design-pattern-la-gi)
2. [Lịch sử: GoF sinh ra từ thế giới OOP](#2-lich-su-gof)
3. [Vì sao Rust thay đổi cuộc chơi](#3-rust-thay-doi-cuoc-choi)
4. [Phân loại pattern + bản đồ tư duy](#4-phan-loai)
5. [4 nguyên tắc nền tảng chi phối mọi quyết định](#5-nguyen-tac-nen-tang)

**Tầng 2 — Idioms nền tảng (phải biết trước mọi pattern)**
6. [Newtype — pattern quan trọng nhất Rust](#6-newtype)
7. [Builder — xây object phức tạp an toàn](#7-builder)
8. [Typestate — đưa trạng thái vào type system](#8-typestate)
9. [RAII / Drop guard — quản lý tài nguyên](#9-raii)
10. [Default + struct update + Constructor functions](#10-default-constructor)

**Tầng 3 — Creational patterns kiểu Rust**
11. [Factory & Abstract Factory → trait + generic](#11-factory)
12. [Singleton → OnceLock/LazyLock (và vì sao Rust ghét nó)](#12-singleton)
13. [Prototype → Clone trait](#13-prototype)

**Tầng 4 — Structural patterns kiểu Rust**
14. [Adapter → trait impl / From](#14-adapter)
15. [Decorator → wrapper + middleware (tower)](#15-decorator)
16. [Facade → module API](#16-facade)
17. [Composite → enum đệ quy + Box](#17-composite)
18. [Proxy → Deref & smart pointers](#18-proxy)
19. [Flyweight → Rc/Arc + interning](#19-flyweight)
20. [Bridge → trait object tách abstraction khỏi impl](#20-bridge)

**Tầng 5 — Behavioral patterns kiểu Rust**
21. [Strategy → 3 cách (closure / generic / dyn)](#21-strategy)
22. [Observer → channel & callback (vì sao classic observer đau với borrow)](#22-observer)
23. [State → enum match vs typestate](#23-state)
24. [Command → closure / enum](#24-command)
25. [Visitor → enum match / serde / double dispatch](#25-visitor)
26. [Template Method → default trait method](#26-template-method)
27. [Chain of Responsibility → middleware / Option chaining](#27-chain)
28. [Iterator, Mediator, Memento — ngắn gọn](#28-others)

**Tầng 6 — Patterns đặc sản Rust (không có trong GoF)**
29. [Sealed trait](#29-sealed-trait)
30. [Extension trait](#30-extension-trait)
31. [Marker type & PhantomData](#31-marker-phantom)
32. [Interior mutability pattern](#32-interior-mutability)
33. [Entry API & Cow](#33-entry-cow)

**Tầng 7 — Concurrency patterns**
34. [Actor model qua channel](#34-actor)
35. [Shared state (Arc<Mutex>) và khi nào tránh](#35-shared-state)
36. [Worker pool / pipeline / fan-out fan-in](#36-worker-pool)

**Tầng 8 — Kiến trúc hệ thống lớn**
37. [Dependency Injection kiểu Rust (không cần framework)](#37-di)
38. [Repository pattern](#38-repository)
39. [Hexagonal / Ports & Adapters](#39-hexagonal)
40. [Plugin architecture](#40-plugin)
41. [Event-driven / CQRS / Event Sourcing](#41-event-driven)
42. [DDD với newtype & make invalid states unrepresentable](#42-ddd)

**Tầng 9 — Case study: từ hệ nhỏ đến hệ lớn**
43. [Stage 0: script 30 dòng — đừng dùng pattern nào](#43-stage0)
44. [Stage 1: thêm cấu trúc — newtype + builder](#44-stage1)
45. [Stage 2: nhiều implementation — strategy + trait](#45-stage2)
46. [Stage 3: web service — repository + hexagonal + DI](#46-stage3)
47. [Stage 4: scale — actor + event-driven + plugin](#47-stage4)
48. [Bản đồ: pattern nào xuất hiện ở quy mô nào](#48-ban-do-quy-mo)

**Tầng 10 — Antipatterns**
49. [Những sai lầm chết người](#49-antipatterns)

**Tầng 11 — Senior wisdom**
50. [Decision tree chọn pattern](#50-decision-tree)
51. [12 nguyên tắc senior](#51-12-nguyen-tac)
52. [Toolkit & crates](#52-toolkit)

---

# TẦNG 1 — TRIẾT LÝ: VÌ SAO DESIGN PATTERN TỒN TẠI?

<a name="1-design-pattern-la-gi"></a>
## 1. Design pattern là gì — bản chất

Hãy bắt đầu từ gốc, đừng vội nói tới Rust.

Khi nhiều người cùng giải một loại bài toán nhiều lần, họ phát hiện ra **những cách giải tốt cứ lặp lại**. Thay vì mỗi lần phát minh lại, người ta **đặt tên** cho cách giải đó. Cái tên + bối cảnh + giải pháp + hệ quả = **một design pattern**.

> Bản chất: **Design pattern là một CÁI TÊN cho một cặp (vấn đề lặp lại, giải pháp đã được kiểm chứng), kèm theo các đánh đổi của nó.**

Giá trị thật của pattern **không phải** là đoạn code. Giá trị là:

1. **Từ vựng chung** — khi bạn nói "chỗ này dùng Builder", cả team hiểu ngay 5 câu giải thích mà không cần viết ra.
2. **Tư duy về lực (forces)** — pattern dạy bạn nhận diện các *áp lực mâu thuẫn* (ví dụ: muốn linh hoạt runtime NHƯNG muốn nhanh; muốn nhiều bước khởi tạo NHƯNG muốn object luôn hợp lệ).
3. **Danh sách đánh đổi** — mỗi pattern có giá phải trả (thêm indirection, thêm allocation, khó debug...).

### Sai lầm phổ biến của người mới

Người mới nghĩ: "Học xong 23 pattern là giỏi thiết kế." Sai.

> **Pattern là *hệ quả* của việc nhận ra bài toán, không phải *điểm khởi đầu*.** Bạn không "đi tìm chỗ để nhét Strategy vào". Bạn gặp bài toán "cần thay thuật toán lúc runtime", rồi nhận ra "à, đây là Strategy".

Dùng pattern khi *chưa* có bài toán = **over-engineering** = antipattern nghiêm trọng nhất. Ta sẽ quay lại điều này rất nhiều lần.

---

<a name="2-lich-su-gof"></a>
## 2. Lịch sử: GoF sinh ra từ thế giới OOP

Năm 1994, 4 tác giả (Gang of Four — Gamma, Helm, Johnson, Vlissides) xuất bản *Design Patterns: Elements of Reusable Object-Oriented Software*. Họ tổng hợp 23 pattern từ thực tế C++/Smalltalk.

**Điều cực kỳ quan trọng mà ít ai nói:** 23 pattern này được sinh ra trong một thế giới có những đặc điểm cụ thể:

| Đặc điểm thế giới OOP 1994 | Hệ quả |
|---|---|
| **Kế thừa (inheritance)** là cơ chế tái sử dụng chính | Nhiều pattern xoay quanh việc "lách" sự cứng nhắc của inheritance |
| **Mọi thứ là object trên heap**, truy cập qua con trỏ | Indirection là mặc định, không ai để ý chi phí |
| **Không có sum type (enum thực thụ)** | Phải dùng class hierarchy + virtual dispatch để biểu diễn "một trong nhiều dạng" |
| **Không có ownership / borrow checker** | Aliasing tự do, ai trỏ tới ai cũng được → Observer, Mediator dễ viết |
| **Null tồn tại khắp nơi** | Cần Null Object pattern |
| **Không có closure first-class (thời đó)** | Strategy/Command phải là class |

Nhìn bảng trên, ta thấy: **rất nhiều pattern GoF là thuốc giải độc cho căn bệnh của chính OOP class-based.**

Ví dụ kinh điển:
- **Strategy pattern** = "đóng gói thuật toán vào object để swap được". Nhưng trong ngôn ngữ có closure first-class, một `Strategy` chỉ là... một hàm. Java 1.4 cần cả một interface + class; Rust cần `Fn(i32) -> i32`.
- **Iterator pattern** = "duyệt collection mà không lộ cấu trúc bên trong". Rust *nướng thẳng* nó vào ngôn ngữ qua trait `Iterator` + `for`.
- **Command pattern** = "đóng gói một lời gọi hàm thành object để hoãn/lưu/undo". Closure là Command sẵn rồi.

> **Bài học gốc:** Khi học một pattern GoF, luôn hỏi: *"Pattern này chữa căn bệnh gì của OOP? Rust có căn bệnh đó không?"* Nếu Rust không có bệnh, bạn không cần thuốc.

---

<a name="3-rust-thay-doi-cuoc-choi"></a>
## 3. Vì sao Rust thay đổi cuộc chơi

Rust có 5 thứ làm thay đổi căn bản cách ta nghĩ về pattern:

### 3.1. Enum là sum type thực thụ + pattern matching

Trong Java, để biểu diễn "một Shape là Circle HOẶC Rectangle HOẶC Triangle", bạn tạo abstract class `Shape` + 3 subclass + virtual method. Đây là nền của Visitor, Composite, State...

Trong Rust:

```rust
enum Shape {
    Circle { radius: f64 },
    Rectangle { w: f64, h: f64 },
    Triangle { base: f64, height: f64 },
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rectangle { w, h } => w * h,
        Shape::Triangle { base, height } => 0.5 * base * height,
    }
}
```

`match` là **exhaustive** — compiler bắt lỗi nếu bạn quên một biến thể. Đây là điều Visitor pattern cố mô phỏng một cách vụng về trong Java (double dispatch). Trong Rust, Visitor thường tan biến thành một `match`.

### 3.2. Trait = interface + nhiều hơn thế, KHÔNG có kế thừa

Rust **không có class inheritance**. Đây là quyết định triết học, không phải thiếu sót. Composition over inheritance không phải lời khuyên — nó là *luật vật lý* của Rust.

Hệ quả: mọi pattern dựa trên "override một method của cha" (Template Method) phải diễn đạt lại qua **default method của trait**. Mọi pattern dựa trên "cây kế thừa sâu" đơn giản là không tồn tại.

### 3.3. Ownership & borrow checker

Đây là thứ thay đổi lớn nhất và đau nhất. Nhiều pattern GoF giả định **aliasing tự do**: object A giữ con trỏ tới B, B giữ con trỏ ngược lại A, ai cũng sửa được ai.

- **Observer**: subject giữ list observers, observer giữ ref tới subject → vòng tham chiếu. Trong Rust điều này va vào borrow checker ngay.
- **Mediator, bidirectional graph**: cực khó với `&mut`.

Rust buộc bạn trả lời: *"Ai sở hữu cái này? Ai chỉ mượn? Mượn bao lâu?"* Pattern nào không trả lời được sẽ không compile. Đây là tại sao trong Rust, Observer thường biến thành **channel** (message passing) thay vì callback giữ tham chiếu.

### 3.4. Closure first-class + Fn/FnMut/FnOnce

Strategy, Command, Callback → chỉ là closure. Không cần class.

### 3.5. Zero-cost abstraction + monomorphization

Generic trong Rust được *monomorphize* (sinh code riêng cho mỗi kiểu) → abstraction không tốn runtime. Điều này thay đổi đánh đổi: trong Java, "linh hoạt" luôn kèm chi phí virtual dispatch nên người ta dè dặt. Trong Rust bạn có thể chọn **static dispatch (generic, nhanh, code phình)** hoặc **dynamic dispatch (`dyn`, gọn, chậm hơn chút)** một cách *có ý thức*.

> **Tóm tắt Tầng 1:** Rust không bỏ design pattern. Rust **đổi hình dạng** của chúng. 30% pattern GoF tan biến vào ngôn ngữ, 40% biến thành thứ nhẹ hơn (closure/enum/trait), 30% vẫn cần nhưng phải viết "kiểu Rust" để qua borrow checker. Và Rust *thêm vào* một bộ pattern đặc sản (Newtype, Typestate, RAII guard...) mà OOP không có.

---

<a name="4-phan-loai"></a>
## 4. Phân loại pattern + bản đồ tư duy

GoF chia 3 nhóm. Ta giữ cách chia này nhưng gắn "trạng thái trong Rust":

```
CREATIONAL (tạo object)
  ├─ Builder ............... ⭐ ĐẮT GIÁ trong Rust (vì không có named/optional args)
  ├─ Factory ............... → constructor function / trait method
  ├─ Abstract Factory ...... → trait + generic
  ├─ Singleton ............. → OnceLock (nhưng thường là code smell)
  └─ Prototype ............. → Clone trait (built-in)

STRUCTURAL (ghép object)
  ├─ Adapter ............... → trait impl / From / wrapper
  ├─ Decorator ............. → wrapper struct / tower middleware ⭐
  ├─ Facade ................ → module + pub API
  ├─ Composite ............. → enum đệ quy + Box ⭐
  ├─ Proxy ................. → Deref / smart pointer
  ├─ Flyweight ............. → Rc/Arc + interning
  └─ Bridge ................ → trait object

BEHAVIORAL (giao tiếp/hành vi)
  ├─ Strategy .............. → closure / generic / dyn ⭐ (tan thành closure)
  ├─ Observer .............. → channel / callback ⚠️ (đau với borrow → dùng channel)
  ├─ State ................. → enum match / typestate ⭐
  ├─ Command ............... → closure / enum (tan thành closure)
  ├─ Iterator .............. → BUILT-IN (Iterator trait)
  ├─ Visitor ............... → match / serde (gần như tan biến)
  ├─ Template Method ....... → default trait method
  ├─ Chain of Resp. ........ → middleware / Option/Result chain
  ├─ Mediator .............. → hiếm; thường là actor/event bus
  ├─ Memento ............... → Clone snapshot
  ├─ Interpreter ........... → enum AST + match
  └─ Null Object ........... → KHÔNG CẦN (có Option<T>)

RUST-SPECIFIC (không có trong GoF) ⭐⭐⭐
  ├─ Newtype
  ├─ Typestate
  ├─ RAII guard
  ├─ Sealed trait
  ├─ Extension trait
  ├─ Marker type / PhantomData
  ├─ Interior mutability
  └─ Entry API / Cow
```

`⭐` = pattern bạn sẽ dùng *thật nhiều* trong Rust. `⚠️` = pattern cần cẩn thận với borrow checker.

---

<a name="5-nguyen-tac-nen-tang"></a>
## 5. Bốn nguyên tắc nền tảng chi phối mọi quyết định

Trước khi đi vào từng pattern, đây là 4 nguyên tắc thiết kế *quan trọng hơn mọi pattern*. Khi nghi ngờ, quay về đây.

### Nguyên tắc 1: Make invalid states unrepresentable (Khiến trạng thái sai không thể biểu diễn được)

Đây là **kim chỉ nam số 1 của thiết kế Rust**. Thay vì kiểm tra dữ liệu sai lúc runtime, hãy thiết kế kiểu sao cho **không thể tạo ra dữ liệu sai ngay từ đầu**.

```rust
// ❌ Cách OOP: trạng thái sai biểu diễn được, phải nhớ validate
struct Connection {
    is_open: bool,
    socket: Option<TcpStream>,  // open=true nhưng socket=None? Bug chờ sẵn.
}

// ✅ Cách Rust: dùng enum — không thể "open mà không có socket"
enum Connection {
    Closed,
    Open(TcpStream),
}
```

Bug "open=true, socket=None" trong phiên bản đầu *không thể tồn tại* trong phiên bản hai. Compiler là người gác cổng.

### Nguyên tắc 2: Parse, don't validate (Phân giải, đừng chỉ kiểm tra)

Khi nhận dữ liệu thô (string, JSON, input người dùng), đừng *kiểm tra rồi vẫn truyền string đi tiếp*. Hãy *phân giải nó thành một kiểu chặt chẽ hơn* — sau điểm đó, type system đảm bảo nó đã hợp lệ.

```rust
// ❌ validate: kiểm tra xong vẫn là String, người sau phải kiểm tra lại
fn validate_email(s: &str) -> bool { s.contains('@') }

// ✅ parse: sau khi có Email, KHÔNG AI phải kiểm tra lại nữa
struct Email(String);
impl Email {
    fn parse(s: String) -> Result<Email, EmailError> {
        if s.contains('@') { Ok(Email(s)) } else { Err(EmailError) }
    }
}
// Hàm nhận `Email` thay vì `&str` → compiler đảm bảo email đã hợp lệ.
```

Đây chính là động cơ của **Newtype** và **Typestate** — hai pattern quan trọng nhất ta học ở Tầng 2.

### Nguyên tắc 3: Composition over inheritance (bắt buộc trong Rust)

Rust không cho bạn lựa chọn — không có inheritance. Tái sử dụng qua: **trait (chia sẻ hành vi)** + **struct chứa struct khác (chia sẻ dữ liệu)** + **generic (tham số hóa hành vi)**.

### Nguyên tắc 4: Trả chi phí một cách có ý thức (zero-cost, nhưng không free)

Mỗi lần thêm indirection (`Box<dyn>`, `Rc`, `Arc<Mutex>`), bạn trả giá: allocation, pointer chasing, runtime dispatch, hoặc lock contention. Rust khiến chi phí *hiện rõ trong kiểu*. Một senior chọn indirection khi **lợi ích linh hoạt > chi phí**, chứ không vì "pattern bảo thế".

> **Tư duy then chốt:** Trước khi áp pattern, hỏi 3 câu: (1) *Bài toán thật là gì?* (2) *Cách nhẹ nhất của Rust để giải?* (3) *Pattern này có thêm chi phí gì không đáng?*

---

# TẦNG 2 — IDIOMS NỀN TẢNG

Đây là những pattern bạn dùng **hàng ngày** trong Rust, kể cả ở project nhỏ. Phải nắm chắc trước khi nói tới GoF.

<a name="6-newtype"></a>
## 6. Newtype — pattern quan trọng nhất Rust

### Bài toán

Bạn có nhiều giá trị cùng kiểu cơ sở nhưng **khác ý nghĩa**:

```rust
fn transfer(from: u64, to: u64, amount: u64) { /* ... */ }

transfer(amount, from_account, to_account); // 😱 compile OK, nhưng SAI THỨ TỰ
```

Cả ba đều là `u64` → compiler không cứu bạn. Đây là lớp bug kinh điển (gửi nhầm ID, nhầm đơn vị mét/feet — vụ Mars Climate Orbiter $327 triệu).

### Giải pháp: bọc trong struct một-trường

```rust
struct AccountId(u64);
struct Money(u64);

fn transfer(from: AccountId, to: AccountId, amount: Money) { /* ... */ }

// transfer(amount, from, to); // ❌ KHÔNG COMPILE — sai kiểu
```

> **Bản chất Newtype:** mượn type system để gắn *ý nghĩa* (semantics) lên một kiểu cơ sở. Zero-cost — lúc chạy `AccountId` chính là `u64`, không tốn byte nào (xem chương **x — data layout**: single-field struct có cùng layout với field của nó).

### Ba công dụng của Newtype

**(a) Type safety — phân biệt giá trị cùng kiểu** (ví dụ trên).

**(b) Vượt qua orphan rule** — bạn không thể `impl Display for Vec<T>` (cả `Display` lẫn `Vec` đều không thuộc crate bạn). Bọc lại:

```rust
struct Wrapper(Vec<String>);
impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}
```

**(c) Đóng gói bất biến (invariant)** — kết hợp với "parse don't validate":

```rust
pub struct NonEmptyName(String);  // trường private!

impl NonEmptyName {
    pub fn new(s: String) -> Result<Self, &'static str> {
        if s.trim().is_empty() { Err("name rỗng") } else { Ok(NonEmptyName(s)) }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

Vì trường là private, **cách duy nhất** để có `NonEmptyName` là qua `new()` — nơi bất biến được kiểm tra. Mọi hàm nhận `NonEmptyName` được *đảm bảo* tên không rỗng. Đây là make-invalid-states-unrepresentable ở mức field.

### Tại sao chọn Newtype?

- Cần phân biệt ngữ nghĩa các giá trị cùng kiểu cơ sở → **luôn dùng**.
- Cần impl trait ngoài cho kiểu ngoài → bắt buộc.
- Cần đảm bảo bất biến tại biên hệ thống → kết hợp constructor + private field.

Chi phí: gần như bằng 0 runtime; chi phí "cú pháp" là phải gọi `.0` hoặc viết `Deref` (xem mục 18).

---

<a name="7-builder"></a>
## 7. Builder — xây object phức tạp an toàn

### Bài toán: Rust KHÔNG có named arguments và optional arguments

Đây là lý do Builder *đắt giá* trong Rust hơn nhiều ngôn ngữ khác. Trong Python bạn viết `Server(port=8080, tls=True)`. Rust không có. Nếu một struct có 10 trường, trong đó 7 cái có giá trị mặc định, bạn sẽ khổ sở:

```rust
// ❌ Constructor địa ngục: nhớ thứ tự 10 tham số, không biết cái nào optional
Server::new("localhost", 8080, 30, 1024, true, false, None, 4, "v1", true);
//                                    ^^^ true/false này là gì??
```

### Giải pháp: Builder

```rust
#[derive(Debug)]
pub struct Server {
    host: String,
    port: u16,
    max_connections: u32,
    tls: bool,
}

pub struct ServerBuilder {
    host: String,
    port: u16,
    max_connections: u32,
    tls: bool,
}

impl ServerBuilder {
    pub fn new(host: impl Into<String>) -> Self {
        ServerBuilder {
            host: host.into(),
            port: 8080,            // mặc định hợp lý
            max_connections: 1024,
            tls: false,
        }
    }
    pub fn port(mut self, p: u16) -> Self { self.port = p; self }
    pub fn max_connections(mut self, n: u32) -> Self { self.max_connections = n; self }
    pub fn tls(mut self, on: bool) -> Self { self.tls = on; self }

    pub fn build(self) -> Server {
        Server { host: self.host, port: self.port,
                 max_connections: self.max_connections, tls: self.tls }
    }
}

// Dùng — đọc như văn xuôi, chỉ set cái cần:
let server = ServerBuilder::new("localhost")
    .port(443)
    .tls(true)
    .build();
```

### Hai biến thể quan trọng

**Owned builder (`mut self` → `Self`)** như trên: chain đẹp, di chuyển giá trị. Phù hợp nhất khi build một lần.

**Mut-ref builder (`&mut self` → `&mut Self`)**: khi muốn build có điều kiện:

```rust
let mut b = ServerBuilder::new("localhost");
b.port(443);
if use_tls { b.tls(true); }   // set có điều kiện dễ hơn
let server = b.build();
```

### Builder + Typestate = bắt buộc trường ở compile time

Builder thường có nhược: quên gọi `build()` hợp lệ, hoặc quên set trường bắt buộc → lỗi runtime. Kết hợp **Typestate** (mục 8) để compiler bắt:

```rust
// Builder không cho gọi build() khi chưa set host — bắt LÚC COMPILE
// (chi tiết ở mục 8)
```

### Tại sao chọn Builder?

- Struct có **nhiều trường optional / mặc định** → Builder thắng tuyệt đối.
- Khởi tạo có **nhiều bước, validation giữa chừng** → Builder.
- Muốn API đọc như văn xuôi, dễ mở rộng (thêm trường = thêm method, không phá call site cũ).

Khi **KHÔNG** dùng: struct 2-3 trường bắt buộc, không optional → chỉ cần `Struct { a, b, c }` hoặc `new(a, b, c)`. Builder lúc này là over-engineering.

> Trong thực tế dùng crate **`derive_builder`** hoặc **`bon`** để sinh builder tự động thay vì viết tay. Nhưng phải hiểu cơ chế trước.

---

<a name="8-typestate"></a>
## 8. Typestate — đưa trạng thái vào type system

Đây là một trong những pattern **đẹp nhất và "Rust nhất"**. OOP gần như không làm được điều này.

### Bài toán: API có trình tự bắt buộc

Một HTTP request builder: phải set URL *trước*, rồi mới set body, rồi mới `send()`. Một file: phải `open` trước khi `read`. Một state machine: chỉ một số chuyển tiếp là hợp lệ.

Cách thông thường (runtime check):

```rust
let mut req = Request::new();
req.send();           // 😱 chưa set URL — runtime panic / Err
req.set_url("...");
```

### Giải pháp: mã hóa trạng thái vào KIỂU

Ý tưởng: mỗi trạng thái là một **kiểu khác nhau**. Method chỉ tồn tại ở trạng thái cho phép. Chuyển trạng thái = **tiêu thụ self và trả về kiểu mới**.

```rust
use std::marker::PhantomData;

// Các trạng thái — chỉ là marker type rỗng (zero-sized)
struct NoUrl;
struct HasUrl;

struct Request<State> {
    url: Option<String>,
    body: Option<String>,
    _state: PhantomData<State>,
}

impl Request<NoUrl> {
    fn new() -> Self {
        Request { url: None, body: None, _state: PhantomData }
    }
    // set_url tiêu thụ Request<NoUrl>, trả về Request<HasUrl>
    fn url(self, u: impl Into<String>) -> Request<HasUrl> {
        Request { url: Some(u.into()), body: self.body, _state: PhantomData }
    }
}

impl Request<HasUrl> {
    fn body(mut self, b: impl Into<String>) -> Self {
        self.body = Some(b.into());
        self
    }
    // send() CHỈ tồn tại khi đã HasUrl
    fn send(self) -> String {
        format!("GET {} body={:?}", self.url.unwrap(), self.body)
    }
}

fn demo() {
    let r = Request::new()      // Request<NoUrl>
        .url("https://x.com")   // Request<HasUrl>
        .body("hello")
        .send();                // OK

    // Request::new().send();   // ❌ KHÔNG COMPILE: NoUrl không có send()
}
```

> **Bản chất:** trạng thái sai (`send` khi chưa có URL) **không biểu diễn được** → không phải bug runtime, mà là *lỗi compile*. Đây là make-invalid-states-unrepresentable ở mức API flow.
>
> Chi phí runtime = **0**. `PhantomData<State>` là zero-sized; `NoUrl`/`HasUrl` không chiếm byte nào (xem chương **x**). Bạn được bảo đảm compile-time hoàn toàn miễn phí lúc chạy.

### Ví dụ thực tế: typestate trong hệ thống thật

- **`embedded-hal`**: một chân GPIO có kiểu `Pin<Input>` vs `Pin<Output>`. Bạn không thể `write()` lên pin đang ở chế độ input — compiler chặn. Cực kỳ quan trọng cho firmware (xem chương **p — embedded**).
- **Builder bắt buộc trường**: như mục 7, `build()` chỉ tồn tại khi `Builder<AllFieldsSet>`.
- **Session/connection**: `Connection<Unauthenticated>` không có method `query()`, chỉ `Connection<Authenticated>` mới có.

### Tại sao chọn Typestate?

- API có **trình tự gọi bắt buộc** và bạn muốn lỗi xảy ra **lúc compile, không phải lúc chạy**.
- Bug do gọi sai thứ tự **đắt** (firmware, tài chính, bảo mật) → đáng để đầu tư.

Khi **KHÔNG** dùng: flow đơn giản, hoặc trạng thái thay đổi nhiều theo runtime không đoán trước được (lúc đó dùng enum-state ở mục 23). Typestate làm phình số lượng `impl` block và khó với người mới đọc.

---

<a name="9-raii"></a>
## 9. RAII / Drop guard — quản lý tài nguyên

RAII = Resource Acquisition Is Initialization. Có gốc từ C++, nhưng Rust biến nó thành **mặc định không thể tắt**.

### Bài toán: dọn dẹp tài nguyên đúng lúc

File phải đóng, lock phải nhả, transaction phải commit/rollback, kết nối phải trả về pool. Trong C/Go bạn dễ quên (`defer` giúp Go, nhưng vẫn phải nhớ viết). Trong Java có `finally`/try-with-resources.

### Giải pháp: trait `Drop` — dọn dẹp tự động khi ra khỏi scope

```rust
struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new(p: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(p)?;
        Ok(TempDir { path: p.into() })
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);  // tự xóa khi hết scope
        println!("Đã dọn {:?}", self.path);
    }
}

fn work() {
    let _tmp = TempDir::new("/tmp/job123").unwrap();
    // ... làm việc với thư mục tạm ...
}   // <- _tmp.drop() chạy TỰ ĐỘNG ở đây, kể cả khi panic
```

> **Bản chất:** trong Rust, *thời điểm dọn dẹp được gắn vào thời điểm ownership kết thúc*. Compiler chèn lời gọi `drop` cho bạn — không quên được, kể cả khi hàm panic và stack unwind.

### Drop guard pattern (scoped guard)

`MutexGuard` của std là ví dụ kinh điển: `lock()` trả về một guard; khi guard drop, lock tự nhả.

```rust
use std::sync::Mutex;
let m = Mutex::new(0);
{
    let mut guard = m.lock().unwrap();  // acquire
    *guard += 1;
}   // <- guard drop → lock nhả TỰ ĐỘNG. Không thể quên unlock.
```

Bạn dùng pattern này để: timer đo thời gian (drop = in elapsed), transaction (drop = rollback nếu chưa commit), span tracing (xem chương **l — observability**).

### Tại sao chọn RAII?

- Bất cứ khi nào có **cặp acquire/release** mà việc quên release gây hại → RAII là cách Rust-idiomatic. Gần như **luôn** dùng thay vì cleanup thủ công.

Cạm bẫy: `Drop` không chạy nếu bạn `std::mem::forget` hoặc process bị kill. Và không có `async Drop` (tính đến nay) → cleanup async cần xử lý đặc biệt (xem chương **f — async**).

---

<a name="10-default-constructor"></a>
## 10. Default + struct update + Constructor functions

### Rust không có "constructor". Quy ước: hàm liên kết `new()`

Khác Java/C++, Rust không có constructor đặc biệt. Quy ước cộng đồng: hàm `Type::new(...)`. Nếu việc tạo có thể thất bại → trả `Result`:

```rust
impl Config {
    fn new(path: &str) -> Result<Self, ConfigError> { /* ... */ }
}
```

Nếu có nhiều cách tạo, đặt tên rõ ràng (đây là Factory function — mục 11):

```rust
impl Color {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self { /* */ }
    fn from_hex(s: &str) -> Result<Self, _> { /* */ }
    fn black() -> Self { /* */ }
}
```

### `Default` trait + struct update syntax

Khi struct có giá trị mặc định hợp lý:

```rust
#[derive(Default)]
struct Settings {
    verbose: bool,
    retries: u32,
    timeout_ms: u64,
}

// Chỉ override cái cần, phần còn lại lấy default — đây là "poor man's named args"
let s = Settings {
    retries: 5,
    ..Default::default()
};
```

`..Default::default()` là **struct update syntax** — một cách nhẹ thay cho Builder khi tất cả trường đều public và đều có default. Dùng nó cho config nội bộ; dùng Builder khi cần validation hoặc API public ổn định.

> **Quyết định nhanh:** ít trường + có default + public → struct update. Nhiều trường optional + cần validation + API public → Builder.

---

# TẦNG 3 — CREATIONAL PATTERNS KIỂU RUST

<a name="11-factory"></a>
## 11. Factory & Abstract Factory → trait + generic

### Factory Method

"Factory" trong GoF = một method tạo object, cho phép subclass quyết định kiểu cụ thể. Rust không có subclass, nên Factory chỉ là... **một hàm/method trả về một kiểu (hoặc trait object)**.

```rust
trait Logger {
    fn log(&self, msg: &str);
}
struct ConsoleLogger;
struct FileLogger { path: String }
impl Logger for ConsoleLogger { fn log(&self, m: &str) { println!("{m}"); } }
impl Logger for FileLogger { fn log(&self, m: &str) { /* ghi file */ } }

// Factory function: chọn impl lúc runtime, trả trait object
fn make_logger(kind: &str) -> Box<dyn Logger> {
    match kind {
        "file" => Box::new(FileLogger { path: "app.log".into() }),
        _ => Box::new(ConsoleLogger),
    }
}
```

### Abstract Factory → trait có nhiều method tạo

Khi cần tạo *cả một họ* object liên quan (ví dụ UI theme: Button + Checkbox + Window cùng phong cách):

```rust
trait WidgetFactory {
    fn button(&self) -> Box<dyn Button>;
    fn checkbox(&self) -> Box<dyn Checkbox>;
}
struct DarkTheme;
struct LightTheme;
// impl WidgetFactory cho mỗi theme...

// Code dùng chỉ biết trait, không biết theme cụ thể:
fn build_ui(factory: &dyn WidgetFactory) {
    let b = factory.button();
    let c = factory.checkbox();
}
```

### Tĩnh hay động? Đây là quyết định senior

- Dùng **generic `<F: WidgetFactory>`** nếu kiểu factory biết lúc compile → zero-cost, monomorphize.
- Dùng **`Box<dyn>`** nếu phải chọn lúc runtime (từ config, từ input người dùng).

```rust
// Static — nhanh, code phình theo số kiểu
fn build_ui<F: WidgetFactory>(factory: &F) { /* */ }

// Dynamic — gọn, một bản code, có vtable lookup
fn build_ui(factory: &dyn WidgetFactory) { /* */ }
```

> **Tại sao chọn Factory?** Khi *điểm tạo object* cần tách khỏi *nơi dùng*, và kiểu cụ thể phụ thuộc config/runtime. Nếu kiểu biết sẵn lúc compile → đừng dùng factory, cứ gọi constructor trực tiếp.

---

<a name="12-singleton"></a>
## 12. Singleton → OnceLock/LazyLock (và vì sao Rust ghét nó)

### Vì sao Rust "ghét" Singleton

Singleton = một instance toàn cục, ai cũng truy cập được. Trong Rust điều này va trực diện vào ownership:
- Global mutable state = aliasing + mutation đồng thời = **chính xác thứ borrow checker sinh ra để cấm**.
- `static mut` là `unsafe` và gần như luôn sai.

> Rust ép bạn hỏi: "Thật sự cần global không, hay chỉ lười truyền tham số?" 90% trường hợp Singleton là **cái cớ để tránh dependency injection** — và DI (mục 37) là cách đúng.

### Khi thật sự cần global bất biến: `OnceLock` / `LazyLock`

Cho cấu hình đọc-một-lần, registry, regex compiled sẵn:

```rust
use std::sync::OnceLock;

fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| Config::load_from_env())
}
// Khởi tạo lazy, thread-safe, chỉ chạy đúng 1 lần. Đọc thoải mái.
```

`LazyLock` (stable 1.80) cho cú pháp gọn hơn khi init không cần tham số:

```rust
use std::sync::LazyLock;
use regex::Regex;

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^@]+@[^@]+$").unwrap());
```

### Nếu cần global MUTABLE: bọc trong khóa, và suy nghĩ lại

```rust
use std::sync::{LazyLock, Mutex};
static COUNTER: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));
// *COUNTER.lock().unwrap() += 1;
```

Nhưng đây là code smell. Mỗi truy cập đều lock → contention. Khó test (state rò rỉ giữa các test). Khó suy luận. **Cân nhắc truyền `Arc<AppState>` qua tham số thay vì global** — đó là cách production thật làm (xem chương **q — axum**: state truyền qua extractor, không phải global).

> **Tại sao (hiếm khi) chọn Singleton?** Chỉ cho dữ liệu *thực sự toàn cục, init một lần, đọc nhiều*: config, logger, metric registry, compiled regex. Còn lại → DI.

---

<a name="13-prototype"></a>
## 13. Prototype → Clone trait

Prototype = "tạo object mới bằng cách copy một object có sẵn" thay vì gọi constructor. Trong Rust đây là **`Clone` trait**, built-in:

```rust
#[derive(Clone)]
struct Document { template: String, sections: Vec<String> }

let base = Document { template: "report".into(), sections: vec![] };
let mut copy = base.clone();   // Prototype
copy.sections.push("intro".into());
```

Không có gì để bàn nhiều — Rust đã nướng Prototype vào ngôn ngữ qua `derive(Clone)`. Điểm cần hiểu là **chi phí**: `clone()` của `Vec`/`String` là *deep copy* (cấp phát mới). Nếu chỉ cần chia sẻ đọc, dùng `Rc::clone`/`Arc::clone` (chỉ tăng bộ đếm, rẻ — xem mục 19 và chương **i — smart pointers**).

> **Tại sao chọn:** khi tạo object mới giống hệt/gần giống cái cũ rẻ hơn dựng từ đầu (ví dụ object có nhiều cấu hình mặc định phức tạp). Cẩn thận chi phí deep clone.

---

# TẦNG 4 — STRUCTURAL PATTERNS KIỂU RUST

<a name="14-adapter"></a>
## 14. Adapter → trait impl / From

Adapter = làm cho interface A tương thích với interface B mà code mong đợi.

### Ba hình dạng trong Rust

**(a) Implement trait có sẵn cho kiểu của mình** — adapter đúng nghĩa:

```rust
// Một thư viện cũ trả về kiểu lạ; ta adapt nó sang Iterator chuẩn
struct LegacyReader { /* ... */ }
impl Iterator for LegacyReader {
    type Item = String;
    fn next(&mut self) -> Option<String> { /* gọi API cũ, chuyển đổi */ todo!() }
}
// Giờ LegacyReader dùng được với .map().filter().collect()...
```

**(b) `From`/`Into` — adapter cho chuyển đổi kiểu:**

```rust
struct Celsius(f64);
struct Fahrenheit(f64);
impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self { Fahrenheit(c.0 * 9.0/5.0 + 32.0) }
}
let f: Fahrenheit = Celsius(100.0).into();
```

**(c) Wrapper struct** — bọc kiểu ngoài, expose interface mình muốn (Newtype + delegate).

> **Tại sao chọn Adapter:** khi tích hợp thư viện bên thứ ba có interface khác với code của bạn, hoặc khi muốn một kiểu cũ "nói được ngôn ngữ" trait chuẩn (Iterator, Read, Write...). Cực kỳ thường gặp khi wrap C FFI (chương **n**).

---

<a name="15-decorator"></a>
## 15. Decorator → wrapper + middleware (tower)

Decorator = thêm hành vi vào object mà không sửa nó, có thể chồng nhiều lớp.

### Hình dạng cơ bản: wrapper bọc trait object

```rust
trait DataSource {
    fn read(&self) -> String;
}
struct File { name: String }
impl DataSource for File {
    fn read(&self) -> String { format!("data from {}", self.name) }
}

// Decorator: bọc một DataSource, thêm logging
struct LoggingSource<S: DataSource> { inner: S }
impl<S: DataSource> DataSource for LoggingSource<S> {
    fn read(&self) -> String {
        println!("[log] reading...");
        let r = self.inner.read();   // gọi cái bị bọc
        println!("[log] done");
        r
    }
}

// Decorator khác: cache
struct CachingSource<S: DataSource> { inner: S, cache: std::cell::RefCell<Option<String>> }
// ...

// Chồng lớp:
let src = LoggingSource { inner: File { name: "x.txt".into() } };
```

### Hình dạng production: Tower middleware

Đây là Decorator ở quy mô lớn nhất trong Rust. **`tower::Service`** là một trait `Request -> Future<Response>`. Middleware (timeout, retry, rate-limit, auth, tracing) là các **layer bọc service**, mỗi layer là một Decorator:

```text
   Request
     │
     ▼
 ┌─────────────┐
 │ Tracing     │  ← decorator
 │ ┌─────────┐ │
 │ │ Timeout │ │  ← decorator
 │ │ ┌─────┐ │ │
 │ │ │ Auth│ │ │  ← decorator
 │ │ │ ┌──┐│ │ │
 │ │ │ │S ││ │ │  ← service thật
 │ │ │ └──┘│ │ │
 │ │ └─────┘ │ │
 │ └─────────┘ │
 └─────────────┘
```

axum dùng chính cơ chế này (`ServiceBuilder::new().layer(...).layer(...)`). Xem chương **q**.

> **Tại sao chọn Decorator:** khi muốn xếp chồng các mối quan tâm cắt ngang (cross-cutting concerns: log, cache, retry, auth) một cách **kết hợp được, tháo lắp được**, mà không nhồi vào logic cốt lõi. Generic wrapper cho static dispatch; `Box<dyn>`/tower cho composition runtime.

---

<a name="16-facade"></a>
## 16. Facade → module API

Facade = một interface đơn giản che giấu một hệ thống con phức tạp.

Trong Rust, Facade chính là **thiết kế module + `pub`**. Bạn có 10 struct nội bộ phức tạp; bạn expose 1 struct + 3 hàm `pub`:

```rust
mod payment {
    mod stripe_client { /* phức tạp */ }
    mod fraud_check { /* phức tạp */ }
    mod ledger { /* phức tạp */ }

    // Facade: API công khai gọn gàng
    pub struct PaymentService { /* ... */ }
    impl PaymentService {
        pub fn charge(&self, amount: Money, card: &Card) -> Result<Receipt, PayError> {
            // điều phối stripe_client + fraud_check + ledger bên trong
            todo!()
        }
    }
}
```

Người dùng chỉ thấy `PaymentService::charge`. Chi tiết bị giấu sau privacy boundary.

> **Tại sao chọn Facade:** giảm tải nhận thức cho người dùng API, tạo ranh giới ổn định để bạn tự do refactor bên trong. Đây là *mặc định* khi thiết kế crate/module — không phải pattern "đặc biệt", mà là vệ sinh API cơ bản.

---

<a name="17-composite"></a>
## 17. Composite → enum đệ quy + Box

Composite = xử lý object đơn lẻ và nhóm object **đồng nhất** (cây). Ví dụ kinh điển: cây thư mục, AST, biểu thức toán học, cây UI.

Trong OOP: abstract class `Component` + `Leaf` + `Composite` chứa `List<Component>`. Trong Rust: **enum đệ quy**.

```rust
enum FileNode {
    File { name: String, size: u64 },
    Dir { name: String, children: Vec<FileNode> },  // đệ quy
}

impl FileNode {
    fn total_size(&self) -> u64 {
        match self {
            FileNode::File { size, .. } => *size,
            FileNode::Dir { children, .. } =>
                children.iter().map(|c| c.total_size()).sum(),  // đệ quy
        }
    }
}
```

### Vì sao đôi khi cần `Box`

Nếu enum *trực tiếp* chứa chính nó (không qua `Vec`/con trỏ), kích thước sẽ vô hạn → compiler báo lỗi. `Vec<FileNode>` ổn (Vec là con trỏ tới heap). Nhưng dạng list-link cần `Box`:

```rust
enum Expr {
    Num(f64),
    Add(Box<Expr>, Box<Expr>),   // Box: con trỏ → kích thước hữu hạn
    Mul(Box<Expr>, Box<Expr>),
}
fn eval(e: &Expr) -> f64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Mul(a, b) => eval(a) * eval(b),
    }
}
```

Đây cũng là **Interpreter pattern** (AST + eval) — trong Rust nó hợp nhất với Composite thành "enum đệ quy + match".

> **Tại sao chọn:** cấu trúc cây/đệ quy với số dạng node **đã biết và cố định** → enum + match (exhaustive, nhanh, không vtable). Nếu số dạng node *mở* (plugin thêm node mới) → cân nhắc `Box<dyn Trait>` thay vì enum (đánh đổi: mất exhaustiveness, được mở rộng).

---

<a name="18-proxy"></a>
## 18. Proxy → Deref & smart pointers

Proxy = một object đứng thay cho object khác, kiểm soát truy cập (lazy load, access control, đếm tham chiếu, remote).

Rust có Proxy *built-in* dưới dạng **smart pointer + `Deref`**:

- `Box<T>` — proxy tới heap.
- `Rc<T>`/`Arc<T>` — proxy đếm tham chiếu (xem chương **i**).
- `MutexGuard<T>` — proxy kiểm soát truy cập có khóa.
- `Ref<T>`/`RefMut<T>` (từ `RefCell`) — proxy kiểm tra borrow lúc runtime.

Tự viết Proxy qua `Deref`:

```rust
use std::ops::Deref;

struct LazyConfig {
    inner: std::cell::OnceCell<Config>,
}
impl LazyConfig {
    fn get(&self) -> &Config {
        self.inner.get_or_init(|| Config::load())  // lazy load — proxy logic
    }
}
```

`Deref` cũng là cách làm Newtype trở nên trong suốt:

```rust
struct Meters(f64);
impl Deref for Meters {
    type Target = f64;
    fn deref(&self) -> &f64 { &self.0 }
}
let d = Meters(5.0);
println!("{}", d.sqrt());  // gọi method của f64 qua deref coercion
```

> ⚠️ **Cẩn thận:** đừng lạm dụng `Deref` để giả vờ kế thừa. `Deref` *chỉ* nên dùng cho smart-pointer-like. Lạm dụng làm method resolution khó hiểu — đây là antipattern (mục 49).
>
> **Tại sao chọn Proxy:** lazy initialization, access control, ref counting, hoặc đại diện cho tài nguyên xa (remote/RPC). Phần lớn nhu cầu đã có sẵn smart pointer của std.

---

<a name="19-flyweight"></a>
## 19. Flyweight → Rc/Arc + interning

Flyweight = chia sẻ phần dữ liệu bất biến chung giữa nhiều object để tiết kiệm bộ nhớ.

```rust
use std::rc::Rc;

// Thay vì mỗi từ giữ một String riêng (tốn bộ nhớ với từ lặp lại nhiều),
// dùng Rc để nhiều chỗ chia sẻ cùng một String.
#[derive(Clone)]
struct Token {
    text: Rc<str>,   // chia sẻ — Rc::clone chỉ tăng counter, không copy chuỗi
    line: u32,
}
```

### String interning — Flyweight cổ điển

Đối với từ lặp lại hàng triệu lần (parser, compiler), ta "intern": lưu mỗi chuỗi duy nhất một lần, các nơi khác giữ một id/Rc:

```rust
use std::collections::HashMap;
use std::rc::Rc;

struct Interner {
    map: HashMap<Rc<str>, ()>,
}
impl Interner {
    fn intern(&mut self, s: &str) -> Rc<str> {
        if let Some((k, _)) = self.map.get_key_value(s) {
            Rc::clone(k)            // đã có → chia sẻ
        } else {
            let rc: Rc<str> = Rc::from(s);
            self.map.insert(Rc::clone(&rc), ());
            rc
        }
    }
}
```

> **Tại sao chọn Flyweight:** khi có **rất nhiều object chia sẻ phần dữ liệu bất biến trùng lặp** và bộ nhớ là vấn đề (game entity, glyph font, token compiler). Dùng `Rc`/`Arc` cho chia sẻ; interning cho chuỗi/giá trị lặp. Chi phí: ref-count overhead, và bạn phải đảm bảo dữ liệu chia sẻ là bất biến (hoặc `Arc<Mutex>` nếu cần sửa — đắt hơn).

---

<a name="20-bridge"></a>
## 20. Bridge → trait object tách abstraction khỏi impl

Bridge = tách **abstraction** (cái người dùng thấy) khỏi **implementation** (cách làm) để hai bên biến đổi độc lập.

Ví dụ: bạn có nhiều loại `Notification` (Alert, Reminder) × nhiều kênh gửi (Email, SMS, Push). Nếu dùng kế thừa, bạn sẽ có bùng nổ tổ hợp (AlertEmail, AlertSMS, ReminderEmail...). Bridge tách hai trục:

```rust
// Trục implementation: kênh gửi
trait Channel {
    fn send(&self, msg: &str);
}
struct Email;
struct Sms;
impl Channel for Email { fn send(&self, m: &str) { println!("email: {m}"); } }
impl Channel for Sms { fn send(&self, m: &str) { println!("sms: {m}"); } }

// Trục abstraction: loại notification — giữ một Channel (bridge)
struct Alert { channel: Box<dyn Channel> }
struct Reminder { channel: Box<dyn Channel> }

impl Alert {
    fn fire(&self, what: &str) {
        self.channel.send(&format!("🚨 ALERT: {what}"));
    }
}
```

Giờ thêm kênh mới (Slack) hoặc loại mới (Digest) đều không gây bùng nổ. Hai trục độc lập.

> **Tại sao chọn Bridge:** khi có **hai (hoặc nhiều) chiều biến thiên độc lập** dễ gây bùng nổ tổ hợp. Khác Strategy ở chỗ Bridge nhấn mạnh *cấu trúc lâu dài* (abstraction giữ tham chiếu tới impl), còn Strategy nhấn mạnh *swap thuật toán*. Trong Rust cả hai đều thành "giữ một `Box<dyn Trait>` hoặc generic param".

---

# TẦNG 5 — BEHAVIORAL PATTERNS KIỂU RUST

<a name="21-strategy"></a>
## 21. Strategy → 3 cách (closure / generic / dyn)

Strategy = đóng gói thuật toán để hoán đổi lúc runtime. **Đây là pattern minh họa rõ nhất triết lý "Rust đổi hình dạng pattern".** Có 3 cách, mỗi cách có đánh đổi khác nhau — biết chọn cái nào là dấu hiệu senior.

### Bài toán mẫu: sắp xếp với tiêu chí thay được

### Cách 1: Closure — nhẹ nhất, dùng khi strategy đơn giản & cục bộ

```rust
fn process(data: &mut [i32], strategy: impl Fn(&i32, &i32) -> std::cmp::Ordering) {
    data.sort_by(strategy);
}
process(&mut v, |a, b| a.cmp(b));          // tăng dần
process(&mut v, |a, b| b.cmp(a));          // giảm dần
```

> Closure **chính là Strategy**. Java cần một interface `Comparator` + class. Rust: một dòng. Đây là "pattern tan vào ngôn ngữ".

### Cách 2: Generic + trait — khi strategy phức tạp, biết lúc compile, cần tốc độ

```rust
trait CompressionStrategy {
    fn compress(&self, data: &[u8]) -> Vec<u8>;
}
struct Gzip;
struct Zstd;
impl CompressionStrategy for Gzip { fn compress(&self, d: &[u8]) -> Vec<u8> { todo!() } }
impl CompressionStrategy for Zstd { fn compress(&self, d: &[u8]) -> Vec<u8> { todo!() } }

struct Archiver<C: CompressionStrategy> { strategy: C }
impl<C: CompressionStrategy> Archiver<C> {
    fn archive(&self, d: &[u8]) -> Vec<u8> { self.strategy.compress(d) }
}
// Monomorphize → zero-cost, không vtable. Nhưng kiểu cố định lúc compile.
```

### Cách 3: `Box<dyn>` — khi phải chọn strategy lúc runtime

```rust
struct Archiver { strategy: Box<dyn CompressionStrategy> }
impl Archiver {
    fn set_strategy(&mut self, s: Box<dyn CompressionStrategy>) { self.strategy = s; }
    fn archive(&self, d: &[u8]) -> Vec<u8> { self.strategy.compress(d) }
}
// Đổi strategy lúc chạy (từ config). Có vtable lookup, một bản code.
```

### Bảng quyết định

| Tình huống | Chọn |
|---|---|
| Strategy đơn giản, cục bộ, một lần | **Closure** |
| Strategy phức tạp, biết lúc compile, hot path cần tốc độ | **Generic `<C>`** |
| Phải đổi strategy lúc runtime / lưu nhiều strategy khác kiểu trong cùng collection | **`Box<dyn>`** |

> **Tại sao quan trọng:** câu hỏi "static hay dynamic dispatch" lặp lại ở *mọi* pattern hành vi. Nắm chắc trade-off này (xem sâu ở chương **c — trait** mục static/dynamic dispatch) là nền của thiết kế Rust.

---

<a name="22-observer"></a>
## 22. Observer → channel & callback (vì sao đau với borrow)

Observer = subject thông báo cho nhiều observer khi state đổi. **Đây là pattern đau nhất khi mang từ OOP sang Rust.**

### Vì sao classic Observer chống lại Rust

Classic: subject giữ `Vec<&mut dyn Observer>`, gọi `observer.update()` khi đổi. Observer có thể giữ ref ngược tới subject. Điều này tạo:
- **Aliasing**: nhiều ref tới observer.
- **Mutation đồng thời**: subject sửa mình + gọi observer sửa nó.
- **Vòng tham chiếu**: subject ↔ observer.

Borrow checker từ chối thẳng. Bạn *có thể* ép qua với `Rc<RefCell<dyn Observer>>` + `Weak` để cắt vòng, nhưng nó xấu, dễ panic runtime (double borrow), và là dấu hiệu bạn đang "viết Java bằng Rust".

### Cách Rust: message passing qua channel

Thay vì observer giữ ref và bị gọi, **subject gửi message vào channel; observer tự đọc**. Tách quyền sở hữu hoàn toàn.

```rust
use std::sync::mpsc;

enum Event { PriceChanged(f64), Closed }

let (tx, rx) = mpsc::channel();

// Subject: chỉ gửi event, không biết ai nghe
std::thread::spawn(move || {
    tx.send(Event::PriceChanged(42.0)).unwrap();
    tx.send(Event::Closed).unwrap();
});

// Observer: tự xử lý, sở hữu state của mình
for ev in rx {
    match ev {
        Event::PriceChanged(p) => println!("giá mới: {p}"),
        Event::Closed => break,
    }
}
```

Nhiều observer → `tokio::sync::broadcast` (mỗi subscriber nhận một bản copy). Đây là cách hệ thống thật làm (event bus, pub/sub).

### Khi callback đơn giản là đủ

Nếu observer chỉ là hàm không giữ state phức tạp, một `Vec<Box<dyn Fn(&Event)>>` là đủ:

```rust
struct EventEmitter {
    listeners: Vec<Box<dyn Fn(&Event)>>,
}
impl EventEmitter {
    fn on(&mut self, f: impl Fn(&Event) + 'static) { self.listeners.push(Box::new(f)); }
    fn emit(&self, e: &Event) { for l in &self.listeners { l(e); } }
}
```

> **Tại sao chọn channel thay vì callback giữ ref:** ownership rõ ràng, không vòng tham chiếu, thread-safe sẵn, dễ test. **Bài học lớn:** khi một pattern OOP chống lại borrow checker, đừng cố ép — hỏi "Rust giải bài toán *gốc* này thế nào?". Với Observer, câu trả lời là message passing (xem chương **f — async** và **w — networking** cho channel/actor).

---

<a name="23-state"></a>
## 23. State → enum match vs typestate

State pattern = hành vi object đổi theo trạng thái nội bộ; trông như object đổi class.

Rust có **hai cách**, chọn theo: trạng thái đổi lúc *runtime* hay biết lúc *compile*.

### Cách A: enum + match — khi chuyển trạng thái xảy ra lúc runtime

```rust
enum TrafficLight {
    Red,
    Yellow,
    Green,
}
impl TrafficLight {
    fn next(self) -> Self {
        match self {
            TrafficLight::Red => TrafficLight::Green,
            TrafficLight::Green => TrafficLight::Yellow,
            TrafficLight::Yellow => TrafficLight::Red,
        }
    }
    fn duration(&self) -> u32 {
        match self { Self::Red => 30, Self::Yellow => 5, Self::Green => 25 }
    }
}
```

Đây là cách *thực dụng nhất* cho phần lớn state machine. Exhaustive match đảm bảo bạn xử lý mọi trạng thái. Trạng thái là dữ liệu → đổi tự do lúc runtime, lưu được, serialize được.

### Cách B: typestate — khi muốn compiler chặn chuyển trạng thái sai (mục 8)

Khi chuyển trạng thái sai phải bị bắt *lúc compile* (firmware, protocol), dùng typestate: mỗi trạng thái là một kiểu, method chỉ tồn tại ở trạng thái hợp lệ.

### Bảng quyết định

| | enum + match | typestate |
|---|---|---|
| Chuyển trạng thái | runtime, linh hoạt | compile-time, cứng |
| Lỗi gọi sai | runtime (`Err`/panic) | **compile error** |
| Lưu/serialize state | dễ | khó (kiểu khác nhau) |
| Số trạng thái nhiều, đổi nhiều | ✅ tốt | ❌ phình impl |
| Bug đắt, flow cố định | ⚠️ ổn | ✅ tốt nhất |

> **Tại sao:** đây lại là câu hỏi "đẩy kiểm tra sang compile-time được không / có đáng không". Nguyên tắc senior: **đẩy lỗi về compile-time khi chi phí lỗi cao và flow ổn định; giữ runtime khi cần linh hoạt.**

---

<a name="24-command"></a>
## 24. Command → closure / enum

Command = đóng gói một hành động (cùng tham số) thành object để: hoãn thực thi, xếp hàng, undo/redo, log.

### Closure là Command

```rust
let mut queue: Vec<Box<dyn FnOnce()>> = Vec::new();
queue.push(Box::new(|| println!("gửi email")));
queue.push(Box::new(|| println!("ghi log")));
for cmd in queue { cmd(); }   // thực thi sau
```

### Enum Command — khi cần inspect/serialize/undo

Closure không serialize được, không "nhìn vào trong" được. Khi cần lưu command xuống đĩa, gửi qua mạng, hoặc undo → dùng **enum**:

```rust
enum Command {
    Insert { pos: usize, text: String },
    Delete { pos: usize, len: usize },
}

struct Editor { content: String, history: Vec<Command> }
impl Editor {
    fn apply(&mut self, cmd: Command) {
        match &cmd {
            Command::Insert { pos, text } => self.content.insert_str(*pos, text),
            Command::Delete { pos, len } => { self.content.replace_range(*pos..*pos+*len, ""); }
        }
        self.history.push(cmd);   // lưu để undo
    }
}
```

Enum command serialize được (gửi qua network → đây là nền của event sourcing/CQRS, mục 41), inspect được, và undo được (lưu nghịch đảo).

> **Tại sao chọn:** closure cho command dùng ngay/cục bộ; **enum** khi cần lưu, gửi, inspect, hoặc undo command. Quy tắc: *cần dữ liệu hóa hành vi (reify) → enum; chỉ cần hoãn thực thi → closure.*

---

<a name="25-visitor"></a>
## 25. Visitor → match / serde / double dispatch

Visitor trong OOP là pattern phức tạp nhất: thêm thao tác mới lên một cây object mà không sửa các class object (double dispatch).

### Vì sao Rust gần như xóa sổ Visitor

Visitor tồn tại vì OOP **khó thêm operation mới lên class hierarchy** (phải sửa mọi class). Nhưng Rust có **enum + match**: thêm operation = thêm một hàm `match`, không đụng vào định nghĩa enum.

```rust
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

// "Visitor" #1: đếm node — chỉ là một hàm match
fn count_nodes(j: &Json) -> usize {
    match j {
        Json::Array(items) => 1 + items.iter().map(count_nodes).sum::<usize>(),
        Json::Object(kvs) => 1 + kvs.iter().map(|(_, v)| count_nodes(v)).sum::<usize>(),
        _ => 1,
    }
}
// "Visitor" #2: serialize — hàm match khác. Không sửa enum Json.
```

### "Expression problem" — đánh đổi cốt lõi

- **enum + match**: dễ thêm *operation* mới (hàm mới), khó thêm *biến thể* mới (sửa enum → mọi match phải cập nhật — nhưng compiler nhắc bạn!).
- **trait object (`Box<dyn>`)**: dễ thêm *kiểu* mới (impl trait), khó thêm *operation* mới (sửa trait → mọi impl phải cập nhật).

Senior chọn theo: *trục nào hay biến đổi hơn?* Nếu thêm operation nhiều → enum. Nếu thêm kiểu nhiều (plugin) → trait.

### Khi Visitor thực sự xuất hiện: serde

`serde` dùng Visitor pattern *thật* (trait `Visitor` với `visit_str`, `visit_i64`...) vì nó phải xử lý dữ liệu *streaming* mà không biết kiểu trước. Đây là trường hợp Visitor chính đáng — bạn hiếm khi tự viết, nhưng nên biết khi đọc code serde.

> **Tại sao (hiếm khi) chọn Visitor thủ công:** chỉ khi cần tách traversal khỏi operation trên cấu trúc *mở* và streaming (như parser). 95% trường hợp: dùng enum + match.

---

<a name="26-template-method"></a>
## 26. Template Method → default trait method

Template Method = định nghĩa khung thuật toán, để "subclass" điền vào các bước. OOP làm bằng abstract method + concrete method trong cùng class cha.

Rust: **default method trong trait** (khung) + **required method** (bước phải điền).

```rust
trait Report {
    // Các bước con phải điền:
    fn title(&self) -> String;
    fn body(&self) -> String;

    // Template method: khung cố định, gọi các bước trên
    fn render(&self) -> String {
        format!("=== {} ===\n{}\n--- hết ---", self.title(), self.body())
    }
}

struct SalesReport { month: String }
impl Report for SalesReport {
    fn title(&self) -> String { format!("Doanh số {}", self.month) }
    fn body(&self) -> String { "...số liệu...".into() }
    // render() dùng default — khung chung
}
```

`render()` là template; `title`/`body` là hook. Mọi impl chia sẻ khung, chỉ thay phần khác biệt.

> **Tại sao chọn:** nhiều kiểu chia sẻ *cùng một quy trình tổng thể* nhưng khác ở vài bước. Default method là cách Rust-idiomatic, không cần inheritance. Cẩn thận: đừng nhồi quá nhiều logic vào default method khiến trait nặng — đó là dấu hiệu nên tách thành nhiều trait nhỏ.

---

<a name="27-chain"></a>
## 27. Chain of Responsibility → middleware / Option chaining

Chain of Responsibility = một request đi qua chuỗi handler; mỗi handler hoặc xử lý hoặc chuyển tiếp.

### Hình dạng 1: middleware (đã gặp ở Decorator/tower, mục 15)

Tower/axum middleware *chính là* chain of responsibility: mỗi layer quyết định xử lý (trả response sớm, ví dụ auth fail) hay gọi `next` chuyển tiếp.

### Hình dạng 2: Option/Result combinator chaining

Rust có chuỗi xử lý built-in qua `Option`/`Result`:

```rust
fn lookup(key: &str) -> Option<String> {
    cache_lookup(key)            // thử cache
        .or_else(|| db_lookup(key))     // không có → thử DB
        .or_else(|| default_value(key)) // không có → mặc định
}
```

`or_else` chain = mỗi nguồn là một "handler", thử lần lượt tới khi có kết quả. Sạch hơn nhiều so với chuỗi `if let` lồng nhau.

### Hình dạng 3: chuỗi handler tường minh

```rust
trait Handler {
    fn handle(&self, req: &Request) -> Option<Response>;  // None = chuyển tiếp
}

fn dispatch(handlers: &[Box<dyn Handler>], req: &Request) -> Response {
    for h in handlers {
        if let Some(resp) = h.handle(req) {
            return resp;   // handler này xử lý được → dừng
        }
    }
    Response::not_found()
}
```

> **Tại sao chọn:** khi một request có thể được xử lý bởi *một trong nhiều* handler, thứ tự quan trọng, và bạn muốn dễ thêm/bớt/sắp xếp lại handler. Web framework là ví dụ kinh điển. Với fallback đơn giản, dùng `or_else` chain thay vì dựng cả cấu trúc handler.

---

<a name="28-others"></a>
## 28. Iterator, Mediator, Memento — ngắn gọn

**Iterator** — đã là một phần của ngôn ngữ. Bạn không "implement Iterator pattern", bạn `impl Iterator for T`. Toàn bộ chương **m — iterator** dành cho nó. Đây là minh chứng mạnh nhất cho "pattern tan vào ngôn ngữ".

**Mediator** — object trung tâm điều phối giao tiếp giữa nhiều object để chúng không gọi trực tiếp nhau. Trong Rust hiếm khi viết thủ công; thay vào đó dùng **event bus / channel / actor** (mục 34). Lý do: mediator giữ ref tới mọi component → đau với borrow, y như Observer. Channel giải quyết gọn.

**Memento** — lưu snapshot trạng thái để khôi phục (undo). Trong Rust: `#[derive(Clone)]` + lưu bản clone, hoặc lưu enum Command nghịch đảo (mục 24). `Clone` cho snapshot toàn bộ; command-log cho undo từng bước (rẻ bộ nhớ hơn).

```rust
struct Editor { content: String }
struct Snapshot(String);   // Memento
impl Editor {
    fn save(&self) -> Snapshot { Snapshot(self.content.clone()) }
    fn restore(&mut self, s: Snapshot) { self.content = s.0; }
}
```

---

# TẦNG 6 — PATTERNS ĐẶC SẢN RUST

Đây là những pattern **không có trong GoF**, sinh ra từ chính đặc thù Rust. Senior Rust dùng chúng nhiều hơn cả pattern GoF.

<a name="29-sealed-trait"></a>
## 29. Sealed trait — trait chỉ crate bạn implement được

### Bài toán

Bạn publish một trait public. Người dùng crate có thể `impl` nó cho kiểu của họ. Nhưng đôi khi bạn *không muốn* thế — bạn muốn trait chỉ áp dụng cho một tập kiểu cố định bạn kiểm soát (để giữ bất biến, để thêm method sau mà không phá backward-compat).

### Giải pháp: sealed trait

```rust
mod sealed {
    pub trait Sealed {}   // private trait trong module riêng
}

// Trait public yêu cầu supertrait Sealed (private) → người ngoài không impl Sealed được
pub trait Shape: sealed::Sealed {
    fn area(&self) -> f64;
}

pub struct Circle { r: f64 }
impl sealed::Sealed for Circle {}     // chỉ crate bạn làm được
impl Shape for Circle { fn area(&self) -> f64 { 3.14 * self.r * self.r } }
```

Người dùng crate có thể *gọi* `Shape`, nhưng không thể `impl Shape` cho kiểu của họ (vì không thể `impl Sealed`).

> **Tại sao chọn:** khi bạn muốn kiểm soát hoàn toàn tập kiểu implement một trait (giữ bất biến nội bộ, tự do thêm method sau này mà không phá code người dùng). Dùng nhiều trong thư viện chuẩn và crate lớn (ví dụ trait nội bộ của `std`, `sqlx`).

---

<a name="30-extension-trait"></a>
## 30. Extension trait — thêm method cho kiểu ngoài

### Bài toán

Bạn muốn thêm method tiện ích cho một kiểu *không thuộc crate bạn* (ví dụ `Result`, `Vec`, kiểu của crate khác). Orphan rule cấm `impl` trait ngoài cho kiểu ngoài, và bạn không sửa được kiểu ngoài.

### Giải pháp: định nghĩa trait MỚI + blanket impl

```rust
trait VecExt<T> {
    fn second(&self) -> Option<&T>;
}
impl<T> VecExt<T> for Vec<T> {       // trait của bạn (ok), kiểu ngoài
    fn second(&self) -> Option<&T> { self.get(1) }
}

let v = vec![10, 20, 30];
println!("{:?}", v.second());   // Some(20) — method "thêm" cho Vec
```

Quy ước đặt tên: `XxxExt`. Đây là cách `itertools` thêm hàng chục method cho mọi `Iterator` (`IteratorExt`), `tokio` thêm `AsyncReadExt`, v.v.

> **Tại sao chọn:** thêm hành vi tiện ích cho kiểu ngoài một cách an toàn (qua orphan rule), giữ API gọn (`.method()` thay vì `helper(x)`). Người dùng phải `use` trait để thấy method — vừa là phiền vừa là tính năng (không ô nhiễm namespace).

---

<a name="31-marker-phantom"></a>
## 31. Marker type & PhantomData

### Marker trait — gắn thuộc tính lên kiểu mà không thêm method

`Send`, `Sync`, `Copy`, `Sized` là marker trait — chúng *không có method*, chỉ đánh dấu "kiểu này có tính chất X". Compiler dùng chúng để quyết định an toàn (ví dụ chỉ `T: Send` mới gửi qua thread được).

Bạn có thể tự định nghĩa marker để gắn ngữ nghĩa compile-time.

### PhantomData — "giả vờ" sở hữu một kiểu mà không lưu nó

`PhantomData<T>` là zero-sized, nhưng nói với compiler "tôi hành xử *như thể* sở hữu một `T`". Dùng cho:

**(a) Typestate** (mục 8) — đánh dấu trạng thái.

**(b) Đơn vị đo / phantom type** — phân biệt giá trị cùng layout nhưng khác ngữ nghĩa, ở mức generic:

```rust
use std::marker::PhantomData;

struct Meters;
struct Feet;

struct Length<Unit> {
    value: f64,
    _unit: PhantomData<Unit>,
}
impl<U> Length<U> {
    fn new(v: f64) -> Self { Length { value: v, _unit: PhantomData } }
}

// Length<Meters> và Length<Feet> là KIỂU KHÁC NHAU → không cộng nhầm
fn add_lengths(a: Length<Meters>, b: Length<Meters>) -> Length<Meters> {
    Length::new(a.value + b.value)
}
// add_lengths(meters, feet); // ❌ compile error
```

> **Tại sao chọn:** mã hóa thông tin compile-time (đơn vị, trạng thái, quyền sở hữu lifetime/variance) vào kiểu mà *không tốn byte runtime*. Đây là công cụ nền của typestate và type-safe API. Xem chương **x** (ZST) và **j — lifetime** (variance) để hiểu sâu.

---

<a name="32-interior-mutability"></a>
## 32. Interior mutability pattern

### Bài toán

Đôi khi bạn cần sửa dữ liệu qua một tham chiếu *bất biến* (`&self`). Ví dụ: cache bên trong một struct logic-immutable, đếm số lần truy cập, lazy init. Quy tắc borrow thông thường cấm điều này.

### Giải pháp: Cell / RefCell / Mutex (chương **i** đào sâu)

```rust
use std::cell::RefCell;

struct Memoizer {
    cache: RefCell<std::collections::HashMap<u64, u64>>,  // sửa được qua &self
}
impl Memoizer {
    fn fib(&self, n: u64) -> u64 {    // &self, không &mut self
        if let Some(&v) = self.cache.borrow().get(&n) { return v; }
        let v = if n < 2 { n } else { self.fib(n-1) + self.fib(n-2) };
        self.cache.borrow_mut().insert(n, v);   // sửa qua tham chiếu bất biến
        v
    }
}
```

> **Bản chất:** interior mutability *dời việc kiểm tra borrow từ compile-time sang runtime*. `Cell`/`RefCell` cho single-thread; `Mutex`/`RwLock` cho multi-thread. Đổi lại: `RefCell` có thể **panic lúc runtime** nếu vi phạm borrow (mượn `mut` khi đang mượn).
>
> **Tại sao chọn:** khi cần "mutation logic-vô hại qua `&self`" (cache, counter, observer list, lazy init). ⚠️ **Đừng lạm dụng `Rc<RefCell<T>>`** như cây búa vạn năng để né borrow checker — đó là antipattern số 1 của người mới (mục 49). Dùng khi thật sự cần, không phải khi lười suy nghĩ về ownership.

---

<a name="33-entry-cow"></a>
## 33. Entry API & Cow

### Entry API — pattern "tra cứu rồi sửa" của HashMap

Bài toán: "nếu key tồn tại thì sửa, không thì chèn mặc định". Cách ngây thơ tra map *hai lần*:

```rust
// ❌ tra 2 lần
if !map.contains_key(&k) { map.insert(k, 0); }
*map.get_mut(&k).unwrap() += 1;

// ✅ Entry API: tra 1 lần, rõ ý đồ
*map.entry(k).or_insert(0) += 1;
```

`entry()` trả về một "chỗ" trong map (`Entry` enum: Occupied/Vacant) → bạn thao tác một lần. Đây là pattern API đặc trưng Rust, hiệu quả và idiomatic.

### Cow — Clone On Write: chỉ copy khi thật sự cần sửa

`Cow<str>` (Clone on Write) cho phép giữ *hoặc* tham chiếu mượn *hoặc* giá trị sở hữu — chỉ clone khi cần ghi:

```rust
use std::borrow::Cow;

// Trả về dữ liệu đã "làm sạch". Nếu input đã sạch → trả tham chiếu (0 alloc).
// Nếu phải sửa → clone rồi sửa.
fn sanitize(input: &str) -> Cow<str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))   // phải sửa → sở hữu
    } else {
        Cow::Borrowed(input)                   // sạch sẵn → mượn, không alloc
    }
}
```

> **Tại sao chọn Cow:** tối ưu hiệu năng khi *phần lớn* trường hợp không cần sửa/clone (parsing, normalization, config). Tránh allocation thừa. Xem chương **i** và **k — performance**.

---

# TẦNG 7 — CONCURRENCY PATTERNS

Đây là nơi Rust tỏa sáng nhất ("fearless concurrency"). Các pattern ở đây quan trọng cho hệ thống lớn.

<a name="34-actor"></a>
## 34. Actor model qua channel

### Bài toán

Nhiều task cần truy cập/sửa một state chung. Cách `Arc<Mutex<State>>` (mục 35) gây lock contention và dễ deadlock khi logic phức tạp.

### Giải pháp: Actor — state thuộc một task, giao tiếp qua message

Một task *sở hữu* state. Không ai chạm trực tiếp. Muốn tác động → gửi message; actor xử lý tuần tự. **Không lock, không data race** vì chỉ một task chạm state.

```rust
use tokio::sync::{mpsc, oneshot};

enum Msg {
    Get { resp: oneshot::Sender<u64> },
    Incr,
}

// Actor: vòng lặp sở hữu state
async fn counter_actor(mut rx: mpsc::Receiver<Msg>) {
    let mut count: u64 = 0;     // state riêng tư của actor
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::Incr => count += 1,
            Msg::Get { resp } => { let _ = resp.send(count); }
        }
    }
}

// Handle: cách bên ngoài nói chuyện với actor
#[derive(Clone)]
struct CounterHandle { tx: mpsc::Sender<Msg> }
impl CounterHandle {
    async fn incr(&self) { self.tx.send(Msg::Incr).await.unwrap(); }
    async fn get(&self) -> u64 {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Msg::Get { resp: tx }).await.unwrap();
        rx.await.unwrap()
    }
}
```

Mẫu `oneshot` cho request-response (gửi kèm "địa chỉ trả lời") là kỹ thuật cốt lõi của actor.

> **Tại sao chọn Actor:** state phức tạp truy cập đồng thời, muốn tránh lock contention/deadlock, muốn mỗi đơn vị có ranh giới rõ. Đây là cách Observer/Mediator được hiện thực đúng trong Rust async. Crate: `actix`, `ractor`, hoặc tự viết với tokio channel. Xem chương **f — async** và **w — networking**.

---

<a name="35-shared-state"></a>
## 35. Shared state (Arc<Mutex>) và khi nào tránh

### Pattern cơ bản: `Arc<Mutex<T>>`

Khi nhiều thread cần chia sẻ và sửa cùng dữ liệu:

```rust
use std::sync::{Arc, Mutex};

let shared = Arc::new(Mutex::new(0u64));     // Arc: chia sẻ sở hữu; Mutex: chỉ 1 ghi
let mut handles = vec![];
for _ in 0..4 {
    let s = Arc::clone(&shared);
    handles.push(std::thread::spawn(move || {
        let mut guard = s.lock().unwrap();   // acquire (RAII guard, mục 9)
        *guard += 1;
    }));   // guard drop → unlock tự động
}
```

- `Arc` = Atomic Reference Counted — chia sẻ ownership giữa thread (chương **i**).
- `Mutex` = chỉ một thread ghi tại một thời điểm. `RwLock` = nhiều đọc HOẶC một ghi (chọn khi đọc >> ghi).

### Khi nào TRÁNH

- **Hot path, contention cao** → lock thành nút thắt cổ chai. Cân nhắc: sharding (nhiều lock nhỏ), atomic (`AtomicU64` cho counter đơn giản), hoặc actor (mục 34).
- **Logic phức tạp giữ lock lâu** → dễ deadlock. Giữ lock càng ngắn càng tốt; *không* gọi await/IO khi đang giữ lock.
- **Async** → dùng `tokio::sync::Mutex`, KHÔNG dùng `std::sync::Mutex` xuyên `.await` (chương **f**).

> **Tại sao chọn:** chia sẻ state đơn giản, contention thấp. Là điểm khởi đầu hợp lý; **đo trước khi tối ưu** (chương **k**) — đừng vội nhảy sang actor/lock-free khi chưa có bằng chứng contention.

---

<a name="36-worker-pool"></a>
## 36. Worker pool / pipeline / fan-out fan-in

### Worker pool — N worker tiêu thụ từ một hàng đợi chung

```rust
use std::sync::{Arc, Mutex, mpsc};
// Mô hình: 1 channel công việc, N worker thread cùng nhận.
let (tx, rx) = mpsc::channel::<Job>();
let rx = Arc::new(Mutex::new(rx));
for _ in 0..num_cpus {
    let rx = Arc::clone(&rx);
    std::thread::spawn(move || {
        while let Ok(job) = rx.lock().unwrap().recv() {
            job.run();
        }
    });
}
```

Trong thực tế dùng **`rayon`** (data parallelism: `.par_iter()`) hoặc **`tokio` task** thay vì tự dựng pool. Tự dựng chỉ khi cần kiểm soát đặc biệt.

### Pipeline — chuỗi stage nối bằng channel

```text
[Source] --ch1--> [Parse] --ch2--> [Transform] --ch3--> [Sink]
```

Mỗi stage là một task, đọc từ channel vào, ghi ra channel ra. Tách biệt, dễ song song hóa từng stage, backpressure tự nhiên (channel đầy → chặn).

### Fan-out / Fan-in

- **Fan-out**: chia một dòng việc cho nhiều worker (tăng throughput).
- **Fan-in**: gom kết quả từ nhiều worker về một nơi (qua một channel chung hoặc `futures::stream::FuturesUnordered`).

> **Tại sao chọn:** xử lý lượng lớn công việc song song. Worker pool cho job đồng nhất; pipeline khi xử lý có nhiều giai đoạn rõ rệt; fan-out/in khi cần scale throughput. Ưu tiên `rayon` (CPU-bound) / `tokio` (IO-bound) trước khi tự viết. Xem chương **m — iterator** (rayon) và **f — async**.

---

# TẦNG 8 — KIẾN TRÚC HỆ THỐNG LỚN

Từ đây ta nói về pattern *kiến trúc* — tổ chức cả codebase, không chỉ vài struct.

<a name="37-di"></a>
## 37. Dependency Injection kiểu Rust (không cần framework)

### Vì sao Rust không có (và không cần) DI framework như Spring

Java cần Spring/Guice vì: tạo object qua reflection, wiring runtime phức tạp, để test cần thay implementation. Rust giải quyết DI **bằng chính type system**, không cần framework, không reflection.

### Cách 1: Generic — DI compile-time, zero-cost

```rust
trait UserRepo {
    fn find(&self, id: u64) -> Option<User>;
}

// Service phụ thuộc vào TRAIT, không vào kiểu cụ thể
struct UserService<R: UserRepo> { repo: R }
impl<R: UserRepo> UserService<R> {
    fn new(repo: R) -> Self { UserService { repo } }
    fn get_name(&self, id: u64) -> Option<String> {
        self.repo.find(id).map(|u| u.name)
    }
}

// Production: tiêm Postgres repo. Test: tiêm mock. Cùng một code.
// let svc = UserService::new(PostgresRepo::new(pool));
// let svc = UserService::new(MockRepo::default());
```

Đây là **dependency injection** đúng nghĩa: dependency (`repo`) được *tiêm từ ngoài* qua constructor, service không tự tạo nó. Test thay mock dễ dàng — không mock framework, chỉ một impl khác.

### Cách 2: `Box<dyn Trait>` / `Arc<dyn Trait>` — DI runtime

Khi cần chọn implementation lúc runtime, hoặc lưu nhiều service khác kiểu cùng chỗ (như app state của web server):

```rust
struct AppState {
    user_repo: Arc<dyn UserRepo + Send + Sync>,
    mailer: Arc<dyn Mailer + Send + Sync>,
}
```

axum truyền `AppState` này qua extractor tới mọi handler — đây là DI ở quy mô web app (chương **q**).

> **Tại sao chọn cách nào:** generic khi dependency biết lúc compile và cần tốc độ (phổ biến cho library); `Arc<dyn>` khi cần linh hoạt runtime / lưu trong state chung (phổ biến cho application). **Nguyên tắc cốt lõi: phụ thuộc vào trait (abstraction), không vào kiểu cụ thể** — đây là Dependency Inversion, nền của test được và thay thế được.

---

<a name="38-repository"></a>
## 38. Repository pattern

Repository = trừu tượng hóa lớp lưu trữ sau một interface giống-collection, tách business logic khỏi chi tiết DB.

```rust
// Trait repository — business logic chỉ biết cái này
trait OrderRepository {
    fn save(&self, order: &Order) -> Result<(), RepoError>;
    fn by_id(&self, id: OrderId) -> Result<Option<Order>, RepoError>;
    fn by_customer(&self, c: CustomerId) -> Result<Vec<Order>, RepoError>;
}

// Impl thật: Postgres
struct PgOrderRepo { pool: sqlx::PgPool }
// impl OrderRepository for PgOrderRepo { ... dùng sqlx ... }

// Impl test: in-memory
struct InMemoryOrderRepo { data: std::sync::Mutex<Vec<Order>> }
// impl OrderRepository for InMemoryOrderRepo { ... }
```

Business logic nhận `&dyn OrderRepository` (hoặc generic) → không biết và không quan tâm đó là Postgres, SQLite, hay HashMap. Đổi DB, test, hay cache đều không đụng business logic.

> **Tại sao chọn:** tách *cái gì* (business: lưu order) khỏi *làm sao* (Postgres/sqlx). Cho phép test không cần DB thật, đổi DB dễ, đặt cache/logging ở một chỗ. ⚠️ Đừng over-abstract: nếu app nhỏ chỉ dùng một DB mãi mãi, repository có thể là indirection thừa — chỉ thêm khi có *nhu cầu thật* (test/đa nguồn). Xem chương **r — database**.

---

<a name="39-hexagonal"></a>
## 39. Hexagonal / Ports & Adapters

Hexagonal Architecture (Alistair Cockburn) = đặt **domain logic ở trung tâm**, mọi thứ bên ngoài (DB, HTTP, message queue, CLI) giao tiếp qua **port** (trait) được hiện thực bởi **adapter**.

```text
        ┌─────────────────────────────────────┐
        │   Adapters (driving — gọi vào)        │
        │   HTTP handler · CLI · gRPC           │
        └───────────────┬─────────────────────┘
                        │ gọi port (trait)
        ┌───────────────▼─────────────────────┐
        │         DOMAIN (core)                │
        │   business logic THUẦN               │
        │   không biết HTTP/SQL là gì          │
        │   định nghĩa các PORT (trait)        │
        └───────────────┬─────────────────────┘
                        │ gọi port (trait)
        ┌───────────────▼─────────────────────┐
        │   Adapters (driven — bị gọi)          │
        │   PostgresRepo · RedisCache · S3      │
        └─────────────────────────────────────┘
```

- **Port** = trait do domain định nghĩa (`OrderRepository`, `PaymentGateway`, `EmailSender`).
- **Adapter** = impl cụ thể (`PgOrderRepo`, `StripeGateway`).
- **Domain** = struct + logic thuần, *không import* sqlx/axum/reqwest. Chỉ phụ thuộc trait nó tự định nghĩa.

Trong Rust, biểu diễn này tự nhiên: domain là một crate (hoặc module) không có dependency tới infra; infra crate `impl` các trait của domain.

> **Tại sao chọn:** hệ thống đủ lớn, sống lâu, cần thay đổi infra (đổi DB, thêm gRPC bên cạnh HTTP) mà không đụng business logic; cần test domain không cần infra. ⚠️ Với hệ nhỏ đây là over-engineering nặng — chỉ áp dụng khi domain phức tạp và team lớn. Là đỉnh cao của "phụ thuộc vào abstraction" (mục 37) + repository (mục 38) ở quy mô kiến trúc.

---

<a name="40-plugin"></a>
## 40. Plugin architecture

Cho phép mở rộng hệ thống bằng module bên ngoài mà không sửa core.

### Cách 1: trait object registry (plugin compile cùng)

```rust
trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, input: &str) -> String;
}

struct PluginRegistry { plugins: Vec<Box<dyn Plugin>> }
impl PluginRegistry {
    fn register(&mut self, p: Box<dyn Plugin>) { self.plugins.push(p); }
    fn run_all(&self, input: &str) -> Vec<String> {
        self.plugins.iter().map(|p| p.execute(input)).collect()
    }
}
```

### Cách 2: dynamic loading (`.so`/`.dll` qua `libloading`)

Khi plugin được biên dịch *riêng* và nạp lúc runtime. Phức tạp hơn (ABI không ổn định trong Rust → cần `extern "C"` interface), nhưng cho phép plugin của bên thứ ba không cần recompile core.

### Cách 3: WASM plugin (an toàn nhất)

Nạp plugin dưới dạng WebAssembly (qua `wasmtime`/`extism`) → sandbox, an toàn, đa ngôn ngữ. Đây là xu hướng hiện đại (xem chương **t — wasm**).

> **Tại sao chọn:** hệ thống cần mở rộng bởi bên thứ ba hoặc bật/tắt tính năng theo cấu hình. Trait registry cho plugin nội bộ; `libloading` cho native plugin; WASM cho sandbox/đa ngôn ngữ. Đánh đổi tăng dần: linh hoạt hơn = phức tạp + rủi ro an toàn hơn.

---

<a name="41-event-driven"></a>
## 41. Event-driven / CQRS / Event Sourcing

### Event-driven: component giao tiếp qua event, không gọi trực tiếp

Đã gặp mầm mống ở Observer (mục 22) và Actor (mục 34). Ở quy mô kiến trúc: một **event bus** (channel, hoặc Kafka/NATS) chuyển event giữa các service. Producer không biết consumer. Giảm coupling cực mạnh, dễ scale.

### CQRS — Command Query Responsibility Segregation

Tách **đường ghi** (command: thay đổi state) khỏi **đường đọc** (query: đọc state). Hai mô hình dữ liệu, hai code path:

```rust
// Lệnh ghi — trả về kết quả/lỗi, không trả data
enum Command { PlaceOrder { items: Vec<Item> }, CancelOrder { id: OrderId } }
// Truy vấn đọc — chỉ đọc, có thể từ read-model tối ưu riêng
enum Query { OrderStatus { id: OrderId }, CustomerOrders { id: CustomerId } }
```

Lợi: tối ưu đọc và ghi độc lập (đọc có thể từ replica/cache, ghi qua validation chặt). Enum Command ở đây chính là Command pattern (mục 24) ở tầm kiến trúc.

### Event Sourcing — lưu chuỗi event thay vì state hiện tại

Thay vì lưu "balance = 100", lưu *mọi event* dẫn tới nó: `Deposited(50)`, `Deposited(70)`, `Withdrew(20)`. State hiện tại = replay các event.

```rust
enum AccountEvent { Deposited(u64), Withdrew(u64) }

fn replay(events: &[AccountEvent]) -> i64 {
    events.iter().fold(0i64, |bal, e| match e {
        AccountEvent::Deposited(a) => bal + *a as i64,
        AccountEvent::Withdrew(a) => bal - *a as i64,
    })
}
```

Lợi: audit log đầy đủ, time-travel debugging, tái dựng state bất kỳ thời điểm. Giá: phức tạp, cần snapshot để không replay triệu event mỗi lần.

> **Tại sao chọn:** hệ thống lớn, nhiều service, cần audit/scale/decoupling cao (tài chính, e-commerce, hệ phân tán). ⚠️ **Cực kỳ dễ over-engineer.** Chỉ dùng khi nghiệp vụ *thật sự* cần (yêu cầu audit, mô hình đọc/ghi lệch nhau rõ rệt). Với CRUD app thường, đây là tự bắn vào chân.

---

<a name="42-ddd"></a>
## 42. DDD với newtype & make invalid states unrepresentable

Domain-Driven Design ở quy mô kiến trúc, nhưng trong Rust nó *bắt đầu* từ những idiom nhỏ ta đã học:

- **Value Object** = Newtype (mục 6): `Email`, `Money`, `OrderId` — bất biến đóng gói trong kiểu.
- **Aggregate** = struct + invariant bảo vệ bởi private field + method.
- **Make invalid states unrepresentable** (nguyên tắc 1) áp dụng triệt để: dùng enum cho state, newtype cho mọi giá trị domain.

```rust
// Domain model nói lên nghiệp vụ, không phải "struct với String và i32"
struct Order {
    id: OrderId,
    customer: CustomerId,
    status: OrderStatus,        // enum: Pending/Paid/Shipped/Cancelled
    total: Money,
    items: Vec<LineItem>,       // bất biến: order luôn có ≥1 item (bảo vệ qua constructor)
}
enum OrderStatus { Pending, Paid { at: Timestamp }, Shipped { tracking: TrackingNo }, Cancelled }
```

Compiler trở thành đồng minh thực thi nghiệp vụ: không thể có order "đã ship mà không có tracking number" vì biến thể `Shipped` *bắt buộc* có `tracking`.

> **Tại sao chọn:** nghiệp vụ phức tạp, nhiều quy tắc, cần code phản ánh đúng domain và compiler thực thi quy tắc. Rust *cực kỳ hợp* với DDD vì type system mạnh. Đây là nơi mọi idiom Tầng 2 kết tinh thành kiến trúc.

---

# TẦNG 9 — CASE STUDY: TỪ HỆ NHỎ ĐẾN HỆ LỚN

Đây là phần trả lời trực tiếp câu hỏi của bạn: *"design từ hệ thống nhỏ đến lớn, áp dụng pattern nào và tại sao?"*. Ta xây **một** hệ thống — "URL shortener" — và để nó lớn dần. Quan sát **pattern xuất hiện đúng lúc nó cần, không sớm hơn**.

<a name="43-stage0"></a>
## 43. Stage 0: script 30 dòng — ĐỪNG dùng pattern nào

Yêu cầu: rút gọn URL trong một script chạy local.

```rust
use std::collections::HashMap;

fn main() {
    let mut store: HashMap<String, String> = HashMap::new();
    store.insert("abc".into(), "https://rust-lang.org".into());

    if let Some(url) = store.get("abc") {
        println!("redirect → {url}");
    }
}
```

**Pattern dùng: KHÔNG CÓ.** Và đó là *đúng*.

> **Bài học quan trọng nhất của cả chương:** ở quy mô này, mọi pattern đều là over-engineering. Một `HashMap` và vài hàm là đủ. **Senior biết khi nào KHÔNG dùng pattern.** Người mới học pattern xong có xu hướng nhét pattern vào mọi nơi — đó là dấu hiệu *thiếu* trưởng thành, không phải thừa.

Khi nào rời Stage 0? Khi xuất hiện *áp lực thật*: nhiều người gọi, cần lưu bền, cần đổi cách lưu, cần test. Pattern là *phản ứng với áp lực*, không phải trang trí.

---

<a name="44-stage1"></a>
## 44. Stage 1: thêm cấu trúc — Newtype + Builder + Constructor

Áp lực mới: code lớn lên, bắt đầu truyền nhầm string (URL gốc vs mã rút gọn), config tạo service rối.

```rust
// Newtype (mục 6): không nhầm short-code với long-url nữa
struct ShortCode(String);
struct LongUrl(String);

// LongUrl tự validate khi tạo (parse don't validate)
impl LongUrl {
    fn parse(s: String) -> Result<LongUrl, &'static str> {
        if s.starts_with("http") { Ok(LongUrl(s)) } else { Err("URL không hợp lệ") }
    }
}

// Builder (mục 7): service có nhiều config optional
struct ShortenerBuilder { base_url: String, code_len: usize, max_entries: usize }
impl ShortenerBuilder {
    fn new() -> Self { Self { base_url: "http://localhost".into(), code_len: 6, max_entries: 10_000 } }
    fn base_url(mut self, u: impl Into<String>) -> Self { self.base_url = u.into(); self }
    fn code_len(mut self, n: usize) -> Self { self.code_len = n; self }
    fn build(self) -> Shortener { /* ... */ todo!() }
}
```

**Pattern: Newtype + Builder.** *Tại sao:* nhầm kiểu string bắt đầu gây bug → Newtype. Config nhiều tham số optional → Builder. Cả hai là idiom Rust cơ bản, chi phí gần 0, lợi ích an toàn ngay.

---

<a name="45-stage2"></a>
## 45. Stage 2: nhiều implementation — Strategy + Repository (trait)

Áp lực mới: cần lưu vào nhiều nơi (memory cho test, Redis cho production), và cần nhiều thuật toán sinh mã (random, sequential, hash).

```rust
// Repository (mục 38): trừu tượng hóa lưu trữ
trait Storage {
    fn put(&self, code: &ShortCode, url: &LongUrl) -> Result<(), StoreErr>;
    fn get(&self, code: &ShortCode) -> Result<Option<LongUrl>, StoreErr>;
}
struct InMemory { /* HashMap */ }
struct RedisStore { /* conn */ }
// impl Storage cho cả hai

// Strategy (mục 21): thuật toán sinh mã thay được
trait CodeGenerator {
    fn generate(&self) -> ShortCode;
}
struct RandomGen { len: usize }
struct HashGen;

// Service phụ thuộc TRAIT (DI, mục 37) — generic cho zero-cost
struct Shortener<S: Storage, G: CodeGenerator> {
    storage: S,
    gen: G,
}
```

**Pattern: Repository + Strategy + DI qua generic.** *Tại sao:* xuất hiện *nhiều cách làm cùng một việc* (lưu, sinh mã) → trừu tượng hóa sau trait. Giờ test dùng `InMemory`, production dùng `Redis`, *cùng business logic*. Đây là lúc trait + DI bắt đầu trả cổ tức.

> Lưu ý quyết định: ta chọn **generic** (`<S, G>`) vì biết kiểu lúc compile, hot path. Nếu cần đổi storage lúc runtime (từ config) → đổi sang `Box<dyn Storage>`.

---

<a name="46-stage3"></a>
## 46. Stage 3: web service — Hexagonal + DI runtime + Decorator

Áp lực mới: thành dịch vụ web thật (axum), nhiều người dùng đồng thời, cần auth/rate-limit/logging, cần test domain không cần khởi động server.

```rust
// HEXAGONAL (mục 39): domain ở giữa, không biết axum/redis
mod domain {
    // port (trait) do domain định nghĩa — đã có Storage, CodeGenerator
    pub fn shorten(storage: &dyn Storage, gen: &dyn CodeGenerator, url: LongUrl)
        -> Result<ShortCode, DomainErr> { /* logic thuần */ todo!() }
}

// Adapter driving: HTTP (axum) — chỉ dịch HTTP ↔ domain
// Adapter driven: RedisStore impl Storage

// DI runtime (mục 37): app state giữ Arc<dyn>
#[derive(Clone)]
struct AppState {
    storage: Arc<dyn Storage + Send + Sync>,
    gen: Arc<dyn CodeGenerator + Send + Sync>,
}

// DECORATOR (mục 15) qua tower: rate-limit, auth, tracing
// app.layer(TraceLayer).layer(RateLimitLayer).layer(AuthLayer)
```

**Pattern: Hexagonal + DI runtime (`Arc<dyn>`) + Decorator (tower middleware).** *Tại sao:*
- Domain tách khỏi HTTP → test logic không cần server, đổi axum→actix không đụng domain.
- `Arc<dyn>` vì app state chia sẻ qua nhiều handler/thread, chọn impl từ config lúc khởi động.
- Middleware cho cross-cutting concerns → không nhồi auth/log vào từng handler.

Đây là kiến trúc của *hầu hết* web service Rust production (chương **q**, **r**).

---

<a name="47-stage4"></a>
## 47. Stage 4: scale — Actor + Event-driven + Plugin

Áp lực mới: hàng triệu request, cần analytics realtime (đếm click), cần mở rộng tính năng (custom domain, QR code) bởi nhiều team, cần audit.

```rust
// ACTOR (mục 34): analytics state thuộc một task, tránh lock contention
//   mọi click gửi message tới analytics actor → xử lý tuần tự, không lock

// EVENT-DRIVEN (mục 41): mỗi sự kiện (UrlCreated, UrlClicked) publish lên bus
enum DomainEvent {
    UrlCreated { code: ShortCode, at: Timestamp },
    UrlClicked { code: ShortCode, at: Timestamp, referer: Option<String> },
}
//   analytics, billing, audit là consumer độc lập → thêm consumer không đụng core

// PLUGIN (mục 40): tính năng mở rộng (QR, custom domain) là plugin
trait Feature: Send + Sync {
    fn on_event(&self, ev: &DomainEvent);
}

// EVENT SOURCING (mục 41) cho audit: lưu chuỗi DomainEvent, replay được
```

**Pattern: Actor + Event-driven + Plugin + (tùy chọn) Event Sourcing.** *Tại sao:*
- Actor: analytics hot, lock sẽ thắt cổ chai → message passing.
- Event-driven: nhiều team thêm tính năng độc lập → decoupling qua event bus.
- Plugin: mở rộng không sửa core.
- Event sourcing: yêu cầu audit + analytics lịch sử.

> ⚠️ Mỗi pattern ở đây thêm *phức tạp đáng kể*. Chỉ thêm khi áp lực thật xuất hiện. Một startup chưa có user **không cần** event sourcing.

---

<a name="48-ban-do-quy-mo"></a>
## 48. Bản đồ: pattern nào xuất hiện ở quy mô nào

```
QUY MÔ          ÁP LỰC XUẤT HIỆN              PATTERN PHẢN ỨNG
─────────────────────────────────────────────────────────────────────
Script          (không có)                    KHÔNG pattern ← quan trọng!
  │
  ▼
Module nhỏ      nhầm kiểu, config rối         Newtype, Builder, Constructor fn
  │
  ▼
Library         nhiều cách làm 1 việc         Strategy, Repository, trait+DI
  │             cần test không cần infra      (generic dispatch)
  ▼
Web service     nhiều client, cross-cutting   Hexagonal, DI runtime (Arc<dyn>),
  │             concerns, tách domain         Decorator/middleware
  ▼
Distributed     scale, nhiều team, audit,     Actor, Event-driven, Plugin,
                analytics realtime            CQRS, Event Sourcing
```

**Quy luật vàng:** pattern là *phản ứng với áp lực*, không phải *điểm khởi đầu*. Đi từ trên xuống — thêm pattern khi (và chỉ khi) áp lực tương ứng xuất hiện. Đi ngược (nhét event sourcing vào script) = over-engineering.

---

# TẦNG 10 — ANTIPATTERNS

<a name="49-antipatterns"></a>
## 49. Những sai lầm chết người

### Antipattern 1: `Rc<RefCell<T>>` khắp nơi để né borrow checker

Đây là antipattern **số 1** của người từ OOP sang. Khi borrow checker từ chối, người mới quấn `Rc<RefCell<>>` quanh mọi thứ để "tắt" nó.

```rust
// ❌ "Viết Java bằng Rust" — graph hai chiều với Rc<RefCell> chằng chịt
type Node = Rc<RefCell<NodeData>>;
struct NodeData { parent: Option<Node>, children: Vec<Node> }
// → borrow panic lúc runtime, vòng tham chiếu rò rỉ bộ nhớ, khó suy luận
```

**Vì sao sai:** mất hết bảo đảm compile-time (chuyển sang runtime panic), rò rỉ bộ nhớ qua reference cycle, code khó hiểu. **Cách đúng:** xem lại *ai thật sự sở hữu cái gì*. Thường lời giải là: dùng index/id thay con trỏ (`Vec<NodeData>` + `usize` id — "arena pattern"), hoặc message passing, hoặc thiết kế lại quan hệ sở hữu một chiều.

### Antipattern 2: Over-engineering — pattern không có bài toán

```rust
// ❌ AbstractWidgetFactoryStrategyBuilder cho một app in "Hello"
```

Nhét pattern khi chưa có áp lực thật (xem Stage 0). Mỗi indirection là một lớp khó debug. **Quy tắc:** chờ áp lực thứ hai (rule of three) rồi mới trừu tượng hóa.

### Antipattern 3: Dịch 1:1 pattern Java sang Rust

Cố viết classic Observer (subject giữ ref tới observer), classic Singleton (`static mut`), deep inheritance giả lập bằng `Deref`. **Vì sao sai:** chống lại grain của Rust → xấu, unsafe, hoặc không compile. **Cách đúng:** hỏi "bài toán *gốc* là gì?" rồi tìm lời giải Rust-native (channel, OnceLock, composition).

### Antipattern 4: Trait quá lớn (God trait)

Một trait 20 method. **Vì sao sai:** khó impl, khó test, vi phạm Interface Segregation. **Cách đúng:** tách thành nhiều trait nhỏ, compose qua supertrait khi cần.

### Antipattern 5: Lạm dụng `Deref` để giả kế thừa

```rust
// ❌ Dùng Deref để Dog "kế thừa" Animal
impl Deref for Dog { type Target = Animal; ... }
```

**Vì sao sai:** `Deref` cho smart pointer, không phải kế thừa. Làm method resolution rối, ngầm định, khó đọc. **Cách đúng:** composition (struct chứa struct) + delegate method tường minh, hoặc trait.

### Antipattern 6: Premature `dyn` / premature generic

Dùng `Box<dyn>` (chậm hơn) khi kiểu biết lúc compile, hoặc generic hóa cái chỉ có một impl mãi mãi. **Cách đúng:** dùng kiểu cụ thể tới khi *thật sự* có nhiều impl hoặc cần runtime polymorphism.

### Antipattern 7: `unwrap()` như xử lý lỗi

Không phải pattern nhưng giết hệ thống production. Xem chương **g — error handling**.

> **Mẫu số chung của mọi antipattern:** áp một giải pháp khi *chưa hiểu bài toán*, hoặc *chống lại* triết lý Rust thay vì thuận theo. Senior fix bằng cách *quay về bài toán gốc*.

---

# TẦNG 11 — SENIOR WISDOM

<a name="50-decision-tree"></a>
## 50. Decision tree chọn pattern

```
Bạn đang gặp bài toán gì?

"Cần phân biệt các giá trị cùng kiểu cơ sở / đảm bảo bất biến"
   → Newtype (+ private field + constructor validate)

"Object có nhiều trường optional, khởi tạo phức tạp"
   → Builder (+ Typestate nếu cần bắt buộc trường lúc compile)

"API có trình tự gọi bắt buộc, muốn lỗi lúc COMPILE"
   → Typestate

"Cần thay thuật toán / hành vi"
   ├─ đơn giản, cục bộ          → Closure
   ├─ phức tạp, biết lúc compile → Generic <T: Trait>
   └─ chọn lúc runtime           → Box<dyn Trait>

"Một object là một trong N dạng đã biết, cố định"
   → enum + match (KHÔNG dùng trait hierarchy)

"Cần thêm hành vi xếp chồng (log/cache/retry/auth)"
   → Decorator / tower middleware

"Cần dọn dẹp tài nguyên tự động"
   → RAII (Drop)

"Nhiều task chia sẻ state"
   ├─ đơn giản, contention thấp  → Arc<Mutex>
   └─ phức tạp / hot              → Actor (channel)

"Component cần thông báo nhau"
   → channel / event bus (KHÔNG classic Observer giữ ref)

"Cần tách business logic khỏi DB/HTTP"
   → Repository (nhỏ) → Hexagonal (lớn)

"Cần test với fake implementation"
   → DI qua trait (generic hoặc Arc<dyn>)

"Cần mở rộng bởi bên thứ ba"
   → Plugin (trait registry → libloading → WASM)

KHI NGHI NGỜ → đừng dùng pattern nào. Chờ áp lực thứ 2.
```

---

<a name="51-12-nguyen-tac"></a>
## 51. 12 nguyên tắc senior về design pattern trong Rust

```
1.  Pattern là PHẢN ỨNG với áp lực, không phải điểm khởi đầu.
    Không có bài toán → không dùng pattern.

2.  Make invalid states unrepresentable. Để compiler thực thi
    bất biến, đừng dựa vào kỷ luật con người.

3.  Parse, don't validate. Phân giải input thô thành kiểu chặt
    tại biên hệ thống; sau đó type system bảo đảm tính hợp lệ.

4.  Composition over inheritance — Rust không cho lựa chọn khác.
    Tái sử dụng qua trait + struct chứa struct + generic.

5.  Phụ thuộc vào ABSTRACTION (trait), không vào kiểu cụ thể.
    Đây là nền của test được, thay thế được, DI.

6.  Static hay dynamic dispatch là quyết định CÓ Ý THỨC:
    generic (nhanh, phình) vs dyn (gọn, vtable). Mặc định generic;
    dùng dyn khi cần runtime polymorphism / giảm code size.

7.  Đẩy lỗi về COMPILE-TIME khi chi phí lỗi cao & flow ổn định
    (typestate). Giữ runtime (enum) khi cần linh hoạt.

8.  Khi pattern OOP chống lại borrow checker, ĐỪNG ép bằng
    Rc<RefCell>. Hỏi "bài toán gốc là gì?" → thường là message passing.

9.  Rule of three: chờ thứ ba lần lặp lại rồi mới trừu tượng hóa.
    Trừu tượng quá sớm = nợ kỹ thuật ngược.

10. Mỗi indirection (Box/Rc/Arc/dyn) có giá: allocation, pointer
    chasing, dispatch, contention. Trả giá có chủ đích, đo trước khi tối ưu.

11. Pattern GoF phần lớn là thuốc giải cho bệnh OOP. Hỏi "Rust có
    bệnh đó không?" trước khi uống thuốc. Nhiều pattern tan vào ngôn ngữ.

12. Senior biết khi nào KHÔNG dùng pattern. Code đơn giản đọc được
    > code "thông minh" đầy pattern. Tối ưu cho người đọc tiếp theo.
```

---

<a name="52-toolkit"></a>
## 52. Toolkit & crates

| Nhu cầu | Crate | Ghi chú |
|---|---|---|
| Sinh Builder tự động | `bon`, `derive_builder`, `typed-builder` | `bon` hiện đại nhất, hỗ trợ named args |
| Global lazy / singleton | `std::sync::LazyLock`/`OnceLock` (1.80+), `once_cell` | ưu tiên std |
| Actor framework | `actix`, `ractor`, `kameo` | hoặc tự viết với tokio channel |
| Data parallelism (worker pool) | `rayon` | `.par_iter()` — CPU-bound |
| Async runtime / task / channel | `tokio` | IO-bound, actor, pipeline |
| Pub/sub broadcast | `tokio::sync::broadcast`, `flume` | observer/event bus |
| Plugin (dynamic load) | `libloading` | native `.so`/`.dll` |
| Plugin (sandbox) | `wasmtime`, `extism` | WASM, an toàn, đa ngôn ngữ |
| Web service (hexagonal/DI) | `axum` + `tower` | middleware = decorator/chain |
| Repository / DB | `sqlx`, `sea-orm`, `diesel` | xem chương **r** |
| Error handling (pattern) | `thiserror` (lib), `anyhow` (app) | xem chương **g** |
| Serialization (Visitor) | `serde` | Visitor pattern thật |
| State machine | `statig`, `sm`, hoặc enum thủ công | typestate thường tự viết |

### Sách & nguồn nên đọc

- **Rust Design Patterns** (rust-unofficial.github.io/patterns) — catalog pattern Rust-native, idioms + antipatterns.
- **Programming Rust** (O'Reilly) — chương về trait/generic là nền của mọi pattern.
- Đọc source: `tokio` (actor/channel), `axum`/`tower` (decorator/chain), `serde` (visitor), `sqlx` (sealed trait, typestate trong query builder).
- *Design Patterns* (GoF) — đọc để hiểu *bài toán gốc*, rồi tự dịch sang Rust (đừng copy giải pháp Java).

---

## Lời kết — từ zero đến senior

Hành trình của bạn:

1. **Zero → Junior:** thuộc idiom Tầng 2 (Newtype, Builder, RAII, Default). Dùng được enum + match thay class hierarchy. Hiểu `Option`/`Result` thay null/exception.

2. **Junior → Mid:** hiểu Strategy 3-cách (closure/generic/dyn), static vs dynamic dispatch, DI qua trait. Biết Repository, Decorator. Bắt đầu thấy pattern GoF "tan" thành gì trong Rust.

3. **Mid → Senior:** hiểu *tại sao* mỗi pattern tồn tại (lực gốc, bệnh OOP), chọn pattern theo áp lực thật, biết khi nào KHÔNG dùng. Thiết kế hệ thống lớn (hexagonal, event-driven, actor) — và quan trọng hơn, biết hệ nào *không cần* chúng.

4. **Senior → hơn nữa:** make-invalid-states-unrepresentable thành phản xạ. Type system trở thành công cụ thiết kế chính. Bạn không "áp pattern" — bạn để bài toán và type system *dẫn* tới cấu trúc đúng.

> **Câu thần chú cuối cùng:** *Đừng hỏi "pattern nào hợp ở đây?". Hỏi "bài toán thật là gì, và đâu là cách nhẹ nhất Rust giải nó?". Pattern sẽ tự lộ ra — hoặc tự biến mất.*

🦀 Đọc kèm chương **c — trait**, **d — generic**, **i — smart pointers**, **f — async** để hiểu sâu cơ chế phía dưới mỗi pattern.
