# Axum Project Realistic — Apply Tất Cả 16 Chủ Đề

> Tài liệu thứ 17 trong bộ Rust nền tảng. Đây là tài liệu **APPLY** — kết hợp toàn bộ kiến thức:
> - [async.md](./async.md) — axum chạy trên tokio
> - [trait.md](./trait.md) — Tower Service trait
> - [error-handling.md](./error-handling.md) — IntoResponse cho HTTP errors
> - [observability.md](./observability.md) — tracing trong handlers
> - [testing.md](./testing.md) — integration test cho HTTP API
> - [smart-pointers.md](./smart-pointers.md) — Arc<Mutex> trong AppState
> - [lifetime.md](./lifetime.md) — public API tránh lifetime
> - [performance.md](./performance.md) — benchmarks
>
> **axum** là web framework Rust phổ biến nhất hiện nay. Built bởi Tokio team, kết hợp:
> - **tower** (service abstraction)
> - **hyper** (HTTP implementation)
> - **tokio** (async runtime)
>
> Tài liệu này hướng dẫn xây 1 **production-ready REST API** — user authentication, database,
> testing, observability, deployment. Mỗi component reference back các tài liệu trước.

---

# Mục lục

- [Tầng 1: axum vs actix vs rocket — Chọn cái nào?](#tầng-1-axum-vs-actix-vs-rocket--chọn-cái-nào)
- [Tầng 2: Tower Service — Foundation](#tầng-2-tower-service--foundation)
- [Tầng 3: Project structure — Workspace layout](#tầng-3-project-structure--workspace-layout)
- [Tầng 4: Routing — URL → handler](#tầng-4-routing--url--handler)
- [Tầng 5: Extractors — Magic của axum](#tầng-5-extractors--magic-của-axum)
- [Tầng 6: State management](#tầng-6-state-management)
- [Tầng 7: Middleware — Layers](#tầng-7-middleware--layers)
- [Tầng 8: Error handling — IntoResponse](#tầng-8-error-handling--intoresponse)
- [Tầng 9: Validation — User input safety](#tầng-9-validation--user-input-safety)
- [Tầng 10: Authentication & Authorization](#tầng-10-authentication--authorization)
- [Tầng 11: Database integration — sqlx](#tầng-11-database-integration--sqlx)
- [Tầng 12: Observability — tracing + metrics](#tầng-12-observability--tracing--metrics)
- [Tầng 13: Configuration management](#tầng-13-configuration-management)
- [Tầng 14: Testing — Unit, Integration, E2E](#tầng-14-testing--unit-integration-e2e)
- [Tầng 15: Deployment — Docker, CI/CD](#tầng-15-deployment--docker-cicd)
- [Tầng 16: Performance & production tuning](#tầng-16-performance--production-tuning)

---

# Tầng 1: axum vs actix vs rocket — Chọn cái nào?

## 1.1 Major Rust web frameworks

| Framework | Async runtime | Style | Notes |
|-----------|---------------|-------|-------|
| **axum** | tokio | Function handlers + extractors | Modern, by Tokio team |
| **actix-web** | actix (custom) | Actor model + handlers | Mature, fastest in some benchmarks |
| **rocket** | tokio | Macro-heavy | Easy DX, slower compile |
| **warp** | tokio | Filter combinators | Functional, learning curve |
| **poem** | tokio | Like axum but with more batteries | Smaller community |
| **salvo** | tokio | Trait-based | Less mature |

## 1.2 Why axum?

- **Tokio ecosystem**: most async crates compatible
- **Tower middleware**: rich ecosystem of layers
- **Type-safe extractors**: ergonomic + compile-time safe
- **Minimal magic**: just functions + types, no DSL
- **Active development**: by Tokio team
- **Stable 1.0+**: API stable since 2023

Recommended for new projects in 2024+.

## 1.3 axum mental model

```rust
async fn handler() -> impl IntoResponse {
    "Hello"
}

let app = Router::new()
    .route("/", get(handler));

axum::serve(listener, app).await.unwrap();
```

- Handler = async function
- Router = mapping URL pattern → handler
- Extractors = function arguments that pull from request
- Responses = anything implementing `IntoResponse`

That's it. Conceptually simple.

## 1.4 Compare to popular frameworks (other languages)

| Lang/Framework | Comparable axum concept |
|----------------|-------------------------|
| Express.js | `app.get("/path", handler)` similar |
| FastAPI (Python) | Extractors + type hints similar to extractors |
| Spring Boot (Java) | Annotations vs Rust type-based |
| Rails (Ruby) | Far less convention magic, more explicit |
| Go (Gin/Fiber) | Handler-based, similar |

axum sits between minimal (Go) and feature-rich (Spring) — Type-driven, not annotation-driven.

---

# Tầng 2: Tower Service — Foundation

## 2.1 What is Tower?

`tower` = abstract networking middleware library. **Service trait**:

```rust
pub trait Service<Request> {
    type Response;
    type Error;
    type Future: Future<Output = Result<Self::Response, Self::Error>>;
    
    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>>;
    fn call(&mut self, req: Request) -> Self::Future;
}
```

A Service is: takes request, returns future of response.

Powerful: HTTP server, gRPC client, RPC frameworks all use Service.

## 2.2 axum is built on Tower

axum's Router implements `Service<Request<Body>>`:
```
Router → Service<Request> → returns Response
```

Every handler, middleware, extractor — all composed as Tower services.

→ Use any Tower middleware in axum (rate limiting, retries, timeouts, ...).

## 2.3 Tower layers

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(TraceLayer::new_for_http())     // tower-http
    .layer(TimeoutLayer::new(Duration::from_secs(30)));
```

Layer wraps a Service to add behavior. Compose like onion:

```
Request →  Trace → Timeout → Router → Handler
                                          │
Response ← Trace ← Timeout ← Router ← Handler
```

## 2.4 Why Tower matters

- **Reusable middleware**: works across axum, tonic (gRPC), other frameworks
- **Composable**: stack layers cleanly
- **Type-safe**: compile-time guarantees about request/response
- **Tested ecosystem**: tower-http, tower-load-shed, etc.

In practice: you'll see `.layer(...)` everywhere in axum apps.

---

# Tầng 3: Project structure — Workspace layout

## 3.1 Recommended layout

```
my-api/
├── Cargo.toml                    # workspace
├── README.md
├── .env.example
├── Dockerfile
├── compose.yaml                  # for local dev
│
├── crates/
│   ├── api/                      # axum binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # entry point
│   │       ├── lib.rs            # for integration tests
│   │       ├── routes/           # route handlers
│   │       │   ├── mod.rs
│   │       │   ├── users.rs
│   │       │   ├── auth.rs
│   │       │   └── health.rs
│   │       ├── middleware/       # custom middleware
│   │       ├── error.rs          # error types
│   │       ├── state.rs          # AppState
│   │       └── config.rs         # configuration
│   │
│   ├── domain/                   # business logic (no web concerns)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── user.rs           # User entity, business rules
│   │       └── auth.rs
│   │
│   └── db/                       # database access
│       ├── Cargo.toml
│       ├── migrations/           # sqlx migrations
│       └── src/
│           ├── lib.rs
│           ├── user.rs           # User repository
│           └── connection.rs
│
└── tests/                        # E2E tests
    └── integration.rs
```

## 3.2 Why workspace?

- **Separate concerns**: web (api) vs business (domain) vs infra (db)
- **Faster compile**: change in `api` doesn't rebuild `db`
- **Reusable**: extract crates for other binaries (CLI, worker)
- **Test boundaries**: each crate has own tests

## 3.3 Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
# Shared dependencies — pin versions once
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "chrono"] }
thiserror = "1"
anyhow = "1"
```

Each crate:
```toml
# crates/api/Cargo.toml
[package]
name = "api"

[dependencies]
tokio.workspace = true
axum.workspace = true
domain = { path = "../domain" }
db = { path = "../db" }
```

## 3.4 Layered architecture

```
┌─────────────────────────────┐
│ HTTP Routes (api crate)     │  ← parse request, format response
├─────────────────────────────┤
│ Application Service          │  ← orchestrate business logic
├─────────────────────────────┤
│ Domain Logic (domain crate) │  ← pure business rules
├─────────────────────────────┤
│ Repository (db crate)        │  ← data access
└─────────────────────────────┘
```

Each layer:
- Only depends on layers below
- Has typed error type
- Independently testable

---

# Tầng 4: Routing — URL → handler

## 4.1 Basic routes

```rust
use axum::{Router, routing::{get, post, put, delete}};

let app = Router::new()
    .route("/health", get(health_check))
    .route("/users", get(list_users).post(create_user))
    .route("/users/:id", get(get_user).delete(delete_user))
    .route("/users/:id/orders", get(list_user_orders));
```

`get`, `post`, `put`, `delete`, `patch` — HTTP method constructors.

`.post(create_user)` chains another method on same path.

## 4.2 Path parameters

```rust
use axum::extract::Path;

async fn get_user(Path(id): Path<u64>) -> impl IntoResponse {
    format!("User {}", id)
}

// Multiple params:
async fn get_user_order(Path((user_id, order_id)): Path<(u64, u64)>) {
    // ...
}

// Named struct:
#[derive(serde::Deserialize)]
struct UserOrder { user_id: u64, order_id: u64 }

async fn get_user_order(Path(p): Path<UserOrder>) {
    // p.user_id, p.order_id
}
```

axum parse path params automatically into type. Type mismatch → 400 Bad Request.

## 4.3 Query parameters

```rust
use axum::extract::Query;

#[derive(serde::Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_users(Query(pagination): Query<Pagination>) -> impl IntoResponse {
    // /users?page=2&per_page=20
}
```

Type-safe query parsing. Optional fields with `Option<T>`.

## 4.4 Nested routers

```rust
let user_routes = Router::new()
    .route("/", get(list_users).post(create_user))
    .route("/:id", get(get_user))
    .route("/:id/orders", get(list_orders));

let admin_routes = Router::new()
    .route("/dashboard", get(admin_dashboard));

let app = Router::new()
    .nest("/users", user_routes)
    .nest("/admin", admin_routes);
```

`.nest()` mounts router at prefix. Modular organization.

## 4.5 Method routing

```rust
use axum::routing::method_routing::*;

let app = Router::new()
    .route("/health", any(health_check))         // any method
    .route("/users", get(list).post(create).put(update))
    .route("/legacy", on(MethodFilter::TRACE, handler));
```

## 4.6 Fallback

```rust
let app = Router::new()
    .route("/users", get(list_users))
    .fallback(handle_404);

async fn handle_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not found")
}
```

Catch-all for unmatched routes.

---

# Tầng 5: Extractors — Magic của axum

## 5.1 What is an extractor?

Function argument that **pulls data from request**.

```rust
async fn handler(
    Path(id): Path<u64>,             // URL param
    Query(q): Query<MyQuery>,        // query string
    Json(body): Json<CreateUser>,    // request body JSON
    State(state): State<AppState>,   // app state
    headers: HeaderMap,              // headers
) -> impl IntoResponse {
    // ...
}
```

axum extracts each automatically. **Type-safe** + **declarative**.

## 5.2 Common extractors

```rust
use axum::{
    extract::{Path, Query, Json, State, Form, Multipart, ConnectInfo, Extension},
    http::HeaderMap,
};

Path<T>                    // URL path params (Path<(A, B)>, Path<MyStruct>)
Query<T>                   // URL query string
Json<T>                    // application/json body
Form<T>                    // application/x-www-form-urlencoded body
Multipart                  // multipart/form-data body (file upload)
State<T>                   // shared app state
Extension<T>               // request-scoped data
HeaderMap                  // all headers
TypedHeader<T>             // specific typed header (axum-extra)
ConnectInfo<SocketAddr>    // client IP
Bytes                      // raw body bytes
String                     // body as string
```

## 5.3 Extractor order matters

```rust
async fn handler(
    State(state): State<AppState>,    // ✅ State first
    Path(id): Path<u64>,
    Json(body): Json<CreateUser>,     // ❌ Body extractors LAST
) { ... }
```

Body extractors (`Json`, `Form`, `Bytes`) **consume** request body — must be **last**.

Only **1** body extractor per handler.

## 5.4 Custom extractor

```rust
use axum::{extract::FromRequestParts, async_trait};

struct AuthUser {
    user_id: u64,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where S: Send + Sync
{
    type Rejection = (StatusCode, &'static str);
    
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing auth"))?;
        
        let token = auth_header.strip_prefix("Bearer ")
            .ok_or((StatusCode::UNAUTHORIZED, "invalid format"))?;
        
        let user_id = verify_jwt(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
        
        Ok(AuthUser { user_id })
    }
}

// Use:
async fn protected_handler(user: AuthUser) -> impl IntoResponse {
    format!("Hello user {}", user.user_id)
}
```

`AuthUser` extractor handles auth automatically. Handler clean.

## 5.5 Extractor rejection — automatic error response

```rust
async fn create_user(Json(body): Json<CreateUser>) {
    // If body not valid JSON or doesn't match CreateUser type
    // → axum returns 400 Bad Request automatically
}
```

Each extractor has `Rejection` type. axum converts to HTTP error.

Override with custom extractor for finer control.

## 5.6 Multiple extractors composition

```rust
async fn handler(
    AuthUser { user_id }: AuthUser,           // custom auth
    Path(resource_id): Path<u64>,             // URL
    Query(filter): Query<FilterParams>,       // query
    State(state): State<AppState>,            // state
    Json(body): Json<UpdateRequest>,          // body LAST
) -> Result<Json<Response>, AppError> {
    // All extracted before handler runs
    // ...
}
```

axum composes cleanly. Type-driven dependency injection.

---

# Tầng 6: State management

## 6.1 AppState pattern

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub config: Arc<Config>,
    pub metrics: Arc<MetricsRegistry>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        db: connect_db().await,
        redis: connect_redis(),
        config: Arc::new(Config::load()),
        metrics: Arc::new(MetricsRegistry::new()),
    };
    
    let app = Router::new()
        .route("/users/:id", get(get_user))
        .with_state(state);
    
    axum::serve(listener, app).await.unwrap();
}

async fn get_user(
    Path(id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id as i64)
        .fetch_one(&state.db).await;
    // ...
}
```

`State<AppState>` extractor injects state. Must be `Clone` (axum clones for each request — typically containing `Arc` internally, so clone is cheap).

## 6.2 Arc internally

```rust
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub db: PgPool,            // PgPool is Arc internally too
    pub config: Config,
}
```

`Clone` for AppState = `Arc::clone` for inner — cheap.

Or wrap individually:
```rust
pub struct AppState {
    pub db: PgPool,                     // already Arc internally
    pub config: Arc<Config>,            // wrap Config in Arc
}
```

## 6.3 Multiple states (advanced)

```rust
let app = Router::new()
    .route("/users", get(handler))
    .with_state(state_a)
    .nest("/admin", admin_router.with_state(state_b));
```

Different routes can have different state. Sometimes useful.

## 6.4 Extension — Request-scoped data

```rust
use axum::Extension;

let app = Router::new()
    .layer(Extension(some_value))
    .route("/", get(handler));

async fn handler(Extension(val): Extension<MyType>) {
    // ...
}
```

`Extension` differs from `State`:
- State: same value for ALL requests (cheap clone)
- Extension: per-request injected via middleware

Use Extension when middleware needs to pass data to handler (e.g., authenticated user info).

---

# Tầng 7: Middleware — Layers

## 7.1 Built-in middleware (tower-http)

```toml
[dependencies]
tower-http = { version = "0.6", features = [
    "trace", "compression-full", "cors", "timeout", "limit"
] }
```

```rust
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    limit::RequestBodyLimitLayer,
};
use std::time::Duration;

let app = Router::new()
    .route("/", get(handler))
    .layer(TraceLayer::new_for_http())                 // tracing spans
    .layer(CompressionLayer::new())                     // gzip/brotli
    .layer(CorsLayer::permissive())                     // CORS
    .layer(TimeoutLayer::new(Duration::from_secs(30)))  // request timeout
    .layer(RequestBodyLimitLayer::new(1024 * 1024));    // 1MB body limit
```

Apply once, all requests benefit.

## 7.2 Order matters — Onion model

```rust
let app = Router::new()
    .route("/", get(handler))
    .layer(layer_a)     // outermost
    .layer(layer_b)     // middle
    .layer(layer_c);    // innermost
```

Execution order:
```
Request:  layer_a → layer_b → layer_c → handler
Response: layer_a ← layer_b ← layer_c ← handler
```

Trace layer outermost (capture everything). Timeout near outer. Compression after handler runs.

## 7.3 Custom middleware

```rust
use axum::{
    middleware::{self, Next},
    extract::Request,
    response::Response,
};

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth = req.headers().get("Authorization")
        .ok_or(AppError::Unauthorized)?
        .to_str()
        .map_err(|_| AppError::Unauthorized)?;
    
    let user = verify_token(auth, &state).await?;
    
    // Inject user into request extensions (handler can extract)
    req.extensions_mut().insert(user);
    
    Ok(next.run(req).await)
}

let app = Router::new()
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
```

## 7.4 Selective middleware

```rust
let public_routes = Router::new()
    .route("/health", get(health))
    .route("/login", post(login));

let protected_routes = Router::new()
    .route("/profile", get(profile))
    .route("/orders", get(orders))
    .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

let app = Router::new()
    .merge(public_routes)
    .merge(protected_routes)
    .layer(TraceLayer::new_for_http());     // global
```

Middleware on subset of routes.

## 7.5 Important middleware

```rust
// Request ID (for tracing correlation):
use tower_http::request_id::*;

let app = Router::new()
    .route("/", get(handler))
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(PropagateRequestIdLayer::x_request_id());

// Rate limiting:
use tower::limit::RateLimitLayer;

.layer(RateLimitLayer::new(100, Duration::from_secs(60)))  // 100/min

// Compression:
.layer(CompressionLayer::new())   // gzip, brotli based on accept-encoding

// Timeout per request:
.layer(TimeoutLayer::new(Duration::from_secs(30)))
```

## 7.6 Tracing layer setup

```rust
.layer(
    TraceLayer::new_for_http()
        .make_span_with(|req: &Request| {
            tracing::info_span!(
                "http_request",
                method = %req.method(),
                uri = %req.uri(),
                request_id = tracing::field::Empty,
            )
        })
        .on_response(|response: &Response, latency: Duration, _span: &Span| {
            tracing::info!(
                status = %response.status(),
                latency_ms = latency.as_millis(),
            );
        })
)
```

Every request gets a span with method, URI, duration. Correlate logs to requests.

---

# Tầng 8: Error handling — IntoResponse

## 8.1 The error type

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("user not found")]
    UserNotFound,
    
    #[error("unauthorized")]
    Unauthorized,
    
    #[error("invalid input: {0}")]
    BadRequest(String),
    
    #[error("rate limited")]
    RateLimited,
    
    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
    
    #[error("database error")]
    Db(#[from] sqlx::Error),
}
```

`thiserror` for typed errors (from [error-handling.md](./error-handling.md) Tầng 6).

## 8.2 IntoResponse impl

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "USER_NOT_FOUND",
                self.to_string(),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                self.to_string(),
            ),
            AppError::BadRequest(_) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                self.to_string(),
            ),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                self.to_string(),
            ),
            AppError::Internal(e) => {
                // Log internal errors with FULL detail
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL",
                    "Internal server error".to_string(),   // hide details from client
                )
            }
            AppError::Db(e) => {
                tracing::error!(error = ?e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Database unavailable".to_string(),
                )
            }
        };
        
        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));
        
        (status, body).into_response()
    }
}
```

Key points:
- Map error variant → HTTP status code
- Generate consistent error response format
- **Log internal errors** with full details
- **Hide internals** from client (security)
- Stable `code` field for client to programmatic handle

## 8.3 Handler uses Result

```rust
async fn get_user(
    Path(id): Path<u64>,
    State(state): State<AppState>,
) -> Result<Json<User>, AppError> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id as i64)
        .fetch_optional(&state.db)
        .await?                              // sqlx::Error → AppError::Db (via #[from])
        .ok_or(AppError::UserNotFound)?;     // None → 404
    
    Ok(Json(user))
}
```

Clean. `?` propagates errors. `IntoResponse` formats them.

## 8.4 Validation errors

```rust
#[derive(Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,
    
    #[validate(length(min = 8))]
    password: String,
    
    #[validate(range(min = 18))]
    age: u32,
}

