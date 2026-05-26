# WASM (WebAssembly) — Minh Hoạ Trực Quan

> Companion visual cho [t-wasm.md](./t-wasm.md). Đọc song song.

---

## 1. Bức tranh lớn — WASM Universe

```
                          WEBASSEMBLY UNIVERSE
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   Binary format · Stack VM · Sandboxed · Fast          │
       │                                                        │
       │   ┌────────────────────────────────────────────────┐    │
       │   │  Rust source code                              │    │
       │   │     │                                          │    │
       │   │     │ compile                                  │    │
       │   │     ▼                                          │    │
       │   │  .wasm (binary instructions)                   │    │
       │   │     │                                          │    │
       │   │     ▼                                          │    │
       │   │  Runtimes:                                     │    │
       │   │   • Browser (V8, SpiderMonkey, JavaScriptCore) │   │
       │   │   • Server (wasmtime, wasmer)                  │    │
       │   │   • Edge (Cloudflare, Fastly)                  │    │
       │   │   • Embedded (custom)                          │    │
       │   └────────────────────────────────────────────────┘    │
       │                                                        │
       │   Use cases:                                           │
       │   ┌────────────────┬────────────────┬─────────────┐   │
       │   │ Browser        │ Edge / Server  │ Plugin sys  │   │
       │   │ • Image proc   │ • Cloudflare   │ • Sandboxed │   │
       │   │ • Crypto       │   Workers      │   3rd party │   │
       │   │ • Games        │ • Fastly       │ • Multi-    │   │
       │   │ • SPA (Yew)    │ • Wasmtime     │   tenant    │   │
       │   └────────────────┴────────────────┴─────────────┘   │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. WASM VM architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   WASM Virtual Machine                                   │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ OPERAND STACK                                  │     │
   │   │   push, pop, arithmetic, control flow          │     │
   │   │   (stack-based VM, not register-based)         │     │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ LINEAR MEMORY (single contiguous byte array)   │     │
   │   │                                                │     │
   │   │   0  ........................................  ~4GB │
   │   │   │                                                │ │
   │   │   │ Globals (.data, .rodata)                       │ │
   │   │   │ Heap (managed by allocator, e.g., dlmalloc)    │ │
   │   │   │ Stack (Rust function locals)                   │ │
   │   │   │                                                │ │
   │   │   Accessed via i32 offset                           │ │
   │   │   Sandboxed (can't escape)                         │ │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ GLOBALS (typed constants/vars)                 │     │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ FUNCTION TABLE (indirect calls)                │     │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ IMPORTS (from host) + EXPORTS (to host)        │     │
   │   │   (functions, globals, memory, tables)         │     │
   │   └────────────────────────────────────────────────┘     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 3. JS vs WASM execution

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   JS:                                                    │
   │   ───                                                    │
   │   Source                                                 │
   │     ↓ parse + compile + JIT optimize                     │
   │   Bytecode → optimized native (multiple tiers)           │
   │     ↓                                                    │
   │   Execute                                                │
   │                                                          │
   │   • Slow startup (parse + initial JIT)                   │
   │   • Variable perf (deopt possible)                       │
   │   • GC pauses                                            │
   │   • Dynamic typing overhead                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WASM:                                                  │
   │   ─────                                                  │
   │   .wasm                                                  │
   │     ↓ decode + validate (fast!)                          │
   │   Module                                                 │
   │     ↓ instantiate                                        │
   │   Instance ready                                         │
   │     ↓ JIT compile to native                              │
   │   Execute                                                │
   │                                                          │
   │   • Fast startup (binary already compiled)               │
   │   • Predictable perf                                     │
   │   • No GC                                                │
   │   • Static typed                                         │
   │   • ~85-95% of native speed                              │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. Rust → WASM toolchain flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust source code                                       │
   │   ────────────                                           │
   │   src/lib.rs                                             │
   │     ↓                                                    │
   │   ┌──────────────────────────────────────────────────┐   │
   │   │ cargo build --target ...                         │   │
   │   │                                                  │   │
   │   │ Targets:                                         │   │
   │   │  • wasm32-unknown-unknown  (browser)             │   │
   │   │  • wasm32-wasip1            (WASI Preview 1)     │   │
   │   │  • wasm32-wasip2            (WASI Preview 2)     │   │
   │   └──────────────────────────────────────────────────┘   │
   │                       │                                  │
   │                       ▼                                  │
   │   ┌──────────────────────────────────────────────────┐   │
   │   │ .wasm file (target/.../release/myapp.wasm)       │   │
   │   └──────────────────────────────────────────────────┘   │
   │                       │                                  │
   │     ┌─────────────────┼─────────────────┐                │
   │     ▼                 ▼                 ▼                │
   │ Browser?         Server/Edge?       Bare WASM?           │
   │     │                 │                 │                │
   │     ▼                 ▼                 ▼                │
   │ wasm-bindgen     wasmtime/wasmer    Direct run         │
   │ wasm-pack        Cloudflare           (rare)             │
   │  ↓               Fastly                                 │
   │ pkg/             ↓                                       │
   │  ├ myapp.wasm   Run as standalone                       │
   │  ├ myapp.js                                             │
   │  └ myapp.d.ts                                           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 5. wasm-bindgen — JS ↔ Rust bridge

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust source (with #[wasm_bindgen]):                    │
   │                                                          │
   │   #[wasm_bindgen]                                        │
   │   pub fn greet(name: &str) -> String {                   │
   │       format!("Hello, {}!", name)                        │
   │   }                                                      │
   │                                                          │
   │   #[wasm_bindgen]                                        │
   │   extern "C" {                                           │
   │       #[wasm_bindgen(js_namespace = console)]            │
   │       fn log(s: &str);                                   │
   │   }                                                      │
   │                                                          │
   │   ↓ compile                                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   GENERATED OUTPUT:                                      │
   │                                                          │
   │   pkg/                                                   │
   │   ├── myapp_bg.wasm        ← actual WASM binary          │
   │   ├── myapp.js              ← JS glue code              │
   │   │     • init()                                         │
   │   │     • greet() wrapper                                │
   │   │     • String marshalling                             │
   │   ├── myapp.d.ts            ← TypeScript types          │
   │   └── package.json                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   USAGE IN JS:                                           │
   │                                                          │
   │   import init, { greet } from './pkg/myapp.js';          │
   │   await init();   // load WASM, instantiate              │
   │   console.log(greet('World'));   // "Hello, World!"      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 6. Marshalling: JS string ↔ Rust &str

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   JS side:                                               │
   │   ─────                                                  │
   │   greet("World")                                         │
   │     ↓                                                    │
   │   1. Encode JS UTF-16 to UTF-8                           │
   │   2. Allocate in WASM linear memory                      │
   │      (call __wbindgen_malloc(len))                       │
   │   3. Copy UTF-8 bytes to allocated region                │
   │   4. Pass (ptr, len) as two i32                          │
   │                                                          │
   │   ┌────────────────────────────────────────────┐         │
   │   │ Linear memory (Rust managed):              │         │
   │   │  [.....World\0........]                    │         │
   │   │   ^                                        │         │
   │   │   ptr (i32)                                │         │
   │   │   len (i32) = 5                            │         │
   │   └────────────────────────────────────────────┘         │
   │                                                          │
   │   Rust receives:  fn greet(name: &str)                   │
   │                   = (ptr, len) → reconstructed &str      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Rust returns String:                                   │
   │   ─────                                                  │
   │   format!("Hello, World!")                               │
   │     ↓                                                    │
   │   1. Allocate "Hello, World!" in linear memory           │
   │   2. Return (ptr, len) to JS                             │
   │                                                          │
   │   JS reads:                                              │
   │   3. Read bytes from (ptr, len)                          │
   │   4. Decode UTF-8 to JS String                           │
   │   5. Free WASM memory: __wbindgen_free(ptr, len)         │
   │                                                          │
   │   Cost: O(n) per call. For huge strings → batch or use   │
   │         direct memory access.                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. Marshalling performance hierarchy

```
   FASTEST → SLOWEST
   ─────────────────
   
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   i32, f64, bool, etc.                                   │
   │   ░░ ~ns       Pass directly. No conversion.            │
   │                                                          │
   │   &[u8], &[T] (slice)                                    │
   │   ░░░ ~ns      Pass ptr + len. Zero copy (view).         │
   │                                                          │
   │   String, Vec (owned)                                    │
   │   ░░░░░ ~µs    Alloc + copy bytes. Free after.           │
   │                                                          │
   │   Custom struct (via wasm-bindgen)                       │
   │   ░░░░░░ ~µs   Opaque handle. Method call per access.   │
   │                                                          │
   │   serde via serde-wasm-bindgen                           │
   │   ░░░░░░░░░ ~µs+   Recursive serialize.                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Optimization:
   ─────────────
   • Primitives free → use for hot path
   • Slices over Vec when don't need ownership
   • Bulk operations over many small calls
   • Direct memory access for huge data
   
   ❌ Per-pixel call from JS to WASM = ms-level slow
   ✅ Pass whole image as &[u8], process in WASM, return
