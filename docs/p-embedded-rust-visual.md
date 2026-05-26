# Embedded Rust — Minh Hoạ Trực Quan

> Companion visual cho [embedded-rust.md](./embedded-rust.md). Đọc song song.

---

## 1. Bức tranh lớn — Embedded Rust Universe

```
                          EMBEDDED RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   Target: MCU (KB-MB RAM, MHz CPU)                     │
       │   No OS, often no heap, real-time constraints          │
       │                                                        │
       │   ┌──────────────────┐                                 │
       │   │  Application      │  ← your code                   │
       │   └────────┬─────────┘                                 │
       │            │ use traits from ↓                         │
       │   ┌──────────────────┐                                 │
       │   │ Framework         │  rtic / embassy / bare loop    │
       │   └────────┬─────────┘                                 │
       │            │                                           │
       │   ┌──────────────────┐                                 │
       │   │ BSP               │  ← board-specific (LEDs, btns) │
       │   └────────┬─────────┘                                 │
       │            │                                           │
       │   ┌──────────────────┐                                 │
       │   │ HAL               │  ← chip-family API (ergonomic) │
       │   └────────┬─────────┘                                 │
       │            │                                           │
       │   ┌──────────────────┐                                 │
       │   │ PAC               │  ← auto from SVD (raw regs)    │
       │   └────────┬─────────┘                                 │
       │            │                                           │
       │   ┌──────────────────┐                                 │
       │   │ Runtime           │  ← cortex-m-rt                 │
       │   └────────┬─────────┘                                 │
       │            │                                           │
       │   ┌──────────────────┐                                 │
       │   │ HARDWARE          │  ← MCU silicon                 │
       │   └──────────────────┘                                 │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Spectrum — Big vs Small

```
   Server  PC   Phone    RPi      Cortex-A     Cortex-M    ATtiny
   ──────  ──   ─────   ─────    ────────     ────────   ──────
   16GB+   8GB  4GB     1GB      128MB        64-512KB   1-8KB
   ↑                              ↑            ↑
   Linux + std Rust         Linux/RTOS    no_std Rust
                            std Rust       Bare metal / embassy
   
   
   Embedded "sweet spot" cho Rust = Cortex-M class:
   ────────────────────────────────────────────────
   • Vài MHz đến 480 MHz
   • 16KB → 512KB RAM
   • 64KB → 2MB Flash
   • ARM Cortex-M0/M3/M4/M7/M33
   • Examples: STM32, nRF52, RP2040, ESP32
```

---

## 3. std vs core vs alloc

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   std                  Needs: OS + heap                  │
   │   ┌─────────────┐                                        │
   │   │ thread, file│   Use on Linux/Windows embedded         │
   │   │ net, io     │                                        │
   │   │ time        │                                        │
   │   └─────────────┘                                        │
   │         ▲                                                │
   │         │ depends on                                     │
   │   ┌─────────────┐                                        │
   │   │ alloc       │   Needs: allocator                     │
   │   │ Vec, String │   Use if you have memory for heap      │
   │   │ Box, Rc     │                                        │
   │   └─────────────┘                                        │
   │         ▲                                                │
   │         │ depends on                                     │
   │   ┌─────────────┐                                        │
   │   │ core        │   No OS, no heap                       │
   │   │ Option      │   Always available                     │
   │   │ Iterator    │                                        │
   │   │ atomic      │                                        │
   │   │ slice/array │                                        │
   │   └─────────────┘                                        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Embedded common case: core + heapless
   ──────────────────────────────────────
   
   #![no_std]
   use heapless::Vec;           ← stack-allocated, fixed-size
   let mut v: Vec<u8, 32> = Vec::new();
   v.push(1).unwrap();
```

---

## 4. Memory layout — MCU

