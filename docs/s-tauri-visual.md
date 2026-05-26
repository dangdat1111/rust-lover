# Tauri — Minh Hoạ Trực Quan

> Companion visual cho [tauri.md](./tauri.md). Đọc song song.

---

## 1. Bức tranh lớn — Tauri Universe

```
                          TAURI APP
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   Rust backend  ◄──── IPC ────►  WebView frontend      │
       │   (trusted)                       (untrusted)          │
       │                                                        │
       │   ┌─────────────┐                ┌─────────────┐       │
       │   │ Commands     │ ←── invoke ──│ React /     │       │
       │   │ Events       │ ── emit ────►│ Vue /       │       │
       │   │ State        │                │ Svelte ...   │       │
       │   │ Plugins      │                │             │       │
       │   └─────────────┘                └─────────────┘       │
       │                                                        │
       │   Capabilities (security boundary)                     │
       │   CSP (XSS protection)                                 │
       │                                                        │
       │   Targets: Windows, macOS, Linux, iOS, Android         │
       │   Bundle: 5-15MB (vs Electron 100-200MB)                │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Tauri vs Electron architecture

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │   ELECTRON APP:                                              │
   │                                                              │
   │   ┌──────────────────────┐  ┌──────────────────────┐         │
   │   │ Main Process         │  │ Renderer Process     │         │
   │   │ (Node.js ~30MB)      │  │ (Chromium ~100MB)    │         │
   │   │                      │  │                      │         │
   │   │ - File API           │  │ - HTML/CSS/JS        │         │
   │   │ - Network             │  │ - DOM                 │         │
   │   │ - System APIs        │  │ - V8 engine          │         │
   │   └──────────────────────┘  └──────────────────────┘         │
   │           Bundled together → 100-200MB binary                │
   │                                                              │
   ├──────────────────────────────────────────────────────────────┤
   │                                                              │
   │   TAURI APP:                                                 │
   │                                                              │
   │   ┌──────────────────────┐  ┌──────────────────────┐         │
   │   │ Rust binary (~10MB)  │  │ System WebView (0MB) │         │
   │   │                      │  │                      │         │
   │   │ - tauri core         │  │ - WKWebView (macOS)  │         │
   │   │ - tokio runtime      │  │ - WebView2 (Windows) │         │
   │   │ - commands           │  │ - WebKitGTK (Linux)  │         │
   │   │ - state              │  │                       │         │
   │   └──────────────────────┘  └──────────────────────┘         │
   │           Total: 5-15MB binary, no bundled browser           │
   │                                                              │
   │   ✅ 10-50x smaller                                          │
   │   ✅ 5-10x less RAM                                          │
   │   ✅ Native-like startup                                     │
   │   ✅ Memory-safe (Rust)                                      │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
```

---

## 3. Process model