async fn create_user(
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    req.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    // ...
}
```

`validator` crate provides derive-based validation. Reject early in handler.

## 8.5 Custom rejection (extractor error)

```rust
use axum::extract::rejection::JsonRejection;

async fn create_user(
    body: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Result<Json<User>, AppError> {
    let Json(req) = body.map_err(|e| {
        AppError::BadRequest(format!("invalid JSON: {}", e))
    })?;
    // ...
}
```

Catch extractor error explicitly, customize message.

---

# Tầng 9: Validation — User input safety

## 9.1 validator crate

```toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }
```

```rust
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "invalid email"))]
    pub email: String,
    
    #[validate(length(min = 3, max = 30, message = "username 3-30 chars"))]
    pub username: String,
    
    #[validate(length(min = 8))]
    pub password: String,
    
    #[validate(range(min = 18, max = 120))]
    pub age: u32,
    
    #[validate(custom(function = "validate_country_code"))]
    pub country: String,
}

fn validate_country_code(s: &str) -> Result<(), validator::ValidationError> {
    if s.len() != 2 || !s.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(validator::ValidationError::new("invalid_country"));
    }
    Ok(())
}
```

## 9.2 Validate in handler

```rust
async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    // Validate input
    req.validate().map_err(|errors| {
        AppError::BadRequest(format!("validation failed: {}", errors))
    })?;
    
    // Continue with valid data
    let user = state.user_service.create(req).await?;
    Ok(Json(user))
}
```

## 9.3 Custom extractor with validation

```rust
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;
    
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        
        value.validate()
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        
        Ok(ValidatedJson(value))
    }
}

