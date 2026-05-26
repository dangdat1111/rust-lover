# Error Handling Rust — Minh Hoạ Trực Quan

> Companion visual cho [error-handling.md](./error-handling.md).
> Dùng để học mắt → não nhanh hơn đọc text.

---

## 1. Bức tranh lớn — Error Handling Universe

```
                       ERROR HANDLING TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   ┌────────────┐         ┌──────────────┐              │
       │   │  panic!    │         │ Result<T,E>  │              │
       │   │ (bugs)     │         │ (recoverable)│              │
       │   └─────┬──────┘         └──────┬───────┘              │
       │         │                       │                      │
       │         │                  ┌────┴─────┐                │
       │         │                  ▼          ▼                │
       │         │             thiserror    anyhow              │
       │         │             (library)    (application)       │
       │         │                  │          │                │
       │         │                  └────┬─────┘                │
       │         │                       │                      │
       │         │              ┌────────┴─────────┐            │
       │         │              ▼                  ▼            │
       │         │         ? operator       error chain         │
       │         │         (From convert)   (source chain)      │
       │         │              │                  │            │
       │         └──────────────┴──────────────────┘            │
       │                        │                                │
       │                        ▼                                │
       │              ┌──────────────────────┐                  │
       │              │   tracing + log      │                  │
       │              │   Backtrace          │                  │
       │              │   Observability      │                  │
       │              └──────────────────────┘                  │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Errors-as-Values vs Exceptions

```
   JAVA / C++ / PYTHON                        RUST
   ─────────────────────                      ────
   
   ┌──────────────────┐                       ┌─────────────────┐
   │ try {            │                       │ let r = op()?;  │
   │   risky_call();  │                       │ // r là T       │
   │ } catch (E e) {  │                       │ // ? = match    │
   │   handle(e);     │                       │ //   propagate  │
   │ }                │                       └─────────────────┘
   └──────────────────┘                              ↑
        ↓                                            │
   Exception flow là                            Result là VALUE
   INVISIBLE control flow                       trên signature
   
   ──────────────────────────                  ──────────────────────
   ❌ Không nhìn signature                      ✅ Signature rõ
      mà biết throws gì                            "may fail with E"
   ❌ Stack unwinding tốn                        ✅ Zero-cost match
   ❌ Quên catch → crash                         ✅ Compiler ép handle
   ❌ Java checked → "throws Exception" everywhere ✅ Compose dễ với ?