```
   ┌──────────────────────────────────────────────────────────┐
   │                  Tauri App                               │
   │                                                          │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ MAIN THREAD (Rust)                              │    │
   │   │                                                 │    │
   │   │  ┌──────────────────────────────────────┐       │    │
   │   │  │ tauri::Builder                       │       │    │
   │   │  │  - generate_handler! commands        │       │    │
   │   │  │  - .manage(state)                    │       │    │
   │   │  │  - .plugin(...)                      │       │    │
   │   │  │  - .run(...)                         │       │    │
   │   │  └──────────────────────────────────────┘       │    │
   │   │                                                 │    │
   │   │  ┌──────────────────────────────────────┐       │    │
   │   │  │ tokio runtime                        │       │    │
   │   │  │  - command handlers (async)          │       │    │
   │   │  │  - event listeners                   │       │    │
   │   │  │  - spawn() background tasks          │       │    │
   │   │  └──────────────────────────────────────┘       │    │
   │   │                                                 │    │
   │   └────────────────────────────────────────────────┘    │
   │                         │                                │
   │                  IPC (async messages)                    │
   │                         │                                │
   │   ┌────────────────────────────────────────────────┐     │
   │   │ WEBVIEW THREAD/PROCESS                          │    │
   │   │                                                 │    │
   │   │  ┌──────────────────────────────────────┐       │    │
   │   │  │ System WebView                       │       │    │
   │   │  │  - JS engine (V8/JavaScriptCore)     │       │    │
   │   │  │  - DOM rendering                     │       │    │
   │   │  │  - React/Vue/... app                 │       │    │
   │   │  │  - @tauri-apps/api                   │       │    │
   │   │  └──────────────────────────────────────┘       │    │
   │   │                                                 │    │
   │   └────────────────────────────────────────────────┘    │
   │                                                          │
   │   ⚠️ Rust can't touch DOM directly                       │
   │   ⚠️ JS can't touch FS directly                          │
   │   ⟹ Separation = security boundary                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. Project structure

```
   my-app/
   ├── src/                       ← FRONTEND
   │   ├── App.tsx                  React/Vue/...
   │   ├── components/
   │   ├── routes/
   │   └── main.tsx
   │
   ├── public/                    ← static assets
   │
   ├── src-tauri/                 ← RUST BACKEND
   │   ├── Cargo.toml
   │   ├── tauri.conf.json          ← Tauri config
   │   │   {
   │   │     "productName": "MyApp",
   │   │     "identifier": "com.example.myapp",
   │   │     "app": {
   │   │       "windows": [...],
   │   │       "security": {"csp": "..."}
   │   │     },
   │   │     "bundle": {...}
   │   │   }
   │   │
   │   ├── src/
   │   │   ├── main.rs            ← tauri::Builder
   │   │   ├── commands.rs
   │   │   ├── state.rs
   │   │   └── error.rs
   │   │
   │   ├── icons/                 ← app icons (multi-platform)
   │   │
   │   ├── capabilities/          ← SECURITY
   │   │   └── default.json        permissions per window
   │   │
   │   ├── build.rs               ← (optional) tauri-specta gen TS
   │   │
   │   └── gen/                   ← auto-generated
   │
   ├── dist/                      ← frontend build output
   │
   ├── package.json
   └── tsconfig.json