// Use:
async fn create_user(
    ValidatedJson(req): ValidatedJson<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    // req is guaranteed valid
}
```

DRY — validation logic in extractor, not every handler.

## 9.4 Field-level validation messages

```rust
// validator output:
{
    "email": [{"code": "email", "message": "invalid email"}],
    "password": [{"code": "length", "message": "must be ≥ 8"}]
}
```

Return to client as structured JSON. Frontend can display per-field.

## 9.5 Schema design

```rust
// ❌ Mix internal + external concerns
struct User {
    id: u64,
    email: String,
    password_hash: String,    // sensitive!
    created_at: DateTime,
}

// ✅ Separate types
struct UserDb {                // internal database model
    id: i64,
    email: String,
    password_hash: String,
    created_at: DateTime,
}

#[derive(Serialize)]
struct UserResponse {           // public API
    id: u64,
    email: String,
    created_at: DateTime,
    // NO password_hash
}

impl From<UserDb> for UserResponse {
    fn from(u: UserDb) -> Self {
        UserResponse {
            id: u.id as u64,
            email: u.email,
            created_at: u.created_at,
        }
    }
}
```

Convert via `From`. Never leak internals.

---

# Tầng 10: Authentication & Authorization

## 10.1 Token-based auth (JWT)

```toml
[dependencies]
jsonwebtoken = "9"
argon2 = "0.5"
```

```rust
use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Validation, Header};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,      // user_id
    exp: usize,        // expiry
    iat: usize,        // issued at
    role: String,
}

