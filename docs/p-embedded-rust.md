# Embedded Rust — Deep Dive

> Tài liệu thứ 16 trong bộ Rust nền tảng. Đọc trước:
> - [memory-model.md](./memory-model.md) — embedded memory layout cực kỳ quan trọng
> - [ownership-borrowing.md](./ownership-borrowing.md) — no_std vẫn dùng
> - [trait.md](./trait.md) — embedded-hal là trait ecosystem
> - [async.md](./async.md) — embassy là async embedded
> - [unsafe-rust.md](./unsafe-rust.md) — register access cần unsafe
>
> **Embedded Rust** = chạy Rust trên hardware nhỏ (MCU/microcontroller — vài KB đến vài MB RAM):
> - **No OS** (bare metal): code chạy thẳng trên CPU
> - **No heap** (no_std): không có allocator mặc định
> - **Real-time**: deadline cứng, không miss interrupts
> - **Resource constraint**: vài KB RAM, vài MB flash
>
> Rust mạnh ở embedded vì:
> - Memory safety không cost runtime → fit hardware nhỏ
> - Zero-cost abstraction → high-level code, low-level speed
> - Type-state pattern → compile-time guarantee (vd: pin config sai → compile error)
> - Async embedded (embassy) → multitasking không cần RTOS
>
> Tài liệu này dạy bạn embedded Rust từ bare-metal đến framework hiện đại.

---

# Mục lục

