# Testing Rust — Minh Hoạ Trực Quan

> Companion visual cho [testing.md](./testing.md). Đọc song song.

---

## 1. Bức tranh lớn — Testing Universe

```
                          TESTING TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   BUILT-IN: #[test], cargo test, doc tests             │
       │                                                        │
       │   ┌──────────────┐  ┌──────────────┐  ┌─────────────┐  │
       │   │ Unit tests   │  │ Integration  │  │ Doc tests   │  │
       │   │ #[cfg(test)] │  │ tests/       │  │ /// ``` ``` │  │
       │   │ in src/      │  │ folder       │  │ in /// docs │  │
       │   └──────────────┘  └──────────────┘  └─────────────┘  │
       │                                                        │
       │   POWER-UPS (crates):                                  │
       │                                                        │
       │   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
       │   │ mockall  │ │ proptest │ │  insta   │ │cargo-fuzz│  │
       │   │ mocks    │ │ property │ │ snapshot │ │  fuzz    │  │
       │   └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
       │                                                        │
       │   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
       │   │criterion │ │llvm-cov  │ │  loom    │ │testcont. │  │
       │   │ bench    │ │ coverage │ │ concur.  │ │ Docker   │  │
       │   └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
       │                                                        │
       │   Triết lý: test BEHAVIOR, not implementation         │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Test Pyramid

```
                              ▲
                             / \
                            /   \         E2E / System tests
                           /     \        ~ 5-10%
                          /  E2E  \       SLOW, FRAGILE
                         /─────────\      Browser, real DB
                        /           \
                       /             \    Integration tests
                      /  Integration  \   ~ 15-25%
                     /─────────────────\  MODERATE
                    /                   \ Module boundaries
                   /                     \
                  /                       \   Unit tests
                 /      Unit tests          \  ~ 70-80%
                /                            \ FAST
               /───────────────────────────────\ Function level
              
   ┌─────────────────────────────────────────────────┐
   │  Time per test:                                 │
   │   Unit:        < 10ms                           │
   │   Integration: 10-1000ms                        │
   │   E2E:         > 1s                             │
   │                                                 │
   │  Invest most in UNIT — feedback nhanh, debug dễ │
   └─────────────────────────────────────────────────┘
```

---

## 3. Test organization

```
   my-crate/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs                  ← Unit tests here
   │   │   ┌──────────────────────────────────────┐
   │   │   │ pub fn foo() { ... }                 │
   │   │   │                                      │
   │   │   │ #[cfg(test)]    ← only in test build │
   │   │   │ mod tests {                          │
   │   │   │     use super::*;                    │
   │   │   │     #[test]                          │
   │   │   │     fn test_foo() {                  │
   │   │   │         // Access PRIVATE items OK   │
   │   │   │     }                                │
   │   │   │ }                                    │
   │   │   └──────────────────────────────────────┘
   │   │
   │   └── handler.rs                              ← Unit tests here too
   │
   ├── tests/                       ← Integration tests
   │   ├── api.rs                   ← Each file = separate test crate
   │   ├── auth.rs                  ← Only PUBLIC API accessible
   │   └── common/                                ← Shared helpers
   │       └── mod.rs                             ← Not run as tests
   │                                                (because /mod.rs)
   │
   ├── benches/                     ← criterion benchmarks
   │   └── my_bench.rs
   │
   ├── examples/                    ← Runnable examples
   │   └── basic.rs
   │
   └── fuzz/                        ← cargo-fuzz targets
       └── fuzz_targets/
           └── parse.rs