pub fn create_token(user_id: u64, role: &str, secret: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        iat: Utc::now().timestamp() as usize,
        role: role.to_string(),
    };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))?;
    Ok(token)
}

pub fn verify_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
```

## 10.2 AuthUser extractor

```rust
pub struct AuthUser {
    pub user_id: u64,
    pub role: Role,
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    User,
    Admin,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;
    
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        
        let token = parts.headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        
        let claims = verify_token(token, &app_state.config.jwt_secret)
            .map_err(|_| AppError::Unauthorized)?;
        
        let user_id: u64 = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
        let role = match claims.role.as_str() {
            "admin" => Role::Admin,
            _ => Role::User,
        };
        
        Ok(AuthUser { user_id, role })
    }
}
```

## 10.3 Login handler

```rust
#[derive(Deserialize, Validate)]
struct LoginRequest {
    #[validate(email)]
    email: String,
    
    #[validate(length(min = 1))]
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    user: UserResponse,
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    req.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    let user = state.user_repo.find_by_email(&req.email).await?
        .ok_or(AppError::Unauthorized)?;
    
    // Verify password (argon2)
    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid hash")))?;
    
    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;
    
    let token = create_token(user.id as u64, &user.role, &state.config.jwt_secret)
        .map_err(|e| AppError::Internal(e.into()))?;
    
    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}
```

## 10.4 Role-based authorization

```rust
pub struct AdminUser(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;
    
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        match user.role {
            Role::Admin => Ok(AdminUser(user)),
            _ => Err(AppError::Forbidden),
        }
    }
}

