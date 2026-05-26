# gRPC với tonic — Minh Hoạ Trực Quan

> Companion visual cho [v-grpc-tonic.md](./v-grpc-tonic.md). Đọc song song.

---

## 1. Bức tranh lớn — gRPC Universe

```
                          gRPC + tonic UNIVERSE
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   gRPC = high-performance RPC framework                │
       │   • HTTP/2 transport                                   │
       │   • Protocol Buffers binary format                     │
       │   • 4 RPC types (unary, server/client/bidi stream)     │
       │   • Cross-language (Rust, Go, Python, ...)             │
       │                                                        │
       │   ┌────────────────────────────────────────────────┐    │
       │   │  .proto schema  (service + messages)            │   │
       │   │            │                                    │   │
       │   │            │ tonic-build codegen                │   │
       │   │            ▼                                    │   │
       │   │  ┌─────────────────────┐                        │   │
       │   │  │ Generated Rust code │                        │   │
       │   │  │ • Server trait      │                        │   │
       │   │  │ • Client struct     │                        │   │
       │   │  │ • Message structs   │                        │   │
       │   │  └─────────┬───────────┘                        │   │
       │   │            │                                    │   │
       │   │            ▼                                    │   │
       │   │  ┌─────────────────────┐                        │   │
       │   │  │ Your implementation │                        │   │
       │   │  └─────────────────────┘                        │   │
       │   └────────────────────────────────────────────────┘   │
       │                                                        │
       │   Use cases:                                           │
       │   • Microservices internal RPC                         │
       │   • Mobile ↔ backend                                   │
       │   • Real-time streaming                                │
       │   • High-perf APIs                                     │
       │                                                        │
       └────────────────────────────────────────────────────────┘
```

---

## 2. gRPC vs REST vs GraphQL

```
   ┌──────────────────────────────────────────────────────────────────┐
   │                                                                  │
   │  Aspect          │ REST        │ GraphQL     │ gRPC              │
   │  ────────────────┼─────────────┼─────────────┼──────────────     │
   │  Protocol         │ HTTP/1.1    │ HTTP        │ HTTP/2           │
   │  Format           │ JSON (text) │ JSON (text) │ Protobuf (binary)│
   │  Size (typical)   │ 100%        │ 80%         │ 20-30%           │
   │  Schema           │ OpenAPI     │ SDL         │ .proto           │
   │  Type-safe        │ Loose       │ Strong (TS) │ Strong (codegen)  │
   │  Streaming        │ SSE (hack)  │ Subscript.  │ Native (4 types) │
   │  Bidi             │ WebSocket   │ Subscript.  │ Native           │
   │  Browser support  │ ✅ Native   │ ✅ Native   │ via gRPC-Web     │
   │  Best for         │ Public API  │ Mobile flex │ Internal RPC     │
   │                                                                  │
   └──────────────────────────────────────────────────────────────────┘
   
   
   When to use each:
   ─────────────────
   
   ┌────────────────────────────┬──────────────────────┐
   │ Need                       │ Use                  │
   ├────────────────────────────┼──────────────────────┤
   │ Public API (browser, curl) │ REST                 │
   │ Mobile flexibility         │ GraphQL              │
   │ Internal microservices     │ gRPC                 │
   │ Real-time streaming        │ gRPC (or WebSocket)  │
   │ Cross-language internal    │ gRPC                 │
   │ Maximum performance        │ gRPC                 │
   └────────────────────────────┴──────────────────────┘
```

---

## 3. 4 types of RPC

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. UNARY (request/response)                            │
   │   ──────────────────                                     │
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     ├──── GetUser(id=42) ──────────►│                  │
   │     │                                 │                  │
   │     │◄──── User(...) ─────────────────│                  │
   │                                                          │
   │   Like REST. Most common.                                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. SERVER STREAMING                                    │
   │   ──────────────────                                     │
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     ├──── Subscribe() ──────────────►│                  │
   │     │                                 │                  │
   │     │◄──── NewsItem 1 ────────────────│                  │
   │     │◄──── NewsItem 2 ────────────────│                  │
   │     │◄──── NewsItem 3 ────────────────│                  │
   │     │       ...                       │                  │
   │     │◄──── (stream end) ──────────────│                  │
   │                                                          │
   │   Use: news feed, logs, telemetry                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. CLIENT STREAMING                                    │
   │   ──────────────────                                     │
   │                                                          │
   │   Client                            Server               │
   │     ├──── UploadChunk 1 ────────────►│                  │
   │     ├──── UploadChunk 2 ────────────►│                  │
   │     ├──── UploadChunk 3 ────────────►│                  │
   │     ├──── (stream end) ─────────────►│                  │
   │     │                                 │                  │
   │     │◄──── UploadResponse ────────────│                  │
   │                                                          │
   │   Use: file upload, batch ingest                         │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   4. BIDIRECTIONAL STREAMING                             │
   │   ────────────────────────                               │
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     ├──── ChatMessage ───────────────►│                  │
   │     │◄──── ChatMessage ────────────────│                 │
   │     ├──── ChatMessage ───────────────►│                  │
   │     │◄──── ChatMessage ────────────────│                 │
   │     │◄──── ChatMessage ────────────────│ (server can send│
   │     ├──── ChatMessage ───────────────►│ anytime)        │
   │     │      ...                        │                  │
   │                                                          │
   │   Use: chat, multiplayer games, voice/video             │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. Project structure