```

---

## 5. IPC — Commands (request/response)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  FRONTEND (TypeScript):                                  │
   │  ──────────────────                                      │
   │                                                          │
   │  import { invoke } from '@tauri-apps/api/core';          │
   │                                                          │
   │  const user = await invoke<User>('get_user', { id: 42 });│
   │  // Promise — async                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  IPC layer:                                              │
   │  ───────                                                 │
   │                                                          │
   │  1. Serialize args (JSON-like via serde)                 │
   │  2. Send message to Rust over IPC channel                │
   │  3. Rust dispatches to command handler                   │
   │  4. Handler executes (async on tokio)                    │
   │  5. Serialize return value                               │
   │  6. Send back to frontend                                │
   │  7. Resolve Promise                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  BACKEND (Rust):                                         │
   │  ─────────                                               │
   │                                                          │
   │  #[tauri::command]                                       │
   │  async fn get_user(                                       │
   │      id: u64,                                            │
   │      state: tauri::State<'_, DbState>                    │
   │  ) -> Result<User, String> {                             │
   │      sqlx::query_as!(User,                                │
   │          "SELECT * FROM users WHERE id = $1", id as i64)  │
   │          .fetch_one(&state.db).await                     │
   │          .map_err(|e| e.to_string())                     │
   │  }                                                       │
   │                                                          │
   │  // Register:                                            │
   │  .invoke_handler(tauri::generate_handler![get_user])     │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 6. IPC — Events (pub/sub)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  BACKEND → FRONTEND:                                     │
   │                                                          │
   │  Rust:                                                   │
   │   window.emit("download-progress", 42)?;                 │
   │   window.emit("download-complete", "/path/file")?;        │
   │            │                                             │
   │            │ broadcast to listeners                      │
   │            ▼                                             │
   │                                                          │
   │  Frontend:                                               │
   │   import { listen } from '@tauri-apps/api/event';        │
   │                                                          │
   │   const unlisten = await listen<number>(                 │
   │     'download-progress',                                 │
   │     (event) => {                                         │
   │       updateProgressBar(event.payload);                  │
   │     }                                                    │
   │   );                                                     │
   │                                                          │
   │   // later: unlisten();                                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  FRONTEND → BACKEND:                                     │
   │                                                          │
   │  Frontend:                                               │
   │   import { emit } from '@tauri-apps/api/event';          │
   │   await emit('user-clicked', { button: 'save' });        │
   │            │                                             │
   │            ▼                                             │
   │                                                          │
   │  Rust:                                                   │
   │   app.listen("user-clicked", |event| {                   │
   │     // handle event                                      │
   │   });                                                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  When commands vs events?                                │
   │  ────────────────────                                    │
   │                                                          │
   │  COMMANDS:                EVENTS:                        │
   │  • Need return value      • Notifications                │
   │  • Single invocation      • One-to-many                  │
   │  • Sync flow              • Fire-and-forget              │
   │  • Errors propagate       • No response needed           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. State management

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Register state at startup:                             │
   │                                                          │
   │   struct AppState {                                      │
   │       db: PgPool,                                        │
   │       config: Arc<Config>,                               │
   │       counter: Mutex<u32>,                               │
   │   }                                                      │
   │                                                          │
   │   tauri::Builder::default()                              │
   │       .manage(AppState { ... })  ← state registered      │
   │       .invoke_handler(...)                               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Access in command:                                     │
   │                                                          │
   │   #[tauri::command]                                      │
   │   async fn get_user(                                     │
   │       state: tauri::State<'_, AppState>,                 │
   │       id: u64,                                           │
   │   ) -> Result<User, String> {                            │
   │       sqlx::query_as!(...)                                │
   │           .fetch_one(&state.db).await                    │
   │           .map_err(|e| e.to_string())                    │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Multiple state types — clean separation:               │
   │                                                          │
   │   struct DbState { pool: PgPool }                        │
   │   struct AuthState { current: Mutex<Option<User>> }      │
   │   struct ConfigState { cfg: Mutex<Config> }              │
   │                                                          │
   │   .manage(DbState { pool })                              │
   │   .manage(AuthState { ... })                             │
   │   .manage(ConfigState { ... })                           │
   │                                                          │
   │   #[tauri::command]                                      │
   │   async fn login(                                        │
   │       db: tauri::State<'_, DbState>,                     │
   │       auth: tauri::State<'_, AuthState>,                 │
   │       config: tauri::State<'_, ConfigState>,             │
   │       creds: Credentials,                                │
   │   ) { ... }                                              │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. Window management

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  STATIC (in tauri.conf.json):                            │
   │                                                          │
   │  "app": {                                                │
   │    "windows": [{                                         │
   │      "label": "main",                                    │
   │      "title": "MyApp",                                   │
   │      "width": 800,                                       │
   │      "height": 600,                                      │
   │      "resizable": true,                                  │
   │      "decorations": true                                  │
   │    }]                                                    │
   │  }                                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  DYNAMIC (create in command):                            │
   │                                                          │
   │  #[tauri::command]                                       │
   │  async fn open_settings(app: tauri::AppHandle) {         │
   │      WebviewWindowBuilder::new(                          │
   │          &app, "settings",                               │
   │          WebviewUrl::App("settings.html".into()))        │
   │          .title("Settings")                              │
   │          .inner_size(400.0, 300.0)                       │
   │          .build()                                        │
   │          .unwrap();                                      │
   │  }                                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Multi-window communication:                             │
   │                                                          │
   │   ┌──────────────┐         ┌──────────────┐              │
   │   │ Main window  │ ◄─────► │ Settings     │              │
   │   │ (label=main) │  events │ (label=...)  │              │
   │   └──────────────┘         └──────────────┘              │
   │           │                       ▲                      │
   │           └───────────────────────┘                      │
   │                  app.emit() to all                       │
   │                  window.emit() to specific               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. Capability-based security

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   WebView (untrusted!) calls plugin/command              │
   │       │                                                  │
   │       ▼                                                  │
   │   ┌─────────────────────────────────────────────┐        │
   │   │ Capability check:                           │        │
   │   │                                             │        │
   │   │ • Is this window in capability?             │        │
   │   │ • Is this permission granted?               │        │
   │   │ • Does scope match (e.g., file path)?       │        │
   │   └────┬───────────────────────────┬────────────┘        │
   │        │                           │                     │
   │       ALLOW                       DENY                   │
   │        │                           │                     │
   │        ▼                           ▼                     │
   │   ┌──────────┐              ┌─────────────────┐          │
   │   │ Forward  │              │ Reject with     │          │
   │   │ to handler│             │ permission error│          │
   │   └──────────┘              └─────────────────┘          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Capability file (capabilities/default.json):
   ────────────────────────────────────────────
   
   {
     "identifier": "default",
     "windows": ["main"],          ← scope to window
     "permissions": [
       "core:default",
       "core:window:default",
       "core:window:allow-set-title",
       "dialog:default",
       "fs:read-files-app",         ← fs limited to AppData
       "fs:write-files-app",
       "fs:scope-app",              ← only app dir
       "shell:allow-open"
     ]
   }
   
   
   Plugin permission examples:
   ───────────────────────────
   
   fs:default              → basic read in known dirs
   fs:allow-read           → unrestricted read (DANGEROUS)
   fs:scope-document       → only ~/Documents
   fs:scope-app            → only AppData
   
   http:default            → no allowed by default
   http:allow-fetch        → allow fetch from frontend
   
   dialog:default          → open/save dialogs OK
   shell:allow-open        → open URLs / files with default app
   shell:allow-execute     → run shell commands (DANGEROUS)
```