async fn admin_only_handler(_: AdminUser) -> impl IntoResponse {
    "Admin access"
}
```

Compile-time guarantee handlers only callable with right role.

## 10.5 Password hashing (Argon2)

```rust
use argon2::{Argon2, PasswordHasher, PasswordVerifier, PasswordHash};
use argon2::password_hash::{SaltString, rand_core::OsRng};

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash error: {}", e))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| anyhow::anyhow!("parse hash: {}", e))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}
```

Never store plain passwords. Use Argon2 (modern, memory-hard).

## 10.6 Other auth patterns

- **Session cookie**: encrypted session ID, lookup user (need session store)
- **OAuth2**: integrate with Google, GitHub, etc. (oauth2 crate)
- **API keys**: long-lived tokens for service-to-service
- **mTLS**: client cert authentication

JWT common for stateless APIs. Sessions better for web apps with CSRF concerns.

---

# Tầng 11: Database integration — sqlx

## 11.1 sqlx — Compile-time SQL

```toml
[dependencies]
sqlx = { version = "0.8", features = [
    "postgres", "runtime-tokio", "uuid", "chrono", "json", "migrate"
] }
```

```rust
use sqlx::{PgPool, Row};

pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPool::connect(url).await
}
```

## 11.2 Compile-time checked queries

```rust
let user: User = sqlx::query_as!(
    User,
    "SELECT id, email, name, created_at FROM users WHERE id = $1",
    id
)
.fetch_one(&pool)
.await?;
```

`query_as!` macro:
- Connects to DB at **compile time**
- Verifies SQL syntax + types
- Generates code returning typed struct

→ SQL typo = compile error, not runtime error.

Requires `DATABASE_URL` env var at compile time OR `cargo sqlx prepare` (offline mode).

## 11.3 Offline mode

```bash
cargo install sqlx-cli
cargo sqlx prepare   # generates .sqlx folder
# Commit .sqlx to repo
# CI build uses .sqlx — no DB needed
```

## 11.4 Repository pattern

```rust
// db/src/user.rs
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserDb {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    
    pub async fn find_by_id(&self, id: i64) -> sqlx::Result<Option<UserDb>> {
        sqlx::query_as!(
            UserDb,
            "SELECT id, email, password_hash, role, created_at FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
    }
    
    pub async fn find_by_email(&self, email: &str) -> sqlx::Result<Option<UserDb>> {
        sqlx::query_as!(
            UserDb,
            "SELECT id, email, password_hash, role, created_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await
    }
    
    pub async fn create(&self, email: &str, password_hash: &str) -> sqlx::Result<UserDb> {
        sqlx::query_as!(
            UserDb,
            r#"
            INSERT INTO users (email, password_hash, role, created_at)
            VALUES ($1, $2, 'user', NOW())
            RETURNING id, email, password_hash, role, created_at
            "#,
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await
    }
}
```

## 11.5 Transactions

```rust
pub async fn create_user_with_profile(
    pool: &PgPool,
    email: &str,
    name: &str,
) -> sqlx::Result<UserDb> {
    let mut tx = pool.begin().await?;
    
    let user = sqlx::query_as!(
        UserDb,
        "INSERT INTO users (email) VALUES ($1) RETURNING *",
        email
    )
    .fetch_one(&mut *tx)
    .await?;
    
    sqlx::query!(
        "INSERT INTO profiles (user_id, name) VALUES ($1, $2)",
        user.id, name
    )
    .execute(&mut *tx)
    .await?;
    
    tx.commit().await?;
    Ok(user)
}
```

Auto-rollback if any step fails (tx not committed = rollback).

## 11.6 Migrations

```bash
sqlx migrate add create_users
# Creates migrations/<timestamp>_create_users.sql
```

```sql
-- migrations/20240526120000_create_users.sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

Run migrations in app startup:
```rust
pub async fn run_migrations(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
```

Or CLI:
```bash
sqlx migrate run
```

## 11.7 Connection pool tuning

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

- `max_connections`: tuned to DB max (often Postgres = 100 default)
- `acquire_timeout`: fail fast if pool exhausted
- `idle_timeout`: close idle connections

## 11.8 Avoiding N+1 query

```rust
// ❌ N+1
async fn get_users_with_orders(pool: &PgPool) -> Result<Vec<UserWithOrders>> {
    let users = sqlx::query_as!(User, "SELECT * FROM users").fetch_all(pool).await?;
    let mut result = vec![];
    for user in users {
        let orders = sqlx::query_as!(Order, "SELECT * FROM orders WHERE user_id = $1", user.id)
            .fetch_all(pool).await?;   // N queries!
        result.push(UserWithOrders { user, orders });
    }
    Ok(result)
}

// ✅ Single query
async fn get_users_with_orders(pool: &PgPool) -> Result<Vec<UserWithOrders>> {
    let rows = sqlx::query!(r#"
        SELECT u.id, u.email, o.id as order_id, o.total
        FROM users u
        LEFT JOIN orders o ON o.user_id = u.id
    "#).fetch_all(pool).await?;
    
    // Group manually
    let mut grouped: HashMap<i64, UserWithOrders> = HashMap::new();
    for row in rows {
        let entry = grouped.entry(row.id).or_insert_with(|| UserWithOrders {
            user: User { id: row.id, email: row.email },
            orders: vec![],
        });
        if let Some(order_id) = row.order_id {
            entry.orders.push(Order { id: order_id, total: row.total.unwrap_or(0) });
        }
    }
    Ok(grouped.into_values().collect())
}
```

Or use `JSON_AGG` to aggregate in SQL.

---

# Tầng 12: Observability — tracing + metrics

## 12.1 Tracing setup

```rust
use tracing_subscriber::prelude::*;

fn init_telemetry() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true);
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
```

Apply [observability.md](./observability.md) Tầng 7 patterns. JSON for production.

## 12.2 #[instrument] on handlers

```rust
#[tracing::instrument(skip(state), fields(user_id = tracing::field::Empty))]
async fn create_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<Order>, AppError> {
    tracing::Span::current().record("user_id", auth.user_id);
    
    tracing::info!("creating order");
    
    let order = state.order_service.create(auth.user_id, req).await?;
    
    tracing::info!(order_id = order.id, "order created");
    Ok(Json(order))
}
```

`skip(state)` avoid logging huge state. `fields(user_id)` add lazy.

## 12.3 TraceLayer for HTTP

```rust
.layer(
    TraceLayer::new_for_http()
        .make_span_with(|req: &Request| {
            let request_id = req.headers()
                .get("x-request-id")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown");
            tracing::info_span!(
                "request",
                method = %req.method(),
                uri = %req.uri(),
                request_id = %request_id,
            )
        })
        .on_response(|resp: &Response, latency: Duration, _: &Span| {
            tracing::info!(
                status = resp.status().as_u16(),
                latency_ms = latency.as_millis(),
                "response"
            );
        })
)
```

Every request: span with request_id, method, URI, status, latency. Correlate everything.

## 12.4 Metrics with prometheus

```rust
use metrics::{counter, histogram, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;

fn init_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9000))
        .install()
        .expect("metrics installation failed");
}

// Custom metric middleware:
async fn metrics_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let start = Instant::now();
    
    counter!("http_requests_total",
        "method" => method.to_string(),
        "path" => path.clone(),
    ).increment(1);
    
    let response = next.run(req).await;
    
    let status = response.status().as_u16();
    let latency = start.elapsed().as_secs_f64();
    
    histogram!("http_request_duration_seconds",
        "method" => method.to_string(),
        "path" => path,
        "status" => status.to_string(),
    ).record(latency);
    
    response
}

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(metrics_middleware));
```

`/metrics` endpoint exposed on port 9000. Prometheus scrapes.

## 12.5 4 Golden Signals dashboard

Mỗi service expose:
- `http_requests_total` (traffic)
- `http_errors_total` (errors)
- `http_request_duration_seconds` (latency P50/P95/P99)
- `connection_pool_used` (saturation)

Grafana dashboards over these. Alert on threshold.

## 12.6 OpenTelemetry integration

```rust
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;

fn init_otel() -> anyhow::Result<()> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://otel-collector:4317")
        .build()?;
    
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "my-api"),
        ]))
        .build();
    
    let tracer = provider.tracer("my-api");
    let otel_layer = OpenTelemetryLayer::new(tracer);
    
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_layer)
        .init();
    
    Ok(())
}
```

Send traces to OTel collector → Jaeger/Tempo. Full distributed tracing.

---

# Tầng 13: Configuration management

## 13.1 Config struct

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub redis: Option<RedisConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiry_hours: u32,
}
```

## 13.2 Load from env + file

```toml
[dependencies]
config = "0.14"
serde = { version = "1", features = ["derive"] }
```

```rust
use config::{Config as ConfigBuilder, File, Environment};

pub fn load() -> anyhow::Result<Config> {
    let environment = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".into());
    
    let settings = ConfigBuilder::builder()
        .add_source(File::with_name("config/default"))
        .add_source(File::with_name(&format!("config/{}", environment)).required(false))
        .add_source(File::with_name("config/local").required(false))
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?;
    
    let config: Config = settings.try_deserialize()?;
    Ok(config)
}
```

Load order:
1. `config/default.toml` (committed defaults)
2. `config/<env>.toml` (per env: dev, staging, prod)
3. `config/local.toml` (gitignored, local dev)
4. Env vars (override everything)

Env var `APP_DATABASE__URL=...` overrides `database.url` (separator `__`).

## 13.3 config/default.toml

```toml
[server]
host = "127.0.0.1"
port = 3000

[database]
max_connections = 20

[jwt]
expiry_hours = 24
```

## 13.4 Secrets management

```toml
# config/local.toml — gitignored
[database]
url = "postgres://localhost/mydb"

[jwt]
secret = "dev-secret-do-not-use-in-prod"
```

Production: secrets from env vars or vault:
```bash
APP_DATABASE__URL="postgres://..."
APP_JWT__SECRET="$(cat /run/secrets/jwt_secret)"
```

Use `secrecy` crate for sensitive fields:
```rust
use secrecy::SecretString;

#[derive(Deserialize)]
struct JwtConfig {
    pub secret: SecretString,    // Debug impl redacts!
}
```

## 13.5 Validate config at startup

```rust
impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.jwt.secret.len() < 32 {
            anyhow::bail!("JWT secret too short (need 32+ chars)");
        }
        if self.server.port == 0 {
            anyhow::bail!("Server port can't be 0");
        }
        Ok(())
    }
}

