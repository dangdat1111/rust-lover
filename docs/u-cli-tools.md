# CLI Tools trong Rust — Deep Dive

> Tài liệu thứ 21 (chương u) trong bộ Rust nền tảng. Đọc trước:
> - [g-error-handling.md](./g-error-handling.md) — CLI error patterns
> - [l-observability.md](./l-observability.md) — log + tracing
> - [o-testing.md](./o-testing.md) — test CLI tools
>
> CLI (Command Line Interface) tools là **domain xuất sắc** của Rust:
> - **Single binary** — không cần install runtime
> - **Fast startup** (~ms vs ~100ms Python/Ruby)
> - **Cross-platform** — Windows, macOS, Linux từ cùng codebase
> - **Static link** dễ — không cần `LD_LIBRARY_PATH`
> - **Strong typing** — args parsed compile-time validated
>
> Real-world Rust CLI tools nổi tiếng:
> - **ripgrep** (fastest grep)
> - **bat** (cat with syntax highlighting)
> - **fd** (faster find)
> - **exa/eza** (modern ls)
> - **starship** (cross-shell prompt)
> - **bottom** (system monitor)
> - **delta** (git diff viewer)
> - **just** (task runner)
> - **gitui/lazygit-style tools**
> - **cargo** itself!
>
> Tài liệu này dạy bạn xây CLI tool **production-quality**: từ argument parsing đến full TUI app.

---

# Mục lục