```
   ┌─────────────────────────────────────────────────────┐
   │                                                     │
   │   FLASH (e.g., 512KB) — non-volatile                │
   │   ┌─────────────────────────────────┐               │
   │   │ Vector table  (0x0000000)       │ ← IRQ handlers│
   │   │ ─────────                       │               │
   │   │ .text         (your code)       │ ← functions  │
   │   │ ─────────                       │               │
   │   │ .rodata       (string literals) │ ← const data │
   │   │               (lookup tables)   │               │
   │   └─────────────────────────────────┘               │
   │                                                     │
   │   RAM (e.g., 128KB) — volatile                      │
   │   ┌─────────────────────────────────┐               │
   │   │ .data         (init vars)       │ ← copied from│
   │   │               static FOO = 42   │   FLASH at  │
   │   │ ─────────                       │   boot       │
   │   │ .bss          (zero-init vars)  │ ← zeroed     │
   │   │               static BAR = 0    │               │
   │   │ ─────────                       │               │
   │   │   ↓ heap (optional, grows ↓)    │               │
   │   │                                 │               │
   │   │      (free space)                │               │
   │   │                                 │               │
   │   │   ↑ STACK (grows ↑)             │               │
   │   │     [4-16KB typical]             │               │
   │   └─────────────────────────────────┘               │
   │                                                     │
   │   PERIPHERALS (0x4000_0000+)                        │
   │   ┌─────────────────────────────────┐               │
   │   │ Memory-mapped registers         │ ← GPIO, UART,│
   │   │  (read/write to control HW)     │   SPI, etc.  │
   │   └─────────────────────────────────┘               │
   │                                                     │
   └─────────────────────────────────────────────────────┘
   
   
   memory.x (linker script):
   ─────────────────────────
   
   MEMORY {
     FLASH : ORIGIN = 0x08000000, LENGTH = 512K
     RAM   : ORIGIN = 0x20000000, LENGTH = 128K
   }
```

---

## 5. Embedded ecosystem stack

```
   ┌────────────────────────────────────────────────────────────┐
   │                                                            │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  Application code                            │         │
   │   │                                              │         │
   │   │  fn main() {                                 │         │
   │   │      led.blink(1.Hz()).await;                │         │
   │   │  }                                           │         │
   │   └──────────────────────────────────────────────┘         │
   │                       │                                    │
   │                       ▼ uses                               │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  BSP (Board Support Package)                 │         │
   │   │                                              │         │
   │   │  let board = NucleoF411::take();              │        │
   │   │  let led = board.led_user;                   │         │
   │   └──────────────────────────────────────────────┘         │
   │                       │                                    │
   │                       ▼ uses                               │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  HAL (Hardware Abstraction Layer)            │         │
   │   │                                              │         │
   │   │  let mut led = gpioa.pa5                     │         │
   │   │      .into_push_pull_output();               │         │
   │   │  led.set_high();                             │         │
   │   └──────────────────────────────────────────────┘         │
   │                       │                                    │
   │                       ▼ uses                               │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  PAC (Peripheral Access Crate)               │         │
   │   │                                              │         │
   │   │  p.GPIOA.moder.modify(|_, w|                 │         │
   │   │      w.moder5().output());                   │         │
   │   │  p.GPIOA.odr.modify(|_, w|                   │         │
   │   │      w.odr5().set_bit());                    │         │
   │   └──────────────────────────────────────────────┘         │
   │                       │                                    │
   │                       ▼ generated from                     │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  SVD (vendor XML)                            │         │
   │   │                                              │         │
   │   │  <peripheral name="GPIOA"                    │         │
   │   │   baseAddress="0x40020000">                  │         │
   │   │    <register>                                │         │
   │   │      <name>MODER</name> ...                  │         │
   │   │    </register>                               │         │
   │   │  </peripheral>                               │         │
   │   └──────────────────────────────────────────────┘         │
   │                       │                                    │
   │                       ▼                                    │
   │   ┌──────────────────────────────────────────────┐         │
   │   │  HARDWARE — MCU silicon                      │         │
   │   └──────────────────────────────────────────────┘         │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
```

---

## 6. embedded-hal — Trait ecosystem