fn main() {
    let config = Config::load().expect("config load failed");
    config.validate().expect("config validation failed");
    // ...
}
```

Crash early — better than runtime mystery.

## 13.6 Pass config to handlers

Via `AppState` (Tầng 6):
```rust
let state = AppState {
    config: Arc::new(config),
    db: pool,
    // ...
};
```

Handler:
```rust
async fn handler(State(state): State<AppState>) {
    let port = state.config.server.port;
    // ...
}
```

---

# Tầng 14: Testing — Unit, Integration, E2E

Apply [testing.md](./testing.md). Specific axum patterns:

## 14.1 Unit tests for business logic

```rust
// domain/src/user.rs
pub fn validate_password_strength(pw: &str) -> Result<(), &'static str> {
    if pw.len() < 8 { return Err("too short"); }
    if !pw.chars().any(|c| c.is_ascii_digit()) { return Err("need digit"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn rejects_short() {
        assert_eq!(validate_password_strength("abc"), Err("too short"));
    }
    
    #[test]
    fn rejects_no_digit() {
        assert_eq!(validate_password_strength("abcdefgh"), Err("need digit"));
    }
    
    #[test]
    fn accepts_valid() {
        assert!(validate_password_strength("password1").is_ok());
    }
}
```

Pure functions, fast tests, no DB.

## 14.2 Integration tests for routes

```rust
// tests/api.rs
use api::{build_app, AppState};
use axum::http::StatusCode;
use tower::ServiceExt;   // for `oneshot`

#[tokio::test]
async fn health_check() {
    let app = build_app(test_state()).await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

`build_app(state)` factory function — test app without binding port. `oneshot` sends single request.

## 14.3 Test DB with transactions

```rust
async fn test_state() -> AppState {
    let pool = PgPool::connect("postgres://localhost/test_db").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    AppState {
        db: pool,
        config: Arc::new(test_config()),
    }
}

#[tokio::test]
async fn create_user_endpoint() {
    let state = test_state().await;
    let app = build_app(state.clone()).await;
    
    let resp = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/users")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"test@test.com","password":"pwd1234"}"#))
            .unwrap()
    ).await.unwrap();
    
    assert_eq!(resp.status(), 201);
    
    // Verify in DB
    let user = sqlx::query!("SELECT email FROM users WHERE email = $1", "test@test.com")
        .fetch_one(&state.db).await.unwrap();
    assert_eq!(user.email, "test@test.com");
}
```

Use `sqlx::test` macro for auto-rollback:
```rust
#[sqlx::test]
async fn test(pool: PgPool) {
    // pool is fresh test DB, auto-cleanup after test
    // ...
}
```

## 14.4 E2E with real HTTP

```rust
#[tokio::test]
async fn e2e_full_flow() {
    let app = build_app(test_state().await).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}", port);
    
    // Create user
    let resp = client.post(format!("{}/users", url))
        .json(&serde_json::json!({
            "email": "test@test.com",
            "password": "pwd1234"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    
    // Login
    let resp: serde_json::Value = client.post(format!("{}/login", url))
        .json(&serde_json::json!({"email": "test@test.com", "password": "pwd1234"}))
        .send().await.unwrap()
        .json().await.unwrap();
    
    let token = resp["token"].as_str().unwrap();
    
    // Access protected
    let resp = client.get(format!("{}/profile", url))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
}
```

Test full request cycle including auth.

## 14.5 Mock external dependencies

```rust
#[automock]
trait EmailService {
    async fn send(&self, to: &str, body: &str) -> Result<(), Error>;
}

#[tokio::test]
async fn signup_sends_welcome_email() {
    let mut email_mock = MockEmailService::new();
    email_mock.expect_send()
        .with(eq("alice@test.com"), predicate::str::contains("welcome"))
        .returning(|_, _| Box::pin(async { Ok(()) }));
    
    let state = AppState {
        email: Arc::new(email_mock),
        // ...
    };
    // ... test signup flow ...
}
```

Mock external services. Real DB if convenient (sqlx::test).

---

# Tầng 15: Deployment — Docker, CI/CD

## 15.1 Multi-stage Dockerfile

```dockerfile
# Builder stage
FROM rust:1.83-slim AS builder
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/api/Cargo.toml crates/api/
COPY crates/domain/Cargo.toml crates/domain/
COPY crates/db/Cargo.toml crates/db/

# Pre-build deps (cached layer)
RUN mkdir -p crates/api/src crates/domain/src crates/db/src && \
    echo "fn main() {}" > crates/api/src/main.rs && \
    touch crates/domain/src/lib.rs crates/db/src/lib.rs && \
    cargo build --release && \
    rm -rf crates/*/src

# Build actual
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/crates/db/migrations /migrations
COPY --from=builder /app/config /config

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/api"]
```

Multi-stage → small final image (~50-100MB vs ~1GB).

## 15.2 docker-compose for local dev

```yaml
# compose.yaml
version: '3.9'

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: app
      POSTGRES_DB: app
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
  
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
  
  api:
    build: .
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgres://app:app@postgres/app
      REDIS_URL: redis://redis:6379
      APP_JWT__SECRET: dev-secret-32-chars-minimum-length
    depends_on:
      - postgres
      - redis

volumes:
  pgdata:
```

`docker compose up` — full stack locally.

## 15.3 GitHub Actions CI

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        ports: ["5432:5432"]
        options: --health-cmd pg_isready --health-interval 10s
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Install sqlx-cli
        run: cargo install sqlx-cli --no-default-features --features postgres
      
      - name: Run migrations
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost/postgres
        run: sqlx migrate run --source crates/db/migrations
      
      - name: Format
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy -- -D warnings
      
      - name: Test
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost/postgres
        run: cargo test
  
  docker:
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.sha }}
```

## 15.4 Graceful shutdown

```rust
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install Ctrl+C handler");
    };
    
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    tracing::info!("shutdown signal received");
}