```

---

## 4. Unit Test syntax

```
   ┌──────────────────────────────────────────────────────────┐
   │  // src/lib.rs                                           │
   │                                                          │
   │  pub fn add(a: i32, b: i32) -> i32 { a + b }             │
   │                                                          │
   │  #[cfg(test)]                                            │
   │  mod tests {                                             │
   │      use super::*;                                       │
   │                                                          │
   │      #[test]                                             │
   │      fn test_add_positive() {                            │
   │          assert_eq!(add(2, 3), 5);                       │
   │      }                                                   │
   │                                                          │
   │      #[test]                                             │
   │      #[should_panic(expected = "divide by zero")]        │
   │      fn test_divide_by_zero() {                          │
   │          divide(1, 0);                                   │
   │      }                                                   │
   │                                                          │
   │      #[test]                                             │
   │      #[ignore = "requires database"]                     │
   │      fn test_needs_db() { ... }                          │
   │                                                          │
   │      #[test]                                             │
   │      fn test_with_result() -> Result<(), Box<dyn Error>> │
   │      {                                                   │
   │          let n: i32 = "42".parse()?;                     │
   │          assert_eq!(n, 42);                              │
   │          Ok(())                                          │
   │      }                                                   │
   │  }                                                       │
   └──────────────────────────────────────────────────────────┘
   
   
   Run:
   ────
   cargo test                       # all
   cargo test test_add              # match name
   cargo test -- --nocapture        # show println
   cargo test -- --test-threads=1   # serial
   cargo test -- --ignored          # only ignored
```

---

## 5. Integration test layout

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │  tests/                                                     │
   │  ├── api.rs                    ← test crate "api"          │
   │  │   ┌─────────────────────────────────────┐                │
   │  │   │ use my_crate::*;                    │                │
   │  │   │                                     │                │
   │  │   │ #[test]                             │                │
   │  │   │ fn test_endpoint() {                │                │
   │  │   │     // Only PUBLIC API              │                │
   │  │   │ }                                   │                │
   │  │   └─────────────────────────────────────┘                │
   │  │                                                          │
   │  ├── auth.rs                   ← separate test crate        │
   │  │   ┌─────────────────────────────────────┐                │
   │  │   │ mod common;             ← shared    │                │
   │  │   │                                     │                │
   │  │   │ #[test]                             │                │
   │  │   │ fn test_auth() {                    │                │
   │  │   │     let env = common::setup();      │                │
   │  │   │ }                                   │                │
   │  │   └─────────────────────────────────────┘                │
   │  │                                                          │
   │  └── common/                   ← SHARED HELPERS             │
   │      └── mod.rs                ← /mod.rs NOT test crate     │
   │          ┌─────────────────────────────────────┐            │
   │          │ pub fn setup() -> TestEnv { ... }   │            │
   │          └─────────────────────────────────────┘            │
   │                                                             │
   │  ⚠️ tests/common.rs (without /mod.rs) WOULD be run!         │
   │  ⚠️ Each integration test file = SEPARATE compile unit     │
   │     Independent test crates                                 │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
```

---

## 6. Doc Tests

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  /// Adds two numbers.                                       │
   │  ///                                                         │
   │  /// # Examples                                              │
   │  ///                                                         │
   │  /// ```                          ← runnable doc test       │
   │  /// use my_crate::add;                                      │
   │  /// let result = add(2, 3);                                 │
   │  /// assert_eq!(result, 5);                                  │
   │  /// ```                                                     │
   │  pub fn add(a: i32, b: i32) -> i32 {                         │
   │      a + b                                                   │
   │  }                                                           │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   
   Doc test attributes:
   ────────────────────
   
   ```ignore      Don't run (e.g., requires external resource)
   ```no_run      Compiles but doesn't run (e.g., side effects)
   ```compile_fail Should FAIL to compile (test that error happens)
   ```should_panic Should panic
   ```text        Not Rust, just text
   
   
   Hide setup with #:
   ──────────────────
   
   /// ```
   /// # use my_crate::*;          ← hidden in rendered docs
   /// # let env = TestEnv::new(); ← but compiled & run
   /// env.run("foo");
   /// assert_eq!(env.output(), "foo result");
   /// ```
   
   
   ⟹ Documentation always correct
   ⟹ Mỗi public function nên có doc test
