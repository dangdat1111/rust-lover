# gRPC với tonic — Deep Dive

> Tài liệu thứ 22 (chương v) trong bộ Rust nền tảng. Đọc trước:
> - [f-async.md](./f-async.md) — tonic chạy trên tokio
> - [g-error-handling.md](./g-error-handling.md) — gRPC error patterns
> - [l-observability.md](./l-observability.md) — distributed tracing
> - [q-axum-project.md](./q-axum-project.md) — tonic dùng cùng Tower stack
>
> **gRPC** = high-performance RPC framework do Google phát triển, dựa trên:
> - **HTTP/2** (multiplexing, binary)
> - **Protocol Buffers** (binary serialization)
> - **Streaming** (unary, server-stream, client-stream, bidirectional)
> - **Cross-language** (Rust, Go, Java, Python, Node.js, ...)
>
> **tonic** = best gRPC implementation cho Rust:
> - Async/await native (tokio)
> - Built on **tower** (same stack as axum)
> - Type-safe codegen từ .proto files
> - Production-grade (used by Cloudflare, Discord, dropbox, etc.)
>
> So với REST:
> - **Faster** — binary protocol, HTTP/2 multiplexing
> - **Smaller** — protobuf ~3-10x nhỏ hơn JSON
> - **Type-safe** — schema-driven, codegen
> - **Streaming** — built-in (vs WebSocket hack cho REST)
> - **Cross-language** — same .proto → many language clients
>
> Use cases:
> - Microservices internal communication
> - Mobile ↔ backend (Android, iOS clients)
> - Real-time streaming (chat, telemetry)
> - High-performance APIs

---

# Mục lục