#[tokio::main]
async fn main() {
    let app = build_app().await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    
    tracing::info!("shutdown complete");
}
```

Receive SIGTERM/SIGINT → finish in-flight requests → exit clean. Kubernetes prefers.

## 15.5 Kubernetes deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-api
  template:
    metadata:
      labels:
        app: my-api
    spec:
      containers:
      - name: api
        image: ghcr.io/me/my-api:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: url
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
        resources:
          requests:
            cpu: 100m
            memory: 64Mi
          limits:
            cpu: 500m
            memory: 256Mi
```

Health endpoints:
```rust
async fn health() -> &'static str { "ok" }

async fn ready(State(state): State<AppState>) -> Result<&'static str, AppError> {
    // Check DB
    sqlx::query!("SELECT 1 as one").fetch_one(&state.db).await?;
    Ok("ready")
}
```

## 15.6 Resource limits

Rust embedded binary typically 5-50MB image, ~20-100MB RAM at idle.

Per pod resources:
- CPU request: 100m (0.1 core)
- CPU limit: 500m (0.5 core)
- Memory: 64MiB request, 256MiB limit

Tune based on load testing. Rust uses much less than JVM / Node.

---

# Tầng 16: Performance & production tuning

## 16.1 Profiling production

Apply [performance.md](./performance.md):

```bash
# CPU profile:
cargo flamegraph --bin api -- 
# Or in container with perf

# Memory profile (heaptrack):
heaptrack ./api

# Async task profiler:
# Add tokio-console support
```

Run under realistic load (loadgen tool: hey, vegeta, k6).

## 16.2 Load testing

```bash
# hey:
hey -n 100000 -c 100 http://localhost:3000/

# vegeta:
echo "GET http://localhost:3000/" | vegeta attack -duration=60s -rate=1000 | vegeta report

# k6:
k6 run --vus 100 --duration 60s loadtest.js
```

Identify P99 latency, throughput, error rate under load.

## 16.3 Production release profile

```toml
[profile.release]
opt-level = 3
lto = "fat"           # +5-15% perf, slow compile
codegen-units = 1     # max optimization
strip = true          # smaller binary
panic = "abort"       # smaller, slightly faster
```

## 16.4 Common bottlenecks

| Symptom | Cause | Fix |
|---------|-------|-----|
| High P99 | DB slow query | Index, EXPLAIN ANALYZE |
| Memory growing | leak / cache no eviction | tracing, dhat, set bounds |
| CPU high | hot loop / over-serialization | profile, optimize |
| Connection errors | pool exhausted | tune pool size |
| Timeouts cascading | no circuit breaker | tower-load-shed |

