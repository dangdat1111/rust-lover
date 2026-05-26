# CLI Tools — Minh Hoạ Trực Quan

> Companion visual cho [u-cli-tools.md](./u-cli-tools.md). Đọc song song.

---

## 1. Bức tranh lớn — CLI Tools Universe

```
                          CLI TOOLS RUST UNIVERSE
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   Stack tổng quan:                                     │
       │                                                        │
       │   ┌─────────────────────────────────────────────────┐  │
       │   │  Argument parsing:  clap (derive | builder)     │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Interactive UX:    dialoguer (prompts)         │  │
       │   │                     indicatif (progress)        │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Terminal UI:       ratatui + crossterm         │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Styling:           console / anstyle           │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Config:            figment, dirs               │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Errors:            anyhow + miette             │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Logging:           tracing-subscriber          │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Testing:           assert_cmd + assert_fs      │  │
       │   ├─────────────────────────────────────────────────┤  │
       │   │  Distribution:      cargo-dist (multi-platform) │  │
       │   └─────────────────────────────────────────────────┘  │
       │                                                        │
       │   Real-world Rust CLIs:                                │
       │   ripgrep, fd, bat, exa, starship, delta,              │
       │   bottom, just, helix, gitui, cargo                    │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. CLI types

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TYPE 1: Simple command                                 │
   │   ─────────────────                                      │
   │   $ mytool input.txt                                     │
   │     → process, output, exit                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   TYPE 2: Subcommand router (git-style)                  │
   │   ──────────────────────                                 │
   │   $ mytool init                                          │
   │   $ mytool add file                                      │
   │   $ mytool commit -m "..."                               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   TYPE 3: Interactive prompts                            │
   │   ────────────────────                                   │
   │   $ mytool                                               │
   │   > What's your name? Alice                              │
   │   > Choose theme: dark/light                             │
   │   ...                                                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   TYPE 4: Full TUI (terminal app)                        │
   │   ──────────────                                         │
   │   ┌──────────────────────────────────────┐               │
   │   │  Files          │  Preview            │              │
   │   │  ─────          │  ─────              │              │
   │   │  > foo.txt      │  Lorem ipsum...     │              │
   │   │    bar.rs       │  ...                │              │
   │   │    baz.md       │  ...                │              │
   │   └──────────────────────────────────────┘               │
   │   Arrow keys, mouse, full-screen modal                   │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   TYPE 5: Daemon / service                               │
   │   ─────────────────                                      │
   │   $ mytool start     (background)                        │
   │   $ mytool status                                        │
   │   $ mytool stop                                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 3. clap derive — Flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Rust source:                                           │
   │                                                          │
   │   #[derive(Parser)]                                      │
   │   #[command(name = "greet", version)]                    │
   │   struct Cli {                                           │
   │       /// Name to greet                                  │
   │       name: String,                                      │
   │                                                          │
   │       /// Number of times                                │
   │       #[arg(short, long, default_value_t = 1)]           │
   │       count: u32,                                        │
   │   }                                                      │
   │                                                          │
   │   fn main() {                                            │
   │       let cli = Cli::parse();                            │
   │       for _ in 0..cli.count {                            │
   │           println!("Hello, {}!", cli.name);              │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Compile time:                                          │
   │   • derive macro generates ARG_PARSER code               │
   │   • Reads doc comments → help text                       │
   │   • Type info → validation                               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Runtime:                                               │
   │                                                          │
   │   $ greet --help                                         │
   │   ┌─────────────────────────────────────┐                │
   │   │ Usage: greet [OPTIONS] <NAME>       │                │
   │   │                                     │                │
   │   │ Arguments:                          │                │
   │   │   <NAME>  Name to greet             │                │
   │   │                                     │                │
   │   │ Options:                            │                │
   │   │   -c, --count <COUNT>  [default: 1] │                │
   │   │   -h, --help                        │                │
   │   │   -V, --version                     │                │
   │   └─────────────────────────────────────┘                │
   │                                                          │
   │   $ greet Alice --count 3                                │
   │   Hello, Alice!                                          │
   │   Hello, Alice!                                          │
   │   Hello, Alice!                                          │
   │                                                          │
   │   $ greet --count abc                                    │
   │   error: invalid value 'abc' for '--count'               │
   │                                                          │
   │   $ greet                                                │
   │   error: missing required argument <NAME>                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. Subcommands hierarchy

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   mytool                                                 │
   │   │                                                      │
   │   ├── init <name>          → Initialize project          │
   │   │                                                      │
   │   ├── add <files...>       → Add files                    │
   │   │   └── --recursive                                    │
   │   │                                                      │
   │   ├── status               → Show status                 │
   │   │                                                      │
   │   ├── config               → Configuration commands      │
   │   │   ├── get <key>                                      │
   │   │   ├── set <key> <value>                              │
   │   │   └── list                                           │
   │   │                                                      │
   │   ├── user                 → User commands               │
   │   │   ├── create <name>                                  │
   │   │   ├── delete <id>                                    │
   │   │   └── list                                           │
   │   │                                                      │
   │   └── completions <shell>  → Generate completions        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Rust code:
   ──────────
   
   #[derive(Subcommand)]
   enum Commands {
       Init { name: String },
       Add { files: Vec<PathBuf>, #[arg(short, long)] recursive: bool },
       Status,
       Config { #[command(subcommand)] action: ConfigCmds },
       User { #[command(subcommand)] action: UserCmds },
   }
   
   #[derive(Subcommand)]
   enum ConfigCmds {
       Get { key: String },
       Set { key: String, value: String },
       List,
   }
   
   Dispatch:
   ─────────
   match cli.command {
       Commands::Init { name } => commands::init(name),
       Commands::Add { files, recursive } => commands::add(files, recursive),
       Commands::Config { action } => match action {
           ConfigCmds::Get { key } => commands::config_get(&key),
           ...
       },
       ...
   }
```