```
   my-grpc-service/
   ├── Cargo.toml
   ├── build.rs                  ← tonic-build runs here
   ├── proto/
   │   ├── user.proto             ← schema
   │   └── common.proto
   └── src/
       ├── lib.rs                 ← include generated code
       ├── server.rs              ← server binary
       ├── client.rs              ← client binary (optional)
       ├── service.rs             ← UserService implementation
       └── error.rs               ← AppError → Status
   
   
   build.rs:
   ─────────
   fn main() -> Result<(), Box<dyn std::error::Error>> {
       tonic_build::configure()
           .build_server(true)
           .build_client(true)
           .compile_protos(&["proto/user.proto"], &["proto"])?;
       Ok(())
   }
   
   
   Each build:
   ──────────
   1. tonic-build reads proto/*.proto
   2. Generates Rust code into target/
   3. Your code includes via `tonic::include_proto!("user.v1")`
```

---

## 5. .proto file structure

```
   ┌──────────────────────────────────────────────────────────┐
   │ syntax = "proto3";                                       │
   │                                                          │
   │ package user.v1;                  ← namespace            │
   │                                                          │
   │ // Service definition                                    │
   │ service UserService {                                    │
   │   rpc GetUser(GetUserRequest) returns (User);            │
   │   rpc CreateUser(CreateUserRequest) returns (User);      │
   │   rpc ListUsers(...) returns (stream User);   ← streaming│
   │   rpc Chat(stream Msg) returns (stream Msg);  ← bidi    │
   │ }                                                        │
   │                                                          │
   │ // Message types                                         │
   │ message User {                                           │
   │   int64 id = 1;                ← field number           │
   │   string email = 2;                                      │
   │   string name = 3;                                       │
   │   Role role = 4;               ← enum                    │
   │   repeated string tags = 5;     ← list                   │
   │ }                                                        │
   │                                                          │
   │ enum Role {                                              │
   │   ROLE_UNSPECIFIED = 0;       ← default                 │
   │   ROLE_USER = 1;                                         │
   │   ROLE_ADMIN = 2;                                        │
   │ }                                                        │
   └──────────────────────────────────────────────────────────┘
   
   
   Field numbers RULES:
   ────────────────────
   • 1-15: 1-byte encoding (use for common fields)
   • 16+: 2-byte
   • NEVER REUSE numbers (breaks compat!)
   • NEVER CHANGE existing number
   • Use `reserved 4, 5;` to mark removed
   
   
   Compatible changes (safe):
   ──────────────────────────
   ✅ Add new field (with new number)
   ✅ Add enum values
   ✅ Add new RPC methods
   ✅ Remove field (BUT reserve number)
   
   Breaking changes (avoid!):
   ──────────────────────────
   ❌ Change field number
   ❌ Change field type
   ❌ Remove and reuse number
```

---

