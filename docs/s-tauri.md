# Tauri — Desktop & Mobile App với Rust

> Tài liệu thứ 19 trong bộ Rust nền tảng. Đọc trước:
> - [async.md](./async.md) — Tauri commands chạy async
> - [error-handling.md](./error-handling.md) — error qua IPC
> - [smart-pointers.md](./smart-pointers.md) — Arc state management
> - [observability.md](./observability.md) — log trong Tauri
> - [axum-project.md](./axum-project.md) — Tauri đôi khi nhúng axum
>
> **Tauri** = framework để xây desktop apps với **Rust backend + WebView frontend**.
> 
> So với Electron:
> - **Nhỏ hơn 10-50x** (5-10MB vs 100-200MB)
> - **Nhanh hơn, ít RAM** (system webview vs bundled Chromium)
> - **An toàn hơn** (Rust + capability model)
> - **Cross-platform**: Windows, macOS, Linux, iOS, Android (v2)
> - **Frontend agnostic**: React, Vue, Svelte, Solid, ... bất cứ thứ gì compile to HTML/JS
>
> Tài liệu này dạy bạn xây Tauri app **production-ready**: IPC, state, security, plugins, packaging.

---

# Mục lục

- [Tầng 1: Tauri là gì? Vs Electron](#tầng-1-tauri-là-gì-vs-electron)
- [Tầng 2: Architecture — Rust + WebView](#tầng-2-architecture--rust--webview)
- [Tầng 3: Project setup — Tauri v2](#tầng-3-project-setup--tauri-v2)
- [Tầng 4: IPC — Commands](#tầng-4-ipc--commands)
- [Tầng 5: IPC — Events](#tầng-5-ipc--events)
- [Tầng 6: State management](#tầng-6-state-management)
- [Tầng 7: Window management](#tầng-7-window-management)
- [Tầng 8: Menu, tray, dialog](#tầng-8-menu-tray-dialog)
- [Tầng 9: File system access](#tầng-9-file-system-access)
- [Tầng 10: Security — Capabilities & permissions](#tầng-10-security--capabilities--permissions)
- [Tầng 11: Plugins ecosystem](#tầng-11-plugins-ecosystem)
- [Tầng 12: Frontend integration patterns](#tầng-12-frontend-integration-patterns)
- [Tầng 13: Packaging & Distribution](#tầng-13-packaging--distribution)
- [Tầng 14: Updater](#tầng-14-updater)
- [Tầng 15: Mobile — iOS & Android (v2)](#tầng-15-mobile--ios--android-v2)
- [Tầng 16: Performance & best practices](#tầng-16-performance--best-practices)

---

# Tầng 1: Tauri là gì? Vs Electron

## 1.1 Desktop app frameworks landscape

| Framework | Backend | Frontend | Binary size | RAM | Notes |
|-----------|---------|----------|-------------|-----|-------|
| **Electron** | Node.js | Chromium bundled | 100-200MB | High | Slack, VS Code, Discord |
| **Tauri** | Rust | System webview | 5-15MB | Low | Modern, secure |
| **Wails** | Go | System webview | 10-20MB | Low | Like Tauri but Go |
| **Flutter Desktop** | Dart | Custom rendering | 30-50MB | Medium | Mobile-first |
| **Qt** | C++ | Qt widgets | 20-50MB | Medium | Native widgets |
| **egui** (Rust) | Rust | Immediate mode GUI | 5-10MB | Low | Pure Rust, no webview |
| **iced** (Rust) | Rust | Elm-style | 5-10MB | Low | Pure Rust |

## 1.2 Why Tauri specifically?

### Vs Electron
```
   Electron app:                Tauri app:
   ─────────────                 ─────────
   Chromium: 100MB+              System webview: 0MB extra
   Node.js: 30MB+                Rust binary: 5-10MB
   Total: ~150MB                 Total: ~10MB
   
   RAM at idle: 200-500MB        RAM at idle: 30-80MB
   Startup: 1-3s                 Startup: <500ms
```

### Vs Pure Rust GUI (egui, iced)
```
   Pure Rust (egui):             Tauri:
   ─────────────                 ──────
   ✅ Single binary               ✅ Reuse web ecosystem
   ✅ No webview overhead         ✅ React/Vue/Svelte UI libraries
   ✅ Maximum control             ✅ CSS, animations easy
   ❌ Limited UI ecosystem        ❌ Webview overhead
   ❌ Custom rendering             ❌ Frontend complexity
```

**Choose Tauri khi:**
- Có frontend team / familiar với React/Vue/Svelte
- Cần UI phức tạp (CSS, animations, design systems)
- Cross-platform desktop + (now) mobile
- Cần plugin ecosystem (camera, geolocation, ...)

**Choose pure Rust khi:**
- Single binary critical (no install)
- Minimal RAM (embedded, kiosk)
- Performance-critical UI (game UI, audio editor)

## 1.3 Tauri v2 — Big changes

Tauri 2.0 (2024+) brings:
- ✅ **Mobile support** (iOS, Android)
- ✅ **Improved IPC** — faster, simpler
- ✅ **Capability-based security** (better than allowlist)
- ✅ **Plugin system** mature (camera, file system, notification, ...)
- ✅ **Better TypeScript bindings**

Tài liệu này focus **Tauri v2**. v1 vẫn used widely, syntax similar.

## 1.4 Mental model

```rust
// Backend (Rust)
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::main]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running");
}
```

```typescript
// Frontend (TypeScript)
import { invoke } from '@tauri-apps/api/core';

const message = await invoke<string>('greet', { name: 'World' });
console.log(message);  // "Hello, World!"
```

That's it. Rust function callable from frontend. The "magic" is IPC infrastructure.

---

# Tầng 2: Architecture — Rust + WebView

## 2.1 Process model

```
┌─────────────────────────────────────────────────────────────┐
│                  Tauri App (single process)                 │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Rust Core (main thread + tokio runtime)            │    │
│  │                                                     │    │
│  │   - tauri::Builder                                  │    │
│  │   - Window management                                │   │
│  │   - Command handlers (#[tauri::command])             │   │
│  │   - Event emitters                                   │   │
│  │   - State (Arc<Mutex<T>>)                            │   │
│  │   - Plugin runtime                                   │   │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                │
│                IPC (async message channel)                  │
│                            │                                │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  WebView (separate thread / process)                │    │
│  │                                                     │    │
│  │   - WebKit / WebKit2GTK (Linux/macOS)                │   │
│  │   - WebView2 / Edge (Windows)                        │   │
│  │   - Renders HTML/CSS/JS                              │   │
│  │   - React/Vue/Svelte app                             │   │
│  │   - @tauri-apps/api JS bindings                      │   │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

WebView dùng **system component** (no bundled Chromium):
- Windows: WebView2 (Edge runtime)
- macOS: WKWebView
- Linux: WebKitGTK

User OS đã có sẵn → app size nhỏ.

## 2.2 Comparison với Electron architecture

```
Electron:
─────────
Main Process (Node.js)    Renderer Process (Chromium)
       │                          │
       └──── IPC ─────────────────┘
       
Bundled: Node.js + Chromium ~150MB

Tauri:
──────
Rust binary (main thread + tokio)  WebView (system)
       │                                  │
       └──── IPC ─────────────────────────┘

Bundled: Rust binary ~10MB only
```

WebView is "lighter" because not bundled.

## 2.3 IPC — How Rust and JS talk

Tauri provides 2 mechanisms:
- **Commands**: JS → Rust (request/response, like RPC)
- **Events**: Bidirectional pub/sub (fire-and-forget)

Both async + serializable via serde.

## 2.4 Threads

```
Main thread (Rust):
   ┌────────────────────────────────────┐
   │ tokio runtime running              │
   │   ├─ command handlers              │
   │   ├─ event listeners               │
   │   └─ background tasks (spawn)      │
   └────────────────────────────────────┘
   
WebView (separate thread/process):
   ┌────────────────────────────────────┐
   │ JS engine                          │
   │   ├─ React/Vue/...                 │
   │   ├─ IPC client                    │
   │   └─ DOM rendering                 │
   └────────────────────────────────────┘
```

Rust code CAN'T touch DOM directly. Must emit events.
JS CAN'T touch system files directly. Must invoke commands.

This separation = security boundary.

## 2.5 Tauri v2 stack

```
Your App:
  ├─ Frontend: React + TypeScript (or Vue/Svelte/Solid)
  │
  ├─ Backend: Rust
  │    ├─ tauri (core)
  │    ├─ tokio (async runtime)
  │    ├─ serde (serialization)
  │    └─ Tauri plugins (file-system, dialog, http, ...)
  │
  └─ Builds for:
       Windows (MSI, NSIS)
       macOS (DMG, App bundle)
       Linux (DEB, RPM, AppImage)
       iOS / Android (Tauri v2)
```

---

# Tầng 3: Project setup — Tauri v2

## 3.1 Prerequisites

```bash
# Rust:
rustup default stable

# Node.js (for frontend):
nvm install 20

# Tauri prerequisites (Linux):
sudo apt install libwebkit2gtk-4.1-dev build-essential \
    curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev \
    librsvg2-dev

# macOS: nothing extra (Xcode CLI tools)
# Windows: WebView2 runtime + MSVC build tools
```

## 3.2 Create new project

```bash
npm create tauri-app@latest my-app
# Pick:
#   - Frontend: React + TypeScript (or Vue/Svelte/...)
#   - Package manager: npm/pnpm/yarn

cd my-app
npm install

npm run tauri dev    # development mode with hot-reload
```

Creates:
```
my-app/
├── src/                  # Frontend (React/Vue/...)
├── src-tauri/            # Rust backend
│   ├── src/
│   │   └── main.rs       # Tauri entry
│   ├── Cargo.toml
│   ├── tauri.conf.json   # config
│   ├── icons/
│   └── capabilities/     # security capabilities
└── package.json          # Frontend deps
```

## 3.3 src-tauri/Cargo.toml

```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tauri-build = "2"

[features]
custom-protocol = ["tauri/custom-protocol"]
```

## 3.4 src-tauri/tauri.conf.json

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "MyApp",
  "version": "0.1.0",
  "identifier": "com.example.myapp",
  
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  
  "app": {
    "windows": [
      {
        "title": "MyApp",
        "width": 800,
        "height": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'"
    }
  },
  
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

Cấu hình toàn bộ app: window size, icons, bundle targets, security CSP.

## 3.5 src-tauri/src/main.rs

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`#![cfg_attr(...)]`: hide console window on Windows release builds.

## 3.6 Dev workflow

```bash
npm run tauri dev      # Hot-reload Rust + Frontend
npm run tauri build    # Production build
```

Hot reload:
- Frontend changes: auto-refresh webview
- Rust changes: recompile + relaunch app (~5-15s)

---

# Tầng 4: IPC — Commands

## 4.1 Basic command

```rust
#[tauri::command]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Register:
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![add])
```

```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<number>('add', { a: 1, b: 2 });
console.log(result);  // 3
```

Auto-serialization. Args/return must implement `serde::Serialize`/`Deserialize`.

## 4.2 Async commands

```rust
#[tauri::command]
async fn fetch_user(id: u64) -> Result<User, String> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id as i64)
        .fetch_one(&db).await
        .map_err(|e| e.to_string())?;
    Ok(user)
}
```

Async functions work natively (tokio runtime in Tauri).

Frontend:
```typescript
try {
    const user = await invoke<User>('fetch_user', { id: 42 });
} catch (err) {
    console.error(err);
}
```

## 4.3 Complex types

```rust
#[derive(serde::Deserialize)]
struct CreateUserRequest {
    email: String,
    name: String,
    age: u32,
}

#[derive(serde::Serialize)]
struct User {
    id: u64,
    email: String,
    name: String,
}

#[tauri::command]
fn create_user(req: CreateUserRequest) -> Result<User, String> {
    if req.age < 18 {
        return Err("must be 18+".into());
    }
    Ok(User {
        id: 1,
        email: req.email,
        name: req.name,
    })
}
```

```typescript
const user = await invoke<User>('create_user', {
    req: {
        email: 'alice@test.com',
        name: 'Alice',
        age: 30,
    }
});
```

Field naming: by default Tauri uses **camelCase in TS, snake_case in Rust**. Specify with serde rename if needed.

## 4.4 Error handling

```rust
use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("validation failed: {0}")]
    Validation(String),
    
    #[error("database error")]
    Database(String),
    
    #[error("not found")]
    NotFound,
}

// Implement Serialize for IPC
impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tauri::command]
fn risky_op(input: String) -> Result<String, CommandError> {
    if input.is_empty() {
        return Err(CommandError::Validation("empty input".into()));
    }
    Ok(input.to_uppercase())
}
```

```typescript
try {
    const result = await invoke<string>('risky_op', { input: '' });
} catch (err) {
    // err = "validation failed: empty input"
    console.error(err);
}
```

`Result<T, E>` → frontend sees Promise. `Err` → Promise rejects.

For typed errors, use TS discriminated union:
```typescript
type CommandError = 
    | { kind: 'Validation', message: string }
    | { kind: 'Database', message: string }
    | { kind: 'NotFound' };
```

Pair with Rust:
```rust
#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum CommandError {
    Validation { message: String },
    Database { message: String },
    NotFound,
}
```

## 4.5 Access AppHandle in command

```rust
#[tauri::command]
async fn save_config(
    app: tauri::AppHandle,
    config: Config,
) -> Result<(), String> {
    let path = app.path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    let config_path = path.join("config.json");
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap())
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

`AppHandle` gives access to:
- App paths (data, config, cache, log dirs)
- Window manipulation
- Plugin instances
- Emit events
- Spawn tasks

## 4.6 Access State

```rust
struct AppState {
    db: PgPool,
}

#[tauri::command]
async fn get_user(
    state: tauri::State<'_, AppState>,
    id: u64,
) -> Result<User, String> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id as i64)
        .fetch_one(&state.db).await
        .map_err(|e| e.to_string())
}

// Register state:
tauri::Builder::default()
    .manage(AppState { db: pool })
    .invoke_handler(...)
```

State must be `Send + Sync + 'static`. Shared across commands.

## 4.7 Streams via Channel

For long operations (file processing, downloads):

```rust
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    progress: u32,
    total: u32,
}

#[tauri::command]
async fn process_file(channel: Channel<ProgressEvent>, path: String) -> Result<(), String> {
    let total = 100;
    for i in 0..total {
        // Simulate work
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        channel.send(ProgressEvent { progress: i + 1, total })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

```typescript
import { Channel, invoke } from '@tauri-apps/api/core';

const channel = new Channel<ProgressEvent>();
channel.onmessage = (event) => {
    console.log(`${event.progress}/${event.total}`);
};

await invoke('process_file', { channel, path: '/tmp/file' });
```

Stream progress → UI shows progress bar live.

## 4.8 Generate TS bindings (specta + tauri-specta)

Maintaining TS types by hand is painful. Use **specta**:

```toml
specta = "2"
tauri-specta = { version = "2", features = ["javascript", "typescript"] }
```

```rust
use specta::Type;

#[derive(Serialize, Deserialize, Type)]
struct User {
    id: u64,
    email: String,
}

#[tauri::command]
#[specta::specta]
fn get_user(id: u64) -> Result<User, String> { ... }

// In build / build.rs:
fn main() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![get_user]);
    
    #[cfg(debug_assertions)]
    builder.export(specta_typescript::Typescript::default(), "../src/bindings.ts")
        .unwrap();
}
```

Generates `src/bindings.ts` with typed wrappers:
```typescript
import { commands } from './bindings';

const user = await commands.getUser(42);   // typed!
```

Auto-sync TS ↔ Rust types. Production-grade DX.

---

# Tầng 5: IPC — Events

## 5.1 Backend → Frontend

```rust
use tauri::Emitter;

#[tauri::command]
async fn start_download(window: tauri::Window, url: String) -> Result<(), String> {
    tokio::spawn(async move {
        for i in 0..=100 {
            // Simulate work
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            window.emit("download-progress", i).unwrap();
        }
        window.emit("download-complete", url).unwrap();
    });
    Ok(())
}
```

```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<number>('download-progress', (event) => {
    console.log(`progress: ${event.payload}%`);
});

const unlisten2 = await listen<string>('download-complete', (event) => {
    console.log(`done: ${event.payload}`);
    unlisten();   // stop listening
    unlisten2();
});
```

Events: fire-and-forget. No response.

## 5.2 Frontend → Backend

```typescript
import { emit } from '@tauri-apps/api/event';

await emit('user-clicked-button', { button: 'save' });
```

```rust
use tauri::Listener;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            app.listen("user-clicked-button", move |event| {
                let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap();
                println!("got event: {:?}", payload);
            });
            Ok(())
        })
        ...
}
```

Less common direction — usually frontend uses commands.

## 5.3 Window-scoped vs global events

```rust
// Global — all windows receive:
app.emit("global-event", payload).unwrap();

// Specific window:
window.emit("window-event", payload).unwrap();

// Listen on specific window only:
window.listen("event", |event| { ... });
```

For multi-window apps, scope events appropriately.

## 5.4 Use case: background tasks

```rust
#[tauri::command]
async fn start_background_task(window: tauri::Window) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            
            let stats = collect_stats().await;
            window.emit("stats-update", stats).unwrap();
        }
    });
}
```

```typescript
await invoke('start_background_task');

await listen('stats-update', (event) => {
    updateUI(event.payload);
});
```

App start → background polling → UI updates real-time.

## 5.5 When commands vs events?

```
   COMMANDS:
   ─────────
   • Request/response (need return value)
   • Synchronous flow (save file → get result)
   • Single invocation
   • Errors propagate
   
   EVENTS:
   ───────
   • Notifications (progress, status)
   • One-to-many (multiple listeners)
   • Long-running background updates
   • Fire-and-forget
```

---

# Tầng 6: State management

## 6.1 Setup state

```rust
use std::sync::Mutex;

struct AppState {
    counter: Mutex<u32>,
    db: PgPool,
}

impl AppState {
    fn new(db: PgPool) -> Self {
        Self {
            counter: Mutex::new(0),
            db,
        }
    }
}

fn main() {
    let pool = futures::executor::block_on(connect_db());
    
    tauri::Builder::default()
        .manage(AppState::new(pool))
        .invoke_handler(tauri::generate_handler![increment, get_count])
        .run(tauri::generate_context!())
        .expect("error");
}
```

`.manage(...)` register state. Type-keyed (one instance per type).

## 6.2 Access in commands

```rust
#[tauri::command]
fn increment(state: tauri::State<'_, AppState>) -> u32 {
    let mut count = state.counter.lock().unwrap();
    *count += 1;
    *count
}

#[tauri::command]
fn get_count(state: tauri::State<'_, AppState>) -> u32 {
    *state.counter.lock().unwrap()
}
```

`tauri::State<'_, T>` extractor injects shared state.

## 6.3 Async state with tokio::sync

```rust
use tokio::sync::Mutex;

struct AppState {
    config: Mutex<Config>,
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, AppState>,
    new_config: Config,
) -> Result<(), String> {
    let mut config = state.config.lock().await;   // async lock
    *config = new_config;
    Ok(())
}
```

Use `tokio::sync::Mutex` if hold lock across `.await`. Else std `Mutex` is fine.

Apply [smart-pointers.md](./smart-pointers.md) Tầng 12 rules.

## 6.4 Multiple state types

```rust
struct DbState { pool: PgPool }
struct UserState { current_user: Mutex<Option<User>> }
struct ConfigState { config: Mutex<Config> }

tauri::Builder::default()
    .manage(DbState { pool })
    .manage(UserState { current_user: Mutex::new(None) })
    .manage(ConfigState { config: Mutex::new(config) })
```

```rust
#[tauri::command]
async fn login(
    db: tauri::State<'_, DbState>,
    user_state: tauri::State<'_, UserState>,
    config: tauri::State<'_, ConfigState>,
    creds: Credentials,
) -> Result<User, String> {
    // ... use all 3 ...
}
```

Multiple State<T> extractors in one command.

## 6.5 State trong async tasks

```rust
#[tauri::command]
async fn start_worker(app: tauri::AppHandle) {
    tokio::spawn(async move {
        let state = app.state::<DbState>();   // get state from AppHandle
        loop {
            // poll DB
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}
```

Spawned task can't borrow state — get fresh via `app.state::<T>()`.

## 6.6 Best practices

```
   ✅ Wrap with Arc in inner types:
   ─────────────────────────────────
   struct AppState {
       inner: Arc<AppStateInner>,
   }
   
   ✅ Use specific types per concern:
   ───────────────────────────────────
   DbState, UserState, ConfigState (don't lump all into one)
   
   ❌ Avoid global mutable static:
   ────────────────────────────────
   static COUNTER: AtomicU64 = AtomicU64::new(0);
   // Works but couples code to specific instance
   
   ✅ Atomic for simple counters:
   ───────────────────────────────
   struct AppState {
       count: AtomicU64,   // lock-free
   }
```

---

# Tầng 7: Window management

## 7.1 Define windows in config

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "MyApp",
        "width": 800,
        "height": 600,
        "minWidth": 400,
        "minHeight": 300,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "transparent": false,
        "alwaysOnTop": false,
        "visible": true,
        "url": "index.html"
      }
    ]
  }
}
```

## 7.2 Create window dynamically

```rust
use tauri::WebviewWindowBuilder;
use tauri::WebviewUrl;

#[tauri::command]
async fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    WebviewWindowBuilder::new(
        &app,
        "settings",                              // unique label
        WebviewUrl::App("settings.html".into()), // URL
    )
    .title("Settings")
    .inner_size(400.0, 300.0)
    .resizable(false)
    .build()
    .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

```typescript
// From frontend:
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

const win = new WebviewWindow('settings', {
    url: 'settings.html',
    title: 'Settings',
    width: 400,
    height: 300,
});

win.once('tauri://created', () => {
    console.log('window opened');
});
```

## 7.3 Window operations

```rust
#[tauri::command]
fn manipulate_window(window: tauri::Window) -> Result<(), String> {
    window.set_title("New Title").map_err(|e| e.to_string())?;
    window.maximize().map_err(|e| e.to_string())?;
    // window.minimize();
    // window.hide();
    // window.show();
    // window.close();
    // window.set_size(...);
    // window.set_position(...);
    // window.center();
    Ok(())
}
```

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

const win = getCurrentWindow();
await win.maximize();
await win.setTitle('New Title');
```

## 7.4 Multi-window communication

```rust
#[tauri::command]
fn send_to_all_windows(app: tauri::AppHandle, msg: String) -> Result<(), String> {
    app.emit("broadcast", msg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn send_to_specific(app: tauri::AppHandle, label: String, msg: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.emit("message", msg).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

Each window has unique label → addressable.

## 7.5 Window decorations & custom title bar

```json
{
  "decorations": false,    // remove native title bar
  "transparent": true       // transparent window
}
```

Then implement custom title bar in HTML/CSS:
```html
<div data-tauri-drag-region class="titlebar">
    <span>MyApp</span>
    <button onclick="...">×</button>
</div>
```

`data-tauri-drag-region` = element drag window.

---

# Tầng 8: Menu, tray, dialog

## 8.1 Application menu

```rust
use tauri::menu::{Menu, MenuItem, Submenu, PredefinedMenuItem};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let file_menu = Submenu::with_items(app.handle(), "File", true, &[
                &MenuItem::with_id(app.handle(), "new", "New", true, Some("CmdOrCtrl+N"))?,
                &MenuItem::with_id(app.handle(), "open", "Open...", true, Some("CmdOrCtrl+O"))?,
                &PredefinedMenuItem::separator(app.handle())?,
                &MenuItem::with_id(app.handle(), "quit", "Quit", true, Some("CmdOrCtrl+Q"))?,
            ])?;
            
            let edit_menu = Submenu::with_items(app.handle(), "Edit", true, &[
                &PredefinedMenuItem::cut(app.handle(), None)?,
                &PredefinedMenuItem::copy(app.handle(), None)?,
                &PredefinedMenuItem::paste(app.handle(), None)?,
            ])?;
            
            let menu = Menu::with_items(app.handle(), &[&file_menu, &edit_menu])?;
            
            app.set_menu(menu)?;
            
            app.on_menu_event(move |app, event| {
                match event.id.0.as_str() {
                    "new" => { /* ... */ }
                    "open" => { /* ... */ }
                    "quit" => app.exit(0),
                    _ => {}
                }
            });
            
            Ok(())
        })
        ...
}
```

Native menu with keyboard shortcuts. Cross-platform.

## 8.2 System tray (icon in notification area)

```rust
use tauri::tray::TrayIconBuilder;
use tauri::menu::{Menu, MenuItem};

let tray = TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_menu_event(|app, event| {
        match event.id.0.as_str() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        }
    })
    .build(app)?;
```

Tray icon → menu on click. Useful for "minimize to tray" pattern.

## 8.3 Dialog plugin

```toml
tauri-plugin-dialog = "2"
```

```rust
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn pick_file(app: tauri::AppHandle) -> Option<String> {
    let path = app.dialog()
        .file()
        .add_filter("Text", &["txt", "md"])
        .blocking_pick_file();
    
    path.map(|p| p.to_string())
}
```

```typescript
import { open, save, message, ask, confirm } from '@tauri-apps/plugin-dialog';

// File picker:
const file = await open({
    multiple: false,
    filters: [{ name: 'Text', extensions: ['txt', 'md'] }],
});

// Save dialog:
const path = await save({ filters: [{ name: 'JSON', extensions: ['json'] }] });

// Message:
await message('Hello!', { title: 'Info' });

// Confirmation:
const ok = await confirm('Delete file?', { title: 'Confirm', kind: 'warning' });
```

Native OS dialogs.

## 8.4 Notifications

```toml
tauri-plugin-notification = "2"
```

```typescript
import { isPermissionGranted, requestPermission, sendNotification } 
  from '@tauri-apps/plugin-notification';

let permission = await isPermissionGranted();
if (!permission) {
    permission = (await requestPermission()) === 'granted';
}

if (permission) {
    sendNotification({ title: 'Hi!', body: 'You have a new message' });
}
```

OS-native notifications.

---

# Tầng 9: File system access

## 9.1 fs plugin

```toml
tauri-plugin-fs = "2"
```

```typescript
import { readTextFile, writeTextFile, exists, mkdir, BaseDirectory } 
  from '@tauri-apps/plugin-fs';

// Read:
const content = await readTextFile('config.json', { 
    baseDir: BaseDirectory.AppData 
});

// Write:
await writeTextFile('config.json', '{"theme":"dark"}', {
    baseDir: BaseDirectory.AppData
});

// Check exists:
const e = await exists('cache', { baseDir: BaseDirectory.AppCache });
if (!e) {
    await mkdir('cache', { baseDir: BaseDirectory.AppCache, recursive: true });
}
```

Tauri provides paths via `BaseDirectory`:
- `AppData` — config (`~/.local/share/com.example.myapp/`)
- `AppConfig` — settings
- `AppCache` — cache files
- `AppLog` — logs
- `Document`, `Download`, `Home` — user dirs

## 9.2 Rust side

```rust
use tauri::Manager;

#[tauri::command]
fn read_config(app: tauri::AppHandle) -> Result<String, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config_path = app_data.join("config.json");
    
    std::fs::read_to_string(&config_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&app_data).map_err(|e| e.to_string())?;
    
    let config_path = app_data.join("config.json");
    std::fs::write(&config_path, content).map_err(|e| e.to_string())
}
```

`app.path()` provides standardized cross-platform paths.

## 9.3 Security — Limit fs access

Tauri **doesn't** allow arbitrary file system access by default. Must declare in capabilities.

Capability `default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:allow-app-read",
    "fs:allow-app-write",
    "fs:scope-app",
    "dialog:default"
  ]
}
```

Only `AppData`/etc. paths allowed. Try read `/etc/passwd` → permission denied.

Tighter security than Electron (no nodejs full access).

## 9.4 Custom permission scope

```json
{
  "permissions": [
    {
      "identifier": "fs:scope",
      "allow": [
        { "path": "$HOME/Documents/MyApp/**" }
      ]
    }
  ]
}
```

Whitelist specific paths. Defense in depth.

---

# Tầng 10: Security — Capabilities & permissions

## 10.1 Tauri v2 security model

```
   ┌────────────────────────────────────────────────────────┐
   │ Frontend WebView (untrusted!)                          │
   │                                                        │
   │ Tries to invoke('command') or use plugin API            │
   │                                                        │
   └──────────────┬─────────────────────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────────────────────┐
   │ Capability layer (security check)                      │
   │                                                        │
   │ Check capability files:                                 │
   │  - Is this window/webview allowed?                      │
   │  - Is this permission granted?                          │
   │  - Does scope match (e.g., URL)?                        │
   │                                                        │
   │ If allowed → forward to plugin/command                 │
   │ If denied → reject with permission error               │
   │                                                        │
   └──────────────┬─────────────────────────────────────────┘
                  │
                  ▼
   ┌────────────────────────────────────────────────────────┐
   │ Plugin / command (Rust)                                │
   │ Trust this can be executed safely                      │
   └────────────────────────────────────────────────────────┘
```

Defense in depth. WebView is **untrusted** (could be compromised by XSS).

## 10.2 Capability files

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:window:default",
    "core:window:allow-set-title",
    "core:window:allow-maximize",
    "dialog:default",
    "fs:read-files-app",
    "fs:write-files-app",
    "shell:allow-open"
  ]
}
```

Lists what main window can do.

## 10.3 Content Security Policy (CSP)

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'"
    }
  }
}
```

Restrict what webview can load (XSS protection):
- `default-src 'self'`: only same-origin
- `img-src 'self' data:`: images from self or data URLs
- `style-src 'self' 'unsafe-inline'`: CSS from self or inline

**Strict CSP** = secure but more development friction.

## 10.4 Permissions for plugins

Each plugin defines permissions:

```json
{
  "permissions": [
    "fs:default",
    "fs:allow-read",
    "fs:allow-write",
    "fs:scope-document",      // Document directory
    "dialog:allow-open",
    "dialog:allow-save",
    "http:default",
    "http:allow-fetch",
    "shell:allow-open"
  ]
}
```

Grant only what app needs. Tauri rejects unallowed calls.

## 10.5 Custom commands need no extra perm (mostly)

```rust
#[tauri::command]
fn my_command() { ... }
```

By default, **all custom commands** callable from frontend. To restrict:

```json
{
  "permissions": [
    {
      "identifier": "allow-my-command",
      "description": "Allow my_command",
      "commands": {
        "allow": ["my_command"]
      }
    }
  ]
}
```

Use for admin-only commands in multi-window apps.

## 10.6 Sanitize input

Frontend → Rust IPC: Tauri serializes via serde. Type safe → no injection.

But: validate semantically!
```rust
#[tauri::command]
fn execute_command(cmd: String) -> Result<String, String> {
    // ❌ DANGEROUS — shell injection!
    let output = std::process::Command::new("sh")
        .arg("-c").arg(&cmd).output();
    // ...
}
```

Don't trust input even from your own frontend. Whitelist, validate.

## 10.7 Security checklist

```
☑ Strict CSP
☑ Minimal capabilities granted
☑ FS scope limited
☑ Input validation in commands
☑ Don't execute arbitrary shell from frontend
☑ HTTPS for any external API
☑ No secrets in frontend code
☑ Update Tauri version regularly
☑ Code-sign distribution binaries
```

---

# Tầng 11: Plugins ecosystem

## 11.1 Official plugins

```toml
# In Cargo.toml:
tauri-plugin-shell = "2"           # Shell command execution
tauri-plugin-dialog = "2"          # Native dialogs
tauri-plugin-fs = "2"              # File system
tauri-plugin-http = "2"             # HTTP client
tauri-plugin-notification = "2"    # Native notifications
tauri-plugin-clipboard-manager = "2" # Clipboard
tauri-plugin-global-shortcut = "2" # Global hotkeys
tauri-plugin-os = "2"              # OS info
tauri-plugin-window-state = "2"    # Save window state
tauri-plugin-store = "2"           # Persistent KV store
tauri-plugin-sql = "2"             # SQLite/Postgres/MySQL
tauri-plugin-log = "2"             # Logging
tauri-plugin-updater = "2"         # Auto-update
tauri-plugin-deep-link = "2"       # Custom URL scheme
tauri-plugin-process = "2"         # Restart, exit
```

## 11.2 Register plugin

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        ...
        .run(...);
}
```

In frontend:
```bash
npm install @tauri-apps/plugin-fs @tauri-apps/plugin-dialog @tauri-apps/plugin-http
```

## 11.3 HTTP plugin example

```typescript
import { fetch } from '@tauri-apps/plugin-http';

const response = await fetch('https://api.example.com/users/42', {
    method: 'GET',
    headers: { 'Authorization': 'Bearer xxx' },
});

const data = await response.json();
```

Why use this vs browser `fetch`?
- Bypass CORS (Rust does request, no origin restriction)
- Allowlist URL patterns in capabilities
- Add cookies, custom certs, etc.

## 11.4 Store plugin (KV)

```typescript
import { Store } from '@tauri-apps/plugin-store';

const store = new Store('settings.dat');

await store.set('theme', 'dark');
await store.save();

const theme = await store.get<string>('theme');
console.log(theme);   // "dark"
```

Persistent KV store. Auto-saves to disk. Simpler than fs read/write JSON.

## 11.5 Global shortcut

```typescript
import { register, unregister } from '@tauri-apps/plugin-global-shortcut';

await register('CommandOrControl+Shift+Y', () => {
    console.log('Shortcut triggered!');
});

// Even when app not focused
```

Useful for: clipboard manager, quick capture, screenshot apps.

## 11.6 Tauri-plugin-sql

```toml
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
```

```typescript
import Database from '@tauri-apps/plugin-sql';

const db = await Database.load('sqlite:test.db');

await db.execute('CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)');
await db.execute('INSERT INTO users VALUES ($1, $2)', [1, 'Alice']);

const users = await db.select<User[]>('SELECT * FROM users');
```

SQLite embedded — no server. Or connect to Postgres/MySQL remote.

## 11.7 Custom plugin

```rust
// In your_plugin/src/lib.rs
use tauri::{plugin::{Builder, TauriPlugin}, Runtime};

#[tauri::command]
fn my_plugin_command() -> String { "hello".into() }

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("myplugin")
        .invoke_handler(tauri::generate_handler![my_plugin_command])
        .build()
}
```

Use:
```rust
tauri::Builder::default()
    .plugin(your_plugin::init())
