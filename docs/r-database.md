# Database trong Rust — Deep Dive

> Tài liệu thứ 18 — chương cuối trong bộ Rust nền tảng. Đọc trước:
> - [async.md](./async.md) — DB clients chủ yếu async
> - [error-handling.md](./error-handling.md) — DB error patterns
> - [smart-pointers.md](./smart-pointers.md) — Arc connection pool
> - [observability.md](./observability.md) — DB query tracing
> - [testing.md](./testing.md) — DB testing patterns
> - [axum-project.md](./axum-project.md) — DB trong web app
>
> Database = **đáy** của hầu hết hệ thống. Performance, correctness, scalability của app
> phụ thuộc lớn vào cách design DB layer.
>
> Tài liệu này deep dive:
> - Library choice: sqlx vs sea-orm vs diesel
> - Connection pooling — sizing, monitoring
> - Transactions, isolation levels
> - Migration strategies — versioning, rollback
> - Query optimization — EXPLAIN, indexes, N+1
> - Advanced patterns — sharding, read replicas, multi-tenancy
> - Testing strategies
>
> Tài liệu lấy **PostgreSQL** làm chuẩn (most popular for Rust). MySQL, SQLite tương tự.

---

# Mục lục

- [Tầng 1: DB trong Rust — Library landscape](#tầng-1-db-trong-rust--library-landscape)
- [Tầng 2: sqlx — Compile-time SQL deep](#tầng-2-sqlx--compile-time-sql-deep)
- [Tầng 3: sea-orm — High-level ORM](#tầng-3-sea-orm--high-level-orm)
- [Tầng 4: diesel — Synchronous classic](#tầng-4-diesel--synchronous-classic)
- [Tầng 5: Connection Pool — Management & Tuning](#tầng-5-connection-pool--management--tuning)
- [Tầng 6: Transactions — ACID & isolation levels](#tầng-6-transactions--acid--isolation-levels)
- [Tầng 7: Migrations — Schema versioning](#tầng-7-migrations--schema-versioning)
- [Tầng 8: Query Optimization — EXPLAIN, indexes](#tầng-8-query-optimization--explain-indexes)
- [Tầng 9: N+1 và batch patterns](#tầng-9-n1-và-batch-patterns)
- [Tầng 10: Caching layers](#tầng-10-caching-layers)
- [Tầng 11: Read replicas & write/read split](#tầng-11-read-replicas--writeread-split)
- [Tầng 12: Sharding — Horizontal scaling](#tầng-12-sharding--horizontal-scaling)
- [Tầng 13: Multi-tenancy patterns](#tầng-13-multi-tenancy-patterns)
- [Tầng 14: Soft deletes, audit logs, time-series](#tầng-14-soft-deletes-audit-logs-time-series)
- [Tầng 15: Testing strategies](#tầng-15-testing-strategies)
- [Tầng 16: Common pitfalls & antipatterns](#tầng-16-common-pitfalls--antipatterns)

---

# Tầng 1: DB trong Rust — Library landscape

## 1.1 Major Rust DB libraries

| Library | Style | Compile-time check | Async | Notes |
|---------|-------|-------------------|-------|-------|
| **sqlx** | Raw SQL | ✅ | ✅ | Most popular |
| **sea-orm** | ORM | ❌ | ✅ | Active record style |
| **diesel** | DSL query builder | ✅ | ⚠️ (separate `diesel-async`) | Original Rust ORM |
| **tokio-postgres** | Low-level | ❌ | ✅ | Underlying driver |
| **rusqlite** | SQLite | ❌ | ❌ (sync) | SQLite only |
| **mongodb** | NoSQL | ❌ | ✅ | Official MongoDB driver |
| **redis** | Cache/store | ❌ | ✅ | Redis client |

## 1.2 Decision matrix

```
   Your needs:                       Choose:
   ────────────                       ──────
   
   • Direct SQL control               sqlx
   • Compile-time SQL safety
   • Async, modern
   
   • ORM-style, less SQL              sea-orm
   • Active record CRUD
   • Async
   
   • Heavy query composition           diesel
   • Compile-time DSL safety
   • OK with synchronous
   
   • PostgreSQL only, low-level        tokio-postgres
   • Maximum control
   
   • Embedded SQLite                   rusqlite
   • No server needed
```

## 1.3 sqlx is the go-to choice

For most projects in 2024+:
- ✅ Compile-time SQL validation
- ✅ Async (tokio)
- ✅ Multiple DBs (PostgreSQL, MySQL, SQLite, MS SQL)
- ✅ Migration tool included
- ✅ Mature, large community
- ❌ Less abstraction than ORM (some find verbose)

Tài liệu chủ yếu cover **sqlx** với references đến alternatives.

## 1.4 Driver vs library

```
   ┌──────────────────────────────────────────────────────────┐
   │  YOUR APP                                                │
   │      │                                                   │
   │      │ uses                                              │
   │      ▼                                                   │
   │  ┌──────────────────────────────────────────────┐        │
   │  │ sqlx / sea-orm / diesel (LIBRARY)            │        │
   │  │   - Query builder / macro                    │        │
   │  │   - Connection pool                           │       │
   │  │   - Migration                                │        │
   │  │   - Type mapping                             │        │
   │  └──────────────────────────────────────────────┘        │
   │      │                                                   │
   │      │ uses                                              │
   │      ▼                                                   │
   │  ┌──────────────────────────────────────────────┐        │
   │  │ DRIVER (low-level)                           │        │
   │  │   - Protocol implementation                   │       │
   │  │   - TCP connection                            │       │
   │  │   - Binary protocol                           │       │
   │  │   - PostgreSQL: tokio-postgres / sqlx-postgres│       │
   │  │   - MySQL: mysql_async                        │       │
   │  └──────────────────────────────────────────────┘        │
   │      │                                                   │
   │      ▼ TCP                                               │
   │  ┌──────────────────────────────────────────────┐        │
   │  │ DATABASE SERVER                              │        │
   │  └──────────────────────────────────────────────┘        │
   └──────────────────────────────────────────────────────────┘
```

You typically don't use driver directly. Library wraps it.

---

# Tầng 2: sqlx — Compile-time SQL deep

## 2.1 Setup

```toml
[dependencies]
sqlx = { version = "0.8", features = [
    "postgres",
    "runtime-tokio",
    "tls-rustls",      # or "tls-native-tls"
    "macros",
    "chrono",
    "uuid",
    "json",
    "migrate",
] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

## 2.2 Connect

```rust
use sqlx::PgPool;

pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPool::connect(url).await
}
```

`PgPool` is `Arc` internally — clone is cheap.

## 2.3 query_as! — Compile-time validated

```rust
#[derive(sqlx::FromRow)]
struct User {
    id: i64,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

let user: User = sqlx::query_as!(
    User,
    "SELECT id, email, created_at FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;
```

### What sqlx checks at COMPILE TIME:
1. SQL syntax (parse via Postgres)
2. All columns exist
3. Column types match struct fields
4. `$1` parameter type matches `user_id`
5. Result count expected (fetch_one vs fetch_all)

### Compile error examples:
```
error: column "emaiil" does not exist
  --> src/users.rs:10:5
   |
10 |     "SELECT id, emaiil, created_at FROM users WHERE id = $1",
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

Typo caught **at compile**.

## 2.4 Fetch methods

```rust
.fetch_one(&pool)        // Error if 0 or >1 rows
.fetch_optional(&pool)   // Option<T>, OK if 0 rows
.fetch_all(&pool)        // Vec<T>
.fetch(&pool)            // Stream<Item = Result<T>>

// For non-query (INSERT, UPDATE, DELETE):
.execute(&pool)          // Returns affected row count
```

`.fetch()` returns Stream — good for very large result sets:
```rust
let mut stream = sqlx::query!("SELECT * FROM big_table").fetch(&pool);
while let Some(row) = stream.try_next().await? {
    process(row);   // process one at a time, no big alloc
}
```

## 2.5 Bindings

```rust
let id: i64 = 42;
let name: &str = "alice";

sqlx::query!("SELECT * FROM users WHERE id = $1 AND name = $2", id, name)
    .fetch_one(&pool).await?;
```

Parameters bind safely. **No SQL injection possible** (prepared statements).

## 2.6 Type mapping (PostgreSQL ↔ Rust)

| PostgreSQL | Rust |
|------------|------|
| `INTEGER` / `INT` | `i32` |
| `BIGINT` / `INT8` | `i64` |
| `SERIAL` | `i32` (uses i32) |
| `BIGSERIAL` | `i64` |
| `TEXT`, `VARCHAR` | `String` / `&str` |
| `BOOLEAN` | `bool` |
| `REAL` | `f32` |
| `DOUBLE PRECISION` | `f64` |
| `NUMERIC` | `bigdecimal::BigDecimal` (with `bigdecimal` feature) |
| `TIMESTAMPTZ` | `chrono::DateTime<Utc>` (with `chrono` feature) |
| `UUID` | `uuid::Uuid` (with `uuid` feature) |
| `JSONB` | `serde_json::Value` (with `json` feature) |
| `BYTEA` | `Vec<u8>` |
| `INTEGER[]` | `Vec<i32>` |
| `NULL` value | `Option<T>` |

## 2.7 Nullable columns

```rust
// Column NOT NULL → T
// Column NULL allowed → Option<T>

#[derive(sqlx::FromRow)]
struct User {
    id: i64,
    email: String,          // NOT NULL
    full_name: Option<String>,  // NULL allowed
}
```

sqlx checks nullability against schema.

## 2.8 Override types

```rust
sqlx::query_as!(
    User,
    r#"SELECT id, email, status as "status: Status" FROM users WHERE id = $1"#,
    user_id
)
```

`as "status: Status"` — tell sqlx column maps to enum `Status` (with `sqlx::Type` derived).

```rust
#[derive(sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "lowercase")]
enum Status {
    Active,
    Inactive,
    Banned,
}
```

## 2.9 Offline mode

```bash
cargo install sqlx-cli --no-default-features --features postgres

cargo sqlx prepare
# Generates .sqlx/ folder with query metadata
git add .sqlx/
```

CI build:
```bash
SQLX_OFFLINE=true cargo build
# Uses .sqlx/ — no DB connection needed
```

Update `.sqlx/` when queries change:
```bash
cargo sqlx prepare
```

## 2.10 Dynamic queries — Limitations

```rust
// ❌ Can't dynamically construct SQL in query_as!
let column = "email";
sqlx::query_as!(User, "SELECT id, {column} FROM users", column)  // COMPILE ERROR
```

For dynamic SQL, use `query` (without `!`) — no compile-time check:

```rust
let column = match user_input {
    "email" => "email",
    "name" => "full_name",
    _ => return Err(...),  // whitelist!
};

let q = format!("SELECT id, {} FROM users", column);
let row = sqlx::query(&q).fetch_one(&pool).await?;
```

⚠️ **Whitelist** user input. Never interpolate user-provided column names directly → SQL injection.

For query builders: see sea-orm or use `sea-query` crate (separate from sea-orm).

---

# Tầng 3: sea-orm — High-level ORM

## 3.1 Setup

```toml
[dependencies]
sea-orm = { version = "1", features = [
    "sqlx-postgres",
    "runtime-tokio-rustls",
    "macros",
] }
```

## 3.2 Entity definition

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub email: String,
    pub full_name: Option<String>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order::Entity")]
    Orders,
}

impl ActiveModelBehavior for ActiveModel {}
```

Generates entity types: `Entity`, `Model`, `ActiveModel`, `Column`.

## 3.3 CRUD operations

```rust
use sea_orm::{Database, EntityTrait, Set, ActiveModelTrait};

let db = Database::connect("postgres://...").await?;

// Insert
let new_user = users::ActiveModel {
    email: Set("alice@test.com".to_string()),
    full_name: Set(Some("Alice".to_string())),
    ..Default::default()
};
let user = new_user.insert(&db).await?;

// Find
let user = users::Entity::find_by_id(42).one(&db).await?;
let users = users::Entity::find().all(&db).await?;

// Update
let mut user: users::ActiveModel = user.unwrap().into();
user.full_name = Set(Some("Alice Smith".to_string()));
user.update(&db).await?;

// Delete
users::Entity::delete_by_id(42).exec(&db).await?;
```

## 3.4 Query builder

```rust
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;

let users = users::Entity::find()
    .filter(users::Column::Email.contains("@example.com"))
    .filter(users::Column::CreatedAt.gt(some_date))
    .order_by_asc(users::Column::CreatedAt)
    .limit(10)
    .all(&db)
    .await?;
```

Type-safe DSL. Refactor-friendly.

## 3.5 Relations / joins

```rust
let user_with_orders = users::Entity::find_by_id(42)
    .find_with_related(orders::Entity)
    .all(&db)
    .await?;
// Returns Vec<(User, Vec<Order>)>
```

## 3.6 Migrations

```rust
// migration/src/m20240526_create_users.rs
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Users::Table)
                .col(ColumnDef::new(Users::Id).big_integer().primary_key())
                .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                .to_owned()
        ).await
    }
    
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Users::Table).to_owned()).await
    }
}
```

DB-agnostic migrations (works on Postgres, MySQL, SQLite).

## 3.7 sea-orm vs sqlx — When to choose?

```
   sqlx:
   • You want direct SQL control
   • Compile-time SQL safety important
   • Queries are simple enough to write SQL
   
   sea-orm:
   • Heavy CRUD operations (simple boilerplate)
   • Want ORM-style abstractions
   • Don't mind not having compile-time SQL check
   • DB-agnostic schema desired
   
   Many projects mix: sea-orm cho CRUD, sqlx cho complex/perf-critical queries
```

---

# Tầng 4: diesel — Synchronous classic

## 4.1 Briefly

```toml
[dependencies]
diesel = { version = "2", features = ["postgres", "chrono", "uuid"] }
```

```rust
use diesel::prelude::*;
use diesel::pg::PgConnection;

fn main() {
    let mut conn = PgConnection::establish("postgres://...").unwrap();
    
    let user: User = users::table
        .filter(users::id.eq(42))
        .first(&mut conn)
        .unwrap();
}
```

diesel:
- **Compile-time SQL** via DSL (different approach from sqlx)
- **Synchronous** (mature, predictable)
- Async via `diesel-async` (separate crate)
- More mature than sqlx (older)

Trade-off:
- DSL learning curve
- Less popular than sqlx for new Rust projects
- Excellent for complex queries with strong typing

For new projects, sqlx more popular. diesel still solid choice — pick based on team preference.

---

# Tầng 5: Connection Pool — Management & Tuning

## 5.1 Why pool?

```
   Without pool — connect per query:
   ──────────────────────────────────
   
   Each request:
   1. TCP connect (1-10ms)
   2. SSL handshake (10-50ms)
   3. Auth (5ms)
   4. Query (1-50ms)
   5. Disconnect
   
   ⟹ Connection overhead dominates query time!
   
   
   With pool — pre-established connections:
   ─────────────────────────────────────────
   
   Pool: [conn1] [conn2] [conn3] ...
                    │
                    │ borrow
                    ▼
   Query → use connection → return to pool
   
   ⟹ Just query time, ~1ms overhead
```

## 5.2 sqlx pool setup

```rust
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Some(Duration::from_secs(600)))
    .max_lifetime(Some(Duration::from_secs(1800)))
    .test_before_acquire(true)
    .connect(&database_url)
    .await?;
```

Settings:
- **max_connections**: hard ceiling
- **min_connections**: maintain warm connections
- **acquire_timeout**: fail fast if pool exhausted
- **idle_timeout**: close idle conns
- **max_lifetime**: rotate stale conns (helps with DB-side disconnects)
- **test_before_acquire**: ping conn before use (catches broken)

## 5.3 Sizing the pool

Common formula:
```
pool_size = (target_throughput / 1000ms) × avg_query_time_ms
```

Example:
- 1000 req/s target
- 5ms avg query
- = 1000 × 0.005 = 5 connections minimum
- + buffer for variability: ~10-20

### Anti-pattern: huge pool

```rust
.max_connections(500)   // Over Postgres typical max_connections=100
```

→ Connection refused. Tune Postgres `max_connections` too.

Postgres typically can handle 100-300 connections well. Beyond that, use **PgBouncer** (transaction pooler).

## 5.4 Postgres max_connections

```sql
SHOW max_connections;   -- typically 100
```

For more concurrency:
- Increase `max_connections` (tune `shared_buffers` accordingly)
- Use **PgBouncer** as transaction-level pooler in front

```
   App pools (many small)            PgBouncer            Postgres
   ┌──────┐ ┌──────┐ ... ┌──────┐    (transaction       (max_connections
   │ App1 │ │ App2 │     │ AppN │    pooler)              = 100)
   │ pool │ │ pool │     │ pool │
   │ 20   │ │ 20   │     │ 20   │ ──►  Pool: 100  ──►  100 actual conns
   └──────┘ └──────┘     └──────┘     reused per txn
```

App can have many small pools, PgBouncer manages real DB connections.

## 5.5 Monitoring pool

```rust
// Periodically log pool state:
async fn report_pool_state(pool: &PgPool) {
    let size = pool.size();
    let idle = pool.num_idle();
    let used = size - idle as u32;
    
    metrics::gauge!("db_pool_size").set(size as f64);
    metrics::gauge!("db_pool_used").set(used as f64);
    metrics::gauge!("db_pool_idle").set(idle as f64);
}
```

Alert when used = max → pool exhausted.

## 5.6 Pool exhaustion symptoms

```
   ❌ Symptoms:
   • Connection acquisition timeouts
   • Cascading failures
   • Long P99 latency (queue at pool)
   
   ❌ Causes:
   • Long-running queries holding connection
   • Transactions not closed
   • Pool size too small for load
   • Connection leaks (handle not returned)
   • DB slow (queries pile up)
```

## 5.7 Connection-per-request anti-pattern

```rust
// ❌ Creating pool per request — defeats the purpose
async fn handler() {
    let pool = PgPool::connect(URL).await?;   // ❌ new pool!
    sqlx::query!("...").fetch_one(&pool).await?;
}
```

Create pool **once** at startup, share via `Arc` / `State<AppState>`.

## 5.8 Postgres-specific tuning

```sql
-- Postgres config (postgresql.conf):
shared_buffers = 4GB         -- 25% of RAM typical
effective_cache_size = 12GB  -- 75% of RAM
work_mem = 16MB              -- per query operation
maintenance_work_mem = 1GB   -- for VACUUM, CREATE INDEX
max_connections = 200
```

Tune based on workload. PgTune (online tool) gives starting points.

---

# Tầng 6: Transactions — ACID & isolation levels

## 6.1 ACID properties

- **A**tomicity: all-or-nothing (commit or rollback)
- **C**onsistency: DB stays valid (constraints enforced)
- **I**solation: concurrent txns don't interfere
- **D**urability: committed data survives crash

Postgres provides all 4.

## 6.2 sqlx transactions

```rust
let mut tx = pool.begin().await?;

sqlx::query!("INSERT INTO users (email) VALUES ($1)", email)
    .execute(&mut *tx).await?;

sqlx::query!("INSERT INTO profiles (user_id) VALUES (lastval())")
    .execute(&mut *tx).await?;

tx.commit().await?;
// If commit not called → rollback on drop
```

`begin()` start transaction. `commit()` finalize. `rollback()` undo (or drop without commit).

## 6.3 Isolation levels

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │   READ UNCOMMITTED — see uncommitted changes (dirty reads)  │
   │   ───────────────                                           │
   │   Postgres: NOT supported (treats as READ COMMITTED)        │
   │                                                             │
   │   READ COMMITTED — see only committed (DEFAULT)             │
   │   ────────────                                              │
   │   • Dirty reads: NO                                         │
   │   • Non-repeatable reads: YES (same row may change)         │
   │   • Phantom reads: YES (new rows appear)                    │
   │                                                             │
   │   REPEATABLE READ — same data on re-read                    │
   │   ──────────────                                            │
   │   • Dirty reads: NO                                         │
   │   • Non-repeatable reads: NO                                │
   │   • Phantom reads: NO (in Postgres, only theoretical risk)  │
   │                                                             │
   │   SERIALIZABLE — appears as serial execution                │
   │   ────────────                                              │
   │   • All anomalies prevented                                 │
   │   • Slowest, highest risk of conflict                       │
   │   • Postgres uses SSI (Serializable Snapshot Isolation)     │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
   
   
   sqlx set isolation:
   ───────────────────
   
   let mut tx = pool.begin().await?;
   sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
       .execute(&mut *tx).await?;
   // ... operations ...
```

## 6.4 Anomalies explained

### Dirty read (READ UNCOMMITTED only)
```
T1: UPDATE x = 5
T2: SELECT x  → sees 5 (uncommitted!)
T1: ROLLBACK
T2: now has stale data
```

### Non-repeatable read (READ COMMITTED)
```
T1: SELECT balance → 100
T2: UPDATE balance = 200; COMMIT
T1: SELECT balance → 200 (changed mid-transaction!)
```

### Phantom read
```
T1: SELECT COUNT(*) WHERE x > 10 → 5
T2: INSERT row with x=15; COMMIT
T1: SELECT COUNT(*) WHERE x > 10 → 6 (new row appeared!)
```

## 6.5 Choose isolation level

```
   READ COMMITTED:  most cases (default Postgres)
                     fast, allows some anomalies
   
   REPEATABLE READ: financial, accounting, snapshot reads
                     for consistent reads within transaction
   
   SERIALIZABLE:    when correctness > speed
                     concurrent transactions logically serial
                     handle serialization failures (retry)
```

## 6.6 Serializable conflict handling

```rust
async fn transfer(from: i64, to: i64, amount: i64, pool: &PgPool) -> Result<()> {
    for attempt in 0..3 {
        let result = transfer_attempt(from, to, amount, pool).await;
        match result {
            Ok(_) => return Ok(()),
            Err(sqlx::Error::Database(e)) if is_serialization_failure(&e) => {
                tracing::warn!(attempt, "serialization conflict, retrying");
                tokio::time::sleep(Duration::from_millis(50 << attempt)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(anyhow!("max retries"))
}

async fn transfer_attempt(...) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx).await?;
    
    // ... debit + credit ...
    
    tx.commit().await?;
    Ok(())
}
```

Postgres returns serialization failure error (SQLSTATE `40001`). Retry with backoff.

## 6.7 SELECT FOR UPDATE — Pessimistic locking

```rust
let mut tx = pool.begin().await?;

let user = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = $1 FOR UPDATE",   // lock row
    user_id
)
.fetch_one(&mut *tx).await?;

// Now no other txn can modify this row until our commit

sqlx::query!("UPDATE users SET balance = $1 WHERE id = $2", new_balance, user_id)
    .execute(&mut *tx).await?;

tx.commit().await?;
```

`FOR UPDATE`: exclusive lock on row. Other txns wait.

Useful when:
- Update-after-read pattern
- Avoid lost updates
- Critical sections

Caveat: deadlock potential if 2 txns lock in different order.

## 6.8 Advisory locks

```rust
sqlx::query!("SELECT pg_advisory_lock($1)", lock_key as i64)
    .execute(&pool).await?;

// ... critical section ...

sqlx::query!("SELECT pg_advisory_unlock($1)", lock_key as i64)
    .execute(&pool).await?;
```

App-defined locks, not tied to rows. Useful for:
- Singleton process (one worker)
- Coordinating cross-process work
- Migration safety

---

# Tầng 7: Migrations — Schema versioning

## 7.1 Migration concept

```
   v0 (initial)
       │
       ▼ migration 001: create users table
   v1
       │
       ▼ migration 002: add email column
   v2
       │
       ▼ migration 003: create orders table
   v3
   
   Track in `_sqlx_migrations` table (or similar)
   Each migration runs once, recorded.
```

## 7.2 sqlx migrations

```bash
sqlx migrate add create_users
# Creates: migrations/20240526120000_create_users.sql
```

```sql
-- migrations/20240526120000_create_users.sql
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

Run:
```bash
sqlx migrate run
```

Or programmatically:
```rust
sqlx::migrate!("./migrations").run(&pool).await?;
```

App runs migrations at startup → DB always at correct schema.

## 7.3 Reversible migrations

```bash
sqlx migrate add -r create_users
# Creates BOTH:
# - migrations/<ts>_create_users.up.sql
# - migrations/<ts>_create_users.down.sql
```

```sql
-- up.sql
CREATE TABLE users (...);

-- down.sql
DROP TABLE users;
```

Roll back:
```bash
sqlx migrate revert
```

⚠️ Reversibility hard in practice. Some changes (data migration) hard to reverse cleanly.

## 7.4 Migration patterns

### Pattern 1: Add nullable column (safe)

```sql
ALTER TABLE users ADD COLUMN phone TEXT;
-- Existing rows: phone = NULL
```

Backward compat — old code ignores new column.

### Pattern 2: Add NOT NULL column (multi-step)

```sql
-- Migration 1: add nullable
ALTER TABLE users ADD COLUMN phone TEXT;

-- App update: code populates phone

-- Migration 2 (later): backfill
UPDATE users SET phone = 'unknown' WHERE phone IS NULL;

-- Migration 3: make NOT NULL
ALTER TABLE users ALTER COLUMN phone SET NOT NULL;
```

Multi-step deploy. Avoid downtime.

### Pattern 3: Rename column (risky)

```sql
-- ❌ Don't do this in production without care:
ALTER TABLE users RENAME COLUMN name TO full_name;

-- Old code reads `name` → error
```

Pattern:
1. Add new column `full_name`
2. Deploy app reading BOTH `name` AND `full_name`
3. Migrate data: `UPDATE users SET full_name = name`
4. Deploy app writing only `full_name`
5. Drop old `name`

### Pattern 4: Drop column (safe with care)

```sql
-- App update: stop reading column
-- Wait for deploys

ALTER TABLE users DROP COLUMN old_field;
```

Don't drop columns code still reads.

## 7.5 Online schema migrations

For big tables (millions of rows):

```sql
-- ❌ Locks table for hours:
ALTER TABLE big_table ADD COLUMN x TEXT NOT NULL DEFAULT 'foo';

-- ✅ Postgres 11+: instant for ADD without DEFAULT
ALTER TABLE big_table ADD COLUMN x TEXT;

-- Or use pg_repack / gh-ost-style tools
```

Postgres specifics:
- ADD COLUMN with default: in Postgres 11+, instant (stored as metadata)
- ADD INDEX CONCURRENTLY: non-blocking
- DROP COLUMN: instant (only metadata)
- ALTER TYPE: rewrites table (slow)

```sql
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);
```

Doesn't block writes. Use for production.

## 7.6 Data migrations

```sql
-- Schema change + data change in one migration
ALTER TABLE users ADD COLUMN normalized_email TEXT;
UPDATE users SET normalized_email = LOWER(email);
ALTER TABLE users ALTER COLUMN normalized_email SET NOT NULL;
CREATE INDEX idx_users_norm_email ON users(normalized_email);
```

For very large tables, batch the UPDATE:
```sql
-- Loop in app code:
UPDATE users SET normalized_email = LOWER(email) 
WHERE id BETWEEN 1 AND 10000 AND normalized_email IS NULL;
```

Avoid one giant transaction.

## 7.7 Migration testing

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_after_migrations(pool: PgPool) {
    // Pool has all migrations applied
    let result = sqlx::query!("SELECT * FROM users WHERE id = $1", 1)
        .fetch_optional(&pool).await.unwrap();
    // ...
}
```

`sqlx::test` runs migrations against fresh test DB per test.

## 7.8 CI: migration check

```yaml
# CI step:
- name: Run migrations
  env:
    DATABASE_URL: postgres://...
  run: sqlx migrate run
```

Verify migrations apply cleanly. Catch syntax errors early.

---

# Tầng 8: Query Optimization — EXPLAIN, indexes

## 8.1 EXPLAIN

```sql
EXPLAIN ANALYZE
SELECT * FROM users WHERE email = 'alice@test.com';
```

Output:
```
Seq Scan on users (cost=0.00..18.50 rows=1 width=88)
                  (actual time=0.012..0.150 rows=1 loops=1)
  Filter: (email = 'alice@test.com'::text)
  Rows Removed by Filter: 999
Planning Time: 0.05 ms
Execution Time: 0.18 ms
```

Reading:
- **Seq Scan**: full table scan — usually bad
- **rows=1**: estimated → actual rows
- **Rows Removed**: filter inefficiency
- **time**: actual execution time

After `CREATE INDEX idx_users_email ON users(email)`:
```
Index Scan using idx_users_email on users (cost=0.28..8.29 rows=1)
                                          (actual time=0.020..0.022 rows=1)
  Index Cond: (email = 'alice@test.com'::text)
Planning Time: 0.10 ms
Execution Time: 0.04 ms
```

4x faster. **Always EXPLAIN** slow queries.

## 8.2 Index types (Postgres)

```sql
-- B-tree (default, most common)
CREATE INDEX idx_users_email ON users(email);

-- Multi-column
CREATE INDEX idx_orders_user_status ON orders(user_id, status);

-- Partial index (only matching rows)
CREATE INDEX idx_active_users ON users(email) WHERE active = true;

-- Expression index
CREATE INDEX idx_users_lower_email ON users(LOWER(email));

-- Unique
CREATE UNIQUE INDEX idx_users_email_unique ON users(email);

-- GIN (for arrays, JSONB)
CREATE INDEX idx_articles_tags ON articles USING GIN(tags);

-- GiST (geometric, full-text)
-- BRIN (block range, for large sequential data)
```

## 8.3 When indexes help

```
   ✅ WHERE clause:        WHERE email = ...
   ✅ JOIN:                ON orders.user_id = users.id
   ✅ ORDER BY:            ORDER BY created_at DESC
   ✅ Unique constraints
   
   ❌ Small tables (< 1000 rows) — seq scan faster
   ❌ Columns with low cardinality (BOOLEAN) — usually
   ❌ Tables with lots of writes (indexes slow inserts)
```

## 8.4 Multi-column index ordering

```sql
CREATE INDEX idx ON orders(user_id, status);

-- Helps:
SELECT * FROM orders WHERE user_id = 42;
SELECT * FROM orders WHERE user_id = 42 AND status = 'paid';

-- Doesn't help:
SELECT * FROM orders WHERE status = 'paid';   -- can't use index!
```

Order matters: leftmost prefix usable.

Index `(a, b, c)` helps queries on `a`, `(a, b)`, `(a, b, c)` — NOT `b` alone, `c` alone.

## 8.5 Covering index

```sql
CREATE INDEX idx_users_email_name ON users(email) INCLUDE (full_name);

SELECT email, full_name FROM users WHERE email = 'x@y.com';
-- Index has both — no need to fetch row data!
```

Faster — index-only scan.

## 8.6 Common query patterns

### Pagination

```sql
-- ❌ OFFSET slow for deep pages (scans all preceding rows)
SELECT * FROM posts ORDER BY id LIMIT 20 OFFSET 100000;

-- ✅ Cursor-based pagination
SELECT * FROM posts WHERE id > $last_seen_id ORDER BY id LIMIT 20;
```

OFFSET 100000 = scan 100000 rows. Cursor-based = O(1) with index.

### Aggregation

```sql
-- Add covering index for aggregate
CREATE INDEX idx_orders_user_amount ON orders(user_id) INCLUDE (amount);

SELECT user_id, SUM(amount) FROM orders GROUP BY user_id;
-- Can use index-only scan
```

### Sort

```sql
CREATE INDEX idx_posts_created ON posts(created_at DESC);

SELECT * FROM posts ORDER BY created_at DESC LIMIT 20;
-- Index already sorted descending
```

## 8.7 EXPLAIN options

```sql
EXPLAIN ANALYZE         -- run query + show plan
EXPLAIN (ANALYZE, BUFFERS)  -- + buffer hit/miss
EXPLAIN (FORMAT JSON)   -- machine-readable
```

`BUFFERS` shows cache hits vs disk reads — useful for tuning `shared_buffers`.

## 8.8 pg_stat_statements

Track query stats over time:

```sql
CREATE EXTENSION pg_stat_statements;

SELECT query, calls, mean_exec_time, total_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;
```

Find slowest queries in production. Tune those.

## 8.9 N+1 problem (preview, deep in next Tầng)

```rust
let users = fetch_users().await?;
for user in users {
    let orders = fetch_orders_for(user.id).await?;   // N queries!
}
```

Solution: JOIN or batch — Tầng 9.

---

# Tầng 9: N+1 và batch patterns

## 9.1 N+1 anatomy

```
   1 query to fetch N users
   N queries to fetch their orders
   = 1 + N queries
   
   100 users → 101 queries
   
   At 5ms each → 505ms total!
```

## 9.2 Solution 1: JOIN

```sql
SELECT u.id, u.email, o.id AS order_id, o.amount
FROM users u
LEFT JOIN orders o ON o.user_id = u.id;
```

```rust
struct UserOrderRow {
    id: i64,
    email: String,
    order_id: Option<i64>,
    amount: Option<i64>,
}

let rows: Vec<UserOrderRow> = sqlx::query_as!(
    UserOrderRow,
    r#"SELECT u.id, u.email, o.id as "order_id?", o.amount as "amount?"
       FROM users u LEFT JOIN orders o ON o.user_id = u.id"#
).fetch_all(&pool).await?;

// Group manually
let mut by_user: HashMap<i64, UserWithOrders> = HashMap::new();
for row in rows {
    let entry = by_user.entry(row.id).or_insert_with(|| UserWithOrders {
        id: row.id,
        email: row.email,
        orders: vec![],
    });
    if let Some(order_id) = row.order_id {
        entry.orders.push(Order { id: order_id, amount: row.amount.unwrap() });
    }
}
```

1 query.

## 9.3 Solution 2: ANY (batch by IDs)

```sql
-- Fetch all users
SELECT * FROM users WHERE active = true;

-- Then fetch all orders for those user IDs:
SELECT * FROM orders WHERE user_id = ANY($1);  -- array of IDs
```

```rust
let users = sqlx::query_as!(User, "SELECT * FROM users WHERE active = true")
    .fetch_all(&pool).await?;

let user_ids: Vec<i64> = users.iter().map(|u| u.id).collect();

let orders = sqlx::query_as!(
    Order,
    "SELECT * FROM orders WHERE user_id = ANY($1)",
    &user_ids[..]
).fetch_all(&pool).await?;

// Group orders by user_id in app code:
let orders_by_user: HashMap<i64, Vec<Order>> = orders.into_iter()
    .into_group_map_by(|o| o.user_id);
```

2 queries. Cleaner than JOIN for complex shapes.

## 9.4 Solution 3: JSON aggregation

```sql
SELECT u.id, u.email,
       COALESCE(JSON_AGG(o.* ORDER BY o.created_at)
                FILTER (WHERE o.id IS NOT NULL),
                '[]'::JSON) AS orders
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
GROUP BY u.id, u.email;
```

```rust
#[derive(sqlx::FromRow)]
struct UserWithOrders {
    id: i64,
    email: String,
    #[sqlx(json)]
    orders: Vec<Order>,
}
```

1 query, no manual grouping. Postgres aggregates JSON.

## 9.5 Batch inserts

```sql
-- ❌ N queries
for user in users {
    INSERT INTO users (email) VALUES ($1);
}

-- ✅ 1 query
INSERT INTO users (email)
SELECT * FROM UNNEST($1::TEXT[]);
```

```rust
let emails: Vec<String> = users.iter().map(|u| u.email.clone()).collect();

sqlx::query!(
    "INSERT INTO users (email) SELECT * FROM UNNEST($1::TEXT[])",
    &emails[..]
).execute(&pool).await?;
```

Faster — 1 network round-trip, 1 commit, 1 parse plan.

## 9.6 COPY for bulk loading

```rust
let mut copy_in = pool.acquire().await?
    .copy_in_raw("COPY users (email, name) FROM STDIN WITH (FORMAT csv)").await?;

for user in &users {
    let line = format!("{},{}\n", user.email, user.name);
    copy_in.send(line.as_bytes()).await?;
}

let count = copy_in.finish().await?;
```

COPY = Postgres bulk loading protocol. Magnitude faster than INSERT for large batches (millions of rows).

## 9.7 Pagination with cursors

```sql
-- Avoid OFFSET for large datasets
SELECT * FROM posts WHERE id > $cursor ORDER BY id LIMIT 20;
```

```rust
async fn list_posts(after_id: Option<i64>) -> Result<Vec<Post>> {
    sqlx::query_as!(
        Post,
        "SELECT * FROM posts WHERE id > $1 ORDER BY id LIMIT 20",
        after_id.unwrap_or(0)
    )
    .fetch_all(&pool).await
}
```

Client tracks last seen `id`. Continue from there. Constant-time regardless of depth.

---

# Tầng 10: Caching layers

## 10.1 Cache types

```
   ┌────────────────────────────────────────────────────────┐
   │                                                        │
   │  Application cache (in-process)                        │
   │   ┌──────────────────┐                                 │
   │   │ Lru / moka       │  Fast, but per-instance         │
   │   │ in-process       │  Lost on restart                │
   │   └──────────────────┘                                 │
   │            ↑                                           │
   │            │ miss                                      │
   │   ┌──────────────────┐                                 │
   │   │ Redis / Memcached│  Shared across instances        │
   │   │ external         │  Persistent (Redis)             │
   │   └──────────────────┘                                 │
   │            ↑                                           │
   │            │ miss                                      │
   │   ┌──────────────────┐                                 │
   │   │ Database          │  Source of truth                │
   │   │                   │                                │
   │   └──────────────────┘                                 │
   │                                                        │
   └────────────────────────────────────────────────────────┘
```

## 10.2 In-process cache với moka

```toml
[dependencies]
moka = { version = "0.12", features = ["future"] }
```

```rust
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

let user_cache: Arc<Cache<i64, User>> = Arc::new(
    Cache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_secs(60))
        .time_to_idle(Duration::from_secs(120))
        .build()
);

// In handler:
async fn get_user(id: i64, state: &AppState) -> Result<User> {
    if let Some(user) = state.user_cache.get(&id).await {
        return Ok(user);
    }
    let user = state.user_repo.find_by_id(id).await?;
    state.user_cache.insert(id, user.clone()).await;
    Ok(user)
}
```

Per-process. Fast. Lost on restart.

## 10.3 Redis cache

```toml
[dependencies]
redis = { version = "0.27", features = ["tokio-comp"] }
```

```rust
use redis::AsyncCommands;

let mut conn = redis_client.get_async_connection().await?;

// Get
let cached: Option<String> = conn.get(format!("user:{}", user_id)).await?;
if let Some(json) = cached {
    return Ok(serde_json::from_str(&json)?);
}

// Miss — fetch from DB
let user = db_fetch(user_id).await?;

// Set with TTL
let json = serde_json::to_string(&user)?;
conn.set_ex::<_, _, ()>(format!("user:{}", user_id), json, 300).await?;

Ok(user)
```

Shared cache across all app instances.

## 10.4 Cache patterns

### Pattern 1: Cache-aside (lookup, miss, fill)
```rust
fn get(key) {
    if let Some(v) = cache.get(key) { return v; }
    let v = db.get(key);
    cache.set(key, v);
    return v;
}
```

Most common. Simple. Stale data after DB write.

### Pattern 2: Write-through
```rust
fn set(key, value) {
    db.set(key, value);
    cache.set(key, value);
}
```

Cache always fresh after write. But all writes go through cache.

### Pattern 3: Cache invalidation on write
```rust
fn update(key, value) {
    db.set(key, value);
    cache.delete(key);   // invalidate
    // Next read repopulates
}
```

Common pattern. Cache eventually consistent.

## 10.5 Cache stampede

```
   Time t=0: cache expires for popular item
   Time t=0.1: 1000 concurrent requests → all miss → 1000 DB queries!
   
   ⟹ DB overload, cascade failure
```

Solutions:
- **Soft expiration**: refresh in background before hard expire
- **Mutex per key**: only 1 request refills, others wait
- **Probabilistic early expire**: refresh slightly before TTL

`moka` has built-in stampede protection via `get_with()`:

```rust
let user = cache.get_with(id, async {
    db_fetch(id).await.unwrap()   // only 1 request runs this concurrently
}).await;
```

## 10.6 Cache invalidation hard

> "There are only two hard things in Computer Science: cache invalidation and naming things."  
> — Phil Karlton

Strategies:
- **Short TTL**: accept stale, simple
- **Manual invalidate on write**: synchronous, but easy to forget
- **Pub/sub invalidation**: write publishes, all instances clear
- **Versioned keys**: `user:42:v3` — bump version to invalidate all

Choose based on data freshness needs.

## 10.7 What to cache

```
   ✅ Cache:
   • Hot reads (user profile by ID)
   • Computed aggregates (counts, statistics)
   • External API responses
   • Translation / lookup data
   
   ❌ Don't cache:
   • Per-user customized data with low hit rate
   • Sensitive data (PII, secrets) — careful
   • Highly volatile data (real-time prices)
```

---

# Tầng 11: Read replicas & write/read split

## 11.1 Architecture

```
   ┌─────────────┐
   │   Client    │
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐
   │  App        │
   │             │
   │  WRITE ────►│ Postgres PRIMARY (writer)
   │             │       │
   │             │       │ replicates
   │             │       ▼
   │  READ  ────►│ Postgres REPLICA(s) (read-only)
   │             │
   └─────────────┘
```

Primary handles writes, replicates async to replicas. Reads go to replicas (scale read throughput).

## 11.2 Setup in Rust

```rust
pub struct DbContext {
    pub writer: PgPool,        // primary
    pub reader: PgPool,        // replica (or pool of replicas)
}

impl DbContext {
    pub async fn new(write_url: &str, read_url: &str) -> Result<Self> {
        Ok(Self {
            writer: PgPool::connect(write_url).await?,
            reader: PgPool::connect(read_url).await?,
        })
    }
}

// Handlers:
async fn create_user(State(db): State<DbContext>, ...) -> Result<User> {
    sqlx::query_as!(User, "INSERT INTO users ... RETURNING *", ...)
        .fetch_one(&db.writer).await   // write to primary
}

async fn get_user(State(db): State<DbContext>, ...) -> Result<User> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", ...)
        .fetch_one(&db.reader).await   // read from replica
}
```

## 11.3 Replication lag

```
   t=0: app writes user.email to primary
   t=10ms: replica catches up
   
   In window: read from replica = stale!
   
   t=5ms: app reads from replica → OLD email
```

Replicas async — lag from ms to seconds.

Patterns to handle:
- **Read your writes**: read from primary after write (for that user)
- **Tolerate eventual consistency**: most reads OK
- **Sync replication**: only commit when replica confirms (slow)

## 11.4 Routing decision

```rust
enum ReadPreference {
    Primary,        // always read from primary
    Replica,        // read from replica (may be stale)
    PreferReplica,  // try replica, fallback primary
}

async fn get_user(
    db: &DbContext,
    user_id: i64,
    pref: ReadPreference,
) -> Result<User> {
    let pool = match pref {
        ReadPreference::Primary => &db.writer,
        ReadPreference::Replica => &db.reader,
        ReadPreference::PreferReplica => {
            // try replica first, fallback to primary
            &db.reader
        }
    };
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool).await
        .map_err(Into::into)
}
```

After writing to user X, read user X from primary for next few seconds. Tradeoff complexity.

## 11.5 Replica load balancing

Multiple replicas — round-robin or random:

```rust
struct DbContext {
    writer: PgPool,
    readers: Vec<PgPool>,
    next_reader: AtomicUsize,
}

impl DbContext {
    fn reader(&self) -> &PgPool {
        let idx = self.next_reader.fetch_add(1, Ordering::Relaxed);
        &self.readers[idx % self.readers.len()]
    }
}
```

Or use PgBouncer / Pgpool with read-replica routing.

## 11.6 When to add replicas

```
   Symptoms:
   • CPU high on primary
   • Read queries queueing
   • Read throughput hitting limits
   
   Replicas help: read-heavy workload (90% reads, 10% writes)
   Replicas don't help: write-heavy (still bottlenecked on primary)
```

---

# Tầng 12: Sharding — Horizontal scaling

## 12.1 When sharding?

Read replicas scale reads. Sharding scales writes.

Need sharding when:
- Single DB can't handle write load
- Data too large for one DB (TB+)
- Latency requires data colocation
- Multi-region

⚠️ Sharding is **complex**. Avoid until truly necessary (~10-100M users typically).

## 12.2 Shard key choice

Pick column to shard by:
- **Even distribution** — avoid hot shards
- **Locality** — related data on same shard (joins, transactions)
- **Stability** — shard key doesn't change

Examples:
- User ID → all user data on same shard
- Tenant ID (multi-tenant) → tenant on own shard
- Geographic region → user in EU shard, US shard, ...

## 12.3 Hash sharding

```rust
fn shard_for_user(user_id: i64, num_shards: u32) -> u32 {
    let hash = compute_hash(user_id);
    (hash % num_shards as u64) as u32
}

// Connect pool per shard:
struct ShardedDb {
    shards: Vec<PgPool>,
}

impl ShardedDb {
    fn pool_for(&self, user_id: i64) -> &PgPool {
        let idx = shard_for_user(user_id, self.shards.len() as u32);
        &self.shards[idx as usize]
    }
}

async fn get_user(db: &ShardedDb, user_id: i64) -> Result<User> {
    let pool = db.pool_for(user_id);
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool).await
}
```

## 12.4 Range sharding

```
   Shard 1: user_id 1-1,000,000
   Shard 2: user_id 1,000,001-2,000,000
   Shard 3: user_id 2,000,001-3,000,000
   ...
```

Lookup easy. But hot spots (new users always on last shard).

## 12.5 Consistent hashing

```
   Add/remove shard with minimal data movement.
   Hash ring with virtual nodes.
   
   Library: jump-consistent-hash crate
```

For dynamic sharding (clouds with auto-scale).

## 12.6 Cross-shard queries

```
   "Get all users where created_at > yesterday"
   
   Has to query EVERY shard → fan out + merge.
```

Pattern:
```rust
async fn query_all_shards<T>(
    db: &ShardedDb,
    query: impl Fn(&PgPool) -> ...
) -> Result<Vec<T>> {
    let futures: Vec<_> = db.shards.iter().map(|p| query(p)).collect();
    let results = futures::future::join_all(futures).await;
    // merge results
}
```

Slower than single-shard. Avoid cross-shard queries when possible.

## 12.7 Distributed transactions — Don't

Two-phase commit across shards = complex, slow, unreliable.

Patterns:
- **Saga**: sequence of local transactions + compensating actions
- **Event sourcing**: log events, eventually consistent
- **Idempotent operations**: retry-safe

Stay within single shard for transactions when possible.

## 12.8 Practical: avoid sharding if possible

- Vertical scaling (bigger DB instance) gets you far
- Read replicas for read scale
- Caching for hot data
- Move analytics to OLAP separately

Sharding is **complex**. Many teams regret early sharding.

---

# Tầng 13: Multi-tenancy patterns

## 13.1 SaaS multi-tenancy options

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Option 1: Shared schema, tenant_id column               │
   │  ────────────────────────────────────────                │
   │                                                          │
   │  Single DB, single schema, tenant_id in every table.     │
   │                                                          │
   │  CREATE TABLE users (                                    │
   │      id BIGSERIAL,                                       │
   │      tenant_id BIGINT NOT NULL,                          │
   │      email TEXT NOT NULL,                                │
   │      UNIQUE(tenant_id, email)                            │
   │  );                                                      │
   │                                                          │
   │  Every query: WHERE tenant_id = $current_tenant          │
   │                                                          │
   │  ✅ Simple, cheap                                        │
   │  ❌ Tenant data leakage risk                             │
   │  ❌ Hard tenant isolation, backup, deletion              │
   │  ❌ Noisy neighbor (one tenant queries hurt all)         │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Option 2: Schema per tenant                             │
   │  ──────────────────────────                              │
   │                                                          │
   │  Single DB, schema = tenant_<id>                         │
   │  Migrations applied to each schema                       │
   │                                                          │
   │  ✅ Better isolation                                     │
   │  ❌ Schema count limited (~1000s)                        │
   │  ❌ Migration complexity                                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  Option 3: DB per tenant                                 │
   │  ──────────────────────                                  │
   │                                                          │
   │  Each tenant has own DB.                                 │
   │                                                          │
   │  ✅ Best isolation                                       │
   │  ✅ Per-tenant backup, scale, delete                     │
   │  ❌ Many connections (limit DB count)                    │
   │  ❌ Cross-tenant queries hard                            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

Most SaaS: Option 1 with care, Option 2 for premium tier, Option 3 for enterprise.

## 13.2 Row-Level Security (Postgres RLS)

```sql
-- Enable RLS on table
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- Policy: only see your tenant's rows
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = current_setting('app.tenant_id')::BIGINT);
```

App sets tenant_id per session:
```rust
sqlx::query!("SET app.tenant_id = $1", tenant_id)
    .execute(&pool).await?;

// Now ALL subsequent queries filtered by tenant
```

Postgres enforces. Reduces accidental leakage.

## 13.3 Middleware tenant extraction

```rust
async fn tenant_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let tenant_id = req.headers()
        .get("x-tenant-id")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or(AppError::BadRequest("missing tenant".into()))?;
    
    // Inject for handlers
    req.extensions_mut().insert(TenantId(tenant_id));
    
    Ok(next.run(req).await)
}

// Extract in handler:
async fn list_users(Extension(tenant): Extension<TenantId>) {
    // use tenant.0
}
```

## 13.4 Connection pool per tenant

For DB-per-tenant:
```rust
struct TenantPools {
    pools: DashMap<i64, PgPool>,
    template_url: String,
}

impl TenantPools {
    async fn get_or_create(&self, tenant_id: i64) -> Result<PgPool> {
        if let Some(p) = self.pools.get(&tenant_id) {
            return Ok(p.clone());
        }
        let url = format!("{}/tenant_{}", self.template_url, tenant_id);
        let pool = PgPool::connect(&url).await?;
        self.pools.insert(tenant_id, pool.clone());
        Ok(pool)
    }
}
```

DashMap for concurrent access. Avoid creating pool too often.

---

# Tầng 14: Soft deletes, audit logs, time-series

## 14.1 Soft delete

```sql
-- Add deleted_at column
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;

-- "Delete" = update
UPDATE users SET deleted_at = NOW() WHERE id = $1;

-- Queries filter out deleted:
SELECT * FROM users WHERE deleted_at IS NULL AND id = $1;
```

Pros:
- Recover deleted data
- Audit / compliance
- No cascade delete issues

Cons:
- All queries need `WHERE deleted_at IS NULL`
- Larger table size
- Unique constraints with soft delete tricky

```sql
-- Allow re-create deleted email:
CREATE UNIQUE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;
```

Partial unique index — only enforce uniqueness on non-deleted.

## 14.2 Audit log

```sql
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT,
    action TEXT NOT NULL,
    table_name TEXT NOT NULL,
    record_id BIGINT NOT NULL,
    old_data JSONB,
    new_data JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Trigger to auto-log:
CREATE OR REPLACE FUNCTION audit_users_changes()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (user_id, action, table_name, record_id, old_data, new_data)
    VALUES (current_setting('app.user_id')::BIGINT, TG_OP, 'users', 
            COALESCE(NEW.id, OLD.id),
            to_jsonb(OLD), to_jsonb(NEW));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_audit
    AFTER INSERT OR UPDATE OR DELETE ON users
    FOR EACH ROW EXECUTE FUNCTION audit_users_changes();
```

Postgres trigger auto-logs every change.

Alternative: app-level logging on every mutation.

## 14.3 Time-series data

For events (clicks, sensor readings):
```sql
CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    user_id BIGINT,
    event_type TEXT,
    data JSONB
);

CREATE INDEX idx_events_timestamp ON events(timestamp DESC);
```

For large volume:
- **Partitioning**: split by month
- **TimescaleDB**: extension for time-series
- **ClickHouse**: separate OLAP DB

## 14.4 Partitioning

```sql
-- Postgres declarative partitioning
CREATE TABLE events (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL,
    data JSONB
) PARTITION BY RANGE (timestamp);

CREATE TABLE events_2024_05 PARTITION OF events
    FOR VALUES FROM ('2024-05-01') TO ('2024-06-01');

CREATE TABLE events_2024_06 PARTITION OF events
    FOR VALUES FROM ('2024-06-01') TO ('2024-07-01');
```

Drop old partitions = drop old data fast.

```sql
DROP TABLE events_2024_05;   -- delete May data instantly
```

vs `DELETE FROM events WHERE timestamp < ...` → slow + bloat.

## 14.5 OLTP vs OLAP separation

```
   OLTP (transactional)              OLAP (analytical)
   ────────────────────              ─────────────────
   Postgres                          ClickHouse / BigQuery
   Many small queries                Few big queries
   Indexed lookups                   Full table scans + aggregates
   ms latency                        seconds latency OK
   
   Pattern: replicate OLTP → OLAP for analytics
   (Debezium, Kafka, dbt)
```

Don't run BI queries on production OLTP DB. Replicate to OLAP system.

---

# Tầng 15: Testing strategies

## 15.1 Test DB approaches

```
   ┌────────────────────────────────────────────────────────┐
   │ 1. In-memory SQLite                                    │
   │    Fast, but different from prod Postgres              │
   │                                                        │
   │ 2. Embedded postgres (test-postgres-init)              │
   │    Real Postgres, started per test                     │
   │                                                        │
   │ 3. Test DB per test (sqlx::test)                       │
   │    Each test gets fresh DB, auto-cleanup               │
   │    BEST for sqlx                                       │
   │                                                        │
   │ 4. Shared DB + transaction rollback                    │
   │    Fast, but tests can leak state                      │
   │                                                        │
   │ 5. Docker compose with real Postgres                   │
   │    For integration/e2e, slowest                        │
   └────────────────────────────────────────────────────────┘
```

## 15.2 sqlx::test pattern

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_create_user(pool: PgPool) {
    let user = create_user(&pool, "test@test.com").await.unwrap();
    assert_eq!(user.email, "test@test.com");
    
    let fetched = find_by_id(&pool, user.id).await.unwrap();
    assert_eq!(fetched.email, "test@test.com");
}
```

Each test:
1. Create fresh DB
2. Run migrations
3. Execute test
4. Drop DB

Slow (couple seconds per test). Use for repo-level tests.

## 15.3 Testing transactions

```rust
#[sqlx::test]
async fn test_transfer_atomicity(pool: PgPool) {
    setup_two_users(&pool).await;
    
    // Simulate failure mid-transaction
    let result = transfer_with_fail(&pool, 1, 2, 100).await;
    assert!(result.is_err());
    
    // Verify both balances unchanged (rollback worked)
    let user_1 = get_balance(&pool, 1).await.unwrap();
    let user_2 = get_balance(&pool, 2).await.unwrap();
    assert_eq!(user_1, INITIAL);
    assert_eq!(user_2, INITIAL);
}
```

## 15.4 Mocking DB

```rust
#[automock]
#[async_trait]
trait UserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<User>>;
    async fn create(&self, email: &str) -> Result<User>;
}

#[tokio::test]
async fn test_service_logic() {
    let mut mock = MockUserRepository::new();
    mock.expect_find_by_id()
        .with(eq(42))
        .returning(|_| Box::pin(async { Ok(Some(User { id: 42, ... })) }));
    
    let svc = UserService::new(Arc::new(mock));
    let result = svc.get_user(42).await.unwrap();
    // ...
}
```

Mock DB layer for **business logic tests**. Use real DB for **repo tests**.

## 15.5 Containerized tests (testcontainers)

```rust
use testcontainers::*;

#[tokio::test]
async fn test_with_real_postgres() {
    let docker = clients::Cli::default();
    let pg = docker.run(images::postgres::Postgres::default());
    let port = pg.get_host_port_ipv4(5432);
    
    let pool = PgPool::connect(&format!("postgres://postgres:postgres@localhost:{}/postgres", port))
        .await.unwrap();
    
    // ... real-world test ...
}
```

Spins Postgres container. Slow but fully realistic.

---

# Tầng 16: Common pitfalls & antipatterns

## 16.1 ❌ N+1 queries

Covered in Tầng 9. The classic antipattern.

## 16.2 ❌ Forgetting WHERE on update/delete

```sql
-- ⚠️ Catastrophic
UPDATE users SET email = 'x';     -- updates ALL users!
DELETE FROM users;                 -- deletes ALL users!
```

In production code, defense:
```rust
async fn update_user(pool: &PgPool, id: i64, email: &str) -> Result<()> {
    let result = sqlx::query!(
        "UPDATE users SET email = $1 WHERE id = $2",
        email, id
    ).execute(pool).await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::UserNotFound);
    }
    Ok(())
}
```

Check rows affected. Sanity check.

## 16.3 ❌ Long-held transactions

```rust
let mut tx = pool.begin().await?;
sqlx::query!("UPDATE users ...").execute(&mut *tx).await?;

// Long external call!
let result = call_external_api().await?;

sqlx::query!("INSERT INTO ...").execute(&mut *tx).await?;
tx.commit().await?;
```

Transaction holds DB lock during 10s API call → pool exhaustion, blocking writers.

Fix: minimize tx scope. Do external calls **outside** tx.

## 16.4 ❌ SELECT * in production

```sql
SELECT * FROM users WHERE id = $1;
```

Issues:
- Schema change adds column → unexpected data
- Bandwidth waste (large columns)
- Indexes can't cover all columns

✅ Explicit columns:
```sql
SELECT id, email, created_at FROM users WHERE id = $1;
```

## 16.5 ❌ Storing JSON when relational works

```sql
-- ❌ Email list as JSON
CREATE TABLE users (
    id BIGSERIAL,
    emails JSONB    -- array of email strings
);

-- Query: "find users with email x" → JSON scan, no index help
```

vs:
```sql
CREATE TABLE user_emails (
    user_id BIGINT,
    email TEXT,
    PRIMARY KEY (user_id, email)
);

CREATE INDEX idx_user_emails_email ON user_emails(email);
-- Now find users by email is indexed
```

JSON for true semi-structured (config, attributes). Relational for things you query/filter.

## 16.6 ❌ Missing connection pool monitoring

```
   Symptoms hard to detect:
   • Latency spikes
   • Timeouts under load
   
   Always monitor:
   • db_pool_size
   • db_pool_used
   • db_pool_idle
   • Queue wait time
```

Set alerts when pool > 80% used.

## 16.7 ❌ No retry / circuit breaker

```rust
// Single attempt — fails on transient DB hiccup
let user = sqlx::query!("...").fetch_one(&pool).await?;
```

For critical operations, retry:
```rust
async fn with_retry<F, Fut, T>(mut f: F) -> Result<T>
where F: FnMut() -> Fut, Fut: Future<Output = Result<T>> {
    for attempt in 0..3 {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if is_transient(&e) && attempt < 2 => {
                tokio::time::sleep(Duration::from_millis(100 << attempt)).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## 16.8 ❌ Not using prepared statements

sqlx uses prepared statements automatically. But raw `query()` without macros may not.

Prepared statements:
- Parse once, execute many
- SQL injection safe
- Faster (no parse overhead)

`query!()` / `query_as!()` always prepared.

## 16.9 ❌ Manual SQL injection prone

```rust
let q = format!("SELECT * FROM users WHERE email = '{}'", user_input);
sqlx::query(&q).fetch_one(&pool).await;
// ⚠️ SQL injection!
```

Always use parameterized:
```rust
sqlx::query!("SELECT * FROM users WHERE email = $1", user_input)
```

## 16.10 ❌ Forgetting indexes

Profile production queries. `pg_stat_statements` shows slow ones. EXPLAIN them. Add indexes.

---

# Tổng kết — 12 nguyên tắc senior

```
┌──────────────────────────────────────────────────────────────────┐
│ 1. sqlx for most projects. Compile-time SQL safety.              │
│                                                                  │
│ 2. Connection pool: size = throughput × query_time. Monitor.     │
│                                                                  │
│ 3. Transactions short. Never call external API inside.           │
│                                                                  │
│ 4. Default: READ COMMITTED. SERIALIZABLE only when needed.       │
│                                                                  │
│ 5. Migrations: reversible if possible. Multi-step for big changes.│
│                                                                  │
│ 6. ALWAYS EXPLAIN slow queries. Add indexes accordingly.         │
│                                                                  │
│ 7. N+1: use JOIN, ANY(), or JSON_AGG. Never loop queries.        │
│                                                                  │
│ 8. Cache hot data. Invalidate carefully.                         │
│                                                                  │
│ 9. Read replicas for read scale. Mind replication lag.           │
│                                                                  │
│ 10. Sharding LAST resort. Vertical + replicas + cache first.     │
│                                                                  │
│ 11. Soft delete + audit log for compliance/recovery.             │
│                                                                  │
│ 12. Test with real DB (sqlx::test or testcontainers).            │
└──────────────────────────────────────────────────────────────────┘
```

---

# Database toolkit

| Tool / Crate | Purpose |
|--------------|---------|
| `sqlx` | Compile-time SQL, multi-DB |
| `sea-orm` | High-level ORM |
| `diesel` | Mature DSL ORM (sync) |
| `tokio-postgres` | Low-level Postgres |
| `redis` | Redis client |
| `mongodb` | MongoDB driver |
| `pgbouncer` | Transaction pooler |
| `pg_stat_statements` | Query stats extension |
| `pgtune` | Postgres config tuner |
| `moka` | In-memory cache |
| `testcontainers` | Real DB in tests |
| `bb8` / `deadpool` | Generic connection pools |
| `pgrx` | Postgres extensions in Rust |
| `cornucopia` | Alternative compile-time SQL |

---

# Lộ trình — Đã hoàn thành bộ tài liệu 18 chương

Bạn đã hoàn thành toàn bộ:

```
1.  memory-model
2.  ownership-borrowing
3.  trait
4.  generic
5.  closure
6.  async
7.  error-handling
8.  macros
9.  smart-pointers
10. lifetime
11. performance
12. observability
13. iterator
14. unsafe-rust
15. testing
16. embedded-rust
17. axum-project
18. database              ← MỚI - CHƯƠNG CUỐI
```

## Tổng tài liệu

- **18 chủ đề** × 2 files (theory + visual) = **36 files**
- **~63,000 dòng** Markdown
- **~2 MB** content

Đủ để build:
- 🌐 **Production web services** (axum)
- 🗄️ **Database-heavy apps** (Postgres + sqlx)
- 🔌 **Embedded systems** (no_std + embassy)
- 🚀 **High-performance services** (profiling + optimization)
- 🧪 **Well-tested codebases** (unit + integration + e2e + property)
- 🔍 **Observable production code** (tracing + metrics + traces)
- ⚙️ **System programming** (unsafe + FFI + atomics)

## Bước tiếp theo

Áp dụng vào project thực tế:

1. **Build a side project** — small but production-quality
2. **Contribute to open source** — sqlx, axum, tokio, ...
3. **Read code** of mature Rust projects: `rustc`, `tokio`, `axum`, `sqlx`, `serde`
4. **Follow Rust news**: This Week in Rust, Rust blog
5. **Conferences**: RustConf, EuroRust, RustFest

🦀 **Chúc bạn senior Rust journey thành công!**