---

## 10. Content Security Policy (CSP)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   CSP = browser-level XSS protection                     │
   │                                                          │
   │   tauri.conf.json:                                       │
   │   "security": {                                          │
   │     "csp": "default-src 'self';                          │
   │            img-src 'self' data: https:;                  │
   │            style-src 'self' 'unsafe-inline';             │
   │            connect-src 'self' https://api.example.com"   │
   │   }                                                      │
   │                                                          │
   │                                                          │
   │   Even if attacker injects <script>...</script>          │
   │   via XSS into webview, CSP blocks execution.            │
   │                                                          │
   │                                                          │
   │   Strict CSP:                                            │
   │   • default-src 'self'                                   │
   │     → only same-origin resources                         │
   │   • script-src 'self' (NO unsafe-inline)                 │
   │     → no inline scripts (eval, onclick)                  │
   │   • style-src 'self'                                     │
   │     → no inline styles                                   │
   │                                                          │
   │   Trade-off: stricter = more secure but more friction    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 11. Plugins ecosystem

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Tauri Plugin Architecture                              │
   │                                                          │
   │   ┌──────────────────────────────────────────────────┐   │
   │   │ tauri-plugin-* (Rust crate)                      │   │
   │   │   - register commands                             │   │
   │   │   - manage state                                  │   │
   │   │   - native code (Swift/Kotlin for mobile)         │   │
   │   └──────────────────────────────────────────────────┘   │
   │                          │                               │
   │                          ▼                               │
   │   ┌──────────────────────────────────────────────────┐   │
   │   │ @tauri-apps/plugin-* (npm package)                │  │
   │   │   - TypeScript bindings                           │   │
   │   │   - Promises wrapping invoke()                    │   │
   │   └──────────────────────────────────────────────────┘   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Official plugins:
   ─────────────────
   
   ┌────────────────────────┬─────────────────────────────────┐
   │ Plugin                 │ Purpose                         │
   ├────────────────────────┼─────────────────────────────────┤
   │ shell                  │ Execute shell commands          │
   │ dialog                 │ Native dialogs (open/save/msg)  │
   │ fs                     │ File system access (scoped)     │
   │ http                   │ HTTP client (bypass CORS)       │
   │ notification           │ Native notifications            │
   │ clipboard-manager      │ Clipboard read/write             │
   │ global-shortcut        │ Global hotkeys                   │
   │ os                     │ OS info, version                 │
   │ window-state           │ Persist window size/pos          │
   │ store                  │ Persistent KV store              │
   │ sql                    │ SQLite/Postgres/MySQL            │
   │ log                    │ File logging                     │
   │ updater                │ Auto-update                      │
   │ deep-link              │ Custom URL scheme                │
   │ process                │ Restart, exit                    │
   │ biometric (mobile)     │ TouchID, FaceID, fingerprint    │
   │ camera (mobile)        │ Camera access                    │
   │ geolocation (mobile)   │ GPS                              │
   └────────────────────────┴─────────────────────────────────┘
