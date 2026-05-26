# Networking trong Rust — Deep Dive

> Tài liệu thứ 23 (chương w) trong bộ Rust nền tảng. Đọc trước:
> - [f-async.md](./f-async.md) — async network I/O dùng tokio
> - [a-memory-model.md](./a-memory-model.md) — buffer, alignment trong network code
> - [n-unsafe-rust.md](./n-unsafe-rust.md) — raw sockets, low-level
> - [v-grpc-tonic.md](./v-grpc-tonic.md) — application-level protocol example
>
> **Networking** = nền tảng của mọi distributed system. Rust mạnh ở networking vì:
> - **Memory safety** không cost (no buffer overflow → CVE)
> - **Zero-cost abstraction** — high-level API + native perf
> - **Async-first** — millions of concurrent connections (tokio)
> - **Low-level access** khi cần (mio, raw sockets)
> - **Mature ecosystem** (tokio, hyper, quinn, smoltcp)
>
> Real-world Rust networking:
> - **Cloudflare** — entire edge in Rust (proxies, WAF, DDoS protection)
> - **Discord** — switched read-states service Go → Rust
> - **Dropbox** — file sync, storage
> - **AWS Firecracker** — VMM với custom TCP/IP
> - **deno** — JS runtime với tokio
> - **mio** — foundation cho tokio, mio-st, ...
> - **quinn** — QUIC implementation
>
> Tài liệu này cover:
> - **Network fundamentals** — OSI layers, sockets, protocols
> - **High-level**: `std::net`, `tokio::net` (TCP/UDP servers)
> - **Low-level**: `mio` (epoll/kqueue/iocp wrapper)
> - **Custom protocols**: framing, encoding, design patterns
> - **Custom TCP/IP**: `smoltcp` (userspace stack for embedded)
> - **Modern protocols**: HTTP/3, QUIC, WebSocket
> - **Production**: scaling, backpressure, connection pool
> - **Network problems** và solutions

---

# Mục lục

