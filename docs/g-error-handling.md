# Error Handling trong Rust — Phong cách Senior

> Tài liệu thứ 7 trong bộ Rust nền tảng. Đọc sau khi đã nắm:
> - [trait.md](./trait.md) — vì Error là một trait
> - [generic.md](./generic.md) — Result<T,E> là generic
> - [ownership-borrowing.md](./ownership-borrowing.md) — error type bound bởi ownership
> - [async.md](./async.md) — async error có quirk riêng
>
> Tài liệu này không chỉ dạy syntax. Mục tiêu: viết error handling như một **senior** —
> hiểu sâu trade-off giữa ergonomic vs type-safety, library vs application, when to recover vs propagate, error context, observability.
>
> **Triết lý cốt lõi**: Errors là **giá trị bình thường**, không phải exception. Đối xử với chúng như mọi dữ liệu khác.

---

# Mục lục

- [Tầng 1: Triết lý Error trong Rust](#tầng-1-triết-lý-error-trong-rust)
- [Tầng 2: panic! vs Result — Đường phân chia](#tầng-2-panic-vs-result--đường-phân-chia)
- [Tầng 3: Result<T,E> sâu hơn — Các API ít người biết](#tầng-3-resulttte-sâu-hơn--các-api-ít-người-biết)
- [Tầng 4: Operator `?` — Desugar và From conversion](#tầng-4-operator---desugar-và-from-conversion)
- [Tầng 5: std::error::Error trait — Hệ sinh thái error](#tầng-5-stderrorerror-trait--hệ-sinh-thái-error)
- [Tầng 6: thiserror — Library error tốt nhất](#tầng-6-thiserror--library-error-tốt-nhất)
- [Tầng 7: anyhow — Application error tốt nhất](#tầng-7-anyhow--application-error-tốt-nhất)
- [Tầng 8: Library vs Application — Chiến lược chọn lựa](#tầng-8-library-vs-application--chiến-lược-chọn-lựa)
- [Tầng 9: Error context, source chain, backtrace](#tầng-9-error-context-source-chain-backtrace)
- [Tầng 10: Patterns nâng cao của senior](#tầng-10-patterns-nâng-cao-của-senior)
- [Tầng 11: Async error — Những điểm đặc biệt](#tầng-11-async-error--những-điểm-đặc-biệt)
- [Tầng 12: Antipatterns — Sai lầm phổ biến](#tầng-12-antipatterns--sai-lầm-phổ-biến)

---

# Tầng 1: Triết lý Error trong Rust

## 1.1 Errors-as-Values

Trước Rust, có 2 trường phái lớn:

### Trường phái 1: Exceptions (Java/C++/Python)

```java
try {
    String content = readFile("foo.txt");
    process(content);
} catch (IOException e) {
    log.error("Failed", e);
} catch (ParseException e) {
    // ...
}
```

**Ưu**: code "happy path" sạch, ít noise.

**Nhược**:
- Exception là **invisible control flow** — không nhìn signature mà biết hàm có throw gì
- Stack unwinding tốn (cost performance, làm khó async)
- Java's checked exception → ai cũng từng `throws Exception` cho qua
- C++ exception đắt → nhiều codebase ban hẳn exception

### Trường phái 2: Return code (C)

```c
int ret = read_file("foo.txt", &content);
if (ret < 0) { perror("..."); return ret; }
ret = process(content);
if (ret < 0) { ... }
```

**Ưu**: visible, no hidden cost.

**Nhược**: dễ quên check, mã trộn lẫn happy path và error path → khó đọc.

### Rust: Result<T,E> — Errors as Values, NHƯNG type-safe và ergonomic

```rust
let content = read_file("foo.txt")?;   // ? thay cho if ret < 0 return
process(content)?;
```

- **Visible**: signature `-> Result<T, E>` nói rõ hàm có thể fail và fail kiểu gì
- **Compile-time enforced**: bắt buộc handle (không thể "quên")
- **Zero-cost**: chỉ là enum match, không runtime overhead
- **Composable**: combinators (`map`, `and_then`, `?`) làm code gọn

Đây là **best-of-both-worlds** của 2 trường phái trên.

## 1.2 Phân loại lỗi

Senior phân biệt rõ 4 loại lỗi:

### Loại 1: Recoverable errors (lỗi có thể xử lý)

User input sai, network timeout, file không tồn tại... → **dùng `Result<T,E>`**.

```rust
fn parse_age(s: &str) -> Result<u32, ParseIntError> {
    s.parse()  // user nhập sai → trả Err, có thể prompt lại
}
```

### Loại 2: Unrecoverable errors / Bugs (lỗi không thể xử lý)

Index out of bounds, divide by zero, invariant violation... → **dùng `panic!`** (hoặc để Rust panic tự nhiên).

```rust
let arr = vec![1, 2, 3];
let x = arr[100];  // panic — đây là bug logic, không recover được
```

### Loại 3: Pre-condition violations / API misuse

Caller dùng sai API (vd: chia cho 0 do chưa check). → `panic!` (hoặc `Result` nếu API public và caller cần handle).

Triết lý: **API misuse nên panic, environment errors nên return Result**.

### Loại 4: Catastrophic system errors

Out-of-memory, stack overflow, signal SIGSEGV — bạn không xử lý được. Hệ điều hành/runtime lo.

## 1.3 Cây quyết định: Khi nào dùng cái gì?

```
   Lỗi này xảy ra ở runtime?
            │
       ┌────┴────┐
      Có        Không (chỉ có khi bug)
       │             │
   Caller có        debug_assert!() / unreachable!()
   khả năng         (chỉ panic ở debug build)
   recover?
       │
   ┌───┴───┐
  Có      Không
   │       │
 Result   panic!
```

## 1.4 Rust không có exception "thực sự"

Rust panic thực ra **có** unwind stack (mặc định) — nhưng:
- **Không thể catch** trong code thông thường (chỉ `catch_unwind` trong FFI / web server boundary)
- **Không nên dùng** như exception (chậm hơn Result rất nhiều)
- Có thể config `panic = "abort"` để bỏ unwind → binary nhỏ + nhanh hơn

Nếu thấy ai trong Rust code dùng `catch_unwind` để xử lý logic, đó là **anti-pattern** trừ khi có lý do FFI rất cụ thể.

---

# Tầng 2: panic! vs Result — Đường phân chia

## 2.1 Panic là gì?

`panic!` = "chương trình rơi vào trạng thái không xử lý được, abort thread hiện tại".

Mặc định: unwind stack (gọi destructor mọi biến) rồi terminate. Có thể set `panic = "abort"` trong Cargo.toml để skip unwind.

```rust
fn main() {
    let v = vec![1, 2, 3];
    let _ = v[10];   // panic: index out of bounds
}
```

```
thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 10'
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## 2.2 Khi nào dùng panic?

### ✅ Đúng:

1. **Invariant violation** (bug logic):
   ```rust
   fn dequeue(&mut self) -> T {
       if self.items.is_empty() {
           panic!("dequeue on empty queue — caller's bug");
       }
       self.items.pop().unwrap()
   }
   ```

2. **Tests**: `assert!`, `assert_eq!` panic — đây là chuẩn.

3. **Quick prototype / examples**: `unwrap()` để code ngắn — đừng để vào production.

4. **Caller violated API contract**: caller truyền giá trị sai sau khi API document.

5. **Indexing nội bộ mà bạn chắc chắn an toàn**: `arr[i]` thay vì `arr.get(i).unwrap()` — readable hơn, vẫn panic nếu sai.

### ❌ Sai:

1. **Lỗi I/O bình thường**:
   ```rust
   let file = File::open("config.toml").unwrap();  // BAD — file có thể không có
   ```

2. **User input** — không bao giờ panic vì input user.

3. **Network errors** — luôn return Result.

4. **Parsing dữ liệu external** (JSON, config, request body) — Result.

## 2.3 unwrap() và expect() — Anh em với panic

```rust
let x: Result<i32, _> = "42".parse();
let v = x.unwrap();           // panic nếu Err
let v = x.expect("config phải có số");  // panic với message
```

**Senior rule**:
- `unwrap()` trong **production code** = code smell, trừ vài exception
- `expect()` với message rõ ràng > `unwrap()` — khi panic, debug nhanh
- Trong **test code**: dùng thoải mái

### Pattern thay thế tốt hơn

```rust
// ❌
let val = some_map.get(&key).unwrap();

// ✅ — bad pattern nếu key có thể không có
let val = some_map.get(&key)
    .ok_or_else(|| MyError::KeyNotFound(key))?;

// ✅ — bug logic nếu key chắc chắn có
let val = some_map.get(&key)
    .expect("invariant: key must be present after insert");
```

## 2.4 Panic vs Result performance

```
   Result happy path:    ~0 ns (compile to tag check)
   Result error path:    ~0 ns (tag check + branch)
   Panic (unwind):       ~10-100 µs (stack walk, destructor calls)
   Panic (abort):        nhanh hơn, nhưng terminate process
```

Panic là **rất đắt** so với Result trên error path. Đừng dùng panic làm control flow.

## 2.5 Panic không cross-FFI boundary

Panic ngang qua FFI (gọi từ C/C++) = **undefined behavior**. Nếu viết Rust function expose qua FFI, phải `catch_unwind`:

```rust
#[no_mangle]
pub extern "C" fn safe_api() -> i32 {
    std::panic::catch_unwind(|| {
        risky_rust_code();
        0
    }).unwrap_or(-1)
}
```

## 2.6 Panic trong async task

`tokio::spawn(async { panic!(...) })` không crash cả chương trình — runtime catch panic và lưu vào `JoinHandle::join()`:

```rust
let h = tokio::spawn(async { panic!("oops"); });
let result = h.await;  // Err(JoinError) — không crash process
```

Đây vừa là tính năng (isolate task panic) vừa là nguy hiểm (panic bị "nuốt" nếu không join handle).

---

# Tầng 3: Result<T,E> sâu hơn — Các API ít người biết

## 3.1 Định nghĩa

```rust
#[must_use]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

`#[must_use]` — compiler warn nếu bạn drop Result mà không handle. Đừng phớt lờ.

## 3.2 Các method chính (cần thuộc lòng)

### Conversion / Inspection

| Method | Output | Mục đích |
|--------|--------|----------|
| `is_ok()` / `is_err()` | `bool` | Check tag |
| `ok()` | `Option<T>` | Convert (drop error) |
| `err()` | `Option<E>` | Lấy error nếu có |
| `as_ref()` | `Result<&T, &E>` | Borrow inside |
| `as_mut()` | `Result<&mut T, &mut E>` | Mut borrow |
| `as_deref()` | `Result<&T::Target, &E>` | Deref the Ok |

### Unwrapping

| Method | Behavior |
|--------|----------|
| `unwrap()` | Panic nếu Err |
| `unwrap_or(default)` | Trả default nếu Err |
| `unwrap_or_else(\|e\| ...)` | Compute default từ error |
| `unwrap_or_default()` | Dùng `Default::default()` |
| `expect("msg")` | Panic với custom msg |
| `unwrap_err()` | Panic nếu Ok |
| `expect_err("msg")` | Like above with msg |

### Transformation

| Method | Behavior |
|--------|----------|
| `map(\|t\| ...)` | Transform Ok value |
| `map_err(\|e\| ...)` | Transform Err value |
| `and_then(\|t\| ...)` | Chain (flatMap) |
| `or_else(\|e\| ...)` | Fallback to another Result |
| `and(other)` | Replace Ok with other |
| `or(other)` | Replace Err with other |

### Inspection (Rust 1.65+)

```rust
result
    .inspect(|t| println!("got value: {:?}", t))
    .inspect_err(|e| println!("got error: {}", e))?;
```

Inspect không thay đổi Result, chỉ "ngó" — hữu ích cho logging.

## 3.3 Patterns thực tế

### Pattern 1: map_err để chuyển error type

```rust
fn read_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")
        .map_err(|e| ConfigError::Io(e))?;
    let cfg: Config = toml::from_str(&content)
        .map_err(|e| ConfigError::Parse(e))?;
    Ok(cfg)
}
```

(Với `thiserror` + `#[from]`, không cần `map_err` này — xem Tầng 6.)

### Pattern 2: and_then để chain Result-returning operations

```rust
fn process(s: &str) -> Result<u32, String> {
    parse_input(s)
        .and_then(|n| validate(n))
        .and_then(|n| transform(n))
}
```

Tương đương `?` operator nhưng functional style. Personal preference.

### Pattern 3: or_else để fallback

```rust
fn read_config() -> Result<Config> {
    read_from_file("config.local.toml")
        .or_else(|_| read_from_file("config.toml"))
        .or_else(|_| read_from_env())
}
```

### Pattern 4: Collect Result vào Vec

```rust
let inputs = vec!["1", "2", "abc", "4"];

// Option A: fail nếu có 1 cái fail
let nums: Result<Vec<i32>, _> = inputs.iter()
    .map(|s| s.parse::<i32>())
    .collect();
// Err(ParseIntError) — vì "abc" fail

// Option B: thu Ok, bỏ Err
let nums: Vec<i32> = inputs.iter()
    .filter_map(|s| s.parse().ok())
    .collect();
// [1, 2, 4]

// Option C: partition
let (oks, errs): (Vec<_>, Vec<_>) = inputs.iter()
    .map(|s| s.parse::<i32>())
    .partition(Result::is_ok);
```

Đây là API hiệu quả thông minh — đặc biệt option A (collect chuyển `Iterator<Result<T,E>>` → `Result<Vec<T>, E>`).

### Pattern 5: Type alias cho Result

```rust
// Trong crate của bạn
pub type Result<T, E = MyError> = std::result::Result<T, E>;

// Sau đó dùng
fn foo() -> Result<u32> {  // tương đương Result<u32, MyError>
    Ok(42)
}
```

Đây là idiom cực kỳ phổ biến. `anyhow`, `std::io`, `std::fmt` đều có Result alias.

## 3.4 Result với `&self`, `Vec`, async

```rust
// Result trong field
struct Cache {
    data: Result<Vec<u8>, CacheError>,
}

// Vec<Result> - không phải cách hay, thường nên là Result<Vec>
let v: Vec<Result<i32, _>> = ...;

// async Result
async fn fetch() -> Result<String, FetchError> { ... }
// = -> impl Future<Output = Result<String, FetchError>>
```

---

# Tầng 4: Operator `?` — Desugar và From conversion

## 4.1 Cú pháp cơ bản

```rust
fn parse_and_double(s: &str) -> Result<i32, ParseIntError> {
    let n: i32 = s.parse()?;
    Ok(n * 2)
}
```

`s.parse()?`:
- Nếu `Ok(n)` → unwrap, gán `n`
- Nếu `Err(e)` → **early return** với `Err(e)`

## 4.2 Desugar — Hiểu chính xác `?` làm gì

Rust 1.0 đã có:

```rust
let n = match s.parse() {
    Ok(v) => v,
    Err(e) => return Err(From::from(e)),
};
```

`?` là **đường tắt** cho pattern này. Quan trọng nhất: nó **gọi `From::from(e)`** để convert error.

## 4.3 `?` + `From` = Magic chuyển đổi

```rust
#[derive(Debug)]
enum AppError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self { AppError::Io(e) }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self { AppError::Parse(e) }
}

fn read_and_parse() -> Result<i32, AppError> {
    let s = std::fs::read_to_string("x.txt")?;   // io::Error → AppError::Io
    let n: i32 = s.trim().parse()?;              // ParseIntError → AppError::Parse
    Ok(n)
}
```

`?` tự gọi `From::from` để convert từng error sang `AppError`. Không cần `map_err` thủ công.

Đây là **lý do tại sao thiserror tồn tại** — generate `From` impl tự động.

## 4.4 `?` chỉ chạy trong function trả về Result hoặc Option

```rust
fn returns_result() -> Result<(), Error> {
    let _ = some_op()?;   // ✅ OK
    Ok(())
}

fn returns_unit() {
    let _ = some_op()?;   // ❌ ERROR: ? requires Result/Option return type
}
```

Hệ quả: `main()` ban đầu không dùng `?` được. Nhưng từ Rust 1.26:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = some_op()?;   // ✅ OK
    Ok(())
}
```

Hoặc với `anyhow`:

```rust
fn main() -> anyhow::Result<()> {
    do_stuff()?;
    Ok(())
}
```

## 4.5 `?` với Option

```rust
fn first_word(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let space = bytes.iter().position(|&b| b == b' ')?;  // ? trên Option
    Some(&s[..space])
}
```

`?` trên `Option<T>`: nếu `None` → return `None`.

## 4.6 Mixing Result và Option

```rust
fn parse_first_word(s: &str) -> Result<i32, MyError> {
    let word = first_word(s).ok_or(MyError::NoWord)?;   // Option → Result
    let n: i32 = word.parse()?;
    Ok(n)
}
```

Khi mix, convert explicit: `Option::ok_or` / `Option::ok_or_else`.

## 4.7 `try_trait_v2` — Tương lai của `?`

Đang nightly: trait `Try` cho phép **custom types** dùng `?`. Mở khả năng `?` với:
- `ControlFlow<B, C>`
- Custom Result types
- Domain-specific monads

Stable code chỉ cần biết Result + Option.

## 4.8 Performance của `?`

`?` là zero-cost — compile thành cùng instructions như match thủ công. Không có hidden alloc, không stack unwinding. Đây là 1 trong những idiom đẹp nhất của Rust: gọn, an toàn, nhanh.

---

# Tầng 5: std::error::Error trait — Hệ sinh thái error

## 5.1 Định nghĩa Error trait

```rust
pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> { None }

    // Old API (deprecated):
    fn description(&self) -> &str { "" }
    fn cause(&self) -> Option<&dyn Error> { self.source() }

    // Nightly:
    fn provide<'a>(&'a self, _request: &mut Request<'a>) { }
}
```

3 yêu cầu chính:
1. **`Debug`** — `{:?}` format (cho dev)
2. **`Display`** — `{}` format (cho user)
3. **`source()`** — error gốc gây ra error này (error chain)

## 5.2 Tự implement Error manually

```rust
use std::fmt;

#[derive(Debug)]
struct ConfigError {
    field: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid config field: {}", self.field)
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|e| e as &(dyn std::error::Error))
    }
}
```

Quá dài và lặp! Đây là lý do `thiserror` tồn tại.

## 5.3 Error chain — source() là gì?

Khi error A "wrap" error B:

```rust
// User calls our_app::handle_request
//   -> our_app::read_config (fails)
//     -> std::io::Error (file not found)

ConfigError {
  field: "database_url",
  source: Some(IoError { ... }),
}

// source chain:
//   ConfigError → IoError → None
```

Khi log error, **không chỉ log error trên cùng** — duyệt full chain:

```rust
fn print_error(err: &dyn std::error::Error) {
    eprintln!("Error: {}", err);
    let mut source = err.source();
    while let Some(e) = source {
        eprintln!("  caused by: {}", e);
        source = e.source();
    }
}
```

Output:
```
Error: invalid config field: database_url
  caused by: failed to read /etc/app/config.toml
  caused by: No such file or directory (os error 2)
```

`anyhow` và `eyre` tự làm điều này khi `Display` error.

## 5.4 `Box<dyn Error>` — Type-erased error

```rust
fn read_anything() -> Result<String, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string("x")?;   // io::Error → Box<dyn Error>
    let _: i32 = s.parse()?;                  // ParseIntError → Box<dyn Error>
    Ok(s)
}
```

`Box<dyn Error>` là **type-erased error container** — chứa bất kỳ type nào impl `Error`. `?` tự dùng `From::from` để box.

**Trade-off**:
- ✅ Đơn giản, không cần define error type riêng
- ❌ Mất type info, caller không match được loại lỗi
- ❌ Heap alloc cho mỗi error

Dùng cho **prototype** và **binary main()**, không cho library API.

### Send + Sync version

```rust
Box<dyn Error + Send + Sync + 'static>
```

Cần Send + Sync nếu error đi qua thread / async task. `anyhow::Error` thực ra wrap chính cái này + thêm context.

## 5.5 Downcast — Lấy lại type cụ thể

```rust
let err: Box<dyn Error> = some_function().unwrap_err();

if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
    println!("It's IO error: {}", io_err);
}
```

`downcast_ref::<T>()` thử match type. Hữu ích khi muốn handle riêng loại lỗi cụ thể.

---

# Tầng 6: thiserror — Library error tốt nhất

## 6.1 Tại sao thiserror?

`std::error::Error` implementation thủ công cực kỳ verbose. `thiserror` là **derive macro** sinh code này cho bạn — zero runtime cost.

Cài: `cargo add thiserror`

```toml
[dependencies]
thiserror = "1.0"
```

## 6.2 Ví dụ cơ bản

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config file not found at {0}")]
    NotFound(String),

    #[error("invalid config syntax: {0}")]
    InvalidSyntax(String),

    #[error("missing required field: {field}")]
    MissingField { field: String },

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("parse error")]
    Parse(#[from] toml::de::Error),
}
```

`thiserror` sinh:
- `impl Display` từ `#[error("...")]`
- `impl std::error::Error` (kể cả `source()`)
- `impl From<std::io::Error>` từ `#[from]`
- `impl From<toml::de::Error>` từ `#[from]`

**Bây giờ chỉ cần dùng `?`**:

```rust
fn read_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;       // io::Error → ConfigError::Io
    let cfg: Config = toml::from_str(&content)?;        // toml::Error → ConfigError::Parse
    Ok(cfg)
}
```

## 6.3 Attribute reference

### `#[error("...")]` — Display message

```rust
#[error("simple message")]
NotFound,

#[error("not found: {0}")]          // positional field
NotFound(String),

#[error("invalid {field}: {value}")] // named fields
Invalid { field: String, value: String },

#[error("status code {0}", self.0)]  // self.0 explicit (rare)
StatusError(u16),
```

Format string syntax giống `format!`.

### `#[from]` — Auto From conversion

```rust
#[error("DB error")]
Database(#[from] sqlx::Error),  // sinh impl From<sqlx::Error> for MyError
```

Mỗi variant chỉ được có 1 `#[from]`. Variant phải có đúng 1 field.

`#[from]` cũng tự gán `#[source]` — error chain hoạt động.

### `#[source]` — Mark inner error nhưng không auto From

```rust
#[error("failed to query {table}")]
QueryFailed {
    table: String,
    #[source]
    source: sqlx::Error,
},
```

Khi error có thêm context (field `table` ở đây), không thể dùng `#[from]` (vì cần truyền `table` manually). `#[source]` chỉ đánh dấu để `error.source()` trả về cái này.

### `#[error(transparent)]` — Pass-through

```rust
#[error(transparent)]
Other(#[from] anyhow::Error),
```

`transparent`: Display sẽ delegate cho inner error. Hữu ích khi wrap một error type khác mà không muốn add prefix.

### `#[backtrace]` — Bắt backtrace (nightly)

```rust
#[error("...")]
MyVariant {
    source: Box<dyn Error>,
    backtrace: Backtrace,
},
```

## 6.4 Pattern: Module-level error type

```rust
// Trong crate database module
pub mod database {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("connection failed")]
        Connection(#[from] tokio_postgres::Error),

        #[error("transaction rolled back")]
        TransactionRollback,

        #[error("constraint violated: {0}")]
        Constraint(String),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}

// Trong crate auth module
pub mod auth {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("invalid credentials")]
        InvalidCredentials,

        #[error("user not found")]
        UserNotFound,

        #[error(transparent)]
        Database(#[from] crate::database::Error),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
```

**Mỗi module có error type riêng**. Module ngoài (auth) wrap module trong (database) qua `#[from]`. Đây là pattern senior — error type follows module boundaries.

## 6.5 Sai lầm thường gặp với thiserror

### ❌ Lỗi 1: Một error type khổng lồ cho cả crate

```rust
#[derive(Error)]
pub enum AppError {
    DatabaseConnection(...),
    DatabaseQuery(...),
    HttpClient(...),
    HttpServer(...),
    ConfigParse(...),
    Auth(...),
    // ... 30+ variants
}
```

Khi 1 module chỉ fail vì 2 loại, vẫn return type chứa 30 variants → caller phải match 30 cái. Code dễ vỡ khi thêm variant.

**Đúng**: error type theo module/feature boundary.

### ❌ Lỗi 2: Quá nhiều variants chi tiết

```rust
#[derive(Error)]
pub enum SqlError {
    UniqueConstraintOnEmail,
    UniqueConstraintOnUsername,
    UniqueConstraintOnPhone,
    // ... 20 variants cho mỗi constraint
}
```

Mỗi lần thêm column, thêm variant → API change. Tốt hơn:

```rust
#[derive(Error)]
pub enum SqlError {
    #[error("constraint violated: {0}")]
    UniqueViolation(String),  // truyền tên constraint qua String
}
```

Quy tắc: variant phân biệt khi **caller xử lý khác nhau**. Nếu caller chỉ "log + retry", gộp lại.

### ❌ Lỗi 3: Dùng String trong tất cả variants

```rust
#[derive(Error)]
pub enum Error {
    #[error("{0}")]
    Generic(String),  // ← thực ra là `anyhow` trá hình
}
```

Nếu cuối cùng chỉ là String, dùng `anyhow` luôn.

## 6.6 Hiệu năng

`thiserror` là **pure compile-time** macro. Zero runtime cost. Error enum của bạn:
- Cỡ = `max(variant_size) + discriminant tag`
- Match là branch table — rất nhanh
- Không alloc trừ khi inner error có alloc (vd String trong variant)

So với exceptions Java: Rust error throughput cao hơn 10-100x trên error path.

---

# Tầng 7: anyhow — Application error tốt nhất

## 7.1 Tại sao anyhow?

Trong **application code** (binary), thường:
- Không cần caller phân biệt loại lỗi (chỉ log + exit / return 500)
- Cần dễ-dàng add **context** ("failed to load config", "while processing user X")
- Cần collect mọi error type vào 1 thứ

`anyhow::Error` = `Box<dyn Error + Send + Sync + 'static>` + context + backtrace.

Cài:
```toml
[dependencies]
anyhow = "1.0"
```

## 7.2 Ví dụ cơ bản

```rust
use anyhow::{Context, Result};

fn read_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config from {}", path))?;
    let cfg: Config = toml::from_str(&content)
        .context("failed to parse config TOML")?;
    Ok(cfg)
}

fn main() -> Result<()> {
    let cfg = read_config("config.toml")?;
    Ok(())
}
```

`anyhow::Result<T>` = `Result<T, anyhow::Error>`.

Khi error xảy ra:
```
Error: failed to parse config TOML
Caused by:
    0: invalid TOML syntax at line 42
    1: expected `=`, found `:`
```

Đẹp ngay từ đầu, không cần code logging custom.

## 7.3 .context() và .with_context()

```rust
let user = db.find_user(&id)
    .context("failed to find user")?;

let user = db.find_user(&id)
    .with_context(|| format!("failed to find user {}", id))?;
```

- `.context("static str")` — message tĩnh, rẻ
- `.with_context(|| ...)` — message tính lazy, chỉ chạy nếu Err

**Rule of thumb**: với_context dùng cho dynamic message (chứa biến).

## 7.4 anyhow! macro để tạo error trên đường

```rust
use anyhow::{anyhow, bail};

fn validate(n: i32) -> Result<()> {
    if n < 0 {
        return Err(anyhow!("invalid value: {}", n));
    }
    // hoặc shortcut:
    if n > 1000 {
        bail!("too large: {}", n);
    }
    Ok(())
}
```

- `anyhow!(...)` — tạo `anyhow::Error`
- `bail!(...)` — `return Err(anyhow!(...))`

## 7.5 ensure! — Như assert! nhưng trả Err

```rust
use anyhow::ensure;

fn validate(n: i32) -> Result<()> {
    ensure!(n >= 0, "n must be non-negative, got {}", n);
    ensure!(n <= 1000, "n must be ≤ 1000, got {}", n);
    Ok(())
}
```

## 7.6 Downcasting

```rust
let err: anyhow::Error = some_function().unwrap_err();

if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
    // handle riêng IO error
}

// Hoặc consume err:
match err.downcast::<std::io::Error>() {
    Ok(io_err) => { /* xử lý io_err */ },
    Err(other) => { /* không phải io::Error, là `other: anyhow::Error` */ },
}
```

## 7.7 Khác biệt thiserror vs anyhow

| Aspect | thiserror | anyhow |
|--------|-----------|--------|
| Error type | Specific enum bạn define | `anyhow::Error` (boxed) |
| Caller có thể match? | ✅ Có | ⚠️ Phải downcast |
| Compile-time check | Strict | Loose |
| Code ngắn? | Phải define enum | Plug-and-play |
| Context | Phải thêm variant | `.context()` |
| Use case | **Library** | **Application** |

**Quy tắc senior**:
- Library (crate xuất bản) → `thiserror`
- Application (binary cuối) → `anyhow`
- Internal modules trong app: tùy — nhiều người dùng `thiserror` cho modules, `anyhow` ở `main()` để collect

## 7.8 eyre — Họ hàng của anyhow

`eyre` = fork của `anyhow` cho phép custom report style (vd: color-eyre cho terminal đẹp).

```rust
use color_eyre::{eyre::Result, eyre::WrapErr};

fn main() -> Result<()> {
    color_eyre::install()?;
    do_stuff().wrap_err("Failed to run app")?;
    Ok(())
}
```

Khác `anyhow` ở error report format — terminal có màu, backtrace gọn.

---

# Tầng 8: Library vs Application — Chiến lược chọn lựa

## 8.1 Phân biệt: Library code vs Application code

### Library code
- Tái sử dụng được, publish lên crates.io
- API là contract — error type là **phần** của API
- Caller cần match được error để recover khác nhau
- Stability quan trọng — đổi error type = breaking change

### Application code
- Top-level binary
- Caller cuối cùng là `main()` hoặc `tokio::main`
- Thường log + exit / return 500 — không match chi tiết
- Tốc độ phát triển quan trọng hơn type strictness

## 8.2 Quy tắc của senior

```
┌──────────────────────────────────────────────────────────┐
│  Library code   → thiserror — strict, typed              │
│  Binary main    → anyhow — ergonomic, flexible           │
│  Internal mods  → thiserror cho per-module               │
│  Test code      → unwrap/expect là OK                    │
└──────────────────────────────────────────────────────────┘
```

## 8.3 Ví dụ: Cấu trúc dự án realistic

```
myapp/
├── Cargo.toml          // workspace
├── crates/
│   ├── myapp-core/       // library — thiserror
│   │   └── src/error.rs
│   ├── myapp-db/         // library — thiserror
│   │   └── src/error.rs
│   ├── myapp-auth/       // library — thiserror
│   │   └── src/error.rs
│   └── myapp-server/     // binary — anyhow
│       └── src/main.rs
```

### myapp-core/src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    
    #[error("not found")]
    NotFound,
}
```

### myapp-db/src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("connection lost")]
    Connection(#[from] tokio_postgres::Error),
    
    #[error("constraint violated: {0}")]
    Constraint(String),
}
```

### myapp-server/src/main.rs

```rust
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = load_config().context("loading config")?;
    let db = myapp_db::connect(&cfg.db_url).await.context("connecting DB")?;
    
    serve(db, cfg).await.context("HTTP server failed")?;
    Ok(())
}
```

Mỗi crate có error riêng (typed). `main()` collect mọi error qua `anyhow` + context. Đây là cấu trúc của hầu hết Rust production codebases lớn.

## 8.4 Khi nào "lib internal" cũng dùng anyhow?

Khi:
- Crate dùng nội bộ trong workspace, không public
- Module có error path quá nhiều biến thể, không value trong việc match
- Refactoring liên tục, error variants chưa stable

Sau khi stable, có thể migrate lên `thiserror` để strict hơn.

---

# Tầng 9: Error context, source chain, backtrace

## 9.1 Context — Thông tin thêm về error

Error gốc thường thiếu context. So sánh:

❌ Không context:
```
Error: file not found
```

✅ Có context:
```
Error: failed to load user profile
Caused by:
    0: failed to read user data file
    1: file not found: /var/users/abc123.json
```

Context = "đường đi của error qua code", giúp debug nhanh hơn rất nhiều.

## 9.2 Cách thêm context

### Với anyhow

```rust
.context("static message")
.with_context(|| format!("dynamic: {}", var))
```

### Với thiserror — phải đưa vào variant

```rust
#[derive(Error, Debug)]
pub enum DbError {
    #[error("query failed on table {table}")]
    Query {
        table: String,
        #[source]
        source: sqlx::Error,
    },
}

// Khi propagate:
let result = sqlx::query("...")
    .execute(&pool)
    .await
    .map_err(|e| DbError::Query {
        table: "users".into(),
        source: e,
    })?;
```

Verbose hơn anyhow nhưng type-safe — caller biết chính xác có thông tin gì.

## 9.3 Source chain navigation

```rust
fn log_full_error(err: &dyn std::error::Error) {
    eprintln!("Error: {}", err);
    let mut current = err.source();
    while let Some(e) = current {
        eprintln!("  Caused by: {}", e);
        current = e.source();
    }
}
```

Với `anyhow::Error`:

```rust
let err: anyhow::Error = ...;
for cause in err.chain() {
    eprintln!("- {}", cause);
}
```

## 9.4 Backtrace — Stack trace lúc error xảy ra

### std::backtrace::Backtrace (stable 1.65+)

```rust
use std::backtrace::Backtrace;

#[derive(Debug)]
struct MyError {
    message: String,
    backtrace: Backtrace,
}

impl MyError {
    fn new(msg: &str) -> Self {
        Self {
            message: msg.into(),
            backtrace: Backtrace::capture(),
        }
    }
}
```

`Backtrace::capture()` bắt stack frames hiện tại. Theo mặc định **chỉ capture nếu `RUST_BACKTRACE=1`** — vì cost cao.

### anyhow tự bắt backtrace

```rust
let err = anyhow!("oops");
// Hiển thị backtrace nếu RUST_BACKTRACE=1:
eprintln!("{:?}", err);
```

### Display vs Debug format

```rust
let err: anyhow::Error = ...;

println!("{}", err);    // Display: chỉ message
println!("{:?}", err);  // Debug: message + source chain + backtrace
```

Production logging thường dùng `{:?}` để có full info.

## 9.5 Tracing và error logging

Crate `tracing` là chuẩn de-facto cho structured logging:

```rust
use tracing::{error, instrument};

#[instrument(err)]
async fn process(user_id: u64) -> Result<()> {
    let user = fetch_user(user_id).await?;
    save_user(user).await?;
    Ok(())
}
```

`#[instrument(err)]` tự log error khi function trả Err — kèm function arguments + source chain.

Combined với `tracing-error` crate:

```rust
use tracing_error::SpanTrace;

#[derive(Debug)]
struct AppError {
    message: String,
    span_trace: SpanTrace,  // bắt span context lúc error
}
```

Span trace > backtrace cho async code (vì stack trace async ngắn, không meaningful).

---

# Tầng 10: Patterns nâng cao của senior

## 10.1 Pattern: Error builder

Khi error có nhiều field optional:

```rust
#[derive(Debug)]
struct ValidationError {
    field: String,
    code: ErrorCode,
    message: String,
    hint: Option<String>,
    docs_url: Option<String>,
}

impl ValidationError {
    fn new(field: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            field: field.into(),
            code,
            message: String::new(),
            hint: None,
            docs_url: None,
        }
    }
    fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into(); self
    }
    fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into()); self
    }
}

// Sử dụng:
let err = ValidationError::new("email", ErrorCode::InvalidFormat)
    .message("not a valid email address")
    .hint("include @ and domain");
```

## 10.2 Pattern: Error code cho API

Khi API public (HTTP/gRPC), client muốn match error programmatically — dùng error code.

```rust
#[derive(Debug, Serialize)]
pub struct ApiError {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl ApiError {
    pub const NOT_FOUND: &'static str = "NOT_FOUND";
    pub const UNAUTHORIZED: &'static str = "UNAUTHORIZED";
    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";
    // ...

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { code: Self::NOT_FOUND, message: msg.into(), details: None }
    }
}
```

Client (frontend, mobile) check `error.code == "RATE_LIMITED"` — independent of message format.

## 10.3 Pattern: Retry với exponential backoff

```rust
async fn fetch_with_retry<F, Fut, T, E>(
    mut f: F,
    max_attempts: u32,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut attempt = 0;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 < max_attempts => {
                let delay = Duration::from_millis(100 << attempt);
                tracing::warn!(?e, attempt, "retry");
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

// Usage:
let user = fetch_with_retry(
    || async { client.get_user(id).await },
    3,
).await?;
```

Production lib: `backoff`, `tokio-retry`, `retry`.

## 10.4 Pattern: Distinguish transient vs permanent errors

```rust
trait IsRetriable {
    fn is_retriable(&self) -> bool;
}

impl IsRetriable for reqwest::Error {
    fn is_retriable(&self) -> bool {
        self.is_timeout()
            || self.is_connect()
            || matches!(self.status(),
                Some(s) if s.is_server_error() || s == 429)
    }
}
```

Retry chỉ với transient (network, 5xx). Permanent (4xx, validation) — return ngay.

## 10.5 Pattern: Result trong iterator

```rust
// Collect Result<Vec<T>, E>
let nums: Result<Vec<i32>, _> = strings.iter()
    .map(|s| s.parse())
    .collect();

// Process Ok, log Err
strings.iter()
    .map(|s| s.parse::<i32>())
    .filter_map(|r| match r {
        Ok(v) => Some(v),
        Err(e) => { tracing::warn!(?e, "skip"); None }
    })
    .for_each(|n| println!("{}", n));

// Bail on first error with try_fold
let sum: Result<i32, _> = strings.iter()
    .map(|s| s.parse::<i32>())
    .try_fold(0, |acc, r| Ok(acc + r?));
```

## 10.6 Pattern: Error response cho HTTP server

Với `axum`:

```rust
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use axum::Json;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Internal(e) => {
                tracing::error!("internal error: {:?}", e);  // log full
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL")
            }
        };
        
        let body = Json(json!({
            "error": { "code": code, "message": self.to_string() }
        }));
        (status, body).into_response()
    }
}

// Handlers giờ chỉ cần return Result<Json<T>, AppError>:
async fn get_user(Path(id): Path<u64>) -> Result<Json<User>, AppError> {
    let user = db.find_user(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(user))
}
```

**Quan trọng**: log internal errors full (Debug), gửi client message ngắn — không leak internals (path, SQL query) ra response.

## 10.7 Pattern: Recoverable vs fatal

```rust
#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("transient: {0}")]
    Transient(String),
    
    #[error("permanent: {0}")]
    Permanent(String),
    
    #[error("fatal — must shutdown: {0}")]
    Fatal(String),
}

async fn worker_loop(mut queue: Queue) {
    while let Some(msg) = queue.recv().await {
        match process(msg).await {
            Ok(()) => continue,
            Err(WorkerError::Transient(_)) => {
                queue.requeue(msg).await;
            }
            Err(WorkerError::Permanent(e)) => {
                tracing::error!(?e, "dead-letter");
                queue.dead_letter(msg).await;
            }
            Err(WorkerError::Fatal(e)) => {
                tracing::error!(?e, "fatal, shutting down");
                break;  // exit loop
            }
        }
    }
}
```

Trong production worker, phân loại error → behavior khác nhau (retry, DLQ, shutdown). Đây là pattern siêu phổ biến.

## 10.8 Pattern: Never type (`!`) cho infallible

```rust
fn always_succeeds() -> Result<i32, std::convert::Infallible> {
    Ok(42)
}
```

`Infallible` = "không bao giờ Err". `?` operator vẫn work. Hữu ích cho generic code.

Trong tương lai, `!` (never type) stable sẽ thay thế. 

## 10.9 Pattern: Validation aggregation

Khi validate form, không nên dừng ở lỗi đầu — collect tất cả:

```rust
#[derive(Error, Debug)]
#[error("validation failed: {0} errors", errors.len())]
pub struct ValidationErrors {
    pub errors: Vec<FieldError>,
}

fn validate_user(input: &UserInput) -> Result<(), ValidationErrors> {
    let mut errors = vec![];
    
    if input.email.is_empty() {
        errors.push(FieldError::new("email", "required"));
    } else if !input.email.contains('@') {
        errors.push(FieldError::new("email", "invalid format"));
    }
    
    if input.age < 18 {
        errors.push(FieldError::new("age", "must be ≥ 18"));
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors { errors })
    }
}
```

Crate `validator` làm điều này automatically với derive macros.

## 10.10 Pattern: Custom Debug để hide sensitive data

```rust
struct DatabaseConfig {
    url: String,
    password: String,  // ← không muốn log
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &self.url)
            .field("password", &"<redacted>")
            .finish()
    }
}
```

Khi error log chứa config, password không leak. Senior thinking: **error path là attack surface** — đừng leak secrets.

---

# Tầng 11: Async error — Những điểm đặc biệt

## 11.1 Error trong async fn — Cần Send + Sync khi spawn

```rust
async fn worker() -> Result<(), MyError> {
    // ...
}

tokio::spawn(async {
    if let Err(e) = worker().await {
        tracing::error!("worker failed: {:?}", e);
    }
});
```

Nếu spawn task, error type phải `Send + 'static`. Hầu hết error qua `thiserror` đều OK vì các field cơ bản Send.

Lỗi điển hình:
```rust
#[derive(Error, Debug)]
pub enum E {
    #[error("error: {0}")]
    Inner(#[from] Rc<dyn std::error::Error>),  // Rc !Send
}
// Future không Send
```

Fix: dùng `Arc` thay `Rc`, hoặc `Box<dyn Error + Send + Sync>`.

## 11.2 JoinError — Task panicked or cancelled

```rust
let handle = tokio::spawn(async { panic!("oops") });
let result = handle.await;

match result {
    Ok(v) => println!("got: {:?}", v),
    Err(join_err) if join_err.is_panic() => {
        eprintln!("task panicked");
    }
    Err(join_err) if join_err.is_cancelled() => {
        eprintln!("task cancelled");
    }
    Err(_) => unreachable!(),
}
```

`JoinError` không phải lỗi từ business logic — là lỗi runtime của task.

## 11.3 Error propagation qua select!

```rust
tokio::select! {
    result = future_a => result?,
    result = future_b => result?,
    _ = tokio::time::sleep(timeout) => {
        return Err(anyhow!("timeout"));
    }
}
```

Mỗi branch của select có thể trả Result, propagate qua `?` bình thường.

## 11.4 try_join! và FuturesUnordered

```rust
// Bail on first error
let (a, b, c) = tokio::try_join!(
    fetch_a(),
    fetch_b(),
    fetch_c(),
)?;
```

vs `join_all` từ `futures`:

```rust
use futures::future::join_all;

let results: Vec<Result<_, _>> = join_all(futures).await;
// All complete, errors are in results — không bail early
```

Trade-off: bail early vs collect all.

## 11.5 Drop và error trong async

`Drop` chạy đồng bộ → không thể `.await` trong Drop. Nếu cleanup cần I/O async:

```rust
struct AsyncResource;

impl AsyncResource {
    async fn close(self) -> Result<()> {
        // async cleanup
        Ok(())
    }
}

impl Drop for AsyncResource {
    fn drop(&mut self) {
        eprintln!("Warning: AsyncResource dropped without close()");
        // không thể await cleanup
    }
}
```

Pattern: explicit `close()` method, drop chỉ log warning. Đây là **async drop problem** — đang được thảo luận trong Rust async working group.

---

# Tầng 12: Antipatterns — Sai lầm phổ biến

## 12.1 ❌ unwrap() khắp nơi

```rust
let val = some_function().unwrap();
```

Trong production = bug đang chờ. Mỗi `unwrap()` là 1 panic potential. Senior tự hỏi: "có thực sự impossible cho Err không?"

**Fix**: dùng `?`, `expect()` với message rõ, hoặc handle.

## 12.2 ❌ Swallowing errors

```rust
let _ = risky_operation();  // ← phớt lờ Result
if let Ok(v) = something() { ... }  // ← bỏ Err lặng lẽ
```

Logic vẫn tiếp tục với state có thể sai. Ít nhất nên log.

**Fix**:
```rust
if let Err(e) = risky_operation() {
    tracing::warn!(?e, "operation failed, continuing anyway");
}
```

## 12.3 ❌ Stringly-typed errors

```rust
fn foo() -> Result<i32, String> { ... }
fn bar() -> Result<(), String> { ... }
```

String error mất type info, không thể match programmatically. Khó wrap, khó test.

**Fix**: define typed error với `thiserror`, hoặc dùng `anyhow::Error` (đã wrap typed errors).

## 12.4 ❌ println!/eprintln! làm logging

```rust
fn process() -> Result<()> {
    if let Err(e) = do_stuff() {
        eprintln!("Error: {}", e);  // ← chỉ Display, mất source chain
        return Err(e);
    }
    Ok(())
}
```

Mất:
- Source chain
- Backtrace
- Structured fields
- Span context (async)

**Fix**: `tracing::error!(?error, "context")` — `?` debug format giữ full info.

## 12.5 ❌ Quá nhiều .map_err() chains

```rust
let v = some_call()
    .map_err(|e| MyError::Io(e))?;
let n = parse(v)
    .map_err(|e| MyError::Parse(e))?;
let r = lookup(n)
    .map_err(|e| MyError::Lookup(e))?;
```

Code noise. Mỗi `map_err` thực ra `thiserror` đã làm tự động với `#[from]`.

**Fix**:
```rust
#[derive(Error)]
pub enum MyError {
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] Parse(#[from] ParseError),
    #[error(transparent)] Lookup(#[from] LookupError),
}

let v = some_call()?;
let n = parse(v)?;
let r = lookup(n)?;
```

## 12.6 ❌ Panic trong library API

Library publish lên crates.io, panic = "your problem":
```rust
pub fn parse(input: &str) -> Config {
    let cfg = toml::from_str(input).unwrap();  // ← user nào input sai sẽ crash app của họ
    cfg
}
```

**Fix**: trả `Result`.

## 12.7 ❌ Quên log error trước khi return 500 (HTTP)

```rust
async fn handler() -> Result<Response, AppError> {
    let data = expensive_db_call().await?;  // ← nếu fail, response 500
    // không có log → debug ko biết tại sao
}
```

**Fix**: log trong `IntoResponse` impl (như pattern Tầng 10.6).

## 12.8 ❌ Sử dụng Error type giống cho mọi module

Đã đề cập Tầng 6.5 — error type khổng lồ cho cả crate là antipattern.

## 12.9 ❌ Box<dyn Error> trong library public API

```rust
pub fn library_function() -> Result<T, Box<dyn Error>> { ... }
```

Caller không match được cụ thể. Public API phải có error type rõ.

**Fix**: `pub enum LibraryError` với thiserror.

## 12.10 ❌ Discard backtrace với .to_string()

```rust
let err_str = my_error.to_string();  // ← chỉ Display, mất backtrace/source
log(err_str);
```

**Fix**: log error object trực tiếp:
```rust
log(format!("{:?}", my_error));  // Debug giữ full info
// hoặc tracing::error!(?my_error);
```

---

# Tổng kết — 12 nguyên tắc của senior

```
┌────────────────────────────────────────────────────────────────┐
│ 1. Errors là values, không phải exception.                     │
│                                                                │
│ 2. Result<T,E> cho recoverable, panic! cho bugs.               │
│                                                                │
│ 3. Library → thiserror, Application → anyhow.                  │
│                                                                │
│ 4. ? + From là magic, đừng dùng map_err thủ công.              │
│                                                                │
│ 5. Add context tại mỗi level — error gốc thường không đủ.      │
│                                                                │
│ 6. Source chain + backtrace là vũ khí debug — đừng vứt đi.     │
│                                                                │
│ 7. Error type theo module/feature boundary, không phải crate.  │
│                                                                │
│ 8. Phân loại: transient/permanent/fatal — xử lý khác nhau.     │
│                                                                │
│ 9. Log error đầy đủ (Debug format) — không leak secrets.       │
│                                                                │
│ 10. Validate aggregate — collect all errors, không bail early. │
│                                                                │
│ 11. unwrap() trong production = code smell. expect() còn đỡ.   │
│                                                                │
│ 12. Test happy path AND error path. Error path là API thật.    │
└────────────────────────────────────────────────────────────────┘
```

---

# Liên kết về memory model

Error là **value type bình thường** trong Rust — không có magic. Nhìn theo memory:

| Error pattern | Memory layout |
|---------------|---------------|
| `Result<T,E>` | Enum: tag (1-8 byte) + max(T,E) size. Niche optimization với Option<&T> |
| `thiserror enum` | Enum giống trên — size = max(variant) + tag |
| `Box<dyn Error>` | Fat pointer (16 byte): data ptr + vtable ptr. Inner type alloc trên heap |
| `anyhow::Error` | `Box<dyn Error+Send+Sync>` + thin context wrapper |
| `?` operator | Compile to match + return — zero-cost |
| `From::from(e)` | Method call, có thể inline; cost = constructor cost của error variant |

→ Error handling Rust **không có hidden runtime cost** (so với exception unwinding).

Cost duy nhất: heap alloc khi dùng `Box<dyn Error>` / `anyhow::Error` cho mỗi error instance. Trên happy path (Ok), zero cost.

---

# Crates cần biết (senior toolkit)

| Crate | Mục đích | Khi dùng |
|-------|----------|----------|
| `thiserror` | Derive Error trait | Library error types |
| `anyhow` | Dynamic boxed error | Application main + binaries |
| `eyre` / `color-eyre` | Như anyhow, terminal đẹp hơn | CLI tools |
| `tracing` | Structured logging | Mọi production code |
| `tracing-error` | SpanTrace cho async | Khi backtrace không meaningful |
| `validator` | Aggregate validation | Form/input validation |
| `backoff` / `tokio-retry` | Retry với backoff | Network calls |
| `snafu` | Alternative thiserror | Khi cần fine-grained source location |

---

# Lộ trình tiếp theo

Bạn đã đầy đủ kỹ năng error handling production. Các chủ đề tiếp theo có thể đi sâu:

- **Macros** — `macro_rules!`, procedural macros (derive như `thiserror` thực ra là proc-macro)
- **Unsafe Rust** — raw pointer, FFI, atomic ordering
- **Testing patterns** — unit test, integration, property-based (proptest), criterion bench
- **Logging & Observability** — tracing, OpenTelemetry, metrics
- **Web frameworks** — axum, actix-web (apply mọi thứ đã học)

Báo chủ đề tiếp theo bạn muốn đào sâu! 🦀