## 6. Binary format efficiency

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Same message: User { id: 42, email: "alice", name: "Alice" }│
   │                                                          │
   │  JSON (text):                                            │
   │  ─────────                                               │
   │  {"id":42,"email":"alice","name":"Alice"}                │
   │  Bytes: 70+                                              │
   │  Parse: parse text, allocate strings, type coerce        │
   │                                                          │
   │  Protobuf (binary):                                      │
   │  ───────────────                                         │
   │  08 2a 12 05 61 6c 69 63 65 1a 05 41 6c 69 63 65        │
   │   ↑  ↑                                                   │
   │   │  └── value 42 (varint)                              │
   │   └── field 1 (id), wire type 0 (varint)                │
   │  Bytes: ~14                                              │
   │  Parse: read tag, decode by type — fast                  │
   │                                                          │
   │  ⟹ ~5x smaller, ~10x faster parse                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Why so small?                                           │
   │                                                          │
   │  • No field names — just numbers                         │
   │  • Variable-length encoding (varint)                     │
   │  • Type info compressed                                  │
   │  • No quotes, commas, brackets                           │
   │  • HTTP/2 header compression                             │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. Codegen flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   proto/user.proto                                       │
   │     service UserService {                                │
   │       rpc GetUser(GetUserRequest) returns (User);        │
   │     }                                                    │
   │     message User { ... }                                 │
   │                                                          │
   │            │                                             │
   │            │  cargo build → build.rs → tonic-build       │
   │            ▼                                             │
   │                                                          │
   │   Generated Rust code:                                   │
   │   ─────────────────                                      │
   │                                                          │
   │   // Message types                                       │
   │   pub struct User {                                      │
   │       pub id: i64,                                       │
   │       pub email: String,                                 │
   │       pub name: String,                                  │
   │       // ...                                             │
   │   }                                                      │
   │                                                          │
   │   pub struct GetUserRequest {                            │
   │       pub id: i64,                                       │
   │   }                                                      │
   │                                                          │
   │   // Server trait (implement this)                       │
   │   #[async_trait]                                         │
   │   pub trait UserService: Send + Sync + 'static {         │
   │       async fn get_user(                                 │
   │           &self,                                         │
   │           request: Request<GetUserRequest>,              │
   │       ) -> Result<Response<User>, Status>;               │
   │   }                                                      │
   │                                                          │
   │   // Server wrapper                                      │
   │   pub struct UserServiceServer<T> { ... }                │
   │                                                          │
   │   // Client                                              │
   │   pub struct UserServiceClient<T> { ... }                │
   │                                                          │
   │   impl<T> UserServiceClient<T> {                         │
   │       pub async fn get_user(                             │
   │           &mut self,                                     │
   │           request: GetUserRequest,                       │
   │       ) -> Result<Response<User>, Status>;               │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. Server implementation flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. Define struct:                                      │
   │                                                          │
   │   pub struct MyUserService {                             │
   │       db: PgPool,                                        │
   │   }                                                      │
   │                                                          │
   │   2. Implement trait:                                    │
   │                                                          │
   │   #[tonic::async_trait]                                  │
   │   impl UserService for MyUserService {                    │
   │       async fn get_user(                                 │
   │           &self,                                         │
   │           request: Request<GetUserRequest>,              │
   │       ) -> Result<Response<User>, Status> {              │
   │           let req = request.into_inner();                │
   │                                                          │
   │           // Business logic                              │
   │           let user = self.db.find_user(req.id).await     │
   │               .map_err(|e| Status::internal(e.to_string()))?│
   │               .ok_or(Status::not_found("not found"))?;   │
   │                                                          │
   │           Ok(Response::new(user))                        │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   │   3. Start server:                                       │
   │                                                          │
   │   #[tokio::main]                                         │
   │   async fn main() -> Result<()> {                        │
   │       let addr = "0.0.0.0:50051".parse()?;               │
   │       let svc = MyUserService::new(...);                 │
   │                                                          │
   │       Server::builder()                                  │
   │           .add_service(UserServiceServer::new(svc))      │
   │           .serve(addr)                                   │
   │           .await?;                                       │
   │                                                          │
   │       Ok(())                                             │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. Client usage

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Client:                                                │
   │                                                          │
   │   #[tokio::main]                                         │
   │   async fn main() -> Result<()> {                        │
   │       let mut client = UserServiceClient::connect(       │
   │           "http://localhost:50051"                       │
   │       ).await?;                                          │
   │                                                          │
   │       let request = tonic::Request::new(                 │
   │           GetUserRequest { id: 42 }                      │
   │       );                                                 │
   │                                                          │
   │       let response = client.get_user(request).await?;    │
   │                                                          │
   │       let user: User = response.into_inner();            │
   │       println!("User: {:?}", user);                      │
   │                                                          │
   │       Ok(())                                             │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Add metadata (auth, request-id):                       │
   │                                                          │
   │   let mut request = tonic::Request::new(...);            │
   │   request.metadata_mut()                                 │
   │       .insert("authorization", "Bearer xyz".parse()?);   │
   │   request.metadata_mut()                                 │
   │       .insert("request-id", "abc123".parse()?);          │
   │                                                          │
   │   client.get_user(request).await?;                       │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Set deadline (timeout):                                │
   │                                                          │
   │   let mut request = tonic::Request::new(...);            │
   │   request.set_timeout(Duration::from_secs(5));           │
   │                                                          │
   │   // Server gets `grpc-timeout` header,                  │
   │   // cancels handler if exceeded                         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 10. Server streaming pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   .proto:                                                │
   │   service NewsService {                                  │
   │       rpc Subscribe(Req) returns (stream NewsItem);      │
   │   }                                                      │
   │                                                          │
   │   Server:                                                │
   │                                                          │
   │   type SubscribeStream = ReceiverStream<                  │
   │       Result<NewsItem, Status>                           │
   │   >;                                                     │
   │                                                          │
   │   async fn subscribe(...) -> Result<Response<Self::SubscribeStream>, Status> {│
   │       let (tx, rx) = tokio::sync::mpsc::channel(4);      │
   │                                                          │
   │       tokio::spawn(async move {                          │
   │           for i in 0..10 {                                │
   │               let item = NewsItem { ... };                │
   │               if tx.send(Ok(item)).await.is_err() {      │
   │                   break;  ← client disconnected           │
   │               }                                          │
   │               sleep(1s).await;                            │
   │           }                                              │
   │       });                                                │
   │                                                          │
   │       Ok(Response::new(ReceiverStream::new(rx)))         │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Client:                                                │
   │                                                          │
   │   let mut stream = client.subscribe(...).await?          │
   │       .into_inner();                                     │
   │                                                          │
   │   while let Some(item) = stream.message().await? {       │
   │       println!("Got: {:?}", item);                       │
   │   }                                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Sequence diagram:                                      │
   │                                                          │
   │   Client            Server                               │
   │     │                  │                                 │
   │     ├── subscribe() ─►│                                 │
   │     │                  │ spawn task                      │
   │     │                  │                                 │
   │     │◄── item 1 ────────│ (sleep 1s)                     │
   │     │◄── item 2 ────────│ (sleep 1s)                     │
   │     │◄── item 3 ────────│ (sleep 1s)                     │
   │     │     ...            │                                │
   │     │◄── (end stream) ──│                                │
   │     │                                                    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 11. Status codes mapping

```
   ┌──────────────────────────────────────────────────────────┐
   │  App error                  → gRPC Status                │
   │  ─────────                  → ─────────                  │
   │                                                          │
   │  Invalid input              → INVALID_ARGUMENT (3)       │
   │  Not authenticated          → UNAUTHENTICATED (16)       │
   │  No permission              → PERMISSION_DENIED (7)      │
   │  Not found                  → NOT_FOUND (5)              │
   │  Duplicate                  → ALREADY_EXISTS (6)         │
   │  Rate limited                → RESOURCE_EXHAUSTED (8)    │
   │  Validation failure          → FAILED_PRECONDITION (9)   │
   │  Conflict                    → ABORTED (10)              │
   │  Timeout                     → DEADLINE_EXCEEDED (4)     │
   │  Server bug                  → INTERNAL (13)             │
   │  Service overloaded          → UNAVAILABLE (14)          │
   │  Not implemented             → UNIMPLEMENTED (12)        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   impl From<AppError> for Status {                       │
   │       fn from(err: AppError) -> Self {                   │
   │           match err {                                    │
   │               AppError::NotFound => Status::not_found(...),│
   │               AppError::BadRequest(m) =>                 │
   │                   Status::invalid_argument(m),           │
   │               AppError::Unauthorized =>                  │
   │                   Status::unauthenticated(...),          │
   │               AppError::Internal(e) => {                 │
   │                   tracing::error!(error = ?e);            │
   │                   Status::internal("server error")       │
   │                   // hide details from client            │
   │               }                                          │
   │           }                                              │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   │   // Now use ? in handlers:                              │
   │   async fn get_user(...) -> Result<..., Status> {        │
   │       let user = self.svc.get_user(id).await?;           │
   │       //  AppError → Status auto                          │
   │       Ok(Response::new(user))                            │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 12. Interceptor architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Request                                                │
   │     │                                                    │
   │     ▼                                                    │
   │   ┌──────────────────────────────┐                       │
   │   │ Server interceptor           │                       │
   │   │  • Auth check                │                       │
   │   │  • Logging                   │                       │
   │   │  • Tracing context extract   │                       │
   │   │  • Rate limit                │                       │
   │   └────────┬─────────────────────┘                       │
   │            │                                             │
   │            ▼ (or reject with Status)                     │
   │   ┌──────────────────────────────┐                       │
   │   │ Tower middleware layers      │                       │
   │   │  • TimeoutLayer               │                       │
   │   │  • TraceLayer                 │                       │
   │   │  • Compression                │                       │
   │   └────────┬─────────────────────┘                       │
   │            │                                             │
   │            ▼                                             │
   │   ┌──────────────────────────────┐                       │
   │   │ Service trait implementation │                       │
   │   │  (your handler)              │                       │
   │   └────────┬─────────────────────┘                       │
   │            │                                             │
   │            ▼                                             │
   │   Response with Status                                   │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Example interceptor (auth):                            │
   │                                                          │
   │   fn auth_interceptor(req: Request<()>)                  │
   │     -> Result<Request<()>, Status>                       │
   │   {                                                      │
   │       match req.metadata().get("authorization") {        │
   │           Some(t) if valid(t) => Ok(req),                 │
   │           _ => Err(Status::unauthenticated("missing")),  │
   │       }                                                  │
   │   }                                                      │
   │                                                          │
   │   Server::builder()                                      │
   │       .add_service(                                      │
   │           UserServiceServer::with_interceptor(           │
   │               svc, auth_interceptor                      │
   │           )                                              │
   │       )                                                  │
   │       .serve(addr).await?;                               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 13. Metadata flow

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   HTTP/2 frame structure:                                │
   │                                                          │
   │   ┌────────────────────────────────────┐                 │
   │   │ HEADERS frame                      │                 │
   │   │   :method: POST                    │                 │
   │   │   :path: /user.v1.UserService/Get  │                 │
   │   │   :authority: example.com           │                 │
   │   │   content-type: application/grpc   │                 │
   │   │   authorization: Bearer xyz   ←── │ metadata         │
   │   │   x-request-id: abc           ←── │                  │
   │   └────────────────────────────────────┘                 │
   │   ┌────────────────────────────────────┐                 │
   │   │ DATA frames                        │                 │
   │   │   Protobuf-encoded request bytes   │                 │
   │   └────────────────────────────────────┘                 │
   │   ┌────────────────────────────────────┐                 │
   │   │ HEADERS frame (trailers)           │                 │
   │   │   grpc-status: 0 (OK)              │                 │
   │   │   grpc-message: ""                 │                 │
   │   └────────────────────────────────────┘                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Client sets:                                           │
   │   let mut req = Request::new(GetUserRequest { id: 42 });  │
   │   req.metadata_mut().insert(                             │
   │       "authorization",                                   │
   │       "Bearer xyz".parse().unwrap()                      │
   │   );                                                     │
   │                                                          │
   │   Server reads:                                          │
   │   async fn get_user(&self, req: Request<...>) -> ... {   │
   │       let auth = req.metadata()                          │
   │           .get("authorization")                          │
   │           .and_then(|v| v.to_str().ok());                │
   │       // ...                                             │
   │   }                                                      │
   │                                                          │
   │   Standard metadata keys:                                │
   │   • authorization     — auth token                        │
   │   • grpc-timeout      — deadline                          │
   │   • x-request-id      — correlation                       │
   │   • traceparent       — OTel trace context                │
   │   • user-agent        — client info                       │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 14. Authentication patterns

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. TOKEN AUTH (JWT, OAuth):                            │
   │                                                          │
   │   Client                            Server               │
   │     │                                 │                  │
   │     │ Headers:                        │                  │
   │     │   authorization: Bearer eyJh... │                  │
   │     ├────────────────────────────────►│                  │
   │     │                                 │ verify JWT       │
   │     │                                 │ extract claims   │
   │     │                                 │ inject into ctx  │
   │     │                                 │ run handler      │
   │     │◄──── response ──────────────────│                  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. mTLS (mutual TLS):                                  │
   │                                                          │
   │   Client (presents client cert)                          │
   │     │                                                    │
   │     ├── TLS handshake with client cert ──► Server        │
   │     │                                       │            │
   │     │                                       │ verify cert│
   │     │                                       │ check CN/SAN│
   │     │                                       │ accept     │
   │     │◄────── connection established ──────│              │
   │     │                                                    │
   │     │ now gRPC over secure channel                       │
   │                                                          │
   │   ✅ Strong auth without tokens                          │
   │   ✅ Service-to-service ideal                            │
   │   ❌ Cert management complexity                          │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. API KEY (simple):                                   │
   │                                                          │
   │   Headers:                                               │
   │     x-api-key: secret-key-xxx                            │
   │                                                          │
   │   Simple but limited (no expiry, no claims)              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Per-method authorization:                              │
   │                                                          │
   │   async fn delete_user(...) -> Result<..., Status> {     │
   │       let claims = request.extensions().get::<Claims>()  │
   │           .ok_or(Status::unauthenticated("..."))?;       │
   │                                                          │
   │       if !claims.is_admin {                              │
   │           return Err(Status::permission_denied(          │
   │               "admin only"                               │
   │           ));                                            │
   │       }                                                  │
   │                                                          │
   │       // ... delete logic ...                            │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 15. Distributed tracing

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Service A (Rust gRPC client)                           │
   │     │                                                    │
   │     │ Span: client.get_user                              │
   │     │ TraceID: abc-123                                   │
   │     │                                                    │
   │     │ Inject into metadata:                              │
   │     │   traceparent: 00-abc123-...-01                    │
   │     │                                                    │
   │     ├─ gRPC call ──────────────────►│ Service B          │
   │                                       │                  │
   │                                       │ Extract from      │
   │                                       │ metadata           │
   │                                       │ TraceID: abc-123 │
   │                                       │ ParentSpan: ...   │
   │                                       │                  │
   │                                       │ Span: server.get_user│
   │                                       │                  │
   │                                       ├─ DB query ───►│ DB │
   │                                       │ Span: db.find  │   │
   │                                       │                  │
   │                                       ├─ another gRPC ──►│ │
   │                                       │ propagate trace  │ │
   │                                                          │
   │   All spans link to same TraceID.                        │
   │   View in Jaeger/Tempo as connected tree.                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Trace propagation visualization:                       │
   │                                                          │
   │   Trace abc-123 (total: 250ms)                           │
   │   ┌─────────────────────────────────────────┐            │
   │   │ Service A: handler  ─── 250ms total     │            │
   │   │   ├── client.get_user (Service B) 200ms │            │
   │   │   │     ├── server.get_user 180ms       │            │
   │   │   │     │     ├── db.query 50ms          │           │
   │   │   │     │     └── client.fetch_orders 100ms│         │
   │   │   │     │           └── server.fetch_orders 80ms│    │
   │   │   ├── compute 30ms                       │            │
   │   │   └── respond 20ms                       │            │
   │   └─────────────────────────────────────────┘            │
   │                                                          │
   │   ⟹ See bottleneck instantly                              │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 16. Testing strategy

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. UNIT TESTS — direct handler call                    │
   │   ────────────────                                       │
   │                                                          │
   │   #[tokio::test]                                         │
   │   async fn test_get_user() {                             │
   │       let svc = MyUserService::with_mock_db();           │
   │       let req = Request::new(GetUserRequest { id: 42 }); │
   │                                                          │
   │       let response = svc.get_user(req).await.unwrap();    │
   │       let user = response.into_inner();                  │
   │                                                          │
   │       assert_eq!(user.id, 42);                           │
   │   }                                                      │
   │                                                          │
   │   ⟹ Fast, no network                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   2. INTEGRATION TESTS — spawn real server                │
   │   ──────────────────────                                 │
   │                                                          │
   │   #[tokio::test]                                         │
   │   async fn test_e2e() {                                  │
   │       let (url, shutdown, handle) = start_test_server().await;│
   │                                                          │
   │       let mut client = UserServiceClient::connect(url)   │
   │           .await.unwrap();                               │
   │                                                          │
   │       let resp = client.get_user(GetUserRequest { id: 42 })│
   │           .await.unwrap();                               │
   │                                                          │
   │       assert_eq!(resp.into_inner().id, 42);              │
   │                                                          │
   │       shutdown.send(()).unwrap();                        │
   │       handle.await.unwrap();                             │
   │   }                                                      │
   │                                                          │
   │   ⟹ Real gRPC over network                                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   3. CLI TESTING — grpcurl                                │
   │   ─────────────                                          │
   │                                                          │
   │   # Reflection enabled? Discover:                        │
   │   $ grpcurl -plaintext localhost:50051 list              │
   │   user.v1.UserService                                    │
   │   grpc.health.v1.Health                                   │
   │                                                          │
   │   $ grpcurl -plaintext localhost:50051 \                 │
   │     list user.v1.UserService                             │
   │   user.v1.UserService.GetUser                            │
   │   user.v1.UserService.ListUsers                          │
   │                                                          │
   │   # Test a method:                                       │
   │   $ grpcurl -plaintext -d '{"id": 42}' \                 │
   │     localhost:50051 user.v1.UserService/GetUser          │
   │   {                                                      │
   │     "id": "42",                                          │
   │     "email": "user42@example.com",                       │
   │     ...                                                  │
   │   }                                                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Production deployment

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Production gRPC stack:                                 │
   │                                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  CLIENT (mobile, web, service)              │        │
   │   │   ├─ TLS                                     │       │
   │   │   ├─ JWT in metadata                         │       │
   │   │   └─ Deadline / timeout set                  │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │ HTTP/2 over TLS                     │
   │                    ▼                                     │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  LOAD BALANCER (envoy / nginx / k8s service)│       │
   │   │   ├─ TLS termination (or pass-through)       │       │
   │   │   ├─ Load distribute                         │       │
   │   │   ├─ Health probe                             │      │
   │   │   └─ Retry, circuit breaker                  │       │
   │   └────────────────┬─────────────────────────────┘       │
   │                    │                                     │
   │     ┌──────────────┼──────────────┐                      │
   │     ▼              ▼              ▼                      │
   │   ┌──────┐ ┌──────┐ ┌──────┐                              │
   │   │ Pod 1│ │ Pod 2│ │ Pod 3│ (tonic server replicas)     │
   │   │      │ │      │ │      │                              │
   │   │ • TLS│ │      │ │      │                              │
   │   │ • Auth│ │      │ │      │                              │
   │   │ • Trace│ │     │ │      │                              │
   │   │ • Health│ │     │ │      │                             │
   │   │ • Metrics│ │    │ │      │                             │
   │   └──┬───┘ └──┬───┘ └──┬───┘                              │
   │      │        │        │                                  │
   │      └────────┼────────┘                                  │
   │               ▼                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  Backend services / DB / cache               │       │
   │   └──────────────────────────────────────────────┘       │
   │                                                          │
   │   Observability:                                         │
   │     • OpenTelemetry → Jaeger/Tempo                       │
   │     • Prometheus metrics scrape                          │
   │     • Logs → Loki/Elasticsearch                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 18. gRPC-Web for browsers

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Problem: browsers can't speak raw gRPC                 │
   │   (HTTP/2 trailers required, not exposed by fetch)       │
   │                                                          │
   │   Solution: gRPC-Web                                     │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Browser              Server (tonic with tonic-web)     │
   │     │                       │                            │
   │     │ HTTP/1.1 or HTTP/2     │                            │
   │     │ Content-Type:           │                            │
   │     │   application/grpc-web  │                            │
   │     ├────────────────────────►│                            │
   │     │                          │ tonic-web layer:         │
   │     │                          │   strip gRPC-Web wrapper │
   │     │                          │   convert to native gRPC │
   │     │                          │                          │
   │     │                          │ handle as normal gRPC    │
   │     │                          │                          │
   │     │◄──── response (gRPC-Web format) ───│                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   Server setup:                                          │
   │                                                          │
   │   Server::builder()                                      │
   │       .accept_http1(true)                                │
   │       .layer(GrpcWebLayer::new())                        │
   │       .add_service(UserServiceServer::new(svc))          │
   │       .serve(addr).await?;                               │
   │                                                          │
   │   Browser code (JS):                                     │
   │                                                          │
   │   import { UserServiceClient } from './generated';        │
   │                                                          │
   │   const client = new UserServiceClient(                  │
   │       'https://api.example.com'                          │
   │   );                                                     │
   │   const response = await client.getUser({ id: 42 });     │
   │                                                          │
   │   Or use Connect protocol (modern alternative):          │
   │   @bufbuild/connect-web                                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 19. Common antipatterns

```
   ❌ 1. Reusing field numbers
   ───────────────────────
   
   // v1
   message User {
       int64 id = 1;
       string email = 2;
   }
   
   // v2 — REMOVED email, REUSED 2 for phone
   message User {
       int64 id = 1;
       string phone = 2;   ← Old clients sending email get garbage!
   }
   
   ✅ Reserve removed numbers:
   message User {
       reserved 2;
       reserved "email";
       int64 id = 1;
       string phone = 3;
   }
   
   
   ❌ 2. Leak internal errors
   ──────────────────────
   
   Err(Status::internal(format!("SQL error: {}", e)))
   // Exposes DB internals, query details
   
   ✅ Log full, return generic:
   tracing::error!(error = ?e, "db error");
   Err(Status::internal("internal error"))
   
   
   ❌ 3. No deadlines
   ──────────────
   
   client.long_operation(req).await?;  // hangs forever?
   
   ✅ Set timeout:
   let mut req = tonic::Request::new(...);
   req.set_timeout(Duration::from_secs(30));
   client.long_operation(req).await?;
   
   
   ❌ 4. Plaintext in production
   ──────────────────────────
   
   Server::builder().serve(addr).await?;  // no TLS
   
   ✅ Always TLS in prod:
   Server::builder()
       .tls_config(tls_config)?
       .serve(addr).await?;
   
   
   ❌ 5. Reflection in production
   ──────────────────────────
   
   Server::builder()
       .add_service(reflection)   ← exposes schema publicly
       .add_service(my_service)
       .serve(addr).await?;
   
   ✅ Disable in prod (security):
   #[cfg(debug_assertions)]
   builder = builder.add_service(reflection);
   
   
   ❌ 6. Sync work in async handler
   ─────────────────────────────
   
   async fn handle(...) -> Result<..., Status> {
       std::thread::sleep(Duration::from_secs(10));  // ❌ blocks executor
   }
   
   ✅ Async sleep:
   tokio::time::sleep(Duration::from_secs(10)).await;
   
   Or spawn_blocking for CPU work:
   tokio::task::spawn_blocking(|| { ... }).await?;
   
   
   ❌ 7. Auth in every handler
   ────────────────────────
   
   async fn handle(req: Request<...>) -> Result<..., Status> {
       if !is_authorized(&req) {     ← duplicate in every method
           return Err(...);
       }
       // ...
   }
   
   ✅ Use interceptor:
   Server::builder()
       .add_service(MyServer::with_interceptor(svc, auth_interceptor))
       .serve(addr).await?;
```

---

## 20. Mind map cuối

```
                              gRPC + TONIC
                                  │
        ┌────────────┬────────────┼────────────┬─────────────┐
        ▼            ▼            ▼            ▼             ▼
     SCHEMA      RPC TYPES    SERVER       CLIENT       PRODUCTION
        │            │            │            │             │
    .proto      Unary         tonic-build  Channel       TLS / mTLS
    protoc      Server stream Server trait Client struct Auth + JWT
    field#s     Client stream Interceptors Metadata      Tracing OTel
    schema evo  Bidi stream   Tower layers Deadlines     Metrics
                              Status codes  Streams       Health probe
                                                          Load balance
                                                          gRPC-Web
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. Schema (.proto) versioned        │
                │     NEVER reuse field numbers        │
                │                                      │
                │  2. 4 RPC types: pick right one      │
                │                                      │
                │  3. Map errors → Status codes        │
                │     Don't leak internals             │
                │                                      │
                │  4. Interceptors for cross-cutting   │
                │                                      │
                │  5. mTLS or JWT auth                 │
                │                                      │
                │  6. Always TLS in production         │
                │                                      │
                │  7. tonic-health for K8s probes      │
                │                                      │
                │  8. OpenTelemetry tracing via       │
                │     metadata propagation             │
                │                                      │
                │  9. Compression for big payloads     │
                │                                      │
                │  10. Deadlines on client requests    │
                │                                      │
                │  11. Reflection in dev only          │
                │                                      │
                │  12. gRPC-Web for browser support    │
                └──────────────────────────────────────┘
```

---

## 21. Bộ tài liệu giờ có 22 chương!

```
   ┌──────────────────────────────────────────────────────────┐
   │             RUST FOUNDATIONS LIBRARY                     │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   a-s:  19 chương foundation + apps                      │
   │   t. wasm                                                │
   │   u. cli-tools                                           │
   │   v. grpc-tonic            ← MỚI                         │
   │      v-grpc-tonic.md + visual                            │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🦀 Bộ kỹ năng full-domain:                            │
   │                                                          │
   │   🌐 Web REST (axum)                                     │
   │   🗄️ Database (sqlx)                                     │
   │   🔌 Embedded (no_std)                                   │
   │   🖥️ Desktop (Tauri)                                     │
   │   📱 Mobile (Tauri v2)                                   │
   │   🌍 WASM (browser/edge)                                 │
   │   📟 CLI tools                                           │
   │   🔌 gRPC microservices  ← MỚI                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

🦀 Báo nếu muốn đào tiếp:
- **Game engines** (Bevy ECS)
- **GUI native** (egui, iced)
- **Cryptography** (rustls, ring)
- **Networking** (mio, smoltcp)
- **OS kernels** (Redox)