```

---

## 7. Assertions hierarchy

```
   ┌──────────────────────────────────────────────────────────┐
   │  Built-in:                                               │
   │                                                          │
   │  assert!(condition)                                      │
   │  assert!(condition, "msg with {}", arg)                  │
   │                                                          │
   │  assert_eq!(actual, expected)                            │
   │  assert_eq!(a, b, "context: {}", c)                      │
   │                                                          │
   │  assert_ne!(a, b)                                        │
   │                                                          │
   │  debug_assert!(condition)        ← only in debug build   │
   │  debug_assert_eq!(a, b)                                  │
   │  debug_assert_ne!(a, b)                                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  Better output (pretty_assertions crate):                │
   │                                                          │
   │  use pretty_assertions::{assert_eq, assert_ne};          │
   │                                                          │
   │  → Diff output (colored), great for big structs          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  Float (approx crate):                                   │
   │                                                          │
   │  use approx::assert_relative_eq;                         │
   │  assert_relative_eq!(0.1 + 0.2, 0.3);   ← OK             │
   │  // assert_eq!(0.1 + 0.2, 0.3) WOULD FAIL                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  Custom helpers:                                         │
   │                                                          │
   │  fn assert_email_valid(email: &str) {                    │
   │      assert!(email.contains('@'), "missing @: {}", email);│
   │      ...                                                  │
   │  }                                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. Mocking landscape — Test doubles

```
   ┌─────────────┬──────────────────────────────────────────┐
   │ Type        │ What                                     │
   ├─────────────┼──────────────────────────────────────────┤
   │ DUMMY       │ Placeholder, never actually used         │
   │             │ e.g., null parameter                     │
   ├─────────────┼──────────────────────────────────────────┤
   │ STUB        │ Returns canned answers                   │
   │             │ "always return Ok(...)"                  │
   ├─────────────┼──────────────────────────────────────────┤
   │ SPY         │ Stub + records calls                     │
   │             │ "verify it was called with X"            │
   ├─────────────┼──────────────────────────────────────────┤
   │ MOCK        │ Pre-programmed expectations              │
   │             │ "must be called with X, then Y, returns Z"│
   │             │ → mockall                                │
   ├─────────────┼──────────────────────────────────────────┤
   │ FAKE        │ Working impl shortcut                    │
   │             │ e.g., InMemoryDb implementing Database   │
   │             │ → SENIOR PREFERRED                       │
   └─────────────┴──────────────────────────────────────────┘
   
   
   Diagram:
   ────────
   
   Production:
   ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ Service  │──► │ Database │──► │   DB     │
   │  (test)  │    │  trait   │    │ Postgres │
   └──────────┘    └──────────┘    └──────────┘
                       ▲
                       │ trait impl
                       │
   Testing options:    │
   ─────────────────   │
                       ├── MockDatabase (mockall — strict)
                       │   "expect this call exactly"
                       │
                       ├── StubDatabase (manual — simple)
                       │   "always returns ..."
                       │
                       └── InMemoryDatabase (fake — preferred)
                           "real query logic, just in-memory"
```

---

## 9. mockall flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  #[automock]                                             │
   │  trait Database {                                        │
   │      fn query_user(&self, name: &str) -> Result<User>;   │
   │  }                                                       │
   │                                                          │
   │  // mockall AUTO-GENERATES MockDatabase                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Test setup:                                             │
   │                                                          │
   │  #[test]                                                 │
   │  fn test_login() {                                       │
   │      let mut mock = MockDatabase::new();                 │
   │                                                          │
   │      mock.expect_query_user()                            │
   │          .with(eq("alice"))           ← match condition  │
   │          .times(1)                    ← exact count      │
   │          .returning(|_| Ok(User {     ← return value     │
   │              id: 1,                                      │
   │              name: "alice".into()                        │
   │          }));                                            │
   │                                                          │
   │      let svc = UserService::new(mock);                   │
   │      let result = svc.login("alice");                    │
   │      assert!(result.is_ok());                            │
   │                                                          │
   │      // On drop, mockall VERIFIES expectations met       │
   │  }                                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Match conditions:
   ─────────────────
   
   .with(eq("alice"))                ← equal
   .with(gt(5))                       ← greater than
   .with(always())                    ← match anything
   .with(predicate::function(         ← custom predicate
       |s: &str| s.starts_with("a")
   ))
   
   
   Verification:
   ─────────────
   
   .times(1)        ← exactly once
   .times(2..)      ← at least 2
   .times(0..=5)    ← 0 to 5
   .never()         ← MUST NOT be called
