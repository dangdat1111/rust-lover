# Networking — Minh Hoạ Trực Quan

> Companion visual cho [w-networking.md](./w-networking.md). Đọc song song.

---

## 1. Bức tranh lớn — Networking Universe

```
                          NETWORKING TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   ┌────────────────────────────────────────────────┐    │
       │   │ APPLICATION layer (HTTP, gRPC, custom)         │    │
       │   ├────────────────────────────────────────────────┤    │
       │   │ TRANSPORT (TCP, UDP, QUIC)                     │    │
       │   ├────────────────────────────────────────────────┤    │
       │   │ NETWORK (IP)                                   │    │
       │   ├────────────────────────────────────────────────┤    │
       │   │ LINK (Ethernet, WiFi)                          │    │
       │   └────────────────────────────────────────────────┘    │
       │                                                        │
       │   Rust libraries by layer:                             │
       │                                                        │
       │   • Application: axum, hyper, tonic, custom            │
       │   • Transport:   tokio::net, mio, quinn (QUIC)         │
       │   • Network/Low: smoltcp (userspace IP), pnet          │
       │                                                        │
       │   Common needs:                                        │
       │   ┌─────────────────────────────────────────────┐      │
       │   │ • Async I/O scale (millions of connections) │      │
       │   │ • Custom binary protocols                   │      │
       │   │ • Framing & encoding                        │      │
       │   │ • Connection pooling                        │      │
       │   │ • Backpressure                              │      │
       │   │ • Resilience (retry, circuit breaker)       │      │
       │   │ • Performance tuning                        │      │
       │   └─────────────────────────────────────────────┘      │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. TCP/IP protocol stack

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Layer 7-5: APPLICATION                                 │
   │   ──────────────                                          │
   │     HTTP, HTTPS, gRPC, WebSocket, MQTT, SMTP, DNS        │
   │                       │                                  │
   │                       ▼                                  │
   │   Layer 4: TRANSPORT                                     │
   │   ──────────────────                                     │
   │     ┌────────────┐  ┌────────────┐  ┌────────────┐      │
   │     │ TCP        │  │ UDP        │  │ QUIC       │      │
   │     │ reliable   │  │ best-effort│  │ multi-     │      │
   │     │ ordered    │  │ fast       │  │ stream     │      │
   │     │ handshake  │  │ no handshake│ │ over UDP   │      │
   │     └────────────┘  └────────────┘  └────────────┘      │
   │                       │                                  │
   │                       ▼                                  │
   │   Layer 3: NETWORK                                       │
   │   ──────────────                                          │
   │     IP (IPv4, IPv6) — routing across networks            │
   │                       │                                  │
   │                       ▼                                  │
   │   Layer 2-1: LINK / PHYSICAL                             │
   │   ──────────────────────                                 │
   │     Ethernet, WiFi, Cellular — bits on wire/air          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 3. TCP 3-way handshake

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     ├──── SYN seq=100 ──────────────►│                  │
   │     │                                 │                  │
   │     │◄──── SYN-ACK seq=200, ack=101 ─│                  │
   │     │                                 │                  │
   │     ├──── ACK ack=201 ──────────────►│                  │
   │     │                                 │                  │
   │   ── Connection established ──        │                  │
   │     │                                 │                  │
   │     ├──── DATA ───────────────────────►│                │
   │     │◄──── DATA ────────────────────│                  │
   │     │ ...                                                │
   │     │                                                    │
   │   Connection close (4-way):                              │
   │     ├──── FIN ────────────────────────►│                │
   │     │◄──── ACK ────────────────────│                    │
   │     │◄──── FIN ────────────────────│                    │
   │     ├──── ACK ────────────────────────►│                │
   │                                                          │
   │   Setup: 1 RTT (~50-100ms cross-continent)               │
   │                                                          │
   │   ⟹ Why connection REUSE matters:                        │
   │     • HTTP/2 multiplexing                                │
   │     • Connection pools                                   │
   │     • Keep-alive                                         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. Sockets in Rust — Layers

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   YOUR APP                                               │
   │      │                                                   │
   │      ▼                                                   │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ HIGH-LEVEL (recommended)                     │       │
   │   │                                              │       │
   │   │   tokio::net::TcpListener                    │       │
   │   │   tokio::net::TcpStream                      │       │
   │   │   tokio::net::UdpSocket                      │       │
   │   │                                              │       │
   │   │   Async, ergonomic, integrated with tokio    │       │
   │   └──────────────────────────────────────────────┘       │
   │      │                                                   │
   │      ▼ uses                                              │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ LOW-LEVEL                                    │       │
   │   │                                              │       │
   │   │   mio::net::TcpListener                      │       │
   │   │   mio::Poll                                  │       │
   │   │                                              │       │
   │   │   Manual event loop, OS-level                │       │
   │   └──────────────────────────────────────────────┘       │
   │      │                                                   │
   │      ▼ uses                                              │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ OS API                                       │       │
   │   │                                              │       │
   │   │   Linux: epoll, io_uring                     │       │
   │   │   macOS/BSD: kqueue                          │       │
   │   │   Windows: IOCP                              │       │
   │   │                                              │       │
   │   │   Native syscalls                            │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   │   For most apps: use tokio::net                          │
   │   For custom runtimes: mio                               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 5. Async TCP server với tokio

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   #[tokio::main]                                         │
   │   async fn main() -> std::io::Result<()> {               │
   │       let listener = TcpListener::bind(                  │
   │           "0.0.0.0:8080"                                 │
   │       ).await?;                                          │
   │                                                          │
   │       loop {                                             │
   │           let (socket, addr) = listener.accept().await?; │
   │           //  ┌────────────────────────────────┐         │
   │           //  │ Each connection = task         │         │
   │           //  │ Tasks are CHEAP (~hundreds B)  │         │
   │           //  │ Scales to MILLIONS             │         │
   │           //  └────────────────────────────────┘         │
   │           tokio::spawn(async move {                       │
   │               handle_client(socket).await                │
   │           });                                            │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   How tokio scales:                                      │
   │                                                          │
   │   Traditional thread-per-connection:                     │
   │   ┌──────────┐ ┌──────────┐ ┌──────────┐                │
   │   │ Thread 1 │ │ Thread 2 │ │ Thread N │                │
   │   │ 8MB stack│ │ 8MB stack│ │ 8MB stack│                │
   │   └──────────┘ └──────────┘ └──────────┘                │
   │   10k conns = 80GB RAM ❌                                │
   │                                                          │
   │   Tokio async tasks:                                     │
   │   ┌────────────────────────────────────┐                │
   │   │ Single thread pool (1 per core)    │                │
   │   │                                    │                │
   │   │  ┌─────┐ ┌─────┐ ┌─────┐ ... ┌─────┐│                │
   │   │  │T1   │ │T2   │ │T3   │     │T10k││                │
   │   │  │~200B│ │~200B│ │~200B│     │~200B││                │
   │   │  └─────┘ └─────┘ └─────┘     └─────┘│                │
   │   │  Schedule on epoll events            │                │
   │   └────────────────────────────────────┘                │
   │   1M conns = ~200MB RAM ✅                                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 6. mio — Low-level event loop

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   mio architecture:                                      │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Poll (mio::Poll)                             │       │
   │   │                                              │       │
   │   │   Wraps OS API:                              │       │
   │   │   • Linux: epoll                             │       │
   │   │   • BSD/macOS: kqueue                        │       │
   │   │   • Windows: IOCP                            │       │
   │   └──────────────────────────────────────────────┘       │
   │                       ▲                                  │
   │                       │ register sources                 │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Sources                                      │       │
   │   │   TcpListener, TcpStream, UdpSocket,         │       │
   │   │   Waker (for cross-thread wake)              │       │
   │   └──────────────────────────────────────────────┘       │
   │                       │                                  │
   │                       │ each has Token (id)              │
   │                       ▼                                  │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Events buffer                                │       │
   │   │   ready events: (token, readiness)           │       │
   │   └──────────────────────────────────────────────┘       │
   │                       │                                  │
   │                       │ poll() drains events             │
   │                       ▼                                  │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ YOUR EVENT LOOP                              │       │
   │   │                                              │       │
   │   │   loop {                                     │       │
   │   │     poll.poll(&mut events, None)?;            │      │
   │   │     for event in &events {                    │      │
   │   │       match event.token() {                   │      │
   │   │         SERVER => accept(),                   │      │
   │   │         tk => handle_conn(tk),                │      │
   │   │       }                                       │      │
   │   │     }                                         │      │
   │   │   }                                           │      │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   │   ⟹ tokio uses mio internally                            │
   │     Most apps: use tokio. Drop to mio for special cases. │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. Framing strategies

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TCP = byte stream, not message stream.                 │
   │   Must FRAME messages.                                   │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   1. FIXED-LENGTH                                        │
   │   ────────────                                            │
   │   [msg1 4B][msg2 4B][msg3 4B]                            │
   │                                                          │
   │   Simple. Wastes if data varies.                         │
   │   Used for fixed-size headers.                           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. LENGTH-PREFIXED (most common)                       │
   │   ─────────────────                                       │
   │   [len: 4B][payload: N bytes][len][payload][...]          │
   │                                                          │
   │   ┌──────┬──────────────┬──────┬──────────┐              │
   │   │ 5    │ "hello"      │ 4    │ "test"   │              │
   │   └──────┴──────────────┴──────┴──────────┘              │
   │                                                          │
   │   ✅ Variable size, easy to parse                        │
   │   ⚠️ Must validate length (DoS)                           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. DELIMITER-BASED                                     │
   │   ─────────────                                          │
   │   msg1\nmsg2\nmsg3\n                                     │
   │                                                          │
   │   Used by: HTTP, SMTP (text protocols)                   │
   │   Issue: must escape delimiter in payload                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   4. PROTOCOL-SPECIFIC                                   │
   │   ───────────────                                        │
   │   HTTP/1.1: \r\n\r\n separates headers from body         │
   │             Content-Length header → body size            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   5. SELF-DESCRIBING                                     │
   │   ────────────                                           │
   │   Protobuf has internal framing (varint length).          │
   │   Just need transport framing.                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. tokio_util::codec — Framed pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   YOUR APP                                               │
   │     │                                                    │
   │     │ Send / receive typed Message                       │
   │     ▼                                                    │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Framed<TcpStream, MyCodec>                   │       │
   │   │                                              │       │
   │   │   • Sink<Message> (send)                     │       │
   │   │   • Stream<Item = Result<Message>> (recv)    │       │
   │   │                                              │       │
   │   │   Handles:                                   │       │
   │   │   • Buffering bytes                          │       │
   │   │   • Calling codec.decode() repeatedly        │       │
   │   │     until full message                       │       │
   │   │   • Calling codec.encode() to send           │       │
   │   └──────────────────────────────────────────────┘       │
   │              │                       ▲                   │
   │              │ encode()              │ decode()           │
   │              ▼                       │                    │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ MyCodec (implements Encoder + Decoder)       │       │
   │   │                                              │       │
   │   │   encode(msg, &mut BytesMut)                 │       │
   │   │     → write bytes to buffer                  │       │
   │   │                                              │       │
   │   │   decode(&mut BytesMut)                      │       │
   │   │     → Some(msg) if enough bytes              │       │
   │   │     → None if need more                      │       │
   │   └──────────────────────────────────────────────┘       │
   │              │                       ▲                   │
   │              ▼                       │                    │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ TcpStream (raw bytes)                        │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. Decoder pattern flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TCP byte stream arrives in chunks:                     │
   │                                                          │
   │   Time t=1: receive [1, 0, 0, 0, 5, h, e]                │
   │   ────────                                                │
   │   Buffer: [1, 0, 0, 0, 5, h, e]                          │
   │                                                          │
   │   call decode(&mut buffer):                              │
   │     • Read header: type=1, len=5                         │
   │     • Need 5 payload bytes, have 2                       │
   │     • Return Ok(None) — need more                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Time t=2: receive [l, l, o, 2, 0, 0]                   │
   │   ────────                                                │
   │   Buffer: [1, 0, 0, 0, 5, h, e, l, l, o, 2, 0, 0]        │
   │                                                          │
   │   call decode(&mut buffer):                              │
   │     • Header: type=1, len=5                              │
   │     • Have all 5 payload bytes "hello"                   │
   │     • Consume [1,0,0,0,5,h,e,l,l,o] (10 bytes)           │
   │     • Buffer becomes: [2, 0, 0]                          │
   │     • Return Ok(Some(Message{type:1, payload:"hello"}))  │
   │                                                          │
   │   call decode again:                                     │
   │     • Buffer: [2, 0, 0] — partial header                 │
   │     • Need 5 header bytes, have 3                        │
   │     • Return Ok(None) — wait for more                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Time t=3: receive [0, 4, t, e, s, t]                   │
   │   ────────                                                │
   │   Buffer: [2, 0, 0, 0, 4, t, e, s, t]                    │
   │                                                          │
   │   call decode:                                           │
   │     • Header: type=2, len=4                              │
   │     • Have all 4 bytes "test"                            │
   │     • Return Ok(Some(Message{type:2, payload:"test"}))   │
   │                                                          │
   │   Framework handles partial reads + retry automatically. │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. UDP vs TCP comparison

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   TCP (reliable stream):                                 │
   │   ─────────                                              │
   │   ┌────────────────────────────────────────────┐         │
   │   │  Client          ───►          Server      │         │
   │   │     │                              │       │         │
   │   │     ├── SYN ────────────────────►│         │         │
   │   │     │◄── SYN-ACK ─────────────────│         │         │
   │   │     ├── ACK ────────────────────►│         │         │
   │   │     │                              │       │         │
   │   │     ├── data1 ───────────────────►│         │         │
   │   │     │◄── ACK ──────────────────────│        │         │
   │   │     ├── data2 ───────────────────►│         │         │
   │   │     │   (retransmit if loss)       │         │        │
   │   │     ├── data3 ───────────────────►│         │         │
   │   │                                              │        │
   │   │   ✅ Ordered, reliable, no dups              │        │
   │   │   ⚠️ Setup cost, head-of-line blocking        │        │
   │   └────────────────────────────────────────────┘         │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   UDP (datagrams):                                       │
   │   ────────                                                │
   │   ┌────────────────────────────────────────────┐         │
   │   │  Client          ───►          Server      │         │
   │   │     │                              │       │         │
   │   │     │ (no setup)                  │       │         │
   │   │     ├── packet1 ─────────────────►│         │         │
   │   │     ├── packet2 ─────────────────►│         │         │
   │   │     ├── packet3 ─────────────────►│         │         │
   │   │     │                              │       │         │
   │   │     (some may be lost, reordered, duplicated)│        │
   │   │     (no retransmission)                       │      │
   │   │                                              │       │
   │   │   ✅ Fast, no setup, low overhead           │        │
   │   │   ⚠️ Unreliable, app must handle              │       │
   │   └────────────────────────────────────────────┘         │
   │                                                          │
   │   Use TCP: web, DB, email (need reliability)              │
   │   Use UDP: DNS, gaming, video (need speed)                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 11. smoltcp — Userspace TCP/IP

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Normal Linux app:                                      │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ App: tokio::net::TcpStream                   │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │ syscall                            │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ KERNEL: Linux network stack                  │       │
   │   │   TCP, IP, Ethernet processing               │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │                                     │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ NIC driver, hardware                         │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   smoltcp (no_std):                                      │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ App: smoltcp::socket::tcp::Socket            │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │ (no syscall, direct call)          │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ smoltcp: Interface + SocketSet                │      │
   │   │   • TCP state machine                        │       │
   │   │   • IP packet routing                        │       │
   │   │   • Ethernet framing                         │       │
   │   │   All in USERSPACE                           │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │                                     │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Device trait (you implement)                 │       │
   │   │   receive(): raw frame from hardware          │      │
   │   │   transmit(): raw frame to hardware           │      │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │                                     │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │ Custom NIC driver, USB ethernet,             │       │
   │   │ TUN/TAP, even MQTT-as-transport               │      │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   │   Use cases:                                             │
   │   • Embedded device with network (no OS)                 │
   │   • Custom NIC driver                                    │
   │   • Unikernel / specialized OS                           │
   │   • AWS Firecracker (VMM)                                │
   │   • Embassy embedded async network                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 12. WebSocket flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     │ HTTP/1.1 GET /ws                │                  │
   │     │ Upgrade: websocket              │                  │
   │     │ Sec-WebSocket-Key: dGhlIHNhb... │                  │
   │     ├────────────────────────────────►│                  │
   │     │                                 │                  │
   │     │ HTTP/1.1 101 Switching Protocols│                  │
   │     │ Upgrade: websocket              │                  │
   │     │ Sec-WebSocket-Accept: s3pPLM...  │                 │
   │     │◄────────────────────────────────│                  │
   │     │                                 │                  │
   │     │ ━━━ now WebSocket protocol ━━━ │                  │
   │     │                                 │                  │
   │     ├── text frame "hello" ─────────►│                  │
   │     │◄── text frame "world" ──────────│                  │
   │     ├── binary frame [bytes] ───────►│                  │
   │     │◄── binary frame [bytes] ─────────│                  │
   │     │      ...                                            │
   │     │                                                    │
   │     ├── close frame ────────────────►│                  │
   │     │◄────── close ──────────────────│                  │
   │     │                                                    │
   │                                                          │
   │   Use cases:                                             │
   │   • Chat apps                                            │
   │   • Live notifications                                   │
   │   • Real-time dashboards                                 │
   │   • Multiplayer games (browser)                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 13. HTTP/3 vs HTTP/2 vs HTTP/1.1

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   HTTP/1.1                                               │
   │   ────────                                                │
   │     1 request per connection (or pipelining w/o reorder) │
   │     Connection: keep-alive (reuse for next request)       │
   │     Head-of-line blocking                                │
   │                                                          │
   │     [Req1] ────► [Resp1] [Req2] ────► [Resp2] ...        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   HTTP/2 (over TCP)                                      │
   │   ──────                                                  │
   │     Multiplex streams over 1 TCP                         │
   │     Binary framing                                       │
   │     Header compression (HPACK)                           │
   │                                                          │
   │     [Stream 1: Req1] ⇒ ...                                │
   │     [Stream 3: Req2] ⇒ ...    (concurrent)              │
   │     [Stream 5: Req3] ⇒ ...                                │
   │                                                          │
   │     ⚠️ TCP head-of-line blocking still!                  │
   │     If packet lost, all streams wait for retransmit       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   HTTP/3 (over QUIC over UDP)                             │
   │   ──────                                                  │
   │     Independent streams (no TCP HoL)                     │
   │     TLS 1.3 built-in                                     │
   │     0-RTT connection resumption                          │
   │     Connection migration (network change)                │
   │     Multiplexing without packet-level coupling           │
   │                                                          │
   │     [QUIC stream 1: Req1] ⇒                              │
   │     [QUIC stream 2: Req2] ⇒  Independent!                │
   │     [QUIC stream 3: Req3] ⇒  Loss in one ≠ wait others   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Rust crates:
   ────────────
   • hyper        → HTTP/1.1, HTTP/2
   • quinn        → QUIC
   • h3           → HTTP/3 over QUIC
   • reqwest      → HTTP client (1.1, 2, optional 3)
```