- [Tầng 1: Vì sao Rust cho CLI?](#tầng-1-vì-sao-rust-cho-cli)
- [Tầng 2: clap — Argument parsing](#tầng-2-clap--argument-parsing)
- [Tầng 3: clap derive — Type-driven CLI](#tầng-3-clap-derive--type-driven-cli)
- [Tầng 4: clap builder — Programmatic API](#tầng-4-clap-builder--programmatic-api)
- [Tầng 5: Subcommands — Complex CLI structure](#tầng-5-subcommands--complex-cli-structure)
- [Tầng 6: Interactive prompts — dialoguer](#tầng-6-interactive-prompts--dialoguer)
- [Tầng 7: Progress & status — indicatif](#tầng-7-progress--status--indicatif)
- [Tầng 8: Terminal styling — console & anstyle](#tầng-8-terminal-styling--console--anstyle)
- [Tầng 9: ratatui — Full TUI framework](#tầng-9-ratatui--full-tui-framework)
- [Tầng 10: Configuration management](#tầng-10-configuration-management)
- [Tầng 11: Logging in CLI](#tầng-11-logging-in-cli)
- [Tầng 12: Error handling cho CLI](#tầng-12-error-handling-cho-cli)
- [Tầng 13: Testing CLI tools](#tầng-13-testing-cli-tools)
- [Tầng 14: Distribution & installation](#tầng-14-distribution--installation)
- [Tầng 15: Performance & startup time](#tầng-15-performance--startup-time)
- [Tầng 16: Patterns & best practices](#tầng-16-patterns--best-practices)

---

# Tầng 1: Vì sao Rust cho CLI?

## 1.1 Comparison với các language

```
   ┌──────────────────────────────────────────────────────────┐
   │ Language │ Startup │ Distribution    │ Cross-platform   │
   ├──────────────────────────────────────────────────────────┤
   │ Bash      │ Instant │ Script only     │ Limited (Win!)  │
   │ Python    │ 50-200ms│ pip + interp.   │ Yes but pip msy │
   │ Node.js   │ 50-100ms│ npm + Node      │ Yes              │
   │ Go        │ ~10ms   │ Single binary   │ Yes              │
   │ Rust      │ ~5ms    │ Single binary   │ Yes              │
   │ C/C++     │ ~5ms    │ Single binary   │ Yes (build/OS)  │
   └──────────────────────────────────────────────────────────┘
   
   
   Rust advantages cho CLI:
   ────────────────────
   
   ✅ Single static binary — drop into PATH, done
   ✅ Fast startup — no runtime, no JIT warmup
   ✅ Memory safe — no segfault in production
   ✅ Easy cross-compile — Linux/Mac/Windows from one machine
   ✅ Mature ecosystem (clap, indicatif, ratatui, ...)
   ✅ Easy parallelism (rayon) for batch operations
   ✅ Type-safe args parsing
   ✅ Helpful error messages (compiler + runtime)
   
   ⚠️ Trade-offs:
   ❌ Compile time slower than Go
   ❌ Binary larger than C (but smaller than Node bundles)
   ❌ Learning curve for borrow checker
```

## 1.2 Real-world Rust CLIs

```
   ┌────────────┬─────────────────────────────────────────┐
   │ Tool       │ What                                    │
   ├────────────┼─────────────────────────────────────────┤
   │ ripgrep    │ Faster grep alternative                 │
   │ fd          │ Faster find alternative                │
   │ bat         │ cat with syntax highlighting           │
   │ exa/eza     │ Modern ls                               │
   │ starship   │ Cross-shell prompt                      │
   │ delta      │ git diff viewer                          │
   │ bottom     │ Better top/htop                          │
   │ just       │ Task runner (Makefile alternative)      │
   │ jaq        │ jq-like JSON processor                   │
   │ choose     │ Better cut/awk                          │
   │ sd         │ Find/replace (sed alternative)          │
   │ tokei      │ Code statistics                          │
   │ dust       │ Disk usage analyzer                     │
   │ broot      │ Tree-based file explorer                 │
   │ helix      │ Modal editor (post-Vim)                 │
   │ gitui      │ Git TUI                                 │
   │ cargo      │ Rust's own package manager (in Rust)    │
   │ rustc      │ The Rust compiler itself                 │
   └────────────┴─────────────────────────────────────────┘
```

Many "X-but-better" tools. Rust ecosystem keeps innovating.

## 1.3 CLI types

```
   1. SIMPLE COMMAND
      mytool input.txt          → process, output, exit
      
   2. SUBCOMMAND ROUTER (like git)
      mytool init               → init mode
      mytool add file           → add mode
      mytool commit -m "..."    → commit mode
      
   3. INTERACTIVE PROMPT
      mytool                    → asks questions, gathers input
      
   4. FULL TUI (Terminal UI)
      mytool                    → full-screen interactive app
                                  arrow keys, panels, etc.
   
   5. DAEMON / SERVICE
      mytool start              → background process
      mytool status             → query state
```

Different libraries for each. Tài liệu cover all 5.

---

# Tầng 2: clap — Argument parsing

## 2.1 clap = standard CLI parser

`clap` (Command Line Argument Parser) = de-facto standard cho Rust CLI.

Two APIs:
- **derive** (preferred) — type-driven, declarative
- **builder** — programmatic, runtime construction

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

## 2.2 Minimal example

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "greet", version, about = "Greet someone")]
struct Cli {
    /// Name to greet
    name: String,
    
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u32,
}

fn main() {
    let cli = Cli::parse();
    
    for _ in 0..cli.count {
        println!("Hello, {}!", cli.name);
    }
}
```

Usage:
```bash
$ greet World
Hello, World!

$ greet --count 3 Alice
Hello, Alice!
Hello, Alice!
Hello, Alice!

$ greet --help
Greet someone

Usage: greet [OPTIONS] <NAME>

Arguments:
  <NAME>  Name to greet

Options:
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -h, --help           Print help
  -V, --version        Print version

$ greet --version
greet 0.1.0
```

clap auto-generates:
- Help message (from doc comments)
- Version flag
- Validation (e.g., u32 parsing)
- Error messages for invalid input

## 2.3 clap features

```
   ┌──────────────────────────────────────────────────────────┐
   │  clap built-in features:                                 │
   │                                                          │
   │  ✅ Auto help (--help / -h)                              │
   │  ✅ Auto version (--version / -V)                        │
   │  ✅ Suggest similar commands ("Did you mean: ...?")      │
   │  ✅ Subcommands                                          │
   │  ✅ Type-safe argument parsing                           │
   │  ✅ Required vs optional vs default values               │
   │  ✅ Multiple values (Vec<T>)                              │
   │  ✅ Custom validators                                    │
   │  ✅ Environment variable fallback                        │
   │  ✅ Shell completions (bash, zsh, fish, PowerShell)      │
   │  ✅ Color output                                         │
   │  ✅ Internationalization                                 │
   └──────────────────────────────────────────────────────────┘
```

---

# Tầng 3: clap derive — Type-driven CLI

## 3.1 Arg attributes

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Input file path
    #[arg(short, long)]
    input: PathBuf,
    
    /// Output file path (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
    
    /// Force overwrite
    #[arg(short, long)]
    force: bool,
    
    /// Files to process (positional)
    files: Vec<PathBuf>,
    
    /// Number of threads
    #[arg(short = 'j', long, default_value_t = num_cpus::get())]
    threads: usize,
    
    /// Format (JSON or YAML)
    #[arg(long, value_enum, default_value_t = Format::Json)]
    format: Format,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Format {
    Json,
    Yaml,
    Toml,
}
```

Attributes:
- `short` / `long` — flag aliases (`-i` / `--input`)
- `default_value_t` — type-safe default
- `action = Count` — count occurrences (`-vvv` = 3)
- `value_enum` — restrict to enum variants
- `Option<T>` — optional arg (not required)
- `Vec<T>` — variadic / multiple

## 3.2 Required vs optional

```rust
struct Cli {
    // Required positional:
    file: String,
    
    // Optional positional:
    file: Option<String>,
    
    // Required flag:
    #[arg(short, long)]
    output: String,
    
    // Optional flag (no default):
    #[arg(short, long)]
    output: Option<String>,
    
    // Flag with default:
    #[arg(short, long, default_value = "out.txt")]
    output: String,
    
    // Boolean flag (always optional):
    #[arg(short, long)]
    verbose: bool,
}
```

## 3.3 Multiple values

```rust
// -i 1 -i 2 -i 3
#[arg(short, long)]
includes: Vec<String>,

// -i 1,2,3
#[arg(short, long, value_delimiter = ',')]
includes: Vec<String>,

// Take 2-5 values: -i 1 2 3
#[arg(short, long, num_args = 2..=5)]
includes: Vec<String>,
```

## 3.4 Custom validation

```rust
#[arg(short, long, value_parser = port_validator)]
port: u16,

fn port_validator(s: &str) -> Result<u16, String> {
    let port: u16 = s.parse().map_err(|_| "not a valid port".to_string())?;
    if port < 1024 {
        return Err("port must be >= 1024".to_string());
    }
    Ok(port)
}
```

Or use built-in range validator:
```rust
#[arg(short, long, value_parser = clap::value_parser!(u16).range(1024..))]
port: u16,
```

## 3.5 Environment variable fallback

```rust
#[arg(long, env = "API_KEY", hide_env_values = true)]
api_key: String,
```

CLI arg > env var > default.

```bash
$ API_KEY=secret mytool        # from env
$ mytool --api-key=secret      # from arg
```

`hide_env_values = true` — don't show secret in help text.

## 3.6 Mutually exclusive / required groups

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, group = "format")]
    json: bool,
    
    #[arg(long, group = "format")]
    yaml: bool,
    
    #[arg(long, group = "format")]
    toml: bool,
}
// User can only pass ONE of --json, --yaml, --toml
```

```rust
#[arg(long, requires = "output")]
overwrite: bool,

#[arg(short, long)]
output: Option<String>,
// --overwrite requires --output
```

## 3.7 Hidden args

```rust
#[arg(long, hide = true)]
debug_internal: bool,
// Available but not shown in --help
```

For internal flags, deprecated args.

## 3.8 Custom help text

```rust
#[derive(Parser)]
#[command(
    name = "mytool",
    version,
    author = "Me <me@example.com>",
    about = "Short description",
    long_about = "Long description\nwith multiple lines",
    after_help = "EXAMPLES:\n  mytool foo\n  mytool --flag bar",
)]
struct Cli {
    /// This shows in --help
    #[arg(short, long, long_help = "More detailed help shown with --help (vs -h)")]
    flag: bool,
}
```

`-h` short help, `--help` long help.

---

# Tầng 4: clap builder — Programmatic API

For dynamic CLI construction (e.g., plugins):

```rust
use clap::{Arg, ArgAction, Command};

fn main() {
    let cmd = Command::new("mytool")
        .version("1.0")
        .author("Me")
        .about("Does stuff")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input file")
                .required(true)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help("Verbose output")
        );
    
    let matches = cmd.get_matches();
    
    let input: &String = matches.get_one("input").unwrap();
    let verbose: u8 = matches.get_count("verbose");
    
    println!("input: {}, verbose: {}", input, verbose);
}
```

Useful when:
- Args depend on config file
- Plugin system adds dynamic args
- Generating CLI from schema

derive API is preferred for static CLIs (most cases).

---

# Tầng 5: Subcommands — Complex CLI structure

## 5.1 Git-like subcommands

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mytool", version, about)]
struct Cli {
    /// Global flag
    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        name: String,
        
        /// Template to use
        #[arg(short, long)]
        template: Option<String>,
    },
    
    /// Add a file
    Add {
        /// Files to add
        files: Vec<PathBuf>,
        
        /// Add recursively
        #[arg(short, long)]
        recursive: bool,
    },
    
    /// Show status
    Status,
    
    /// Configuration commands
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    Get { key: String },
    Set { key: String, value: String },
    List,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, template } => {
            println!("Init {} with template {:?}", name, template);
        }
        Commands::Add { files, recursive } => {
            println!("Add {:?} (recursive: {})", files, recursive);
        }
        Commands::Status => {
            println!("Status");
        }
        Commands::Config { action } => {
            match action {
                ConfigCommands::Get { key } => println!("Get {}", key),
                ConfigCommands::Set { key, value } => println!("Set {}={}", key, value),
                ConfigCommands::List => println!("List config"),
            }
        }
    }
}
```

Usage:
```bash
mytool init myproject --template rust
mytool add file1.txt file2.txt --recursive
mytool status
mytool config get database.url
mytool config set database.url postgres://...
```

`global = true` — flag available on all subcommands.

## 5.2 Subcommand aliases

```rust
#[derive(Subcommand)]
enum Commands {
    #[command(alias = "rm", alias = "delete")]
    Remove { file: String },
}
```

`mytool remove foo` = `mytool rm foo` = `mytool delete foo`

## 5.3 Subcommand hierarchy

```
   mytool
   ├── init <name>
   ├── add <files>
   ├── status
   ├── config
   │   ├── get <key>
   │   ├── set <key> <value>
   │   └── list
   └── user
       ├── create <name>
       ├── delete <id>
       └── list
```

Type per command level. Clean structure.

## 5.4 Dispatch pattern

```rust
fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose)?;
    
    match cli.command {
        Commands::Init { name, template } => commands::init(name, template),
        Commands::Add { files, recursive } => commands::add(files, recursive),
        Commands::Status => commands::status(),
        Commands::Config { action } => match action {
            ConfigCommands::Get { key } => commands::config_get(&key),
            ConfigCommands::Set { key, value } => commands::config_set(&key, &value),
            ConfigCommands::List => commands::config_list(),
        },
    }
}
```

Each command → separate function in `commands/` module.

---

# Tầng 6: Interactive prompts — dialoguer

## 6.1 When to use prompts?

For CLI apps that need **user input interactively**:
- Setup wizards (`mytool init`)
- Confirmations ("Delete file?")
- Multi-step workflows
- Tools without script-friendly args

```toml
[dependencies]
dialoguer = { version = "0.11", features = ["fuzzy-select"] }
```

## 6.2 Simple prompts

```rust
use dialoguer::{Input, Confirm, Select, MultiSelect, Password};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Text input:
    let name: String = Input::new()
        .with_prompt("What's your name?")
        .interact_text()?;
    
    // With validation:
    let email: String = Input::new()
        .with_prompt("Email")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.contains('@') { Ok(()) } else { Err("Invalid email") }
        })
        .interact_text()?;
    
    // Number:
    let age: u32 = Input::new()
        .with_prompt("Age")
        .interact_text()?;
    
    // Default value:
    let port: u16 = Input::new()
        .with_prompt("Port")
        .default(8080)
        .interact_text()?;
    
    // Password (hidden):
    let password = Password::new()
        .with_prompt("Password")
        .with_confirmation("Confirm", "Passwords don't match")
        .interact()?;
    
    Ok(())
}
```

## 6.3 Confirmation

```rust
let confirmed = Confirm::new()
    .with_prompt("Delete all files?")
    .default(false)
    .interact()?;

if confirmed {
    println!("Deleting...");
} else {
    println!("Cancelled.");
}
```

## 6.4 Select from list

```rust
let options = &["Rust", "Go", "Python", "Ruby"];

let selection = Select::new()
    .with_prompt("Choose language")
    .items(options)
    .default(0)
    .interact()?;

println!("You chose: {}", options[selection]);
```

```
Choose language › Rust
  Rust
❯ Go
  Python
  Ruby
```

Arrow keys navigate, Enter confirms.

## 6.5 Multi-select

```rust
let features = &["Async", "Database", "HTTP", "TLS"];

let selections = MultiSelect::new()
    .with_prompt("Select features")
    .items(features)
    .interact()?;

for &i in &selections {
    println!("Selected: {}", features[i]);
}
```

Space to toggle, Enter to confirm.

## 6.6 Fuzzy select

```rust
use dialoguer::FuzzySelect;

let countries = vec!["Vietnam", "Thailand", "Japan", "Korea", "China"];

let selection = FuzzySelect::new()
    .with_prompt("Country")
    .items(&countries)
    .interact()?;
```

Type to filter. Like fzf.

## 6.7 Editor

```rust
use dialoguer::Editor;

if let Some(content) = Editor::new().edit("Initial text")? {
    println!("Edited: {}", content);
} else {
    println!("Cancelled");
}
```

Opens `$EDITOR` (vim, nano, ...) for multiline input.

## 6.8 Themed prompts

```rust
use dialoguer::theme::ColorfulTheme;

let name: String = Input::with_theme(&ColorfulTheme::default())
    .with_prompt("Name")
    .interact_text()?;
```

Better visuals. Various themes available.

## 6.9 Interactive vs script mode

```rust
fn ask_or_use<T>(arg: Option<T>, default_fn: impl FnOnce() -> Result<T>) -> Result<T> {
    if let Some(v) = arg {
        Ok(v)
    } else if atty::is(atty::Stream::Stdin) {
        // Interactive — ask user
        default_fn()
    } else {
        // Piped/script — error
        Err(anyhow::anyhow!("missing arg, not interactive"))
    }
}

let name = ask_or_use(args.name, || {
    Input::new().with_prompt("Name").interact_text()
        .map_err(Into::into)
})?;
```

CLI also script-friendly. Don't force prompts in pipes.

---

# Tầng 7: Progress & status — indicatif

## 7.1 indicatif

```toml
[dependencies]
indicatif = "0.17"
```

For long-running operations: progress bars, spinners, multi-progress.

## 7.2 Basic progress bar

```rust
use indicatif::ProgressBar;
use std::time::Duration;

fn main() {
    let total = 100;
    let pb = ProgressBar::new(total);
    
    for _ in 0..total {
        std::thread::sleep(Duration::from_millis(50));
        pb.inc(1);
    }
    
    pb.finish_with_message("done!");
}
```

```
████████████████████████████████████████ 100/100 done!
```

## 7.3 Customized style

```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(total);
pb.set_style(
    ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})"
    )
    .unwrap()
    .progress_chars("█▉▊▋▌▍▎▏  ")
);
```

Template variables:
- `{bar:40}` — 40-char wide bar
- `{pos}/{len}` — current/total
- `{percent}` — percentage
- `{elapsed_precise}` — elapsed time
- `{eta}` — ETA
- `{msg}` — current message
- `{spinner}` — spinning indicator

## 7.4 Spinner (indeterminate)

```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new_spinner();
pb.set_style(
    ProgressStyle::with_template("{spinner:.blue} {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
);
pb.enable_steady_tick(Duration::from_millis(100));

pb.set_message("Connecting to server...");
do_work();
pb.set_message("Downloading...");
do_more_work();
pb.finish_with_message("Done!");
```

Useful when total work unknown.

## 7.5 Multi-progress (parallel bars)

```rust
use indicatif::{MultiProgress, ProgressBar};
use std::thread;

let multi = MultiProgress::new();

let mut handles = vec![];
for i in 0..5 {
    let pb = multi.add(ProgressBar::new(100));
    pb.set_style(ProgressStyle::with_template("[{prefix}] {bar:40} {pos}/{len}").unwrap());
    pb.set_prefix(format!("Worker {}", i));
    
    handles.push(thread::spawn(move || {
        for _ in 0..100 {
            thread::sleep(Duration::from_millis(50));
            pb.inc(1);
        }
        pb.finish();
    }));
}

for h in handles { h.join().unwrap(); }
multi.clear().unwrap();
```

```
[Worker 0] ████████████████████ 100/100
[Worker 1] █████████░░░░░░░░░░░ 45/100
[Worker 2] ███████████░░░░░░░░░ 55/100
[Worker 3] ███████░░░░░░░░░░░░░ 35/100
[Worker 4] █████████████░░░░░░░ 65/100
```

Concurrent download/processing apps.

## 7.6 With iterators

```rust
use indicatif::ProgressIterator;

let v: Vec<i32> = (0..100).collect();

for i in v.iter().progress() {
    process(*i);
}
```

Magic — wrap iterator with progress bar.

```rust
// With custom bar:
let pb = ProgressBar::new(v.len() as u64);
for i in v.iter().progress_with(pb.clone()) {
    process(*i);
}
```

## 7.7 Rayon integration

```rust
use rayon::prelude::*;
use indicatif::ParallelProgressIterator;

let result: Vec<_> = items
    .par_iter()
    .progress_count(items.len() as u64)
    .map(|item| process(item))
    .collect();
```

Parallel + progress in one chain.

## 7.8 println! while progress bar active

```rust
// ❌ This breaks progress bar display:
pb.inc(1);
println!("Processing item {}", i);

// ✅ Use pb.println():
pb.inc(1);
pb.println(format!("Processing item {}", i));
```

`pb.println()` correctly handles cursor.

---

# Tầng 8: Terminal styling — console & anstyle

## 8.1 Color & style

```toml
[dependencies]
console = "0.15"
anstyle = "1"
```

```rust
use console::style;

fn main() {
    println!("{}", style("Error!").red().bold());
    println!("{}", style("Warning").yellow());
    println!("{}", style("Success").green());
    println!("{}", style("Info").blue().italic());
    println!("{}", style("Underlined").underlined());
}
```

```
Error!     ← red + bold
Warning    ← yellow
Success    ← green
Info       ← blue + italic
Underlined ← underlined
```

## 8.2 Terminal detection

```rust
use console::Term;

let term = Term::stdout();

if term.features().colors_supported() {
    println!("Has colors");
}

if term.is_term() {
    println!("Is a real terminal");
} else {
    println!("Piped or redirected");
}

let (rows, cols) = term.size();
println!("Terminal: {} x {}", cols, rows);
```

Detect:
- Color support
- TTY (vs pipe)
- Window size
- Unicode support

## 8.3 Don't color when piped

```rust
use atty::Stream;

fn main() {
    let use_color = atty::is(Stream::Stdout);
    
    let msg = if use_color {
        style("Hello").red().to_string()
    } else {
        "Hello".to_string()
    };
    println!("{}", msg);
}
```

Or use `console::Term::stdout().is_term()`.

ANSI escapes in pipes → noise. `mytool | grep "ERROR"` gets confused.

## 8.4 NO_COLOR env var

Respect `NO_COLOR=1` env var (standard):
```rust
use std::env;

let use_color = env::var_os("NO_COLOR").is_none() && atty::is(Stream::Stdout);
```

Many users set this to disable colors globally.

## 8.5 Modern: anstyle

```rust
use anstyle::{AnsiColor, Color, Style};

let red = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold();
let reset = Style::new();

println!("{}Error!{}", red.render(), reset.render());
```

Lighter weight. Used by clap internally.

## 8.6 Cursor manipulation

```rust
use console::Term;

let term = Term::stdout();

term.clear_line()?;
term.move_cursor_up(2)?;
term.write_str("Updated line")?;
term.flush()?;
```

For dynamic UI (replacing previous output).

## 8.7 Reading keys (raw mode)

```rust
use console::{Term, Key};

let term = Term::stdout();

println!("Press any key...");
loop {
    match term.read_key()? {
        Key::Char('q') => break,
        Key::ArrowUp => println!("Up"),
        Key::ArrowDown => println!("Down"),
        Key::Enter => println!("Enter"),
        Key::Escape => break,
        _ => {}
    }
}
```

For simple interactive tools. For complex UIs → ratatui (Tầng 9).

---

# Tầng 9: ratatui — Full TUI framework

## 9.1 Beyond simple prompts

For complex terminal UIs (Vim, htop, k9s style):
- Multiple panels
- Mouse + keyboard
- Live updates
- Custom widgets

→ Use **ratatui** (formerly tui-rs).

## 9.2 Setup

```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
```

`crossterm` = cross-platform terminal backend (Linux/Mac/Windows).

## 9.3 Hello ratatui

```rust
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Terminal,
};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let result = run(&mut terminal);
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    
    result
}

fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.area());
            
            let title = Paragraph::new("My TUI App (Press q to quit)")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);
            
            let items = vec![
                ListItem::new("Item 1"),
                ListItem::new("Item 2"),
                ListItem::new("Item 3"),
            ];
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Items"));
            f.render_widget(list, chunks[1]);
        })?;
        
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
        }
    }
    Ok(())
}
```

Hello-world TUI app. Full-screen, modal.

## 9.4 ratatui architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │ ratatui = immediate-mode terminal UI                     │
   │                                                          │
   │  Loop:                                                   │
   │   1. Read events (keys, mouse, resize)                   │
   │   2. Update state                                        │
   │   3. Re-render entire screen                             │
   │   4. Diff with previous frame, send only deltas         │
   │   5. Repeat                                              │
   │                                                          │
   │ Frame rate: ~30-60 FPS typical                          │
   │ Backend: crossterm (cross-platform)                      │
   │           or termion (Unix only)                         │
   └──────────────────────────────────────────────────────────┘
```

Like web games (immediate mode), redraw each frame.

## 9.5 Widgets

Built-in:
```rust
use ratatui::widgets::*;

Paragraph::new("text")           // text block
List::new(items)                  // selectable list
Table::new(rows, widths)          // table
Tabs::new(titles)                 // tabs at top
Gauge::default()                  // progress gauge
LineGauge::default()              // line gauge
BarChart::default()               // bar chart
Sparkline::default()              // sparkline (mini chart)
Block::default()                  // bordered container
Chart::default()                  // 2D chart
Canvas::default()                 // pixel-level drawing
Clear                              // clear widget
```

Combine into apps:
```rust
let table = Table::new(rows, [Constraint::Length(20), Constraint::Length(30)])
    .header(Row::new(vec!["Name", "Email"]))
    .block(Block::default().borders(Borders::ALL).title("Users"));

f.render_widget(table, area);
```

## 9.6 Layout

```rust
let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(30),    // 30% width
        Constraint::Min(0),             // remaining
    ])
    .split(area);

// chunks[0] = left panel (30%)
// chunks[1] = right panel (70%)
```

Nested layouts:
```rust
let main_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(f.area());

// main_chunks[0] = top bar
// main_chunks[1] = content area

let content_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(30), Constraint::Min(0)])
    .split(main_chunks[1]);

// content_chunks[0] = left panel
// content_chunks[1] = right panel
```

Like CSS Flexbox for terminal.

## 9.7 State management

```rust
struct App {
    items: Vec<String>,
    selected: usize,
    input: String,
}

impl App {
    fn new() -> Self {
        Self {
            items: vec!["a".into(), "b".into(), "c".into()],
            selected: 0,
            input: String::new(),
        }
    }
    
    fn next(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }
    
    fn previous(&mut self) {
        if self.selected == 0 {
            self.selected = self.items.len() - 1;
        } else {
            self.selected -= 1;
        }
    }
}

fn run(...) -> Result<()> {
    let mut app = App::new();
    
    loop {
        terminal.draw(|f| ui(f, &app))?;
        
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
    Ok(())
}
```

Separate **state** + **UI render** + **input handling**.

## 9.8 Stateful widgets

```rust
let mut state = ListState::default();
state.select(Some(app.selected));

let items: Vec<ListItem> = app.items.iter()
    .map(|i| ListItem::new(i.as_str()))
    .collect();

let list = List::new(items)
    .block(Block::default().borders(Borders::ALL).title("Items"))
    .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
    .highlight_symbol("▶ ");

f.render_stateful_widget(list, area, &mut state);
```

Lists, tables — track selection via state object.

## 9.9 Async events (tokio + ratatui)

```rust
use tokio::time::{interval, Duration};
use crossterm::event::EventStream;
use futures::StreamExt;

async fn run<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut events = EventStream::new();
    let mut tick = interval(Duration::from_millis(100));
    
    loop {
        terminal.draw(|f| ui(f, ...))?;
        
        tokio::select! {
            _ = tick.tick() => {
                // background update (e.g., fetch new data)
            }
            Some(Ok(event)) = events.next() => {
                if let Event::Key(key) = event {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
```

Real-time TUI with background updates (e.g., monitoring tools).

## 9.10 Real-world TUI examples in Rust

- **gitui**: Git TUI
- **lazygit**-like in Rust
- **helix**: modal editor
- **bottom**: system monitor
- **dust**: disk usage analyzer
- **broot**: file explorer
- **zellij**: terminal multiplexer
- **k9s**-like tools for Kubernetes
- **monitor**: log/metric monitoring TUIs

Many production-quality. ratatui matures fast.

---

# Tầng 10: Configuration management

## 10.1 Config sources

CLI tool config from:
1. **CLI args** (highest priority)
2. **Env vars**
3. **Local config file** (./mytool.toml)
4. **User config file** (~/.config/mytool/config.toml)
5. **System config** (/etc/mytool/config.toml)
6. **Defaults** (lowest)

## 10.2 figment crate

```toml
[dependencies]
figment = { version = "0.10", features = ["env", "toml"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    server: String,
    port: u16,
    timeout: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: "localhost".into(),
            port: 8080,
            timeout: 30,
        }
    }
}

fn load_config() -> Result<Config, figment::Error> {
    Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file("/etc/mytool/config.toml"))
        .merge(Toml::file(dirs::config_dir().unwrap().join("mytool/config.toml")))
        .merge(Toml::file("./mytool.toml"))
        .merge(Env::prefixed("MYTOOL_"))
        .extract()
}
```

Layered config. Each layer overrides earlier.

## 10.3 dirs crate — Platform paths

```rust
use dirs;

dirs::home_dir()         // ~/
dirs::config_dir()       // ~/.config (Linux) | Library/Application Support (Mac)
dirs::cache_dir()        // ~/.cache
dirs::data_dir()         // ~/.local/share
dirs::download_dir()     // ~/Downloads
```

Cross-platform paths. Don't hardcode `~/.foo`.

## 10.4 Pattern: Init config on first run

```rust
fn ensure_config() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().unwrap().join("mytool");
    std::fs::create_dir_all(&config_dir)?;
    
    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        let default = Config::default();
        let toml = toml::to_string_pretty(&default)?;
        std::fs::write(&config_path, toml)?;
        println!("Created default config at {}", config_path.display());
    }
    
    Ok(config_path)
}
```

First run → create default. Then user edits.

## 10.5 Combine clap + config

```rust
#[derive(Parser)]
struct Cli {
    /// Config file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Override server
    #[arg(long)]
    server: Option<String>,
    
    /// Override port
    #[arg(long)]
    port: Option<u16>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_config(cli.config.as_deref())?;
    
    // CLI args override config:
    if let Some(s) = cli.server { config.server = s; }
    if let Some(p) = cli.port { config.port = p; }
    
    run(config)?;
    Ok(())
}
```

Standard layering. CLI > env > files > defaults.

---

# Tầng 11: Logging in CLI

## 11.1 Logging strategy

```
   ┌──────────────────────────────────────────────────────────┐
   │ CLI logging triple:                                      │
   │                                                          │
   │  • stdout: program OUTPUT (data, results)                │
   │  • stderr: LOGS / status (info, warn, error)             │
   │  • Log file: detailed debug (optional)                   │
   │                                                          │
   │ Rationale:                                               │
   │  • Allows piping: mytool | other-tool                    │
   │  • Stderr won't break pipes                              │
   │  • Log file for post-mortem                              │
   └──────────────────────────────────────────────────────────┘
```

```rust
// Output (always to stdout):
println!("{}", result);

// Status (to stderr):
eprintln!("Processing {} items...", count);
```

## 11.2 tracing for CLI

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

```rust
use tracing_subscriber::fmt::format::FmtSpan;

fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(format!("mytool={}", level))
        .with_writer(std::io::stderr)        // logs to STDERR not STDOUT
        .with_span_events(FmtSpan::CLOSE)
        .init();
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose);
    
    tracing::info!("Starting");
    tracing::debug!(?cli, "parsed cli");
    
    do_work()?;
    
    tracing::info!("Done");
    Ok(())
}
```

`-v` `-vv` `-vvv` controls verbosity. Stderr for logs (preserve stdout for output).

## 11.3 Pretty output vs JSON

```rust
fn init_logging(verbose: u8, json: bool) {
    let level = ...;
    
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(format!("mytool={}", level))
        .with_writer(std::io::stderr);
    
    if json {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}
```

```bash
# Human:
mytool -v
# INFO mytool: Processing
# WARN mytool: Skipping invalid item

# Machine:
mytool -v --log-format=json
# {"level":"INFO","msg":"Processing"}
# {"level":"WARN","msg":"Skipping invalid item"}
```

## 11.4 Log to file too

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

let stderr_layer = tracing_subscriber::fmt::layer()
    .with_writer(std::io::stderr)
    .with_filter(env_filter);

let log_file = std::fs::File::create(log_path)?;
let file_layer = tracing_subscriber::fmt::layer()
    .with_writer(log_file)
    .with_ansi(false)   // no colors in file
    .json();

tracing_subscriber::registry()
    .with(stderr_layer)
    .with(file_layer)
    .init();
```

Concise on stderr, detailed JSON in file for debugging.

---

# Tầng 12: Error handling cho CLI

## 12.1 Exit codes

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    
    match run(cli) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            ExitCode::from(1)
        }
    }
}
```

Standard exit codes:
- `0` — success
- `1` — generic error
- `2` — usage error (CLI parse fail)
- `64-78` — Unix sysexits.h (specific errors)
- `126` — command not executable
- `127` — command not found
- `130` — SIGINT (Ctrl+C)

## 12.2 Anyhow for application errors

```toml
[dependencies]
anyhow = "1"
```

```rust
use anyhow::{Context, Result};

fn run() -> Result<()> {
    let config = load_config()
        .context("loading config")?;
    
    let data = process_files(&config.files)
        .context("processing files")?;
    
    save_results(&data)
        .context("saving results")?;
    
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);   // {:#} shows full chain
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
```

Output:
```
Error: processing files: file not found: /etc/missing.txt
```

Context chain helps users diagnose.

## 12.3 User-friendly error messages

```rust
use anyhow::{Context, Result};

fn open_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| {
            format!(
                "Could not read config from '{}'. \
                 Try `mytool config init` to create one.",
                path.display()
            )
        })?;
    
    toml::from_str(&content)
        .with_context(|| {
            format!(
                "Config file '{}' has invalid TOML syntax. \
                 Check for unmatched brackets or quotes.",
                path.display()
            )
        })
}
```

Explain to user what to do.

## 12.4 miette for fancy errors

```toml
[dependencies]
miette = { version = "7", features = ["fancy"] }
thiserror = "1"
```

```rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("invalid config")]
#[diagnostic(
    code(mytool::config_error),
    help("check the syntax around the highlighted area"),
)]
struct ConfigError {
    #[source_code]
    src: NamedSource,
    