```
   ┌──────────────────────────────────────────────────────────┐
   │  Without embedded-hal:                                   │
   │                                                          │
   │  STM32 sensor driver ──❌── nRF52 sensor driver         │
   │  (incompatible APIs)                                     │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │  With embedded-hal traits:                               │
   │                                                          │
   │   ┌─────────────────┐                                    │
   │   │ Sensor driver   │                                    │
   │   │ (e.g., BME280)  │                                    │
   │   │ Generic over    │                                    │
   │   │ I2C trait       │                                    │
   │   └────────┬────────┘                                    │
   │            │                                             │
   │            ▼ uses                                        │
   │   ┌─────────────────────────────────────┐                │
   │   │ embedded_hal::i2c::I2c trait        │                │
   │   └────────┬──────────────┬─────────────┘                │
   │            │              │                              │
   │      impl on   ↓     ↓ impl on                           │
   │   ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
   │   │ stm32-hal    │ │ nrf52-hal    │ │ rp-hal       │    │
   │   │ I2C          │ │ TWIM         │ │ I2C          │    │
   │   └──────────────┘ └──────────────┘ └──────────────┘    │
   │                                                          │
   │  ⟹ Same sensor driver works on ALL these MCUs!          │
   └──────────────────────────────────────────────────────────┘
   
   
   Standard traits:
   ────────────────
   
   GPIO:     OutputPin, InputPin
   SPI:      SpiDevice, SpiBus
   I2C:      I2c
   UART:     Read, Write
   Timer:    DelayMs, DelayUs
   PWM:      SetDutyCycle
   ADC:      OneShot
```

---

## 7. Type-state pattern — Compile-time hardware safety

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   State transitions encoded in TYPE:                     │
   │                                                          │
   │   ┌──────────────┐                                       │
   │   │ Pin<Disabled>│  ← initial state                      │
   │   └──────┬───────┘                                       │
   │          │                                               │
   │          ├── .into_push_pull_output() ──►                │
   │          │   ┌──────────────────────┐                    │
   │          │   │ Pin<Output<PushPull>>│                    │
   │          │   │   .set_high()  ✅    │                    │
   │          │   │   .set_low()   ✅    │                    │
   │          │   │   .is_high()   ❌    │                    │
   │          │   └──────────────────────┘                    │
   │          │                                               │
   │          ├── .into_floating_input() ──►                  │
   │          │   ┌──────────────────────┐                    │
   │          │   │ Pin<Input<Floating>> │                    │
   │          │   │   .is_high()   ✅    │                    │
   │          │   │   .set_high()  ❌    │ ← compile error!  │
   │          │   └──────────────────────┘                    │
   │          │                                               │
   │          └── .into_alternate::<AF1>() ──►                │
   │              ┌──────────────────────┐                    │
   │              │ Pin<Alternate<AF1>>  │                    │
   │              │   (for SPI, UART...) │                    │
   │              └──────────────────────┘                    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   ⟹ Hardware misuse = COMPILE ERROR, not runtime crash
```

---

## 8. Interrupts — Hardware events flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Normal execution                                       │
   │            │                                             │
   │            │ HARDWARE EVENT                              │
   │            │ (timer expire, GPIO toggle, UART data)      │
   │            ▼                                             │
   │   ┌─────────────────────────┐                            │
   │   │  CPU action:            │                            │
   │   │  1. Save current PC,    │ ← context save             │
   │   │     registers, flags    │                            │
   │   │  2. Lookup vector table │                            │
   │   │  3. Jump to handler     │                            │
   │   └─────────┬───────────────┘                            │
   │             │                                            │
   │             ▼                                            │
   │   ┌─────────────────────────┐                            │
   │   │  #[interrupt]           │                            │
   │   │  fn TIM2() {            │                            │
   │   │      // 1. Clear flag   │                            │
   │   │      // 2. Quick work   │                            │
   │   │      // 3. Signal main  │                            │
   │   │  }                      │                            │
   │   └─────────┬───────────────┘                            │
   │             │                                            │
   │             ▼ return                                     │
   │   ┌─────────────────────────┐                            │
   │   │  CPU action:            │                            │
   │   │  1. Restore PC, regs    │                            │
   │   │  2. Resume normal code  │                            │
   │   └─────────────────────────┘                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   IRQ rules:
   ──────────
   ✅ Short (microseconds)
   ✅ Clear interrupt flag (else fires forever!)
   ✅ Signal main with atomic / queue
   
   ❌ NO long computation
   ❌ NO println / defmt::info! (slow)
   ❌ NO heap alloc
   ❌ NO float on no-FPU MCU
   ❌ NO wait/poll for peripheral
   
   
   Sharing data with main:
   ───────────────────────
   
   ❌ static mut COUNT  (race)
   ✅ AtomicU32          (atomic single var)
   ✅ critical_section + Mutex<RefCell<T>>  (complex data)
```