```

---

## 12. Type-safe bindings (tauri-specta)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Without tauri-specta:                                  │
   │                                                          │
   │   // Manual TS types — easy to drift                     │
   │   interface User {                                       │
   │       id: number;                                        │
   │       email: string;                                     │
   │   }                                                      │
   │                                                          │
   │   const user = await invoke<User>('get_user', { id: 42 });│
   │   //                ^^^^ manually keep in sync           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   With tauri-specta:                                     │
   │                                                          │
   │   Rust:                                                  │
   │     #[derive(Type, Serialize, Deserialize)]              │
   │     struct User { id: u64, email: String }               │
   │                                                          │
   │     #[tauri::command]                                    │
   │     #[specta::specta]                                    │
   │     fn get_user(id: u64) -> Result<User, String> {...}   │
   │                                                          │
   │   build.rs:                                              │
   │     builder.commands(collect_commands![get_user])         │
   │            .export(typescript, "../src/bindings.ts");    │
   │                                                          │
   │   Auto-generated bindings.ts:                            │
   │     export interface User { id: number; email: string; } │
   │     export const commands = {                            │
   │       getUser: (id: number) => invoke<...>('get_user',...│
   │     }                                                    │
   │                                                          │
   │   Frontend:                                              │
   │     import { commands } from './bindings';               │
   │     const user = await commands.getUser(42);             │
   │     //                ^^^^ fully typed!                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   ⟹ Compile-time guarantee TS ↔ Rust types in sync.
   ⟹ Refactor Rust → TS bindings auto-update.
```

---