```

---

## 10. proptest — Property-based

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Example-based testing:                                  │
   │  ──────────────────────                                  │
   │  assert_eq!(reverse("hello"), "olleh");                  │
   │  assert_eq!(reverse(""), "");                            │
   │  ⟹ Misses edge cases you don't think of!                 │
   │                                                          │
   │  Property-based testing:                                 │
   │  ───────────────────────                                 │
   │  proptest! {                                             │
   │      #[test]                                             │
   │      fn reverse_twice_identity(s: String) {              │
   │          let r = reverse(&reverse(&s));                  │
   │          prop_assert_eq!(r, s);                          │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   │  ⟹ proptest GENERATES random strings                     │
   │  ⟹ Tests property for 256+ inputs                        │
   │  ⟹ If fails, SHRINKS to minimal example                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Shrinking visualization:
   ────────────────────────
   
   proptest tries: ["abc",  Some chinese 中文,  "long random...", ...]
                       │
                       │ Bug found at input "🦀🦀🦀abc中文 with newline\n"
                       │
                       ▼ Shrinking starts:
                       
   Try smaller variations:
       "🦀🦀🦀abc中文 with newline"       (remove \n)  ← still fails
       "🦀🦀abc中文 with newline"         (remove 🦀)
       "abc中文 with newline"
       "abc中文"
       "中文"                              ← still fails!
       "中"                                ← FAILS — minimal!
                                          │
                                          ▼
                          Report: bug at single Unicode char "中"
                          
   📌 Easier to debug minimal case than original noise.
```

---