---

## 14. Connection pooling

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Without pool:                                          │
   │   ─────────                                              │
   │                                                          │
   │   Each request:                                          │
   │   1. TCP connect (~50ms cross-region)                    │
   │   2. TLS handshake (~50ms)                               │
   │   3. Request                                             │
   │   4. Response                                            │
   │   5. Close                                               │
   │                                                          │
   │   Total overhead: ~100ms per request                     │
   │                                                          │
   │   1000 requests = 1000 connects, 100 seconds wasted      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   With pool:                                             │
   │   ───────                                                 │
   │                                                          │
   │   ┌─────────────────────────────────────────────┐        │
   │   │ Pool (20 connections to host)               │        │
   │   │                                             │        │
   │   │  ┌────┐ ┌────┐ ┌────┐ ┌────┐                │        │
   │   │  │ C1 │ │ C2 │ │ C3 │ │ C4 │ ...           │        │
   │   │  │idle│ │busy│ │idle│ │busy│                │        │
   │   │  └────┘ └────┘ └────┘ └────┘                │        │
   │   └─────────────────────────────────────────────┘        │
   │                                                          │
   │   Each request:                                          │
   │   1. Get connection from pool (~µs)                      │
   │   2. Send request                                        │
   │   3. Receive response                                    │
   │   4. Return connection to pool                           │
   │                                                          │
   │   Overhead: ~µs per request after warmup                 │
   │                                                          │
   │   reqwest::Client has pool built-in.                     │
   │   For custom protocols: bb8, deadpool.                   │
   │                                                          │
   │   ⚠️ Don't create new Client per request!                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 15. Backpressure pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ❌ WITHOUT BACKPRESSURE:                               │
   │                                                          │
   │   Producer ──fast──► [unbounded queue] ──slow──► Consumer│
   │                            │                              │
   │                            │ grows infinitely             │
   │                            ▼                              │
   │                       OOM crash 💥                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ WITH BACKPRESSURE:                                  │
   │                                                          │
   │   Producer ─► [bounded queue cap=100] ──► Consumer        │
   │       │                │                    │            │
   │       │                │ when full,         │            │
   │       │                │ send() awaits      │            │
   │       │                ▼                                  │
   │       │           Producer blocks                        │
   │       │                                                  │
   │       └─ slows down naturally                            │
   │                                                          │
   │   Code:                                                  │
   │   ──────                                                 │
   │   let (tx, mut rx) = mpsc::channel::<Msg>(100);          │
   │   //                                       ^^^           │
   │   //                            bounded capacity         │
   │                                                          │
   │   // Producer:                                           │
   │   tokio::spawn(async move {                              │
   │       loop {                                             │
   │           let msg = produce().await;                     │
   │           tx.send(msg).await?;   // blocks if full       │
   │       }                                                  │
   │   });                                                    │
   │                                                          │
   │   // Consumer:                                           │
   │   while let Some(msg) = rx.recv().await {                │
   │       process(msg).await;        // slow                 │
   │   }                                                      │
   │                                                          │
   │   Memory bounded. System self-regulates.                 │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 16. Circuit breaker pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   States:                                                │
   │                                                          │
   │       ┌──────────┐                                       │
   │       │  CLOSED  │  ← normal, requests pass through       │
   │       │          │    count failures                      │
   │       └────┬─────┘                                       │
   │            │ threshold exceeded                          │
   │            ▼                                             │
   │       ┌──────────┐                                       │
   │       │   OPEN   │  ← all requests fail FAST              │
   │       │          │    (without hitting upstream)          │
   │       └────┬─────┘                                       │
   │            │ timeout expires                              │
   │            ▼                                             │
   │       ┌──────────┐                                       │
   │       │HALF-OPEN │  ← let 1 request through to test       │
   │       │          │                                        │
   │       └────┬─────┘                                       │
   │            │                                              │
   │     ┌──────┴──────┐                                       │
   │     │             │                                       │
   │   success       failure                                  │
   │     │             │                                       │
   │     ▼             ▼                                       │
   │   CLOSED        OPEN                                     │
   │   (recovered)  (still broken, wait more)                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Benefit:                                               │
   │                                                          │
   │   When upstream service down:                            │
   │   • Without breaker: every request takes 30s timeout     │
   │   • With breaker: fail fast (ms), no resource exhaust    │
   │                                                          │
   │   Prevents cascading failure across services.            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Network problems & solutions matrix

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Problem                  │ Solution                    │
   │   ───────                  │ ─────────                   │
   │                                                          │
   │   Slow request (latency)   │ • CDN / edge computing       │
   │                             │ • Connection pool            │
   │                             │ • HTTP/2 multiplexing        │
   │                             │ • Caching                    │
   │                                                          │
   │   Packet loss              │ TCP retransmits automatically│
   │                             │ App-level: retry + backoff  │
   │                                                          │
   │   Connection limit          │ Async (tokio) — millions of  │
   │                             │   connections                │
   │                             │ Increase ulimit -n           │
   │                             │ App-level semaphore limit    │
   │                                                          │
   │   NAT timeout              │ TCP keepalive                │
   │                             │ App heartbeat ping           │
   │                                                          │
   │   DNS slow                 │ DNS cache (hickory-dns)      │
   │                             │ Use IP directly when known   │
   │                                                          │
   │   TLS handshake slow       │ TLS 1.3, session resumption  │
   │                             │ Connection reuse             │
   │                                                          │
   │   Slowloris attack         │ Read timeout per byte         │
   │                             │ Total request timeout         │
   │                                                          │
   │   DDoS                     │ Rate limiting                 │
   │                             │ Load balancer                 │
   │                             │ Cloudflare / WAF              │
   │                                                          │
   │   Backpressure missing      │ Bounded channels             │
   │                             │ Semaphore for resource limits│
   │                                                          │
   │   Upstream failures         │ Circuit breaker              │
   │                             │ Retry with backoff           │
   │                             │ Bulkhead (isolate resources) │
   │                                                          │
   │   Stale connections         │ Connection health check       │
   │                             │ Idle timeout, max lifetime   │
   │                             │ TCP keepalive                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. Performance tuning

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   SOCKET OPTIONS:                                        │
   │                                                          │
   │   • TCP_NODELAY        → disable Nagle, lower latency    │
   │   • SO_REUSEADDR        → reuse port quickly             │
   │   • SO_REUSEPORT        → multi-process listen            │
   │   • SO_KEEPALIVE        → detect dead peers              │
   │   • TCP_USER_TIMEOUT    → app-level TCP timeout          │
   │   • SO_RCVBUF/SNDBUF   → buffer sizes                    │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   KERNEL TUNING (Linux):                                 │
   │                                                          │
   │   $ sysctl -w net.core.somaxconn=4096                    │
   │     # listen() backlog                                   │
   │                                                          │
   │   $ sysctl -w net.ipv4.tcp_max_syn_backlog=8192          │
   │     # SYN queue size                                     │
   │                                                          │
   │   $ sysctl -w net.ipv4.tcp_tw_reuse=1                    │
   │     # reuse TIME_WAIT sockets                             │
   │                                                          │
   │   $ sysctl -w fs.file-max=1000000                        │
   │     # max open files                                     │
   │                                                          │
   │   $ ulimit -n 1000000                                    │
   │     # per-process file descriptor limit                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ZERO-COPY:                                             │
   │                                                          │
   │   bytes::Bytes — Arc-backed buffer                        │
   │   ┌─────────────────────┐                                │
   │   │ Bytes (cheap clone) │                                │
   │   └──────────┬──────────┘                                │
   │              │ Arc                                        │
   │              ▼                                            │
   │   ┌─────────────────────┐                                │
   │   │ Underlying buffer   │ ← shared                       │
   │   └─────────────────────┘                                │
   │                                                          │
   │   clone() = O(1), no memcpy                              │
   │   Multiple consumers, single backing storage             │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 19. Network architecture patterns

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. CLIENT-SERVER                                       │
   │   ──────────────                                          │
   │                                                          │
   │   Many clients ──► One (or few) servers                  │
   │                                                          │
   │   Pros: Simple, centralized control                      │
   │   Cons: Single point of failure, scaling cost             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. MICROSERVICES                                       │
   │   ────────────────                                       │
   │                                                          │
   │   ┌──────┐    ┌──────┐    ┌──────┐                       │
   │   │User  │ ◄──►Auth  │ ◄──►Order │                       │
   │   │Svc   │    │Svc   │    │Svc   │                       │
   │   └──────┘    └──────┘    └──────┘                       │
   │       ▲           ▲           ▲                          │
   │       └───────────┼───────────┘                          │
   │                   │                                      │
   │             API Gateway                                  │
   │                   ▲                                      │
   │                   │                                      │
   │                Client                                    │
   │                                                          │
   │   Internal comm: gRPC / HTTP                             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. PEER-TO-PEER (P2P)                                  │
   │   ─────────────────                                      │
   │                                                          │
   │   No central server. Peers discover + comm directly.     │
   │                                                          │
   │   ┌──────┐    ┌──────┐                                   │
   │   │Peer A│◄──►│Peer B│                                   │
   │   └──┬───┘    └──┬───┘                                   │
   │      │            │                                      │
   │      └────────────┴──────────┐                           │
   │                                ▼                         │
   │                            ┌──────┐                      │
   │                            │Peer C│                      │
   │                            └──────┘                      │
   │                                                          │
   │   Examples: BitTorrent, IPFS, blockchain                 │
   │   Libraries: libp2p                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   4. PUB/SUB                                             │
   │   ──────                                                  │
   │                                                          │
   │   ┌───────────┐                                          │
   │   │Publisher  │──►topic──►┌──────────┐──►Subscriber 1   │
   │   └───────────┘            │  Broker  │──►Subscriber 2   │
   │   ┌───────────┐──►topic──►│  (Kafka, │──►Subscriber 3   │
   │   │Publisher 2│            │   NATS)  │                  │
   │   └───────────┘            └──────────┘                  │
   │                                                          │
   │   Decouple producers/consumers via topics.                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 20. Common antipatterns