```

---

## 8. DOM access pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust (web-sys):                                        │
   │                                                          │
   │   use web_sys::*;                                        │
   │                                                          │
   │   #[wasm_bindgen]                                        │
   │   pub fn render() -> Result<(), JsValue> {               │
   │       let window = web_sys::window().unwrap();           │
   │       let document = window.document().unwrap();         │
   │       let body = document.body().unwrap();               │
   │                                                          │
   │       let div = document.create_element("div")?;         │
   │       div.set_text_content(Some("Hello!"));              │
   │       body.append_child(&div)?;                          │
   │                                                          │
   │       Ok(())                                             │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Behind the scenes (each call):                         │
   │                                                          │
   │   Rust ──── JsValue handle ────► JS                     │
   │                                                          │
   │   web-sys generates wrappers:                            │
   │     document.create_element("div")                       │
   │       ↓                                                  │
   │     extern "C" fn __wbg_createElement_xxx(...) -> u32;   │
   │       ↓                                                  │
   │     JS side: this.createElement(args)                    │
   │                                                          │
   │   Each method = boundary crossing.                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ❌ ANTIPATTERN — many DOM crossings:                   │
   │                                                          │
   │   for i in 0..1000 {                                     │
   │       let el = document.create_element("div")?;          │
   │       body.append_child(&el)?;                           │
   │       // 2000 boundary crossings!                        │
   │   }                                                      │
   │                                                          │
   │   ✅ BETTER — batch in JS:                                │
   │                                                          │
   │   #[wasm_bindgen]                                        │
   │   pub fn generate_html(n: usize) -> String {             │
   │       (0..n).map(|i| format!("<div>{}</div>", i))        │
   │              .collect()                                  │
   │   }                                                      │
   │                                                          │
   │   // JS: body.innerHTML = wasm.generate_html(1000);      │
   │   // 1 boundary call. ~1000x faster.                     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. Async / Promise interop

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust async fn → JS Promise                             │
   │                                                          │
   │   #[wasm_bindgen]                                        │
   │   pub async fn fetch_data() -> Result<String, JsValue> { │
   │       let resp = JsFuture::from(                         │
   │           web_sys::window().unwrap()                     │
   │               .fetch_with_str("/api/data")               │
   │       ).await?;                                          │
   │                                                          │
   │       let r: Response = resp.dyn_into()?;                │
   │       let text = JsFuture::from(r.text()?).await?;       │
   │       Ok(text.as_string().unwrap())                      │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   JS uses as normal Promise:                             │
   │                                                          │
   │   const data = await fetch_data();                       │
   │   console.log(data);                                     │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Underlying mechanism:                                  │
   │                                                          │
   │   Rust async fn   →   compiled state machine             │
   │                       (chapter f)                         │
   │                                                          │
   │   JsFuture::from(promise)   →   poll() returns Pending   │
   │                                  until JS Promise resolves│
   │                                                          │
   │   wasm-bindgen-futures::spawn_local(future)              │
   │     →   schedule via JS microtask queue                  │
   │                                                          │
   │   No tokio runtime — uses browser event loop directly    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. Web Worker — WASM in background thread

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   PROBLEM: WASM on main thread blocks UI                 │
   │                                                          │
   │     Main thread:                                         │
   │     ──────────────                                        │
   │     Render frame                                         │
   │       │ scheduler                                        │
   │       ▼                                                  │
   │     User input                                            │
   │       │                                                  │
   │       ▼                                                  │
   │     ┌─────────────────────────────┐                       │
   │     │ WASM compute (heavy: 100ms) │ ← blocks frame!     │
   │     └─────────────────────────────┘                       │
   │       │                                                  │
   │       ▼                                                  │
   │     Render (delayed)                                     │
   │       │                                                  │
   │     UI freezes during WASM work                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   SOLUTION: Web Worker                                   │
   │                                                          │
   │     Main thread:           Worker thread:                │
   │     ──────────────         ──────────────                │
   │     UI fluid               WASM loaded                   │
   │       │                       │                          │
   │       │ postMessage           │                          │
   │       ├──────────────────────►│                          │
   │       │                       │                          │
   │       │                       │ heavy compute            │
   │       │ (UI continues)         │                         │
   │       │                       │ (~100ms)                 │
   │       │                       │                          │
   │       │   postMessage         │                          │
   │       │◄──────────────────────│                          │
   │       │ result                │                          │
   │       ▼                                                  │
   │     Update UI                                            │
   │                                                          │
   │   ⟹ UI never blocks                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Code:                                                  │
   │                                                          │
   │   // main.js                                             │
   │   const worker = new Worker('worker.js',                  │
   │       { type: 'module' });                                │
   │   worker.onmessage = (e) => console.log(e.data);          │
   │   worker.postMessage({ input: [...big data...] });        │
   │                                                          │
   │   // worker.js                                           │
   │   import init, { compute } from './pkg/myapp.js';        │
   │   await init();                                          │
   │   self.onmessage = (e) => {                              │
   │       const result = compute(e.data.input);              │
   │       self.postMessage(result);                          │
   │   };                                                     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 11. WASI — Server-side WASM

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Browser WASM (wasm32-unknown-unknown):                 │
   │   ───────────────────────────                            │
   │   • No std I/O                                            │
   │   • No file system                                       │
   │   • No network (use fetch via JS)                        │
   │   • No env vars                                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WASI (wasm32-wasip1 / wasip2):                         │
   │   ───────────────────────                                │
   │   • Full std::io                                          │
   │   • File system (preopened dirs)                         │
   │   • Network (preview 2)                                  │
   │   • Env vars                                             │
   │   • Time, random                                          │
   │                                                          │
   │   Built on capability-based security:                    │
   │   ────────────────────────────────────                   │
   │                                                          │
   │   $ wasmtime run --dir=. myapp.wasm                      │
   │                ────────                                  │
   │                Grant access to current dir.              │
   │                Without --dir, no FS access.              │
   │                                                          │
   │   $ wasmtime run --env=DEBUG=1 myapp.wasm                │
   │                                                          │
   │   $ wasmtime run --tcplisten=8080 myapp.wasm             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Code (same as native Rust):                            │
   │                                                          │
   │   fn main() -> std::io::Result<()> {                     │
   │       let content = std::fs::read_to_string("data.txt")?;│
   │       println!("Got: {}", content);                      │
   │                                                          │
   │       for arg in std::env::args() {                      │
   │           println!("Arg: {}", arg);                      │
   │       }                                                  │
   │                                                          │
   │       Ok(())                                             │
   │   }                                                      │
   │                                                          │
   │   $ cargo build --target wasm32-wasip1 --release         │
   │   $ wasmtime run --dir=. target/.../myapp.wasm           │
   │                                                          │
   │   ⟹ Runs same WASM on Mac, Linux, Windows,               │
   │     Cloudflare, Fastly, AWS Lambda (with adapter)        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 12. Edge computing architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TRADITIONAL CLOUD:                                     │
   │   ─────────────────                                      │
   │   User (Vietnam)                                         │
   │     │                                                    │
   │     │ ~200ms DNS + connection                            │
   │     ▼                                                    │
   │   Load balancer (US-East region)                         │
   │     │                                                    │
   │     │ ~10ms                                              │
   │     ▼                                                    │
   │   Server (US-East)                                       │
   │     │                                                    │
   │     │ ~5ms to DB                                         │
   │     ▼                                                    │
   │   Database (US-East)                                     │
   │                                                          │
   │   Total: ~220ms round-trip                               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   EDGE WITH WASM:                                        │
   │   ──────────────                                         │
   │   User (Vietnam)                                         │
   │     │                                                    │
   │     │ ~10ms to nearest edge node (Singapore)             │
   │     ▼                                                    │
   │   Edge node (Singapore — Cloudflare/Fastly)              │
   │     │                                                    │
   │     │ ~1ms WASM cold start                               │
   │     ▼                                                    │
   │   WASM module runs HERE                                  │
   │     │                                                    │
   │     │ Maybe fetch from KV (local)                         │
   │     │ Or forward to origin (rarely)                       │
   │     ▼                                                    │
   │   Response                                               │
   │                                                          │
   │   Total: ~15-30ms                                        │
   │                                                          │
   │   ⟹ Much faster for global users                         │
   │   ⟹ Cold start < 10ms (vs containers ~100ms-seconds)     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Why WASM perfect for edge:
   ──────────────────────────
   
   ✅ Tiny binaries (KB) → fast load
   ✅ Fast cold start (ms)
   ✅ Sandboxed → multi-tenant safe
   ✅ Capability-based → fine-grained perms
   ✅ Language-agnostic (Rust, Go, JS, ...)
   ✅ Portable (same .wasm everywhere)
```