```

---

## 3. Cây quyết định — panic vs Result

```
                  CÓ XẢY RA LỖI Ở RUNTIME?
                          │
              ┌───────────┴───────────┐
             YES                      NO (chỉ bug logic)
              │                       │
        ┌─────┴─────┐                 ▼
        │           │            debug_assert!() / 
   CALLER CÓ      KHÔNG          unreachable!()
   recover được?  recover         (chỉ check ở debug)
        │           │
        ▼           ▼
     Result      panic!
        │
   ┌────┴────┐
   │         │
   │ ✅ User input sai
   │ ✅ Network timeout
   │ ✅ File not found
   │ ✅ Parse JSON external
   │ ✅ DB constraint violated
   
   ⚠️ panic! cho:
   ── Index out of bounds
   ── Divide by zero
   ── Invariant violation ("không thể xảy ra")
   ── API misuse (caller's bug)
   ── Tests (assert!)
```

---

## 4. Result<T,E> — Memory Layout

```
   pub enum Result<T, E> {
       Ok(T),
       Err(E),
   }

   Memory layout:
   ──────────────
   ┌──────────────────────────────────┐
   │ Discriminant tag (1-8 bytes)     │ ← 0 = Ok, 1 = Err
   ├──────────────────────────────────┤
   │ Variant data: max(T, E) bytes    │
   │                                  │ ← chỉ 1 trong 2 active
   │  Ok variant:  [   T   ]          │
   │  Err variant: [   E   ]          │
   └──────────────────────────────────┘

   sizeof(Result<i32, String>) = 32 byte (trên x64):
   ┌──────┬───────────────────────────┐
   │ tag  │ max(i32=4, String=24)     │
   │ 8B   │ = 24B                     │
   └──────┴───────────────────────────┘
   total = 32 byte (aligned to 8)


   Niche optimization (smart compiler):
   ────────────────────────────────────
   Option<&T> chỉ tốn sizeof::<&T>() = 8 byte
   vì &T không thể null → 0x0 dùng làm tag None.
   
   Result<&T, ()> cũng được optimize tương tự khi E nhỏ.
```

---

## 5. ? Operator — Desugar visualization

```
   Code bạn viết:                  Compiler expand:
   ─────────────                   ────────────────
   
   let n = s.parse()?;     ───►    let n = match s.parse() {
                                       Ok(v) => v,
                                       Err(e) => return Err(From::from(e)),
                                   };

   Quan trọng:    │             │
              ────┘             └────
        Early return            From::from(e) — CHUYỂN ĐỔI ERROR


   Flow visualization:
   ───────────────────

        ┌─────────────────────┐
        │  s.parse()          │
        └─────────┬───────────┘
                  │
            ┌─────┴─────┐
           Ok          Err
            │           │
            ▼           ▼
       gán cho n   From::from(e)
                       │
                       ▼
                  return Err(...)
                       │
                       ▼
                  caller nhận Err
```

---

## 6. From + ? = Magic chuyển đổi error

```
   #[derive(Error, Debug)]
   pub enum AppError {
       Io(#[from] std::io::Error),         ← thiserror sinh impl From
       Parse(#[from] ParseIntError),       ← cho từng inner type
   }
   
   ──────────────────────────────────────────────────────────────
   
   fn process() -> Result<i32, AppError> {
   
       let s = read_file()?;
                              │
                              │ read_file trả Result<_, io::Error>
                              │ ? gọi From::from(io::Error)
                              │ → AppError::Io(io::Error)
                              │ return Err(...)
                              ▼
                         
       let n: i32 = s.parse()?;
                              │
                              │ parse trả Result<_, ParseIntError>
                              │ ? gọi From::from(ParseIntError)
                              │ → AppError::Parse(...)
                              │ return Err(...)
                              ▼
       
       Ok(n)
   }
   
   ──────────────────────────────────────────────────────────────
   
   📌 KHÔNG cần .map_err() ở mỗi dòng — ? tự chuyển kiểu nhờ From.
```

---

## 7. Error Chain — Source linked list

```
   Error gốc nhất ──── wrap ──── wrap ──── wrap ──── error trên cùng
   (root cause)                                       (top error)
   
   Display khi log:
   ────────────────
   
   ┌─────────────────────────────────────────────────────┐
   │ Error: failed to load user profile                  │  ← top
   │ Caused by:                                          │
   │   0: failed to read user data file                  │  ← wrap 1
   │   1: failed to open /var/users/abc.json             │  ← wrap 2
   │   2: No such file or directory (os error 2)         │  ← root
   └─────────────────────────────────────────────────────┘
   
   
   Trong memory:
   ──────────────
   
   ┌────────────────────────────────────┐
   │ AppError::UserProfile {            │ ← top
   │   user_id: 123,                    │
   │   source: ─────┐                   │
   │ }              │                   │
   └────────────────┼───────────────────┘
                    │ .source()
                    ▼
   ┌────────────────────────────────────┐
   │ DataError::ReadFile {              │ ← wrap 1
   │   path: "abc.json",                │
   │   source: ─────┐                   │
   │ }              │                   │
   └────────────────┼───────────────────┘
                    │ .source()
                    ▼
   ┌────────────────────────────────────┐
   │ std::io::Error                     │ ← root
   │   kind: NotFound                   │
   │   message: "..."                   │
   │   source: None                     │
   └────────────────────────────────────┘
   
   Duyệt chain để in full:
   while let Some(e) = err.source() { println!(" - {}", e); }
```

---

## 8. thiserror — Derive macro flow

```
   Code bạn viết:                Compile-time expand thành:
   ─────────────                 ─────────────────────────
   
   #[derive(Error, Debug)]       impl std::fmt::Display for ConfigError {
   pub enum ConfigError {            fn fmt(&self, f) -> Result {
       #[error("not found: {0}")]        match self {
       NotFound(String),                     Self::NotFound(s) => write!(f,
                                                "not found: {}", s),
       #[error("IO error")]                  Self::Io(_) => write!(f,
       Io(#[from] io::Error),                   "IO error"),
   }                                     }
                                     }
                                 }
                                 
                                 impl std::error::Error for ConfigError {
                                     fn source(&self) -> Option<&...> {
                                         match self {
                                             Self::Io(e) => Some(e),
                                             _ => None,
                                         }
                                     }
                                 }
                                 
                                 impl From<io::Error> for ConfigError {
                                     fn from(e) -> Self { Self::Io(e) }
                                 }
   
   ──────────────────────────────────────────────────────────────
   
   Tất cả compile-time! Zero runtime cost.
```

---

## 9. thiserror attributes — Cheat sheet

```
   ┌──────────────────────────────────────────────────────────┐
   │ ATTRIBUTE          │ EFFECT                              │
   ├──────────────────────────────────────────────────────────┤
   │ #[error("msg")]    │ Tạo Display message                 │
   │                    │                                     │
   │ #[error("{0}")]    │ Format positional field             │
   │                    │                                     │
   │ #[error("{name}")] │ Format named field                  │
   │                    │                                     │
   │ #[from]            │ Auto-implement From<InnerType>      │
   │                    │ + tự gán source()                   │
   │                    │                                     │
   │ #[source]          │ Mark inner error, KHÔNG From        │
   │                    │ (dùng khi muốn add context fields)  │
   │                    │                                     │
   │ #[error(           │ Display delegate inner error        │
   │   transparent)]    │ (KHÔNG prefix)                      │
   │                    │                                     │
   │ #[backtrace]       │ Bắt backtrace (nightly)             │
   └──────────────────────────────────────────────────────────┘
   
   
   Ví dụ 3 patterns:
   ─────────────────
   
   #[error("file {path} corrupted")]      ← Display format
   Corrupted { path: String },
   
   #[error("IO failed")]                   ← #[from] tự sinh From
   Io(#[from] std::io::Error),
   
   #[error("query failed on {table}")]    ← #[source] + extra field
   QueryFailed {
       table: String,
       #[source]
       source: sqlx::Error,
   },
   
   #[error(transparent)]                   ← transparent
   Other(#[from] anyhow::Error),
```

---

## 10. anyhow::Error — Memory layout

```
   anyhow::Error
   ─────────────
   
   Wrapped: Box<dyn Error + Send + Sync + 'static> + context + backtrace
   
   ┌───────────────────────────────────────┐
   │ anyhow::Error (8 byte trên stack)     │
   │   ptr ───────────────────┐            │
   └──────────────────────────┼────────────┘
                              │
                              ▼ heap
   ┌────────────────────────────────────────┐
   │ ErrorImpl<E>                           │
   │   vtable                                │
   │   backtrace: Option<Backtrace>         │
   │   error: E                              │
   │   context_chain: ...                   │
   └────────────────────────────────────────┘

   So với Box<dyn Error> thuần:
   ────────────────────────────
   Box<dyn Error>:    16 byte (fat pointer: data + vtable)
   anyhow::Error:      8 byte (thin pointer, vtable lưu kèm trong impl)
   
   → anyhow nhỏ hơn, lưu trong Result rẻ hơn.
```

---

## 11. anyhow context flow

```
   fn load_user(id: u64) -> Result<User> {
       let path = format!("/users/{}.json", id);
       
       let content = std::fs::read_to_string(&path)
           .with_context(|| format!("read {}", path))?;
                              │
                              │ thêm context "read /users/123.json"
                              │ trước khi return Err
                              ▼
       
       let user: User = serde_json::from_str(&content)
           .context("parse user JSON")?;
                              │
                              │ thêm context "parse user JSON"
                              ▼
       Ok(user)
   }
   
   ─────────────────────────────────────────────
   
   Output khi error (chain build từ trong ra ngoài):
   
   ┌────────────────────────────────────────────┐
   │ Error: parse user JSON                     │ ← context 2
   │ Caused by:                                 │
   │   0: read /users/123.json                  │ ← context 1
   │   1: No such file or directory (os err 2)  │ ← root
   └────────────────────────────────────────────┘
```

---

## 12. thiserror vs anyhow — Bảng so sánh

```
   ┌─────────────────┬──────────────────────┬─────────────────────┐
   │                 │ thiserror            │ anyhow              │
   ├─────────────────┼──────────────────────┼─────────────────────┤
   │ Use case        │ Library              │ Application         │
   │                 │                      │                     │
   │ Error type      │ Specific enum        │ anyhow::Error       │
   │                 │ (bạn define)         │ (boxed dyn Error)   │
   │                 │                      │                     │
   │ Caller match?   │ ✅ Có thể            │ ⚠️ Phải downcast    │
   │                 │                      │                     │
   │ Type safety     │ Strong               │ Weak (dyn)          │
   │                 │                      │                     │
   │ Add context     │ Thêm variant         │ .context() / .with  │
   │                 │ với fields           │                     │
   │                 │                      │                     │
   │ Boilerplate     │ Define enum          │ ~Zero               │
   │                 │ (ít với derive)      │                     │
   │                 │                      │                     │
   │ Public API?     │ ✅ Tốt — stable      │ ❌ Không ideal      │
   │                 │                      │                     │
   │ Internal app?   │ OK, hơi verbose      │ ✅ Tốt — nhanh      │
   └─────────────────┴──────────────────────┴─────────────────────┘
   
   
   Quy tắc:
   ────────
   ┌──────────────────────────────────────────────────────┐
   │ Library publish lên crates.io  →  thiserror          │
   │ Binary main() / application    →  anyhow             │
   │ Workspace crates internal      →  thường thiserror   │
   │ Test code                      →  unwrap() OK        │
   └──────────────────────────────────────────────────────┘
```

---

## 13. Workspace structure — Senior pattern

```
   myapp/
   ├── Cargo.toml (workspace)
   ├── crates/
   │   │
   │   ├── myapp-core/        ← LIBRARY: thiserror
   │   │   └── src/error.rs   pub enum CoreError {...}
   │   │
   │   ├── myapp-db/          ← LIBRARY: thiserror
   │   │   └── src/error.rs   pub enum DbError {...}
   │   │
   │   ├── myapp-auth/        ← LIBRARY: thiserror
   │   │   └── src/error.rs   pub enum AuthError {
   │   │                          #[from] Db(DbError),  ← chain
   │   │                          ...
   │   │                      }
   │   │
   │   └── myapp-server/      ← BINARY: anyhow
   │       └── src/main.rs    use anyhow::{Result, Context};
   │                          
   │                          let cfg = load()
   │                              .context("loading config")?;
   │
   └── ...
   
   
   ┌──────────────────────────────────────────────────┐
   │  Mỗi module/crate có error type riêng (typed).   │
   │  main() collect mọi loại qua anyhow + context.   │
   │  Error TYPED không leak từ library lên main.     │
   └──────────────────────────────────────────────────┘
```

---

## 14. Source chain hoạt động trong từng level

```
   APP (myapp-server)
   ─────────────────
   anyhow::Error
       │
       │ .context("processing user 42")
       │
       ▼ wraps
   AUTH (myapp-auth)
   ─────────────────
   AuthError::Database
       │
       │ #[from] auto
       │
       ▼ wraps
   DB (myapp-db)
   ─────────────────
   DbError::Connection
       │
       │ #[from] auto
       │
       ▼ wraps
   STD
   ───
   tokio_postgres::Error
       │
       │
       ▼ wraps
   std::io::Error (root cause)
   
   
   Log từ main():
   ──────────────
   tracing::error!(error = ?err, "request failed");
   
   ⟹ Output:
   ┌────────────────────────────────────────────────────┐
   │ Error: processing user 42                          │
   │ Caused by:                                         │
   │   0: database error                                │
   │   1: connection error                              │
   │   2: db is unreachable: timed out                  │
   │   3: Connection refused (os error 111)             │
   └────────────────────────────────────────────────────┘
```

---

## 15. Recoverable vs Transient vs Fatal

```
                       ERROR PHÂN LOẠI
                              │
        ┌─────────────┬───────┴────────┬────────────┐
        ▼             ▼                ▼            ▼
    TRANSIENT     PERMANENT         FATAL       BUG
    (tạm thời)    (vĩnh viễn)       (chết)      (logic)
        │             │                │            │
   Retry?            Skip,         Shutdown      panic!
   Exponential       DLQ           gracefully    (assert)
   backoff           Log            cleanup       fix code
        │             │                │
        │             │                │
   Examples:     Examples:        Examples:
   • Network     • Validation     • Out of memory
     timeout       failed         • Disk full
   • DB connect  • Auth invalid   • Database wiped
     reset       • Resource       • Config corrupt
   • 5xx/429       not found      • Critical
                 • 4xx (most)       invariant
   
   
   Code pattern:
   ──────────────
   match process(msg).await {
       Ok(_) => continue,
       Err(e) if is_transient(&e) => {
           sleep_and_retry(&msg).await;
       }
       Err(e) if is_permanent(&e) => {
           dead_letter(&msg, e).await;
       }
       Err(e) => {
           log_fatal_and_shutdown(e);
           break;
       }
   }
```

---

## 16. ? + map_err vs ? + #[from]

```
   ❌ TRƯỚC (verbose):
   ────────────────────
   fn read_config() -> Result<Config, MyError> {
       let s = std::fs::read_to_string("c.toml")
           .map_err(|e| MyError::Io(e))?;
       let cfg: Config = toml::from_str(&s)
           .map_err(|e| MyError::Parse(e))?;
       Ok(cfg)
   }
   
   ↑ Mỗi dòng phải explicit chuyển kiểu
   
   
   ✅ SAU (sạch với thiserror):
   ─────────────────────────────
   
   #[derive(Error, Debug)]
   pub enum MyError {
       #[error("IO error")]
       Io(#[from] std::io::Error),       ← #[from] sinh impl From
       
       #[error("parse error")]
       Parse(#[from] toml::de::Error),   ← #[from] sinh impl From
   }
   
   fn read_config() -> Result<Config, MyError> {
       let s = std::fs::read_to_string("c.toml")?;  ← ? + From auto
       let cfg: Config = toml::from_str(&s)?;       ← ? + From auto
       Ok(cfg)
   }
   
   ↑ Code sạch, type-safe, không nhiều magic.
```

---

## 17. HTTP server error response pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                 axum handler                             │
   │                                                          │
   │  async fn get_user(Path(id)) -> Result<Json<User>, App> │
   │                              ├── thành công             │
   │                              ▼                          │
   │                          Json<User>                     │
   │                              │                          │
   │                              ▼                          │
   │                       HTTP 200 + JSON body              │
   │                                                          │
   │                              ├── error                  │
   │                              ▼                          │
   │                       AppError                          │
   │                              │                          │
   │            ┌─────────────────┼─────────────┐            │
   │            ▼                 ▼             ▼            │
   │       NotFound         Unauthorized    Internal         │
   │       404 + body       401 + body      500 + body       │
   │       "NOT_FOUND"      "UNAUTHORIZED"  log full         │
   │                                        + "INTERNAL"     │
   │                                          (hide info)    │
   └──────────────────────────────────────────────────────────┘
   
   
   IntoResponse impl pseudo-code:
   ──────────────────────────────
   
   impl IntoResponse for AppError {
       fn into_response(self) -> Response {
           let (status, code) = match &self {
               NotFound      => (404, "NOT_FOUND"),
               Unauthorized  => (401, "UNAUTHORIZED"),
               Internal(e)   => {
                   tracing::error!(?e, "internal");  ← log full DEBUG
                   (500, "INTERNAL")
               }
           };
           Json(json!({
               "error": { "code": code, "message": self.to_string() }
           }))
       }
   }
   
   ⚠️  Quan trọng: client KHÔNG cần biết internal stack trace.
       Log đầy đủ ở server, return ngắn gọn cho client.
```

---

## 18. Retry với exponential backoff

```
                    ATTEMPT 1
                       │
              fetch_user(id)
                       │
              ┌────────┴────────┐
              ▼                 ▼
              Ok              Err (transient)
              │                 │
              │            wait 100ms
           return            │
                          ATTEMPT 2
                             │
                       fetch_user(id)
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
                    Ok              Err (transient)
                    │                 │
                    │            wait 200ms
                 return            │
                                ATTEMPT 3
                                   │
                             fetch_user(id)
                                   │
                          ┌────────┴────────┐
                          ▼                 ▼
                          Ok              Err
                          │                 │
                          │             give up
                       return        return Err
   
   Delay tăng: 100ms → 200ms → 400ms → 800ms → ...
   (vì 100 << attempt: 100<<0, 100<<1, 100<<2, ...)
   
   
   ⚠️ Chỉ retry với TRANSIENT errors:
   ────────────────────────────────────
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

---

## 19. Validation aggregation — Collect all errors

```
   ❌ Stop-on-first (xấu trải nghiệm UX):
   ──────────────────────────────────────
   
   fn validate(input) -> Result<(), Error> {
       if input.email.is_empty() {
           return Err("email required");
       }
       if input.age < 18 {
           return Err("age must be ≥ 18");   ← user fix email, submit, lại lỗi age
       }
       Ok(())
   }
   
   User experience: submit → email lỗi → fix → submit → age lỗi → ...
   

   ✅ Collect all (UX tốt):
   ────────────────────────
   
   fn validate(input) -> Result<(), Vec<FieldError>> {
       let mut errors = vec![];
       
       if input.email.is_empty() {
           errors.push(FieldError::new("email", "required"));
       }
       
       if input.age < 18 {
           errors.push(FieldError::new("age", "must be ≥ 18"));
       }
       
       if input.username.len() > 32 {
           errors.push(FieldError::new("username", "too long"));
       }
       
       if errors.is_empty() {
           Ok(())
       } else {
           Err(errors)
       }
   }
   
   ┌──────────────────────────────────────────┐
   │ Server response:                         │
   │   422 Unprocessable                      │
   │   {                                      │
   │     "errors": [                          │
   │       { "field": "email", "msg": "..." },│
   │       { "field": "age",   "msg": "..." },│
   │     ]                                    │
   │   }                                      │
   └──────────────────────────────────────────┘
   
   User thấy TẤT CẢ lỗi 1 lần → fix all → submit OK.
```

---

## 20. unwrap() vs expect() vs ? — Lựa chọn

```
                   ERROR TỪ FUNCTION
                          │
              ┌───────────┴───────────┐
              │                       │
          Có khả năng              KHÔNG BAO GIỜ
          xảy ra ở                  xảy ra (vì
          runtime?                  invariant)
              │                       │
              ▼                       ▼
   ┌──────────────────┐      ┌──────────────────┐
   │ Production code? │      │ ✅ expect("..."  │
   └─────────┬────────┘      │   với explanation│
             │               │   tại sao OK)    │
        ┌────┴────┐          │                  │
       YES        NO         │ NOT unwrap()     │
        │         │          └──────────────────┘
        ▼         ▼          
   ┌────────┐ ┌─────────┐    
   │ ✅ ?   │ │ ✅ unwrap│    
   │ Result │ │ trong   │    
   │ + tracing│ tests,  │    
   │ + context│ examples│    
   └────────┘ └─────────┘    
   
   
   ❌ Anti-patterns:
   ──────────────────
   .unwrap() trong production code không có comment
   .unwrap() để "tạm" rồi quên fix
   .unwrap() vì lười define error variant
   .expect("") (empty message — ngang unwrap)
```

---

## 21. Logging error đúng cách

```
   ❌ TỆ — chỉ Display:
   ────────────────────
   if let Err(e) = do_stuff() {
       eprintln!("Error: {}", e);   ← chỉ Display, mất source chain
   }
   
   Output:
   ┌──────────────────────────────┐
   │ Error: query failed          │
   └──────────────────────────────┘
                                     ↑ thiếu root cause!
   
   
   ✅ TỐT — Debug format giữ chain:
   ────────────────────────────────
   if let Err(e) = do_stuff() {
       tracing::error!(?error = e, "doing stuff");
   }
   
   Output:
   ┌──────────────────────────────────────────────┐
   │ ERROR doing stuff error=AppError::Query {    │
   │   table: "users",                            │
   │   source: tokio_postgres::Error {            │
   │     kind: Connection,                        │
   │     source: io::Error {                      │
   │       kind: TimedOut, ...                    │
   │     }                                        │
   │   }                                          │
   │ }                                            │
   └──────────────────────────────────────────────┘
                                     ↑ FULL chain
   
   
   ✅ TUYỆT VỜI — anyhow display chain:
   ─────────────────────────────────────
   let err: anyhow::Error = ...;
   tracing::error!("{:?}", err);   // anyhow Debug = chain + backtrace
   
   ┌──────────────────────────────────────────────┐
   │ Error: processing user 42                    │
   │ Caused by:                                   │
   │   0: query failed                            │
   │   1: connection lost                         │
   │   2: TimedOut                                │
   │ Backtrace:                                   │
   │   0: myapp::main at src/main.rs:42           │
   │   1: ...                                     │
   └──────────────────────────────────────────────┘
```

---

## 22. 12 Antipatterns chốt lại

```
   ┌─────────────────────────────────────────────────────────────┐
   │  ❌ ANTIPATTERN              ✅ INSTEAD                     │
   ├─────────────────────────────────────────────────────────────┤
   │  1. .unwrap() khắp nơi       → ? hoặc expect("rõ ràng")    │
   │                                                             │
   │  2. let _ = risky();         → ít nhất log warning          │
   │     (swallow error)                                         │
   │                                                             │
   │  3. Result<T, String>        → Define typed error           │
   │     (stringly-typed)            với thiserror               │
   │                                                             │
   │  4. eprintln!(error)         → tracing::error!(?err, "..." )│
   │                                                             │
   │  5. .map_err() mỗi dòng      → #[from] trên enum variants  │
   │                                                             │
   │  6. panic! trong library     → return Result                │
   │                                                             │
   │  7. Quên log trước return 500 → log trong IntoResponse      │
   │                                                             │
   │  8. Một enum khổng lồ cho    → split theo module/feature    │
   │     cả crate                    boundaries                  │
   │                                                             │
   │  9. Box<dyn Error> public API → typed enum thiserror        │
   │                                                             │
   │ 10. err.to_string()           → format!("{:?}", err)        │
   │     mất chain                    hoặc tracing ?error        │
   │                                                             │
   │ 11. Block error path khi test → test error path explicitly  │
   │                                                             │
   │ 12. Leak secrets trong error  → custom Debug để redact      │
   │     message                                                 │
   └─────────────────────────────────────────────────────────────┘
```

---

## 23. Crate ecosystem — Senior toolkit

```
   ┌──────────────────────────────────────────────────────────┐
   │                    ERROR HANDLING                        │
   │                                                          │
   │  ┌───────────────┐         ┌────────────────────┐       │
   │  │  thiserror    │         │  anyhow / eyre     │       │
   │  │ (library)     │         │  (application)     │       │
   │  │               │         │                    │       │
   │  │ derive macro  │         │  Context trait     │       │
   │  │ Error trait   │         │  Boxed dyn Error   │       │
   │  └───────┬───────┘         └─────────┬──────────┘       │
   │          │                            │                  │
   │          └────────────┬───────────────┘                  │
   │                       ▼                                  │
   │                  ┌────────────┐                          │
   │                  │  tracing   │ ← logging                │
   │                  │  + tracing-                          │
   │                  │  error     │   span context           │
   │                  └────────────┘                          │
   │                       │                                  │
   │  ┌────────────────────┼───────────────────────┐         │
   │  ▼                    ▼                       ▼         │
   │ ┌─────────┐    ┌──────────────┐      ┌──────────────┐   │
   │ │ backoff │    │  validator   │      │   snafu      │   │
   │ │ tokio-  │    │ (form val)   │      │  (alt to     │   │
   │ │ retry   │    │              │      │   thiserror) │   │
   │ └─────────┘    └──────────────┘      └──────────────┘   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Quick reference:
   ────────────────
   • thiserror     — Define library error types
   • anyhow        — Application error handling
   • eyre          — anyhow alternative với custom report
   • color-eyre    — Terminal đẹp
   • tracing       — Structured logging (chuẩn de-facto)
   • tracing-error — SpanTrace cho async
   • validator     — Form validation
   • backoff       — Exponential backoff retry
   • tokio-retry   — Retry cho async
```

---

## 24. Mind map cuối — Tổng hợp Error Handling

```
                            ERROR HANDLING
                                   │
       ┌──────────────┬────────────┼────────────┬─────────────┐
       │              │            │            │             │
   TRIẾT LÝ      TOOLS        OPERATORS    PATTERNS      OBSERVABILITY
       │              │            │            │             │
   errors-as-      thiserror      ?           retry          tracing
     values        (library)     From          backoff       Debug fmt
   panic!=bug     anyhow         ok_or         classify      source chain
   Result=        (app)          ?:            transient     backtrace
     recoverable  eyre           desugar       /permanent    span context
                  validator                    /fatal        observability
                  backoff
   
   
              ┌─────────────────────────────────────────────┐
              │  CORE INSIGHT cho SENIOR                    │
              │  ─────────────────────────                  │
              │                                             │
              │  Error path là API thật của bạn.            │
              │  Happy path ai cũng viết được.              │
              │                                             │
              │  Phải:                                      │
              │  • Test error path                          │
              │  • Log đầy đủ (không mất chain)             │
              │  • Cho client biết "code"                   │
              │  • Đừng leak internal info                  │
              │  • Phân loại retry-able                     │
              │  • Add context tại mỗi level                │
              │                                             │
              │  Code reviewer xịn → đọc error path FIRST   │
              └─────────────────────────────────────────────┘
```

---

## 25. Bộ tài liệu Rust hoàn thiện

```
   ┌──────────────────────────────────────────────────────────┐
   │           RUST FOUNDATIONS LIBRARY                       │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   1. memory-model              — Bộ nhớ                  │
   │      memory-model-visual                                 │
   │                                                          │
   │   2. ownership-borrowing       — Quyền sở hữu           │
   │      ownership-borrowing-visual                          │
   │                                                          │
   │   3. trait                     — Polymorphism           │
   │      trait-visual                                        │
   │                                                          │
   │   4. generic                   — Parametric polymorphism │
   │      generic-visual                                      │
   │                                                          │
   │   5. closure                   — Function as value      │
   │      closure-visual                                      │
   │                                                          │
   │   6. async                     — Concurrency model      │
   │      async-visual                                        │
   │                                                          │
   │   7. error-handling            — Error handling         │
   │      error-handling-visual       ← VỪA HOÀN THÀNH       │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 14 files                                         │
   │                                                          │
   │   Bạn đã có nền tảng đầy đủ để viết Rust production 🦀  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Đã làm chủ Error Handling, có thể đi tiếp:

- **Testing patterns** — unit, integration, property-based (proptest), criterion bench, mocking
- **Macros** — `macro_rules!`, procedural macros (chính `thiserror` là proc-macro!)
- **Logging & Observability** — tracing nâng cao, OpenTelemetry, metrics, distributed tracing
- **Unsafe Rust** — raw pointer, FFI, atomic ordering
- **Web framework** — axum/actix realistic project (apply tất cả đã học)
- **Database** — sqlx, sea-orm, diesel với async + error handling đẹp

Báo chủ đề muốn đào sâu! 🦀⚡