- [Tầng 1: Embedded là gì? Tại sao Rust?](#tầng-1-embedded-là-gì-tại-sao-rust)
- [Tầng 2: no_std — Rust không có standard library](#tầng-2-no_std--rust-không-có-standard-library)
- [Tầng 3: Embedded toolchain — Cross-compilation](#tầng-3-embedded-toolchain--cross-compilation)
- [Tầng 4: Memory layout — Flash, RAM, Stack](#tầng-4-memory-layout--flash-ram-stack)
- [Tầng 5: Embedded ecosystem — PAC, HAL, BSP](#tầng-5-embedded-ecosystem--pac-hal-bsp)
- [Tầng 6: embedded-hal — Trait ecosystem](#tầng-6-embedded-hal--trait-ecosystem)
- [Tầng 7: Type-state pattern — Compile-time hardware safety](#tầng-7-type-state-pattern--compile-time-hardware-safety)
- [Tầng 8: Interrupts — Hardware events](#tầng-8-interrupts--hardware-events)
- [Tầng 9: RTIC — Real-Time Interrupt-driven Concurrency](#tầng-9-rtic--real-time-interrupt-driven-concurrency)
- [Tầng 10: Embassy — Async embedded](#tầng-10-embassy--async-embedded)
- [Tầng 11: DMA — Direct Memory Access](#tầng-11-dma--direct-memory-access)
- [Tầng 12: Real-time constraints](#tầng-12-real-time-constraints)
- [Tầng 13: Debugging — probe-rs, defmt, RTT](#tầng-13-debugging--probe-rs-defmt-rtt)
- [Tầng 14: Power & low-power patterns](#tầng-14-power--low-power-patterns)
- [Tầng 15: Patterns và Antipatterns](#tầng-15-patterns-và-antipatterns)

---

# Tầng 1: Embedded là gì? Tại sao Rust?

## 1.1 Embedded systems spectrum

```
   Big                                                 Small
   ────────────────────────────────────────────────────────►
   
   Server  │ PC │ Phone │ RPi │ Beaglebone │ Cortex-A │ Cortex-M │ ATtiny
   16GB+   │ 8GB│ 4GB   │ 1GB │ 512MB      │ 128MB    │ 64-512KB │ 1-8KB
            │
            │  ← Linux + std Rust
            │
                                  Linux/RTOS    no_std Rust  no_std Rust
                                  std Rust       embassy      bare metal
```

Embedded thường = **MCU** (Microcontroller Unit):
- ARM Cortex-M0/M3/M4/M7/M33 (STM32, nRF52, RP2040)
- RISC-V (ESP32-C3, GD32V)
- ESP32 (Xtensa)
- AVR (Arduino, ATtiny)

Tầm 1KB-512KB RAM, 4KB-2MB flash. CPU 16-300 MHz.

## 1.2 Khác biệt với "PC programming"

| Aspect | PC/Server (std) | Embedded (no_std) |
|--------|-----------------|-------------------|
| OS | Yes (Linux, Win) | Often no |
| Heap | Yes (malloc) | Usually no |
| Threads | Yes | RTOS hoặc interrupts |
| std::io | Yes | No (write to UART) |
| println! | Yes | No (defmt::info!) |
| panic | Process die | MCU stuck/reset |
| Stack size | MB | KB |
| Reset | Restart process | Hard reset MCU |

## 1.3 Tại sao Rust cho embedded?

### Lý do 1: Memory safety không runtime cost
C/C++ classic embedded language. Bugs:
- Buffer overflow
- Use-after-free
- Concurrent access glitch
- Wrong pointer arithmetic

Rust prevent compile-time → **không debug nightmare in field**.

### Lý do 2: Zero-cost abstraction
HAL với traits → high-level code, no runtime overhead:
```rust
led.set_high().unwrap();   // compiles to single ARM instruction
```

### Lý do 3: Type-state pattern
Compile-time hardware safety:
```rust
let pin = pin.into_push_pull_output();
pin.set_high();   // OK

let pin = pin.into_floating_input();
pin.set_high();   // ❌ compile error — input không có set_high
```

Wrong pin mode = compile fail, not runtime crash.

### Lý do 4: Modern async (embassy)
Multitasking on MCU without RTOS, ~minimal RAM. (Tầng 10)

### Lý do 5: Cargo ecosystem
- 10000+ no_std crates
- Cargo dependency management
- Built-in test runner
- Documentation tooling

C/C++ embedded: Makefiles, ad-hoc, painful.

## 1.4 Khi nào Rust embedded chưa fit?

- Vendor SDK chỉ có C bindings (work nhưng kém ergonomic)
- Very old MCU (8-bit AVR cũ) — support less mature
- Team experienced với C/C++
- Strict certification (DO-178C, ISO 26262) — Rust được approve dần

Trends: Rust embedded growing fast. Big corp (Microsoft, Volvo, Ferrous Systems) use in production.

---

# Tầng 2: no_std — Rust không có standard library

## 2.1 std vs core vs alloc

```
   ┌────────────────────────────────────────────────────┐
   │  std       — Standard library (file, net, thread) │
   │   │         Requires OS + heap                    │
   │   │                                                │
   │   ├── alloc — Heap collections (Vec, String, Box) │
   │   │           Requires allocator                  │
   │   │                                                │
   │   └── core  — Language primitives (Option,        │
   │              iterator, slice, etc.)               │
   │              No OS, no heap                       │
   └────────────────────────────────────────────────────┘
   
   
   Embedded:
   ─────────
   • Bare metal: only `core` available
   • With allocator: `core` + `alloc`
   • Linux embedded: full `std`
```

## 2.2 #![no_std] attribute

```rust
#![no_std]      // Don't link std
#![no_main]     // Don't use main convention

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // bare-metal entry point
    loop {}
}
```

`#![no_std]` tells compiler don't link std. Use `core` instead.

`#![no_main]` because no OS to call `main()`. Manual entry point.

## 2.3 What's missing without std?

```
   ❌ NOT available in no_std:
   ─────────────────────────
   • println!, eprintln!    → use defmt::info!()
   • std::io, std::fs       → no filesystem
   • std::net               → no network (unless with stack)
   • std::thread             → no OS threads
   • std::sync (parts)       → core::sync::atomic exists
   • Vec, String, HashMap    → in alloc, need allocator
   • Mutex                   → critical_section crate
   • Box<T>                  → in alloc
   • std::time::Instant      → use hardware timer
   
   ✅ Available in no_std (from core):
   ───────────────────────────────────
   • Option, Result
   • Iterator trait + methods
   • slice, array, str
   • core::sync::atomic
   • core::cell::Cell, RefCell
   • Traits (Debug, Display, Iterator)
   • Numeric types, math (core::f32::sqrt)
   • const fn
```

## 2.4 alloc — Optional heap

```rust
#![no_std]

extern crate alloc;
use alloc::{vec::Vec, string::String, boxed::Box};
```

`alloc` crate adds heap collections. Need to provide allocator:

```rust
use embedded_alloc::Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

fn main() {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe {
        HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
    
    let v = vec![1, 2, 3];   // Now Vec works!
}
```

Trade-off:
- ✅ More flexible (Vec, String)
- ❌ Slower (alloc cost)
- ❌ Fragmentation
- ❌ OOM crash potential

Many embedded codebases avoid alloc entirely — use fixed-size arrays.

## 2.5 heapless crate — Fixed-size collections

```toml
[dependencies]
heapless = "0.8"
```

```rust
use heapless::Vec;
use heapless::String;

let mut v: Vec<u8, 32> = Vec::new();   // max 32 elements
v.push(1).unwrap();

let mut s: String<64> = String::new();   // max 64 bytes
```

`heapless` gives `Vec`, `String`, `HashMap`, queue — all **stack-allocated** with fixed max size. No heap needed.

Most embedded Rust uses heapless.

## 2.6 Common no_std crates

| Crate | Purpose |
|-------|---------|
| `heapless` | Stack-allocated collections |
| `embedded-hal` | Trait ecosystem (Tầng 6) |
| `cortex-m` / `cortex-m-rt` | ARM Cortex-M runtime |
| `nb` | Non-blocking abstraction |
| `defmt` | Logging for embedded |
| `panic-probe` | Panic handler that prints via defmt |
| `critical-section` | Cross-platform critical sections |
| `embassy-*` | Async embedded |
| `rtic` | Real-time framework |

---

# Tầng 3: Embedded toolchain — Cross-compilation

## 3.1 Targets

```bash
# List targets:
rustup target list

# Install target for ARM Cortex-M4 (e.g., STM32F4, nRF52):
rustup target add thumbv7em-none-eabihf

# RISC-V:
rustup target add riscv32imc-unknown-none-elf

# ESP32 (Xtensa) — needs ESP toolchain:
# https://github.com/esp-rs/rust
```

Common ARM Cortex-M targets:
- `thumbv6m-none-eabi` — Cortex-M0/M0+ (RP2040)
- `thumbv7m-none-eabi` — Cortex-M3
- `thumbv7em-none-eabi` — Cortex-M4/M7 (no FPU)
- `thumbv7em-none-eabihf` — Cortex-M4/M7 (with FPU, hard-float)
- `thumbv8m.main-none-eabihf` — Cortex-M33

`-none-` = no OS. `-eabi` = ABI. `hf` = hard-float (use FPU).

## 3.2 Cargo config

```toml
# .cargo/config.toml
[build]
target = "thumbv7em-none-eabihf"

[target.thumbv7em-none-eabihf]
runner = "probe-rs run --chip STM32F411RETx"
rustflags = [
    "-C", "link-arg=-Tlink.x",
]
```

Now `cargo run` flashes and runs on hardware.

## 3.3 memory.x — Memory layout

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 512K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
```

Tells linker where code (FLASH) and data (RAM) go on the chip. Vendor datasheet has these addresses.

`cortex-m-rt` reads this. Without correct memory.x, code won't run.

## 3.4 Cargo.toml — Embedded dependencies

```toml
[dependencies]
cortex-m = "0.7"
cortex-m-rt = "0.7"
panic-probe = { version = "0.3", features = ["print-defmt"] }
defmt = "0.3"
defmt-rtt = "0.4"

stm32f4xx-hal = { version = "0.21", features = ["stm32f411"] }

[profile.release]
opt-level = "s"     # optimize for size
codegen-units = 1
lto = true
debug = true         # keep symbols for debugging
```

`opt-level = "s"` or `"z"` — embedded prefers small binary. Flash is limited.

## 3.5 Minimal example

```rust
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_probe as _;
use defmt_rtt as _;

#[entry]
fn main() -> ! {
    defmt::info!("Hello from embedded!");
    loop {
        cortex_m::asm::wfi();   // wait for interrupt (low power)
    }
}
```

- `#[entry]` = entry point macro from cortex-m-rt
- `-> !` = function never returns
- `wfi` = "wait for interrupt", CPU sleeps

Compile:
```bash
cargo build --release
```

Output: `target/thumbv7em-none-eabihf/release/myapp` — ELF file.

Flash:
```bash
cargo run --release   # via probe-rs
```

---

# Tầng 4: Memory layout — Flash, RAM, Stack

## 4.1 Embedded memory regions

```
   ┌─────────────────────────────────────────┐
   │  FLASH (e.g., 512KB)                    │  ← non-volatile, code + .rodata
   │   ┌────────────────────┐                │
   │   │ Vector table       │  ← interrupt handlers
   │   │ Code (.text)       │  ← your program
   │   │ Read-only data     │  ← constants, string literals
   │   └────────────────────┘                │
   ├─────────────────────────────────────────┤
   │  RAM (e.g., 128KB)                       │
   │   ┌────────────────────┐                │
   │   │ .data (initialized) │ ← static muts │
   │   │ .bss (zeroed)      │ ← static = 0  │
   │   │                    │                │
   │   │   ↓ heap grows ↓   │  (optional)   │
   │   │                    │                │
   │   │       ...           │                │
   │   │                    │                │
   │   │   ↑ stack grows ↑  │                │
   │   │ STACK (top of RAM) │                │
   │   └────────────────────┘                │
   └─────────────────────────────────────────┘
   
   Plus: Peripheral registers at fixed addresses (0x40000000+)
```

Flash: code + constants. Not modifiable at runtime (without flash unlock).
RAM: variables. Wiped on reset.
Stack: grows down from top of RAM. Limited (often 4-16KB).

## 4.2 Vector table

First section of flash. Contains:
- Reset handler (called on power-up/reset)
- Initial stack pointer
- Interrupt handlers (NMI, HardFault, IRQs)

```
Flash offset 0x000: [initial SP value]
Flash offset 0x004: [Reset_Handler address]
Flash offset 0x008: [NMI_Handler address]
Flash offset 0x00C: [HardFault_Handler address]
...
Flash offset 0x040+: [IRQ handlers]
```

`cortex-m-rt` provides default vector table. `#[interrupt]` macro fills entries.

## 4.3 Stack size

```ld
/* memory.x */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
```

Default: stack starts at top of RAM, grows down. Heap (if exists) at bottom.

Stack overflow on embedded = silent corruption (or HardFault).

→ Audit stack usage. Tools:
- `cargo-call-stack` (estimates worst-case)
- `cargo size` (binary size info)

## 4.4 .data vs .bss

```rust
static mut DATA: u32 = 42;     // .data — initial value 42 in flash, copied to RAM at boot
static mut BSS: u32 = 0;       // .bss — zeroed at boot, no flash space needed
```

`.data` costs flash + RAM. `.bss` costs only RAM. Prefer `= 0` initialization.

## 4.5 Const vs static

```rust
const PI: f32 = 3.14159;        // Inlined at use site (no fixed address)
static PI: f32 = 3.14159;        // Fixed address in flash (.rodata)
static mut COUNTER: u32 = 0;     // Fixed address in RAM (.data)
```

- `const` cheap (no memory cost)
- `static` is in flash (.rodata) or RAM (.data/.bss)

## 4.6 Memory-mapped registers

Peripherals (GPIO, UART, timers) controlled via **memory-mapped registers**:

```rust
// Set bit 5 of GPIOA mode register:
unsafe {
    let addr = 0x4002_0000 as *mut u32;
    let val = core::ptr::read_volatile(addr);
    core::ptr::write_volatile(addr, val | (1 << 5));
}
```

`volatile`: prevent compiler optimize away reads/writes to MMIO.

Reading register might have side effects (e.g., clearing flag). Writing affects hardware.

**Never** use cached / reordered access for MMIO. Always volatile.

## 4.7 Linker script in detail

`memory.x` defines memory regions. `link.x` (from cortex-m-rt) maps sections:
- `.text` → FLASH
- `.rodata` → FLASH
- `.data` → RAM (init from FLASH)
- `.bss` → RAM (zeroed)
- `.vector_table` → start of FLASH

Customize if needed (e.g., bootloader at start of flash).

---

# Tầng 5: Embedded ecosystem — PAC, HAL, BSP

## 5.1 Layers of abstraction

```
   ┌────────────────────────────────────────────────────┐
   │ Application code                                   │  ← your code
   │ (rtic, embassy, bare loop)                         │
   ├────────────────────────────────────────────────────┤
   │ BSP (Board Support Package)                        │  ← board-specific
   │ (e.g., nucleo-f411re, blackpill)                   │     (pin maps, LEDs, btns)
   ├────────────────────────────────────────────────────┤
   │ HAL (Hardware Abstraction Layer)                   │  ← chip-family ergonomic API
   │ (e.g., stm32f4xx-hal, nrf52840-hal)                │     (GPIO, UART, SPI, I2C)
   ├────────────────────────────────────────────────────┤
   │ PAC (Peripheral Access Crate)                      │  ← auto-generated from SVD
   │ (e.g., stm32f4, nrf52840-pac)                      │     (raw register access)
   ├────────────────────────────────────────────────────┤
   │ Cortex-M / RISC-V runtime                          │  ← architecture
   │ (cortex-m-rt)                                      │
   ├────────────────────────────────────────────────────┤
   │ Hardware (MCU)                                     │
   └────────────────────────────────────────────────────┘
```

## 5.2 PAC — Peripheral Access Crate

Auto-generated from SVD (System View Description, vendor XML):

```rust
use stm32f4::stm32f411::Peripherals;

let p = Peripherals::take().unwrap();   // get all peripherals (once)
p.GPIOA.moder.modify(|_, w| w.moder5().output());  // set PA5 as output
p.GPIOA.odr.modify(|_, w| w.odr5().set_bit());     // PA5 high
```

Type-safe register access. Each register / field has typed methods.

Disadvantage: still verbose, low-level.

## 5.3 HAL — Hardware Abstraction Layer

Higher-level API over PAC:

```rust
use stm32f4xx_hal::{prelude::*, pac, gpio::*};

let dp = pac::Peripherals::take().unwrap();
let rcc = dp.RCC.constrain();
let clocks = rcc.cfgr.freeze();
let gpioa = dp.GPIOA.split();

let mut led = gpioa.pa5.into_push_pull_output();
led.set_high();
```

Type-state encoded:
- `pa5: Pin<Gpioa, 5, Disabled>`
- After `.into_push_pull_output()`: `Pin<Gpioa, 5, Output<PushPull>>`
- `.set_high()` available only for Output type
- Try `.set_high()` on Input pin → compile error

## 5.4 BSP — Board Support Package

For specific board (chip + onboard peripherals):

```rust
use nucleo_f411re::{prelude::*, Board};

let board = Board::take().unwrap();
let mut led = board.led_user;
led.toggle();
```

BSP knows: LED is on PA5, button on PC13, etc. Convenient for tutorials.

## 5.5 SVD — Vendor description

SVD (XML) describes all peripherals + registers:
```xml
<peripheral>
  <name>GPIOA</name>
  <baseAddress>0x40020000</baseAddress>
  <registers>
    <register>
      <name>MODER</name>
      <addressOffset>0x00</addressOffset>
      ...
    </register>
  </registers>
</peripheral>
```

`svd2rust` tool generates Rust PAC from SVD. Most vendors publish SVD.

## 5.6 Ecosystem maturity

```
   ✅ Mature:
   ──────────
   STM32 (stm32-rs/stm32f4xx-hal, stm32g0xx-hal, ...)
   nRF52 (nrf-rs)
   RP2040 (rp-hal)
   ESP32 (esp-rs — separate toolchain)
   
   ⚠️ Less mature:
   ──────────────
   Microchip PIC32
   Renesas RA
   TI MSP430
   
   Choose chip with active Rust support → easier journey
```

---

# Tầng 6: embedded-hal — Trait ecosystem

## 6.1 The problem: vendor lock-in

Without embedded-hal:
- Sensor library for STM32 won't work on nRF52
- Each HAL has different API
- Code không portable

## 6.2 embedded-hal traits

`embedded-hal` defines standard traits cho peripherals:

```rust
// GPIO
trait OutputPin {
    fn set_high(&mut self) -> Result<(), Error>;
    fn set_low(&mut self) -> Result<(), Error>;
}

trait InputPin {
    fn is_high(&self) -> Result<bool, Error>;
}

// SPI
trait SpiDevice<W = u8> {
    fn transaction(&mut self, ops: &mut [Operation<W>]) -> Result<(), Error>;
}

// I2C
trait I2c<A = SevenBitAddress, B = u8> {
    fn read(&mut self, address: A, buffer: &mut [B]) -> Result<(), Error>;
    fn write(&mut self, address: A, bytes: &[B]) -> Result<(), Error>;
}

// UART
trait Read<W = u8> { ... }
trait Write<W = u8> { ... }

// Delay
trait DelayMs<T> { fn delay_ms(&mut self, ms: T); }
trait DelayUs<T> { fn delay_us(&mut self, us: T); }
```

## 6.3 Sensor library that works anywhere

```rust
// BME280 driver crate
pub struct Bme280<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C, E> Bme280<I2C> 
where I2C: embedded_hal::i2c::I2c<Error = E>
{
    pub fn read_temp(&mut self) -> Result<f32, E> {
        let mut buf = [0u8; 4];
        self.i2c.write_read(self.address, &[0xFA], &mut buf)?;
        // parse temp
        Ok(temp)
    }
}
```

Works on STM32, nRF52, RP2040 — anyone implementing `embedded_hal::i2c::I2c`.

## 6.4 Using sensor library

```rust
// On STM32:
let i2c = I2c::new(dp.I2C1, (scl, sda), 100.kHz(), &clocks);
let mut sensor = Bme280::new(i2c, 0x76);
let temp = sensor.read_temp().unwrap();

// On nRF52:
let i2c = twim::Twim::new(dp.TWIM0, ...);
let mut sensor = Bme280::new(i2c, 0x76);  // same code!
let temp = sensor.read_temp().unwrap();
```

Same sensor code, different MCUs. Ecosystem benefit.

## 6.5 embedded-hal-async

For async drivers:
```rust
trait I2c { 
    async fn read(...) -> Result<(), Error>;
    async fn write(...) -> Result<(), Error>;
}
```

Used by embassy. Drivers can be `async`-aware → non-blocking I/O.

## 6.6 Version evolution

- embedded-hal 0.2 (blocking + nb)
- embedded-hal 1.0 (stabilized, fewer traits, cleaner)
- embedded-hal-async (async version)

Most ecosystem moving to 1.0. New code use 1.0.

---

# Tầng 7: Type-state pattern — Compile-time hardware safety

## 7.1 The pattern

Pin in different states:
- Uninitialized
- Configured as output
- Configured as input pull-up
- Used as alternate function (SPI MOSI, UART TX, ...)

Each state has different valid operations. Type-state encodes state in type:

```rust
struct Pin<MODE> {
    _mode: PhantomData<MODE>,
}

struct Disabled;
struct Output<PUSH_PULL>;
struct Input<PullUp>;
struct Alternate<AF1>;

impl Pin<Disabled> {
    fn into_push_pull_output(self) -> Pin<Output<PushPull>> { ... }
    fn into_input(self) -> Pin<Input<NoPull>> { ... }
}

impl<P> Pin<Output<P>> {
    fn set_high(&mut self) { ... }
    fn set_low(&mut self) { ... }
}

impl Pin<Input<NoPull>> {
    fn is_high(&self) -> bool { ... }
    // NO set_high — can't set input high!
}
```

Use:
```rust
let pin: Pin<Disabled> = ...;
let pin = pin.into_push_pull_output();   // type: Pin<Output<PushPull>>
pin.set_high();                           // OK

let pin = pin.into_input();               // type: Pin<Input<NoPull>>
pin.set_high();                           // ❌ COMPILE ERROR
```

Hardware misuse caught at compile time.

## 7.2 Resource ownership

```rust
let i2c = I2c::new(dp.I2C1, ...);   // takes ownership of I2C1 peripheral

let i2c2 = I2c::new(dp.I2C1, ...);  // ❌ ERROR: dp.I2C1 already moved
```

Each peripheral can only be used once. Type system prevents conflicts.

## 7.3 Singleton — Peripherals::take()

```rust
let p = stm32f4::stm32f411::Peripherals::take().unwrap();
let p2 = stm32f4::stm32f411::Peripherals::take();
assert!(p2.is_none());   // can only take once!
```

Ensures no two parts of code claim same hardware.

## 7.4 Compile-time pin connection

```rust
// SPI requires specific pins:
let spi = Spi::new(
    dp.SPI1,
    (sck_pin, miso_pin, mosi_pin),   // must be SPI1's pins
    config,
);

// Wrong pin → compile error
let wrong_pin = gpiob.pb0.into_push_pull_output();
let spi = Spi::new(dp.SPI1, (wrong_pin, ...), config);
// ❌ pb0 is not SPI1 SCK — compile error
```

Datasheet says "PA5/SCK1, PA6/MISO1, PA7/MOSI1 for SPI1". HAL enforces.

## 7.5 Frozen clocks

```rust
let rcc = dp.RCC.constrain();
let clocks = rcc.cfgr
    .sysclk(84.MHz())
    .freeze();                       // clock config frozen

// Now `clocks` is immutable
let i2c = I2c::new(dp.I2C1, ..., &clocks);   // pass to peripherals
// Can't change clock setup mid-program → consistent timing
```

After freeze, clock cannot be modified. Subsequent code uses fixed timings.

---

# Tầng 8: Interrupts — Hardware events

## 8.1 What are interrupts?

Hardware event triggers CPU to:
1. Save current state
2. Jump to interrupt handler
3. Execute handler
4. Resume normal code

Events: timer expire, GPIO change, UART data ready, ADC complete, ...

Essential for responsive embedded systems.

## 8.2 Manual IRQ handler (cortex-m)

```rust
use stm32f4xx_hal::pac::interrupt;
use cortex_m::peripheral::NVIC;
use stm32f4xx_hal::pac::Interrupt;

#[interrupt]
fn TIM2() {
    // Timer 2 interrupt
    defmt::info!("Timer tick!");
    // Clear interrupt flag (else fires forever)
    unsafe {
        (*stm32f4::stm32f411::TIM2::ptr()).sr.write(|w| w);
    }
}

fn main() -> ! {
    let mut nvic = cortex_m::Peripherals::take().unwrap().NVIC;
    unsafe { NVIC::unmask(Interrupt::TIM2); }
    
    // ... setup timer ...
    
    loop {
        cortex_m::asm::wfi();
    }
}
```

`#[interrupt]` macro registers handler in vector table.

## 8.3 Sharing data between main and IRQ

Classic challenge. IRQ can preempt main. Sharing data needs sync.

### ❌ Race condition:
```rust
static mut COUNTER: u32 = 0;

#[interrupt]
fn TIM2() {
    unsafe { COUNTER += 1; }    // ⚠️ race with main
}

fn main() -> ! {
    loop {
        unsafe { defmt::info!("{}", COUNTER); }   // ⚠️ race
    }
}
```

### ✅ Atomic for simple types:
```rust
use core::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn TIM2() {
    COUNTER.fetch_add(1, Ordering::Relaxed);
}

fn main() -> ! {
    loop {
        defmt::info!("{}", COUNTER.load(Ordering::Relaxed));
    }
}
```

### ✅ Critical section for complex data:
```rust
use critical_section::Mutex;
use core::cell::RefCell;

static SHARED: Mutex<RefCell<Vec<u32>>> = Mutex::new(RefCell::new(Vec::new()));

#[interrupt]
fn TIM2() {
    critical_section::with(|cs| {
        let mut v = SHARED.borrow_ref_mut(cs);
        v.push(123);
    });
}

fn main() -> ! {
    loop {
        critical_section::with(|cs| {
            let v = SHARED.borrow_ref(cs);
            for item in v.iter() { ... }
        });
    }
}
```

`critical_section::with` disables interrupts temporarily — safe access.

## 8.4 IRQ priorities

NVIC supports priorities. Higher priority preempts lower.

```rust
unsafe {
    let mut nvic = cortex_m::Peripherals::steal().NVIC;
    nvic.set_priority(Interrupt::TIM2, 1);   // priority 1
    nvic.set_priority(Interrupt::TIM3, 0);   // priority 0 (higher)
}
```

Priority 0 = highest in Cortex-M. Critical hardware (faults) priority 0.

Wrong priority assignment → priority inversion, deadlines missed.

## 8.5 IRQ guidelines

```
   ✅ DO in IRQ:
   • Quick (microseconds)
   • Clear interrupt flag
   • Update shared state (atomic preferred)
   • Signal main loop via flag/queue
   
   ❌ DON'T in IRQ:
   • Long computation
   • Print (defmt::info!) — slow
   • Allocate memory
   • Wait/poll for another peripheral
   • Float operations (slow without FPU; even with FPU has lazy stacking cost)
   
   📌 IRQ = signal, main = do work
```

## 8.6 Polling vs Interrupt

```rust
// ❌ Polling — CPU 100%
loop {
    if uart.read_ready() {
        let byte = uart.read();
        process(byte);
    }
}

// ✅ Interrupt-driven
#[interrupt]
fn UART_RX() {
    let byte = uart.read();
    QUEUE.push(byte);   // queue, signal main
}

fn main() -> ! {
    loop {
        if let Some(byte) = QUEUE.pop() {
            process(byte);
        }
        cortex_m::asm::wfi();   // sleep until next IRQ
    }
}
```

Interrupt-driven + WFI → low power + responsive.

---

# Tầng 9: RTIC — Real-Time Interrupt-driven Concurrency

## 9.1 RTIC philosophy

Manual IRQ + critical_section can be tedious. **RTIC** (Real-Time Interrupt-driven Concurrency) — declarative framework:

- Tasks = priority-assigned IRQs or software tasks
- Shared resources = automatically locked when accessed
- No runtime overhead (compile-time analysis)
- Deadline guarantees

## 9.2 Setup

```toml
[dependencies]
rtic = { version = "2.1", features = ["thumbv7-backend"] }
stm32f4xx-hal = "0.21"
```

## 9.3 Example

```rust
#![no_std]
#![no_main]

use panic_probe as _;
use defmt_rtt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use stm32f4xx_hal::{prelude::*, gpio::*, timer::*};
    
    #[shared]
    struct Shared {
        counter: u32,
    }
    
    #[local]
    struct Local {
        led: PA5<Output<PushPull>>,
        timer: Timer<TIM2>,
    }
    
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let dp = ctx.device;
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze();
        let gpioa = dp.GPIOA.split();
        let led = gpioa.pa5.into_push_pull_output();
        
        let mut timer = Timer::new(dp.TIM2, &clocks);
        timer.start(1.Hz()).unwrap();
        timer.listen(Event::Update);
        
        (Shared { counter: 0 }, Local { led, timer })
    }
    
    #[task(binds = TIM2, priority = 1, shared = [counter], local = [led, timer])]
    fn tim2_handler(mut ctx: tim2_handler::Context) {
        ctx.local.led.toggle();
        ctx.shared.counter.lock(|c| *c += 1);   // auto-lock!
        ctx.local.timer.clear_interrupt(Event::Update);
    }
    
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
```

Key parts:
- `#[shared]` — shared between tasks (auto-locked)
- `#[local]` — owned by one task
- `#[init]` — runs once at startup
- `#[task(binds = TIM2)]` — software task or IRQ handler
- `#[idle]` — runs when nothing else to do

## 9.4 Priority and locks

```rust
#[task(priority = 1, shared = [data])]
fn low_task(mut ctx: low_task::Context) {
    ctx.shared.data.lock(|d| { ... });   // locks IRQ priority 1+ for short time
}

#[task(priority = 2, shared = [data])]
fn high_task(mut ctx: high_task::Context) {
    ctx.shared.data.lock(|d| { ... });   // no actual lock — already highest
}
```

Lock = raise NVIC priority temporarily. Compile-time analysis ensures correctness.

## 9.5 Software tasks

```rust
#[task(priority = 1)]
async fn slow_task(_: slow_task::Context) {
    // ... slow work ...
}

#[task(binds = USART1)]
fn rx_handler(_: rx_handler::Context) {
    // spawn slow_task on lower priority
    slow_task::spawn().ok();
}
```

Heavy work in software task (priority N), IRQ handler short.

## 9.6 RTIC strengths

- **Deterministic**: deadline analysis possible
- **Zero-cost**: compile to optimal IRQ + lock sequences
- **Safe**: priority-based locking prevents deadlocks
- **Composable**: scale to many tasks

Used in industrial, automotive, drone projects.

## 9.7 RTIC vs threads

| Threads (RTOS) | RTIC |
|----------------|------|
| Each thread has stack | Tasks share stack (only one runs at a time per priority) |
| Context switch overhead | No (just IRQ priority change) |
| Mutex blocking | Compile-time analysis |
| Real-time? Hard | Easier deadlines |

Trade-off: RTIC has steeper learning curve. RTOS familiar to C devs.

---

# Tầng 10: Embassy — Async embedded

## 10.1 Embassy là gì?

Embassy: **async/await** trên embedded. Like tokio but for MCU.

```rust
async fn blink_led(mut led: Pin<...>) {
    loop {
        led.set_high();
        Timer::after_secs(1).await;   // ← yields, low power
        led.set_low();
        Timer::after_secs(1).await;
    }
}
```

Looks like Linux Rust. Runs on 32KB RAM MCU.

## 10.2 Why embassy?

Traditional embedded multitasking:
- **Polling loop**: tedious, high CPU
- **IRQ-driven**: complex shared state
- **RTOS**: heavyweight, many threads = many stacks

Embassy:
- **Concurrent tasks** with async/await
- **Single stack** (no per-task stack)
- **Low power**: tasks yield → CPU sleep
- **Familiar Rust async syntax**

## 10.3 Setup

```toml
[dependencies]
embassy-executor = { version = "0.6", features = ["arch-cortex-m", "executor-thread"] }
embassy-stm32 = { version = "0.1", features = ["stm32f411re", "time-driver-tim2"] }
embassy-time = "0.3"
embassy-futures = "0.1"
```

## 10.4 Multi-task example

```rust
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
async fn blink_led(mut led: Output<'static>) {
    loop {
        led.set_high();
        Timer::after_millis(500).await;
        led.set_low();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn read_sensor(mut i2c: ...) {
    loop {
        let value = read_temp(&mut i2c).await;
        defmt::info!("Temp: {}", value);
        Timer::after_secs(10).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    
    let led = Output::new(p.PA5, Level::Low, Speed::Low);
    spawner.spawn(blink_led(led)).unwrap();
    
    let i2c = embassy_stm32::i2c::I2c::new(p.I2C1, p.PB6, p.PB7, ...);
    spawner.spawn(read_sensor(i2c)).unwrap();
    
    // Main can do other things or block here forever
}
```

2 tasks chạy concurrent — blink LED + read sensor every 10s.

## 10.5 Async I/O — Truly non-blocking

```rust
// Async UART read:
let mut buf = [0u8; 64];
let n = uart.read(&mut buf).await;   // CPU sleep until data arrives
```

Hardware IRQ wakes task when data ready. No polling.

## 10.6 Embassy executor architecture

```
   ┌─────────────────────────────────────────┐
   │ Embassy Executor (no_std)               │
   │                                          │
   │ Task pool: pre-allocated futures        │
   │ Ready queue: tasks waiting to poll       │
   │                                          │
   │ Loop:                                    │
   │   - Poll ready tasks                     │
   │   - When all idle: WFI (CPU sleep)       │
   │   - IRQ wakes → poll affected task       │
   └─────────────────────────────────────────┘
```

Tasks share single stack. Each task is async fn → state machine sized at compile.

## 10.7 Embassy time driver

Embassy needs hardware timer for Timer::after():

```rust
features = ["time-driver-tim2"]   // use TIM2 for time
```

Picks a free hardware timer.

## 10.8 Embassy vs RTIC

| Aspect | Embassy | RTIC |
|--------|---------|------|
| Style | async/await | task + IRQ |
| Concurrency model | Cooperative | Preemptive |
| Real-time | Harder (cooperative) | Easier (priority-based) |
| Familiarity | Like tokio | Embedded-specific |
| Learning curve | Easier if know async | Steeper |
| Resource usage | Slightly higher | Minimal |

Recommendation:
- **Embassy**: prototyping, less time-critical, want async-style
- **RTIC**: hard real-time, deterministic deadlines

Many projects use **Embassy** today — simpler to write.

## 10.9 Embassy ecosystem

- `embassy-net` — TCP/IP stack (network!)
- `embassy-usb` — USB device implementation
- `embassy-nrf` / `embassy-stm32` / `embassy-rp` — chip support
- `embassy-sync` — async sync primitives (Channel, Mutex)

Bigger than just executor — full embedded ecosystem.

---

# Tầng 11: DMA — Direct Memory Access

## 11.1 What is DMA?

**DMA** (Direct Memory Access): hardware moves data peripheral ↔ memory **without CPU**.

Use cases:
- UART RX large buffer (CPU sleeps)
- ADC continuous sampling
- SPI bulk transfer
- Audio DAC streaming

Saves CPU cycles. Lower power.

## 11.2 DMA flow

```
   Without DMA:
   ────────────
   for byte in incoming { cpu_copy_to_buffer(byte); }
   
   With DMA:
   ─────────
   dma.setup(peripheral_addr, &mut buffer, length);
   // CPU returns immediately
   // DMA engine copies bytes one by one in background
   // Generates interrupt when done or buffer half-full
```

## 11.3 DMA in HAL

```rust
// SPI with DMA:
let mut dma_buffer = [0u8; 1024];
let transfer = spi.write_dma(&mut dma_buffer);

// transfer is a Future — CPU free
let result = transfer.await;   // when DMA done
```

(With embassy / async; sync drivers have callback.)

## 11.4 Memory considerations

DMA needs:
- **Static buffer** (or `'static lifetime`)
- **Aligned memory** (often 4-byte)
- Sometimes specific memory region (e.g., AHB-accessible SRAM)

```rust
#[link_section = ".dma_buffer"]
static mut DMA_BUF: [u8; 256] = [0; 256];
```

Or specific SRAM banks for certain chips.

## 11.5 Circular DMA

For continuous reception:
```rust
// DMA wraps around when buffer full
dma.setup_circular(peripheral, &mut buffer);
// Interrupts fire at half-full and full
// Main code reads first half while DMA fills second
```

Double-buffering pattern. Stream audio, large UART data.

## 11.6 DMA + interrupt

```rust
#[interrupt]
fn DMA1_STREAM0() {
    // Buffer full
    PROCESS_QUEUE.push(...);
    
    // Clear flag, DMA continues
}
```

Combine: hardware DMA + software queue + low-priority task processes.

## 11.7 DMA caveats

⚠️ **UB potential**:
- DMA writing while CPU reads → race
- DMA writing past buffer end → corrupt other memory
- Cache coherence (Cortex-M7 with data cache)

Modern HAL crates handle these. But know they exist.

---

# Tầng 12: Real-time constraints

## 12.1 Hard vs Soft real-time

```
   ┌──────────────────────────────────────────────────────────┐
   │ HARD real-time:                                          │
   │  Missed deadline = FAILURE (safety, life)                │
   │  Examples: airbag, pacemaker, motor control              │
   │  Required: provable WCET                                 │
   │                                                          │
   │ SOFT real-time:                                          │
   │  Missed deadline = quality degraded                      │
   │  Examples: audio (glitch), UI (lag)                      │
   │  Acceptable: occasional misses                           │
   │                                                          │
   │ Best-effort:                                             │
   │  No deadlines, just throughput                           │
   │  Examples: data logger                                   │
   └──────────────────────────────────────────────────────────┘
```

Embedded often hard real-time. Architecture matters.

## 12.2 WCET — Worst-Case Execution Time

WCET = maximum time function can take. For hard real-time:

- Profile in worst conditions
- Analyze code statically (specialized tools)
- Avoid unpredictable operations:
  - Heap alloc (varies)
  - Recursive (stack depth)
  - Loops with input-dependent count
  - Cache misses (deterministic vs probabilistic)

## 12.3 Predictability matters

```rust
// ❌ Unpredictable
fn process(data: &[u8]) {
    let v: Vec<u8> = data.iter().filter(|&&b| b > 0).collect();
    // Alloc → unpredictable time
}

// ✅ Predictable
fn process(data: &[u8], output: &mut [u8; 64]) -> usize {
    let mut count = 0;
    for &b in data {
        if b > 0 && count < 64 {
            output[count] = b;
            count += 1;
        }
    }
    count
}
```

Fixed-size buffer, no alloc → deterministic timing.

## 12.4 Avoid syscalls / library calls

`Vec::push` → maybe alloc → unpredictable
`String::format!` → alloc + parse → slow + unpredictable
`HashMap::get` → hashing + chain walk → variable time

In hard real-time → avoid in critical paths. Use stack arrays, fixed-size, precomputed.

## 12.5 Priorities and analysis

RTIC's analysis: Rate Monotonic Analysis (RMA) for fixed-priority preemptive scheduling.

Each task: period (T), deadline (D), execution time (C).
Schedulability: sum of C/T < bound (Liu-Layland).

→ Verify deadline meetable BEFORE deployment.

## 12.6 Avoid priority inversion

```
   Low priority task A locks resource R
   Medium priority task B preempts A — A can't release R
   High priority task C wants R — must wait for A — but A blocked by B
   → C delayed indefinitely
```

Solutions:
- Priority inheritance (low task temporarily inherits high priority)
- Priority ceiling protocol (RTIC uses this)

Manual locks → easy to mess up. RTIC handles automatically.

## 12.7 Tooling

- `cargo-call-stack` — estimate stack usage
- `cargo-bloat` — binary size
- `probe-rs run` — flash + run
- Logic analyzer / oscilloscope — verify real-time behavior
- `defmt-rtt` — fast logging (Tầng 13)

## 12.8 Watchdog

Watchdog timer: reset MCU if not "fed" periodically. Safety net for crashes.

```rust
fn main() -> ! {
    let mut wdt = Watchdog::new(dp.IWDG, 100.ms());
    wdt.start();
    
    loop {
        do_work();
        wdt.feed();   // reset watchdog timer
    }
}
```

If `do_work()` hangs > 100ms → MCU reset. Last line of defense.

---

# Tầng 13: Debugging — probe-rs, defmt, RTT

## 13.1 probe-rs

Modern debugger / flasher for embedded Rust:

```bash
cargo install probe-rs --features cli

probe-rs list                # list connected probes
probe-rs flash --chip STM32F411RETx myapp
probe-rs run --chip STM32F411RETx myapp   # flash + run + log
```

Supports many probes: ST-Link, J-Link, CMSIS-DAP.

Cargo runner integration:
```toml
[target.thumbv7em-none-eabihf]
runner = "probe-rs run --chip STM32F411RETx"
```

Then `cargo run` = flash + run.

## 13.2 defmt — Efficient embedded logging

```rust
defmt::info!("counter = {}", count);
defmt::error!("Failed: {:?}", err);
defmt::trace!("entered fn");
```

Looks like `log` / `tracing`. But:
- **Compressed**: format string interned in flash, only args sent
- **Fast**: ~µs per log (vs ms for println-style)
- **No alloc**

Architecture:
```
   Device: defmt::info!("x = {}", x);
                │
                │ Format string compiled to ID, only x sent
                ▼ via RTT (Real-Time Transfer)
   Host: probe-rs decodes ID → original format string + args
```

## 13.3 RTT — Real-Time Transfer

RTT (Segger): non-blocking serial-like channel over debug interface.

Memory buffer in MCU RAM. Host reads via probe.

Speed: 100KB/s+, faster than UART, non-intrusive.

`defmt-rtt` crate uses RTT as transport.

## 13.4 Panic handler with defmt

```rust
use panic_probe as _;

// On panic:
// Prints panic location + message via defmt
// Then waits for reset
```

`panic-probe`:
- Hooks panic
- Outputs message via defmt
- Better than `panic-halt` (silent) or `panic-semihosting` (slow)

## 13.5 GDB debugging

```bash
# Start GDB server:
probe-rs gdb-server --chip STM32F411RETx

# Connect GDB:
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/myapp
(gdb) target remote :1337
(gdb) break main
(gdb) continue
(gdb) info locals
(gdb) print my_var
```

Set breakpoints, step through code, inspect variables. Just like host debugging.

`vscode` + `cortex-debug` extension → integrated debugging UI.

## 13.6 Logic analyzer / oscilloscope

For hardware behavior:
- GPIO toggle timing
- SPI / I2C / UART signals
- Interrupt latency

Tools: Saleae Logic, Sigrok, oscilloscope.

`defmt::info!` only sees what code does. Logic analyzer sees what hardware actually does.

## 13.7 Common debugging patterns

### Pattern 1: Toggle pin in IRQ
```rust
#[interrupt]
fn TIM2() {
    // Set debug pin high at entry, low at exit
    DEBUG_PIN.toggle();
    // ... handler ...
}
```

Measure with logic analyzer → IRQ latency.

### Pattern 2: Profile sections
```rust
let start = cortex_m::peripheral::DWT::cycle_count();
expensive_fn();
let elapsed = cortex_m::peripheral::DWT::cycle_count() - start;
defmt::info!("took {} cycles", elapsed);
```

DWT cycle counter measures down to single cycle.

### Pattern 3: Catch HardFault
```rust
#[exception]
unsafe fn HardFault(ef: &cortex_m_rt::ExceptionFrame) -> ! {
    defmt::error!("HardFault! PC: {:08x}", ef.pc());
    loop {}
}
```

Print PC where fault happened. Look up in disassembly.

---

# Tầng 14: Power & low-power patterns

## 14.1 Why power matters

Battery devices (sensor, wearable, IoT):
- Active: 10-100 mA
- Sleep: 1-100 µA (10000x less)

Achieve months/years battery → must sleep most of time.

## 14.2 Sleep modes

ARM Cortex-M:
- **Sleep**: CPU stops, peripherals active. Wake on IRQ.
- **Deep sleep**: more aggressive, faster wake.
- **Stop / Standby**: clocks off, RAM retained. Wake by RTC / EXTI.

```rust
loop {
    do_work();
    cortex_m::asm::wfi();   // sleep until next IRQ
}
```

WFI = Wait For Interrupt. CPU off until something happens.

## 14.3 Embassy + low power

Embassy WFI between polls:
```rust
loop {
    timer.wait().await;   // sleep until timer
    led.toggle();
}
```

Cooperative scheduling + WFI → naturally low power.

## 14.4 Peripheral on/off

```rust
// Turn off unused peripherals:
rcc.ahb1enr.modify(|_, w| w.gpioben().clear_bit());  // disable GPIOB clock
```

Per-peripheral clock gating saves power.

## 14.5 Optimize wake-up frequency

```rust
// Bad: wake every 1ms to check sensor (overkill)
Timer::after_millis(1).await;
read_sensor();

// Good: read at sensor's actual update rate
Timer::after_secs(10).await;
read_sensor();
```

Lower frequency = more sleep time = better battery.

## 14.6 Power profiling

Tools:
- Power Profiler Kit (Nordic) — measure current
- INA219 / INA226 chips — DIY current sensor
- `defmt::info!` battery voltage periodically

Profile under realistic load → optimize hot spots.

---

# Tầng 15: Patterns và Antipatterns

## 15.1 ✅ Pattern: Bounded queues

```rust
use heapless::spsc::Queue;

static mut QUEUE: Queue<u32, 16> = Queue::new();
```

Fixed-size SPSC (Single Producer Single Consumer) queue. No alloc. Bounded latency.

## 15.2 ✅ Pattern: Type-state for protocol

```rust
struct ModbusFrame<S> { ... }

struct Building;
struct Complete;

impl ModbusFrame<Building> {
    fn add_data(self, d: u8) -> Self { ... }
    fn finalize(self) -> ModbusFrame<Complete> { ... }
}

impl ModbusFrame<Complete> {
    fn send(self, uart: &mut Uart) { ... }
}
```

Can't send incomplete frame. Compile-time guarantee.

## 15.3 ✅ Pattern: Resource handles

```rust
let p = pac::Peripherals::take().unwrap();
let gpio = p.GPIOA;     // owns GPIOA
let led_pin = gpio.split().pa5.into_output();   // owns just pin 5
```

Ownership of hardware resources → compile-time conflict prevention.

## 15.4 ✅ Pattern: Watchdog feed in main loop

```rust
loop {
    do_task();
    watchdog.feed();
}
```

If `do_task` hangs > timeout → MCU reset. Recovery without manual intervention.

## 15.5 ✅ Pattern: defmt logging

```rust
defmt::info!("starting");
defmt::error!("failed: {:?}", err);
```

Cheap, structured. Replace `println!` thinking.

## 15.6 ❌ Antipattern: Use heap for everything

```rust
let v: Vec<u8> = read_data();   // alloc — slow + fragmentation
let s = format!("Got {}", n);    // alloc + format
```

Embedded: prefer stack arrays + heapless. Save heap for special cases.

## 15.7 ❌ Antipattern: Long IRQ handlers

```rust
#[interrupt]
fn TIM2() {
    for i in 0..1000 {        // ❌ slow IRQ
        complex_calc(i);
    }
}
```

Long IRQ blocks lower-priority IRQs → missed deadlines.

✅ Defer to lower-priority task:
```rust
#[interrupt]
fn TIM2() {
    flag.store(true, Ordering::Relaxed);   // signal main
    clear_irq();
}

fn main() {
    if flag.load(Ordering::Relaxed) {
        complex_calc();
        flag.store(false, Ordering::Relaxed);
    }
}
```

## 15.8 ❌ Antipattern: panic in production

```rust
let v: u32 = some_fn();
let x = v.checked_div(divisor).unwrap();   // ❌ panic on 0
```

Panic = MCU halt or reset. In production: handle error gracefully.

✅ Pattern:
```rust
let x = v.checked_div(divisor).unwrap_or(DEFAULT);
// Or
let Some(x) = v.checked_div(divisor) else {
    log_error();
    return;
};
```

## 15.9 ❌ Antipattern: Floating-point in IRQ on no-FPU

```rust
#[interrupt]
fn TIM2() {
    let avg: f32 = total / count;   // ❌ slow software emulation
}
```

On Cortex-M0/M3 (no FPU): float = software emulation, hundreds of cycles. IRQ deadline missed.

✅ Use integer math, fixed-point, or move out of IRQ.

## 15.10 ❌ Antipattern: Blocking in async (embassy)

```rust
#[embassy_executor::task]
async fn task() {
    loop {
        let _ = read_blocking();   // ❌ blocks executor!
    }
}
```

Embassy executor cooperative — long sync work blocks other tasks.

✅ Use async version:
```rust
let data = read_async().await;
```

Or `embassy::task::yield_now().await` between heavy chunks.

## 15.11 ❌ Antipattern: Static mut without sync

```rust
static mut FLAG: bool = false;

#[interrupt]
fn TIM2() {
    unsafe { FLAG = true; }   // ❌ race with main
}

fn main() {
    unsafe {
        if FLAG { ... }       // ❌ race
    }
}
```

✅ Use atomic:
```rust
static FLAG: AtomicBool = AtomicBool::new(false);
FLAG.store(true, Ordering::Relaxed);
FLAG.load(Ordering::Relaxed);
```

## 15.12 ❌ Antipattern: Ignoring stack overflow

```rust
fn recursive(depth: u32) {
    if depth > 0 { recursive(depth - 1); }
}
```

Deep recursion → stack overflow → silent corruption.

Embedded stack ~4KB. Tools:
- `cargo call-stack` — estimate
- Linker check (newer cortex-m-rt has stack overflow detection)

---

# Tổng kết — 12 nguyên tắc senior

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. #![no_std] strip stdlib. Use core + alloc (optional) + heapless│
│                                                                  │
│ 2. Memory: stack (KB), heap optional. Audit usage.               │
│                                                                  │
│ 3. PAC → HAL → BSP. Use highest convenient layer.                │
│                                                                  │
│ 4. embedded-hal traits → portable drivers across MCUs.           │
│                                                                  │
│ 5. Type-state pattern: hardware misuse = compile error.          │
│                                                                  │
│ 6. IRQ: short, atomic state, defer to tasks.                     │
│                                                                  │
│ 7. RTIC for hard real-time. Embassy for async style.             │
│                                                                  │
│ 8. DMA for bulk transfer, free CPU.                              │
│                                                                  │
│ 9. defmt + probe-rs + RTT for debugging.                         │
│                                                                  │
│ 10. Watchdog as safety net.                                      │
│                                                                  │
│ 11. Low-power: WFI, peripheral clock gating, optimize wake freq. │
│                                                                  │
│ 12. Avoid panic in production. Use Result. Plan for failure.     │
└──────────────────────────────────────────────────────────────────┘
```

---

# Khi Rust embedded vẫn hard

- Vendor SDK chỉ có C — `bindgen` cho rough bindings
- Some MCU PAC chưa generated → mất công viết
- Cross-debug khó hơn host
- Compile-flash-test cycle ~5-10s
- Certain certifications (DO-178C) chưa qualify Rust

→ Workaround: bridge to C library, contribute PAC, qualify toolchain (Ferrocene).

---

# Embedded Rust toolkit

| Crate / Tool | Purpose |
|--------------|---------|
| `cortex-m`, `cortex-m-rt` | ARM Cortex-M support |
| `riscv`, `riscv-rt` | RISC-V support |
| `embedded-hal` | Trait ecosystem |
| `embedded-hal-async` | Async traits |
| `heapless` | Stack-only collections |
| `nb` | Non-blocking abstraction |
| `defmt` | Logging |
| `defmt-rtt` | RTT transport |
| `panic-probe` | Panic via defmt |
| `probe-rs` | Flasher/debugger |
| `critical-section` | Cross-platform critical sections |
| `embassy-*` | Async framework |
| `rtic` | Real-time framework |
| `embedded-alloc` | Allocator (if needed) |
| `embedded-graphics` | 2D graphics |
| `smoltcp` | TCP/IP no_std stack |
| `usbd-*` | USB device |

---

# Lộ trình tiếp theo

Bạn đã có 16 chủ đề:

```
1. memory-model      9. smart-pointers
2. ownership-borrow 10. lifetime
3. trait            11. performance
4. generic          12. observability
5. closure          13. iterator
6. async            14. unsafe-rust
7. error-handling   15. testing
8. macros           16. embedded-rust   ← MỚI
```

Còn 2 topic ứng dụng cuối:

- **Web framework realistic** — axum project apply 16 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool

Báo cái nào muốn đào sâu! 🦀⚡
