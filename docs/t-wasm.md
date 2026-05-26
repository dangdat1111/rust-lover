# WASM (WebAssembly) trong Rust — Deep Dive

> Tài liệu thứ 20 (chương t) trong bộ Rust nền tảng. Đọc trước:
> - [a-memory-model.md](./a-memory-model.md) — WASM có memory model riêng
> - [c-trait.md](./c-trait.md) — wasm-bindgen dùng traits
> - [f-async.md](./f-async.md) — WASM async via Promises
> - [n-unsafe-rust.md](./n-unsafe-rust.md) — JS interop dùng unsafe nhiều
> - [s-tauri.md](./s-tauri.md) — Tauri có thể export logic to WASM
>
> **WebAssembly (WASM)** là binary instruction format cho stack-based VM.
> - **Portable**: chạy được mọi platform (browser, server, edge, embedded)
> - **Fast**: near-native performance (~85-95% native speed)
> - **Safe**: sandboxed, can't escape VM
> - **Compact**: binary nhỏ, fast to download/parse
> - **Language-agnostic**: Rust, C++, Go, AssemblyScript, ...
>
> Rust là **best-in-class** cho WASM:
> - Zero-cost abstraction → fit nhỏ
> - No GC → predictable performance
> - Strong type → reliable interop
> - Mature tooling (wasm-bindgen, wasm-pack)
>
> Use cases:
> - **Browser**: heavy compute (image, video, crypto, games, ML)
> - **Edge**: serverless on Cloudflare Workers, Fastly Compute@Edge
> - **Server**: plugin systems, multi-tenant isolation
> - **Embedded**: portable runtime (IoT, satellites)

---

# Mục lục