## 16.5 Database tuning

- Add indexes for WHERE/JOIN columns
- `EXPLAIN ANALYZE` slow queries
- Connection pool size = ~workers * factor
- Read replicas for read-heavy
- Query cache (Redis) for hot data

## 16.6 Caching

```rust
use moka::sync::Cache;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub user_cache: Arc<Cache<i64, User>>,
    // ...
}

let state = AppState {
    user_cache: Arc::new(
        Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(60))
            .build()
    ),
    // ...
};

// Use:
async fn get_user(id: i64, state: &AppState) -> Result<User> {
    if let Some(user) = state.user_cache.get(&id) {
        return Ok(user);   // cache hit
    }
    let user = state.user_repo.find(id).await?;
    state.user_cache.insert(id, user.clone());
    Ok(user)
}
```

`moka` — in-memory cache with TTL, LRU. Reduce DB load.

## 16.7 Rate limiting

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let governor_conf = Arc::new(
    GovernorConfigBuilder::default()
        .per_second(100)
        .burst_size(20)
        .finish()
        .unwrap()
);

let app = Router::new()
    .route("/", get(handler))
    .layer(GovernorLayer { config: governor_conf });
```

Per-IP rate limiting. Block abuse.

## 16.8 Connection pool size

```
pool_size = (target_throughput / 1000ms) × avg_query_time_ms

Example:
  target = 100 req/s
  avg query = 50ms
  pool = 100 × 0.050 = 5 connections (minimum)
  + buffer for spikes
```

Too small → queue. Too large → DB overload.

## 16.9 Graceful degradation

```rust
async fn handler(State(state): State<AppState>) -> Result<Json<Data>, AppError> {
    // Try primary DB
    match state.db.query("...").await {
        Ok(d) => Ok(Json(d)),
        Err(_) => {
            // Fallback to cache
            if let Some(cached) = state.cache.get("key") {
                tracing::warn!("DB down, serving cached");
                return Ok(Json(cached));
            }
            Err(AppError::Internal(anyhow!("no fallback")))
        }
    }
}
```

When upstream fails, serve stale data > error.

## 16.10 Production checklist

```
✅ TLS termination (Caddy, nginx, or in-app rustls)
✅ Tracing → log aggregator
✅ Metrics → Prometheus scrape
✅ Health checks /health, /ready
✅ Graceful shutdown
✅ Resource limits set
✅ Config from env (no secrets in code)
✅ Database migrations run on deploy
✅ Connection pool sized
✅ Cache layer (Redis or in-memory)
✅ Rate limiting
✅ Request timeout
✅ Body size limit
✅ CORS configured
✅ Error responses don't leak internals
✅ Backup strategy for DB
✅ Disaster recovery plan
```

---

# Tổng kết — Project applies 16 chapters

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. memory-model     → AppState với Arc, smart pointer choice    │
│ 2. ownership        → handler signatures, extractor moves       │
│ 3. trait            → IntoResponse, Service, FromRequestParts   │
│ 4. generic          → State<T>, Extension<T>, Path<T>           │
│ 5. closure          → middleware closures, handler bodies       │
│ 6. async            → all handlers, db queries, tokio runtime   │
│ 7. error-handling   → AppError + IntoResponse pattern           │
│ 8. macros           → query_as!, instrument, derive(...)         │
│ 9. smart-pointers   → Arc<AppState>, PgPool, moka Cache         │
│ 10. lifetime        → owned API, Arc to avoid lifetimes         │
│ 11. performance     → criterion bench, hot paths, caching       │
│ 12. observability   → tracing + prometheus + OTel               │
│ 13. iterator        → query result processing, .iter().map()    │
│ 14. unsafe-rust     → typically NONE in app code (libraries use)│
│ 15. testing         → unit + integration + e2e patterns         │
│ 16. embedded-rust   → not applicable (server, not MCU)          │
└─────────────────────────────────────────────────────────────────┘
```

---

# 12 nguyên tắc senior axum project

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. Workspace layout: api / domain / db separation.               │
│                                                                  │
│ 2. AppState = single shared state, Clone-able (Arc inside).      │
│                                                                  │
│ 3. Error type with IntoResponse — consistent error format.       │
│                                                                  │
│ 4. Don't leak internals to clients (hide stack trace, DB names). │
│                                                                  │
│ 5. Validation in extractor — handlers receive valid data.        │
│                                                                  │
│ 6. Compile-time SQL (sqlx::query!) — typos = compile error.      │
│                                                                  │
│ 7. Every request: trace span with request_id.                    │
│                                                                  │
│ 8. 4 Golden Signals exposed: traffic, error, latency, saturation.│
│                                                                  │
│ 9. Test pyramid: unit (domain) > integration (handler) > e2e.    │
│                                                                  │
│ 10. Graceful shutdown — finish in-flight on SIGTERM.             │
│                                                                  │
│ 11. Config from env + file + validate at startup.                │
│                                                                  │
│ 12. Production checklist: TLS, health, metrics, limits.          │
└──────────────────────────────────────────────────────────────────┘
```

---

# axum project toolkit

| Crate | Purpose |
|-------|---------|
| `axum` | Web framework |
| `tokio` | Async runtime |
| `tower` / `tower-http` | Middleware |
| `serde` / `serde_json` | Serialization |
| `validator` | Input validation |
| `sqlx` | Database |
| `tracing` / `tracing-subscriber` | Logging |
| `metrics` / `metrics-exporter-prometheus` | Metrics |
| `jsonwebtoken` | JWT |
| `argon2` | Password hashing |
| `thiserror` / `anyhow` | Error handling |
| `config` | Configuration |
| `moka` | In-memory cache |
| `tower-governor` | Rate limiting |
| `reqwest` | HTTP client (for tests) |
| `mockall` | Mocking |

---

# Lộ trình tiếp theo

Bạn đã có 17 chủ đề:

```
1-16. (memory-model, ownership, trait, generic, closure, async,
       error-handling, macros, smart-pointers, lifetime, performance,
       observability, iterator, unsafe-rust, testing, embedded-rust)
17. axum-project       ← MỚI
```

Còn 1 topic ứng dụng cuối:

- **Database deep dive** — sqlx, sea-orm, transaction patterns, migration strategies, connection pool tuning, query optimization

Báo nếu muốn đào sâu! 🦀⚡