```

Bundle reusable functionality. Useful for company-internal libraries.

---

# Tầng 12: Frontend integration patterns

## 12.1 React + Tauri pattern

```tsx
import { invoke } from '@tauri-apps/api/core';
import { useState, useEffect } from 'react';

interface User {
    id: number;
    name: string;
    email: string;
}

function UserList() {
    const [users, setUsers] = useState<User[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    
    useEffect(() => {
        async function load() {
            setLoading(true);
            try {
                const data = await invoke<User[]>('list_users');
                setUsers(data);
            } catch (e) {
                setError(String(e));
            } finally {
                setLoading(false);
            }
        }
        load();
    }, []);
    
    if (loading) return <div>Loading...</div>;
    if (error) return <div>Error: {error}</div>;
    
    return (
        <ul>
            {users.map(u => (
                <li key={u.id}>{u.name} - {u.email}</li>
            ))}
        </ul>
    );
}
```

Standard React patterns work. Tauri commands look like async functions.

## 12.2 TanStack Query for caching

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

function useUsers() {
    return useQuery({
        queryKey: ['users'],
        queryFn: () => invoke<User[]>('list_users'),
    });
}

function useCreateUser() {
    const qc = useQueryClient();
    return useMutation({
        mutationFn: (req: CreateUserRequest) => 
            invoke<User>('create_user', { req }),
        onSuccess: () => {
            qc.invalidateQueries({ queryKey: ['users'] });
        },
    });
}
```

Production frontend pattern: TanStack Query handles caching, refetching, optimistic updates.

## 12.3 Event subscription hook

```typescript
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';

function useTauriEvent<T>(event: string, handler: (payload: T) => void) {
    useEffect(() => {
        let unlisten: UnlistenFn;
        listen<T>(event, (e) => handler(e.payload))
            .then(fn => { unlisten = fn; });
        
        return () => {
            unlisten?.();
        };
    }, [event, handler]);
}

// Usage:
function DownloadProgress() {
    const [progress, setProgress] = useState(0);
    useTauriEvent<number>('download-progress', setProgress);
    return <progress value={progress} max={100} />;
}
```

## 12.4 Routing

```tsx
import { BrowserRouter, Routes, Route } from 'react-router-dom';

function App() {
    return (
        <BrowserRouter>
            <Routes>
                <Route path="/" element={<Home />} />
                <Route path="/users" element={<UserList />} />
                <Route path="/settings" element={<Settings />} />
            </Routes>
        </BrowserRouter>
    );
}
```

Use hash routing or memory router if file://protocol issues:
```tsx
import { HashRouter } from 'react-router-dom';
<HashRouter>...</HashRouter>
```

## 12.5 State management on frontend

Same as web app:
- React Context for simple state
- Zustand / Jotai for lightweight global
- Redux Toolkit for complex
- TanStack Query for server state

Don't duplicate state in Rust + JS. Pick one source of truth:
- Persistent / system / business logic → Rust
- UI / form state → JS

## 12.6 Type safety with bindings

With `tauri-specta` (Tầng 4.8):
```typescript
import { commands, events } from './bindings';

// Typed commands:
const user = await commands.getUser(42);   // returns User typed

// Typed events:
events.downloadProgress.listen((event) => {
    // event.payload typed
});
```

No more manual `invoke<User>` type assertions. Auto-generated from Rust.

---

# Tầng 13: Packaging & Distribution

## 13.1 Build for current platform

```bash
npm run tauri build
```

Output:
- macOS: `src-tauri/target/release/bundle/dmg/MyApp_0.1.0_x64.dmg`
- Windows: `src-tauri/target/release/bundle/msi/MyApp_0.1.0_x64_en-US.msi`
- Linux: `src-tauri/target/release/bundle/deb/myapp_0.1.0_amd64.deb`

## 13.2 Cross-compile

Cross-compiling Tauri is tricky (system webview + native bindings). Common pattern:
- macOS: build on macOS
- Windows: build on Windows
- Linux: build on Linux
- Or use GitHub Actions matrix (build each platform separately)

## 13.3 GitHub Actions

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        platform: [macos-latest, ubuntu-22.04, windows-latest]
    
    runs-on: ${{ matrix.platform }}
    
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - uses: dtolnay/rust-toolchain@stable
      
      - name: install dependencies (ubuntu only)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev \
            libappindicator3-dev librsvg2-dev patchelf
      
      - name: install frontend dependencies
        run: npm ci
      
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'App ${{ github.ref_name }}'
          releaseBody: 'See assets below'
          releaseDraft: true
          prerelease: false
```

Auto-build for all 3 platforms, attach to release.

## 13.4 Code signing

**Production app SHOULD code-sign**. Without sign:
- macOS: "unidentified developer" warning, user manually allows
- Windows: SmartScreen warning
- Linux: less of an issue

### macOS signing

```bash
# Apple Developer ID Application certificate
export APPLE_CERTIFICATE="..."   # base64 .p12
export APPLE_CERTIFICATE_PASSWORD="..."
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name"
export APPLE_ID="your-apple-id@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="..."

npm run tauri build
# Signs + notarizes for Gatekeeper
```

### Windows signing

```toml
# tauri.conf.json
"bundle": {
  "windows": {
    "certificateThumbprint": "...",
    "digestAlgorithm": "sha256",
    "timestampUrl": "http://timestamp.digicert.com"
  }
}
```

Need: code signing certificate ($200+/year).

## 13.5 Reduce binary size

```toml
[profile.release]
panic = "abort"      # smaller, slightly faster
codegen-units = 1
lto = true
strip = true         # remove debug symbols
opt-level = "s"      # optimize for size
```

Typical reductions:
- `strip = true`: -30-50% binary size
- `lto = true`: -10-20% + perf
- `opt-level = "s"`: -10-30%

Don't use `panic = "abort"` if you need `catch_unwind` (rare in Tauri).

## 13.6 App store distribution

### Mac App Store
- Need: Apple Developer Program ($99/year)
- Use entitlements, sandboxing
- Submit via Xcode / Transporter

### Microsoft Store
- Need: Microsoft Partner Center account
- MSIX package

Often skip stores → direct download from website. App stores have stricter sandboxing requirements.

---

# Tầng 14: Updater

## 14.1 Updater plugin

```toml
tauri-plugin-updater = "2"
```

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        ...
}
```

## 14.2 Server side — update manifest

Host on your CDN:
```json
{
  "version": "0.2.0",
  "notes": "Bug fixes",
  "pub_date": "2024-05-26T12:00:00Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "...",
      "url": "https://cdn/app/MyApp-x86_64.app.tar.gz"
    },
    "darwin-aarch64": {
      "signature": "...",
      "url": "https://cdn/app/MyApp-aarch64.app.tar.gz"
    },
    "linux-x86_64": {
      "signature": "...",
      "url": "https://cdn/app/myapp_0.2.0_amd64.AppImage.tar.gz"
    },
    "windows-x86_64": {
      "signature": "...",
      "url": "https://cdn/app/MyApp-x86_64.msi.zip"
    }
  }
}
```

## 14.3 tauri.conf.json updater config

```json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://cdn.example.com/updates/{{target}}-{{arch}}/{{current_version}}"],
      "dialog": true,
      "pubkey": "..."
    }
  }
}
```

Public key for signature verification.

## 14.4 Frontend check + apply

```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

async function checkForUpdates() {
    try {
        const update = await check();
        if (update?.available) {
            console.log(`Update ${update.version} available`);
            await update.downloadAndInstall();
            await relaunch();
        }
    } catch (e) {
        console.error('Update check failed', e);
    }
}

// Call on app start or periodic check
checkForUpdates();
```

Downloads signed update, applies, relaunches.

## 14.5 Sign updates

```bash
# Generate signing key (once):
npm run tauri signer generate -- -w ~/.tauri/myapp.key

# Sign new release:
npm run tauri build
# Auto-signed if TAURI_SIGNING_PRIVATE_KEY env var set
```

Embed `pubkey` in `tauri.conf.json` → app verifies signature before installing.

Critical for security. Without sign → attacker can serve malicious update.

## 14.6 Rollback strategy

Tauri updater downloads + replaces binary. No automatic rollback.

If new version crashes:
- User manually downloads old version
- Or app has multiple versions installed (advanced)

Better: thorough test before release. Beta channel for power users.

---

# Tầng 15: Mobile — iOS & Android (v2)

## 15.1 Tauri 2 mobile support

Tauri 2 (2024+) supports iOS and Android **out of box**. Same Rust + WebView model, mobile native APIs.

```bash
# Setup mobile:
npm run tauri ios init    # iOS
npm run tauri android init  # Android

# Dev:
npm run tauri ios dev
npm run tauri android dev

# Build:
npm run tauri ios build
npm run tauri android build
```

Output: `.ipa` (iOS), `.apk`/`.aab` (Android).

## 15.2 Mobile-specific considerations

```
   ┌──────────────────────────────────────────────────────────┐
   │ MOBILE differences:                                      │
   │                                                          │
   │ • Touch input vs mouse                                   │
   │ • Smaller screens                                         │
   │ • Battery / power constraints                             │
   │ • Background restrictions (suspend)                       │
   │ • No file system access (sandboxed)                       │
   │ • Push notifications (separate APIs)                      │
   │ • App store policies                                      │
   │                                                          │
   │ • iOS: WKWebView (similar to macOS)                       │
   │ • Android: System WebView (Chromium-based)                │
   └──────────────────────────────────────────────────────────┘
```

## 15.3 Native plugins for mobile

Camera, GPS, biometrics — need platform code:
```rust
// Tauri 2 supports Kotlin/Swift code in plugin
// Plugins like:
// - tauri-plugin-camera
// - tauri-plugin-geolocation
// - tauri-plugin-biometric
// - tauri-plugin-haptics
```

Use existing plugins or write custom Swift/Kotlin bridges.

## 15.4 Conditional code

```rust
#[cfg(target_os = "ios")]
fn ios_specific() { ... }

#[cfg(target_os = "android")]
fn android_specific() { ... }

#[cfg(desktop)]
fn desktop_only() { ... }
```

Different feature sets per platform.

## 15.5 Responsive frontend

Mobile UI needs:
- Larger touch targets
- Swipe gestures
- Fixed bottom navigation
- Status bar / safe area awareness

Use CSS media queries, modern responsive design.

## 15.6 Status

Mobile is **newer** in Tauri:
- v2 stable for desktop
- Mobile stable for major flows
- Some plugins desktop-only

For mobile-heavy app, evaluate vs Flutter / React Native. Tauri is competitive when:
- Reuse desktop codebase
- Need native Rust performance
- Want web tech stack

---

# Tầng 16: Performance & best practices

## 16.1 Startup performance

```
   Tauri app start:
   ───────────────
   1. OS launches binary
   2. Rust initializes
   3. Window creates
   4. WebView starts
   5. HTML/JS loads
   6. App ready
   
   Target: < 500ms cold start
```

Optimize:
- Minimize Rust setup
- Lazy-load JS bundles (code splitting)
- Show splash screen first

## 16.2 Bundle size

```bash
# Frontend bundle analysis:
npm run build -- --analyze    # depends on bundler

# Rust binary:
cargo bloat --release
```

Reduce frontend:
- Tree-shake unused libs
- Code splitting per route
- Use compression (gzip/brotli)
- Lazy-load heavy components

Reduce Rust:
- `opt-level = "s"`, `lto = true`, `strip = true`
- Remove unused dependencies
- Use feature flags

## 16.3 IPC performance

IPC has overhead — every command call serializes args.

```typescript
// ❌ Many small commands
for (let i = 0; i < 1000; i++) {
    await invoke('process', { i });   // 1000 IPC calls!
}

// ✅ Batch
await invoke('process_batch', { items: Array.from({length: 1000}, (_, i) => i) });
```

Magnitude faster.

## 16.4 Memory profiling

```rust
// Use heaptrack or dhat in dev:
#[cfg(feature = "dhat-heap")]
let _profiler = dhat::Profiler::new_heap();
```

WebView memory: profile in browser dev tools (Tauri has Web Inspector in dev mode).

```bash
# Tauri dev: right-click → Inspect Element
# Use Chrome/Safari devtools
```

## 16.5 Logging

```toml
tauri-plugin-log = "2"
```

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new()
            .target(LogTarget::LogDir)        // app log directory
            .target(LogTarget::Stdout)         // stdout (dev)
            .level(log::LevelFilter::Info)
            .build())
        ...
}
```

```typescript
import { info, warn, error } from '@tauri-apps/plugin-log';