```
   ❌ 1. Unbounded channels
   ──────────────────────
   let (tx, rx) = mpsc::unbounded_channel();
   // Memory grows if consumer slow → OOM
   
   ✅ Bounded:
   let (tx, rx) = mpsc::channel(100);
   
   
   ❌ 2. Sync I/O in async
   ─────────────────────
   async fn handle() {
       std::fs::read("file");   // blocks executor!
   }
   
   ✅ Async or spawn_blocking:
   tokio::fs::read("file").await;
   
   
   ❌ 3. No timeout
   ──────────────
   client.get(url).send().await?;   // could hang forever
   
   ✅ Wrap in timeout:
   tokio::time::timeout(Duration::from_secs(10),
       client.get(url).send()).await??;
   
   
   ❌ 4. New client per request
   ──────────────────────────
   async fn fetch(url: &str) {
       let client = reqwest::Client::new();  // new pool!
       client.get(url).send().await
   }
   
   ✅ Reuse client (built-in pool):
   let client = reqwest::Client::new();   // once
   client.get(url).send().await
   
   
   ❌ 5. Trust client-provided sizes
   ─────────────────────────────
   let len = u32::from_be_bytes(header) as usize;
   let mut buf = vec![0u8; len];   // attacker sends len=4GB!
   
   ✅ Validate:
   if len > MAX_FRAME_SIZE {
       return Err(...);
   }
   
   
   ❌ 6. Reading "until disconnect" no timeout
   ───────────────────────────────────
   loop {
       let n = stream.read(&mut buf).await?;
       // What if attacker opens conn and never sends?
       // Hangs forever
   }
   
   ✅ Per-read timeout:
   tokio::time::timeout(Duration::from_secs(30),
       stream.read(&mut buf)).await??;
   
   
   ❌ 7. Missing TCP keepalive on long-lived
   ────────────────────────────────────
   // NAT may silently drop conn after idle
   // Next send fails with "broken pipe"
   
   ✅ Enable keepalive:
   socket.set_keepalive(true)?;
```