---

## 13. Cloudflare Workers in Rust

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust Worker:                                           │
   │                                                          │
   │   use worker::*;                                         │
   │                                                          │
   │   #[event(fetch)]                                        │
   │   async fn main(                                          │
   │       req: Request,                                      │
   │       env: Env,                                          │
   │       _ctx: Context,                                     │
   │   ) -> Result<Response> {                                │
   │       Router::new()                                      │
   │           .get_async("/api/users/:id", |_, ctx| async move {│
   │               let id = ctx.param("id").unwrap();          │
   │               let kv = ctx.env.kv("USERS")?;             │
   │               let user = kv.get(&id).json::<User>().await?;│
   │               Response::from_json(&user)                 │
   │           })                                             │
   │           .run(req, env)                                 │
   │           .await                                         │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Deploy:                                                │
   │                                                          │
   │   $ npx wrangler init my-worker --lang=rust              │
   │   $ npx wrangler deploy                                  │
   │                                                          │
   │   Result: deployed to 300+ Cloudflare edge locations     │
   │           globally.                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Cloudflare Storage Options:                            │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ KV       — eventually consistent KV store    │       │
   │   │            edge-distributed                  │       │
   │   │                                              │       │
   │   │ D1       — SQLite at edge, replicated         │      │
   │   │            ACID transactions                 │       │
   │   │                                              │       │
   │   │ R2       — S3-compatible object storage      │       │
   │   │                                              │       │
   │   │ Durable  — Single-threaded actors with       │       │
   │   │ Objects    persistent state                  │       │
   │   │                                              │       │
   │   │ Queues   — Async message processing          │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. Component Model — Future

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   WASM Core:                                             │
   │   ──────────                                             │
   │   • Stack VM + linear memory + imports/exports          │
   │   • Limited types (i32, i64, f32, f64)                  │
   │   • Strings/structs via manual marshalling              │
   │                                                          │
   │   Component Model (Preview 2):                           │
   │   ─────────────────────────                              │
   │   • Higher-level type system                             │
   │   • Records, variants, lists, options, tuples            │
   │   • Resources (handles to opaque data)                   │
   │   • Cross-language composition (no glue code)            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WIT (WebAssembly Interface Types):                     │
   │                                                          │
   │   // calculator.wit                                      │
   │   package my:calc;                                       │
   │                                                          │
   │   interface calc {                                       │
   │       record point {                                     │
   │           x: f64, y: f64,                                │
   │       }                                                  │
   │                                                          │
   │       distance: func(a: point, b: point) -> f64;         │
   │   }                                                      │
   │                                                          │
   │   world component {                                       │
   │       export calc;                                       │
   │   }                                                      │
   │                                                          │
   │   ↓ wit-bindgen generates Rust/Go/Python/JS bindings     │
   │                                                          │
   │   Universal interface — any language consumes.           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Use cases:                                             │
   │                                                          │
   │   • Plugin systems: load 3rd-party safely                 │
   │   • Polyglot apps: Rust core + Python ML + JS UI          │
   │   • Cross-org components: standardized interfaces        │
   │   • Universal modules: single .wasm runs anywhere        │
   │                                                          │
   │   Status 2024:                                           │
   │   • ✅ Standardized (Preview 2)                          │
   │   • ✅ wasmtime supports                                 │
   │   • ⚠️ Browser support: limited                          │
   │   • ⚠️ Less mature than wasm-bindgen for browser         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 15. Performance optimization stack

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. RUST COMPILE PROFILE                                │
   │   ─────────────────────                                  │
   │                                                          │
   │   [profile.release]                                      │
   │   opt-level = "z"       # smallest                       │
   │   lto = true            # link-time opt                  │
   │   codegen-units = 1     # max optimization                │
   │   panic = "abort"       # smaller binary                  │
   │   strip = true          # remove symbols                  │
   │                                                          │
   │   Effect: 500KB → 200KB                                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. wasm-opt POSTPROCESS                                 │
   │                                                          │
   │   $ wasm-opt -Oz input.wasm -o output.wasm               │
   │                                                          │
   │   Aggressive size optimization (binaryen).               │
   │   Often -Oz > -Os > -O3 for binary size.                 │
   │                                                          │
   │   Effect: 200KB → 100KB                                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. SMALLER ALLOCATOR                                   │
   │                                                          │
   │   #[global_allocator]                                    │
   │   static ALLOC: wee_alloc::WeeAlloc = ...;                │
   │                                                          │
   │   Effect: -10KB binary                                   │
   │   Trade-off: slightly slower allocs                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   4. MINIMIZE WEB-SYS FEATURES                           │
   │                                                          │
   │   web-sys = { version = "0.3", features = [              │
   │       "console", "Window"   # only what you need!         │
   │   ]}                                                     │
   │                                                          │
   │   Don't enable "all" features — bloats binary.           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   5. SIMD (if applicable)                                │
   │                                                          │
   │   RUSTFLAGS='-C target-feature=+simd128'                 │
   │   cargo build --target wasm32-unknown-unknown            │
   │                                                          │
   │   4x speedup for numeric loops.                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   6. MINIMIZE JS BOUNDARY CROSSINGS                      │
   │                                                          │
   │   Batch operations.                                      │
   │   Use slices for bulk data.                              │
   │   Avoid per-element JS calls.                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 16. Web app framework comparison

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Framework  │ Style         │ Notes                     │
   │  ──────────────────────────────────────────────────      │
   │                                                          │
   │  Yew         │ React-like   │ JSX-like html!{}          │
   │              │              │ Hooks, components         │
   │              │              │ Most mature                │
   │                                                          │
   │  Leptos      │ SolidJS-like  │ Fine-grained signals     │
   │              │              │ SSR + hydration            │
   │              │              │ Fastest WASM SPA           │
   │                                                          │
   │  Dioxus      │ React-like    │ Targets: Web/Desktop/    │
   │              │              │   Mobile/TUI                │
   │              │              │ Single codebase            │
   │                                                          │
   │  Sycamore    │ SolidJS-like  │ Like Leptos, smaller     │
   │                                                          │
   │  Sauron      │ Elm-like      │ Functional, MVU          │
   │                                                          │
   │  Iced        │ Elm-like      │ Native + WASM target      │
   │              │              │ Game-like apps             │
   │                                                          │
   │  egui        │ Immediate     │ Game UI, debug tools      │
   │              │ mode          │ Bevy-compatible            │
   │                                                          │
   │  Bevy        │ Game engine   │ ECS, full game features   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Decision tree:
   ──────────────
   
                What are you building?
                       │
        ┌──────────────┼──────────────┬─────────────┐
        ▼              ▼              ▼             ▼
     SPA          Game            Multi-target   Tool/CLI
        │              │              │             │
   ┌────┴────┐         │              │             │
   React-like  Solid-like      Dioxus            (likely
   familiar?  optimization?                       no GUI)
        │         │
        ▼         ▼
       Yew     Leptos          Bevy/iced/egui
```

---

## 17. WASM use cases by domain

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  DOMAIN                  │ WHY WASM                      │
   │  ────────────────────    │ ───────────                   │
   │                                                          │
   │  Image/video processing  │ Heavy SIMD math, performance │
   │  (Photoshop Web)         │                              │
   │                                                          │
   │  Cryptography            │ Constant-time, no GC pauses  │
   │  (1Password)             │ Memory safety                 │
   │                                                          │
   │  Machine learning        │ Local inference (privacy)    │
   │  (TF.js with WASM)       │ Math performance              │
   │                                                          │
   │  Games                   │ Bevy/Unity → WASM            │
   │                          │ No install, instant play     │
   │                                                          │
   │  Edge computing          │ Cold start ms, sandboxed     │
   │  (Cloudflare Workers)    │ Multi-tenant safe             │
   │                                                          │
   │  Plugin systems          │ Untrusted code sandboxed     │
   │  (Shopify Functions)     │ Cross-language               │
   │                                                          │
   │  Audio/video codecs      │ Real-time processing         │
   │  (Discord voice)         │ Hardware acceleration         │
   │                                                          │
   │  Browser apps             │ Performance-critical paths   │
   │  (Figma, Google Earth)    │ Port existing C++ codebase  │
   │                                                          │
   │  Code playgrounds         │ Run user code safely         │
   │  (Rust Playground)        │ No backend needed             │
   │                                                          │
   │  Universal modules        │ Same .wasm runs everywhere   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. Common patterns visualization

```
   ✅ 1. Compute-heavy in WASM, UI in JS
   ──────────────────────────────────────
   
   JS: orchestrate, DOM, fetch, render
   WASM: process pixels, encode/decode, math, crypto
   
   const data = await fetch(...);
   const result = wasm.processImage(data);
   renderToCanvas(result);
   
   
   ✅ 2. Reuse Rust code: native + WASM
   ─────────────────────────────────────
   
   shared/        ← Pure Rust crate
     validate_email()
     calculate_tax()
   ↓ compiles to both
   • Native (server)
   • WASM (browser)
   
   Same logic, no duplication.
   
   
   ✅ 3. Plugin system with WASI
   ──────────────────────────────
   
   Host (Rust):                Plugin (any language):
   ──────────                   ────────────────
   load(plugin.wasm)            compile to .wasm
   instantiate                  expose function
   call function                
   sandbox enforced
   
   3rd parties upload .wasm → host runs safely.
   
   
   ✅ 4. Edge computing
   ─────────────────
   
   User → nearest edge node → WASM (cold start ms)
                              → KV/D1 (local data)
                              → Response
   
   ~10-30ms total.
   
   
   ✅ 5. Streaming compile
   ─────────────────────
   
   const module = await WebAssembly.compileStreaming(
       fetch('app.wasm')
   );
   
   Compile in parallel with download.
```

---

## 19. Antipatterns

```
   ❌ 1. Tiny functions in WASM
   ────────────────────────────
   
   #[wasm_bindgen]
   pub fn add(a: i32, b: i32) -> i32 { a + b }
   
   Boundary crossing cost >>> work.
   JS faster for this.
   
   ✅ Put add() in JS. Use WASM for big work.
   
   
   ❌ 2. Many DOM crossings from Rust
   ──────────────────────────────────
   
   for i in 0..1000 {
       let el = document.create_element(...)?;
       body.append_child(&el)?;
   }
   // 2000 boundary crossings!
   
   ✅ Build HTML string in WASM, set innerHTML in JS.
   
   
   ❌ 3. Forget to free()
   ─────────────────────
   
   const counter = new Counter();
   counter.increment();
   // Never freed → memory leak in WASM heap
   
   ✅ Use try/finally or modern FinalizationRegistry-based auto-free.
   
   
   ❌ 4. No panic hook
   ─────────────────
   
   panic!("...");    // silent abort, no error in console
   
   ✅ console_error_panic_hook::set_once() at startup
   
   
   ❌ 5. Large strings everywhere
   ─────────────────────────────
   
   pub fn process(text: String) -> String { ... }
   // Marshal cost both ways
   
   ✅ Process incrementally, or use Uint8Array direct memory.
   
   
   ❌ 6. Not lazy-loading
   ─────────────────────
   
   import init from './pkg/myapp.js';
   await init();   // user pays even if not used
   
   ✅ Dynamic import on demand:
   const wasm = await import('./pkg/myapp.js');
   
   
   ❌ 7. Over-using web-sys features
   ────────────────────────────────
   
   web-sys = { features = ["all-the-things"] }
   // Bloats binary
   
   ✅ Enable only what you need.
```

---

## 20. Mind map cuối

```
                              WASM
                                │
        ┌────────────┬──────────┼──────────┬─────────────┐
        ▼            ▼          ▼          ▼             ▼
   ARCHITECTURE  TOOLING     BROWSER    SERVER       FRAMEWORKS
        │            │          │          │             │
   Linear mem    wasm-bindgen  web-sys   WASI         Yew
   Stack VM       wasm-pack    js-sys    wasmtime     Leptos
   Imports        wasm-opt     web-sys   Cloudflare   Dioxus
   Exports        cargo+target Workers   Fastly       Bevy
   Sandboxed                   Web Worker             egui
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. WASM = compute, JS = orchestrate │
                │                                      │
                │  2. wasm-bindgen for browser,        │
                │     WASI for server/edge             │
                │                                      │
                │  3. Minimize JS↔WASM crossings       │
                │                                      │
                │  4. console_error_panic_hook ALWAYS  │
                │                                      │
                │  5. Manual free() — no GC            │
                │                                      │
                │  6. wasm-opt -Oz for size            │
                │                                      │
                │  7. Web Worker for heavy compute     │
                │                                      │
                │  8. WASI capability-based security   │
                │                                      │
                │  9. Edge platforms = perfect fit     │
                │                                      │
                │  10. Component Model = future        │
                │                                      │
                │  11. Reuse Rust on backend + WASM    │
                │                                      │
                │  12. Test with wasm-bindgen-test     │
                └──────────────────────────────────────┘
```

---

## 21. Bộ tài liệu giờ có 20 chương!

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   PHẦN I-V (a-s): 19 chương foundation đã có             │
   │                                                          │
   │   PHẦN VI: Universal deployment                          │
   │   ────────                                                │
   │   t. wasm                       — Browser+Edge+Server    │
   │      t-wasm.md + t-wasm-visual.md   ← VỪA HOÀN THÀNH    │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 20 chương × 2 files = 40 files                   │
   │                                                          │
   │   🦀 Bộ kỹ năng FULL-STACK + WASM ĐẦY ĐỦ                 │
   │                                                          │
   │   🌐 Web (axum)                                          │
   │   🗄️ Database (sqlx)                                     │
   │   🔌 Embedded (no_std)                                   │
   │   🖥️ Desktop (Tauri)                                     │
   │   📱 Mobile (Tauri v2)                                   │
   │   🌍 WASM (browser/edge/server) ← MỚI                    │
   │   🚀 Performance                                         │
   │   🧪 Testing                                             │
   │   🔍 Observability                                       │
   │   ⚙️ Unsafe + FFI                                        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Đã 20 chương. Nếu muốn đào tiếp:

- **Game engines** (Bevy ECS architecture)
- **CLI tools** (clap, dialoguer, indicatif, ratatui)
- **GUI native** (egui, iced — pure Rust, no webview)
- **gRPC** (tonic, prost)
- **Cryptography** (rustls, ring, age)
- **OS kernels** (Redox patterns)

🦀 Báo nếu muốn tiếp!