---

## 5. dialoguer — Interactive prompts

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   INPUT:                                                 │
   │   $ mytool init                                          │
   │                                                          │
   │   Project name › my-app                                  │
   │                                                          │
   │   Email › alice@example.com                              │
   │     ✓ validated: contains '@'                            │
   │                                                          │
   │   Port › 8080 (default)                                  │
   │                                                          │
   │   Password › ********                                    │
   │   Confirm  › ********                                    │
   │     ✓ match                                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   CONFIRM:                                               │
   │   ? Delete all files? › (y/N) y                          │
   │   ✓ Deleting...                                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   SELECT:                                                │
   │   ? Choose language ›                                    │
   │     Rust                                                 │
   │   ❯ Go                ← highlighted                       │
   │     Python                                               │
   │     Ruby                                                 │
   │   (arrow keys to navigate, enter to confirm)             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   MULTI-SELECT:                                          │
   │   ? Select features (space to toggle):                   │
   │     [x] Async                                            │
   │     [ ] Database                                         │
   │   ❯ [x] HTTP                                              │
   │     [ ] TLS                                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   FUZZY SELECT (like fzf):                               │
   │   ? Country › Viet                                       │
   │     ❯ Vietnam                                            │
   │       Vienna                                             │
   │     (filters as you type)                                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 6. indicatif — Progress visualization

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   BASIC PROGRESS BAR:                                    │
   │                                                          │
   │   ████████████████░░░░░░░░░░░░░░░░ 50/100 ETA 30s        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   CUSTOM STYLE:                                          │
   │                                                          │
   │   ⠋ [00:00:05] [██████████░░░░░░░] 50/100 (ETA 30s)      │
   │   ↑ spinner                                              │
   │       ↑ elapsed                                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   SPINNER (indeterminate):                               │
   │                                                          │
   │   ⠋ Connecting to server...                              │
   │   ⠙ Connecting to server...                              │
   │   ⠹ Connecting to server...                              │
   │   ⠸ Connecting to server...                              │
   │   (animation continues until finished)                   │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   MULTI-PROGRESS (parallel):                             │
   │                                                          │
   │   [Worker 0] ████████████████████ 100/100                │
   │   [Worker 1] █████████░░░░░░░░░░░ 45/100  (running)      │
   │   [Worker 2] ███████████░░░░░░░░░ 55/100  (running)      │
   │   [Worker 3] ███████░░░░░░░░░░░░░ 35/100  (running)      │
   │   [Worker 4] █████████████░░░░░░░ 65/100  (running)      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WITH ITERATOR:                                         │
   │                                                          │
   │   for item in items.iter().progress() {                  │
   │       process(item);                                     │
   │   }                                                      │
   │   // auto progress bar based on iter count               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WITH RAYON (parallel):                                 │
   │                                                          │
   │   items.par_iter()                                       │
   │       .progress_count(items.len() as u64)                │
   │       .map(process)                                      │
   │       .collect()                                         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. Terminal styling

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   COLORS:                                                │
   │                                                          │
   │   ⚠️ Warning    ← yellow                                  │
   │   ❌ Error      ← red                                    │
   │   ✅ Success    ← green                                  │
   │   ℹ️ Info        ← blue                                   │
   │                                                          │
   │   STYLES:                                                │
   │                                                          │
   │   Bold text         ← style().bold()                     │
   │   Italic text       ← style().italic()                   │
   │   Underlined        ← style().underlined()               │
   │   Strikethrough     ← style().strikethrough()            │
   │   Inverse           ← style().reverse()                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   TERMINAL DETECTION:                                    │
   │                                                          │
   │   $ mytool         ← TTY, use colors                     │
   │   ✅ Success                                              │
   │                                                          │
   │   $ mytool | cat   ← pipe, plain text                    │
   │   Success                                                │
   │                                                          │
   │   $ NO_COLOR=1 mytool   ← respect NO_COLOR              │
   │   Success                                                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Implementation:                                        │
   │                                                          │
   │   use console::{style, Term};                            │
   │   use atty::Stream;                                      │
   │                                                          │
   │   let use_color = atty::is(Stream::Stdout)               │
   │       && std::env::var_os("NO_COLOR").is_none();         │
   │                                                          │
   │   if use_color {                                         │
   │       println!("{}", style("Error").red().bold());        │
   │   } else {                                               │
   │       println!("Error");                                 │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. ratatui — Full TUI architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TUI APP LIFECYCLE:                                     │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Setup:                                          │    │
   │   │  - enable_raw_mode()                            │    │
   │   │  - EnterAlternateScreen                         │    │
   │   │  - new Terminal                                 │    │
   │   └─────────────────────────────────────────────────┘    │
   │                       │                                  │
   │                       ▼                                  │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ MAIN LOOP:                                      │    │
   │   │                                                 │    │
   │   │   loop {                                        │    │
   │   │     terminal.draw(|f| ui(f, &app_state))?;       │   │
   │   │     ↓                                           │    │
   │   │     RENDER full frame to buffer                 │    │
   │   │     DIFF with prev frame                        │    │
   │   │     SEND only deltas to terminal (efficient)    │    │
   │   │     ↓                                           │    │
   │   │                                                 │    │
   │   │     match event::read()? {                      │    │
   │   │         Key('q') => break,                       │    │
   │   │         Key(Down) => app.next_item(),           │    │
   │   │         ...                                     │    │
   │   │     }                                           │    │
   │   │   }                                             │    │
   │   └─────────────────────────────────────────────────┘    │
   │                       │                                  │
   │                       ▼                                  │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ Cleanup:                                        │    │
   │   │  - LeaveAlternateScreen                         │    │
   │   │  - disable_raw_mode()                           │    │
   │   │  - show_cursor()                                │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   Frame rate: 30-60 FPS typical (or event-driven)        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. ratatui layout

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   LAYOUT — like CSS Flexbox for terminal                 │
   │                                                          │
   │   let chunks = Layout::default()                          │
   │       .direction(Direction::Horizontal)                  │
   │       .constraints([                                     │
   │           Constraint::Percentage(30),  // left 30%       │
   │           Constraint::Min(0),           // right rest     │
   │       ])                                                 │
   │       .split(area);                                      │
   │                                                          │
   │   Result:                                                │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ chunks[0] (30%) │ chunks[1] (70%)            │       │
   │   │                 │                             │      │
   │   │   Left panel    │      Right panel            │      │
   │   │                 │                             │      │
   │   │                 │                             │      │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   NESTED LAYOUT:                                         │
   │                                                          │
   │   let main = Layout::vertical([                          │
   │       Constraint::Length(3),     // header               │
   │       Constraint::Min(0),         // content              │
   │       Constraint::Length(1),     // footer status         │
   │   ]).split(area);                                        │
   │                                                          │
   │   let content = Layout::horizontal([                     │
   │       Constraint::Percentage(30),  // sidebar             │
   │       Constraint::Min(0),          // main view           │
   │   ]).split(main[1]);                                     │
   │                                                          │
   │   Result:                                                │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Header                                       │       │
   │   ├────────────┬─────────────────────────────────┤       │
   │   │            │                                 │       │
   │   │ Sidebar    │  Main View                      │       │
   │   │            │                                 │       │
   │   │            │                                 │       │
   │   ├────────────┴─────────────────────────────────┤       │
   │   │ Status: connected | 5 items                  │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. ratatui widgets

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Built-in widgets:                                      │
   │                                                          │
   │   ┌─────────────────────┐                                │
   │   │ Block (border)      │  ← container with border       │
   │   │ ┌─────────────────┐ │                                │
   │   │ │  content        │ │                                │
   │   │ └─────────────────┘ │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   │   Paragraph:                                             │
   │   ┌─────────────────────┐                                │
   │   │ Lorem ipsum dolor   │                                │
   │   │ sit amet, ...       │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   │   List:                                                  │
   │   ┌─────────────────────┐                                │
   │   │   Item 1            │                                │
   │   │ ▶ Item 2 (selected) │ ← highlight                    │
   │   │   Item 3            │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   │   Table:                                                 │
   │   ┌─────────────────────────┐                            │
   │   │ Name      Email          │                            │
   │   │ ──────────────────────── │                           │
   │   │ Alice     a@example.com  │                           │
   │   │ Bob       b@example.com  │                            │
   │   └─────────────────────────┘                            │
   │                                                          │
   │   Tabs:                                                  │
   │   ┌────────────────────────────────────┐                │
   │   │ │ Files │ Editor │ Console │       │                │
   │   └────────────────────────────────────┘                │
   │                                                          │
   │   Gauge:                                                 │
   │   ┌─────────────────────┐                                │
   │   │ ████████░░░░░ 60%   │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   │   BarChart:                                              │
   │   ┌─────────────────────┐                                │
   │   │ ▆ ▇ █ ▆ ▃ ▅ ▇        │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   │   Sparkline:                                             │
   │   ┌─────────────────────┐                                │
   │   │ ▁▂▄▅▃▆▇█▆▄▂▁         │                                │
   │   └─────────────────────┘                                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 11. Full TUI app example

```
   ┌──────────────────────────────────────────────────────────┐
   │ File Manager TUI (like a simple `broot`)                 │
   │                                                          │
   │ ┌───────────────────────────────────────────────────┐    │
   │ │ ~/projects/myapp                                  │    │
   │ ├───────────────────┬───────────────────────────────┤    │
   │ │ Files             │ Preview                       │    │
   │ │ ─────             │ ─────                         │    │
   │ │ ▶ src/             │ /// Main entry point          │    │
   │ │   tests/           │ fn main() {                   │    │
   │ │   Cargo.toml       │     println!("Hello");        │    │
   │ │   README.md        │ }                             │    │
   │ │   .gitignore        │                              │    │
   │ │   ...              │                               │    │
   │ │                   │                               │    │
   │ │                   │                               │    │
   │ ├───────────────────┴───────────────────────────────┤    │
   │ │ ↑↓: navigate │ →: enter dir │ ←: back │ q: quit  │    │
   │ └───────────────────────────────────────────────────┘    │
   │                                                          │
   │ State: current dir, selected index, scroll offset,        │
   │        search query, mode (browse/search/edit)           │
   │                                                          │
   │ Each frame: draw all from state. No imperative mutate.   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 12. Configuration layering

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Priority (highest first):                              │
   │                                                          │
   │   1. CLI arguments        $ mytool --port 9000           │
   │      │                                                   │
   │   2. Environment vars     $ MYTOOL_PORT=9000 mytool      │
   │      │                                                   │
   │   3. Local config         ./mytool.toml                  │
   │      │                                                   │
   │   4. User config          ~/.config/mytool/config.toml   │
   │      │                                                   │
   │   5. System config        /etc/mytool/config.toml        │
   │      │                                                   │
   │   6. Defaults             (hard-coded in code)           │
   │                                                          │
   │   ─────────────────────────────────────                  │
   │                                                          │
   │   Use figment crate to layer:                            │
   │                                                          │
   │   Figment::from(Serialized::defaults(Config::default())) │
   │       .merge(Toml::file("/etc/mytool/config.toml"))      │
   │       .merge(Toml::file(user_config_path))               │
   │       .merge(Toml::file("./mytool.toml"))                │
   │       .merge(Env::prefixed("MYTOOL_"))                   │
   │       .extract()                                         │
   │                                                          │
   │   Then CLI args override extracted values explicitly.    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Platform paths (dirs crate):                           │
   │                                                          │
   │   ┌──────────────────┬──────────────────────────────┐    │
   │   │ Platform         │ config_dir()                 │    │
   │   ├──────────────────┼──────────────────────────────┤    │
   │   │ Linux            │ ~/.config                    │    │
   │   │ macOS            │ ~/Library/Application Support│    │
   │   │ Windows          │ %APPDATA%                    │    │
   │   └──────────────────┴──────────────────────────────┘    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 13. Output vs status streams

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   STDOUT vs STDERR:                                      │
   │                                                          │
   │   ┌────────────────────┬─────────────────────────────┐   │
   │   │ STDOUT (data)      │ STDERR (status/logs)        │   │
   │   ├────────────────────┼─────────────────────────────┤   │
   │   │ Machine-readable   │ Human-friendly              │   │
   │   │ Pipes-friendly     │ Doesn't break pipes         │   │
   │   │ Results            │ Progress, info, errors      │   │
   │   │                    │                             │   │
   │   │ println!(...)      │ eprintln!(...)              │   │
   │   │                    │ tracing::info!(...)         │   │
   │   └────────────────────┴─────────────────────────────┘   │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ❌ ANTIPATTERN:                                        │
   │                                                          │
   │   println!("Connecting to server...");   ← status        │
   │   println!("{}", json_data);              ← data         │
   │                                                          │
   │   $ mytool | jq '.field'                                 │
   │   parse error: Connecting to server...                   │
   │   (jq tries to parse status as JSON, fails!)             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ CORRECT:                                            │
   │                                                          │
   │   eprintln!("Connecting...");      ← stderr              │
   │   println!("{}", json_data);        ← stdout              │
   │                                                          │
   │   $ mytool | jq '.field'                                 │
   │   "the_field_value"                                      │
   │   (status visible on terminal, doesn't break pipe)       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. Logging strategy

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   3-tier logging:                                        │
   │                                                          │
   │   ┌────────────────┐                                     │
   │   │ STDOUT          │  → program OUTPUT (results)        │
   │   │ (println!)      │     Pipes here                     │
   │   └────────────────┘                                     │
   │                                                          │
   │   ┌────────────────┐                                     │
   │   │ STDERR          │  → STATUS, info, warning           │
   │   │ (tracing,       │     Human reads                    │
   │   │  eprintln!)     │     Doesn't break pipes            │
   │   └────────────────┘                                     │
   │                                                          │
   │   ┌────────────────┐                                     │
   │   │ LOG FILE        │  → Detailed debug                  │
   │   │ (optional)      │     Post-mortem                    │
   │   │                 │     JSON for analysis              │
   │   └────────────────┘                                     │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Verbosity via -v flag:                                 │
   │                                                          │
   │   $ mytool          → warn level                          │
   │   $ mytool -v       → info level                          │
   │   $ mytool -vv      → debug level                         │
   │   $ mytool -vvv     → trace level                         │
   │                                                          │
   │   Implementation:                                        │
   │                                                          │
   │   let level = match cli.verbose {                        │
   │       0 => "warn",                                       │
   │       1 => "info",                                       │
   │       2 => "debug",                                      │
   │       _ => "trace",                                      │
   │   };                                                     │
   │                                                          │
   │   tracing_subscriber::fmt()                              │
   │       .with_env_filter(format!("mytool={}", level))      │
   │       .with_writer(std::io::stderr)   ← KEY: stderr!    │
   │       .init();                                           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 15. Error handling cho CLI

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ❌ BAD — stack trace to user:                          │
   │                                                          │
   │   $ mytool bad_input                                     │
   │   thread 'main' panicked at src/main.rs:42:5:            │
   │   called `Result::unwrap()` on an `Err` value:           │
   │   NotFound { ... }                                       │
   │   note: run with `RUST_BACKTRACE=1` ...                  │
   │                                                          │
   │   ⟹ User confused, doesn't know what to do                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ GOOD — clean error with context:                    │
   │                                                          │
   │   $ mytool bad_input                                     │
   │   Error: processing input                                │
   │   Caused by:                                             │
   │       0: file not found: /tmp/bad_input                  │
   │       1: No such file or directory (os error 2)          │
   │                                                          │
   │   Implementation:                                        │
   │                                                          │
   │   use anyhow::{Context, Result};                         │
   │                                                          │
   │   fn run() -> Result<()> {                               │
   │       process(&input)                                    │
   │           .context("processing input")?;                 │
   │       Ok(())                                             │
   │   }                                                      │
   │                                                          │
   │   fn main() -> ExitCode {                                │
   │       if let Err(e) = run() {                            │
   │           eprintln!("Error: {:#}", e);  // {:#} = chain  │
   │           return ExitCode::from(1);                      │
   │       }                                                  │
   │       ExitCode::SUCCESS                                  │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ EXCELLENT — miette fancy errors:                    │
   │                                                          │
   │   Error: mytool::parse_error                             │
   │                                                          │
   │     × invalid config syntax                              │
   │      ╭─[config.toml:3:5]                                 │
   │    2 │ port = 8080                                       │
   │    3 │ timeout = "thirty"                                │
   │      ·          ─────────                                │
   │      ·              ╰── expected number                  │
   │    4 │ server = "..."                                    │
   │      ╰────                                               │
   │     help: timeout must be an integer (seconds)           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 16. Testing CLI

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   3 LEVELS:                                              │
   │                                                          │
   │   1. UNIT TESTS — pure logic                             │
   │      ──────────                                          │
   │      pub fn count_lines(s: &str) -> usize {              │
   │          s.lines().count()                               │
   │      }                                                   │
   │                                                          │
   │      #[test]                                             │
   │      fn test_count() {                                   │
   │          assert_eq!(count_lines("a\nb"), 2);             │
   │      }                                                   │
   │                                                          │
   │      Fast, no subprocess.                                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. INTEGRATION TESTS — full CLI subprocess             │
   │      ─────────────────                                   │
   │      use assert_cmd::Command;                            │
   │                                                          │
   │      #[test]                                             │
   │      fn test_help() {                                    │
   │          Command::cargo_bin("mytool").unwrap()           │
   │              .arg("--help")                              │
   │              .assert()                                   │
   │              .success()                                  │
   │              .stdout(predicate::str::contains("Usage:"));│
   │      }                                                   │
   │                                                          │
   │      Build → run as subprocess → check stdout/stderr/exit│
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. TUI TESTS — render to in-memory buffer              │
   │      ──────────                                          │
   │      use ratatui::backend::TestBackend;                  │
   │                                                          │
   │      #[test]                                             │
   │      fn test_ui() {                                      │
   │          let backend = TestBackend::new(80, 24);         │
   │          let mut terminal = Terminal::new(backend)?;     │
   │          terminal.draw(|f| ui(f, &state))?;              │
   │                                                          │
   │          let buf = terminal.backend().buffer();          │
   │          // assert chars at positions                    │
   │      }                                                   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Distribution matrix

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Audience              Method            Command         │
   │   ───────────────────────────────────────────────────    │
   │                                                          │
   │   Rust developers        cargo install     cargo install │
   │                                            mytool         │
   │                                                          │
   │   macOS users            Homebrew          brew install  │
   │                                            you/tap/mytool│
   │                                                          │
   │   Linux                  curl|sh installer curl -L \     │
   │                                            ...|sh        │
   │                                                          │
   │                          apt/dpkg          sudo dpkg -i  │
   │                                            mytool.deb    │
   │                                                          │
   │   Windows                Scoop              scoop install │
   │                                            mytool         │
   │                                                          │
   │                          MSI installer     download .msi  │
   │                                                          │
   │   JS developers          npm wrapper       npm install -g │
   │                                            mytool         │
   │                                                          │
   │   DevOps                 Docker image      docker run    │
   │                                            myorg/mytool   │
   │                                                          │
   │   Everyone               GitHub Releases   download from  │
   │                          (pre-built)        release page  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   cargo-dist automates ALL of this:                      │
   │                                                          │
   │   $ cargo install cargo-dist                             │
   │   $ cargo dist init                                       │
   │   $ git tag v1.0 && git push --tags                      │
   │       ↓                                                  │
   │   GitHub Actions runs:                                   │
   │   • Build for Linux x86_64, ARM                          │
   │   • Build for macOS x86_64, ARM                          │
   │   • Build for Windows                                    │
   │   • Generate installer scripts                           │
   │   • Generate Homebrew formula                            │
   │   • Generate Scoop manifest                              │
   │   • Attach all to GitHub Release                         │
   │                                                          │
   │   Done. Users install with their preferred method.        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. Performance — Startup time

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   STARTUP COMPARISON:                                    │
   │                                                          │
   │   Rust CLI         ░░ ~5-20ms                            │
   │   Go CLI           ░░░ ~10-30ms                          │
   │   Node.js CLI      ██░░░ ~100-300ms                      │
   │   Python CLI       ██░ ~50-200ms                          │
   │   Java CLI         █████████░░░░░ ~500-2000ms             │
   │                                                          │
   │   For frequent invocations (shell prompt, file watchers),│
   │   every ms counts.                                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   MEASURE:                                               │
   │                                                          │
   │   $ hyperfine --warmup 3 --runs 100 'mytool --version'   │
   │                                                          │
   │   Benchmark 1: mytool --version                          │
   │     Time (mean ± σ):       5.2 ms ±  0.5 ms              │
   │     Range (min … max):     4.8 ms …   7.0 ms             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   OPTIMIZE:                                              │
   │                                                          │
   │   [profile.release]                                      │
   │   opt-level = 3                                          │
   │   lto = true                                             │
   │   codegen-units = 1                                      │
   │   strip = true                                           │
   │   panic = "abort"                                        │
   │                                                          │
   │   Disable unused clap features:                          │
   │   clap = { features = ["derive"], default-features = false }│
   │                                                          │
   │   Lazy-init heavy deps:                                  │
   │   match cli.command {                                    │
   │       Commands::Version => return print_version(),      │
   │       Commands::Server => {                              │
   │           let rt = tokio::runtime::Runtime::new()?;      │
   │           rt.block_on(...)                              │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   │   Don't init tokio for --version!                        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 19. Standard CLI conventions

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ✅ STANDARD CONVENTIONS:                               │
   │                                                          │
   │   Flag                Purpose                            │
   │   ───────────────────────────                            │
   │   -h, --help          Show help                          │
   │   -V, --version       Show version                       │
   │   -v, --verbose       Increase verbosity (repeatable)    │
   │   -q, --quiet         Suppress output                    │
   │   -f, --force         Override safety                    │
   │   -y, --yes           Auto-confirm                       │
   │   -n, --dry-run       Show what would happen             │
   │   -o, --output        Output destination                  │
   │   -i, --input         Input source                        │
   │   --color=<when>      always/auto/never                  │
   │   --json              Machine-readable output            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ STANDARD BEHAVIORS:                                 │
   │                                                          │
   │   • Stdout for data                                      │
   │   • Stderr for status/errors                             │
   │   • Exit 0 success, non-zero error                       │
   │   • Color when TTY, plain when piped                     │
   │   • Respect NO_COLOR env var                             │
   │   • Read stdin if no input arg                           │
   │   • Graceful Ctrl+C (cleanup, exit 130)                  │
   │   • --help works without other args                      │
   │   • Suggest similar commands on typos                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ DESTRUCTIVE OPS PROTECTION:                         │
   │                                                          │
   │   Before delete/overwrite:                               │
   │   - Confirm prompt                                       │
   │   - Or require -f / --force flag                          │
   │   - Or have --dry-run option                              │
   │                                                          │
   │   $ mytool delete *.txt                                  │
   │   ? Delete 42 files? (y/N)                               │
   │                                                          │
   │   $ mytool delete *.txt --dry-run                        │
   │   Would delete: foo.txt                                  │
   │   Would delete: bar.txt                                  │
   │   ...                                                    │
   │                                                          │
   │   $ mytool delete *.txt --force                           │
   │   Deleted: foo.txt                                       │
   │   Deleted: bar.txt                                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 20. Common antipatterns

```
   ❌ 1. Mix output + status on stdout
   ───────────────────────────────
   println!("Connecting...");   ← status
   println!("{}", json_data);    ← data
   
   ⟹ Breaks pipes
   
   ✅ eprintln!("Connecting...");
      println!("{}", json_data);
   
   
   ❌ 2. Hard-coded paths
   ────────────────────
   let config = "/home/user/.mytool/config.toml";
   
   ✅ dirs::config_dir().unwrap().join("mytool/config.toml")
   
   
   ❌ 3. Panic for user errors
   ───────────────────────
   let f = std::fs::read("input").unwrap();
   ⟹ thread 'main' panicked... — confuses users
   
   ✅ Use anyhow with context:
   let f = std::fs::read(path)
       .with_context(|| format!("reading {}", path.display()))?;
   
   
   ❌ 4. Color in pipe
   ────────────────
   println!("{}", style("ERROR").red());
   ⟹ ANSI escape codes in piped output
   
   ✅ Detect TTY:
   if atty::is(Stream::Stdout) {
       println!("{}", style("ERROR").red());
   } else {
       println!("ERROR");
   }
   
   
   ❌ 5. Slow startup
   ────────────────
   fn main() {
       let rt = tokio::runtime::Runtime::new()?;   ← always init
       rt.block_on(...)
   }
   
   $ mytool --version    ← waits for tokio init!
   
   ✅ Lazy init:
   match cli.command {
       Version => print_version(),   ← no runtime needed
       Server => {
           let rt = tokio::runtime::Runtime::new()?;
           rt.block_on(server())
       }
   }
   
   
   ❌ 6. Verbose flag = boolean
   ─────────────────────────
   if cli.verbose { /* extra log */ }
   
   ✅ Repeatable for level:
   #[arg(short, long, action = ArgAction::Count)]
   verbose: u8,
   
   Then -v=info, -vv=debug, -vvv=trace
   
   
   ❌ 7. Stack trace to user
   ──────────────────────
   .unwrap()
   ⟹ User sees "thread 'main' panicked..."
   
   ✅ Custom main that handles errors:
   fn main() -> ExitCode {
       if let Err(e) = run() {
           eprintln!("Error: {:#}", e);
           return ExitCode::from(1);
       }
       ExitCode::SUCCESS
   }
```

---

## 21. Mind map cuối

```
                              CLI TOOLS
                                  │
        ┌────────────┬────────────┼────────────┬─────────────┐
        ▼            ▼            ▼            ▼             ▼
    ARGS         INTERACTIVE   DISPLAY      CONFIG       DEPLOYMENT
        │            │            │            │             │
    clap         dialoguer    indicatif    figment      cargo-dist
    derive       prompts      progress     dirs         Homebrew
    builder      Confirm      spinner      env vars     Scoop
    subcommands  Select       multi-prog   layered      npm
                  FuzzySelect ratatui      cli args     Docker
                  Editor      console      defaults     deb/rpm
                              styling
                              
                              
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. clap derive for type-safe args   │
                │                                      │
                │  2. Subcommands like git/cargo       │
                │                                      │
                │  3. dialoguer + indicatif for UX    │
                │                                      │
                │  4. ratatui + crossterm for TUI      │
                │                                      │
                │  5. stdout = data, stderr = status   │
                │                                      │
                │  6. NO_COLOR + TTY detection         │
                │                                      │
                │  7. Layered config (defaults<file<env│
                │     <cli)                            │
                │                                      │
                │  8. anyhow context for user errors   │
                │                                      │
                │  9. assert_cmd for integration test  │
                │                                      │
                │  10. cargo-dist for distribution     │
                │                                      │
                │  11. Lazy init heavy deps for fast   │
                │      startup                         │
                │                                      │
                │  12. Standard CLI conventions        │
                │      (-h, -V, -v, exit codes, etc.) │
                └──────────────────────────────────────┘
```

---

## 22. Bộ tài liệu giờ có 21 chương!

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   PHẦN I-V (a-s): 19 chương foundation + apps            │
   │                                                          │
   │   PHẦN VI: Specialized domains                           │
   │   ──────────                                             │
   │   t. wasm                       — Browser+Edge+Server    │
   │   u. cli-tools                  — Command-line apps      │
   │      u-cli-tools.md + visual    ← VỪA HOÀN THÀNH         │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 21 chương × 2 files = 42 files                   │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust FULL DOMAIN:                        │
   │                                                          │
   │   🌐 Web (axum)                                          │
   │   🗄️ Database (sqlx)                                     │
   │   🔌 Embedded (no_std)                                   │
   │   🖥️ Desktop (Tauri)                                     │
   │   📱 Mobile (Tauri v2)                                   │
   │   🌍 WASM (browser/edge)                                 │
   │   📟 CLI tools (clap, ratatui)  ← MỚI                    │
   │   🚀 Performance                                         │
   │   🧪 Testing                                             │
   │   🔍 Observability                                       │
   │   ⚙️ Unsafe + FFI                                        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced — pure Rust, no webview)
- **gRPC** (tonic, prost)
- **Cryptography** (rustls, ring, age)
- **OS kernels** (Redox patterns)
- **Networking** (mio, smoltcp)

🦀 Báo nếu muốn tiếp!
