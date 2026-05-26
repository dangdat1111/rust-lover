# Testing trong Rust — Deep Dive

> Tài liệu thứ 15 trong bộ Rust nền tảng. Đọc trước:
> - [error-handling.md](./error-handling.md) — test error path
> - [async.md](./async.md) — async test có quirk
> - [performance.md](./performance.md) — criterion benchmark
> - [observability.md](./observability.md) — test observability config
>
> Testing trong Rust **built-in, đẹp, và mạnh**. Không cần framework ngoài cho 80% nhu cầu.
> Hệ sinh thái crates cung cấp 20% còn lại: proptest, mockall, insta, cargo-fuzz, criterion.
>
> Tài liệu này dạy bạn test như senior:
> - Test pyramid: unit → integration → e2e
> - Mocking đúng cách (không phải overuse)
> - Property-based testing để catch edge cases
> - Snapshot testing cho output complex
> - Fuzz testing cho unsafe / parser
> - Async testing patterns
> - Test organization production

---

# Mục lục

- [Tầng 1: Triết lý testing](#tầng-1-triết-lý-testing)
- [Tầng 2: Unit Tests — Built-in framework](#tầng-2-unit-tests--built-in-framework)
- [Tầng 3: Integration Tests](#tầng-3-integration-tests)
- [Tầng 4: Doc Tests — Tests trong documentation](#tầng-4-doc-tests--tests-trong-documentation)
- [Tầng 5: Assertions — assert!, assert_eq!, panics](#tầng-5-assertions--assert-assert_eq-panics)
- [Tầng 6: Test Fixtures và Helpers](#tầng-6-test-fixtures-và-helpers)
- [Tầng 7: Mocking — mockall và alternatives](#tầng-7-mocking--mockall-và-alternatives)
- [Tầng 8: Property-Based Testing — proptest](#tầng-8-property-based-testing--proptest)
- [Tầng 9: Snapshot Testing — insta](#tầng-9-snapshot-testing--insta)
- [Tầng 10: Fuzz Testing — cargo-fuzz](#tầng-10-fuzz-testing--cargo-fuzz)
- [Tầng 11: Async Testing](#tầng-11-async-testing)
- [Tầng 12: Test Coverage — cargo-llvm-cov](#tầng-12-test-coverage--cargo-llvm-cov)
- [Tầng 13: Benchmarks — criterion](#tầng-13-benchmarks--criterion)
- [Tầng 14: CI/CD integration](#tầng-14-cicd-integration)
- [Tầng 15: Patterns và Antipatterns](#tầng-15-patterns-và-antipatterns)

---

# Tầng 1: Triết lý testing

## 1.1 Tại sao test?

```
   Code không test = bug đang chờ
   
   Cost của bug:
   ──────────────
   • Developer machine:  ~minutes
   • CI catch:           ~hours  
   • QA catch:           ~days
   • Production catch:   ~weeks + reputation damage
   
   ⟹ Catch bugs ASAP. Test = early detection.
```

Rust nổi tiếng "if it compiles, it works" — vì borrow checker catch nhiều bug.

**NHƯNG** compiler không catch:
- Logic errors (parse sai format, off-by-one, edge cases)
- API contract violations
- Performance regressions
- Concurrency issues (deadlock, livelock — đôi khi)
- Integration bugs (DB schema, network)

→ Vẫn cần test.

## 1.2 Test Pyramid

```
                         ▲
                        / \
                       /   \         E2E / System tests
                      / E2E \        (slow, fragile, few)
                     /───────\
                    /         \
                   /           \      Integration tests
                  /Integration  \     (moderate speed, more)
                 /───────────────\
                /                 \
               /                   \   Unit tests
              /     Unit tests       \  (fast, many)
             /───────────────────────\
            
   • Unit:        70-80% of tests, ms each
   • Integration: 15-25%, ~100ms each
   • E2E:         5-10%, seconds each
   
   Đầu tư nhiều vào UNIT — feedback nhanh, debug dễ.
```

## 1.3 What to test?

```
   ✅ TEST:
   • Public API behavior (contract)
   • Business logic
   • Error paths (often overlooked!)
   • Edge cases (empty, max, overflow)
   • Concurrency scenarios
   • Performance critical paths (with criterion)
   
   ❌ DON'T TEST:
   • Private implementation details
   • Trivial getters/setters
   • Third-party libraries (their tests)
   • Compiler behavior
```

**Quy tắc**: test **what** (behavior), không **how** (implementation).

## 1.4 TDD vs Test-After

```
   ┌──────────────────────────────────────────────────────────┐
   │ TDD (Test-Driven Development):                           │
   │ 1. Write failing test                                    │
   │ 2. Write minimum code to pass                            │
   │ 3. Refactor                                              │
   │ → Forces small, testable design                          │
   │                                                          │
   │ Test-After:                                              │
   │ 1. Write code                                            │
   │ 2. Add tests                                             │
   │ → More common in practice, ok if disciplined             │
   │                                                          │
   │ Senior pragmatic:                                        │
   │ • TDD cho complex logic / unclear design                 │
   │ • Test-after cho exploration / prototype                 │
   │ • Always test BEFORE marking "done"                      │
   └──────────────────────────────────────────────────────────┘
```

## 1.5 F.I.R.S.T. principles

Good tests are:
- **F**ast — milliseconds, not seconds
- **I**ndependent — don't depend on order
- **R**epeatable — same result every run
- **S**elf-validating — pass/fail clear
- **T**imely — written near production code

---

# Tầng 2: Unit Tests — Built-in framework

## 2.1 Cấu trúc cơ bản

```rust
// src/lib.rs hoặc src/foo.rs

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
    
    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }
}
```

Key parts:
- `#[cfg(test)]` — module chỉ compile khi test
- `mod tests` — convention name
- `use super::*` — bring parent items
- `#[test]` — mark function as test

Run:
```bash
cargo test
cargo test test_add               # run specific
cargo test --lib                  # only lib tests
cargo test -- --nocapture         # show println!
cargo test -- --test-threads=1    # serial run
```

## 2.2 Test attributes

```rust
#[test]
fn basic_test() { ... }

#[test]
#[should_panic]
fn must_panic() {
    panic!("expected");
}

#[test]
#[should_panic(expected = "invalid")]
fn must_panic_with_message() {
    panic!("invalid input");
}

#[test]
#[ignore]
fn slow_or_broken_test() {
    // Skipped by default. Run with: cargo test -- --ignored
}

#[test]
#[ignore = "requires database"]
fn needs_db() { ... }
```

`#[ignore]` useful for:
- Slow tests (run in CI only)
- Tests requiring external resources
- Temporarily disabled tests

## 2.3 Test return Result

```rust
#[test]
fn test_with_result() -> Result<(), Box<dyn std::error::Error>> {
    let n: i32 = "42".parse()?;
    assert_eq!(n, 42);
    Ok(())
}
```

Dùng `?` trong test — clean error propagation. Test fail nếu return Err.

## 2.4 Naming conventions

```rust
#[test]
fn test_<unit>_<scenario>_<expected>() { ... }

// Examples:
fn test_add_two_positive_returns_sum() { ... }
fn test_parse_empty_string_returns_error() { ... }
fn test_login_invalid_password_returns_unauthorized() { ... }
```

Hoặc style ngắn hơn:
```rust
fn it_returns_sum_for_positives() { ... }
fn errors_when_empty() { ... }
```

Nhất quán trong project.

## 2.5 Private function testing

```rust
// src/lib.rs
fn private_helper(x: i32) -> i32 { x * 2 }   // private

pub fn public_api(x: i32) -> i32 {
    private_helper(x) + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_private() {
        assert_eq!(private_helper(5), 10);   // OK — same module
    }
}
```

Unit tests trong cùng file → access private items. Khác Java: không cần "package private".

## 2.6 Test organization — In-file vs separate

### Option A: Tests in same file (idiomatic Rust)
```rust
// src/foo.rs
pub fn foo() { ... }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_foo() { ... }
}
```

### Option B: Tests in separate module
```rust
// src/foo.rs
pub fn foo() { ... }

#[cfg(test)]
mod tests;   // → src/foo/tests.rs

// src/foo/tests.rs
use super::*;
#[test]
fn test_foo() { ... }
```

Option A phổ biến hơn — keep close to code. Option B for large test modules.

## 2.7 Running tests

```bash
# All tests:
cargo test

# Specific:
cargo test test_add               # match name
cargo test foo::tests             # by module path
cargo test -- --exact test_add    # exact match

# By tag (via name pattern):
cargo test integration

# Output:
cargo test -- --nocapture         # show println!
cargo test -- --show-output       # show output even on success

# Parallelism:
cargo test -- --test-threads=1    # serial (for race-condition prone)

# Coverage / debug:
cargo test -- --list              # list all tests
cargo test -- --ignored           # only ignored
```

## 2.8 #[cfg(test)] vs cfg-attr

```rust
#[cfg(test)]
use proptest::prelude::*;   // only import for tests

#[cfg_attr(test, derive(Debug))]   // derive Debug only when testing
struct Config { ... }

#[cfg(test)]
mod tests {
    // entire module compiled only for tests
}
```

`#[cfg(test)]` excludes from production binary → smaller release.

---

# Tầng 3: Integration Tests

## 3.1 Vị trí

```
my-crate/
├── Cargo.toml
├── src/
│   └── lib.rs
├── tests/                  ← integration tests directory
│   ├── basic.rs            ← one test crate per file
│   ├── auth.rs
│   └── common/             ← helpers (not run as tests)
│       └── mod.rs
```

Mỗi file trong `tests/` = **separate test binary**, separate cargo compile unit.

## 3.2 Viết integration test

```rust
// tests/basic.rs
use my_crate::*;

#[test]
fn integration_test() {
    let result = my_crate::add(2, 3);
    assert_eq!(result, 5);
}

#[test]
fn another_test() {
    // ...
}
```

Differences from unit tests:
- File `tests/foo.rs`, not in `src/`
- No `#[cfg(test)]` needed (entire file is for tests)
- Access **only public API** (cannot test private fn)
- Each file is its own crate (isolated)

## 3.3 Shared test helpers

```rust
// tests/common/mod.rs   ← NOT tests/common.rs!
pub fn setup() -> TestEnv {
    TestEnv::new()
}

pub struct TestEnv { ... }
```

```rust
// tests/auth.rs
mod common;
use common::*;

#[test]
fn auth_test() {
    let env = common::setup();
    // ...
}
```

`tests/common/mod.rs` (with /mod.rs) is **NOT** treated as test crate. Use for shared utilities.

⚠️ Pitfall: `tests/common.rs` (without /mod.rs) WOULD be run as test.

## 3.4 Integration test cho binary

Binary crates (with `src/main.rs`) can't easily test via integration. Pattern:

```
my-app/
├── src/
│   ├── main.rs        ← thin wrapper
│   └── lib.rs         ← all logic here
├── tests/
│   └── integration.rs ← test via lib.rs
```

Refactor logic into `lib.rs`, `main.rs` just `fn main() { my_app::run() }`.

## 3.5 Setup / Teardown

Rust không có built-in setUp/tearDown như JUnit. Patterns:

### Pattern 1: Test fixture struct
```rust
struct TestFixture {
    db: TestDb,
    server: TestServer,
}

impl TestFixture {
    fn new() -> Self {
        TestFixture {
            db: TestDb::spawn(),
            server: TestServer::spawn(),
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        self.server.shutdown();
        self.db.cleanup();
    }
}

#[test]
fn test_login() {
    let fx = TestFixture::new();   // setup
    // ... test ...
    // fx drops → cleanup
}
```

RAII — fixture cleanup tự động qua Drop.

### Pattern 2: Setup helper function
```rust
fn setup() -> (Db, Cleanup) { ... }

#[test]
fn test() {
    let (db, _cleanup) = setup();   // _cleanup drops at end
    // ...
}
```

## 3.6 Test database

```rust
struct TestDb {
    name: String,
    conn: PgConnection,
}

impl TestDb {
    fn new() -> Self {
        // Create unique DB
        let name = format!("test_{}", uuid::Uuid::new_v4());
        create_db(&name);
        run_migrations(&name);
        TestDb { name, conn: PgConnection::connect(&name) }
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        drop_db(&self.name);   // cleanup
    }
}
```

Or use transaction rollback pattern (faster):
```rust
let mut tx = pool.begin().await?;
// ... operations ...
// Don't commit → all changes rolled back
drop(tx);
```

---

# Tầng 4: Doc Tests — Tests trong documentation

## 4.1 Doc test syntax

```rust
/// Adds two numbers.
///
/// # Examples
///
/// ```
/// use my_crate::add;
///
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Run:
```bash
cargo test --doc
```

`cargo test` runs cả unit + integration + doc tests.

## 4.2 Lợi ích doc tests

- **Documentation always correct**: code trong docs phải compile + pass
- **Example for users**: API doc có usage examples
- **No drift**: docs không thể outdated

Mỗi public function nên có 1+ doc test.

## 4.3 Doc test attributes

```rust
/// ```ignore
/// // Won't run (e.g., requires external resource)
/// ```
///
/// ```no_run
/// // Compiles but doesn't run (e.g., side effects)
/// ```
///
/// ```compile_fail
/// // Should FAIL to compile (test that error happens)
/// ```
///
/// ```should_panic
/// // Should panic
/// ```
///
/// ```text
/// // Not Rust, just text
/// ```
```

## 4.4 Hide setup code

```rust
/// ```
/// # use my_crate::*;
/// # let mut env = TestEnv::new();
/// env.run_command("foo");
/// assert_eq!(env.output(), "foo result");
/// ```
```

Lines starting `#` are hidden in rendered docs but still compile. Useful for boilerplate.

## 4.5 When doc tests overkill

- Internal functions (use unit tests)
- Very simple functions (`fn add(a, b) { a + b }`)
- Tests that need extensive setup

Use unit tests + integration tests for those.

---

# Tầng 5: Assertions — assert!, assert_eq!, panics

## 5.1 Built-in macros

```rust
// assert!(condition) — Panic if false
assert!(x > 0);
assert!(s.contains("hello"));

// assert!(condition, "message")
assert!(x > 0, "x must be positive, got {}", x);

// assert_eq!(a, b) — Panic if a != b, print both
assert_eq!(result, 42);
assert_eq!(parsed, expected, "Failed to parse: {}", input);

// assert_ne!(a, b) — Panic if a == b
assert_ne!(error_code, 0);
```

Failure output:
```
thread 'tests::test_foo' panicked at 'assertion `left == right` failed
  left: 5
 right: 6'
```

Both values printed → easy debug.

## 5.2 debug_assert! family

```rust
debug_assert!(invariant_holds);          // Only checks in debug build
debug_assert_eq!(actual, expected);
debug_assert_ne!(a, b);
```

Use for expensive invariant checks during development. Stripped in release.

Don't confuse:
- `assert!` — runtime check, always runs
- `debug_assert!` — only in debug

## 5.3 Custom assertion helpers

```rust
fn assert_email_valid(email: &str) {
    assert!(email.contains('@'), "missing @: {}", email);
    assert!(email.contains('.'), "missing .: {}", email);
    assert!(email.len() > 5, "too short: {}", email);
}

#[test]
fn test_signup() {
    let user = signup("john@doe.com");
    assert_email_valid(&user.email);
}
```

Reusable assertions with rich error messages.

## 5.4 pretty_assertions crate

```toml
[dev-dependencies]
pretty_assertions = "1"
```

```rust
#[cfg(test)]
mod tests {
    use pretty_assertions::{assert_eq, assert_ne};
    
    #[test]
    fn test() {
        assert_eq!(complex_struct1, complex_struct2);
        // Diff output instead of full dump → much better for big structs
    }
}
```

Output: colored diff style. Required for complex types.

## 5.5 Float comparisons

```rust
// ❌ float comparison broken with ==
assert_eq!(0.1 + 0.2, 0.3);   // FAILS! 0.30000000000000004 vs 0.3

// ✅ approx crate
use approx::assert_relative_eq;
assert_relative_eq!(0.1 + 0.2, 0.3);
assert_relative_eq!(0.1 + 0.2, 0.3, epsilon = 1e-10);
```

`approx` crate provides float-safe comparisons.

## 5.6 should_panic specifics

```rust
#[test]
#[should_panic]
fn panics_any() { panic!("anything"); }

#[test]
#[should_panic(expected = "div by zero")]
fn panics_specific() { panic!("attempted to divide by zero"); }
// Matches if panic message CONTAINS "div by zero"
```

Be specific to avoid false positives (test passes for wrong panic).

## 5.7 Result-based assertion

```rust
#[test]
fn test_parse() -> Result<(), Box<dyn std::error::Error>> {
    let n: i32 = "42".parse()?;
    assert_eq!(n, 42);
    Ok(())
}
```

Prefer Result over `unwrap()` in tests — clearer failure.

But sometimes unwrap is OK:
```rust
#[test]
fn quick_test() {
    let n: i32 = "42".parse().unwrap();   // OK in tests
    assert_eq!(n, 42);
}
```

`unwrap()` causes test fail with panic — fine for tests, not production.

---

# Tầng 6: Test Fixtures và Helpers

## 6.1 The setup problem

```rust
#[test]
fn test_1() {
    let db = setup_db();
    let user = create_user(&db, "alice");
    // ... test ...
}

#[test]
fn test_2() {
    let db = setup_db();        // ⚠️ duplicate setup
    let user = create_user(&db, "bob");
    // ... test ...
}
```

DRY violation. Need helpers.

## 6.2 Fixture pattern

```rust
struct UserFixture {
    db: TestDb,
    user: User,
}

impl UserFixture {
    fn new(name: &str) -> Self {
        let db = TestDb::new();
        let user = create_user(&db, name);
        UserFixture { db, user }
    }
    
    fn with_orders(mut self, count: usize) -> Self {
        for _ in 0..count {
            create_order(&self.db, &self.user);
        }
        self
    }
}

#[test]
fn test_user_with_orders() {
    let fx = UserFixture::new("alice").with_orders(3);
    
    let orders = list_orders(&fx.db, &fx.user);
    assert_eq!(orders.len(), 3);
}
```

Builder-style fixture — flexible setup.

## 6.3 Test helpers module

```rust
// src/test_helpers.rs (or in tests/common/mod.rs)

#[cfg(test)]
pub fn make_request(path: &str) -> TestRequest { ... }

#[cfg(test)]
pub fn assert_response_ok<T: serde::Serialize>(resp: &Response, expected: T) {
    // shared assertion logic
}
```

Centralize helpers — easier to maintain.

## 6.4 Test data builders

```rust
struct UserBuilder {
    name: String,
    age: u32,
    active: bool,
}

impl UserBuilder {
    fn new() -> Self {
        UserBuilder {
            name: "default".into(),
            age: 18,
            active: true,
        }
    }
    
    fn name(mut self, n: &str) -> Self { self.name = n.into(); self }
    fn age(mut self, a: u32) -> Self { self.age = a; self }
    fn inactive(mut self) -> Self { self.active = false; self }
    
    fn build(self) -> User {
        User { name: self.name, age: self.age, active: self.active }
    }
}

#[test]
fn test() {
    let u = UserBuilder::new()
        .name("alice")
        .age(30)
        .build();
}
```

Builder for test data → clean, default fields without boilerplate.

## 6.5 Shared state — Watch out

```rust
// ❌ shared mutable state
static mut COUNT: u32 = 0;

#[test]
fn test_1() { unsafe { COUNT += 1; } }   // race in parallel test

#[test]
fn test_2() { unsafe { COUNT += 1; } }
```

Tests run **in parallel** by default. Shared mutable state → race / order-dependence.

Fix:
- Each test owns its data
- Or use `#[serial]` from `serial_test` crate
- Or `--test-threads=1`

## 6.6 Test environment

```rust
fn with_env<F>(key: &str, val: &str, f: F)
where F: FnOnce() {
    std::env::set_var(key, val);
    f();
    std::env::remove_var(key);
}

#[test]
fn test_with_env() {
    with_env("DEBUG", "true", || {
        let result = read_config();
        assert!(result.debug);
    });
}
```

⚠️ `std::env::set_var` is process-global — affects parallel tests. Use `serial_test::serial` if needed.

---

# Tầng 7: Mocking — mockall và alternatives

## 7.1 Vấn đề: External dependencies in unit tests

```rust
struct UserService<DB: Database> {
    db: DB,
}

impl<DB: Database> UserService<DB> {
    fn login(&self, user: &str, pw: &str) -> Result<Session, Error> {
        let row = self.db.query_user(user)?;
        // ...
    }
}
```

Unit test cần `DB` instance. Real DB:
- Slow (network, disk)
- Hard to setup/cleanup
- Brittle (DB state shared)

→ Need **mock** DB for unit testing.

## 7.2 Manual mock với trait

```rust
trait Database {
    fn query_user(&self, name: &str) -> Result<UserRow, Error>;
}

// Production:
struct PgDatabase { ... }
impl Database for PgDatabase { ... }

// Mock for testing:
struct MockDatabase {
    users: HashMap<String, UserRow>,
}

impl Database for MockDatabase {
    fn query_user(&self, name: &str) -> Result<UserRow, Error> {
        self.users.get(name).cloned().ok_or(Error::NotFound)
    }
}

#[test]
fn test_login() {
    let mut mock = MockDatabase { users: HashMap::new() };
    mock.users.insert("alice".into(), UserRow { ... });
    
    let svc = UserService { db: mock };
    let result = svc.login("alice", "pwd");
    assert!(result.is_ok());
}
```

Simple, no extra crate. Good for small projects.

## 7.3 mockall — Auto-generate mocks

```toml
[dev-dependencies]
mockall = "0.13"
```

```rust
use mockall::*;

#[automock]
trait Database {
    fn query_user(&self, name: &str) -> Result<UserRow, Error>;
}

// mockall generates `MockDatabase` automatically

#[test]
fn test_login() {
    let mut mock = MockDatabase::new();
    
    mock.expect_query_user()
        .with(eq("alice"))
        .returning(|_| Ok(UserRow { id: 1, name: "alice".into() }));
    
    let svc = UserService { db: mock };
    let result = svc.login("alice", "pwd");
    assert!(result.is_ok());
}
```

Features:
- `expect_<method>().with(matcher).returning(closure)` — define behavior
- Match: `eq`, `gt`, `lt`, `always`, `function(|x| ...)`, `predicate`
- Returning: `returning(closure)`, `return_const(value)`
- Verify call count: `.times(2)`, `.never()`
- Sequence: ensure order of calls

## 7.4 mockall — Methods on existing struct

```rust
#[automock]
impl MyStruct {
    pub fn foo(&self) -> i32 { ... }
    pub fn bar(&self, x: i32) -> String { ... }
}
```

Mock struct (not just trait). More invasive.

## 7.5 mockall patterns

### Sequence matching
```rust
let mut seq = Sequence::new();

mock.expect_call()
    .times(1)
    .in_sequence(&mut seq)
    .returning(|| 1);

mock.expect_call()
    .times(1)
    .in_sequence(&mut seq)
    .returning(|| 2);
```

### Strict mode
```rust
let mock = MockDatabase::new();
// No expectations set
mock.query_user("alice");
// PANIC: unexpected call
```

mockall is **strict by default**. Every call must match an expectation.

### Default behaviors
```rust
mock.expect_query_user()
    .returning(|_| Ok(default_row()));   // any call returns default
```

## 7.6 When mocking gets bad

```rust
// ❌ Mock everything — fragile test
#[test]
fn test_complex_workflow() {
    let mut db = MockDatabase::new();
    let mut http = MockHttpClient::new();
    let mut cache = MockCache::new();
    
    // 30 lines of mock setup...
    // Now testing mock behavior, not real logic
}
```

When mocks dominate test → smell. Refactor:
- Use real implementations for trusted dependencies
- Or use integration tests instead

## 7.7 Test doubles — Terminology

| Type | What |
|------|------|
| **Dummy** | Placeholder, never used (e.g., null) |
| **Stub** | Returns canned answers |
| **Spy** | Stub + records calls |
| **Mock** | Pre-programmed expectations, verifies on assertion |
| **Fake** | Working impl shortcut (e.g., in-memory DB) |

Rust mockall generates **mocks**. Manual structs are often **fakes**.

Senior pragmatic: **fakes > mocks**. Less coupling to implementation.

```rust
// Fake — in-memory DB
struct InMemoryDb {
    users: RefCell<HashMap<String, UserRow>>,
}

impl Database for InMemoryDb { ... }   // real impl, just in-memory
```

Test using `InMemoryDb` → tests real query logic without external DB.

---

# Tầng 8: Property-Based Testing — proptest

## 8.1 Limitation of example-based tests

```rust
#[test]
fn test_reverse() {
    assert_eq!(reverse("hello"), "olleh");
    assert_eq!(reverse(""), "");
    assert_eq!(reverse("a"), "a");
}
```

You test specific examples. **Edge cases miss**:
- Unicode characters
- Very long strings
- Special characters

→ Property-based: test **properties** that hold for **all** inputs.

## 8.2 proptest setup

```toml
[dev-dependencies]
proptest = "1"
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn reverse_twice_is_identity(s: String) {
        let r = reverse(&reverse(&s));
        prop_assert_eq!(r, s);
    }
}
```

proptest **generates** random strings, checks property `reverse(reverse(x)) == x` holds.

Run:
```bash
cargo test
# proptest tries 256 random inputs (default), shrinks on failure
```

## 8.3 Property examples

```rust
proptest! {
    #[test]
    fn sort_is_idempotent(mut v: Vec<i32>) {
        let mut sorted = v.clone();
        sorted.sort();
        let mut twice = sorted.clone();
        twice.sort();
        prop_assert_eq!(sorted, twice);
    }
    
    #[test]
    fn sort_preserves_length(mut v: Vec<i32>) {
        let original_len = v.len();
        v.sort();
        prop_assert_eq!(v.len(), original_len);
    }
    
    #[test]
    fn encode_decode_roundtrip(data: Vec<u8>) {
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        prop_assert_eq!(decoded, data);
    }
}
```

## 8.4 Custom strategies

```rust
fn email_strategy() -> impl Strategy<Value = String> {
    "[a-z]{3,10}@[a-z]{3,8}\\.com"   // regex generator
        .prop_map(|s| s)
}

proptest! {
    #[test]
    fn parse_valid_email(email in email_strategy()) {
        assert!(parse_email(&email).is_ok());
    }
}
```

Strategies generate **specific** types of inputs.

## 8.5 Shrinking — Find minimal failing case

```rust
proptest! {
    #[test]
    fn bad_function(v: Vec<i32>) {
        prop_assert!(bad_fn(&v) >= 0);
    }
}

// If bad_fn(vec![5, 2, 3, 1, 0]) fails,
// proptest will SHRINK:
//   Try shorter: [5, 2, 3], [5, 2], [5], []
//   Try smaller values: [0, 0, 0, 0, 0]
//   Until smallest failing case found, e.g., [0]
```

Report shows **minimal failing example** — much easier to debug.

## 8.6 Catch real bugs

Property tests famously find:
- Off-by-one
- Integer overflow
- Unicode edge cases
- Empty / null cases
- Boundary conditions

Standard library use proptest for sort, string manipulation, etc.

## 8.7 quickcheck (alternative)

```toml
[dev-dependencies]
quickcheck = "1"
quickcheck_macros = "1"
```

```rust
use quickcheck_macros::quickcheck;

#[quickcheck]
fn reverse_twice_identity(s: String) -> bool {
    reverse(&reverse(&s)) == s
}
```

Older but simpler. Less powerful than proptest (no custom strategies, weaker shrinking).

Most senior projects use proptest.

---

# Tầng 9: Snapshot Testing — insta

## 9.1 Vấn đề: Big complex output

```rust
#[test]
fn test_render() {
    let result = render(input);
    assert_eq!(result, "very long expected string...");   // ugly
}
```

For complex outputs (JSON, HTML, AST), assertions get unwieldy.

## 9.2 insta crate

```toml
[dev-dependencies]
insta = "1"
```

```rust
use insta::assert_yaml_snapshot;

#[test]
fn test_user_render() {
    let user = User { name: "alice", age: 30 };
    insta::assert_yaml_snapshot!(user);
}
```

First run: creates `tests/snapshots/test_user_render.snap` with output.
Subsequent runs: compare to snapshot.

If output changes:
```bash
cargo insta review   # interactive accept/reject
```

## 9.3 Use cases

- Render output (HTML, JSON)
- Compiler errors / diagnostics
- AST / code generation
- Configuration dumps
- Complex structs (Debug format)

## 9.4 Snapshot formats

```rust
insta::assert_debug_snapshot!(value);        // Debug format
insta::assert_yaml_snapshot!(value);         // YAML (serde)
insta::assert_json_snapshot!(value);         // JSON
insta::assert_display_snapshot!(value);      // Display
```

YAML/JSON require `serde::Serialize`.

## 9.5 Workflow

```bash
# Run tests:
cargo test

# If output changed, snapshot diff:
# - Old snapshot saved
# - New "fresh" snapshot file created

# Review:
cargo insta review
# Interactive: accept new, keep old, or stop

# Or accept all:
cargo insta accept
```

## 9.6 Inline snapshots

```rust
#[test]
fn test() {
    let result = render(input);
    insta::assert_yaml_snapshot!(result, @r###"
    name: alice
    age: 30
    "###);
}
```

Snapshot in test file itself. Good for short outputs.

## 9.7 When snapshot perfect fit

- Output is **deterministic**
- Output is **big** (assert_eq awkward)
- You **don't write expected manually** — let tool do it

**NOT for**:
- Non-deterministic outputs (timestamps, UUIDs — need redaction)
- Tiny outputs (just use assert_eq)
- When you need to verify specific properties (not just "looks same")

## 9.8 Redacting non-deterministic fields

```rust
insta::assert_json_snapshot!(value, {
    ".created_at" => "[timestamp]",
    ".id" => "[uuid]",
});
```

Replace dynamic fields with placeholders before comparison.

---

# Tầng 10: Fuzz Testing — cargo-fuzz

## 10.1 Fuzz testing — What

**Fuzzing**: feed **random / malformed inputs** to code, look for crashes, hangs, UB.

Classic for:
- Parsers (JSON, HTTP, etc.)
- Decoders (image, audio)
- Unsafe code (memory bugs)
- Security-critical code

Found bugs in: rustc, serde, regex, image parsers, web servers.

## 10.2 cargo-fuzz setup

```bash
cargo install cargo-fuzz
cd my-crate
cargo fuzz init
cargo fuzz add my_target
```

Creates:
```
fuzz/
├── Cargo.toml
└── fuzz_targets/
    └── my_target.rs
```

## 10.3 Fuzz target

```rust
// fuzz/fuzz_targets/parse_json.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = my_crate::parse_json(s);   // shouldn't panic on any input
    }
});
```

`fuzz_target!` macro takes closure with random bytes. Runs millions of times.

Run:
```bash
cargo +nightly fuzz run parse_json
```

Indefinite. Crashes → save to `fuzz/artifacts/`.

## 10.4 Structured fuzzing — arbitrary

```toml
[dependencies]
arbitrary = "1"
```

```rust
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct Input {
    name: String,
    age: u32,
    items: Vec<u8>,
}

fuzz_target!(|input: Input| {
    process(input);   // typed fuzzer
});
```

Better than raw bytes — fuzzer generates valid type structure.

## 10.5 Coverage-guided fuzzing

cargo-fuzz uses **libFuzzer** (LLVM):
- Instruments code to track coverage
- Mutates inputs to maximize coverage
- Finds paths human wouldn't think of

→ Very effective for finding bugs.

## 10.6 Saving inputs (corpus)

```bash
fuzz/corpus/my_target/   # interesting inputs found by fuzzer
fuzz/artifacts/my_target/ # crashes
```

Save corpus to git — speed up future fuzzing.

## 10.7 Property-based fuzzing — Quickcheck for unsafe

For unsafe code, combine fuzz + property:

```rust
fuzz_target!(|data: &[u8]| {
    let v = parse(data);
    // Property: parsing then serializing roundtrips
    let serialized = serialize(&v);
    let v2 = parse(&serialized);
    assert_eq!(v, v2);
});
```

## 10.8 When to fuzz

✅ Fuzz:
- Parsers, decoders
- Network protocols
- Unsafe code
- Security-sensitive code (auth, crypto)
- Library exposed to untrusted input

⚠️ Skip:
- Pure business logic (proptest enough)
- Internal-only code
- Trivial functions

---

# Tầng 11: Async Testing

## 11.1 Vấn đề: #[test] không async

```rust
#[test]
async fn async_test() {   // ❌ ERROR: tests can't be async
    let result = async_fn().await;
}
```

Need test runtime.

## 11.2 #[tokio::test]

```rust
#[tokio::test]
async fn test_async() {
    let result = async_fn().await;
    assert_eq!(result, expected);
}
```

`#[tokio::test]` wraps test in tokio runtime. Replaces `#[test]`.

Variants:
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent() { ... }

#[tokio::test(start_paused = true)]
async fn test_with_pausable_time() {
    tokio::time::sleep(Duration::from_secs(1)).await;   // instant — virtual time
}
```

## 11.3 Other runtimes

```rust
#[async_std::test]
async fn test() { ... }
```

Or manual:
```rust
#[test]
fn test() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        async_fn().await;
    });
}
```

## 11.4 Testing async time

```rust
#[tokio::test(start_paused = true)]
async fn test_timeout() {
    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(60)).await;
        "done"
    });
    
    tokio::time::advance(Duration::from_secs(60)).await;   // skip ahead
    assert_eq!(task.await.unwrap(), "done");
}
```

`start_paused = true` → time is virtual, controlled by `advance`. Fast tests for timeouts.

## 11.5 Concurrent test patterns

```rust
#[tokio::test]
async fn test_concurrent_writes() {
    let counter = Arc::new(AtomicI32::new(0));
    
    let mut handles = vec![];
    for _ in 0..100 {
        let c = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            c.fetch_add(1, Ordering::Relaxed);
        }));
    }
    
    for h in handles { h.await.unwrap(); }
    
    assert_eq!(counter.load(Ordering::Relaxed), 100);
}
```

## 11.6 Testing với mock async

```rust
#[automock]
#[async_trait]
trait AsyncDb {
    async fn fetch(&self, id: u64) -> Result<User, Error>;
}

#[tokio::test]
async fn test() {
    let mut mock = MockAsyncDb::new();
    mock.expect_fetch()
        .returning(|_| Box::pin(async { Ok(User { ... }) }));
    
    let svc = Service::new(mock);
    let result = svc.handle(1).await;
    assert!(result.is_ok());
}
```

`async_trait` crate + mockall support async traits.

## 11.7 loom — Concurrent correctness testing

```toml
[dev-dependencies]
loom = "0.7"
```

```rust
#[cfg(loom)]
use loom::sync::Mutex;
#[cfg(not(loom))]
use std::sync::Mutex;

#[test]
#[cfg_attr(loom, ignore)]
fn test() {
    loom::model(|| {
        // Loom explores ALL thread interleavings
        let m = Arc::new(Mutex::new(0));
        let m1 = Arc::clone(&m);
        let t = loom::thread::spawn(move || {
            *m1.lock().unwrap() = 1;
        });
        *m.lock().unwrap() = 2;
        t.join().unwrap();
    });
}
```

Run:
```bash
RUSTFLAGS="--cfg loom" cargo test
```

loom **exhaustively** tests interleavings → finds rare races. Slow but thorough.

## 11.8 Real I/O testing

For HTTP, DB, etc.:
- Use **test container** (testcontainers crate)
- Spin up real Postgres/Redis/etc. in Docker
- Slower but realistic

```rust
use testcontainers::*;

#[tokio::test]
async fn test_db() {
    let docker = clients::Cli::default();
    let pg = docker.run(images::postgres::Postgres::default());
    let port = pg.get_host_port_ipv4(5432);
    
    let pool = connect(&format!("postgres://localhost:{}/postgres", port)).await?;
    // ... real DB tests ...
}
```

---

# Tầng 12: Test Coverage — cargo-llvm-cov

## 12.1 Why coverage?

```
   Coverage = % code executed by tests
   
   ⚠️ 100% coverage ≠ no bugs (just means executed, not verified correct)
   ⚠️ Low coverage ≠ broken
   
   But:
   • Highlights untested code
   • Helps allocate testing effort
   • Catches obviously missed paths
```

## 12.2 cargo-llvm-cov

```bash
cargo install cargo-llvm-cov

cargo llvm-cov
cargo llvm-cov --html   # HTML report
cargo llvm-cov --lcov   # LCOV format (for CI)
cargo llvm-cov --json   # JSON for tooling
```

Uses LLVM source-based coverage — accurate, fast.

## 12.3 Output example

```
Filename                  Regions  Missed  Cover  Functions  Missed  Cover
src/lib.rs                100      5       95%    20         0       100%
src/handler.rs            234      45      80%    15         2       86%
src/utils.rs              50       50      0%     5          5       0%
```

`src/utils.rs` 0% covered → write tests.

## 12.4 HTML report

```bash
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

Click into file → see line-by-line:
- Green: covered
- Red: not covered
- Yellow: partial (some branches covered)

Eye candy. Drill into specific files.

## 12.5 Coverage in CI

```yaml
# GitHub Actions
- name: Coverage
  run: cargo llvm-cov --lcov --output-path lcov.info

- name: Upload to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: lcov.info
```

Codecov / Coveralls track coverage over time, comment PRs.

## 12.6 Coverage gotchas

- Coverage measures execution, NOT correctness
- 100% coverage misses untested edge cases
- Branch coverage > line coverage (sometimes)
- Don't fixate on number — focus on critical paths

Target: 70-85% line, focus on business logic + error paths.

## 12.7 Targeted testing

```bash
cargo llvm-cov --show-missing-lines | head
```

Shows uncovered lines first. Prioritize tests there.

---

# Tầng 13: Benchmarks — criterion

Cross-reference [performance.md Tầng 4](./performance.md). Key points:

## 13.1 Setup

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "my_bench"
harness = false
```

## 13.2 Bench file

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

## 13.3 Run

```bash
cargo bench
# Output: target/criterion/report/index.html
```

## 13.4 Regression detection in CI

```bash
cargo bench -- --save-baseline main
# After change:
cargo bench -- --baseline main
# Fails if regressed > threshold
```

## 13.5 What to benchmark

- Hot paths (after profile)
- Public API performance
- Algorithm comparisons
- Performance-critical libraries

**Don't** benchmark:
- Trivial code
- Code not in hot path
- Without `black_box`

---

# Tầng 14: CI/CD integration

## 14.1 GitHub Actions example

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Format
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy -- -D warnings
      
      - name: Build
        run: cargo build --verbose
      
      - name: Test
        run: cargo test --verbose
      
      - name: Doc Tests
        run: cargo test --doc
  
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-llvm-cov
      - run: cargo llvm-cov --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
  
  miri:
    runs-on: ubuntu-latest
    if: contains(github.event.head_commit.message, '[miri]') || github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: miri
      - run: cargo +nightly miri test
```

## 14.2 Quality gates

```yaml
- name: Quality gates
  run: |
    cargo fmt --check
    cargo clippy -- -D warnings           # treat warnings as errors
    cargo test --all-features
    cargo audit                            # vulnerability scan
```

Block merge if any fails.

## 14.3 Matrix testing

```yaml
strategy:
  matrix:
    rust: [stable, beta, nightly]
    os: [ubuntu-latest, macos-latest, windows-latest]

runs-on: ${{ matrix.os }}
steps:
  - uses: dtolnay/rust-toolchain@${{ matrix.rust }}
  - run: cargo test
```

Test on multiple Rust versions + OS combinations.

## 14.4 Benchmark on PRs

```yaml
- name: Benchmark
  run: |
    cargo bench --bench my_bench -- --save-baseline pr-${{ github.event.number }}
    
    # Compare to main:
    git checkout main
    cargo bench -- --baseline pr-${{ github.event.number }}
```

Report regression. Comment on PR.

## 14.5 Test parallelization

```yaml
- name: Test (parallel)
  run: cargo nextest run
```

`cargo-nextest` — modern test runner:
- Parallel execution by default
- Better output
- Retry flaky tests
- JUnit XML output

Recommended for big projects.

---

# Tầng 15: Patterns và Antipatterns

## 15.1 ✅ Pattern: Arrange-Act-Assert

```rust
#[test]
fn test_login() {
    // Arrange
    let svc = UserService::new(MockDb::new());
    let creds = Credentials { user: "alice", pw: "pwd" };
    
    // Act
    let result = svc.login(creds);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap().user_id, 1);
}
```

Clear structure. Easy to read.

## 15.2 ✅ Pattern: One assertion per test

```rust
// ❌ Multiple unrelated assertions
#[test]
fn test_user() {
    let user = create_user("alice");
    assert_eq!(user.name, "alice");
    assert!(user.is_active);
    assert_eq!(user.balance, 0);
    // If first fails, others not checked
}

// ✅ Separate tests
#[test]
fn test_user_has_name() {
    let user = create_user("alice");
    assert_eq!(user.name, "alice");
}

#[test]
fn test_user_starts_active() {
    let user = create_user("alice");
    assert!(user.is_active);
}
```

Specific failures, all run independently.

(But: closely-related assertions can group: `assert_eq!(user.name, ...); assert_eq!(user.email, ...);` — fine if testing same concern.)

## 15.3 ✅ Pattern: Test names describe scenario

```rust
// ✅ Clear
#[test]
fn login_with_invalid_password_returns_unauthorized() { ... }

#[test]
fn parse_empty_string_returns_default() { ... }

// ❌ Vague
#[test]
fn test_login() { ... }

#[test]
fn test1() { ... }
```

## 15.4 ✅ Pattern: Test error path

```rust
#[test]
fn parse_invalid_input_returns_specific_error() {
    let result = parse("garbage");
    
    match result {
        Err(MyError::InvalidFormat(msg)) => {
            assert!(msg.contains("expected ="));
        }
        _ => panic!("Wrong error variant: {:?}", result),
    }
}
```

Error path is API too. Test it thoroughly.

## 15.5 ✅ Pattern: Use Result in tests

```rust
#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let user = create_user(&config)?;
    assert_eq!(user.name, "alice");
    Ok(())
}
```

Cleaner than `unwrap()` everywhere.

## 15.6 ❌ Antipattern: Testing implementation details

```rust
// ❌ Tests internal cache field
#[test]
fn test_cache_uses_hashmap() {
    let svc = Service::new();
    svc.handle(1);
    assert_eq!(svc.cache.len(), 1);   // accessing private field
}

// ✅ Test behavior
#[test]
fn test_repeated_calls_return_same_result() {
    let svc = Service::new();
    let r1 = svc.handle(1);
    let r2 = svc.handle(1);
    assert_eq!(r1, r2);   // observable behavior
}
```

Implementation detail tests = brittle. Refactor breaks tests even if behavior same.

## 15.7 ❌ Antipattern: Over-mocking

```rust
// ❌ Mock everything
let mut db = MockDb::new();
let mut http = MockHttp::new();
let mut cache = MockCache::new();
let mut logger = MockLogger::new();
let mut metrics = MockMetrics::new();
// Mock setup dominates test

// ✅ Use fakes / real for trusted dependencies
let svc = Service::new(InMemoryDb::new(), real_logger);
```

Mock-heavy tests test mocks, not code.

## 15.8 ❌ Antipattern: Test order dependency

```rust
static mut COUNT: u32 = 0;

#[test]
fn test_a() { unsafe { COUNT = 1; } }

#[test]
fn test_b() {
    unsafe { assert_eq!(COUNT, 1); }   // ❌ depends on test_a running first
}
```

Tests run in **parallel** + **random order**. Each test independent.

## 15.9 ❌ Antipattern: Time-sensitive tests

```rust
#[test]
fn test_timeout() {
    let start = Instant::now();
    sleep(Duration::from_millis(100));
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() >= 100);
    assert!(elapsed.as_millis() < 110);   // ❌ flaky on slow CI
}
```

Tight timing → flake. Use generous bounds or virtual time.

## 15.10 ❌ Antipattern: Production-like data

```rust
#[test]
fn test() {
    let user = User {
        // 50 fields with realistic data
    };
}
```

Test data should be **minimal**. Only fields relevant to test.

## 15.11 ❌ Antipattern: ignore for "broken" tests

```rust
#[test]
#[ignore = "broken, fix later"]
fn flaky_test() { ... }
```

Becomes permanent. Either fix or delete.

## 15.12 ❌ Antipattern: println! for debugging

```rust
#[test]
fn test() {
    let result = compute();
    println!("got: {:?}", result);   // ❌ silent unless --nocapture
    assert_eq!(result, expected);
}
```

If `println!` is for assertion context → use `assert_eq!`'s built-in failure output, or rich assertions.

If for debugging → use IDE debugger or `dbg!` macro.

```rust
#[test]
fn test() {
    let result = dbg!(compute());   // prints + returns value
    assert_eq!(result, expected);
}
```

---

# Tổng kết — 12 nguyên tắc senior

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Test pyramid: unit > integration > E2E.                       │
│                                                                  │
│ 2. Test BEHAVIOR, not implementation.                            │
│                                                                  │
│ 3. Each test independent + parallel-safe.                        │
│                                                                  │
│ 4. F.I.R.S.T. — Fast, Independent, Repeatable, Self-validating,  │
│    Timely.                                                       │
│                                                                  │
│ 5. Test error paths! API contract = success + error.             │
│                                                                  │
│ 6. Mocking: prefer fakes (real-like) over strict mocks.          │
│                                                                  │
│ 7. proptest cho edge cases khó tưởng tượng.                      │
│                                                                  │
│ 8. insta snapshot cho output phức tạp.                           │
│                                                                  │
│ 9. fuzz parsers / unsafe / security-critical code.               │
│                                                                  │
│ 10. async test với #[tokio::test] + virtual time.                │
│                                                                  │
│ 11. Coverage 70-85% target. Quality > quantity.                  │
│                                                                  │
│ 12. CI: fmt + clippy + test + miri (cho unsafe) + coverage.      │
└──────────────────────────────────────────────────────────────────┘
```

---

# Senior test toolkit

| Crate / Tool | Purpose |
|--------------|---------|
| `proptest` | Property-based testing |
| `quickcheck` | Alternative property-based (older) |
| `mockall` | Mock objects |
| `insta` | Snapshot testing |
| `cargo-fuzz` | Fuzz testing |
| `criterion` | Benchmarks |
| `cargo-llvm-cov` | Coverage |
| `cargo-nextest` | Modern test runner |
| `testcontainers` | Docker containers for tests |
| `pretty_assertions` | Better assert_eq output |
| `approx` | Float comparison |
| `serial_test` | Serial test execution |
| `loom` | Concurrent correctness |
| `tokio-test` | Async test utilities |
| `arbitrary` | Structured fuzzing |
| `dbg!` | Print + return |

---

# Lộ trình tiếp theo

Bạn đã có 15 chủ đề:

```
1-14. (memory-model, ownership, trait, generic, closure, async,
       error-handling, macros, smart-pointers, lifetime, performance,
       observability, iterator, unsafe-rust)
15. testing            ← MỚI
```

Còn các topic thực hành:

- **Web framework realistic** — axum project apply 15 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