- [Tầng 1: WASM là gì? Vì sao Rust?](#tầng-1-wasm-là-gì-vì-sao-rust)
- [Tầng 2: WASM architecture & memory model](#tầng-2-wasm-architecture--memory-model)
- [Tầng 3: Rust → WASM toolchain](#tầng-3-rust--wasm-toolchain)
- [Tầng 4: wasm-bindgen — Bridge JS ↔ Rust](#tầng-4-wasm-bindgen--bridge-js--rust)
- [Tầng 5: Type marshalling — Pass data JS ↔ Rust](#tầng-5-type-marshalling--pass-data-js--rust)
- [Tầng 6: wasm-pack — Build cho npm](#tầng-6-wasm-pack--build-cho-npm)
- [Tầng 7: DOM access & web APIs](#tầng-7-dom-access--web-apis)
- [Tầng 8: Async / Promise interop](#tầng-8-async--promise-interop)
- [Tầng 9: Web Workers — WASM trong thread khác](#tầng-9-web-workers--wasm-trong-thread-khác)
- [Tầng 10: WASI — WASM Server-side & Edge](#tầng-10-wasi--wasm-server-side--edge)
- [Tầng 11: Cloudflare Workers, Fastly, Edge platforms](#tầng-11-cloudflare-workers-fastly-edge-platforms)
- [Tầng 12: Component Model — Tương lai của WASM](#tầng-12-component-model--tương-lai-của-wasm)
- [Tầng 13: Performance — Optimize WASM](#tầng-13-performance--optimize-wasm)
- [Tầng 14: Testing WASM](#tầng-14-testing-wasm)
- [Tầng 15: Patterns & Antipatterns](#tầng-15-patterns--antipatterns)
- [Tầng 16: Real-world apps & frameworks](#tầng-16-real-world-apps--frameworks)

---

# Tầng 1: WASM là gì? Vì sao Rust?

## 1.1 Lịch sử & motivation

```
   2015: asm.js — JS subset for ahead-of-time compilation
            ↓
   2017: WASM 1.0 (MVP) — browser standard
            ↓
   2019: WASI (WebAssembly System Interface)
            ↓
   2024+: Component Model, async support, threads
```

WASM = stack-based virtual machine với binary instruction format.

Mục tiêu ban đầu: chạy code C/C++/Rust **trong browser** nhanh hơn JS. Sau đó mở rộng ra server, edge, embedded.

## 1.2 Vì sao WASM thay vì JS?

```
   ┌────────────────────────────────────────────────────────────┐
   │  JS                       WASM                             │
   │  ──                       ────                             │
   │                                                            │
   │  Interpreted/JIT          Compiled                         │
   │  Dynamic types            Static types                     │
   │  GC pauses                No GC (or minimal)               │
   │  Parse + compile slow     Fast decode + validate           │
   │  Variable perf            Predictable perf                  │
   │  Hard to optimize         Easy to optimize                  │
   │                                                            │
   │  Speed: baseline          Speed: 1.5-3x faster              │
   │                           (closer to native)                │
   └────────────────────────────────────────────────────────────┘
```

WASM modules:
- **Compile once**, run everywhere (browser, server, edge)
- Near-native speed (~85-95%)
- Binary format nhỏ (smaller download)
- Sandboxed → security

## 1.3 Vì sao Rust cho WASM?

```
   ┌──────────────────────────────────────────────────────────┐
   │  Language    │ WASM-friendly? │ Notes                   │
   ├──────────────┼────────────────┼─────────────────────────┤
   │ Rust          │ ✅ Excellent   │ Zero-cost, no GC, mature│
   │ C/C++         │ ✅ Good        │ Manual memory, emscripten│
   │ Go            │ ⚠️ Big binary   │ GC included            │
   │ AssemblyScript│ ✅ Good        │ TS subset → WASM       │
   │ Python (?)    │ ⚠️ Pyodide      │ CPython compiled, slow │
   │ Kotlin/Native │ ✅ Good        │                         │
   └──────────────┴────────────────┴─────────────────────────┘
```

Rust advantages:
- **Small binaries** (~10-100 KB typical, vs Go ~1-2 MB)
- **No GC** → predictable, no jank
- **Zero-cost abstraction** → high-level code, fast output
- **Strong typing** → JS interop reliable
- **Mature tooling**: wasm-bindgen, wasm-pack, web-sys, js-sys

## 1.4 Real-world Rust + WASM examples

```
   ┌──────────────────────────────────────────────────────────┐
   │ • Figma  — Performance-critical UI in C++→WASM           │
   │ • Google Earth — 3D rendering in WASM                    │
   │ • Photoshop Web — full Photoshop in browser              │
   │ • 1Password — crypto in Rust→WASM                        │
   │ • Cloudflare Workers — Rust→WASM at edge                 │
   │ • Shopify Functions — Rust→WASM extensions               │
   │ • Discord — voice processing                              │
   │ • Yew, Leptos — full SPA in Rust→WASM                    │
   │ • Bevy — game engine, WASM target                         │
   │ • RustPython, Pyodide — Python in browser                 │
   └──────────────────────────────────────────────────────────┘
```

## 1.5 Khi NÊN dùng WASM?

```
   ✅ DO:
   • Compute-heavy code (image, video, ML, crypto)
   • Port existing C/C++/Rust to web
   • Performance-critical paths
   • Plugin systems (sandboxed)
   • Edge computing (faster cold start than JS containers)
   • Cross-platform binary distribution
   
   ❌ DON'T:
   • Simple UI logic (JS sufficient + better DX)
   • Heavy DOM manipulation (JS still faster across boundary)
   • Small functions (call overhead > work)
   • SEO-critical content (server-render with WASM, not client)
```

---

# Tầng 2: WASM architecture & memory model

## 2.1 WASM virtual machine

```
   ┌──────────────────────────────────────────────────────────┐
   │                  WASM VIRTUAL MACHINE                    │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Stack (operand stack — for VM operations)       │    │
   │   │   push, pop, arithmetic, control flow           │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Linear memory (single contiguous byte array)    │    │
   │   │   Default 64KB, grow to ~4GB (32-bit addressing)│    │
   │   │   Accessed via i32 offset                        │    │
   │   │   Initialized at module load                     │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Globals (typed constants/variables)             │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Function table (for indirect calls)             │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Imports (from host) + Exports (to host)         │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

## 2.2 Linear memory

```rust
// Trong WASM:
let memory: WebAssembly.Memory = new WebAssembly.Memory({ initial: 1, maximum: 100 });
// initial: 1 page = 64KB
// maximum: 100 pages = 6.4MB
```

WASM memory:
- **Single linear array of bytes**
- Grows in 64KB "pages"
- Accessed via i32 offset (max 4GB)
- Sandboxed (can't access host memory outside)

```
   Linear memory layout (managed by Rust):
   
   0           heap_start    stack_top    end
   ▼           ▼             ▼            ▼
   ┌───────────┬─────────────┬────────────┐
   │ globals   │  heap       │   stack    │
   │ .data     │  (alloc)    │  (locals)  │
   │ .rodata   │   ↓ grow    │   ↑ grow   │
   └───────────┴─────────────┴────────────┘
```

Rust allocator (default: `wee_alloc` for size, `dlmalloc` for speed) manages heap.

## 2.3 Stack-based VM

```
   WASM instruction: i32.add
   
   Stack before:    [..., a, b]
                              ▲ b on top
   Pop 2 values, add, push result:
   Stack after:     [..., a+b]
   
   
   Example function:
   ────────────────
   (func $add (param i32 i32) (result i32)
     local.get 0       ;; push first param
     local.get 1       ;; push second param
     i32.add           ;; pop 2, push sum
   )
   
   Equivalent Rust:
   ────────────────
   fn add(a: i32, b: i32) -> i32 { a + b }
```

WASM is **bytecode**, not a high-level language. Rust compiles to it.

## 2.4 No GC, no pointers (in WASM sense)

WASM doesn't have:
- Heap GC (Rust does manual via allocator)
- Object references in instruction set
- Direct memory protection (sandbox in memory bounds)

Rust → WASM:
- `Box<T>`, `Vec<T>`: allocator in linear memory
- `&T`: just `i32` offset in linear memory
- `String`: ptr + len, just 2 `i32` to JS

## 2.5 Hostfunctions = imports

WASM is **pure** — can't I/O, alloc, syscall. Must **import** functions from host:

```
   WASM:
     imports console.log(i32, i32)   ← host provides
   
   Host (JS):
     console.log = (ptr, len) => {
       const bytes = new Uint8Array(memory.buffer, ptr, len);
       console.log(new TextDecoder().decode(bytes));
     };
```

Browser provides DOM APIs. WASI provides POSIX-like file/network APIs. Wrapper crate (`web-sys`, `wasi`) makes Rust ergonomic.

## 2.6 Types in WASM (limited!)

Native WASM types:
- `i32`, `i64` — integers
- `f32`, `f64` — floats
- `funcref`, `externref` — references (newer)

That's it! No strings, no structs, no objects natively.

Rust types (String, Vec, struct) marshal as:
- Strings → ptr + len in linear memory
- Structs → bytes in linear memory
- Complex types → opaque handles via wasm-bindgen

---

# Tầng 3: Rust → WASM toolchain

## 3.1 WASM targets

```bash
# Browser (with wasm-bindgen):
rustup target add wasm32-unknown-unknown

# WASI (server, edge):
rustup target add wasm32-wasip1   # newer (formerly wasi)
rustup target add wasm32-wasip2   # newest (Component Model)
```

Different targets for different runtimes:
- `wasm32-unknown-unknown` — minimal, no host APIs (paired with wasm-bindgen for browser)
- `wasm32-wasip1` — WASI Preview 1 (file, env, time)
- `wasm32-wasip2` — WASI Preview 2 (Component Model)

## 3.2 Bare wasm32-unknown-unknown

Smallest, no APIs:
```rust
#![no_main]

#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```bash
cargo build --target wasm32-unknown-unknown --release
# Output: target/wasm32-unknown-unknown/release/myapp.wasm
```

Output very small (~100 byte for `add`). But can't do anything practical without host imports.

## 3.3 With wasm-bindgen (browser)

```toml
[lib]
crate-type = ["cdylib"]   # output .wasm

[dependencies]
wasm-bindgen = "0.2"
```

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

```bash
cargo install wasm-pack
wasm-pack build --target web
# Output: pkg/ directory with .wasm + JS glue + .d.ts
```

Use in JS:
```javascript
import init, { add, greet } from './pkg/myapp.js';

await init();   // load WASM
console.log(add(1, 2));        // 3
console.log(greet('World'));    // "Hello, World!"
```

## 3.4 Common build commands

```bash
# Browser:
wasm-pack build --target web        # ESM (for native ES modules)
wasm-pack build --target bundler    # for webpack/rollup
wasm-pack build --target nodejs     # for Node.js
wasm-pack build --target no-modules # plain script tag

# WASI:
cargo build --target wasm32-wasip1 --release

# Run WASI:
wasmtime run target/wasm32-wasip1/release/myapp.wasm
```

## 3.5 Cargo.toml setup

```toml
[package]
name = "my-wasm-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for WASM, rlib for unit tests

[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Window", "Document"] }
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"

[dependencies.console_error_panic_hook]
version = "0.1"
optional = true

[features]
default = ["console_error_panic_hook"]

[profile.release]
opt-level = "s"        # optimize for size (or "z" for more aggressive)
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

`opt-level = "s"` important for WASM (file size = download time).

---

# Tầng 4: wasm-bindgen — Bridge JS ↔ Rust

## 4.1 wasm-bindgen là gì?

WASM types limited (i32/f32/etc). To pass strings, objects, callbacks → need glue code.

**wasm-bindgen** generates:
- JS wrapper code
- Rust traits for marshalling
- TypeScript .d.ts definitions
- All "magic" of seamless JS interop

## 4.2 Basic example

```rust
use wasm_bindgen::prelude::*;

// Export Rust function to JS
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Import JS function into Rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    // Alias to Rust:
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
    
    // JS class:
    type Date;
    
    #[wasm_bindgen(constructor)]
    fn new() -> Date;
    
    #[wasm_bindgen(method, getter)]
    fn time(this: &Date) -> f64;
}

#[wasm_bindgen]
pub fn now() -> f64 {
    let d = Date::new();
    d.time()
}
```

## 4.3 Exporting types

```rust
#[wasm_bindgen]
pub struct Counter {
    value: i32,
}

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Counter {
        Counter { value: 0 }
    }
    
    pub fn increment(&mut self) {
        self.value += 1;
    }
    
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> i32 {
        self.value
    }
}
```

JS usage:
```javascript
import { Counter } from './pkg/myapp.js';

const c = new Counter();
c.increment();
c.increment();
console.log(c.value);   // 2

c.free();   // manually free Rust memory!
```

**Important**: JS sees `Counter` as opaque handle. Rust memory must be **manually freed** (unlike JS GC).

Modern wasm-bindgen has `WeakRef`-based auto-free (experimental).

## 4.4 Generate TypeScript types

```bash
wasm-pack build --target web --typescript
# pkg/myapp.d.ts auto-generated
```

```typescript
// pkg/myapp.d.ts
export class Counter {
  free(): void;
  constructor();
  increment(): void;
  readonly value: number;
}

export function greet(name: string): string;
```

Type-safe usage in TS.

## 4.5 Enums

```rust
#[wasm_bindgen]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[wasm_bindgen]
pub fn describe(c: Color) -> String {
    match c {
        Color::Red => "Hot".into(),
        Color::Green => "Cool".into(),
        Color::Blue => "Cold".into(),
    }
}
```

```javascript
import { Color, describe } from './pkg/myapp.js';

console.log(describe(Color.Red));    // "Hot"
```

C-like enums only (no variants with data). For complex enums, use `serde-wasm-bindgen` (Tầng 5).

## 4.6 Callback closures

```rust
#[wasm_bindgen]
pub fn run_callback(cb: &js_sys::Function) {
    let this = JsValue::null();
    let val = JsValue::from(42);
    cb.call1(&this, &val).unwrap();
}
```

```javascript
import { run_callback } from './pkg/myapp.js';

run_callback((x) => console.log("got:", x));
// "got: 42"
```

## 4.7 Closure from Rust → JS

```rust
use wasm_bindgen::closure::Closure;

#[wasm_bindgen]
pub fn register_handler() -> js_sys::Function {
    let cb = Closure::wrap(Box::new(move |x: i32| {
        web_sys::console::log_1(&format!("got {}", x).into());
    }) as Box<dyn FnMut(i32)>);
    
    let f = cb.as_ref().unchecked_ref::<js_sys::Function>().clone();
    cb.forget();   // prevent drop (memory leak if not careful!)
    f
}
```

Trade-off: `forget()` leaks closure. Manage lifecycle carefully.

---

# Tầng 5: Type marshalling — Pass data JS ↔ Rust

## 5.1 Primitive types

```rust
#[wasm_bindgen]
pub fn primitives(a: i32, b: u32, c: f64, d: bool) {}
```

Direct in WASM. No conversion needed. Free.

## 5.2 Strings

```rust
#[wasm_bindgen]
pub fn process_string(s: &str) -> String {
    s.to_uppercase()
}
```

JS → Rust: encode JS string (UTF-16) to UTF-8, write into linear memory, pass ptr + len.

Rust → JS: ptr + len returned, JS reads bytes, decode UTF-8 to UTF-16.

Cost: O(n) per call. For huge strings, consider working with bytes directly.

## 5.3 Vec<T> → Array

```rust
#[wasm_bindgen]
pub fn sum_vec(v: Vec<i32>) -> i32 {
    v.iter().sum()
}

#[wasm_bindgen]
pub fn double(v: Vec<f64>) -> Vec<f64> {
    v.into_iter().map(|x| x * 2.0).collect()
}
```

JS sees `Array<number>`. Marshal via TypedArray + alloc/free in linear memory.

Faster: pass `&[f64]` (slice) — no ownership transfer.

## 5.4 Complex types — serde-wasm-bindgen

```toml
[dependencies]
serde-wasm-bindgen = "0.6"
serde = { version = "1", features = ["derive"] }
```

```rust
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    emails: Vec<String>,
}

#[wasm_bindgen]
pub fn create_user(input: JsValue) -> Result<JsValue, JsValue> {
    let req: User = serde_wasm_bindgen::from_value(input)?;
    
    let user = User {
        id: 1,
        name: req.name.to_uppercase(),
        emails: req.emails,
    };
    
    Ok(serde_wasm_bindgen::to_value(&user)?)
}
```

JS:
```javascript
const user = create_user({
    id: 0,
    name: 'alice',
    emails: ['a@b.com', 'c@d.com'],
});
// user = { id: 1, name: 'ALICE', emails: [...] }
```

Magic: serde converts via JsValue. Slower than direct binding, but flexible.

## 5.5 Pass large data — ArrayBuffer

For images, audio, large binary:

```rust
#[wasm_bindgen]
pub fn process_image(data: &[u8]) -> Vec<u8> {
    // Process pixels
    data.iter().map(|b| b.wrapping_add(10)).collect()
}
```

```javascript
const img = await fetch('/image.bin').then(r => r.arrayBuffer());
const result = process_image(new Uint8Array(img));
```

Or share memory zero-copy:

```rust
#[wasm_bindgen]
pub fn process_in_place(ptr: *mut u8, len: usize) {
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    for b in slice {
        *b = b.wrapping_add(10);
    }
}
```

```javascript
const buffer = wasm.memory.buffer;
const view = new Uint8Array(buffer, ptr, len);
// View shares memory directly — no copy
process_in_place(ptr, len);
// view now has updated data
```

Advanced — `unsafe` Rust. Faster for huge data.

## 5.6 Performance hierarchy of marshalling

```
   Fastest → Slowest:
   ────────────────
   
   Primitives (i32, f64)    →  near-free
   &[u8] / &[T]              →  ptr + len, no copy if view
   String / Vec              →  alloc + copy
   serde (JsValue)           →  alloc + copy + serialize
   Object handle              →  cross-boundary call per access
```

Minimize marshalling for hot paths. Batch operations.

## 5.7 Error handling

```rust
#[wasm_bindgen]
pub fn risky(input: i32) -> Result<i32, JsError> {
    if input < 0 {
        return Err(JsError::new("negative input"));
    }
    Ok(input * 2)
}
```

JS:
```javascript
try {
    const r = risky(-5);
} catch (e) {
    console.error(e.message);   // "negative input"
}
```

`Result<T, JsError>` → Promise rejects in JS.

---

# Tầng 6: wasm-pack — Build cho npm

## 6.1 wasm-pack tool

```bash
cargo install wasm-pack

wasm-pack build --target <target>
wasm-pack publish    # to npm
wasm-pack test       # run tests
```

Generates ready-to-use npm package.

## 6.2 Output structure

```
pkg/
├── package.json         # npm package
├── myapp.js             # JS glue
├── myapp.d.ts           # TypeScript types
├── myapp_bg.wasm        # compiled WASM
├── myapp_bg.js          # internal bindings
├── README.md
└── LICENSE
```

Drop into web app:
```javascript
import { greet } from './pkg/myapp.js';   // ES module
```

Or publish to npm:
```bash
cd pkg
npm publish
```

Then use:
```javascript
import { greet } from 'my-wasm-app';
```

## 6.3 Build targets explained

```bash
wasm-pack build --target web
```

- ESM with named exports
- Uses native ES modules in browser
- Best for modern apps

```bash
wasm-pack build --target bundler
```

- For webpack, rollup, vite
- Webpack 4+ understands `*.wasm`
- Slightly different glue code

```bash
wasm-pack build --target nodejs
```

- For Node.js
- Uses `require`-compatible JS

```bash
wasm-pack build --target no-modules
```

- Plain script tag, no modules
- Sets global variable

## 6.4 Common project structure

```
my-wasm-app/
├── Cargo.toml
├── src/
│   └── lib.rs          # Rust source
├── tests/
│   └── web.rs          # WASM tests (run in browser via wasm-bindgen-test)
├── pkg/                # Generated (gitignore)
│
└── www/                # Frontend
    ├── package.json
    ├── index.html
    ├── index.js
    └── webpack.config.js   # or vite.config.js
```

JS app imports from local `pkg/`.

## 6.5 wasm-pack vs cargo build

```bash
# Manual:
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/myapp.wasm \
    --out-dir pkg --target web
wasm-opt -Os pkg/myapp_bg.wasm -o pkg/myapp_bg_optimized.wasm
```

wasm-pack does all that + creates package.json. More convenient.

## 6.6 Integration with frontend tools

### Vite (modern)
```js
// vite.config.js
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default {
    plugins: [wasm(), topLevelAwait()],
};
```

```javascript
import init, { greet } from './pkg/myapp.js';

await init();   // top-level await — Vite plugin handles
greet('World');
```

### Webpack
```js
// webpack.config.js
module.exports = {
    experiments: { asyncWebAssembly: true },
};
```

```javascript
import('./pkg/myapp.js').then(({ greet }) => {
    console.log(greet('World'));
});
```

---

# Tầng 7: DOM access & web APIs

## 7.1 web-sys & js-sys crates

```toml
[dependencies]
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "console", "Document", "Window", "Element",
    "HtmlElement", "HtmlInputElement", "Event", "MouseEvent",
    "Performance", "Storage",
] }
```

- **js-sys**: JS standard library bindings (Array, Object, Promise, ...)
- **web-sys**: Web API bindings (DOM, fetch, IndexedDB, ...)

Each browser API requires opt-in via features (keep WASM small).

## 7.2 Console log

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Or use web-sys:
use web_sys::console;

#[wasm_bindgen]
pub fn say_hi() {
    console::log_1(&"Hello from Rust!".into());
}
```

`console::log_1` for 1 arg, `log_2`, `log_3`, ... Annoying API. Use macro:

```rust
macro_rules! log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

log!("Hello, {}!", "World");
```

## 7.3 DOM manipulation

```rust
use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlElement, Window};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let body = document.body().ok_or("no body")?;
    
    let div = document.create_element("div")?;
    div.set_text_content(Some("Hello from Rust!"));
    div.set_attribute("style", "padding: 20px; color: blue;")?;
    
    body.append_child(&div)?;
    
    Ok(())
}
```

`#[wasm_bindgen(start)]` runs when WASM loads.

## 7.4 Event handlers

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::{Event, HtmlElement};

#[wasm_bindgen]
pub fn add_click_handler(elem: &HtmlElement) {
    let closure = Closure::wrap(Box::new(move |_event: Event| {
        web_sys::console::log_1(&"Clicked!".into());
    }) as Box<dyn FnMut(Event)>);
    
    elem.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();   // leak (cleanup needed)
}
```

Closures + lifetimes tricky. `forget()` to leak; or store closure handle to free later.

## 7.5 Fetch API

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

#[wasm_bindgen]
pub async fn fetch_url(url: &str) -> Result<String, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    
    let request = Request::new_with_str_and_init(url, &opts)?;
    
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    
    let resp: Response = resp_value.dyn_into()?;
    
    let text = JsFuture::from(resp.text()?).await?;
    Ok(text.as_string().unwrap())
}
```

```javascript
const text = await fetch_url('https://api.example.com/data');
console.log(text);
```

`async fn` returning `Promise` works seamlessly.

## 7.6 localStorage

```rust
use web_sys::Storage;

#[wasm_bindgen]
pub fn save_value(key: &str, value: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage()?.unwrap();
    storage.set_item(key, value)?;
    Ok(())
}

#[wasm_bindgen]
pub fn get_value(key: &str) -> Option<String> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().ok()??;
    storage.get_item(key).ok().flatten()
}
```

## 7.7 Performance API

```rust
use web_sys::Performance;

#[wasm_bindgen]
pub fn benchmark() -> f64 {
    let window = web_sys::window().unwrap();
    let perf = window.performance().unwrap();
    let start = perf.now();
    
    // ... work ...
    
    perf.now() - start
}
```

## 7.8 console_error_panic_hook

By default, Rust panic in WASM = silent crash. Install hook:

```toml
[dependencies]
console_error_panic_hook = "0.1"
```

```rust
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    // ...
}
```

Now panics show in browser console with stack trace.

---

# Tầng 8: Async / Promise interop

## 8.1 wasm-bindgen-futures

```toml
[dependencies]
wasm-bindgen-futures = "0.4"
```

Async Rust ↔ JS Promise:
- Rust `async fn` returning `T` → JS Promise resolving with `T`
- JS Promise → Rust `JsFuture` → `.await`

## 8.2 Rust async → JS Promise

```rust
use wasm_bindgen::prelude::*;
use gloo::timers::future::TimeoutFuture;

#[wasm_bindgen]
pub async fn delayed_greet(name: String, ms: u32) -> String {
    TimeoutFuture::new(ms).await;
    format!("Hello, {} (after {}ms)", name, ms)
}
```

```javascript
const msg = await delayed_greet('World', 1000);
console.log(msg);
```

## 8.3 Calling JS Promise from Rust

```rust
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub async fn use_js_promise() -> Result<JsValue, JsValue> {
    let promise = js_sys::Promise::resolve(&JsValue::from(42));
    let value = JsFuture::from(promise).await?;
    Ok(value)
}
```

`JsFuture::from(promise).await` — wait for JS Promise.

## 8.4 No tokio in WASM (browser)

```rust
// ❌ doesn't work in wasm32-unknown-unknown (browser)
tokio::time::sleep(Duration::from_secs(1)).await;
```

Browser has no thread pool. Use:
- `wasm-bindgen-futures` for spawn
- `gloo::timers` for sleep
- `gloo::events` for event listeners

```rust
use gloo::timers::future::TimeoutFuture;

#[wasm_bindgen]
pub async fn slow_fn() {
    TimeoutFuture::new(1000).await;   // 1 second
}
```

## 8.5 spawn_local — Fire & forget

```rust
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
pub fn start_background() {
    spawn_local(async {
        loop {
            TimeoutFuture::new(5000).await;
            web_sys::console::log_1(&"tick".into());
        }
    });
}
```

Run async task in background. Like tokio::spawn but for browser.

## 8.6 Channel patterns

For complex async, use `futures::channel`:

```rust
use futures::channel::oneshot;

#[wasm_bindgen]
pub async fn wait_for_event() -> JsValue {
    let (tx, rx) = oneshot::channel();
    
    let button = get_button();
    let tx = std::rc::Rc::new(std::cell::Cell::new(Some(tx)));
    let tx_clone = tx.clone();
    
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if let Some(tx) = tx_clone.take() {
            let _ = tx.send(JsValue::from("clicked"));
        }
    }) as Box<dyn FnMut(_)>);
    
    button.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    
    rx.await.unwrap_or(JsValue::null())
}
```

Common pattern: event listener → channel → async function awaits.

---

# Tầng 9: Web Workers — WASM trong thread khác

## 9.1 Vấn đề: WASM trên main thread

WASM trong browser default chạy trên **main thread** (UI thread). Heavy compute → UI freeze.

Solution: **Web Worker** = separate thread.

## 9.2 Setup web worker với WASM

```javascript
// main.js
const worker = new Worker('worker.js', { type: 'module' });

worker.onmessage = (e) => {
    console.log('Result:', e.data);
};

worker.postMessage({ input: [1, 2, 3, 4, 5] });
```

```javascript
// worker.js
import init, { compute_heavy } from './pkg/myapp.js';

await init();

self.onmessage = async (e) => {
    const result = compute_heavy(e.data.input);
    self.postMessage(result);
};
```

WASM loaded in worker, no UI block.

## 9.3 wasm-bindgen-rayon (parallel WASM)

```toml
[dependencies]
wasm-bindgen-rayon = "1"
rayon = "1"
```

```rust
use wasm_bindgen::prelude::*;
use rayon::prelude::*;

#[wasm_bindgen]
pub fn parallel_sum(v: Vec<i32>) -> i32 {
    v.par_iter().sum()
}
```

Requires SharedArrayBuffer (HTTP headers: COOP/COEP).

Limits in browser:
- Need COOP / COEP headers
- Atomics + Mutex available
- Setup more complex

## 9.4 Shared memory

```javascript
// Server must send:
// Cross-Origin-Opener-Policy: same-origin
// Cross-Origin-Embedder-Policy: require-corp

// In JS:
const memory = new WebAssembly.Memory({ initial: 10, maximum: 100, shared: true });
```

Then WASM threads access same memory. Atomic ops work.

## 9.5 Pattern: Background WASM with Comlink

```javascript
// Comlink simplifies worker RPC
import * as Comlink from 'comlink';

// worker.js
import init, { compute } from './pkg/myapp.js';
await init();
Comlink.expose({ compute });

// main.js
const worker = new Worker('worker.js', { type: 'module' });
const api = Comlink.wrap(worker);
const result = await api.compute([1, 2, 3]);
```

Cleaner than postMessage. Type-safe (with TS).

---

# Tầng 10: WASI — WASM Server-side & Edge

## 10.1 WASI là gì?

**WASI** (WebAssembly System Interface) = POSIX-like API cho WASM ngoài browser.

Provides:
- File system (open, read, write)
- Environment (env vars, args)
- Time (clock)
- Random
- Network (preview 2 only)

Run WASM as standalone executable. Sandbox + capability-based security.

## 10.2 Target wasm32-wasip1

```bash
rustup target add wasm32-wasip1

cargo build --target wasm32-wasip1 --release
```

Now `std::fs`, `println!`, `std::env::args()` work in WASM.

## 10.3 Hello WASI

```rust
// src/main.rs
fn main() {
    println!("Hello from WASI!");
    
    for (i, arg) in std::env::args().enumerate() {
        println!("Arg {}: {}", i, arg);
    }
}
```

```bash
cargo build --target wasm32-wasip1 --release
# Output: target/wasm32-wasip1/release/myapp.wasm
```

## 10.4 Run with wasmtime

```bash
# Install:
curl https://wasmtime.dev/install.sh -sSf | bash

# Run:
wasmtime run target/wasm32-wasip1/release/myapp.wasm arg1 arg2

# Output:
# Hello from WASI!
# Arg 0: target/wasm32-wasip1/release/myapp.wasm
# Arg 1: arg1
# Arg 2: arg2
```

## 10.5 File system access

```rust
use std::fs;

fn main() -> std::io::Result<()> {
    let content = fs::read_to_string("input.txt")?;
    println!("Got: {}", content);
    
    fs::write("output.txt", content.to_uppercase())?;
    Ok(())
}
```

```bash
wasmtime run --dir=. myapp.wasm
# --dir=. grants access to current directory
# Without --dir, no FS access (sandboxed)
```

WASI is **capability-based** — must explicitly allow file/network access. Tighter security than native.

## 10.6 wasmtime as library

Embed WASM in Rust host:
```toml
[dependencies]
wasmtime = "26"
wasmtime-wasi = "26"
```

```rust
use wasmtime::*;
use wasmtime_wasi::WasiCtx;
use wasmtime_wasi::sync::WasiCtxBuilder;

fn run_wasm() -> wasmtime::Result<()> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "myapp.wasm")?;
    
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s: &mut WasiCtx| s)?;
    
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .build();
    
    let mut store = Store::new(&engine, wasi);
    let instance = linker.instantiate(&mut store, &module)?;
    
    let func = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    func.call(&mut store, ())?;
    
    Ok(())
}
```

→ Run WASM module from your Rust host. Plugin system!

## 10.7 Use cases for WASI

- **Plugin systems**: app loads untrusted WASM safely
- **Multi-language**: run Go, Python (compiled), Rust uniformly
- **Edge computing**: Fastly Compute@Edge, Cloudflare Workers
- **Containers replacement**: smaller, faster cold start
- **Serverless**: per-request isolation
- **Universal deployment**: same .wasm everywhere

---

# Tầng 11: Cloudflare Workers, Fastly, Edge platforms

## 11.1 Edge computing với WASM

```
   Traditional servers:
   ────────────────────
   User → DNS → Load balancer → Server (1 region) → DB
   Latency: ~100ms+
   
   Edge platforms:
   ───────────────
   User → DNS → Edge node (nearest to user, 100+ locations)
        ↓
   WASM module runs HERE (cold start ~5ms)
   ↓
   Maybe forward to origin
   
   Latency: ~10-50ms
   Cold start: ~ms (vs ~seconds for container)
```

WASM is **perfect** for edge:
- Tiny binaries (KB, not MB)
- Fast cold start
- Sandboxed
- Multi-tenant safe

## 11.2 Cloudflare Workers (Rust)

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
worker = "0.4"
```

```rust
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Response::ok("Hello from Cloudflare Workers!")
}
```

Build + deploy:
```bash
npx wrangler init my-worker --lang=rust
npx wrangler dev      # local
npx wrangler deploy   # to Cloudflare network
```

Now your Rust runs on Cloudflare's 300+ edge locations.

## 11.3 Worker với routing

```rust
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get_async("/users/:id", |_req, ctx| async move {
            let id = ctx.param("id").unwrap();
            // Fetch from D1 (Cloudflare's SQLite)
            // ... or KV store ...
            Response::ok(format!("User {}", id))
        })
        .post_async("/users", |mut req, ctx| async move {
            let body = req.text().await?;
            Response::ok(format!("Created: {}", body))
        })
        .run(req, env)
        .await
}
```

Like axum but for Workers.

## 11.4 KV store

```rust
let kv = env.kv("MY_NAMESPACE")?;

// Read
let value: Option<String> = kv.get("key").text().await?;

// Write
kv.put("key", "value")?.execute().await?;

// Delete
kv.delete("key").await?;
```

Eventually consistent, geo-distributed KV. Fast reads at edge.

## 11.5 D1 (Cloudflare's SQLite)

```rust
let d1 = env.d1("DB")?;

let users = d1.prepare("SELECT * FROM users WHERE active = ?")
    .bind(&[true.into()])?
    .all::<User>()
    .await?;
```

SQLite replicated to edge regions. Reads local, writes propagate.

## 11.6 Durable Objects

For state with strong consistency:
```rust
#[durable_object]
pub struct Counter {
    state: State,
    env: Env,
}

#[durable_object]
impl DurableObject for Counter {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }
    
    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let storage = self.state.storage();
        let count: i32 = storage.get("count").await.unwrap_or(0);
        let new_count = count + 1;
        storage.put("count", new_count).await?;
        Response::ok(format!("{}", new_count))
    }
}
```

Single-threaded actor with persistent state. Per-object instance.

## 11.7 Fastly Compute@Edge

Similar concept, different SDK:

```toml
[dependencies]
fastly = "0.11"
```

```rust
use fastly::http::{StatusCode, Method};
use fastly::{Request, Response};

#[fastly::main]
fn main(req: Request) -> Result<Response, fastly::Error> {
    match (req.get_method(), req.get_path()) {
        (&Method::GET, "/") => Ok(Response::from_status(StatusCode::OK).with_body("Hello!")),
        _ => Ok(Response::from_status(StatusCode::NOT_FOUND)),
    }
}
```

Build & deploy via Fastly CLI. Similar workflow.

## 11.8 Comparison

```
   ┌──────────────────────────────────────────────────────────┐
   │ Platform           │ Runtime    │ Languages    │ Notes   │
   ├──────────────────────────────────────────────────────────┤
   │ Cloudflare Workers │ V8 isolates│ JS, WASM     │ Largest │
   │                    │            │ (Rust, Go)   │ network │
   │ Fastly Compute@Edge│ Wasmtime    │ WASM only    │ Pure   │
   │                    │            │ (Rust, JS,   │ WASM   │
   │                    │            │ Go via WASM) │        │
   │ Deno Deploy        │ V8         │ JS, TS, WASM │        │
   │ AWS Lambda@Edge    │ Node.js    │ JS           │ Less   │
   │                    │            │              │ flex   │
   │ Akamai EdgeWorkers │ V8         │ JS           │        │
   └──────────────────────────────────────────────────────────┘
```

WASM = portability across platforms. Same Rust code → Cloudflare + Fastly + own host.

---

# Tầng 12: Component Model — Tương lai của WASM

## 12.1 Component Model

WASM Core: just bytecode + linear memory + imports/exports. Limited types.

**Component Model** = higher-level structure:
- Rich types (strings, lists, records, variants, options)
- Interface types via WIT (WebAssembly Interface Types)
- Cross-language composition
- Resource ownership

→ Better composability across languages without manual bindgen.

## 12.2 WIT (WebAssembly Interface Types)

```wit
// counter.wit
package my:counter;

interface counter {
    record state {
        value: u64,
        max: u64,
    }
    
    new: func(max: u64) -> state;
    increment: func(state: state) -> state;
}

world component {
    export counter;
}
```

WIT files define interfaces — like protobuf or OpenAPI, but for WASM.

## 12.3 Rust + Component Model

```toml
[dependencies]
wit-bindgen = "0.30"
```

```rust
wit_bindgen::generate!({
    world: "component",
    path: "wit/counter.wit",
});

struct Counter;

impl Guest for Counter {
    fn new(max: u64) -> exports::counter::State {
        exports::counter::State { value: 0, max }
    }
    
    fn increment(s: exports::counter::State) -> exports::counter::State {
        exports::counter::State {
            value: (s.value + 1).min(s.max),
            max: s.max,
        }
    }
}

export!(Counter);
```

Build:
```bash
cargo build --target wasm32-wasip2 --release
```

Output is a **Component** (not just core WASM). Includes interface metadata.

## 12.4 Use a Component from any language

```bash
# Rust → Component, used from JS:
wasmtime run --invoke 'increment({value: 5, max: 100})' component.wasm
# Output: {value: 6, max: 100}
```

Or in JS host:
```javascript
import { Counter } from './component.wasm';
const state = Counter.new(100);
const next = Counter.increment(state);
```

Or in Go, Python, etc. Same .wasm, multiple languages consume.

## 12.5 Status

Component Model in 2024:
- ✅ Standardized (Preview 2)
- ✅ wasmtime supports
- ✅ wasm-tools, wit-bindgen mature
- ⚠️ Browser support: minimal yet
- ⚠️ Less ecosystem than wasm-bindgen

For server-side, edge: Component Model is the future.
For browser today: wasm-bindgen still default.

## 12.6 Use cases

- **Plugin systems**: load 3rd-party Components safely
- **Microservices**: compose components written in different languages
- **Polyglot apps**: Rust core + Python ML + JS UI all as components
- **Universal modules**: single .wasm runs everywhere

---

# Tầng 13: Performance — Optimize WASM

## 13.1 Binary size optimization

```toml
[profile.release]
opt-level = "z"        # smallest (sometimes "s" faster)
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Then run **wasm-opt**:
```bash
wasm-opt -Oz -o output.wasm input.wasm
```

`-Oz` aggressive size optimization. Typical results:
- Cargo release: 500 KB
- + opt-level=z: 300 KB
- + lto: 200 KB
- + strip: 180 KB
- + wasm-opt -Oz: 100 KB

## 13.2 Use wee_alloc instead of dlmalloc

Default Rust allocator (`dlmalloc`) adds ~10KB. Smaller alternative:

```toml
[dependencies]
wee_alloc = "0.4"
```

```rust
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

Saves ~10KB. Slightly slower allocation but rarely matters.

## 13.3 Avoid std types in hot path

```rust
// ❌ String allocation
#[wasm_bindgen]
pub fn process(input: &str) -> String {
    input.to_uppercase()
}

// ✅ Use slice when possible
#[wasm_bindgen]
pub fn process_bytes(input: &[u8], out: &mut [u8]) {
    for (i, b) in input.iter().enumerate() {
        out[i] = b.to_ascii_uppercase();
    }
}
```

String allocation = heap call + marshalling cost.

## 13.4 Minimize JS ↔ WASM boundary crossings

```rust
// ❌ Many small calls
for (let i = 0; i < 1000; i++) {
    wasm.process_one(i);   // 1000 boundary crossings!
}

// ✅ Batch
wasm.process_batch(Array.from({length: 1000}, (_, i) => i));
```

Each boundary call has overhead (~ns to µs). Batch when possible.

## 13.5 Use typed arrays

```javascript
// Slow: pass array
wasm.process([1, 2, 3, 4, 5]);   // marshal to Vec

// Fast: typed array
const arr = new Int32Array([1, 2, 3, 4, 5]);
wasm.process(arr);   // direct memory copy
```

Typed arrays match WASM linear memory layout. Faster marshal.

## 13.6 SIMD

WASM has SIMD instructions (since Preview 2 / 128-bit SIMD spec):

```rust
#[cfg(target_feature = "simd128")]
use std::arch::wasm32::*;

#[wasm_bindgen]
pub fn sum_simd(v: &[i32]) -> i32 {
    let mut sum = 0;
    for chunk in v.chunks_exact(4) {
        unsafe {
            let vec = v128_load(chunk.as_ptr() as *const v128);
            // ... SIMD ops ...
        }
    }
    sum
}
```

Compile with: `RUSTFLAGS='-C target-feature=+simd128' cargo build`

4x+ speedup for numeric loops.

## 13.7 Profiling WASM

```javascript
performance.mark('start');
wasm.heavyFunction();
performance.mark('end');
performance.measure('heavy', 'start', 'end');
```

Browser DevTools → Performance tab → see WASM execution.

For deeper profiling:
- `chrome://tracing` for low-level
- Firefox profiler with WASM source maps

## 13.8 Streaming compilation

```javascript
// ❌ Wait for fetch then compile
const bytes = await fetch('app.wasm').then(r => r.arrayBuffer());
const module = await WebAssembly.compile(bytes);

// ✅ Stream + compile in parallel
const module = await WebAssembly.compileStreaming(fetch('app.wasm'));
```

Streaming compile = parse while downloading. Faster.

`wasm-bindgen`-generated `init()` uses streaming by default.

## 13.9 Lazy loading

```javascript
// Don't load WASM upfront
button.addEventListener('click', async () => {
    const wasm = await import('./pkg/myapp.js');
    wasm.heavyOperation();
});
```

Code-split — WASM only loaded when needed.

## 13.10 Caching

```javascript
// Browser caches .wasm based on HTTP headers
// Use proper Cache-Control + hashed filenames

// IndexedDB cache for offline:
import { precacheWasm } from './pwa';
await precacheWasm('myapp_v1.wasm');
```

WASM modules cache-friendly. Use for offline-first apps.

---

# Tầng 14: Testing WASM

## 14.1 Standard cargo test

Unit tests in Rust code (non-WASM):
```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
```

```bash
cargo test    # runs on native, not WASM
```

Catches bugs early. Faster than WASM tests.

## 14.2 wasm-bindgen-test (browser tests)

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
```

```rust
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_dom() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    assert!(document.create_element("div").is_ok());
}
```

```bash
wasm-pack test --headless --firefox
# Or --chrome, --safari
```

Runs in actual browser (headless). Verifies DOM + browser API integration.

## 14.3 Mocha-style tests

Many projects: write WASM, test via JS:
```javascript
// __tests__/integration.test.js
import init, { add } from '../pkg/myapp.js';

beforeAll(async () => {
    await init();
});

test('add', () => {
    expect(add(1, 2)).toBe(3);
});
```

Jest, Vitest, Mocha — all work after `init()` in setup.

## 14.4 WASI tests

```bash
# Run WASM tests via wasmtime:
cargo test --target wasm32-wasip1
# Auto wraps test runner
```

Works for tests not needing browser APIs.

## 14.5 Property-based testing

proptest works in WASM:
```rust
use proptest::prelude::*;

#[wasm_bindgen_test]
fn test_reverse_twice() {
    proptest!(|(s in ".*")| {
        let r = reverse(&reverse(&s));
        prop_assert_eq!(r, s);
    });
}
```

## 14.6 CI for WASM

```yaml
# GitHub Actions
- uses: actions/checkout@v4
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown, wasm32-wasip1

- name: Install wasm-pack
  run: cargo install wasm-pack

- name: Run native tests
  run: cargo test

- name: Run WASM tests
  run: wasm-pack test --headless --firefox

- name: Build for production
  run: wasm-pack build --release
```

---

# Tầng 15: Patterns & Antipatterns

## 15.1 ✅ Pattern: WASM for compute, JS for I/O

```javascript
// JS: orchestration, DOM, fetch
async function processImage(url) {
    const imageData = await fetch(url).then(r => r.arrayBuffer());
    const pixels = new Uint8Array(imageData);
    
    // WASM: heavy pixel processing
    const filtered = wasm.applyFilter(pixels);
    
    // JS: display
    return renderToCanvas(filtered);
}
```

Split: JS for ecosystem things, WASM for math.

## 15.2 ✅ Pattern: Reuse Rust code on backend + WASM

```rust
// shared crate — pure logic
pub fn validate_email(s: &str) -> bool { ... }
pub fn calculate_tax(amount: f64, rate: f64) -> f64 { ... }

// Compile for native (server) AND WASM (client)
// Same code, runs both places
```

Avoid duplicate logic in JS/Rust.

## 15.3 ✅ Pattern: Plugins via WASM

Use WASM as plugin format:
```rust
// Host (your Rust app):
let module = Module::from_file(&engine, plugin.wasm)?;
let instance = Linker::new(&engine).instantiate(&mut store, &module)?;
let process = instance.get_typed_func::<i32, i32>(&mut store, "process")?;

let result = process.call(&mut store, 42)?;
```

3rd party uploads .wasm → your app loads, sandboxed.

## 15.4 ✅ Pattern: Optimistic UI + WASM verification

```javascript
// User clicks button
showOptimisticResult();  // immediate JS feedback

// Background: WASM computes actual result
wasm.computeResult().then(actual => {
    if (actual !== optimisticGuess) {
        showCorrection();
    }
});
```

Best of both: snappy UI + accurate computation.

## 15.5 ❌ Antipattern: Tiny functions in WASM

```rust
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

Each call has IPC overhead. JS does this faster.

WASM wins on:
- Functions taking >100µs of work
- Compute over big data
- Repeated calls without crossing boundary

## 15.6 ❌ Antipattern: Heavy DOM manipulation from Rust

```rust
for i in 0..1000 {
    let el = document.create_element("div")?;
    body.append_child(&el)?;
}
```

Each Rust→DOM crossing has cost. Slower than:
```javascript
// In JS
let html = '';
for (let i = 0; i < 1000; i++) html += '<div></div>';
document.body.innerHTML += html;
```

DOM is JS-native. Use Rust for DOM only when computing + manipulating tightly coupled.

## 15.7 ❌ Antipattern: Forget to free

```javascript
const c = new Counter();
c.increment();
// c never freed → memory leak in WASM heap!
```

Modern wasm-bindgen has FinalizationRegistry-based auto-cleanup, but partial. Best practice:
```javascript
const c = new Counter();
try {
    c.increment();
} finally {
    c.free();
}
```

Or use disposable pattern.

## 15.8 ❌ Antipattern: Large strings everywhere

```rust
#[wasm_bindgen]
pub fn process_all(text: String) -> String {
    text.lines().map(|l| l.to_uppercase()).collect::<Vec<_>>().join("\n")
}
```

String marshal back/forth = expensive. For huge text:
- Process incrementally
- Use shared memory + offsets
- Or do it in JS (smaller per-call cost)

## 15.9 ❌ Antipattern: panic without console_error_panic_hook

```rust
#[wasm_bindgen]
pub fn risky() {
    panic!("oops");   // Silent abort in WASM!
}
```

Without hook: WASM aborts, no error in console. Hard to debug.

```rust
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}
```

Now panic shows stack trace. Always install.

## 15.10 ❌ Antipattern: Not lazy-loading

```javascript
import init from './pkg/myapp.js';   // synchronously loaded
await init();
```

User pays download cost even if not using WASM features.

```javascript
// Lazy:
async function useWasm() {
    const wasm = await import('./pkg/myapp.js');
    await wasm.default();
    return wasm;
}

button.onclick = async () => {
    const wasm = await useWasm();
    wasm.heavyOp();
};
```

Code-split. Improve initial page load.

---

# Tầng 16: Real-world apps & frameworks

## 16.1 Yew — React-like in Rust

```toml
[dependencies]
yew = { version = "0.21", features = ["csr"] }
```

```rust
use yew::prelude::*;

#[function_component]
fn App() -> Html {
    let counter = use_state(|| 0);
    
    let onclick = {
        let counter = counter.clone();
        move |_| counter.set(*counter + 1)
    };
    
    html! {
        <div>
            <p>{ "Count: " }{ *counter }</p>
            <button {onclick}>{ "Increment" }</button>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
```

React-like component model, in Rust. Compile entire SPA to WASM.

## 16.2 Leptos — Fine-grained reactivity

```toml
[dependencies]
leptos = "0.6"
```

```rust
use leptos::*;

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = create_signal(0);
    
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Click me: " {count}
        </button>
    }
}

fn main() {
    mount_to_body(App);
}
```

Signals = fine-grained reactivity (like SolidJS). Often faster than VDOM diffing.

## 16.3 Dioxus — React + Native

```rust
use dioxus::prelude::*;

fn App() -> Element {
    let mut count = use_signal(|| 0);
    
    rsx! {
        button {
            onclick: move |_| count += 1,
            "Clicked {count}"
        }
    }
}
```

Targets: Web, Desktop, Mobile, TUI. Single codebase.

## 16.4 Bevy — Game engine

```toml
[dependencies]
bevy = "0.14"
```

ECS-based game engine. Compiles to WASM:
```bash
cargo build --target wasm32-unknown-unknown --release
```

Games like [Helion Cubed](https://heliumcubed.itch.io/) run in browser via Bevy + WASM.

## 16.5 RustPython, Pyodide

Python interpreters compiled to WASM. Run Python in browser:
```javascript
import { loadPyodide } from 'pyodide';

const pyodide = await loadPyodide();
pyodide.runPython(`
    import numpy as np
    arr = np.array([1, 2, 3])
    print(arr.mean())
`);
```

Powerful — full Python ecosystem in browser. Heavy (10+ MB), but useful for ML demos, Jupyter-like.

## 16.6 1Password, Figma, Photoshop

Production examples:
- **1Password**: crypto in Rust → WASM, runs in browser
- **Figma**: heavy rendering in C++ → WASM
- **Photoshop Web**: 80% C++ → WASM, in browser
- **Google Earth**: 3D engine in WASM
- **Discord**: Rust → WASM for voice processing

Pattern: heavy compute → WASM, UI → JS framework.

## 16.7 Cloudflare Workers in production

```rust
// Real Cloudflare Worker handling API requests
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    
    router
        .get_async("/api/users/:id", |_req, ctx| async move {
            let id = ctx.param("id").ok_or("missing id")?;
            let db = ctx.env.d1("DB")?;
            let user = db.prepare("SELECT * FROM users WHERE id = ?")
                .bind(&[id.into()])?
                .first::<User>(None).await?;
            
            match user {
                Some(u) => Response::from_json(&u),
                None => Response::error("Not found", 404),
            }
        })
        .post_async("/api/users", |mut req, ctx| async move {
            let user: User = req.json().await?;
            // ... save to D1 ...
            Response::from_json(&user)
        })
        .run(req, env)
        .await
}
```

Scale to billions of requests. Edge-distributed Rust.

## 16.8 Choose framework

```
   Yew         — React-like, mature, simple
   Leptos      — SolidJS-like, very fast
   Dioxus      — React + multi-target
   Sycamore    — SolidJS-like alternative
   Sauron      — Elm-like (HTML-only)
   Iced        — Elm-like (native + WASM)
   egui        — Immediate mode (game UI)
   
   For SPA: Yew/Leptos/Dioxus
   For games: Bevy + egui
   For server-side rendering: Leptos (SSR + hydration)
```

---

# Tổng kết — 12 nguyên tắc senior WASM

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. WASM = compute-heavy code, JS = orchestration + DOM.          │
│                                                                  │
│ 2. wasm-bindgen + wasm-pack for browser. WASI for server/edge.   │
│                                                                  │
│ 3. Minimize JS ↔ WASM boundary crossings. Batch.                 │
│                                                                  │
│ 4. Primitives free, strings/Vec cost, serde slowest.             │
│                                                                  │
│ 5. console_error_panic_hook ALWAYS in dev.                       │
│                                                                  │
│ 6. Manual free() — WASM has no GC. Or use FinalizationRegistry.  │
│                                                                  │
│ 7. opt-level="z" + lto + wasm-opt for size.                      │
│                                                                  │
│ 8. wee_alloc to save ~10KB binary.                               │
│                                                                  │
│ 9. Web Worker for heavy WASM — don't block main thread.          │
│                                                                  │
│ 10. WASI = capability-based security. Grant least privilege.     │
│                                                                  │
│ 11. Edge platforms (Cloudflare/Fastly) = WASM perfect fit.       │
│                                                                  │
│ 12. Component Model is future. Use for new server-side WASM.     │
└──────────────────────────────────────────────────────────────────┘
```

---

# WASM toolkit

| Tool / Crate | Purpose |
|--------------|---------|
| `wasm-bindgen` | JS interop bindings |
| `wasm-pack` | Build pipeline |
| `wasm-bindgen-cli` | CLI tool |
| `web-sys` | Web API bindings |
| `js-sys` | JS stdlib bindings |
| `wasm-bindgen-futures` | async/Promise interop |
| `serde-wasm-bindgen` | serde + JsValue |
| `gloo` | Convenience helpers |
| `console_error_panic_hook` | Panic in console |
| `wee_alloc` | Small allocator |
| `wasm-opt` (binaryen) | Optimize .wasm |
| `wabt` | WebAssembly Binary Toolkit |
| `wasmtime` | WASI runtime |
| `wasmer` | Alt WASI runtime |
| `wit-bindgen` | Component Model bindings |
| `worker` (Cloudflare) | CF Workers SDK |
| `fastly` | Fastly Compute SDK |
| `yew`, `leptos`, `dioxus` | Web frameworks |
| `bevy` | Game engine |
| `trunk` | Like wasm-pack for SPAs |

---

# Bộ tài liệu Rust giờ có 20 chương!

```
   📚 RUST FOUNDATIONS LIBRARY
   
   Phần I:   a, b, c, d, e          (foundations)
   Phần II:  f, g, h                 (concurrency + errors)
   Phần III: i, j                    (memory advanced)
   Phần IV:  k, l, m, n, o           (production)
   Phần V:   p, q, r, s, t           (applications)
   
   ───────────────────────────────────────
   a. memory-model         k. performance
   b. ownership-borrowing  l. observability
   c. trait                m. iterator
   d. generic              n. unsafe-rust
   e. closure              o. testing
   f. async                p. embedded-rust
   g. error-handling       q. axum-project
   h. macros               r. database
   i. smart-pointers       s. tauri
   j. lifetime             t. wasm           ← MỚI
   
   Tổng: 20 chương × 2 files = 40 files
```

Bộ kỹ năng full-stack Rust giờ phủ:
- 🌐 Web (axum)
- 🗄️ Database (sqlx)
- 🔌 Embedded (no_std)
- 🖥️ Desktop (Tauri)
- 📱 Mobile (Tauri v2)
- 🌍 **WASM** (browser + edge + server) ← MỚI
- 🚀 Performance
- 🧪 Testing
- 🔍 Observability

Còn nhiều domain có thể đào sâu nếu muốn:
- **Game engines** (Bevy ECS)
- **CLI tools** (clap, dialoguer)
- **GUI native** (egui, iced)
- **gRPC** (tonic)
- **Cryptography** (rustls, ring)
- **OS kernels** (Redox)

Báo nếu muốn tiếp! 🦀⚡