- [Tầng 1: Network fundamentals](#tầng-1-network-fundamentals)
- [Tầng 2: Sockets in Rust](#tầng-2-sockets-in-rust)
- [Tầng 3: tokio::net — Async sockets](#tầng-3-tokionet--async-sockets)
- [Tầng 4: mio — Low-level async I/O](#tầng-4-mio--low-level-async-io)
- [Tầng 5: TCP custom protocols](#tầng-5-tcp-custom-protocols)
- [Tầng 6: Framing — Tokio codec](#tầng-6-framing--tokio-codec)
- [Tầng 7: UDP — Connection-less protocols](#tầng-7-udp--connection-less-protocols)
- [Tầng 8: smoltcp — Userspace TCP/IP stack](#tầng-8-smoltcp--userspace-tcpip-stack)
- [Tầng 9: WebSocket](#tầng-9-websocket)
- [Tầng 10: HTTP/3 và QUIC](#tầng-10-http3-và-quic)
- [Tầng 11: P2P networking](#tầng-11-p2p-networking)
- [Tầng 12: Network problems & solutions](#tầng-12-network-problems--solutions)
- [Tầng 13: Connection management — Pool, retry, circuit breaker](#tầng-13-connection-management--pool-retry-circuit-breaker)
- [Tầng 14: Performance tuning — Socket options, kernel](#tầng-14-performance-tuning--socket-options-kernel)
- [Tầng 15: Network testing](#tầng-15-network-testing)
- [Tầng 16: Patterns & antipatterns](#tầng-16-patterns--antipatterns)

---

# Tầng 1: Network fundamentals

## 1.1 OSI vs TCP/IP model

```
   ┌──────────────────────────────────────────────────────────┐
   │ OSI 7 layers          │ TCP/IP 4 layers    │ Rust libs    │
   ├──────────────────────────────────────────────────────────┤
   │ 7. Application         │ Application       │ axum, hyper, │
   │ 6. Presentation        │                   │ tonic        │
   │ 5. Session             │                   │              │
   │ 4. Transport           │ Transport         │ tokio::net   │
   │                        │                   │ quinn (QUIC) │
   │ 3. Network             │ Internet (IP)     │ smoltcp,     │
   │                        │                   │ pnet         │
   │ 2. Data Link           │ Link (Ethernet)   │ pnet, libpcap│
   │ 1. Physical            │ Physical          │ (drivers)    │
   └──────────────────────────────────────────────────────────┘
```

In practice, most Rust networking happens at **Transport** (TCP/UDP) and **Application** (HTTP/gRPC/...).

For low-level (custom protocols, packet inspection): smoltcp, pnet.

## 1.2 Protocols stack

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   APPLICATION layer                                      │
   │     HTTP/REST, gRPC, GraphQL, MQTT, SMTP, FTP, ...        │
   │                                                          │
   │   ──────────────────────────────────────────────         │
   │                                                          │
   │   TRANSPORT layer                                        │
   │     TCP                       UDP                        │
   │     ────                      ────                       │
   │     • Connection-oriented      • Connectionless           │
   │     • Reliable (ack, retransmit)• Best-effort            │
   │     • Ordered                 • Unordered                 │
   │     • Slow setup (3-way handshake)• Fast                  │
   │     • Flow control            • No flow control           │
   │     • For: web, DB, email     • For: DNS, gaming, video   │
   │                                                          │
   │     QUIC (over UDP) — best of both: reliable + fast      │
   │                                                          │
   │   ──────────────────────────────────────────────         │
   │                                                          │
   │   NETWORK layer (Internet)                               │
   │     IP (IPv4, IPv6)                                      │
   │     ICMP (ping, traceroute)                              │
   │     IPSec                                                │
   │                                                          │
   │   ──────────────────────────────────────────────         │
   │                                                          │
   │   LINK layer                                             │
   │     Ethernet, WiFi, ARP                                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

## 1.3 Key concepts

```
   IP address     → 32-bit IPv4 (192.168.1.1) or 128-bit IPv6 (::1)
                    Routable address on network
   
   Port           → 16-bit (0-65535)
                    Multiplex services on same IP
                    Well-known: 80 (HTTP), 443 (HTTPS), 22 (SSH)
                    Ephemeral: 49152-65535 (client-side)
   
   Socket         → Endpoint (IP + port + protocol)
                    OS abstraction for network I/O
   
   Connection     → Established socket pair (client + server)
                    Identified by (src_ip, src_port, dst_ip, dst_port, proto)
   
   MTU            → Max Transmission Unit (1500 bytes typical Ethernet)
                    Larger packets fragmented (slow!)
```

## 1.4 TCP handshake (3-way)

```
   Client                              Server
     │                                   │
     ├──── SYN (seq=x) ─────────────────►│
     │                                   │
     │◄──── SYN-ACK (seq=y, ack=x+1) ───│
     │                                   │
     ├──── ACK (ack=y+1) ───────────────►│
     │                                   │
     ├──── DATA ───────────────────────►│
     │◄──── DATA ────────────────────────│
     │                                   │
     ... (bidirectional data)            │
     │                                   │
     ├──── FIN ─────────────────────────►│
     │◄──── FIN-ACK ─────────────────────│
     │                                   │
   Connection closed
   
   3-way handshake setup: ~1 RTT (round-trip time)
   ~50-100ms for cross-continent connection
```

This setup cost is why **connection reuse** matters (HTTP/2 multiplexing, connection pools).

## 1.5 Common network operations

```
   ┌─────────────────────────────────────────────────────┐
   │                                                     │
   │   bind()    — server: claim a port                  │
   │   listen()  — accept incoming connections           │
   │   accept()  — handle new client                     │
   │   connect() — client: initiate connection           │
   │   send()/write()  — send bytes                      │
   │   recv()/read()   — receive bytes                   │
   │   close()   — terminate connection                  │
   │                                                     │
   │   Socket options:                                   │
   │     SO_REUSEADDR   — reuse port quickly             │
   │     TCP_NODELAY    — disable Nagle's algorithm      │
   │     SO_KEEPALIVE   — TCP keepalive                  │
   │     SO_LINGER       — close behavior                │
   │     TCP_USER_TIMEOUT — TCP-level timeout            │
   │                                                     │
   └─────────────────────────────────────────────────────┘
```

---

# Tầng 2: Sockets in Rust

## 2.1 std::net (synchronous)

```rust
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

// Server
fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    
    for stream in listener.incoming() {
        let mut stream = stream?;
        
        let mut buffer = [0u8; 1024];
        let n = stream.read(&mut buffer)?;
        
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        stream.write_all(response)?;
    }
    Ok(())
}
```

Simple. But **synchronous**: one connection blocks all others. For high concurrency → need threads or async.

## 2.2 Thread-per-connection (simple)

```rust
for stream in listener.incoming() {
    let stream = stream?;
    
    std::thread::spawn(move || {
        handle_client(stream);
    });
}
```

Each client = OS thread. Works for ~few hundred connections.

**Limits**:
- Each thread = ~2-8MB stack
- 10k connections = 80GB RAM (impossible)
- Context switching overhead

For C10K+ → need async (tokio).

## 2.3 std::net with timeouts

```rust
let stream = TcpStream::connect_timeout(
    &"127.0.0.1:8080".parse().unwrap(),
    Duration::from_secs(5)
)?;

stream.set_read_timeout(Some(Duration::from_secs(10)))?;
stream.set_write_timeout(Some(Duration::from_secs(10)))?;
```

Without timeouts, hung connection blocks thread forever.

## 2.4 UDP socket

```rust
use std::net::UdpSocket;

let socket = UdpSocket::bind("0.0.0.0:8080")?;

let mut buf = [0u8; 1024];
let (n, src) = socket.recv_from(&mut buf)?;
println!("Got {} bytes from {}", n, src);

socket.send_to(b"reply", src)?;
```

Connectionless. Each `recv_from` is independent packet. Each `send_to` to any address.

---

# Tầng 3: tokio::net — Async sockets

## 3.1 Setup

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

## 3.2 Async TCP server

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    
    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Connection from {}", addr);
        
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            
            loop {
                match socket.read(&mut buffer).await {
                    Ok(0) => return,   // connection closed
                    Ok(n) => {
                        // echo back
                        if socket.write_all(&buffer[..n]).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });
    }
}
```

`tokio::spawn` per connection. Scales to **millions** because tasks are cheap (~hundreds of bytes each, vs MB for threads).

## 3.3 Connect to server

```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    
    stream.write_all(b"Hello\n").await?;
    
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    
    println!("Received: {:?}", &buf[..n]);
    Ok(())
}
```

## 3.4 Split read/write

```rust
let (mut reader, mut writer) = stream.split();
// Or owned: stream.into_split() returns OwnedReadHalf + OwnedWriteHalf

// Now can read and write concurrently:
let read_task = tokio::spawn(async move {
    let mut buf = [0u8; 1024];
    while let Ok(n) = reader.read(&mut buf).await {
        if n == 0 { break; }
        // process
    }
});

let write_task = tokio::spawn(async move {
    writer.write_all(b"data").await.unwrap();
});
```

Important for chat servers, proxies — read and write independent.

## 3.5 Async UDP

```rust
use tokio::net::UdpSocket;

let socket = UdpSocket::bind("0.0.0.0:8080").await?;

let mut buf = [0u8; 1024];
loop {
    let (n, addr) = socket.recv_from(&mut buf).await?;
    
    // Echo back
    socket.send_to(&buf[..n], addr).await?;
}
```

## 3.6 With timeouts

```rust
use tokio::time::{timeout, Duration};

let stream = timeout(
    Duration::from_secs(5),
    TcpStream::connect("server:8080")
).await??;

let result = timeout(
    Duration::from_secs(10),
    stream.read(&mut buf)
).await;

match result {
    Ok(Ok(n)) => { /* got n bytes */ }
    Ok(Err(e)) => { /* read error */ }
    Err(_) => { /* timeout */ }
}
```

`tokio::time::timeout` wraps any future with deadline.

## 3.7 BufReader / BufWriter

```rust
use tokio::io::{BufReader, BufWriter, AsyncBufReadExt};

let (read_half, write_half) = stream.split();
let mut reader = BufReader::new(read_half);
let mut writer = BufWriter::new(write_half);

// Read line-by-line:
let mut line = String::new();
reader.read_line(&mut line).await?;

// Write efficiently:
writer.write_all(b"data").await?;
writer.flush().await?;   // important — explicit flush
```

Buffered I/O reduces syscalls for many small reads/writes.

## 3.8 Concurrent client handling pattern

```rust
async fn handle_client(socket: TcpStream) -> std::io::Result<()> {
    let (read_half, write_half) = socket.into_split();
    let reader = BufReader::new(read_half);
    let writer = Arc::new(Mutex::new(BufWriter::new(write_half)));
    
    let mut lines = reader.lines();
    
    while let Some(line) = lines.next_line().await? {
        let writer = Arc::clone(&writer);
        tokio::spawn(async move {
            process_command(line, writer).await
        });
    }
    Ok(())
}
```

Process commands concurrently per client. Mutex protect writer.

Or use channels for back-pressure:
```rust
let (tx, mut rx) = mpsc::channel(100);
// Reader sends to channel
// Single writer task drains channel
```

---

# Tầng 4: mio — Low-level async I/O

## 4.1 mio = OS-level I/O notification

`mio` (Metal IO) = thin wrapper over OS async APIs:
- Linux: `epoll`
- macOS/BSD: `kqueue`
- Windows: `IOCP`
- io_uring (newer Linux)

Foundation underneath tokio. Most apps don't use mio directly — use tokio.

When to use mio:
- Building custom runtime
- Need maximum control / minimum overhead
- Custom protocols with unusual I/O patterns
- Embedded (no tokio runtime)

## 4.2 mio basic example

```toml
[dependencies]
mio = { version = "1", features = ["net", "os-poll"] }
```

```rust
use mio::{Events, Poll, Interest, Token};
use mio::net::TcpListener;

const SERVER: Token = Token(0);

fn main() -> std::io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    
    let mut listener = TcpListener::bind("127.0.0.1:8080".parse()?)?;
    
    poll.registry().register(&mut listener, SERVER, Interest::READABLE)?;
    
    let mut next_token = Token(1);
    let mut connections = std::collections::HashMap::new();
    
    loop {
        poll.poll(&mut events, None)?;
        
        for event in &events {
            match event.token() {
                SERVER => {
                    // Accept new connections
                    let (mut stream, _addr) = listener.accept()?;
                    let token = next_token;
                    next_token = Token(next_token.0 + 1);
                    
                    poll.registry().register(
                        &mut stream, token, 
                        Interest::READABLE | Interest::WRITABLE
                    )?;
                    
                    connections.insert(token, stream);
                }
                token => {
                    // Handle existing connection
                    if let Some(stream) = connections.get_mut(&token) {
                        // ... read/write ...
                    }
                }
            }
        }
    }
}
```

Manual:
1. Create Poll
2. Register sources with tokens
3. Loop: poll for events
4. Dispatch by token

mio handles low-level. You build event loop on top.

## 4.3 mio vs tokio

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   mio                       tokio                        │
   │   ───                       ─────                        │
   │   Low-level                 High-level                   │
   │   Manual event loop         Built-in runtime             │
   │   Manual state machine      async/await                  │
   │   No allocator              Task scheduler               │
   │   No timers                 Timers, intervals            │
   │   No IO macros              println, network easy        │
   │                                                          │
   │   ~ syscalls + OS API        Hides complexity            │
   │   Use: custom runtime,       Use: 99% applications       │
   │        max performance       max productivity             │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

Default: **tokio**. Drop to mio only if you have specific reason.

## 4.4 io_uring (Linux 5.1+)

Newer Linux API. **Completion-based** instead of readiness-based:

```
   epoll: "Socket is ready for read" → CPU calls read()
   io_uring: "Read for me, notify when done" → kernel reads, returns data
```

Less syscalls. Faster for high-throughput servers.

Rust support:
- `tokio-uring` — io_uring runtime (experimental)
- `glommio` — io_uring focused runtime
- `monoio` — single-thread io_uring runtime

Cutting edge. Not always faster — depends on workload.

---

# Tầng 5: TCP custom protocols

## 5.1 Why custom protocol?

Beyond HTTP:
- **Database wire protocols** (Postgres, MySQL, Redis)
- **Messaging** (Kafka, RabbitMQ, MQTT)
- **Game networking** (custom UDP/TCP)
- **IoT** (CoAP, MQTT)
- **Distributed systems** (Raft, gossip)
- **High-performance trading** (FIX, ITCH)

## 5.2 Protocol design questions

```
   ┌──────────────────────────────────────────────────────────┐
   │ Question                  │ Considerations               │
   ├──────────────────────────────────────────────────────────┤
   │ Binary or text?           │ Binary: faster, opaque       │
   │                            │ Text: easier debug (e.g. HTTP)│
   │                                                          │
   │ Framing?                  │ How to know where message    │
   │                            │ ends in stream?              │
   │                                                          │
   │ Encoding?                 │ Protobuf, MessagePack, custom │
   │                                                          │
   │ Versioning?               │ Negotiate version on connect? │
   │                                                          │
   │ Authentication?           │ TLS, token, challenge?       │
   │                                                          │
   │ Flow control?              │ Backpressure mechanism       │
   │                                                          │
   │ Error handling?            │ Error codes, retry           │
   │                                                          │
   │ Connection management?     │ Long-lived, request/response │
   │                                                          │
   │ Multiplexing?              │ Multiple streams per conn?   │
   │                            │ (HTTP/2 style)               │
   └──────────────────────────────────────────────────────────┘
```

## 5.3 Framing strategies

TCP = byte stream. Need to **frame** messages.

### Strategy 1: Fixed-length

```
[msg1: 4 bytes][msg2: 4 bytes][msg3: 4 bytes]
```

Simple. Wastes if data varies. Used for **fixed-size headers**.

### Strategy 2: Length-prefixed

```
[len: 4 bytes][data: N bytes][len: 4 bytes][data: M bytes]...
```

Most common. Read length first, then exact N bytes.

```rust
async fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    
    Ok(data)
}
```

### Strategy 3: Delimiter-based

```
msg1\nmsg2\nmsg3\n
```

Used by HTTP, SMTP. Line-based protocols.

Issue: must escape delimiter in payload.

### Strategy 4: Protocol-specific markers

```
HTTP: \r\n\r\n separates headers from body
Then Content-Length header tells body size
```

Hybrid approach. Used by HTTP/1.1.

### Strategy 5: Self-describing format

Protobuf has its own framing (varint length prefix). Just need transport-level framing.

## 5.4 Example: Simple length-prefixed protocol

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Frame format:
//   [4-byte BE u32 length][payload]

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    if len > 10 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too big",
        ));
    }
    
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok(data)
}
```

**Important**: validate length to prevent DoS (attacker sends length=4GB).

---

# Tầng 6: Framing — Tokio codec

## 6.1 tokio_util::codec

For complex framing, use `Framed` + codec:

```toml
[dependencies]
tokio-util = { version = "0.7", features = ["codec"] }
bytes = "1"
```

## 6.2 LengthDelimitedCodec

```rust
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};

let codec = LengthDelimitedCodec::builder()
    .length_field_offset(0)
    .length_field_length(4)
    .max_frame_length(1024 * 1024)   // 1MB max
    .new_codec();

let mut framed = Framed::new(socket, codec);

// Send a frame:
framed.send("hello".into()).await?;

// Receive frames:
while let Some(result) = framed.next().await {
    let bytes = result?;
    println!("Got frame: {:?}", bytes);
}
```

`Framed` = wraps socket as Stream + Sink of frames. Handles framing automatically.

## 6.3 LinesCodec — Newline-delimited

```rust
use tokio_util::codec::{Framed, LinesCodec};

let mut framed = Framed::new(socket, LinesCodec::new());

framed.send("Hello\n".to_string()).await?;

while let Some(Ok(line)) = framed.next().await {
    println!("Got line: {}", line);
}
```

Each `\n` = boundary. Like telnet protocol.

## 6.4 Custom codec

```rust
use tokio_util::codec::{Encoder, Decoder};
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug)]
struct MyMessage {
    msg_type: u8,
    payload: Vec<u8>,
}

struct MyCodec;

impl Decoder for MyCodec {
    type Item = MyMessage;
    type Error = std::io::Error;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<MyMessage>, Self::Error> {
        if src.len() < 5 {
            // Not enough for header
            return Ok(None);
        }
        
        let msg_type = src[0];
        let len = u32::from_be_bytes(src[1..5].try_into().unwrap()) as usize;
        
        if src.len() < 5 + len {
            // Need more bytes
            src.reserve(5 + len - src.len());
            return Ok(None);
        }
        
        // Skip header
        src.advance(5);
        
        // Read payload
        let payload = src.split_to(len).to_vec();
        
        Ok(Some(MyMessage { msg_type, payload }))
    }
}

impl Encoder<MyMessage> for MyCodec {
    type Error = std::io::Error;
    
    fn encode(&mut self, msg: MyMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(msg.msg_type);
        dst.put_u32(msg.payload.len() as u32);
        dst.put_slice(&msg.payload);
        Ok(())
    }
}

// Use:
let mut framed = Framed::new(socket, MyCodec);
```

Custom codec for any binary format.

## 6.5 Decoder pattern

```
   TCP byte stream:
   ─────────────────
   [type 1][len 5][hello][type 2][len 4][test]
   
   decode() called multiple times:
   ────────────────────────────
   
   Call 1: src = [1, 0,0,0,5, h,e]   (partial)
           → return Ok(None), need more
   
   Call 2: src = [1, 0,0,0,5, h,e,l,l,o, 2, 0,0]   (one full + start of next)
           → consume [1, 0,0,0,5, h,e,l,l,o]
           → return Ok(Some(Message1))
   
   Call 3: src = [2, 0,0,0,4, t,e,s,t]   (second one complete)
           → return Ok(Some(Message2))
```

Decoder may return None if not enough bytes. Framework calls again when more arrives.

## 6.6 Serde + codec

Combine for typed messages:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
enum Message {
    Login { user: String },
    SendMessage { to: String, content: String },
    Logout,
}

// Serialize with bincode or messagepack:
fn encode(msg: &Message) -> Vec<u8> {
    bincode::serialize(msg).unwrap()
}

fn decode(bytes: &[u8]) -> Result<Message, bincode::Error> {
    bincode::deserialize(bytes)
}

// Custom codec wraps these
```

Type-safe protocol messages. Common pattern.

---

# Tầng 7: UDP — Connection-less protocols

## 7.1 UDP characteristics

```
   ┌──────────────────────────────────────────────────────────┐
   │  TCP                       UDP                           │
   │  ───                       ───                           │
   │  Connection-oriented       Connectionless                │
   │  Reliable                  Best-effort                   │
   │  Ordered                   Unordered                     │
   │  Slow setup (handshake)    No setup                      │
   │  Flow control              No flow control                │
   │  Per-byte stream           Per-message (packet)          │
   │  Larger overhead           Smaller overhead              │
   │                                                          │
   │  Use for:                  Use for:                      │
   │   web, DB, email           DNS, gaming, video, VoIP      │
   │   most apps                custom protocols              │
   │   when reliability matters when low latency matters      │
   └──────────────────────────────────────────────────────────┘
```

## 7.2 UDP server

```rust
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080").await?;
    println!("UDP server on {}", socket.local_addr()?);
    
    let mut buf = [0u8; 65536];   // max UDP datagram
    
    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        let msg = &buf[..n];
        
        println!("From {}: {} bytes", peer, n);
        
        // Echo back
        socket.send_to(msg, peer).await?;
    }
}
```

No accept. Each datagram has src address.

## 7.3 UDP issues

```
   ❌ Packet loss
      Network drops some packets. Need app-level retransmit.
   
   ❌ Reordering
      Packets arrive out of order. Need sequence numbers.
   
   ❌ Duplication
      Same packet may arrive multiple times. Need dedup.
   
   ❌ Fragmentation
      UDP > MTU (~1500 bytes) gets IP-fragmented. Reassembly slow.
      Keep UDP packets < 1500 bytes (or 1200 for VPN).
   
   ❌ NAT timeout
      NAT may drop "connection" after silence. Keepalive needed.
   
   ❌ Amplification attacks
      Attacker spoofs source IP, server reflects big response to victim.
      Mitigate: rate limit, response size limit.
   
   ✅ Speed
      No handshake. ~1 RTT savings.
   
   ✅ Real-time
      No retry delays. Drop old packets in favor of new (audio/video).
```

## 7.4 Building reliable UDP

Many "custom reliable UDP" protocols exist. Example QUIC, KCP.

Pattern:
```
   Sender:
   1. Number each packet
   2. Buffer recent packets (for retransmit)
   3. Wait for ACK
   4. If no ACK in timeout, retransmit
   
   Receiver:
   1. Sort by sequence number
   2. Send ACK per packet
   3. Detect gaps → request retransmit
   4. Buffer out-of-order, deliver in order
```

But: reinventing TCP. Often better to use QUIC (Tầng 10).

## 7.5 Use cases for UDP

```
   • DNS — small request/response, retry by app
   • Gaming — position updates (drop stale)
   • Video streaming — old frames useless
   • VoIP — audio packets, drop stale > retransmit
   • IoT sensor data — many devices, small messages
   • Distributed systems heartbeat
   • Discovery protocols (mDNS, SSDP)
   • Custom high-perf protocols (research, HFT)
```

---

# Tầng 8: smoltcp — Userspace TCP/IP stack

## 8.1 smoltcp = TCP/IP without OS

`smoltcp` = implements TCP/IP in **userspace**. No kernel involved.

Use cases:
- **Embedded** (no_std, no OS network stack)
- **Custom NIC drivers** (you receive raw frames)
- **VMs / unikernels** (own network stack)
- **Network experimentation**

```toml
[dependencies]
smoltcp = { version = "0.11", default-features = false, features = [
    "alloc",
    "medium-ethernet",
    "proto-ipv4",
    "socket-tcp",
    "socket-udp",
] }
```

## 8.2 Example: TCP server with smoltcp

```rust
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::time::Instant;
use smoltcp::wire::*;

fn main() {
    let mut device = create_device();  // your custom device
    let config = Config::new(EthernetAddress([0x02, 0, 0, 0, 0, 0x01]).into());
    let mut iface = Interface::new(config, &mut device, Instant::now());
    
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::v4(192, 168, 0, 100), 24)).unwrap();
    });
    
    // Create socket
    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    
    let mut sockets = SocketSet::new(vec![]);
    let tcp_handle = sockets.add(tcp_socket);
    
    let mut listening = false;
    
    loop {
        let timestamp = Instant::now();
        iface.poll(timestamp, &mut device, &mut sockets);
        
        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);
        if !listening {
            socket.listen(8080).unwrap();
            listening = true;
        }
        
        if socket.may_recv() {
            let data = socket.recv(|buffer| {
                let n = buffer.len();
                let bytes = buffer.to_vec();
                (n, bytes)
            }).unwrap();
            
            println!("Got: {:?}", data);
            socket.send_slice(&data).unwrap();   // echo
        }
        
        // ... advance time / poll device ...
    }
}
```

Manual everything: state machine, polling, time. No tokio.

## 8.3 Device trait

You provide raw frame I/O:

```rust
use smoltcp::phy::{Device, DeviceCapabilities, RxToken, TxToken};

struct MyDevice {
    // your driver state
}

impl Device for MyDevice {
    type RxToken<'a> = MyRxToken<'a> where Self: 'a;
    type TxToken<'a> = MyTxToken<'a> where Self: 'a;
    
    fn receive(&mut self, _ts: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        // Pull raw frame from hardware / driver
        // Return tokens for processing
    }
    
    fn transmit(&mut self, _ts: Instant) -> Option<Self::TxToken<'_>> {
        // Get buffer for sending frame
    }
    
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps
    }
}
```

Implement for your NIC, USB ethernet, TUN/TAP, custom hardware.

## 8.4 smoltcp on no_std

For embedded (chương p):
```toml
smoltcp = { version = "0.11", default-features = false, features = [
    "medium-ethernet",
    "proto-ipv4",
    "socket-tcp",
] }
```

No allocator. All buffers static. Embedded device gets TCP/IP without RTOS.

Used by:
- **Embassy net** (async embedded network)
- **Firecracker** (AWS microVM)
- **Custom routers, firewalls**

## 8.5 When to use smoltcp?

```
   ✅ DO:
   • Embedded device with network (no OS stack)
   • Custom hardware NIC driver
   • Unikernel / specialized OS
   • Reproducible network behavior
   • Educational (understand TCP/IP)
   
   ❌ DON'T:
   • Regular server app (use OS sockets, tokio)
   • Already-working TCP stack
   • Quick prototyping
```

For 99% of apps: OS sockets + tokio.

---

# Tầng 9: WebSocket

## 9.1 WebSocket = bidirectional TCP over HTTP upgrade

```
   Client                              Server
     │                                   │
     │ HTTP/1.1 GET /ws                  │
     │ Upgrade: websocket                │
     ├──────────────────────────────────►│
     │                                   │
     │ HTTP/1.1 101 Switching Protocols  │
     │ Upgrade: websocket                │
     │◄──────────────────────────────────│
     │                                   │
     │ ━━ now WebSocket frames ━━        │
     │                                   │
     ├──── text/binary frame ──────────►│
     │◄──── text/binary frame ──────────│
     │     ...                            │
     │                                   │
     │ ── close frame ──────────────────►│
     │◄────── close ─────────────────────│
```

Used for: chat, notifications, live updates in browsers.

## 9.2 tokio-tungstenite

```toml
[dependencies]
tokio-tungstenite = "0.24"
futures-util = "0.3"
```

## 9.3 WebSocket server

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
    
    Ok(())
}

async fn handle_connection(stream: TcpStream) {
    let ws = accept_async(stream).await.unwrap();
    let (mut write, mut read) = ws.split();
    
    while let Some(Ok(msg)) = read.next().await {
        match msg {
            Message::Text(text) => {
                write.send(Message::Text(format!("Echo: {}", text))).await.unwrap();
            }
            Message::Binary(data) => {
                write.send(Message::Binary(data)).await.unwrap();
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
```

## 9.4 WebSocket client

```rust
use tokio_tungstenite::connect_async;
use tungstenite::Message;
use futures_util::{StreamExt, SinkExt};

let (ws, _) = connect_async("ws://localhost:8080").await?;
let (mut write, mut read) = ws.split();

write.send(Message::Text("Hello".into())).await?;

while let Some(Ok(msg)) = read.next().await {
    println!("Got: {:?}", msg);
}
```

## 9.5 axum WebSocket integration

```rust
use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    routing::get,
    Router,
};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            socket.send(Message::Text(format!("Echo: {}", text))).await.unwrap();
        }
    }
}

let app = Router::new()
    .route("/ws", get(ws_handler));
```

Integrated with axum. Standard HTTP route upgrades to WebSocket.

## 9.6 WebSocket vs SSE vs gRPC streaming

```
   ┌──────────────────────────────────────────────────────────┐
   │ Aspect       │ WebSocket  │ SSE         │ gRPC stream    │
   ├──────────────────────────────────────────────────────────┤
   │ Direction    │ Bidi       │ Server→client│ Bidi          │
   │ Protocol     │ Custom      │ HTTP/1.1    │ HTTP/2        │
   │ Browser?     │ ✅          │ ✅           │ via gRPC-Web  │
   │ Reconnect    │ Manual      │ Auto         │ Manual        │
   │ Headers per  │ Once         │ Once         │ HTTP/2 frames│
   │ Binary       │ ✅           │ Text only    │ ✅            │
   │ Use          │ Chat,games  │ Notifications│ Microservices│
   └──────────────────────────────────────────────────────────┘
```

WebSocket for browser bidi. gRPC for service-to-service.

---

# Tầng 10: HTTP/3 và QUIC

## 10.1 QUIC = Next-gen transport

QUIC (Quick UDP Internet Connections):
- Built on **UDP**, not TCP
- **TLS 1.3 mandatory** (encryption built-in)
- **Multiplexing** without head-of-line blocking (vs HTTP/2 over TCP)
- **0-RTT connection** resumption
- **Connection migration** (network change, IP changes — still connected)
- Used by **HTTP/3**

```
   HTTP/3 stack:
   ──────────
   ┌─────────────────────────┐
   │ HTTP/3 application      │
   ├─────────────────────────┤
   │ QUIC (transport + TLS)  │
   ├─────────────────────────┤
   │ UDP                     │
   ├─────────────────────────┤
   │ IP                      │
   └─────────────────────────┘
```

## 10.2 quinn crate

```toml
[dependencies]
quinn = "0.11"
rustls = "0.23"
```

```rust
use quinn::{Endpoint, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_config = ServerConfig::with_single_cert(
        vec![cert_der],
        priv_key,
    )?;
    
    let endpoint = Endpoint::server(
        server_config,
        "0.0.0.0:4433".parse()?,
    )?;
    
    while let Some(connecting) = endpoint.accept().await {
        let connection = connecting.await?;
        tokio::spawn(handle_connection(connection));
    }
    Ok(())
}

async fn handle_connection(conn: quinn::Connection) -> Result<(), Box<dyn std::error::Error>> {
    while let Ok((mut send, mut recv)) = conn.accept_bi().await {
        // Each pair = one stream
        let req = recv.read_to_end(usize::MAX).await?;
        send.write_all(b"response").await?;
        send.finish()?;
    }
    Ok(())
}
```

## 10.3 Why HTTP/3 / QUIC?

```
   HTTP/2 over TCP problems:
   ─────────────
   • TCP head-of-line blocking — 1 packet loss blocks all streams
   • Slow connection setup (TCP + TLS = 3 RTT)
   • Can't change network without reconnect
   
   QUIC fixes:
   ────────
   • Independent streams per connection (no HoL block)
   • 1 RTT setup (or 0 RTT resumption)
   • Connection ID — survive network change
   • Encryption built-in
   • Better congestion control
```

## 10.4 When to use HTTP/3?

```
   ✅ Mobile apps (network changes common)
   ✅ Globally distributed clients
   ✅ Streaming media
   ✅ Real-time applications
   
   ⚠️ NAT/firewalls may block UDP
   ⚠️ Server complexity
   ⚠️ Browser support good now (Chrome, Firefox, Safari)
```

Cloudflare, Google, Microsoft use HTTP/3 in production. Mature in 2024+.

## 10.5 Rust QUIC ecosystem

- **quinn** — most popular QUIC
- **s2n-quic** — Amazon's QUIC implementation
- **h3** — HTTP/3 layer (uses quinn or other QUIC)
- **hyper-h3** — HTTP/3 hyper integration

For HTTP/3 server:
```toml
h3 = "0.0.6"
h3-quinn = "0.0.7"
```

---

# Tầng 11: P2P networking

## 11.1 libp2p

Peer-to-peer networking framework. Same as IPFS, Filecoin, Ethereum.

```toml
[dependencies]
libp2p = { version = "0.55", features = [
    "tcp", "quic", "noise", "yamux",
    "gossipsub", "kad", "identify",
] }
```

## 11.2 Concepts

```
   • Peer ID — cryptographic identity (public key hash)
   • Multiaddr — addressing: /ip4/127.0.0.1/tcp/8000/p2p/<peer-id>
   • Transport — TCP, QUIC, WebSocket, WebRTC
   • Security — Noise, TLS for encryption + auth
   • Multiplexing — yamux, mplex for multi-stream over connection
   • Protocols — application-level: gossip, kademlia, file transfer
```

## 11.3 Simple peer example

```rust
use libp2p::{
    futures::StreamExt,
    swarm::SwarmEvent,
    SwarmBuilder, identify, identity,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(Default::default(), libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|key| {
            identify::Behaviour::new(identify::Config::new(
                "/my-app/1.0".to_string(),
                key.public(),
            ))
        })?
        .build();
    
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {address:?}");
            }
            SwarmEvent::Behaviour(event) => {
                println!("Identify event: {event:?}");
            }
            _ => {}
        }
    }
}
```

Each peer can listen, dial, discover other peers, exchange messages.

## 11.4 Use cases for P2P

```
   • Decentralized storage (IPFS, Filecoin)
   • Blockchain nodes
   • Mesh networks (offline-first apps)
   • File sharing (BitTorrent-like)
   • Real-time collaboration without central server
   • Resilient to censorship / outages
   • Local-first apps with sync
```

P2P is harder than client-server (NAT, discovery, security). But unique capabilities.

---

# Tầng 12: Network problems & solutions

## 12.1 Common network problems

```
   ┌──────────────────────────────────────────────────────────┐
   │ Problem                  │ Symptom                       │
   ├──────────────────────────────────────────────────────────┤
   │ Latency                  │ Slow request                  │
   │ Packet loss              │ Retries, timeouts             │
   │ Bandwidth limit          │ Throughput cap                │
   │ Connection limit          │ "too many open files"        │
   │ NAT timeout              │ Long-idle conn drops          │
   │ DNS slow / fail          │ Hostname lookup hang          │
   │ TLS handshake slow       │ First request slow            │
   │ Server overload           │ Cascading errors              │
   │ Backpressure missing      │ Memory growth, OOM            │
   │ Slowloris attack          │ Sockets stuck                 │
   │ DDoS                      │ Service down                  │
   │ Stale connections         │ Use-after-close errors        │
   └──────────────────────────────────────────────────────────┘
```

## 12.2 Latency

```
   Sources of latency:
   ───────────────────
   1. Distance (speed of light): ~5ms per 1000km
   2. Routing (hops between routers)
   3. Congestion (queuing in switches)
   4. Server processing time
   5. Database queries
   6. Serialization
   
   Mitigations:
   ────────────
   • CDN / edge computing (compute closer to user)
   • Connection pooling (avoid handshake per request)
   • HTTP/2, HTTP/3 multiplexing
   • Compression (less data = less time)
   • Caching (avoid round-trip)
   • Async (multiple requests in flight)
```

## 12.3 Packet loss handling

```rust
// Pattern: retry with exponential backoff
async fn fetch_with_retry(url: &str) -> Result<Response> {
    let mut delay_ms = 100;
    for attempt in 0..5 {
        match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => return Ok(resp),
            Ok(_) | Err(_) => {
                if attempt < 4 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;   // exponential backoff
                }
            }
        }
    }
    Err(anyhow::anyhow!("max retries"))
}
```

Crates: `backoff`, `tokio-retry`.

## 12.4 Backpressure

```
   Scenario: server reads from client faster than processes
              → memory grows → OOM crash
   
   Solution: bounded channels, awaiting consumer
```

```rust
use tokio::sync::mpsc;

// Bounded channel — backpressure
let (tx, mut rx) = mpsc::channel::<Message>(100);

// Producer:
tokio::spawn(async move {
    loop {
        let msg = read_from_network().await?;
        // Backpressure: send() awaits if channel full
        if tx.send(msg).await.is_err() {
            break;
        }
    }
});

// Consumer:
while let Some(msg) = rx.recv().await {
    process(msg).await;
}
```

Channel full → producer blocks. Memory bounded.

## 12.5 Connection limits

```rust
// File descriptor limit:
// ulimit -n  → see current
// ulimit -n 100000  → raise

// In Rust, no built-in limit, OS handles.
// Use semaphore for app-level limit:

use tokio::sync::Semaphore;
use std::sync::Arc;

let max_conn = Arc::new(Semaphore::new(1000));

loop {
    let (stream, _) = listener.accept().await?;
    let permit = max_conn.clone().acquire_owned().await.unwrap();
    
    tokio::spawn(async move {
        handle_client(stream).await;
        drop(permit);   // releases slot
    });
}
```

Limit total concurrent connections. Prevents resource exhaustion.

## 12.6 Slowloris attack

```
   Attacker opens many connections, sends data byte-by-byte
   forever, never completes request → server keeps sockets open
   
   Mitigation:
   ───────────
   • Read timeout per byte/operation
   • Total request timeout
   • Detect "incomplete request" patterns
```

```rust
let result = tokio::time::timeout(
    Duration::from_secs(30),    // total timeout
    handle_request(stream)
).await;
```

## 12.7 TLS handshake cost

```
   TLS 1.3: 1 RTT for handshake
   TLS 1.2: 2 RTT
   TLS Resumption: 0 RTT
```

Reduce:
- TLS 1.3 (modern)
- Session resumption (cache session keys)
- Connection reuse (HTTP/2 — many requests, 1 handshake)
- Pre-warm connections in pool

---

# Tầng 13: Connection management — Pool, retry, circuit breaker

## 13.1 Connection pool

```rust
// HTTP client pool (reqwest):
let client = reqwest::Client::builder()
    .pool_idle_timeout(Duration::from_secs(90))
    .pool_max_idle_per_host(10)
    .timeout(Duration::from_secs(30))
    .build()?;

// reqwest reuses HTTP connections automatically
// connection per origin host
```

For custom protocols, use `bb8` or `deadpool`:

```toml
[dependencies]
bb8 = "0.8"
```

```rust
use bb8::{Pool, ManageConnection};

struct MyManager;

#[async_trait::async_trait]
impl ManageConnection for MyManager {
    type Connection = TcpStream;
    type Error = std::io::Error;
    
    async fn connect(&self) -> Result<TcpStream, std::io::Error> {
        TcpStream::connect("server:8080").await
    }
    
    async fn is_valid(&self, _conn: &mut TcpStream) -> Result<(), std::io::Error> {
        // ping or just trust
        Ok(())
    }
    
    fn has_broken(&self, _conn: &mut TcpStream) -> bool {
        false
    }
}

let pool = Pool::builder()
    .max_size(20)
    .build(MyManager).await?;

let mut conn = pool.get().await?;
conn.write_all(b"data").await?;
// drop returns to pool
```

## 13.2 Circuit breaker pattern

When upstream service fails, don't keep hammering it:

```
   States:
   ───────
   CLOSED  — normal, requests pass through
              count failures, if threshold exceeded → OPEN
   OPEN    — all requests fail fast (without hitting upstream)
              after timeout → HALF_OPEN
   HALF_OPEN — let 1 request through to test
              success → CLOSED
              failure → OPEN
```

Crate: `failsafe`, `circuit-breaker`.

```rust
use failsafe::backoff::Constant;
use failsafe::failure_policy::ConsecutiveFailures;
use failsafe::{Config, CircuitBreaker};

let breaker = Config::new()
    .failure_policy(ConsecutiveFailures::new(3, Constant::new(Duration::from_secs(5))))
    .build();

let result = breaker.call(|| async {
    fetch_from_upstream().await
}).await;
// Fails fast if upstream consistently failing
```

## 13.3 Bulkhead pattern

Isolate resources to prevent cascading:

```rust
// Per-service semaphores:
struct Services {
    db_limit: Arc<Semaphore>,           // 20 concurrent DB ops
    external_api_limit: Arc<Semaphore>, // 10 concurrent external calls
}

async fn handle_request(svc: &Services) -> Result<()> {
    let _db_permit = svc.db_limit.acquire().await?;
    let data = fetch_from_db().await?;
    drop(_db_permit);
    
    let _api_permit = svc.external_api_limit.acquire().await?;
    let result = call_external(data).await?;
    Ok(())
}
```

If external API slow, don't exhaust DB pool. Each resource bounded independently.

## 13.4 Retry with backoff + jitter

```rust
async fn retry_with_jitter<F, Fut, T, E>(
    mut f: F,
    max_attempts: u32,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut delay = 100u64;
    for attempt in 0..max_attempts {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 < max_attempts => {
                // Add jitter to avoid thundering herd:
                let jitter = rand::random::<u64>() % delay;
                tokio::time::sleep(Duration::from_millis(delay + jitter)).await;
                delay = (delay * 2).min(30_000);   // cap at 30s
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

Jitter prevents all clients retrying at same moment. Smooths recovery.

---

# Tầng 14: Performance tuning — Socket options, kernel

## 14.1 Socket options

```rust
use socket2::{Socket, Domain, Type, Protocol};

let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

// Disable Nagle's algorithm (low latency, more syscalls)
socket.set_nodelay(true)?;

// Reuse port quickly (don't wait for TIME_WAIT):
socket.set_reuse_address(true)?;
socket.set_reuse_port(true)?;   // Linux multi-thread listen

// TCP keepalive (detect dead peers):
socket.set_keepalive(true)?;

// Buffer sizes:
socket.set_recv_buffer_size(256 * 1024)?;
socket.set_send_buffer_size(256 * 1024)?;

// Linger (close behavior):
socket.set_linger(Some(Duration::from_secs(0)))?;   // immediate close

// Timeout:
socket.set_read_timeout(Some(Duration::from_secs(30)))?;
```

## 14.2 TCP_NODELAY

```
   Nagle's algorithm: batch small writes into single packet
                       (reduces small-packet overhead)
   
   Problem: latency-sensitive apps want immediate send
            Game commands, chat, real-time
   
   Solution: TCP_NODELAY → send immediately, more packets
             worse bandwidth, better latency
```

Default: depends on platform. Often want to disable for interactive.

## 14.3 SO_REUSEPORT (Linux)

```rust
// Multiple processes/threads bind same port, kernel balances
socket.set_reuse_port(true)?;
```

Useful for multi-process servers (sharded by core).

## 14.4 Kernel parameters

```bash
# View limits:
cat /proc/sys/net/core/somaxconn        # listen backlog
cat /proc/sys/net/core/rmem_max         # max recv buffer
cat /proc/sys/net/core/wmem_max         # max send buffer
cat /proc/sys/net/ipv4/tcp_max_syn_backlog
ulimit -n                                # file descriptors

# Tune for high-perf server:
sysctl -w net.core.somaxconn=4096
sysctl -w net.ipv4.tcp_max_syn_backlog=8192
sysctl -w net.ipv4.tcp_tw_reuse=1
sysctl -w fs.file-max=1000000
ulimit -n 1000000
```

Default Linux config tuned for desktop, not server. Production needs tuning.

## 14.5 epoll vs io_uring

```
   epoll:
   ──────
   • Readiness-based: kernel says "fd is ready", app calls read()
   • Maturity: Linux since 2002, well-understood
   • Performance: great for ~10k+ connections
   • Bottleneck: many syscalls (one per ready event)
   
   io_uring:
   ─────────
   • Completion-based: app says "read for me", kernel notifies done
   • Linux 5.1+ (2019)
   • Reduces syscalls (batch operations)
   • Better for high-throughput (millions ops/sec)
   • Still maturing, complex API
```

Tokio defaults to epoll. `tokio-uring` for io_uring (still beta).

## 14.6 Zero-copy

```rust
use tokio::io::AsyncWriteExt;
use bytes::Bytes;

// Standard: write copies to kernel buffer
socket.write_all(&data).await?;

// Zero-copy (Linux sendfile, Bytes):
// Vector I/O — multiple buffers without copy:
let bufs = [
    std::io::IoSlice::new(b"header\n"),
    std::io::IoSlice::new(&body),
];
socket.write_vectored(&bufs).await?;
```

Zero-copy via `Bytes`, `bytes::BytesMut` — sharing buffers without clone.

```rust
let data = Bytes::from(big_vec);
let clone1 = data.clone();   // O(1), no copy
let clone2 = data.clone();
// All share same underlying buffer
```

## 14.7 SIMD for parsing

```rust
// HTTP parsers like picohttpparser use SIMD for fast parsing
// httparse crate uses simd-x86 to scan headers fast

let mut headers = [httparse::EMPTY_HEADER; 64];
let mut req = httparse::Request::new(&mut headers);
req.parse(buf)?;   // Uses SIMD internally
```

## 14.8 Profiling network code

```
   Tools:
   ──────
   • tcpdump / wireshark — packet capture
   • iftop / nethogs — bandwidth monitor
   • ss / netstat — socket state
   • perf — CPU profiling
   • bpftrace — Linux tracing
   • tokio-console — async runtime profiling
   • iperf3 — throughput benchmark
```

Network bottlenecks often in:
- Allocations (use bytes crate)
- Syscall count (batch operations)
- Lock contention (sharding)
- DNS resolution (cache)

---

# Tầng 15: Network testing

## 15.1 Unit test protocol code

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_encode() {
        let data = b"hello";
        let mut buf = BytesMut::new();
        MyCodec.encode(MyMessage { msg_type: 1, payload: data.to_vec() }, &mut buf).unwrap();
        
        assert_eq!(buf.as_ref(), &[1, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o']);
    }
    
    #[test]
    fn test_frame_decode_partial() {
        let mut buf = BytesMut::from(&[1, 0, 0, 0, 5, b'h', b'e'][..]);
        let result = MyCodec.decode(&mut buf).unwrap();
        assert!(result.is_none());   // not enough bytes
    }
    
    #[test]
    fn test_frame_decode_complete() {
        let mut buf = BytesMut::from(&[1, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o'][..]);
        let msg = MyCodec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(msg.msg_type, 1);
        assert_eq!(msg.payload, b"hello");
    }
}
```

## 15.2 Integration test with real socket

```rust
#[tokio::test]
async fn test_server_echo() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        socket.write_all(&buf[..n]).await.unwrap();
    });
    
    // Client side:
    let mut client = TcpStream::connect(addr).await.unwrap();
    client.write_all(b"hello").await.unwrap();
    
    let mut buf = [0u8; 1024];
    let n = client.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"hello");
}
```

Random port (`:0` = OS picks) avoids conflicts.

## 15.3 Property test for protocol

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn encode_decode_roundtrip(msg_type: u8, payload: Vec<u8>) {
        let msg = MyMessage { msg_type, payload: payload.clone() };
        
        let mut buf = BytesMut::new();
        MyCodec.encode(msg, &mut buf).unwrap();
        
        let decoded = MyCodec.decode(&mut buf).unwrap().unwrap();
        prop_assert_eq!(decoded.msg_type, msg_type);
        prop_assert_eq!(decoded.payload, payload);
    }
}
```

Random inputs → catch edge cases (empty payload, max size, etc.).

## 15.4 Fault injection

```rust
// Slow connection simulation:
struct SlowStream<S: AsyncRead + AsyncWrite> {
    inner: S,
    delay: Duration,
}

// Add latency to each read/write
```

Or use external tools:
```bash
# tc (traffic control) — simulate slow network:
sudo tc qdisc add dev lo root netem delay 100ms loss 1%

# Run tests
# Undo:
sudo tc qdisc del dev lo root
```

`netem` injects latency, loss, duplication, reorder. Test resilience.

## 15.5 Load testing

```bash
# wrk2 — HTTP load
wrk -t12 -c400 -d30s -R10000 http://localhost:8080/

# tcpkali — TCP load
tcpkali -m "TEST" -c 1000 localhost:8080

# iperf3 — bandwidth
iperf3 -s   # server
iperf3 -c localhost -t 30   # client, 30s
```

Test under load before production.

---

# Tầng 16: Patterns & antipatterns

## 16.1 ✅ Pattern: Bounded channels

```rust
let (tx, rx) = mpsc::channel(100);   // not unbounded!
```

Backpressure built-in. Avoid OOM on slow consumers.

## 16.2 ✅ Pattern: Connection pooling

```rust
// reqwest: built-in pool
let client = reqwest::Client::new();  // reuse for all requests
```

Don't create client per request — defeats pool.

## 16.3 ✅ Pattern: Graceful shutdown

```rust
use tokio::signal;

let server = tokio::spawn(async move { run_server().await });

signal::ctrl_c().await?;

// Send shutdown signal to server
shutdown_tx.send(()).ok();

// Wait for in-flight requests:
tokio::time::timeout(Duration::from_secs(30), server).await??;
```

Don't drop connections mid-request.

## 16.4 ✅ Pattern: Timeouts everywhere

```rust
// Server side:
let result = timeout(Duration::from_secs(30), handle_request(stream)).await;

// Client side:
let resp = timeout(Duration::from_secs(10), client.get(url).send()).await;
```

Without timeouts, slow clients hold resources forever.

## 16.5 ✅ Pattern: Connection limit per service

```rust
let semaphore = Arc::new(Semaphore::new(1000));

loop {
    let (socket, _) = listener.accept().await?;
    let permit = semaphore.clone().acquire_owned().await.unwrap();
    
    tokio::spawn(async move {
        handle(socket).await;
        drop(permit);
    });
}
```

## 16.6 ❌ Antipattern: Unbounded channels

```rust
let (tx, rx) = mpsc::unbounded_channel();
```

No backpressure. Memory grows if consumer slow. OOM.

## 16.7 ❌ Antipattern: Sync I/O in async context

```rust
async fn handler() {
    std::fs::read("file");   // ❌ blocks executor!
}
```

✅ `tokio::fs::read()` or `tokio::task::spawn_blocking()`.

## 16.8 ❌ Antipattern: No timeout on external call

```rust
client.get(url).send().await?;  // could hang forever
```

✅ Always wrap in `tokio::time::timeout`.

## 16.9 ❌ Antipattern: Per-request connection

```rust
async fn fetch(url: &str) {
    let client = reqwest::Client::new();   // new pool every time!
    client.get(url).send().await
}
```

✅ Reuse client (which has pool internally).

## 16.10 ❌ Antipattern: Reading "until disconnect" without timeout

```rust
loop {
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 { break; }   // disconnect
    // ... process ...
}
```

What if client opens connection but never sends? Server hangs forever.

✅ Per-read timeout + total timeout.

## 16.11 ❌ Antipattern: Trusting client-provided sizes

```rust
let len = u32::from_be_bytes(header) as usize;
let mut buf = vec![0u8; len];  // attacker sends len=4GB → OOM!
```

✅ Validate length against max:
```rust
if len > MAX_FRAME_SIZE {
    return Err(...);
}
```

## 16.12 ❌ Antipattern: TCP keep-alive missing

Long idle connection may be dropped by middlebox. Next send fails.

✅ Enable TCP keepalive or app-level heartbeat:
```rust
socket.set_keepalive(true)?;
```

---

# Tổng kết — 12 nguyên tắc senior Networking

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. tokio::net for 99% async network code. mio cho extreme cases. │
│                                                                  │
│ 2. Bounded channels for backpressure. Never unbounded.           │
│                                                                  │
│ 3. Timeouts EVERYWHERE — connect, read, write, total request.    │
│                                                                  │
│ 4. Validate length / size of incoming data. Anti-DoS.            │
│                                                                  │
│ 5. Connection pooling for outbound. reuse, not create per req.   │
│                                                                  │
│ 6. Framing strategy: length-prefixed most common.                │
│                                                                  │
│ 7. tokio_util::codec for typed protocols.                         │
│                                                                  │
│ 8. Graceful shutdown — finish in-flight before exit.             │
│                                                                  │
│ 9. Circuit breaker for unreliable upstream.                      │
│                                                                  │
│ 10. Retry with backoff + jitter. Avoid thundering herd.          │
│                                                                  │
│ 11. TCP_NODELAY for interactive. SO_REUSEPORT for multi-process. │
│                                                                  │
│ 12. Tune kernel + OS limits for production. ulimit -n, sysctl.   │
└──────────────────────────────────────────────────────────────────┘
```

---

# Networking toolkit

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime + TCP/UDP |
| `tokio-util` | Codec framework |
| `mio` | Low-level async I/O |
| `socket2` | Socket options, raw sockets |
| `bytes` | Zero-copy byte buffers |
| `hyper` | HTTP/1.1 + HTTP/2 |
| `reqwest` | HTTP client |
| `quinn` / `s2n-quic` | QUIC implementation |
| `h3` | HTTP/3 |
| `tokio-tungstenite` | WebSocket |
| `tonic` | gRPC |
| `smoltcp` | Userspace TCP/IP |
| `libp2p` | Peer-to-peer |
| `pnet` | Packet manipulation |
| `trust-dns` / `hickory-dns` | DNS |
| `bb8` / `deadpool` | Connection pools |
| `backoff` | Retry strategies |
| `failsafe` | Circuit breaker |
| `tokio-console` | Async profiling |
| `httparse` | Fast HTTP parsing |
| `bincode` / `rmp-serde` | Binary serialization |

---

# Bộ tài liệu giờ có 23 chương!

```
   📚 RUST FOUNDATIONS LIBRARY
   
   a-s:  19 chương (foundations + apps)
   t. wasm
   u. cli-tools
   v. grpc-tonic
   w. networking         ← MỚI
   
   Tổng: 23 chương × 2 = 46 files
```

🦀 Bộ kỹ năng full-domain Rust:
- 🌐 Web REST (axum)
- 🗄️ Database (sqlx)
- 🔌 Embedded (no_std)
- 🖥️ Desktop (Tauri)
- 📱 Mobile (Tauri v2)
- 🌍 WASM (browser/edge)
- 📟 CLI tools
- 🔌 gRPC microservices
- 🔗 **Networking** (custom protocols, low-level) ← MỚI

Còn nhiều domain để đào tiếp:
- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced)
- **Cryptography** (rustls, ring)
- **OS kernels** (Redox)

Báo nếu muốn tiếp! 🦀⚡