---

## 21. Mind map cuối

```
                              NETWORKING
                                  │
        ┌────────────┬────────────┼────────────┬─────────────┐
        ▼            ▼            ▼            ▼             ▼
    PROTOCOLS    LIBRARIES    CUSTOM     RESILIENCE     PERFORMANCE
        │            │            │            │             │
    TCP/UDP      tokio        Framing     Backpressure  Connection pool
    HTTP/1/2/3   mio          Codecs      Circuit       TLS resume
    QUIC         hyper        smoltcp      breaker       SIMD parse
    WebSocket    tonic        libp2p      Retry+backoff Zero-copy
    DNS          quinn        Custom       Timeouts      Kernel tuning
                 reqwest      protocols    Bulkhead       io_uring
                 tokio-       MQTT         Pool limits   TCP_NODELAY
                 tungstenite                              SO_REUSEPORT
                 socket2
                 smoltcp
                 
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. tokio for 99% async networking   │
                │                                      │
                │  2. Bounded channels (backpressure)  │
                │                                      │
                │  3. TIMEOUTS EVERYWHERE              │
                │                                      │
                │  4. Validate sizes (anti-DoS)         │
                │                                      │
                │  5. Connection pool for outbound     │
                │                                      │
                │  6. Framing: length-prefixed common  │
                │                                      │
                │  7. tokio_util::codec for typed       │
                │                                      │
                │  8. Graceful shutdown                │
                │                                      │
                │  9. Circuit breaker for unreliable   │
                │                                      │
                │  10. Retry + backoff + jitter        │
                │                                      │
                │  11. TCP_NODELAY for interactive     │
                │                                      │
                │  12. Tune kernel limits (production) │
                └──────────────────────────────────────┘
```

---

## 22. Bộ tài liệu giờ có 23 chương!

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   a-s:  19 chương foundation + apps                      │
   │   t. wasm                                                │
   │   u. cli-tools                                           │
   │   v. grpc-tonic                                          │
   │   w. networking            ← MỚI                         │
   │      w-networking.md + visual                            │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng FULL networking:                         │
   │                                                          │
   │   🌐 HTTP (axum, hyper, reqwest)                         │
   │   📡 TCP/UDP (tokio::net)                                │
   │   🌍 WASM + Edge (Cloudflare Workers)                    │
   │   🔌 gRPC microservices (tonic)                          │
   │   🌐 WebSocket (tokio-tungstenite)                       │
   │   ⚡ QUIC/HTTP3 (quinn)                                  │
   │   🔌 P2P (libp2p)                                         │
   │   🛠️ Custom protocols (framing, codecs)                 │
   │   🔬 Userspace TCP/IP (smoltcp)                          │
   │   ⚙️ Low-level (mio, socket2)                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

🦀 Báo nếu muốn tiếp:
- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced)
- **Cryptography** (rustls, ring)
- **OS kernels** (Redox)