    #[label("here")]
    bad_bit: SourceSpan,
}

fn main() -> miette::Result<()> {
    let result = parse_config("config.toml");
    if let Err(e) = result {
        return Err(e.into());
    }
    Ok(())
}
```

Output:
```
Error: mytool::config_error

  × invalid config
   ╭─[config.toml:3:5]
 2 │ port = 8080
 3 │ timeout = "thirty"
   ·          ─────────
   ·                ╰── here
 4 │ server = "..."
   ╰────
  help: check the syntax around the highlighted area
```

Like rustc error messages. Great for parser-heavy CLIs.

## 12.5 Avoid stack traces in production CLI

```bash
$ mytool bad_input
Error: thread 'main' panicked at src/main.rs:42
note: run with `RUST_BACKTRACE=1` ...
```

❌ Confusing for users. Use proper error handling. Reserve panics for bugs.

```rust
fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
```

✅ Clean message.

---

# Tầng 13: Testing CLI tools

## 13.1 Unit test core logic

```rust
// Separate logic from CLI
pub fn process_count(input: &str) -> Result<usize> {
    Ok(input.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn empty() {
        assert_eq!(process_count("").unwrap(), 0);
    }
    
    #[test]
    fn lines() {
        assert_eq!(process_count("a\nb\nc").unwrap(), 3);
    }
}
```

## 13.2 Integration test with assert_cmd

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

```rust
// tests/cli.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("mytool").unwrap();
    cmd.arg("--help").assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_process_file() {
    let mut cmd = Command::cargo_bin("mytool").unwrap();
    cmd.arg("count").arg("tests/fixtures/data.txt")
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn test_missing_file() {
    let mut cmd = Command::cargo_bin("mytool").unwrap();
    cmd.arg("count").arg("doesnt-exist.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
```

`assert_cmd` builds your binary, runs as subprocess, checks output + exit code.

## 13.3 Temp directories

```toml
[dev-dependencies]
assert_fs = "1"
```

```rust
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn test_write_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    let output = temp.child("output.txt");
    
    Command::cargo_bin("mytool").unwrap()
        .arg("process")
        .arg("--output").arg(output.path())
        .arg("input.txt")
        .assert()
        .success();
    
    output.assert(predicate::path::exists())
          .assert(predicate::str::contains("expected content"));
}
// temp dir auto-cleaned
```

## 13.4 TUI testing

ratatui has test utilities:
```rust
use ratatui::backend::TestBackend;

#[test]
fn test_render() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal.draw(|f| ui(f, &State::default())).unwrap();
    
    let buffer = terminal.backend().buffer();
    // Inspect rendered chars
    let line = buffer.content().chunks(80).next().unwrap();
    let text: String = line.iter().map(|c| c.symbol()).collect();
    assert!(text.contains("My App"));
}
```

Render TUI to in-memory buffer, assert content.

## 13.5 Snapshot tests for output

```rust
use insta::assert_snapshot;

#[test]
fn test_format_output() {
    let output = format_table(&data);
    assert_snapshot!(output);
}
```

Capture CLI output → compare to saved snapshot. Easy to spot output regressions.

---

# Tầng 14: Distribution & installation

## 14.1 cargo install

```bash
cargo install mytool
```

Installs from crates.io into `~/.cargo/bin/`.

Publish:
```bash
cargo login
cargo publish
```

Convenient but Rust users only.

## 14.2 Pre-built binaries (cargo-dist)

`cargo-dist` builds binaries for many targets, attaches to GitHub Release.

```bash
cargo install cargo-dist
cargo dist init
git tag v0.1.0 && git push --tags
# Triggers GH Action that builds + uploads
```

Produces:
- `mytool-x86_64-unknown-linux-gnu.tar.xz`
- `mytool-x86_64-apple-darwin.tar.xz`
- `mytool-aarch64-apple-darwin.tar.xz`
- `mytool-x86_64-pc-windows-msvc.zip`
- Shell installer scripts (`curl ... | sh`)

Users:
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/you/mytool/releases/latest/download/mytool-installer.sh | sh
```

## 14.3 Homebrew (macOS/Linux)

```ruby
# homebrew-tap/Formula/mytool.rb
class Mytool < Formula
  desc "My CLI tool"
  homepage "https://github.com/you/mytool"
  url "https://github.com/you/mytool/releases/download/v1.0/mytool-x86_64-apple-darwin.tar.xz"
  sha256 "..."
  
  def install
    bin.install "mytool"
  end
end
```

Users: `brew install you/tap/mytool`.

cargo-dist can auto-generate this.

## 14.4 Scoop (Windows)

JSON manifest similar to Homebrew. Auto-generated by cargo-dist.

## 14.5 npm wrapper

```json
{
  "name": "mytool",
  "version": "1.0.0",
  "bin": {
    "mytool": "./mytool"
  },
  "scripts": {
    "postinstall": "node install.js"
  }
}
```

`install.js` downloads appropriate Rust binary for platform.

`npm install -g mytool` — works for JS folks.

## 14.6 Docker image

```dockerfile
FROM debian:bookworm-slim
COPY mytool /usr/local/bin/
ENTRYPOINT ["mytool"]
```

```bash
docker run -v $(pwd):/data myorg/mytool process /data/input.txt
```

Useful for CI / sandbox.

## 14.7 Linux packages (deb, rpm)

`cargo deb`:
```bash
cargo install cargo-deb
cargo deb
# Creates target/debian/mytool_0.1.0_amd64.deb
```

```bash
cargo install cargo-generate-rpm
cargo generate-rpm
```

## 14.8 Distribution matrix

```
   ┌──────────────┬──────────────────────────────────────┐
   │ Audience     │ Use                                  │
   ├──────────────┼──────────────────────────────────────┤
   │ Rust devs    │ cargo install                        │
   │ Mac users    │ Homebrew                              │
   │ Linux users  │ apt/dnf packages + curl|sh installer │
   │ Windows users│ Scoop + winget + .msi               │
   │ JS devs      │ npm wrapper                          │
   │ DevOps       │ Docker image                         │
   │ Everyone     │ GitHub Releases pre-built            │
   └──────────────┴──────────────────────────────────────┘
```

Use cargo-dist for best multi-platform automation.

---

# Tầng 15: Performance & startup time

## 15.1 Why startup time matters

```
   Tool                Cold start
   ─────                ──────────
   bash (alias)         ~0ms
   Rust CLI             ~5-20ms
   Go CLI               ~10-30ms
   Node.js CLI          ~100-300ms
   Python CLI           ~50-200ms
   Java CLI             ~500-2000ms
   
   For frequent invocations (in shell prompt, file watchers):
   Every ms counts.
```

starship (cross-shell prompt) — runs on every prompt → must be fast.

## 15.2 Measure startup

```bash
hyperfine --warmup 3 --runs 100 'mytool --version'

# Output:
# Time (mean ± σ):       5.2 ms ±  0.5 ms
# Range (min … max):     4.8 ms …   7.0 ms
```

## 15.3 Optimize binary size

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Smaller binary → faster load + execute.

## 15.4 Avoid heavy deps for fast paths

`serde`, `tokio`, `reqwest` — expensive to init. Lazy-load:

```rust
// Don't init runtime if not needed:
fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Version => {
            println!("v1.0");   // no runtime needed
            return Ok(());
        }
        Commands::Server => {
            // NOW init tokio
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(server::run())
        }
        _ => sync_command(),
    }
}
```

Don't init runtime for `--version`!

## 15.5 Lazy static init

```rust
use std::sync::OnceLock;

static REGEX: OnceLock<Regex> = OnceLock::new();

fn get_regex() -> &'static Regex {
    REGEX.get_or_init(|| Regex::new(r"...").unwrap())
}
```

Compile regex once per process. Use only when called.

## 15.6 Avoid clap full features

```toml
clap = { version = "4", features = ["derive"], default-features = false }
```

Disable unused: `cargo`-style help, suggestions, etc. Saves binary size + startup.

## 15.7 Reduce reading

```rust
// ❌ Reads entire file into memory
let content = std::fs::read_to_string(path)?;
for line in content.lines() { ... }

// ✅ Streaming read
let file = std::fs::File::open(path)?;
let reader = std::io::BufReader::new(file);
for line in reader.lines() {
    let line = line?;
    process(&line);
}
```

For huge files: streaming. Saves memory + better cache use.

## 15.8 Parallelism for batch

```rust
use rayon::prelude::*;

let results: Vec<_> = files
    .par_iter()
    .map(|f| process_file(f))
    .collect();
```

Multi-core for CPU-bound. 4-8x speedup typical.

## 15.9 Reduce stdout flushing

```rust
// ❌ Flush each println
for i in 0..1000 {
    println!("{}", i);   // implicit flush per line
}

// ✅ Buffered
use std::io::{Write, BufWriter};
let stdout = std::io::stdout();
let mut out = BufWriter::new(stdout.lock());
for i in 0..1000 {
    writeln!(out, "{}", i)?;
}
// flush on drop
```

For high-throughput output.

---

# Tầng 16: Patterns & best practices

## 16.1 ✅ Pattern: Standard CLI conventions

```
   ✅ DO:
   • --help / -h
   • --version / -V
   • -v / --verbose for log level
   • -q / --quiet to suppress output
   • Reasonable exit codes (0/1/2)
   • Pipes-friendly (read stdin, write stdout)
   • Color when terminal, plain when piped
   • Respect NO_COLOR env var
   • Graceful Ctrl+C handling
   • Subcommands like git/cargo (init/add/...)
```

## 16.2 ✅ Pattern: Output vs status

```rust
// Data goes to stdout (machine-readable)
println!("{}", json_result);

// Status to stderr (human-friendly)
eprintln!("Processed {} files", count);
```

Allows: `mytool | jq '.field' | other-tool`.

## 16.3 ✅ Pattern: --dry-run

```rust
struct Cli {
    /// Show what would be done without doing it
    #[arg(long)]
    dry_run: bool,
}

fn delete(file: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("Would delete: {}", file.display());
    } else {
        std::fs::remove_file(file)?;
        println!("Deleted: {}", file.display());
    }
    Ok(())
}
```

Safe by default for destructive ops.

## 16.4 ✅ Pattern: Confirm destructive

```rust
if !cli.force {
    let confirm = Confirm::new()
        .with_prompt("This will delete 100 files. Continue?")
        .default(false)
        .interact()?;
    
    if !confirm { return Ok(()); }
}
delete_files(&files)?;
```

`-f / --force` to skip.

## 16.5 ✅ Pattern: Auto-completion

```rust
use clap_complete::{generate, Shell};

#[derive(Subcommand)]
enum Commands {
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    // ... other commands
}

fn handle_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "mytool", &mut std::io::stdout());
}
```

```bash
# Install completions:
mytool completions bash > ~/.local/share/bash-completion/completions/mytool
mytool completions zsh > "${fpath[1]}/_mytool"
mytool completions fish > ~/.config/fish/completions/mytool.fish
```

## 16.6 ✅ Pattern: Graceful Ctrl+C

```rust
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let task = tokio::spawn(long_running());
    
    tokio::select! {
        _ = signal::ctrl_c() => {
            eprintln!("Cancelled by user");
            // cleanup
            return Ok(());
        }
        result = task => {
            result??;
        }
    }
    Ok(())
}
```

Ctrl+C doesn't leave files / locks dangling.

## 16.7 ❌ Antipattern: Mix output and status on stdout

```rust
// ❌ Breaks pipes
println!("Connecting to server...");
println!("{}", result);

mytool | jq '.field'   // jq sees "Connecting..." as JSON, fails
```

✅ Status to stderr.

## 16.8 ❌ Antipattern: Hard-coded paths

```rust
let config = std::fs::read_to_string("/home/user/.mytool/config.toml")?;  // ❌
```

✅ Use `dirs::config_dir()`.

## 16.9 ❌ Antipattern: Long synchronous tasks without progress

```rust
// User sees frozen terminal:
process_huge_dataset();
```

✅ Add progress bar with indicatif.

## 16.10 ❌ Antipattern: Verbose flag toggles ONLY one thing

```rust
if cli.verbose {
    println!("Detailed output");
}
```

✅ Use `-v / -vv / -vvv` for log levels, integrate with tracing.

## 16.11 ❌ Antipattern: panic for user errors

```rust
let content = std::fs::read_to_string("file.txt").unwrap();   // ❌
```

```bash
$ mytool
thread 'main' panicked at ...
note: run with RUST_BACKTRACE=1 ...
```

User confused. Use `?` + anyhow context.

## 16.12 ❌ Antipattern: Color in pipe

```rust
println!("{}", style("error").red());
// Pipes to file → ANSI codes in file
```

✅ Detect TTY, disable colors when piped.

---

# Tổng kết — 12 nguyên tắc senior CLI

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. clap derive for type-safe args. Standard conventions.         │
│                                                                  │
│ 2. Subcommands for complex CLIs (git/cargo style).               │
│                                                                  │
│ 3. dialoguer for interactive, indicatif for progress.            │
│                                                                  │
│ 4. ratatui for full TUI apps. crossterm backend.                 │
│                                                                  │
│ 5. Output to stdout, status/logs to stderr.                       │
│                                                                  │
│ 6. Respect TTY detection. NO_COLOR support.                       │
│                                                                  │
│ 7. Config layers: defaults < file < env < CLI args.              │
│                                                                  │
│ 8. anyhow + context for user-friendly errors.                    │
│                                                                  │
│ 9. Exit codes proper (0/1/2/sysexits).                           │
│                                                                  │
│ 10. Test with assert_cmd + assert_fs.                            │
│                                                                  │
│ 11. cargo-dist for cross-platform distribution.                   │
│                                                                  │
│ 12. Lazy init heavy deps. Profile startup time.                  │
└──────────────────────────────────────────────────────────────────┘
```

---

# CLI toolkit

| Crate | Purpose |
|-------|---------|
| `clap` | Argument parsing |
| `clap_complete` | Shell completions |
| `dialoguer` | Interactive prompts |
| `indicatif` | Progress bars, spinners |
| `console` | Terminal styling, detection |
| `anstyle` | Modern ANSI styling |
| `ratatui` | TUI framework |
| `crossterm` | Cross-platform terminal |
| `tui-textarea` | Text input for ratatui |
| `figment` | Layered config |
| `dirs` | Platform paths |
| `anyhow` / `thiserror` | Error handling |
| `miette` | Fancy error reporting |
| `tracing` | Logging |
| `assert_cmd` | CLI testing |
| `assert_fs` | Filesystem fixtures |
| `predicates` | Test assertions |
| `insta` | Snapshot tests |
| `cargo-dist` | Multi-platform distribution |
| `cargo-deb` / `cargo-generate-rpm` | Linux packaging |
| `hyperfine` | Benchmarking |
| `rayon` | Parallel iterators |
| `atty` | TTY detection |

---

# Bộ tài liệu giờ có 21 chương!

```
   📚 RUST FOUNDATIONS LIBRARY
   
   a-e:   Nền tảng (memory, ownership, trait, generic, closure)
   f-h:   Concurrency + errors (async, error, macros)
   i-j:   Memory advanced (smart pointers, lifetime)
   k-o:   Production (perf, observ, iterator, unsafe, testing)
   p-s:   Apps (embedded, axum, database, tauri)
   t-u:   Special domains (wasm, CLI tools)   ← MỚI CHƯƠNG u
```

🦀 Báo nếu muốn tiếp với:
- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced)
- **gRPC** (tonic)
- **Cryptography** (rustls, ring, age)
- ...