- [Tầng 1: gRPC concepts](#tầng-1-grpc-concepts)
- [Tầng 2: Protocol Buffers](#tầng-2-protocol-buffers)
- [Tầng 3: tonic setup & code generation](#tầng-3-tonic-setup--code-generation)
- [Tầng 4: Unary RPC — Request/response](#tầng-4-unary-rpc--requestresponse)
- [Tầng 5: Server-side streaming](#tầng-5-server-side-streaming)
- [Tầng 6: Client-side streaming](#tầng-6-client-side-streaming)
- [Tầng 7: Bidirectional streaming](#tầng-7-bidirectional-streaming)
- [Tầng 8: Error handling — Status codes](#tầng-8-error-handling--status-codes)
- [Tầng 9: Metadata — Headers + trailers](#tầng-9-metadata--headers--trailers)
- [Tầng 10: Interceptors & middleware](#tầng-10-interceptors--middleware)
- [Tầng 11: Authentication](#tầng-11-authentication)
- [Tầng 12: TLS](#tầng-12-tls)
- [Tầng 13: Health checks & reflection](#tầng-13-health-checks--reflection)
- [Tầng 14: Observability — Tracing, metrics](#tầng-14-observability--tracing-metrics)
- [Tầng 15: Testing gRPC services](#tầng-15-testing-grpc-services)
- [Tầng 16: gRPC-Web, load balancing, production](#tầng-16-grpc-web-load-balancing-production)

---

# Tầng 1: gRPC concepts

## 1.1 gRPC vs REST vs GraphQL

```
   ┌──────────────────────────────────────────────────────────┐
   │ Aspect          │ REST       │ GraphQL    │ gRPC        │
   ├──────────────────────────────────────────────────────────┤
   │ Protocol         │ HTTP/1.1   │ HTTP       │ HTTP/2     │
   │ Format           │ JSON       │ JSON       │ Protobuf   │
   │ Schema           │ OpenAPI    │ SDL        │ .proto     │
   │ Type-safe?      │ Loose      │ Strong (TS)│ Strong     │
   │ Streaming        │ SSE (hack) │ Subscript. │ Native     │
   │ Bidirectional   │ WebSocket  │ Subscript. │ Native     │
   │ Browser support │ Native     │ Native     │ via Web/proxy│
   │ Cross-language   │ ✅         │ ✅          │ ✅          │
   │ Best for        │ Public API │ Flex query │ Internal RPC│
   │                  │ Web apps   │ Mobile     │ Microservices│
   └──────────────────────────────────────────────────────────┘
```

## 1.2 gRPC architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   PROTOCOL BUFFERS (.proto file)                         │
   │                                                          │
   │   service UserService {                                  │
   │       rpc GetUser(GetUserRequest) returns (User);        │
   │       rpc ListUsers(Empty) returns (stream User);        │
   │   }                                                      │
   │                                                          │
   │              │ protoc / tonic-build compiles             │
   │              ▼                                           │
   │                                                          │
   │   ┌──────────────────────┐   ┌──────────────────────┐    │
   │   │ Generated Server     │   │ Generated Client      │   │
   │   │ trait UserService    │   │ struct UserService    │   │
   │   │ (Rust)               │   │ Client (Rust)         │   │
   │   └──────────┬───────────┘   └──────────┬───────────┘    │
   │              │                          │                │
   │              │ implement                │ use            │
   │              ▼                          ▼                │
   │   ┌──────────────────────┐   ┌──────────────────────┐    │
   │   │ Your server code     │   │ Your client code      │   │
   │   └──────────────────────┘   └──────────────────────┘    │
   │              │                          │                │
   │              │ HTTP/2 over TCP/TLS      │                │
   │              └──── tonic transport ─────┘                │
   │                          │                                │
   │                          ▼                                │
   │              ┌────────────────────────┐                  │
   │              │ Binary wire format     │                  │
   │              │ (Protobuf encoded)     │                  │
   │              └────────────────────────┘                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

## 1.3 4 types of RPC

```
   1. UNARY (request/response, like REST):
      ─────────
      Client ─── request ───►  Server
             ◄── response ───
      
   2. SERVER STREAMING:
      ─────────
      Client ─── request ───►  Server
             ◄── msg1 ──── 
             ◄── msg2 ──── 
             ◄── msg3 ──── 
             ◄── close ───
      
   3. CLIENT STREAMING:
      ─────────
      Client ─── msg1 ─────►  Server
             ──── msg2 ─────► 
             ──── msg3 ─────► 
             ──── close ───►
             ◄── response ───
      
   4. BIDIRECTIONAL STREAMING:
      ─────────
      Client ─── msg ──────►  Server
             ◄── msg ──────
             ─── msg ──────►
             ◄── msg ──────
                  ...
             (both sides freely send anytime)
```

## 1.4 Why HTTP/2?

```
   HTTP/1.1 problem:
   ──────────────
   • 1 request per TCP connection at a time (head-of-line blocking)
   • Text-based headers (parse cost)
   • No multiplexing
   • No server push
   
   HTTP/2 benefits:
   ────────────────
   • Multiplex many streams over 1 TCP connection
   • Binary frames (compact, fast parse)
   • Header compression (HPACK)
   • Server push
   • Stream prioritization
   
   gRPC leverages all of these.
```

## 1.5 Production users

```
   • Google (everything internally)
   • Cloudflare (Workers ↔ services)
   • Netflix (microservices)
   • Spotify (cross-service comm)
   • Discord (voice service)
   • dropbox (file storage backend)
   • Square (payments)
   • PagerDuty (event processing)
   • Many fintech, gaming companies
```

---

# Tầng 2: Protocol Buffers

## 2.1 .proto file syntax

```protobuf
syntax = "proto3";

package user.v1;

// User service definition
service UserService {
    rpc GetUser(GetUserRequest) returns (User);
    rpc CreateUser(CreateUserRequest) returns (User);
    rpc ListUsers(ListUsersRequest) returns (stream User);
    rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}

message User {
    int64 id = 1;
    string email = 2;
    string name = 3;
    int32 age = 4;
    Role role = 5;
    repeated string tags = 6;
    google.protobuf.Timestamp created_at = 7;
}

enum Role {
    ROLE_UNSPECIFIED = 0;
    ROLE_USER = 1;
    ROLE_ADMIN = 2;
}

message GetUserRequest {
    int64 id = 1;
}

message CreateUserRequest {
    string email = 1;
    string name = 2;
    int32 age = 3;
}

message ListUsersRequest {
    int32 limit = 1;
    string cursor = 2;
}

message ChatMessage {
    string from = 1;
    string content = 2;
}
```

## 2.2 Field types

```
   Scalar types:
   ─────────────
   int32, int64        — signed
   uint32, uint64       — unsigned
   sint32, sint64       — signed (varint, better for negatives)
   fixed32, fixed64     — fixed-size
   bool
   string               — UTF-8
   bytes                — binary
   float, double        — floating point
   
   Complex:
   ────────
   message Foo {}       — nested type
   enum Color {}        — enum
   repeated T           — list (T can be any type)
   map<K, V>            — map
   oneof choice {}      — union (one variant active)
   Option-like → use message wrapper or oneof
```

## 2.3 Field numbers

```protobuf
message User {
    int64 id = 1;       // field number 1
    string email = 2;
    string name = 3;
}
```

Field numbers identify fields **on the wire** (binary format).

**Rules**:
- 1-15: 1-byte encoding (use for most common fields)
- 16-2047: 2-byte
- **NEVER REUSE** numbers (breaks backward compat)
- **NEVER CHANGE** number of existing field (breaks readers)
- Reserved range: 19000-19999 (proto internal)

## 2.4 Schema evolution rules

```
   ✅ COMPATIBLE changes:
   ─────────────────
   • Add new optional fields (with new number)
   • Add new enum values (clients ignore unknown)
   • Add new services / methods
   • Remove fields (BUT reserve number)
   
   ❌ BREAKING changes:
   ───────────────
   • Change field number
   • Change field type (int32 ↔ int64 OK in some cases)
   • Rename fields (depends on language — Rust uses snake_case)
   • Make optional → required (proto3 no required, just defaults)
   • Remove and reuse field number
```

```protobuf
message User {
    reserved 4, 5;          // mark removed fields
    reserved "old_field";   // mark removed field name
    
    int64 id = 1;
    string email = 2;
    string name = 3;
    // 4, 5 removed — never reuse
    string phone = 6;       // new field
}
```

## 2.5 Well-known types

Google provides standard types:
```protobuf
import "google/protobuf/timestamp.proto";
import "google/protobuf/duration.proto";
import "google/protobuf/empty.proto";
import "google/protobuf/any.proto";
import "google/protobuf/wrappers.proto";

message Event {
    google.protobuf.Timestamp time = 1;
    google.protobuf.Duration duration = 2;
}

service Ping {
    rpc Ping(google.protobuf.Empty) returns (google.protobuf.Empty);
}
```

`Timestamp`, `Duration`, `Empty`, etc. — language-agnostic.

## 2.6 oneof — Union types

```protobuf
message Payment {
    int64 amount = 1;
    oneof method {
        Card card = 2;
        BankTransfer bank = 3;
        Crypto crypto = 4;
    }
}

message Card { string number = 1; }
message BankTransfer { string account = 1; }
message Crypto { string wallet = 1; }
```

Rust mapping:
```rust
pub struct Payment {
    pub amount: i64,
    pub method: Option<payment::Method>,
}

pub mod payment {
    pub enum Method {
        Card(Card),
        Bank(BankTransfer),
        Crypto(Crypto),
    }
}
```

Idiomatic Rust enum. Type-safe.

## 2.7 Binary format vs JSON

```
   Same message (User { id: 42, email: "alice", name: "Alice" }):
   
   JSON (~70 bytes):
   {"id":42,"email":"alice","name":"Alice"}
   
   Protobuf (~14 bytes):
   08 2a 12 05 61 6c 69 63 65 1a 05 41 6c 69 63 65
   │  │  │  │  ...
   │  │  │  └── len 5
   │  │  └── field 2 (email), wire type 2 (length-delimited)
   │  └── value 42
   └── field 1 (id), wire type 0 (varint)
   
   ~5x smaller. Faster to parse.
```

---

# Tầng 3: tonic setup & code generation

## 3.1 Project structure

```
my-grpc-service/
├── Cargo.toml
├── build.rs                  # Code gen at build time
├── proto/
│   └── user.proto            # Schema
└── src/
    ├── main.rs               # Server
    ├── lib.rs
    └── client.rs             # Optional client binary
```

## 3.2 Cargo.toml

```toml
[package]
name = "my-grpc-service"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "server"
path = "src/server.rs"

[[bin]]
name = "client"
path = "src/client.rs"

[dependencies]
tonic = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }
tonic-reflection = "0.12"
tonic-health = "0.12"

[build-dependencies]
tonic-build = "0.12"
```

## 3.3 build.rs — Code generation

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/generated")          // optional
        .compile_protos(
            &["proto/user.proto"],
            &["proto"],                     // include paths
        )?;
    Ok(())
}
```

Each build:
1. Read `proto/user.proto`
2. Generate Rust code (structs, enums, traits, clients)
3. Compile generated code with your code

## 3.4 Include generated code

```rust
// src/lib.rs

pub mod user {
    pub mod v1 {
        tonic::include_proto!("user.v1");   // package name in proto
    }
}

// Or specify path:
// include!("generated/user.v1.rs");
```

Now you have:
- `user::v1::UserServiceServer<T>` — trait wrap
- `user::v1::UserServiceClient<T>` — client
- `user::v1::User` — Rust struct
- `user::v1::GetUserRequest` — Rust struct
- etc.

## 3.5 Generated code structure

```rust
// What tonic-build generates (simplified):

pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub age: i32,
    pub role: i32,                 // enum as i32
    pub tags: Vec<String>,
    pub created_at: Option<prost_types::Timestamp>,
}

#[async_trait]
pub trait UserService: Send + Sync + 'static {
    async fn get_user(
        &self,
        request: tonic::Request<GetUserRequest>,
    ) -> Result<tonic::Response<User>, tonic::Status>;
    
    // ... other methods
}

// Server type:
pub struct UserServiceServer<T: UserService> { ... }

// Client type:
pub struct UserServiceClient<T> { ... }
```

## 3.6 tonic-build options

```rust
tonic_build::configure()
    .build_server(true)
    .build_client(true)
    .type_attribute(".", "#[derive(serde::Serialize)]")  // add derive to all types
    .field_attribute("User.email", "#[serde(rename = \"emailAddress\")]")
    .out_dir("src/generated")
    .compile_well_known_types(true)
    .compile_protos(&["proto/user.proto"], &["proto"])?;
```

Add custom derives, serde, etc. Important for JSON serialization too.

---

# Tầng 4: Unary RPC — Request/response

## 4.1 Server implementation

```rust
use tonic::{Request, Response, Status};
use my_grpc_service::user::v1::{
    user_service_server::{UserService, UserServiceServer},
    GetUserRequest, User,
};

#[derive(Default)]
pub struct MyUserService {
    // state: e.g., DB connection pool
}

#[tonic::async_trait]
impl UserService for MyUserService {
    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<User>, Status> {
        let req = request.into_inner();
        
        // Business logic
        if req.id <= 0 {
            return Err(Status::invalid_argument("id must be positive"));
        }
        
        // Fetch from DB
        let user = User {
            id: req.id,
            email: format!("user{}@example.com", req.id),
            name: "Alice".to_string(),
            age: 30,
            role: 1,
            tags: vec!["admin".to_string()],
            created_at: Some(prost_types::Timestamp::default()),
        };
        
        Ok(Response::new(user))
    }
}
```

## 4.2 Server startup

```rust
// src/server.rs
use tonic::transport::Server;
use my_grpc_service::user::v1::user_service_server::UserServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let svc = MyUserService::default();
    
    println!("Server listening on {}", addr);
    
    Server::builder()
        .add_service(UserServiceServer::new(svc))
        .serve(addr)
        .await?;
    
    Ok(())
}
```

Run:
```bash
cargo run --bin server
# Server listening on 0.0.0.0:50051
```

## 4.3 Client

```rust
// src/client.rs
use my_grpc_service::user::v1::{
    user_service_client::UserServiceClient,
    GetUserRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = UserServiceClient::connect("http://localhost:50051").await?;
    
    let request = tonic::Request::new(GetUserRequest { id: 42 });
    let response = client.get_user(request).await?;
    
    println!("User: {:?}", response.into_inner());
    
    Ok(())
}
```

Run:
```bash
cargo run --bin client
# User: User { id: 42, email: "user42@example.com", name: "Alice", ... }
```

## 4.4 Request/Response wrapper

```rust
// Server receives:
async fn get_user(
    &self,
    request: Request<GetUserRequest>,
) -> Result<Response<User>, Status> {
    // Access metadata:
    let metadata = request.metadata();
    let auth = metadata.get("authorization");
    
    // Access remote address:
    let addr = request.remote_addr();
    
    // Get inner message:
    let req: GetUserRequest = request.into_inner();
    
    // Build response:
    let user = User { ... };
    let mut response = Response::new(user);
    
    // Set response metadata:
    response.metadata_mut().insert("server-version", "1.0".parse()?);
    
    Ok(response)
}
```

`Request<T>` and `Response<T>` wrap actual message with metadata, extensions.

## 4.5 Multiple services on one server

```rust
Server::builder()
    .add_service(UserServiceServer::new(user_svc))
    .add_service(OrderServiceServer::new(order_svc))
    .add_service(BillingServiceServer::new(billing_svc))
    .serve(addr)
    .await?;
```

One port, multiple services. Like axum routes.

---

# Tầng 5: Server-side streaming

## 5.1 .proto definition

```protobuf
service NewsService {
    rpc Subscribe(SubscribeRequest) returns (stream NewsItem);
}

message NewsItem {
    string title = 1;
    string body = 2;
    int64 timestamp = 3;
}
```

`returns (stream NewsItem)` — server can send multiple responses.

## 5.2 Server implementation

```rust
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

type StreamResult<T> = Result<Response<ReceiverStream<Result<T, Status>>>, Status>;

#[tonic::async_trait]
impl NewsService for MyNewsService {
    type SubscribeStream = ReceiverStream<Result<NewsItem, Status>>;
    
    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        // Spawn task to send items
        tokio::spawn(async move {
            for i in 0..10 {
                let item = NewsItem {
                    title: format!("News {}", i),
                    body: "...".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                
                if tx.send(Ok(item)).await.is_err() {
                    break;   // client disconnected
                }
                
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
```

Pattern: channel + spawned task. Server sends items as they're ready.

## 5.3 Client consuming stream

```rust
let request = tonic::Request::new(SubscribeRequest {});
let mut stream = client.subscribe(request).await?.into_inner();

while let Some(item) = stream.message().await? {
    println!("Received: {:?}", item);
}
```

Or with `StreamExt`:
```rust
use tokio_stream::StreamExt;

let mut stream = client.subscribe(request).await?.into_inner();

while let Some(result) = stream.next().await {
    match result {
        Ok(item) => println!("Item: {:?}", item),
        Err(status) => eprintln!("Error: {}", status),
    }
}
```

## 5.4 Use cases for server streaming

- **News feed** — push items as they come
- **Logs / telemetry** — server pushes log lines
- **Server-sent updates** — notifications, alerts
- **Long-running operations** — report progress
- **Real-time data** — stock prices, sensor data
- **Pagination alternative** — stream through large dataset

vs REST: use Server-Sent Events (SSE) or WebSocket. gRPC streaming is cleaner.

---

# Tầng 6: Client-side streaming

## 6.1 .proto definition

```protobuf
service UploadService {
    rpc UploadFile(stream UploadChunk) returns (UploadResponse);
}

message UploadChunk {
    bytes data = 1;
    bool last = 2;
}

message UploadResponse {
    string file_id = 1;
    int64 size = 2;
}
```

`stream UploadChunk` — client sends multiple messages.

## 6.2 Server implementation

```rust
use tokio_stream::StreamExt;

#[tonic::async_trait]
impl UploadService for MyUploadService {
    async fn upload_file(
        &self,
        request: Request<tonic::Streaming<UploadChunk>>,
    ) -> Result<Response<UploadResponse>, Status> {
        let mut stream = request.into_inner();
        
        let mut total_size: i64 = 0;
        let mut buffer = Vec::new();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            total_size += chunk.data.len() as i64;
            buffer.extend_from_slice(&chunk.data);
            
            if chunk.last {
                break;
            }
        }
        
        // Save buffer to disk, get file_id
        let file_id = save_file(buffer).await?;
        
        Ok(Response::new(UploadResponse {
            file_id,
            size: total_size,
        }))
    }
}
```

## 6.3 Client implementation

```rust
use tokio_stream;

async fn upload(client: &mut UploadServiceClient<Channel>) -> Result<(), Box<dyn Error>> {
    let file = tokio::fs::read("big_file.bin").await?;
    
    // Create stream of chunks
    let chunks: Vec<UploadChunk> = file.chunks(4096).enumerate().map(|(i, c)| {
        UploadChunk {
            data: c.to_vec(),
            last: i == file.len() / 4096,   // mark last chunk
        }
    }).collect();
    
    let request = tonic::Request::new(tokio_stream::iter(chunks));
    let response = client.upload_file(request).await?;
    
    println!("Uploaded: {:?}", response.into_inner());
    Ok(())
}
```

`tokio_stream::iter(...)` creates Stream from iterator.

## 6.4 Use cases for client streaming

- **File upload** (chunked)
- **Batch operations** — submit many items
- **Telemetry collection** — client pushes metrics
- **Logging** — agent → log server stream
- **Data ingestion** — continuous data feed

---

# Tầng 7: Bidirectional streaming

## 7.1 .proto definition

```protobuf
service ChatService {
    rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}

message ChatMessage {
    string user = 1;
    string content = 2;
    int64 timestamp = 3;
}
```

Both sides stream. Independent in time.

## 7.2 Server implementation

```rust
#[tonic::async_trait]
impl ChatService for MyChatService {
    type ChatStream = ReceiverStream<Result<ChatMessage, Status>>;
    
    async fn chat(
        &self,
        request: Request<tonic::Streaming<ChatMessage>>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        // Spawn task to process incoming + send responses
        tokio::spawn(async move {
            while let Some(msg) = in_stream.next().await {
                match msg {
                    Ok(msg) => {
                        // Echo back with prefix
                        let response = ChatMessage {
                            user: "server".to_string(),
                            content: format!("Got: {}", msg.content),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        
                        if tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Stream error: {:?}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
```

## 7.3 Client implementation

```rust
async fn chat(client: &mut ChatServiceClient<Channel>) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let request = tonic::Request::new(ReceiverStream::new(rx));
    
    // Spawn task to send messages
    tokio::spawn(async move {
        for i in 0..5 {
            tx.send(ChatMessage {
                user: "alice".to_string(),
                content: format!("Hello {}", i),
                timestamp: chrono::Utc::now().timestamp(),
            }).await.unwrap();
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    
    // Receive responses
    let mut response_stream = client.chat(request).await?.into_inner();
    
    while let Some(msg) = response_stream.next().await {
        println!("Got: {:?}", msg?);
    }
    
    Ok(())
}
```

Both: send + receive concurrent.

## 7.4 Use cases for bidirectional

- **Chat applications** — both sides freely send
- **Multiplayer games** — player state ↔ game state
- **Collaborative editing** — operations ↔ updates
- **Voice / video calls** — audio/video frames
- **Real-time trading** — orders ↔ confirmations

vs WebSocket: gRPC bidirectional has structure (typed messages). Stronger typing.

---

# Tầng 8: Error handling — Status codes

## 8.1 gRPC status codes (subset)

```
   ┌──────────────────┬──────────────────────────────────┐
   │ Status code      │ Meaning                          │
   ├──────────────────┼──────────────────────────────────┤
   │ OK               │ Success (not an error)           │
   │ CANCELLED        │ Client cancelled                  │
   │ UNKNOWN          │ Unknown error                     │
   │ INVALID_ARGUMENT │ Bad request data                  │
   │ DEADLINE_EXCEEDED│ Timeout                          │
   │ NOT_FOUND        │ Resource not found                │
   │ ALREADY_EXISTS   │ Duplicate                         │
   │ PERMISSION_DENIED│ No permission                     │
   │ RESOURCE_EXHAUSTED│ Quota exceeded                  │
   │ FAILED_PRECONDITION│ State invalid                  │
   │ ABORTED          │ Aborted (e.g., concurrent modif) │
   │ OUT_OF_RANGE     │ Out of range                      │
   │ UNIMPLEMENTED    │ Not implemented                   │
   │ INTERNAL         │ Server bug                        │
   │ UNAVAILABLE      │ Service down/overload              │
   │ DATA_LOSS        │ Data corruption                   │
   │ UNAUTHENTICATED  │ No / invalid credentials          │
   └──────────────────┴──────────────────────────────────┘
```

Like HTTP status codes but more granular.

## 8.2 Returning errors

```rust
async fn get_user(
    &self,
    request: Request<GetUserRequest>,
) -> Result<Response<User>, Status> {
    let req = request.into_inner();
    
    if req.id <= 0 {
        return Err(Status::invalid_argument("id must be positive"));
    }
    
    let user = match self.db.find_user(req.id).await {
        Ok(Some(u)) => u,
        Ok(None) => return Err(Status::not_found(format!("user {} not found", req.id))),
        Err(e) => return Err(Status::internal(format!("database error: {}", e))),
    };
    
    if user.is_banned {
        return Err(Status::permission_denied("account banned"));
    }
    
    Ok(Response::new(user))
}
```

`Status` has helper constructors per code.

## 8.3 Mapping app errors to Status

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("user not found")]
    UserNotFound,
    
    #[error("invalid input: {0}")]
    BadRequest(String),
    
    #[error("unauthorized")]
    Unauthorized,
    
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::UserNotFound => Status::not_found("user not found"),
            AppError::BadRequest(msg) => Status::invalid_argument(msg),
            AppError::Unauthorized => Status::unauthenticated("..."),
            AppError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                Status::internal("internal server error")   // hide details
            }
        }
    }
}

// Now use ? in handlers:
async fn get_user(...) -> Result<Response<User>, Status> {
    let user = self.service.get_user(req.id).await?;   // AppError → Status
    Ok(Response::new(user))
}
```

## 8.4 Rich error details

Status can carry additional details via metadata or `details` field:

```rust
use tonic::Code;

let mut status = Status::new(Code::InvalidArgument, "validation failed");
status.metadata_mut().insert("field", "email".parse().unwrap());
status.metadata_mut().insert("reason", "invalid format".parse().unwrap());
return Err(status);
```

Client:
```rust
match client.create_user(req).await {
    Ok(resp) => { ... }
    Err(status) => {
        println!("Code: {:?}", status.code());
        println!("Message: {}", status.message());
        if let Some(field) = status.metadata().get("field") {
            println!("Failed field: {:?}", field);
        }
    }
}
```

For structured error details, use Google's `google.rpc.Status` with `Any` types. Crate `tonic-types` helps.

---

# Tầng 9: Metadata — Headers + trailers

## 9.1 What is metadata?

Like HTTP headers, but for gRPC. Sent in HTTP/2 frames.

Two kinds:
- **Headers** — before message body
- **Trailers** — after (used for trailing status)

## 9.2 Set metadata in client request

```rust
let mut request = tonic::Request::new(GetUserRequest { id: 42 });
request.metadata_mut().insert("authorization", "Bearer token123".parse().unwrap());
request.metadata_mut().insert("request-id", "abc-xyz".parse().unwrap());

let response = client.get_user(request).await?;
```

## 9.3 Read metadata in server

```rust
async fn get_user(
    &self,
    request: Request<GetUserRequest>,
) -> Result<Response<User>, Status> {
    let metadata = request.metadata();
    
    let auth = metadata.get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(Status::unauthenticated("missing authorization"))?;
    
    let request_id = metadata.get("request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    tracing::info!(request_id, "processing");
    
    // ...
}
```

## 9.4 Server sets response metadata

```rust
async fn get_user(...) -> Result<Response<User>, Status> {
    let user = User { ... };
    let mut response = Response::new(user);
    
    response.metadata_mut().insert("server-version", "1.0".parse().unwrap());
    response.metadata_mut().insert("rate-limit-remaining", "99".parse().unwrap());
    
    Ok(response)
}
```

Client reads:
```rust
let response = client.get_user(request).await?;
let version = response.metadata().get("server-version");
let user = response.into_inner();
```

## 9.5 Binary metadata

```rust
// Keys ending with "-bin" carry binary data
request.metadata_mut().insert_bin(
    "trace-bin",
    MetadataValue::from_bytes(b"\x00\x01\x02"),
);
```

For binary tokens, binary IDs.

## 9.6 Standard metadata keys

- `authorization` — auth token
- `user-agent` — client info
- `grpc-timeout` — RPC deadline
- `x-request-id` — request correlation
- `traceparent` — OpenTelemetry trace context

---

# Tầng 10: Interceptors & middleware

## 10.1 Interceptors

gRPC equivalent of axum middleware. Intercept all requests.

## 10.2 Server interceptor

```rust
use tonic::service::Interceptor;

fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    println!("Intercepting: {:?}", req.metadata());
    
    // Check auth
    match req.metadata().get("authorization") {
        Some(t) if t == "Bearer secret123" => Ok(req),
        _ => Err(Status::unauthenticated("invalid token")),
    }
}

// Apply to service:
Server::builder()
    .add_service(UserServiceServer::with_interceptor(svc, intercept))
    .serve(addr)
    .await?;
```

Interceptor runs before handler. Can reject, modify, log.

## 10.3 Tower middleware (more powerful)

Since tonic uses tower:
```rust
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

let layer = ServiceBuilder::new()
    .timeout(Duration::from_secs(30))
    .layer(TraceLayer::new_for_grpc())
    .layer(GrpcWebLayer::new());

Server::builder()
    .layer(layer)
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

Reuse tower-http middleware (trace, compression, timeout, rate limit).

## 10.4 Client interceptor

```rust
async fn add_auth_token(mut req: Request<()>) -> Result<Request<()>, Status> {
    let token = MetadataValue::try_from("Bearer secret123").unwrap();
    req.metadata_mut().insert("authorization", token);
    Ok(req)
}

let channel = Channel::from_static("http://localhost:50051").connect().await?;
let client = UserServiceClient::with_interceptor(channel, add_auth_token);
```

Client interceptor: add auth on every request.

## 10.5 Tracing interceptor

```rust
use tracing::Span;
use opentelemetry::propagation::Injector;

fn trace_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    let span = Span::current();
    let cx = span.context();
    
    let mut injector = MetadataMap::new();
    opentelemetry::global::get_text_map_propagator(|prop| {
        prop.inject_context(&cx, &mut injector);
    });
    
    // Add traceparent to request
    for (k, v) in injector.iter() {
        req.metadata_mut().insert(k.clone(), v.clone());
    }
    
    Ok(req)
}
```

Distributed tracing across services.

---

# Tầng 11: Authentication

## 11.1 Token-based auth

```rust
// Server side:
fn auth_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    let token = req.metadata()
        .get("authorization")
        .ok_or(Status::unauthenticated("missing token"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid token format"))?;
    
    let token = token.strip_prefix("Bearer ")
        .ok_or(Status::unauthenticated("expected Bearer token"))?;
    
    let claims = verify_jwt(token)
        .map_err(|_| Status::unauthenticated("invalid token"))?;
    
    // Inject claims into request extensions
    let mut req = req;
    req.extensions_mut().insert(claims);
    
    Ok(req)
}
```

Handler accesses claims:
```rust
async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
    let claims: &Claims = request.extensions().get()
        .ok_or(Status::internal("no auth claims"))?;
    
    let user_id = claims.user_id;
    // ...
}
```

## 11.2 Client side — token per request

```rust
let token = "Bearer eyJh...";
let token_value: MetadataValue<_> = format!("Bearer {}", token).parse().unwrap();

let interceptor = move |mut req: Request<()>| {
    req.metadata_mut().insert("authorization", token_value.clone());
    Ok(req)
};

let channel = Channel::from_static("http://localhost:50051").connect().await?;
let mut client = UserServiceClient::with_interceptor(channel, interceptor);

client.get_user(request).await?;
```

## 11.3 mTLS (mutual TLS)

For service-to-service auth:
```rust
use tonic::transport::{Identity, ServerTlsConfig};

let cert = std::fs::read("server.crt")?;
let key = std::fs::read("server.key")?;
let server_identity = Identity::from_pem(cert, key);

let ca_cert = std::fs::read("ca.crt")?;
let client_ca = Certificate::from_pem(ca_cert);

let tls = ServerTlsConfig::new()
    .identity(server_identity)
    .client_ca_root(client_ca);

Server::builder()
    .tls_config(tls)?
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

Client must present valid cert. Strong auth without tokens.

## 11.4 Per-method auth requirements

```rust
async fn list_users(...) -> Result<Response<Self::ListUsersStream>, Status> {
    let claims: &Claims = request.extensions().get()
        .ok_or(Status::unauthenticated("login required"))?;
    
    if !claims.is_admin {
        return Err(Status::permission_denied("admin only"));
    }
    
    // ...
}
```

Check role inside handler when needed.

---

# Tầng 12: TLS

## 12.1 Why TLS for gRPC?

- Protect data in transit
- Required by some browsers for gRPC-Web
- Prevents tampering
- Authentication (mTLS)

Production gRPC SHOULD use TLS.

## 12.2 Server TLS

```rust
use tonic::transport::{Identity, Server, ServerTlsConfig};

let cert = std::fs::read("server.pem")?;
let key = std::fs::read("server.key")?;
let identity = Identity::from_pem(cert, key);

let tls = ServerTlsConfig::new().identity(identity);

Server::builder()
    .tls_config(tls)?
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

## 12.3 Client TLS

```rust
use tonic::transport::{Certificate, ClientTlsConfig, Channel};

let ca = std::fs::read("ca.pem")?;
let ca = Certificate::from_pem(ca);

let tls = ClientTlsConfig::new()
    .ca_certificate(ca)
    .domain_name("my-server.example.com");

let channel = Channel::from_static("https://my-server.example.com:50051")
    .tls_config(tls)?
    .connect()
    .await?;

let mut client = UserServiceClient::new(channel);
```

`https://` scheme. Specify domain for SNI/hostname verification.

## 12.4 Self-signed for dev

```bash
openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout server.key -out server.pem \
    -days 365 -subj '/CN=localhost'
```

Client must trust this CA. For dev only — use real CA (Let's Encrypt) for production.

## 12.5 Cert rotation

In production, certs rotate. Pattern:
- Watch cert file changes
- Reload TLS config without restart
- Or use cert manager (e.g., cert-manager in Kubernetes)

Tonic has limited hot-reload. Often just restart with new cert.

---

# Tầng 13: Health checks & reflection

## 13.1 Health check protocol

gRPC has standard health check protocol. Use `tonic-health` crate.

```rust
use tonic_health::server::{health_reporter, HealthReporter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut health_reporter, health_service) = health_reporter();
    
    health_reporter
        .set_serving::<UserServiceServer<MyUserService>>()
        .await;
    
    Server::builder()
        .add_service(health_service)
        .add_service(UserServiceServer::new(svc))
        .serve(addr)
        .await?;
    
    Ok(())
}
```

Health check service exposed. Probe with:
```bash
grpc_health_probe -addr=localhost:50051
```

Kubernetes uses this for liveness/readiness.

## 13.2 Dynamic health status

```rust
// Mark service down temporarily:
health_reporter.set_not_serving::<UserServiceServer<MyUserService>>().await;

// ... maintenance ...

// Mark serving again:
health_reporter.set_serving::<UserServiceServer<MyUserService>>().await;
```

For graceful degradation.

## 13.3 Reflection

gRPC server reflection: clients can discover services without proto file.

```rust
use tonic_reflection::server::Builder;

let reflection = Builder::configure()
    .register_encoded_file_descriptor_set(USER_FILE_DESCRIPTOR_SET)
    .build()?;

Server::builder()
    .add_service(reflection)
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

build.rs:
```rust
tonic_build::configure()
    .file_descriptor_set_path(
        std::path::PathBuf::from(std::env::var("OUT_DIR")?)
            .join("user_descriptor.bin"))
    .compile_protos(...)?;
```

Then:
```rust
pub const USER_FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(
    concat!(env!("OUT_DIR"), "/user_descriptor.bin"));
```

Use `grpcurl` for ad-hoc testing:
```bash
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext localhost:50051 list user.v1.UserService
grpcurl -plaintext -d '{"id": 42}' localhost:50051 user.v1.UserService/GetUser
```

Like `curl` for gRPC. Reflection makes this discoverable.

---

# Tầng 14: Observability — Tracing, metrics

## 14.1 Tracing layer

```rust
use tower_http::trace::TraceLayer;

Server::builder()
    .layer(TraceLayer::new_for_grpc())
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

Logs each request: method, status, duration.

## 14.2 #[instrument] on handlers

```rust
#[tonic::async_trait]
impl UserService for MyUserService {
    #[tracing::instrument(skip(self), fields(user_id = %request.get_ref().id))]
    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<User>, Status> {
        tracing::info!("processing");
        
        // ...
        
        Ok(Response::new(user))
    }
}
```

Span per handler call. Include request fields.

## 14.3 OpenTelemetry distributed tracing

```rust
use opentelemetry::propagation::Injector;
use opentelemetry::propagation::Extractor;

// Server: extract trace context from metadata
struct MetadataExtractor<'a>(&'a tonic::metadata::MetadataMap);

impl<'a> Extractor for MetadataExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

fn trace_extract(req: &Request<()>) -> opentelemetry::Context {
    opentelemetry::global::get_text_map_propagator(|prop| {
        prop.extract(&MetadataExtractor(req.metadata()))
    })
}
```

Server reads `traceparent` header, continues trace from client. End-to-end distributed tracing.

## 14.4 Metrics

```rust
use metrics::{counter, histogram};
use std::time::Instant;

async fn observed_call(method: &str, future: impl Future) -> Result<...> {
    counter!("grpc_requests_total", "method" => method).increment(1);
    
    let start = Instant::now();
    let result = future.await;
    let duration = start.elapsed().as_secs_f64();
    
    let status = match &result {
        Ok(_) => "ok",
        Err(_) => "error",
    };
    
    histogram!("grpc_request_duration_seconds",
        "method" => method,
        "status" => status,
    ).record(duration);
    
    result
}
```

Custom middleware to track per-method metrics.

## 14.5 gRPC-specific metrics (4 Golden Signals)

```
   • Traffic:    grpc_requests_total (counter)
   • Errors:     grpc_errors_total (counter, labeled by code)
   • Latency:    grpc_request_duration_seconds (histogram)
   • Saturation: connection pool, queue size
```

Grafana dashboards over these.

---

# Tầng 15: Testing gRPC services

## 15.1 Unit test handler

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_user() {
        let svc = MyUserService::default();
        let req = tonic::Request::new(GetUserRequest { id: 42 });
        
        let response = svc.get_user(req).await.unwrap();
        let user = response.into_inner();
        
        assert_eq!(user.id, 42);
        assert!(user.email.contains("@"));
    }
    
    #[tokio::test]
    async fn test_invalid_id() {
        let svc = MyUserService::default();
        let req = tonic::Request::new(GetUserRequest { id: -1 });
        
        let result = svc.get_user(req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }
}
```

Direct call to handler. Fast, no network.

## 15.2 Integration test với in-memory channel

```rust
use tonic::transport::Server;
use tokio::sync::oneshot;

#[tokio::test]
async fn integration_test() {
    let (tx, rx) = oneshot::channel::<()>();
    
    let svc = MyUserService::default();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(UserServiceServer::new(svc))
            .serve_with_shutdown("127.0.0.1:0".parse().unwrap(), async {
                rx.await.ok();
            })
            .await
            .unwrap();
    });
    
    // ... make client requests ...
    
    tx.send(()).unwrap();
    server.await.unwrap();
}
```

Spawn server, do tests, shutdown.

## 15.3 Use random port + waiting

```rust
async fn start_test_server() -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);
    
    let (tx, rx) = oneshot::channel();
    let svc = MyUserService::default();
    
    let handle = tokio::spawn(async move {
        let incoming = tonic::transport::server::TcpIncoming::from_listener(
            listener, true, None
        ).unwrap();
        
        Server::builder()
            .add_service(UserServiceServer::new(svc))
            .serve_with_incoming_shutdown(incoming, async {
                rx.await.ok();
            })
            .await
            .unwrap();
    });
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    (url, tx, handle)
}

#[tokio::test]
async fn test_e2e() {
    let (url, shutdown, handle) = start_test_server().await;
    
    let mut client = UserServiceClient::connect(url).await.unwrap();
    let response = client.get_user(GetUserRequest { id: 42 }).await.unwrap();
    
    assert_eq!(response.into_inner().id, 42);
    
    shutdown.send(()).unwrap();
    handle.await.unwrap();
}
```

Full network test. Slower but realistic.

## 15.4 Mock with mockall

```rust
#[automock]
#[async_trait]
trait UserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>, anyhow::Error>;
}

#[tokio::test]
async fn test_service_logic() {
    let mut mock_repo = MockUserRepository::new();
    mock_repo.expect_find_by_id()
        .with(eq(42))
        .returning(|_| Box::pin(async { 
            Ok(Some(User { id: 42, ..Default::default() }))
        }));
    
    let svc = MyUserService::new(Arc::new(mock_repo));
    let response = svc.get_user(Request::new(GetUserRequest { id: 42 }))
        .await.unwrap();
    
    assert_eq!(response.into_inner().id, 42);
}
```

Mock dependencies, test service logic.

---

# Tầng 16: gRPC-Web, load balancing, production

## 16.1 gRPC-Web

Browsers can't speak raw gRPC (requires HTTP/2 trailers). Solution: **gRPC-Web** proxy or in-server:

```toml
[dependencies]
tonic-web = "0.12"
```

```rust
use tonic_web::GrpcWebLayer;

Server::builder()
    .accept_http1(true)
    .layer(GrpcWebLayer::new())
    .add_service(UserServiceServer::new(svc))
    .serve(addr)
    .await?;
```

Now browser can call:
```javascript
import { UserServiceClient } from './generated/user_grpc_web_pb.js';

const client = new UserServiceClient('http://localhost:50051');
client.getUser({ id: 42 }, {}, (err, response) => {
    console.log(response.toObject());
});
```

Or use newer `@bufbuild/connect-web` (Connect protocol, gRPC-compatible).

## 16.2 Load balancing

### Client-side load balancing

```rust
use tonic::transport::Endpoint;

let endpoints = vec![
    "http://server1:50051",
    "http://server2:50051",
    "http://server3:50051",
];

let endpoints = endpoints.into_iter()
    .map(|e| Endpoint::from_static(e));

let channel = Channel::balance_list(endpoints);
let mut client = UserServiceClient::new(channel);
```

tonic balances requests across endpoints (round-robin default).

### Server-side (proxy)

Use envoy, linkerd, or Kubernetes service mesh. More features (retries, circuit breaking, health checks).

## 16.3 Connection pooling

Channel reuses HTTP/2 connections automatically. Multiplexes many streams.

For high traffic, multiple channels:
```rust
let channels: Vec<Channel> = (0..4).map(|_| {
    Channel::from_static("http://server:50051").connect_lazy()
}).collect();

// Round-robin among channels
```

But: HTTP/2 1 connection handles many streams. Usually 1 channel enough.

## 16.4 Retries

tonic doesn't have built-in retries (gRPC standard). Use tower:
```rust
use tower::retry::{Policy, Retry};

#[derive(Clone)]
struct RetryPolicy;

impl<Req: Clone, Res, E> Policy<Req, Res, E> for RetryPolicy {
    type Future = futures::future::Ready<()>;
    
    fn retry(&self, _req: &Req, result: Result<&Res, &E>) -> Option<Self::Future> {
        match result {
            Err(_) => Some(futures::future::ready(())),
            Ok(_) => None,
        }
    }
    
    fn clone_request(&self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

let client = ServiceBuilder::new()
    .layer(Retry::new(RetryPolicy, ...))
    .service(client);
```

Or use `tower::retry::Budget` for limits.

## 16.5 Compression

```rust
// Server:
Server::builder()
    .add_service(
        UserServiceServer::new(svc)
            .accept_compressed(CompressionEncoding::Gzip)
            .send_compressed(CompressionEncoding::Gzip)
    )
    .serve(addr)
    .await?;

// Client:
let client = UserServiceClient::new(channel)
    .send_compressed(CompressionEncoding::Gzip)
    .accept_compressed(CompressionEncoding::Gzip);
```

Reduce bandwidth for large messages. Cost: CPU.

## 16.6 Deadlines (timeouts)

```rust
// Client sets:
let mut request = tonic::Request::new(GetUserRequest { id: 42 });
request.set_timeout(Duration::from_secs(5));

let response = client.get_user(request).await?;
```

Sent as `grpc-timeout` header. Server respects automatically (cancels handler).

## 16.7 Cancellation

```rust
// If client drops or times out, request stream cancels
async fn long_op(&self, req: Request<...>) -> Result<...> {
    for i in 0..1000 {
        // Check cancellation:
        if req.metadata().get("cancel").is_some() {
            return Err(Status::cancelled("..."));
        }
        
        do_work().await;
    }
    Ok(...)
}
```

Tonic propagates cancellation through `Request`'s context.

## 16.8 Production checklist

```
☑ TLS / mTLS
☑ Authentication (JWT / mTLS)
☑ Authorization per method
☑ Health check endpoint (tonic-health)
☑ Reflection in dev (disabled in prod for security)
☑ Tracing layer with OpenTelemetry
☑ Metrics export (Prometheus)
☑ Compression for big messages
☑ Deadlines on requests
☑ Rate limiting (tower)
☑ Connection pooling / load balancing
☑ Graceful shutdown
☑ Schema versioning strategy
```

## 16.9 Compare to alternatives

```
   ┌──────────────────────────────────────────────────────────┐
   │ Internal microservices       → gRPC (typed, fast)        │
   │ Public API                    → REST + OpenAPI           │
   │ Mobile ↔ backend              → gRPC or REST             │
   │ Real-time chat / streaming    → gRPC bidirectional       │
   │ Flexible queries (clients     → GraphQL                  │
   │  need different shapes)                                  │
   │ Browser-only                  → REST or GraphQL          │
   │ Cross-language interop        → gRPC (proto)            │
   └──────────────────────────────────────────────────────────┘
```

---

# Tổng kết — 12 nguyên tắc senior gRPC

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Schema (.proto) versioned. Never reuse field numbers.         │
│                                                                  │
│ 2. Use unary for simple req/resp, streams for real-time.         │
│                                                                  │
│ 3. Map app errors → Status codes consistently.                   │
│                                                                  │
│ 4. Don't leak internals in Status messages (security).           │
│                                                                  │
│ 5. Interceptors for cross-cutting (auth, tracing, logging).      │
│                                                                  │
│ 6. mTLS for service-to-service auth. JWT for user auth.          │
│                                                                  │
│ 7. Always TLS in production.                                     │
│                                                                  │
│ 8. tonic-health for K8s probes.                                   │
│                                                                  │
│ 9. OpenTelemetry tracing through metadata propagation.           │
│                                                                  │
│ 10. Compression for big payloads.                                │
│                                                                  │
│ 11. Deadlines on client requests. Server respects.               │
│                                                                  │
│ 12. Reflection disabled in production (security exposure).       │
└──────────────────────────────────────────────────────────────────┘
```

---

# tonic toolkit

| Crate | Purpose |
|-------|---------|
| `tonic` | gRPC server/client framework |
| `tonic-build` | Codegen from .proto |
| `prost` | Protobuf encoding |
| `prost-build` | Protobuf compiler |
| `prost-types` | Well-known types (Timestamp, etc.) |
| `tonic-health` | Health check service |
| `tonic-reflection` | Server reflection |
| `tonic-web` | gRPC-Web for browsers |
| `tonic-types` | Rich error types |
| `tower` | Middleware (timeout, retry, rate limit) |
| `tower-http` | HTTP-specific layers |
| `grpcurl` | CLI client (like curl) |
| `grpc_health_probe` | Health check CLI |
| `bloomrpc`, `Postman` | GUI clients for testing |

---

# Bộ tài liệu giờ có 22 chương!

```
   📚 RUST FOUNDATIONS LIBRARY
   
   a-s:  19 chương (foundations + apps)
   t. wasm
   u. cli-tools
   v. grpc-tonic       ← MỚI
   
   Tổng: 22 chương × 2 = 44 files
```

🦀 Bộ kỹ năng full-domain Rust:
- 🌐 Web (axum REST)
- 🗄️ Database (sqlx)
- 🔌 Embedded (no_std)
- 🖥️ Desktop (Tauri)
- 📱 Mobile (Tauri v2)
- 🌍 WASM (browser/edge)
- 📟 CLI tools
- 🔌 **gRPC** (microservices internal) ← MỚI

Còn nhiều domain để đào tiếp:
- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced)
- **Cryptography** (rustls, ring)
- **OS kernels** (Redox)

Báo nếu muốn tiếp! 🦀⚡