---

## 9. RTIC framework

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   #[rtic::app(device = pac, peripherals = true)]         │
   │   mod app {                                              │
   │                                                          │
   │     #[shared] struct Shared {                            │
   │         counter: u32,                                    │
   │     }                                                    │
   │                                                          │
   │     #[local] struct Local {                              │
   │         led: Pin<Output>,                                │
   │     }                                                    │
   │                                                          │
   │     #[init]                                              │
   │     fn init(ctx) -> (Shared, Local) {                    │
   │         // setup hardware                                │
   │         (Shared { counter: 0 }, Local { led })           │
   │     }                                                    │
   │                                                          │
   │     #[task(binds = TIM2, priority = 1,                   │
   │            shared = [counter], local = [led])]           │
   │     fn timer_task(mut ctx) {                             │
   │         ctx.local.led.toggle();              ← local: no lock│
   │         ctx.shared.counter.lock(|c| *c += 1); ← auto-lock│
   │     }                                                    │
   │                                                          │
   │     #[idle]                                              │
   │     fn idle(_) -> ! {                                    │
   │         loop { cortex_m::asm::wfi(); }                   │
   │     }                                                    │
   │                                                          │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Priority + lock mechanism:
   ──────────────────────────
   
   Task A (prio 1)        Task B (prio 2 — higher)
   ─────────────────────   ─────────────────────────
   
   .lock(counter) →
   raise NVIC prio to 2    (B can't preempt here!)
   |
   |  access counter
   |
   end lock →
   restore prio to 1       Now B can preempt
   |
   |                         (B runs)
   |                       .lock(counter) →
   |                       NO actual lock — already top prio
   |                       |
   |                       |  access counter
   |                       end lock
   |                        ↓
   |                       (B done)
   continue
   
   
   ⟹ Compile-time analysis ensures lock correctness
   ⟹ No deadlock possible (priority ceiling)
```

---

## 10. Embassy — Async embedded

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   #[embassy_executor::main]                              │
   │   async fn main(spawner: Spawner) {                      │
   │     let p = embassy_stm32::init(...);                    │
   │                                                          │
   │     spawner.spawn(blink_task(p.PA5)).unwrap();           │
   │     spawner.spawn(sensor_task(p.I2C1, ...)).unwrap();    │
   │     spawner.spawn(uart_task(p.USART1, ...)).unwrap();    │
   │   }                                                      │
   │                                                          │
   │   #[embassy_executor::task]                              │
   │   async fn blink_task(mut led: Output) {                 │
   │     loop {                                               │
   │       led.toggle();                                      │
   │       Timer::after_millis(500).await;  ← yields!         │
   │     }                                                    │
   │   }                                                      │
   │                                                          │
   │   #[embassy_executor::task]                              │
   │   async fn sensor_task(mut i2c: I2c) {                   │
   │     loop {                                               │
   │       let v = read_temp(&mut i2c).await;                 │
   │       defmt::info!("Temp: {}", v);                       │
   │       Timer::after_secs(10).await;                       │
   │     }                                                    │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Executor architecture:
   ──────────────────────
   
   ┌─────────────────────────────────────────┐
   │ Embassy Executor                        │
   │                                         │
   │  Task pool (pre-allocated futures):     │
   │  ┌────────────────────────────────┐     │
   │  │ blink_task: state machine       │    │
   │  │ sensor_task: state machine      │    │
   │  │ uart_task: state machine        │    │
   │  └────────────────────────────────┘     │
   │                                         │
   │  Ready queue: [tasks to poll]           │
   │                                         │
   │  Loop:                                  │
   │    - Poll ready tasks                   │
   │    - When all idle: WFI (CPU sleep)     │
   │    - Hardware IRQ wakes affected task   │
   │    - Push to ready queue                │
   └─────────────────────────────────────────┘
   
   
   Tasks share SINGLE stack (state machine in RAM).
   No per-task stack overhead → fit in tiny MCU.
   
   
   Embassy vs RTIC:
   ─────────────────
   
   ┌───────────────┬─────────────────┬─────────────────┐
   │               │ Embassy         │ RTIC            │
   ├───────────────┼─────────────────┼─────────────────┤
   │ Style         │ async/await     │ Task + IRQ      │
   │ Concurrency   │ Cooperative     │ Preemptive      │
   │ Real-time     │ Harder          │ Easier          │
   │ Familiarity   │ Like tokio      │ Embedded-only   │
   │ Learning curve│ Easier (if async│ Steeper         │
   │               │  background)    │                 │
   │ Best for      │ Prototyping,    │ Hard real-time, │
   │               │ IoT, modern     │ industrial      │
   └───────────────┴─────────────────┴─────────────────┘
```

---

## 11. DMA — Direct Memory Access

```
   ┌────────────────────────────────────────────────────────┐
   │  Without DMA:                                          │
   │                                                        │
   │   UART byte arrives                                    │
   │     ↓                                                  │
   │   IRQ fires                                            │
   │     ↓                                                  │
   │   CPU copies byte → buffer                             │
   │     ↓                                                  │
   │   CPU clear flag                                       │
   │     ↓                                                  │
   │   CPU returns                                          │
   │                                                        │
   │   ⟹ CPU involved EVERY byte. At 1Mbps = 1M IRQs/sec    │
   │                                                        │
   ├────────────────────────────────────────────────────────┤
   │  With DMA:                                             │
   │                                                        │
   │   Setup once:                                          │
   │     dma.setup(uart, &mut buf, 1024);                   │
   │                                                        │
   │   ┌─────────┐         ┌─────────┐                      │
   │   │ UART    │ ──byte→ │  DMA    │ ──byte→ ┌──────┐    │
   │   │ peripheral       │ engine  │         │ RAM  │    │
   │   └─────────┘         └─────────┘         │ buf  │    │
   │                            │              └──────┘    │
   │                            │ when 1024 bytes done:    │
   │                            ▼                          │
   │                       IRQ fires once                  │
   │                                                        │
   │   ⟹ CPU free during transfer                          │
   │   ⟹ 1 IRQ instead of 1M                                │
   │                                                        │
   └────────────────────────────────────────────────────────┘
   
   
   Async DMA with embassy:
   ───────────────────────
   
   let mut buf = [0u8; 1024];
   uart.read(&mut buf).await;   ← CPU sleeps via WFI
   //                ▲
   //                DMA + IRQ + task wake handled
   //                automatically by embassy
   
   process(&buf);
   
   
   Circular DMA (double-buffering):
   ────────────────────────────────
   
   Buffer:  [─────────────────────────────]
            ▲                              ▲
            │                              │
            │ DMA writes here (half 1)     │
            │                              │
            ▼ half-full IRQ                │
   Main reads half 1                       │
                                           ▼
                                  DMA writes here (half 2)
                                           │
                                           ▼ full IRQ
                                  Main reads half 2
                                  DMA wraps to half 1
                                  (continuous stream)
```

---

## 12. Real-time concepts

```
   ┌────────────────────────────────────────────────────────────┐
   │                                                            │
   │   HARD Real-Time:                                          │
   │   ────────────                                             │
   │   Missing deadline = SYSTEM FAILURE                        │
   │   Examples: airbag, pacemaker, motor controller            │
   │                                                            │
   │   ┌──────────────────────────────────┐                     │
   │   │ Deadline                          │                    │
   │   │     │                             │                    │
   │   │     ▼                             │                    │
   │   │  ███████░░░░░░  ← task done early │ ✅                 │
   │   │     │                             │                    │
   │   │  ███████████░░  ← just in time    │ ✅                 │
   │   │     │                             │                    │
   │   │  █████████████████  ← MISSED!     │ ❌ SYSTEM FAIL    │
   │   └──────────────────────────────────┘                     │
   │                                                            │
   ├────────────────────────────────────────────────────────────┤
   │                                                            │
   │   SOFT Real-Time:                                          │
   │   ────────────                                             │
   │   Miss deadline = quality degraded                         │
   │   Examples: audio (glitch), UI (lag), video (frame drop)   │
   │                                                            │
   │   Acceptable: 1% misses → user-tolerable                   │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
   
   
   WCET — Worst-Case Execution Time:
   ─────────────────────────────────
   
   Distribution of execution times:
   
   freq │     ████
        │   ████████
        │  ██████████
        │ ████████████░░
        │              ░░ ← rare worst case
        └─────────────────────────►  time
        Average        WCET
   
   For hard real-time: design for WCET, not average!
   
   
   What hurts predictability:
   ──────────────────────────
   
   ❌ Heap alloc (varies — fragmentation)
   ❌ Recursive functions (depth depends on data)
   ❌ Loops with input-dependent count
   ❌ Cache misses (data-dependent)
   ❌ Floating-point on no-FPU MCU
   ❌ Long IRQ blocking other IRQs
   
   ✅ Stack arrays (size known)
   ✅ Bounded loops (known iterations)
   ✅ Integer math
   ✅ Type-state design
```

---

## 13. Debugging stack

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   YOUR APP                                               │
   │       │                                                  │
   │       │ defmt::info!("count = {}", n);                   │
   │       ▼                                                  │
   │   ┌────────────────────────────────────────┐             │
   │   │ defmt — compress format strings        │             │
   │   │ ───────                                │             │
   │   │ At compile, "count = {}" → ID 0x42     │             │
   │   │ At runtime, send: [0x42, n bytes]      │             │
   │   │ → Fast (~µs vs ms for println)         │             │
   │   └─────────┬──────────────────────────────┘             │
   │             │                                            │
   │             ▼                                            │
   │   ┌────────────────────────────────────────┐             │
   │   │ defmt-rtt — transport                  │             │
   │   │ ──────────                              │             │
   │   │ Write to RTT buffer in MCU RAM          │             │
   │   │ Non-blocking, very fast                 │             │
   │   └─────────┬──────────────────────────────┘             │
   │             │                                            │
   │             ▼  via debug interface                       │
   │   ┌────────────────────────────────────────┐             │
   │   │ probe-rs (host side)                   │             │
   │   │ ──────                                  │             │
   │   │ Read RTT, decode IDs via ELF symbols    │             │
   │   │ Re-render: "count = 42"                 │             │
   │   └─────────┬──────────────────────────────┘             │
   │             │                                            │
   │             ▼                                            │
   │   HOST TERMINAL                                          │
   │   INFO myapp: count = 42                                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Hardware debug interface:
   ─────────────────────────
   
   ┌──────────┐  SWD pins  ┌──────────┐  USB  ┌──────────┐
   │ Computer ├────────────┤ Debugger ├───────┤ MCU      │
   │ probe-rs │ (4 wires)  │ ST-Link  │       │ (target) │
   │          │            │ J-Link   │       │          │
   └──────────┘            └──────────┘       └──────────┘
   
   Commands:
   • probe-rs flash myapp        — write to flash
   • probe-rs run myapp          — flash + run + stream defmt
   • probe-rs gdb-server         — attach GDB
   • probe-rs reset              — reset MCU
```

---

## 14. Power optimization

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Power profile of typical sensor node:                 │
   │                                                          │
   │   Current (mA)                                           │
   │      ▲                                                   │
   │   10 │  █                                                │
   │      │  █              █                                 │
   │    1 │  █              █                                 │
   │      │  █              █                                 │
   │  0.1 │  █              █                                 │
   │      │  █              █                                 │
   │ 0.01 │  █▓▓▓▓▓▓▓▓▓▓▓▓▓▓█▓▓▓▓▓▓▓▓▓▓▓▓▓▓                  │
   │      └──┴──────────────┴────────────────►  time         │
   │          read sensor    transmit                          │
   │         (active 10ms)  (active 20ms)                     │
   │                                                          │
   │   ⟹ 99% of time SLEEPING                                │
   │      Battery life depends on sleep current               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Sleep modes (Cortex-M):
   ──────────────────────
   
   ┌────────────┬──────────────────┬──────────────┐
   │ Mode       │ State            │ Wake source  │
   ├────────────┼──────────────────┼──────────────┤
   │ Run        │ CPU + periph on  │ —            │
   │ Sleep      │ CPU off,         │ Any IRQ      │
   │            │ periph on        │              │
   │ Stop       │ Most clocks off  │ RTC, EXTI    │
   │ Standby    │ Almost all off,  │ Wake-up pin, │
   │            │ RAM lost         │ RTC          │
   └────────────┴──────────────────┴──────────────┘
   
   
   Code pattern:
   ─────────────
   
   loop {
       do_work();
       cortex_m::asm::wfi();   ← CPU off until next IRQ
   }
   
   // Embassy automatically WFI when all tasks idle
```

---

## 15. Common patterns visualization

```
   ✅ Bounded queue (lock-free SPSC):
   ─────────────────────────────────
   
   use heapless::spsc::{Producer, Consumer};
   
   IRQ producer ──► Queue (16 slots) ──► Main consumer
                        ▲
                        │ no lock needed
                        │ atomic indices
                        │
                  ┌──┬──┬──┬──┬──┬──┐
                  │  │  │X │X │X │  │  ← capacity 16
                  └──┴──┴──┴──┴──┴──┘
                        ↑     ↑
                     consumer producer
                     (main)   (IRQ)
   
   
   ✅ Watchdog feed pattern:
   ──────────────────────────
   
   loop {
     task1();
     task2();
     task3();
     watchdog.feed();  ← reset watchdog timer
   }
   
   If loop hangs > timeout → MCU auto-reset
   Recovery without manual intervention.
   
   
   ✅ Type-state protocol:
   ───────────────────────
   
   let frame = ModbusFrame::new(addr);
   let frame = frame.add_data(0x12);
   let frame = frame.add_data(0x34);
   let frame = frame.finalize();    ← Building → Complete
   frame.send(&mut uart);            ← only Complete can send
   
   Incomplete frame = compile error.
```

---

## 16. Antipatterns

```
   ❌ 1. Heap alloc trong hot path
   ───────────────────────────────
   
   #[interrupt]
   fn UART_RX() {
       let v: Vec<u8> = read_data();   // alloc!
       process(&v);
   }
   
   ⟹ Slow + unpredictable timing
   
   ✅ Stack array hoặc heapless:
   #[interrupt]
   fn UART_RX() {
       let mut buf = [0u8; 64];
       let n = read_into(&mut buf);
       process(&buf[..n]);
   }
   
   
   ❌ 2. Long IRQ handler
   ──────────────────────
   
   #[interrupt]
   fn TIM2() {
       for i in 0..1000 {
           complex_calc(i);   // ❌ blocks other IRQs
       }
   }
   
   ✅ Quick IRQ, defer to task:
   #[interrupt]
   fn TIM2() {
       FLAG.store(true, Ordering::Relaxed);  // signal
       clear_irq();
   }
   
   // Main loop or low-priority task does work
   
   
   ❌ 3. Float trên MCU no-FPU
   ────────────────────────────
   
   #[interrupt]
   fn TIM2() {
       let avg: f32 = total as f32 / count as f32;  // SLOW emulation
   }
   
   ✅ Integer math:
   let avg = (total * 1000) / count;  // fixed-point
   
   
   ❌ 4. panic in production
   ─────────────────────────
   
   let x = v.unwrap();  // panic → MCU stuck / reset
   
   ✅ Handle gracefully:
   match v {
       Some(x) => use_it(x),
       None => log_and_recover(),
   }
   
   
   ❌ 5. static mut without atomic
   ───────────────────────────────
   
   static mut FLAG: bool = false;
   
   #[interrupt]
   fn TIM2() { unsafe { FLAG = true; } }   // race
   
   ✅ Atomic:
   static FLAG: AtomicBool = AtomicBool::new(false);
   FLAG.store(true, Ordering::Relaxed);
   
   
   ❌ 6. Blocking in embassy async
   ───────────────────────────────
   
   #[embassy_executor::task]
   async fn task() {
       loop {
           let _ = read_blocking();  // ❌ blocks executor
       }
   }
   
   ✅ Async or yield:
   let data = read_async().await;
   // Or:
   embassy_futures::yield_now().await;
```

---

## 17. Tools matrix

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  CROSS-COMPILE                BUILD                      │
   │  ─────────────                ─────                      │
   │  rustup target add ...        cargo build --release      │
   │  thumbv7em-none-eabihf        cargo bloat                │
   │  riscv32imc-unknown-none-elf                             │
   │                                                          │
   │  FLASH + RUN                  DEBUG                      │
   │  ───────────                  ─────                      │
   │  probe-rs run                 probe-rs gdb-server        │
   │  probe-rs flash               arm-none-eabi-gdb          │
   │                               VS Code + cortex-debug     │
   │                                                          │
   │  LOGGING                      ANALYSIS                   │
   │  ─────                        ────────                   │
   │  defmt                        cargo call-stack           │
   │  defmt-rtt                    cargo size                 │
   │  defmt-print                  Logic analyzer (Saleae)    │
   │                               Oscilloscope               │
   │                                                          │
   │  FRAMEWORKS                   ECOSYSTEM                  │
   │  ──────────                   ─────────                  │
   │  rtic                         heapless                   │
   │  embassy                      nb                         │
   │  bare-metal                   critical-section           │
   │                               smoltcp (TCP/IP)           │
   │                               embedded-graphics           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. Mind map cuối

```
                          EMBEDDED RUST
                                │
        ┌────────────┬──────────┼──────────┬─────────────┐
        ▼            ▼          ▼          ▼             ▼
   NO_STD       ECOSYSTEM    FRAMEWORKS  REAL-TIME    DEBUGGING
        │            │          │          │             │
   #![no_std]    PAC         RTIC       IRQ           probe-rs
   core+alloc    HAL         Embassy    Priority      defmt
   heapless      BSP         Bare loop  WCET          RTT
                 embedded-                Deadlines    GDB
                 hal                      Watchdog    Logic an.
                 embedded-                DMA
                 hal-async                Critical
                                          section
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. #![no_std] strip stdlib,         │
                │     use core+heapless                │
                │                                      │
                │  2. PAC → HAL → BSP layering         │
                │                                      │
                │  3. embedded-hal traits → portable   │
                │     drivers                          │
                │                                      │
                │  4. Type-state = HW safety at compile│
                │                                      │
                │  5. IRQ: short, atomic, defer        │
                │                                      │
                │  6. RTIC for hard real-time          │
                │     Embassy for async style          │
                │                                      │
                │  7. DMA for bulk, free CPU           │
                │                                      │
                │  8. defmt + probe-rs + RTT           │
                │                                      │
                │  9. Watchdog as safety net           │
                │                                      │
                │  10. WFI + clock gating for low power│
                │                                      │
                │  11. No panic in production          │
                │                                      │
                │  12. Profile WCET for hard real-time │
                └──────────────────────────────────────┘
```

---

## 19. Bộ tài liệu Rust giờ có 16 chủ đề

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
   │  16. embedded-rust           — Embedded + RTIC + Embassy │
   │      embedded-rust-visual    ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng Rust full-stack ĐẦY ĐỦ                   │
   │   Từ embedded MCU đến server production                  │
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

Còn 2 topic ứng dụng cuối:

- **Web framework realistic** — axum project apply 16 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool

Báo cái nào muốn đào sâu! 🦀⚡