info('user clicked button');
error('something broke', { err: 'details' });
```

Logs to file by default. Helpful for debug production.

## 16.6 Crash reporting

Production app SHOULD report crashes:

```toml
# Sentry integration
sentry = "0.34"
sentry-tauri = "..."
```

Or use Crashpad / Breakpad — capture native crashes (segfault).

Without crash reports → silent failures, hard to fix.

## 16.7 Async best practices

```rust
// ❌ Don't block in command — UI freezes!
#[tauri::command]
fn slow_op() -> String {
    std::thread::sleep(Duration::from_secs(5));
    "done".into()
}

// ✅ Async (uses tokio)
#[tauri::command]
async fn slow_op() -> String {
    tokio::time::sleep(Duration::from_secs(5)).await;
    "done".into()
}

// ✅ CPU-bound — spawn_blocking
#[tauri::command]
async fn cpu_heavy() -> String {
    tokio::task::spawn_blocking(|| {
        compute_intensive()
    }).await.unwrap()
}
```

Apply [async.md](./async.md) lessons.

## 16.8 Best practices summary

```
┌────────────────────────────────────────────────────────────┐
│ ✅ Type bindings with tauri-specta                         │
│ ✅ Minimize IPC calls (batch when possible)                │
│ ✅ Async commands for I/O                                  │
│ ✅ spawn_blocking for CPU work                             │
│ ✅ State management via .manage(Arc<T>)                    │
│ ✅ Capabilities + CSP for security                         │
│ ✅ Code-sign releases                                      │
│ ✅ Sign updates                                            │
│ ✅ Log to file + crash reporting                           │
│ ✅ Test on all target platforms                            │
│ ✅ Profile bundle size + startup time                      │
│ ✅ Use plugins (don't reinvent)                            │
└────────────────────────────────────────────────────────────┘
```

---

# Tổng kết — 12 nguyên tắc senior Tauri

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Rust backend = trusted, WebView = untrusted (XSS possible).   │
│                                                                  │
│ 2. Commands for request/response, Events for pub/sub.            │
│                                                                  │
│ 3. State via .manage(), shared across commands.                  │
│                                                                  │
│ 4. tauri-specta for type-safe TS bindings (no manual types).     │
│                                                                  │
│ 5. Capabilities + CSP — least privilege.                         │
│                                                                  │
│ 6. fs scoped to AppData et al, never full disk.                  │
│                                                                  │
│ 7. Code-sign on macOS + Windows for production.                  │
│                                                                  │
│ 8. Updater signed — verify pubkey on client.                     │
│                                                                  │
│ 9. Async commands. spawn_blocking for CPU.                       │
│                                                                  │
│ 10. Batch IPC — many small calls = overhead.                     │
│                                                                  │
│ 11. Plugins ecosystem — reuse, don't reinvent.                   │
│                                                                  │
│ 12. Test on all target platforms (Win/Mac/Linux/Mobile).          │
└──────────────────────────────────────────────────────────────────┘
```

---

# Apply previous chapters to Tauri

| Chapter | Tauri usage |
|---------|-------------|
| trait | IpcResponse, IntoResponse |
| generic | State<T>, commands generic |
| async | All commands, tokio runtime |
| error-handling | CommandError + Serialize for IPC |
| smart-pointers | Arc<AppState> shared |
| observability | tauri-plugin-log, tracing |
| testing | Unit test Rust, e2e with webdriver |

---

# Tauri toolkit

| Crate / Tool | Purpose |
|--------------|---------|
| `tauri` | Core framework |
| `tauri-build` | Build script |
| `tauri-plugin-*` | Official plugins |
| `tauri-specta` | TS binding generator |
| `specta` | Type metadata |
| `@tauri-apps/api` | Frontend SDK |
| `@tauri-apps/plugin-*` | Frontend plugin SDKs |

---

# 🦀 Bộ tài liệu giờ có 19 chương!

```
docs/
├── 1-16. Foundations (memory, ownership, trait, ..., embedded)
├── 17. axum-project        — Web app realistic
├── 18. database            — Database deep dive
└── 19. tauri               ← MỚI — Desktop & Mobile apps
```

Bộ kỹ năng Rust full-stack:
- 🌐 Web services (axum)
- 🗄️ Database systems (sqlx)
- 🔌 Embedded (no_std, embassy)
- 🖥️ Desktop apps (Tauri)
- 📱 Mobile apps (Tauri v2)
- 🚀 High-performance (profiling, unsafe)
- 🧪 Well-tested codebases

🎓 Chúc bạn senior Rust journey thành công!