## 13. Frontend integration pattern (React)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   React component using Tauri:                           │
   │                                                          │
   │   function UserProfile({ id }: { id: number }) {         │
   │     const [user, setUser] = useState<User | null>(null); │
   │     const [error, setError] = useState<string | null>(null);│
   │                                                          │
   │     useEffect(() => {                                     │
   │       invoke<User>('get_user', { id })                   │
   │         .then(setUser)                                   │
   │         .catch(e => setError(String(e)));                │
   │     }, [id]);                                            │
   │                                                          │
   │     // Subscribe to events                               │
   │     useEffect(() => {                                     │
   │       const unlisten = listen<User>(                     │
   │         'user-updated',                                  │
   │         (e) => setUser(e.payload)                        │
   │       );                                                 │
   │       return () => { unlisten.then(fn => fn()); };       │
   │     }, []);                                              │
   │                                                          │
   │     if (error) return <div>Error: {error}</div>;         │
   │     if (!user) return <div>Loading...</div>;             │
   │     return <div>{user.name}</div>;                       │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   With TanStack Query (production):                      │
   │                                                          │
   │   function useUser(id: number) {                         │
   │     return useQuery({                                    │
   │       queryKey: ['users', id],                           │
   │       queryFn: () => invoke<User>('get_user', { id }),   │
   │     });                                                  │
   │   }                                                      │
   │                                                          │
   │   // Caching, refetch, optimistic updates, ... built-in │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. Packaging output

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   npm run tauri build                                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   macOS (run on Mac):                                    │
   │     src-tauri/target/release/bundle/                     │
   │       ├── macos/MyApp.app                                │
   │       ├── dmg/MyApp_0.1.0_x64.dmg                        │
   │       └── (universal binary if configured)               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Windows (run on Windows):                              │
   │     src-tauri/target/release/bundle/                     │
   │       ├── msi/MyApp_0.1.0_x64_en-US.msi                  │
   │       └── nsis/MyApp_0.1.0_x64-setup.exe                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Linux (run on Linux):                                  │
   │     src-tauri/target/release/bundle/                     │
   │       ├── deb/myapp_0.1.0_amd64.deb                      │
   │       ├── rpm/myapp-0.1.0-1.x86_64.rpm                   │
   │       └── appimage/myapp_0.1.0_amd64.AppImage            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   iOS / Android (Tauri v2):                              │
   │     iOS: .ipa for App Store                              │
   │     Android: .apk, .aab for Play Store                    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   CI matrix build (GitHub Actions):
   ─────────────────────────────────
   
   matrix:
     platform: [macos-latest, ubuntu-22.04, windows-latest]
   
   → Builds for all 3 platforms in parallel
   → Attached to GitHub Release
```

---

## 15. Auto-update flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   App start / periodic check:                            │
   │                                                          │
   │   ┌──────────────────────────────────────────┐           │
   │   │ App reads tauri.conf.json:               │           │
   │   │   "updater": {                           │           │
   │   │     "endpoints": ["https://cdn/..."],     │          │
   │   │     "pubkey": "..."                      │           │
   │   │   }                                      │           │
   │   └────────┬─────────────────────────────────┘           │
   │            │                                             │
   │            ▼  HTTP fetch update manifest                 │
   │   ┌──────────────────────────────────────────┐           │
   │   │ Server returns:                          │           │
   │   │   {                                      │           │
   │   │     "version": "0.2.0",                  │           │
   │   │     "platforms": {                       │           │
   │   │       "darwin-x86_64": {                 │           │
   │   │         "signature": "...",              │           │
   │   │         "url": "https://cdn/..."         │           │
   │   │       }                                  │           │
   │   │     }                                    │           │
   │   │   }                                      │           │
   │   └────────┬─────────────────────────────────┘           │
   │            │                                             │
   │            ▼  Compare versions                           │
   │   ┌──────────────────────────────────────────┐           │
   │   │ Version 0.2.0 > 0.1.0 → update available │           │
   │   └────────┬─────────────────────────────────┘           │
   │            │                                             │
   │            ▼  User confirms (optional)                   │
   │   ┌──────────────────────────────────────────┐           │
   │   │ Download update binary                   │           │
   │   │ Verify signature with pubkey ✅          │           │
   │   │   (REJECT if invalid signature)          │           │
   │   └────────┬─────────────────────────────────┘           │
   │            │                                             │
   │            ▼                                             │
   │   ┌──────────────────────────────────────────┐           │
   │   │ Replace binary                            │          │
   │   │ Relaunch app                              │          │
   │   └──────────────────────────────────────────┘           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   ⚠️ ALWAYS sign updates. Embed pubkey in app.
       Without sign → attacker can serve malicious update.
```

---

## 16. Mobile (Tauri v2)

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Same Rust + WebView model, mobile native APIs          │
   │                                                          │
   │   $ npm run tauri ios init                               │
   │   $ npm run tauri android init                           │
   │                                                          │
   │   ┌──────────────────────────────────────┐               │
   │   │ Tauri app                            │               │
   │   │   Rust code (shared)                  │              │
   │   │       │                              │               │
   │   │       │ compiles to:                 │               │
   │   │       │                              │               │
   │   │       ▼                              │               │
   │   │   ┌──────────────┐ ┌──────────────┐  │               │
   │   │   │ iOS binary   │ │ Android JNI  │  │               │
   │   │   │ (aarch64)    │ │ (aarch64,    │  │               │
   │   │   │              │ │  armv7, x86) │  │               │
   │   │   └──────────────┘ └──────────────┘  │               │
   │   │                                      │               │
   │   │   WebView:                            │              │
   │   │   ┌──────────────┐ ┌──────────────┐  │               │
   │   │   │ WKWebView    │ │ System       │  │               │
   │   │   │              │ │ WebView      │  │               │
   │   │   │              │ │ (Chrome-based)│ │               │
   │   │   └──────────────┘ └──────────────┘  │               │
   │   │                                      │               │
   │   │   Native APIs (camera, GPS,           │              │
   │   │   biometric, haptic):                 │              │
   │   │   ┌──────────────┐ ┌──────────────┐  │               │
   │   │   │ Swift bridge │ │ Kotlin bridge│  │               │
   │   │   │ (in plugins) │ │ (in plugins) │  │               │
   │   │   └──────────────┘ └──────────────┘  │               │
   │   └──────────────────────────────────────┘               │
   │                                                          │
   │   Build output:                                          │
   │     iOS:     .ipa (App Store)                            │
   │     Android: .apk, .aab (Play Store)                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Performance comparison

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │   Metric              Electron        Tauri                  │
   │   ─────────────────  ──────────      ──────                  │
   │                                                              │
   │   Binary size         100-200 MB     5-15 MB                 │
   │                       ████████        █                       │
   │                                                              │
   │   RAM at idle         200-500 MB     30-80 MB                │
   │                       ██████          █                       │
   │                                                              │
   │   Startup time        1000-3000 ms   200-500 ms              │
   │                       ████            █                       │
   │                                                              │
   │   Bundle includes     Node.js +      System WebView           │
   │                       Chromium                                │
   │                                                              │
   │   Frontend:           HTML/CSS/JS    HTML/CSS/JS             │
   │   Backend:            Node.js        Rust                    │
   │                       Garbage coll.  No GC, ownership        │
   │                                                              │
   │   Security:           Less strict    Capability + CSP        │
   │   File access:        Full nodejs    Scoped (default deny)   │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   
   When Tauri better:
   ──────────────────
   ✅ Distribute small binaries (< 20MB target)
   ✅ Low-RAM environment (kiosk, old hardware)
   ✅ Security-conscious (defense in depth)
   ✅ Want native performance from backend
   ✅ Cross-platform desktop + mobile
   
   When Electron better:
   ─────────────────────
   ✅ Existing Electron app (no migrate need)
   ✅ Heavy reliance on Node.js packages
   ✅ Team only knows JS, not Rust
```

---

## 18. Security model layers

```
   ┌──────────────────────────────────────────────────────────┐
   │                  DEFENSE IN DEPTH                        │
   │                                                          │
   │   LAYER 1: WebView CSP                                   │
   │   ────────────────────                                   │
   │   Prevent XSS execution in frontend                      │
   │                                                          │
   │   LAYER 2: Capability-based IPC                          │
   │   ───────────────────────────                            │
   │   Even if XSS happens, can't call arbitrary commands     │
   │                                                          │
   │   LAYER 3: Permission scopes                             │
   │   ─────────────────────                                  │
   │   fs:scope-app → can't read /etc/passwd                  │
   │   http: only allowed URLs                                │
   │                                                          │
   │   LAYER 4: Rust memory safety                            │
   │   ──────────────────────                                 │
   │   No buffer overflow in Rust commands                    │
   │                                                          │
   │   LAYER 5: Code signing                                  │
   │   ──────────────                                         │
   │   User verifies app authenticity                         │
   │                                                          │
   │   LAYER 6: Signed updates                                │
   │   ────────────────                                       │
   │   Prevent malicious updates                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Compare Electron (default):                            │
   │   • nodejs full access from renderer (if context isolated │
   │     disabled — common bad config)                        │
   │   • CSP optional                                         │
   │   • No capability system                                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 19. Common patterns visualization

```
   ✅ 1. Async command với state:
   ───────────────────────────
   #[tauri::command]
   async fn fetch_user(
       state: tauri::State<'_, DbState>,
       id: u64,
   ) -> Result<User, String> {
       sqlx::query_as!(...).fetch_one(&state.db).await
           .map_err(|e| e.to_string())
   }
   
   
   ✅ 2. Background task with events:
   ──────────────────────────────
   #[tauri::command]
   async fn start_sync(window: tauri::Window) {
       tokio::spawn(async move {
           loop {
               // sync work
               window.emit("sync-progress", percent).unwrap();
               tokio::time::sleep(Duration::from_secs(10)).await;
           }
       });
   }
   
   
   ✅ 3. Typed bindings (tauri-specta):
   ────────────────────────────────
   Auto-generate TS from Rust types
   No manual sync
   
   
   ✅ 4. Plugins for common needs:
   ───────────────────────────
   tauri-plugin-store     → persistent KV
   tauri-plugin-sql       → embedded DB
   tauri-plugin-http      → bypass CORS
   tauri-plugin-notification → native notifs
   
   
   ✅ 5. Capabilities + CSP:
   ─────────────────────
   Least privilege
   Defense in depth
```

---

## 20. Antipatterns visualization

```
   ❌ 1. Sync blocking in command:
   ──────────────────────────────
   #[tauri::command]
   fn slow() -> String {
       std::thread::sleep(Duration::from_secs(5));  // ❌ blocks UI
       "done".into()
   }
   
   ✅ Async:
   async fn slow() -> String {
       tokio::time::sleep(Duration::from_secs(5)).await;
       "done".into()
   }
   
   
   ❌ 2. Many small IPC calls:
   ──────────────────────────
   for (let i = 0; i < 1000; i++) {
       await invoke('process', { i });  // 1000 IPC = slow
   }
   
   ✅ Batch:
   await invoke('process_batch', { items: range(0, 1000) });
   
   
   ❌ 3. Skip code signing in prod:
   ────────────────────────────
   Unsigned macOS app → user blocked by Gatekeeper
   Unsigned Windows → SmartScreen warning
   
   ✅ Sign for production
   
   
   ❌ 4. Skip updater signature:
   ────────────────────────────
   Unsigned update → attacker MITM serves malicious version
   
   ✅ Sign updates, verify pubkey on client
   
   
   ❌ 5. Allow all permissions:
   ───────────────────────────
   "permissions": ["fs:allow-read", "fs:allow-write"]  // ❌ all paths
   
   ✅ Scope:
   "permissions": ["fs:scope-app"]   // only AppData
   
   
   ❌ 6. Shell execution from frontend:
   ────────────────────────────────
   await invoke('exec', { cmd: userInput });  // ❌ INJECTION!
   
   ✅ Whitelist commands or don't expose:
   await invoke('do_specific_action');
```

---

## 21. Mind map cuối

```
                              TAURI
                                │
        ┌────────────┬──────────┼──────────┬─────────────┐
        ▼            ▼          ▼          ▼             ▼
   ARCHITECTURE  IPC       SECURITY   PACKAGING    PLATFORM
        │            │          │          │             │
   Rust+WebView Commands  Capabilities Bundle      Windows
   Threads       Events    CSP         Code sign   macOS
   Process       State     Scope       Updater     Linux
                 Channel   Plugin perms             iOS (v2)
                                                    Android(v2)
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. Rust trusted, WebView untrusted │
                │                                      │
                │  2. Commands for RPC, Events pub/sub │
                │                                      │
                │  3. State via .manage(Arc<T>)        │
                │                                      │
                │  4. tauri-specta for typed bindings  │
                │                                      │
                │  5. Capabilities + CSP defense       │
                │                                      │
                │  6. fs scoped to app dirs            │
                │                                      │
                │  7. Code-sign macOS/Windows          │
                │                                      │
                │  8. Sign updates, verify pubkey      │
                │                                      │
                │  9. Async commands + spawn_blocking  │
                │                                      │
                │  10. Batch IPC, reduce calls         │
                │                                      │
                │  11. Plugins ecosystem — don't       │
                │      reinvent                        │
                │                                      │
                │  12. Test all platforms (Win/Mac/    │
                │      Linux + mobile if needed)       │
                └──────────────────────────────────────┘
```

---

## 22. Bộ tài liệu Rust giờ có 19 chương!

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   1-16. Core foundations                                 │
   │                                                          │
   │  17. axum-project            — Web realistic            │
   │  18. database                — DB deep dive             │
   │  19. tauri                   — Desktop & Mobile app     │
   │      tauri-visual            ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng FULL-STACK Rust:                         │
   │                                                          │
   │   🌐 Web (axum)                                          │
   │   🗄️ Database (sqlx)                                     │
   │   🔌 Embedded (no_std + embassy)                         │
   │   🖥️ Desktop (Tauri)                                     │
   │   📱 Mobile (Tauri v2)                                   │
   │   🚀 High-performance (profiling)                        │
   │   🧪 Testing                                             │
   │   🔍 Observability                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Bộ tài liệu giờ có 19 chương. Nếu muốn đào tiếp:

- **WASM (WebAssembly)** — Rust compile to WASM cho browser + edge
- **Game engines** — Bevy ECS framework
- **CLI tools** — clap, dialoguer, indicatif
- **GUI native** — egui, iced (pure Rust GUI, no webview)
- **gRPC services** — tonic
- **Message queues** — Kafka, NATS, RabbitMQ clients
- **Cryptography** — rustls, ring, age, secrecy

Báo cái nào muốn đào sâu! 🦀⚡