## 11. proptest patterns

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Pattern 1: Round-trip                                   │
   │  ─────────────────────                                   │
   │  proptest! {                                             │
   │      #[test]                                             │
   │      fn encode_decode_roundtrip(data: Vec<u8>) {         │
   │          let encoded = encode(&data);                    │
   │          let decoded = decode(&encoded).unwrap();        │
   │          prop_assert_eq!(decoded, data);                 │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Pattern 2: Invariant                                    │
   │  ────────────────────                                    │
   │  proptest! {                                             │
   │      #[test]                                             │
   │      fn sort_preserves_length(mut v: Vec<i32>) {         │
   │          let original = v.len();                         │
   │          v.sort();                                       │
   │          prop_assert_eq!(v.len(), original);             │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Pattern 3: Idempotency                                  │
   │  ───────────────────────                                 │
   │  proptest! {                                             │
   │      #[test]                                             │
   │      fn parse_format_idempotent(input: String) {         │
   │          if let Ok(v) = parse(&input) {                  │
   │              let formatted = format(&v);                 │
   │              let v2 = parse(&formatted).unwrap();        │
   │              prop_assert_eq!(v, v2);                     │
   │          }                                               │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Pattern 4: Cross-check (oracle)                         │
   │  ───────────────────────────────                         │
   │  proptest! {                                             │
   │      #[test]                                             │
   │      fn my_sort_matches_std(mut v: Vec<i32>) {           │
   │          let mut std_sorted = v.clone();                 │
   │          std_sorted.sort();                              │
   │          my_sort(&mut v);                                │
   │          prop_assert_eq!(v, std_sorted);                 │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 12. insta — Snapshot testing

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Problem: big complex output                             │
   │  ────────────────────────────                            │
   │                                                          │
   │  let html = render(input);                               │
   │  assert_eq!(html, "<div>...very long...</div>");  ← ugly │
   │                                                          │
   │                                                          │
   │  Solution: snapshot                                      │
   │  ─────────────────                                       │
   │                                                          │
   │  insta::assert_yaml_snapshot!(rendered_value);           │
   │                                                          │
   │  ⟹ First run: creates tests/snapshots/test_name.snap     │
   │  ⟹ Next runs: compare to snapshot                         │
   │  ⟹ Diff if mismatch                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Workflow:
   ─────────
   
            cargo test
                 │
                 ▼
        ┌─────────────────┐
        │ First run?      │
        │ Snapshot exists?│
        └────┬───────┬────┘
             │       │
            NO      YES
             │       │
             ▼       ▼
        Create   Compare
        snapshot to existing
             │       │
             │  ┌────┴────┐
             │ Match    Mismatch
             │   │         │
             │   ▼         ▼
             │  Pass    .snap.new file
             ▼           created
        Manual review
                         │
                         ▼
                    cargo insta review
                         │
                  ┌──────┴──────┐
                  │             │
              Accept           Reject
              (update snap)   (keep old)
   
   
   File structure:
   ───────────────
   
   tests/
   └── snapshots/
       ├── test_render.snap         ← committed
       └── test_render.snap.new     ← uncommitted, after change
```

---

## 13. cargo-fuzz — Fuzz testing

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Fuzzing: random / malformed inputs → find crashes       │
   │                                                          │
   │  Most effective for:                                     │
   │   • Parsers (JSON, HTTP, image, audio)                   │
   │   • Decoders                                             │
   │   • Unsafe code (memory bugs)                            │
   │   • Security-critical code                               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Setup:
   ──────
   
   $ cargo install cargo-fuzz
   $ cargo fuzz init
   $ cargo fuzz add parse_json
   
   ⟹ Creates fuzz/fuzz_targets/parse_json.rs
   
   
   Target:
   ───────
   
   #![no_main]
   use libfuzzer_sys::fuzz_target;
   
   fuzz_target!(|data: &[u8]| {
       if let Ok(s) = std::str::from_utf8(data) {
           let _ = my_crate::parse_json(s);
           // Should NOT panic / hang / UB on any input
       }
   });
   
   
   Run:
   ────
   
   $ cargo +nightly fuzz run parse_json
   
   ⟹ Generates inputs, mutates, runs forever
   ⟹ Coverage-guided (libFuzzer)
   ⟹ Crashes saved to fuzz/artifacts/parse_json/
   
   
   Structured fuzzing với arbitrary:
   ─────────────────────────────────
   
   #[derive(Arbitrary, Debug)]
   struct Input {
       name: String,
       age: u32,
       items: Vec<u8>,
   }
   
   fuzz_target!(|input: Input| {
       process(input);
   });
   
   ⟹ Fuzzer generates TYPED inputs → more meaningful
   
   
   Fuzzing flow:
   ─────────────
   
            Random bytes
                 │
                 ▼
        ┌─────────────────┐
        │  Run target     │
        │  Track coverage │
        └────┬───────┬────┘
             │       │
          Pass    Crash/hang
             │       │
             ▼       ▼
        Mutate    Save to
        for next  artifacts/
        round       │
                    ▼
                Reproduce
                bug case
```

---

## 14. Async testing

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  ❌ #[test] can't be async                              │
   │                                                          │
   │  #[test]                                                 │
   │  async fn test() {  // ERROR                            │
   │      ...                                                 │
   │  }                                                       │
   │                                                          │
   │  ✅ Use #[tokio::test]                                  │
   │                                                          │
   │  #[tokio::test]                                          │
   │  async fn test() {                                       │
   │      let result = async_fn().await;                      │
   │      assert_eq!(result, expected);                       │
   │  }                                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Variants:
   ─────────
   
   #[tokio::test]                              # default current_thread
   #[tokio::test(flavor = "multi_thread")]     # multi-thread
   #[tokio::test(start_paused = true)]         # virtual time
   
   
   Virtual time:
   ─────────────
   
   #[tokio::test(start_paused = true)]
   async fn test_timeout() {
       let task = tokio::spawn(async {
           tokio::time::sleep(Duration::from_secs(60)).await;
           "done"
       });
       
       // Skip ahead 60s — INSTANT, no real wait!
       tokio::time::advance(Duration::from_secs(60)).await;
       
       assert_eq!(task.await.unwrap(), "done");
   }
   
   ⟹ Test timing-dependent code without slow real waits
   
   
   loom — Exhaustive concurrent testing:
   ──────────────────────────────────────
   
   #[test]
   fn test() {
       loom::model(|| {
           // Loom explores ALL possible thread interleavings
           let m = Arc::new(loom::sync::Mutex::new(0));
           let m1 = Arc::clone(&m);
           
           let t = loom::thread::spawn(move || {
               *m1.lock().unwrap() = 1;
           });
           
           *m.lock().unwrap() = 2;
           t.join().unwrap();
       });
   }
   
   $ RUSTFLAGS="--cfg loom" cargo test
   
   ⟹ Find rare race conditions
   ⟹ Slow but THOROUGH for unsafe/lock-free
```

---

## 15. Coverage workflow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  $ cargo install cargo-llvm-cov                          │
   │                                                          │
   │  $ cargo llvm-cov                                        │
   │                                                          │
   │  Output:                                                 │
   │  ┌────────────────────────────────────────────────────┐  │
   │  │ Filename       Regions  Missed  Cover  Lines  Cover│  │
   │  │ src/lib.rs     100      5       95%    50     98%  │  │
   │  │ src/handler.rs 234      45      80%    100    85%  │  │
   │  │ src/utils.rs   50       50      0%     30     0%   │  │  ← write tests!
   │  └────────────────────────────────────────────────────┘  │
   │                                                          │
   │  $ cargo llvm-cov --html                                 │
   │  $ open target/llvm-cov/html/index.html                  │
   │                                                          │
   │  HTML report — drill into files:                         │
   │  ────────────────────────────                            │
   │  Green lines: covered                                    │
   │  Red lines: not covered  ← write tests for these         │
   │  Yellow: partial branch coverage                         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Coverage in CI:
   ───────────────
   
   $ cargo llvm-cov --lcov --output-path lcov.info
   $ # Upload to Codecov / Coveralls
   
   ⟹ Track coverage trend over time
   ⟹ Block PRs that drop coverage
   
   
   ⚠️ Coverage caveats:
   ────────────────────
   
   • 100% coverage ≠ no bugs (just means executed)
   • Low coverage ≠ broken
   • Focus on BUSINESS LOGIC + ERROR PATHS
   • Target: 70-85% line coverage
   • Quality > quantity
```

---

## 16. CI pipeline visualization

```
   ┌──────────────────────────────────────────────────────────┐
   │                  CI PIPELINE                             │
   │                                                          │
   │   ┌────────────────┐                                     │
   │   │ git push       │                                     │
   │   └────────┬───────┘                                     │
   │            │                                             │
   │            ▼                                             │
   │   ┌────────────────┐                                     │
   │   │ Trigger CI     │                                     │
   │   └────────┬───────┘                                     │
   │            │                                             │
   │            ▼                                             │
   │   ┌────────────────────────────────────────┐             │
   │   │ Quality gates (parallel)               │             │
   │   │                                        │             │
   │   │  cargo fmt --check        ────────► ✅│             │
   │   │  cargo clippy -D warnings ────────► ✅│             │
   │   │  cargo build              ────────► ✅│             │
   │   └────────┬───────────────────────────────┘             │
   │            │                                             │
   │            ▼                                             │
   │   ┌────────────────────────────────────────┐             │
   │   │ Tests (parallel)                       │             │
   │   │                                        │             │
   │   │  cargo test               ────────► ✅│             │
   │   │  cargo test --doc         ────────► ✅│             │
   │   │  cargo llvm-cov           ────────► ✅│             │
   │   │  cargo +nightly miri test ────────► ✅│ (unsafe)     │
   │   └────────┬───────────────────────────────┘             │
   │            │                                             │
   │            ▼                                             │
   │   ┌────────────────────────────────────────┐             │
   │   │ Benchmark (compare to baseline)        │             │
   │   │  cargo bench  ────────► no regression  │             │
   │   └────────┬───────────────────────────────┘             │
   │            │                                             │
   │            ▼                                             │
   │   ┌────────────────────────────────────────┐             │
   │   │ Deploy / Merge                         │             │
   │   └────────────────────────────────────────┘             │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Patterns matrix

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │  What                       │ Pattern                       │
   │  ────                       │ ───────                       │
   │                                                              │
   │  Test structure             │ Arrange-Act-Assert            │
   │                                                              │
   │  Test name                  │ <unit>_<scenario>_<expected>  │
   │                                                              │
   │  Multiple checks per test   │ Split into multiple tests     │
   │                                                              │
   │  Error path                 │ Test error AND success paths  │
   │                                                              │
   │  Test helpers               │ tests/common/mod.rs           │
   │                                                              │
   │  Setup/teardown             │ RAII fixture struct (Drop)    │
   │                                                              │
   │  External dependencies      │ Trait + Mock/Fake             │
   │                                                              │
   │  Random inputs              │ proptest                      │
   │                                                              │
   │  Complex output             │ insta snapshot                │
   │                                                              │
   │  Parsers / unsafe           │ cargo-fuzz                    │
   │                                                              │
   │  Async                      │ #[tokio::test]                │
   │                                                              │
   │  Time-dependent             │ Virtual time (tokio paused)   │
   │                                                              │
   │  Concurrent correctness     │ loom                          │
   │                                                              │
   │  Real DB                    │ testcontainers + Docker       │
   │                                                              │
   │  Benchmark                  │ criterion                     │
   │                                                              │
   │  Coverage                   │ cargo-llvm-cov                │
   │                                                              │
   │  CI                         │ fmt+clippy+test+coverage      │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 18. Antipatterns visualization

```
   ❌ 1. Test implementation details
   ─────────────────────────────────
   
   #[test]
   fn test_uses_hashmap() {
       let svc = Service::new();
       svc.handle(1);
       assert_eq!(svc.cache.len(), 1);   // private field!
   }
   
   ✅ Test observable behavior:
   #[test]
   fn test_repeated_calls_consistent() {
       let svc = Service::new();
       let r1 = svc.handle(1);
       let r2 = svc.handle(1);
       assert_eq!(r1, r2);
   }
   
   
   ❌ 2. Over-mocking
   ──────────────────
   
   let mut db = MockDb::new();
   let mut http = MockHttp::new();
   let mut cache = MockCache::new();
   let mut log = MockLogger::new();
   let mut metric = MockMetrics::new();
   // 30 lines mock setup
   // Testing mocks, not real logic!
   
   ✅ Use fakes for trusted deps:
   let svc = Service::new(
       InMemoryDb::new(),       ← fake, real query logic
       real_logger,
   );
   
   
   ❌ 3. Test order dependency
   ──────────────────────────
   
   static mut COUNT: u32 = 0;
   
   #[test] fn test_a() { unsafe { COUNT = 1; } }
   #[test] fn test_b() {
       unsafe { assert_eq!(COUNT, 1); }   // depends on test_a!
   }
   
   ⟹ Tests run PARALLEL + RANDOM ORDER
   ⟹ Each test must be INDEPENDENT
   
   
   ❌ 4. Time-sensitive
   ────────────────────
   
   #[test]
   fn test_timeout() {
       let start = Instant::now();
       sleep(Duration::from_millis(100));
       assert!(start.elapsed().as_millis() < 110);   // flaky on slow CI
   }
   
   ✅ Use virtual time (tokio paused) or generous bounds
   
   
   ❌ 5. #[ignore] = forgotten
   ──────────────────────────
   
   #[test]
   #[ignore = "broken, fix later"]
   fn flaky() { ... }
   
   ⟹ Becomes permanent technical debt
   ⟹ Either FIX or DELETE
   
   
   ❌ 6. Production data in tests
   ──────────────────────────────
   
   #[test]
   fn test() {
       let user = User {
           // 50 fields with realistic data
           // Most irrelevant to test
       };
   }
   
   ✅ Minimal test data via builder:
   let user = UserBuilder::new()
       .name("alice")
       .build();    // other fields = default
```

---

## 19. Senior toolkit

```
   ┌──────────────────────────────────────────────────────────────┐
   │                  TESTING ECOSYSTEM                           │
   │                                                              │
   │   BUILT-IN                  PROPERTY/EXAMPLE                 │
   │   ──────────                ────────────────                 │
   │   cargo test                proptest                         │
   │   #[test]                   quickcheck                       │
   │   assert_eq!                                                 │
   │   #[should_panic]                                            │
   │                                                              │
   │   MOCKING                   SNAPSHOT                         │
   │   ────────                  ────────                         │
   │   mockall                   insta                            │
   │   fakes (manual)            cargo insta review               │
   │                                                              │
   │   FUZZ                      BENCH                            │
   │   ────                      ─────                            │
   │   cargo-fuzz                criterion                        │
   │   arbitrary                 hyperfine                        │
   │                                                              │
   │   COVERAGE                  CONCURRENT                       │
   │   ─────────                 ──────────                       │
   │   cargo-llvm-cov            loom                             │
   │                             #[tokio::test]                   │
   │                             tokio::time::pause                │
   │                                                              │
   │   ASSERTIONS                INFRA                            │
   │   ───────────               ──────                           │
   │   pretty_assertions         testcontainers                   │
   │   approx                    serial_test                      │
   │                             dbg!                             │
   │                                                              │
   │   RUNNER                    UNSAFE                           │
   │   ───────                   ──────                           │
   │   cargo-nextest             miri                             │
   │                             sanitizers                       │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 20. Mind map cuối

```
                              TESTING
                                 │
        ┌────────────┬───────────┼───────────┬───────────────┐
        ▼            ▼           ▼           ▼               ▼
   STRUCTURE    KINDS         TOOLS       PATTERNS      ANTIPATTERNS
        │            │           │           │               │
   Pyramid       Unit         mockall     AAA           Implement detail
   F.I.R.S.T.    Integration  proptest    1 assert/test Over-mock
                 Doc          insta       Test errors   Order depend
                 Bench        cargo-fuzz  Fakes>mocks   Time-sensitive
                              criterion   Result<>      #[ignore] perm
                              llvm-cov    Builders      Prod data
                              loom        Round-trip
                              testcont.   Invariant
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. Test pyramid: unit > integ > e2e │
                │                                      │
                │  2. Test BEHAVIOR, not implementation│
                │                                      │
                │  3. F.I.R.S.T. principles            │
                │                                      │
                │  4. Test error paths!                │
                │                                      │
                │  5. Fakes > mocks                    │
                │                                      │
                │  6. proptest for edge cases          │
                │                                      │
                │  7. insta for complex output         │
                │                                      │
                │  8. Fuzz parsers / unsafe            │
                │                                      │
                │  9. Async: #[tokio::test] + virtual  │
                │     time                             │
                │                                      │
                │  10. Coverage 70-85% sensible target │
                │                                      │
                │  11. CI: fmt+clippy+test+cov+miri    │
                │                                      │
                │  12. Tests are FIRST-CLASS code      │
                │      — refactor, review, maintain    │
                └──────────────────────────────────────┘
```

---

## 21. Bộ tài liệu Rust giờ có 15 chủ đề

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
   │  12. observability           — Logs/Traces/Metrics      │
   │  13. iterator                — Iterator + Stream + Rayon │
   │  14. unsafe-rust             — Unsafe + FFI + Atomic    │
   │  15. testing                 — Testing patterns         │
   │      testing-visual          ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust production senior ĐẦY ĐỦ           │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Sau testing, các topic thực hành cuối:

- **Web framework realistic** — axum project apply 15 chủ đề vào dự án thực tế
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
