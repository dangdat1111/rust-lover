# Database — Minh Hoạ Trực Quan

> Companion visual cho [database.md](./database.md). Đọc song song.

---

## 1. Bức tranh lớn — Database Universe

```
                          DATABASE TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   ┌──────────┐  ┌──────────┐  ┌──────────┐             │
       │   │  sqlx    │  │ sea-orm  │  │ diesel   │             │
       │   │ raw SQL  │  │ ORM      │  │ DSL      │             │
       │   │ compile  │  │ active   │  │ compile  │             │
       │   │ checked  │  │ record   │  │ DSL      │             │
       │   └──────────┘  └──────────┘  └──────────┘             │
       │                                                        │
       │   CONCERNS:                                            │
       │   ┌─────────────────────────────────────────────────┐  │
       │   │ Connection pool  │ Transactions   │ Migrations  │  │
       │   │ Query optimize   │ Indexes        │ Caching     │  │
       │   │ N+1 prevention   │ Read replicas  │ Sharding    │  │
       │   │ Multi-tenancy    │ Audit logs     │ Testing     │  │
       │   └─────────────────────────────────────────────────┘  │
       │                                                        │
       │   Default choice 2024+: sqlx + PostgreSQL              │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Library landscape

```
   ┌─────────────────────────────────────────────────────────────┐
   │                                                             │
   │  Library          Style           Compile  Async  Score    │
   │  ─────────       ──────────      ─────────────────────────  │
   │  sqlx             Raw SQL         ✅       ✅     ★★★★★    │
   │  sea-orm          ORM             ❌       ✅     ★★★★     │
   │  diesel           DSL             ✅       sync   ★★★★     │
   │                                    (-async crate)            │
   │  tokio-postgres   Low-level       ❌       ✅     ★★★      │
   │  rusqlite         SQLite          ❌       sync   ★★★      │
   │  mongodb          NoSQL           ❌       ✅     ★★★      │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
   
   
   Decision tree:
   ──────────────
   
                Choose DB library
                       │
              ┌────────┴────────┐
            SQL?              NoSQL?
              │                 │
              │             ┌───┴───┐
              │           Mongo?   Redis?
              │             │       │
              │          mongodb  redis
              │
        ┌─────┴─────┐
       Direct      ORM
       control?    style?
        │           │
       sqlx     ┌───┴───┐
              sea-orm  diesel
              (async)  (mature)
```

---

## 3. Driver vs Library stack

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   YOUR APP                                               │
   │       │                                                  │
   │       │ uses                                             │
   │       ▼                                                  │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  LIBRARY (sqlx / sea-orm / diesel)           │       │
   │   │   • Query macro / builder                    │       │
   │   │   • Connection pool                          │       │
   │   │   • Migrations                               │       │
   │   │   • Type mapping (Rust ↔ SQL)                │       │
   │   └──────────────────────────────────────────────┘       │
   │       │                                                  │
   │       │ uses                                             │
   │       ▼                                                  │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  DRIVER (low-level)                          │       │
   │   │   • Wire protocol                            │       │
   │   │   • TCP/TLS                                  │       │
   │   │   • Binary format                            │       │
   │   │   • PostgreSQL: tokio-postgres / sqlx-pg     │       │
   │   └──────────────────────────────────────────────┘       │
   │       │                                                  │
   │       ▼ TCP/TLS                                          │
   │   ┌──────────────────────────────────────────────┐       │
   │   │  DATABASE SERVER (Postgres / MySQL / SQLite) │       │
   │   └──────────────────────────────────────────────┘       │
   └──────────────────────────────────────────────────────────┘
```

---

## 4. sqlx compile-time validation

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Compile time (cargo build):                            │
   │   ─────────────────────────                              │
   │                                                          │
   │   sqlx::query_as!(User,                                  │
   │       "SELECT id, email FROM users WHERE id = $1",       │
   │       user_id                                            │
   │   )                                                      │
   │            │                                             │
   │            ▼  macro processes                            │
   │   ┌─────────────────────────────────────┐                │
   │   │ Connect to DATABASE_URL              │               │
   │   │ Run "PREPARE" on query               │               │
   │   │ Check:                               │               │
   │   │   • SQL syntax valid                 │               │
   │   │   • columns exist                    │               │
   │   │   • types match User struct          │               │
   │   │   • $1 type matches user_id          │               │
   │   │ Generate typed code                  │               │
   │   └─────────────────────────────────────┘                │
   │            │                                             │
   │            ▼                                             │
   │   If error → COMPILE FAIL                                │
   │     error: column "emaiil" does not exist                │
   │      --> src/main.rs:42                                  │
   │                                                          │
   │   ╔══════════════════════════════════════╗               │
   │   ║ SQL typo caught BEFORE deploy!       ║               │
   │   ╚══════════════════════════════════════╝               │
   │                                                          │
   │   Offline mode (CI without DB):                          │
   │   ─────────────────────────                              │
   │   cargo sqlx prepare    → .sqlx/*.json                   │
   │   git add .sqlx/                                         │
   │   CI: SQLX_OFFLINE=true cargo build                      │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 5. Connection Pool architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   APP                                                    │
   │   ┌────────────────────────────────────────┐             │
   │   │ Handler 1 ───┐                         │             │
   │   │ Handler 2 ───┼───►  PgPool             │             │
   │   │ Handler 3 ───┘   (max_connections=20)  │             │
   │   │ ...                                    │             │
   │   └─────────────┬──────────────────────────┘             │
   │                 │                                         │
   │                 ▼                                         │
   │   ┌─────────────────────────────────────────┐            │
   │   │ Pool (Arc internally):                  │            │
   │   │                                         │            │
   │   │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐    │            │
   │   │  │conn1 │ │conn2 │ │conn3 │ │conn4 │    │            │
   │   │  │ idle │ │busy  │ │busy  │ │ idle │    │            │
   │   │  └──────┘ └──────┘ └──────┘ └──────┘    │            │
   │   │  ...                                     │            │
   │   └─────────────────────────────────────────┘            │
   │                 │                                         │
   │                 │ TCP                                     │
   │                 ▼                                         │
   │   ┌─────────────────────────────────────────┐            │
   │   │ Postgres (max_connections = 100)        │            │
   │   └─────────────────────────────────────────┘            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Without pool:                With pool:
   ─────────────                ──────────
   
   Each query:                  Pre-established conns:
   • TCP connect (10ms)         • Borrow from pool (~1µs)
   • TLS handshake (10-50ms)    • Use, return (~1µs)
   • Auth (5ms)                 
   • Query (5ms)                Net: just query time
   • Disconnect                 
   = 70-100ms overhead          = ~5ms total
   
   ⟹ Pool reduces 10-20x overhead
   
   
   Pool sizing formula:
   ────────────────────
   
   pool_size = (target_throughput / 1000ms) × avg_query_ms
   
   Example:
     1000 req/s × 5ms/query = 5 conns minimum
     + buffer = 10-20 conns
   
   📌 Too small → queue, latency spike
   📌 Too large → DB overload
```

---

## 6. PgBouncer for scale

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  WITHOUT PgBouncer — direct app→DB:                      │
   │  ─────────────────────────────────                       │
   │                                                          │
   │   App1 (20 conns) ──┐                                    │
   │   App2 (20 conns) ──┼──► Postgres (max_connections=100)  │
   │   App3 (20 conns) ──┤    20+20+20+20+20 = 100            │
   │   App4 (20 conns) ──┤    LIMITED to 5 apps               │
   │   App5 (20 conns) ──┘                                    │
   │                                                          │
   │  ⟹ Can't scale apps independently                        │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  WITH PgBouncer (transaction pooler):                    │
   │  ────────────────────────────────                        │
   │                                                          │
   │   App1 (50 conns) ──┐                                    │
   │   App2 (50 conns) ──┤                                    │
   │   App3 (50 conns) ──┤    ┌───────────────┐               │
   │   App4 (50 conns) ──┼───►│  PgBouncer    │──► Postgres   │
   │   App5 (50 conns) ──┤    │ (multiplexes  │   (100 conns) │
   │   ...               ──┘    │  per txn)    │               │
   │                            └───────────────┘               │
   │   100s of app conns        100 real conns                │
   │                                                          │
   │  ⟹ Scale apps without exhausting Postgres                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 7. Transactions ACID

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ATOMICITY                                              │
   │   ─────────                                              │
   │   All steps succeed OR all roll back. No partial state.  │
   │                                                          │
   │   BEGIN                                                  │
   │     UPDATE accounts SET balance = balance - 100 WHERE id=1│
   │     UPDATE accounts SET balance = balance + 100 WHERE id=2│
   │     -- crash here: BOTH rolled back. ✅                  │
   │   COMMIT                                                 │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   CONSISTENCY                                            │
   │   ─────────                                              │
   │   DB stays valid. Constraints enforced.                  │
   │                                                          │
   │   Example: UNIQUE constraint, FK, CHECK                  │
   │   Insert violating → REJECT                              │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ISOLATION                                              │
   │   ─────────                                              │
   │   Concurrent txns don't interfere.                       │
   │   See "Isolation Levels" below                           │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   DURABILITY                                             │
   │   ──────────                                             │
   │   Committed data survives crash.                         │
   │   WAL (Write-Ahead Log) + fsync                          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. Isolation levels — anomaly matrix

```
   ┌────────────────────┬──────────┬──────────┬──────────┬──────────┐
   │ Isolation Level    │ Dirty    │ Non-rep. │ Phantom  │ Lost     │
   │                    │ Read     │ Read     │ Read     │ Update   │
   ├────────────────────┼──────────┼──────────┼──────────┼──────────┤
   │ READ UNCOMMITTED   │ Possible │ Possible │ Possible │ Possible │
   │ (Postgres: same    │          │          │          │          │
   │  as READ COMMITTED)│          │          │          │          │
   ├────────────────────┼──────────┼──────────┼──────────┼──────────┤
   │ READ COMMITTED     │ ❌       │ Possible │ Possible │ Possible │
   │ (Postgres default) │          │          │          │          │
   ├────────────────────┼──────────┼──────────┼──────────┼──────────┤
   │ REPEATABLE READ    │ ❌       │ ❌       │ Possible │ ❌       │
   │ (Postgres: also no │          │          │ (theory) │          │
   │  phantom in MVCC)  │          │          │          │          │
   ├────────────────────┼──────────┼──────────┼──────────┼──────────┤
   │ SERIALIZABLE       │ ❌       │ ❌       │ ❌       │ ❌       │
   │ (slowest, retries  │          │          │          │          │
   │  needed)           │          │          │          │          │
   └────────────────────┴──────────┴──────────┴──────────┴──────────┘
   
   
   Anomaly examples:
   ─────────────────
   
   DIRTY READ:
       T1: UPDATE x = 5         (uncommitted)
       T2: SELECT x → 5
       T1: ROLLBACK
       T2: now has stale data
   
   NON-REPEATABLE READ:
       T1: SELECT balance → 100
       T2: UPDATE balance=200; COMMIT
       T1: SELECT balance → 200 (changed!)
   
   PHANTOM READ:
       T1: SELECT COUNT WHERE x > 10 → 5
       T2: INSERT row x=15; COMMIT
       T1: SELECT COUNT WHERE x > 10 → 6 (new row!)
   
   
   When to use:
   ────────────
   
   READ COMMITTED:   most cases (default)
   REPEATABLE READ:  financial reports, consistent reads
   SERIALIZABLE:     when correctness > speed
                     (retry on conflict)
```

---

## 9. SELECT FOR UPDATE — Pessimistic lock

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   Without FOR UPDATE (race condition):                   │
   │   ─────────────────────────────────                      │
   │                                                          │
   │   T1: SELECT balance FROM acc WHERE id=1  → 100          │
   │   T2: SELECT balance FROM acc WHERE id=1  → 100          │
   │   T1: UPDATE acc SET balance=100-30 WHERE id=1  → 70     │
   │   T2: UPDATE acc SET balance=100-50 WHERE id=1  → 50     │
   │                                                          │
   │   Expected: 100 - 30 - 50 = 20                           │
   │   Actual:   50 (T1's update lost!)                       │
   │                                                          │
   │   ❌ LOST UPDATE                                         │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   With FOR UPDATE (lock):                                │
   │   ────────────────                                       │
   │                                                          │
   │   T1: BEGIN                                              │
   │   T1: SELECT balance FROM acc WHERE id=1 FOR UPDATE       │
   │       → 100 (lock acquired)                              │
   │                                                          │
   │   T2: BEGIN                                              │
   │   T2: SELECT balance FROM acc WHERE id=1 FOR UPDATE       │
   │       → BLOCKED waiting for T1                           │
   │                                                          │
   │   T1: UPDATE acc SET balance=70 WHERE id=1               │
   │   T1: COMMIT  ✅                                         │
   │                                                          │
   │   T2: continues:                                         │
   │   T2: → 70 (sees updated value)                          │
   │   T2: UPDATE acc SET balance=20 WHERE id=1               │
   │   T2: COMMIT  ✅                                         │
   │                                                          │
   │   Result: 100 → 70 → 20 ✅                               │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   ⚠️ Deadlock risk:
   ──────────────────
   T1: LOCK row A, then try LOCK row B
   T2: LOCK row B, then try LOCK row A
   → Both blocked forever
   
   Postgres detects + kills one with deadlock error.
   App must retry.
   
   Mitigation: ALWAYS lock in same order across all txns.
```

---

## 10. Migrations strategy

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   MIGRATION LIFECYCLE                                    │
   │                                                          │
   │   v0 (initial DB)                                        │
   │      │                                                   │
   │      ▼ 001_create_users.sql                              │
   │   v1                                                     │
   │      │                                                   │
   │      ▼ 002_add_email_index.sql                           │
   │   v2                                                     │
   │      │                                                   │
   │      ▼ 003_create_orders.sql                             │
   │   v3                                                     │
   │                                                          │
   │   Tracking: _sqlx_migrations table (idempotent)          │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Safe migration patterns:
   ────────────────────────
   
   ┌────────────────────────────────────────────────────────────┐
   │ ✅ Add nullable column (safe):                             │
   │    ALTER TABLE users ADD COLUMN phone TEXT;                │
   │    Old app reads: ignores phone ✅                         │
   │                                                            │
   │ ✅ Add NOT NULL column (multi-step):                       │
   │    1. ADD COLUMN (nullable)                                │
   │    2. Deploy app writing phone                             │
   │    3. UPDATE rows backfilling                              │
   │    4. ALTER COLUMN SET NOT NULL                            │
   │                                                            │
   │ ⚠️ Rename column (risky):                                  │
   │    1. ADD new column                                       │
   │    2. Deploy app reading BOTH old + new                    │
   │    3. UPDATE: copy old → new                               │
   │    4. Deploy app reading only new                          │
   │    5. DROP old column                                      │
   │                                                            │
   │ ⚠️ Drop column (safe with care):                           │
   │    1. Deploy app NOT reading column                        │
   │    2. Wait for deploys to settle                           │
   │    3. ALTER TABLE DROP COLUMN                              │
   └────────────────────────────────────────────────────────────┘
   
   
   Online DDL (no downtime):
   ─────────────────────────
   
   • CREATE INDEX CONCURRENTLY (non-blocking)
   • Postgres 11+: ADD COLUMN with DEFAULT (instant, metadata only)
   • Use pg_repack for table rewrites
```

---

## 11. EXPLAIN ANALYZE

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'x';  │
   │                                                          │
   │  BEFORE index:                                           │
   │  ─────────────                                           │
   │   Seq Scan on users (cost=0.00..18.50 rows=1 width=88)   │
   │                     (actual time=0.012..0.150 rows=1)    │
   │     Filter: (email = 'x'::text)                          │
   │     Rows Removed by Filter: 999                          │
   │   Planning Time: 0.05 ms                                 │
   │   Execution Time: 0.18 ms                                │
   │     │                                                    │
   │     │ ❌ Full table scan, removes 999 rows               │
   │     ▼                                                    │
   │                                                          │
   │  AFTER: CREATE INDEX idx_users_email ON users(email);   │
   │  ────────────                                            │
   │   Index Scan using idx_users_email on users              │
   │     (cost=0.28..8.29 rows=1)                             │
   │     (actual time=0.020..0.022 rows=1)                    │
   │     Index Cond: (email = 'x'::text)                      │
   │   Planning Time: 0.10 ms                                 │
   │   Execution Time: 0.04 ms                                │
   │     │                                                    │
   │     │ ✅ Index lookup, 4x faster                         │
   │     ▼                                                    │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Cost vs actual time:
   ────────────────────
   
   cost=0.00..18.50      ← planner's estimate (lower=better)
   rows=1                ← estimated rows
   actual time=0.012     ← real measurement
   Rows Removed by Filter: 999  ← inefficiency!
   
   📌 Big "Rows Removed" → missing index
   📌 Seq Scan on big table → usually wants index
```

---

## 12. N+1 problem visualization

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ❌ N+1 PATTERN:                                        │
   │                                                          │
   │   Query 1: SELECT * FROM users WHERE active = true       │
   │           → returns 100 users                            │
   │                                                          │
   │   For each user (100 times):                             │
   │     Query 2: SELECT * FROM orders WHERE user_id = $1     │
   │                                                          │
   │   Total: 1 + 100 = 101 queries                           │
   │   At 5ms each: 101 × 5 = 505ms                           │
   │                                                          │
   │   Network overhead dominates                             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ Solution 1: JOIN                                    │
   │                                                          │
   │   1 query:                                               │
   │     SELECT u.*, o.*                                      │
   │     FROM users u                                         │
   │     LEFT JOIN orders o ON o.user_id = u.id               │
   │     WHERE u.active = true                                │
   │                                                          │
   │   Need: group by user_id in app code                     │
   │   Total: 1 query, ~10ms                                  │
   │   Speedup: 50x                                            │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ Solution 2: ANY (2 queries)                         │
   │                                                          │
   │   Query 1: SELECT * FROM users WHERE active = true       │
   │   Query 2: SELECT * FROM orders WHERE user_id = ANY($1)  │
   │              with $1 = [list of user IDs]                │
   │                                                          │
   │   2 queries total                                        │
   │   Group in app: orders.into_group_map_by(|o| o.user_id)  │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   ✅ Solution 3: JSON_AGG (1 query)                      │
   │                                                          │
   │   SELECT u.id, u.email,                                  │
   │     COALESCE(JSON_AGG(o.*), '[]'::JSON) as orders        │
   │   FROM users u                                           │
   │   LEFT JOIN orders o ON o.user_id = u.id                 │
   │   GROUP BY u.id, u.email;                                │
   │                                                          │
   │   Postgres aggregates JSON. App deserializes.            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 13. Index types reference

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Index Type    │ Use Case                                │
   │  ────────────  │ ────────                                │
   │                                                          │
   │  B-tree         │ DEFAULT, most common                   │
   │   (CREATE       │ WHERE = / < / > / BETWEEN              │
   │   INDEX)        │ ORDER BY                               │
   │                                                          │
   │  Multi-column   │ WHERE a = ? AND b = ?                  │
   │   (a, b, c)     │ Leftmost prefix rule                   │
   │                                                          │
   │  Partial         │ WHERE matching specific condition     │
   │   WHERE active=t│ Smaller, faster, only relevant rows    │
   │                                                          │
   │  Expression     │ Index on computed value                │
   │   LOWER(email)  │ e.g., case-insensitive lookups          │
   │                                                          │
   │  Unique         │ Enforce uniqueness + lookup             │
   │   UNIQUE INDEX  │                                         │
   │                                                          │
   │  GIN            │ Arrays, JSONB, full-text                │
   │   USING GIN     │                                         │
   │                                                          │
   │  GiST           │ Geometric, ranges                       │
   │                                                          │
   │  BRIN           │ Very large sequential data              │
   │                  (time-series, log tables)                │
   │                                                          │
   │  Covering       │ Include columns to avoid table lookup   │
   │   INCLUDE       │                                         │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Multi-column index ordering matters:
   ────────────────────────────────────
   
   CREATE INDEX idx ON orders(user_id, status);
   
   Helps:
   ✅ WHERE user_id = ?
   ✅ WHERE user_id = ? AND status = ?
   
   Doesn't help:
   ❌ WHERE status = ?         (can't use index)
   ❌ WHERE created_at = ?     (column not in index)
   
   Rule: index (a, b, c) helps queries on
         a, (a,b), (a,b,c) — leftmost prefix
```

---

## 14. Caching layers

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │  Request flow:                                           │
   │                                                          │
   │   App                                                    │
   │    │                                                     │
   │    ▼                                                     │
   │   ┌──────────────────┐  cache hit (ns)                   │
   │   │ In-process moka   │ ─────────────► return            │
   │   │ (per instance)    │                                  │
   │   └──────┬───────────┘                                   │
   │          │ miss                                          │
   │          ▼                                               │
   │   ┌──────────────────┐  cache hit (ms — network)         │
   │   │ Redis (shared)    │ ─────────────► return            │
   │   │                   │                                  │
   │   └──────┬───────────┘                                   │
   │          │ miss                                          │
   │          ▼                                               │
   │   ┌──────────────────┐  ms — DB query                    │
   │   │ Postgres          │ ─────────────► return            │
   │   │ (source of truth) │   + populate caches              │
   │   └──────────────────┘                                   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Cache patterns:
   ───────────────
   
   ┌─────────────────────────────────────────────────────────┐
   │ CACHE-ASIDE (most common):                              │
   │ ──────────                                               │
   │   if cache.has(k): return cache.get(k)                  │
   │   v = db.get(k)                                         │
   │   cache.set(k, v, ttl=60s)                              │
   │   return v                                              │
   │                                                          │
   │ WRITE-THROUGH:                                          │
   │ ──────────────                                          │
   │   db.set(k, v)                                          │
   │   cache.set(k, v)                                       │
   │                                                          │
   │ INVALIDATION on write:                                  │
   │ ──────────────────                                      │
   │   db.set(k, v)                                          │
   │   cache.delete(k)   # next read repopulates             │
   │                                                          │
   └─────────────────────────────────────────────────────────┘
   
   
   Cache stampede:
   ───────────────
   
   t=0: cache expires for popular item
   t=0.1: 1000 concurrent requests → all miss
         → 1000 DB queries (DB overload!)
   
   Fix:
   ────
   • moka.get_with(): only 1 request rebuilds, others wait
   • Mutex per key
   • Soft expiration (refresh ahead of TTL)
```

---

## 15. Read replicas architecture

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   ┌──────────┐                                           │
   │   │  Client  │                                           │
   │   └────┬─────┘                                           │
   │        │                                                 │
   │        ▼                                                 │
   │   ┌──────────────────────────┐                           │
   │   │  Application              │                          │
   │   │                           │                          │
   │   │  ┌──────────┐ ┌──────────┐│                          │
   │   │  │ WRITE    │ │ READ     ││                          │
   │   │  │ requests │ │ requests ││                          │
   │   │  └────┬─────┘ └────┬─────┘│                          │
   │   └───────┼────────────┼──────┘                          │
   │           │            │                                 │
   │           ▼            ▼                                 │
   │   ┌──────────┐  ┌──────────────────────┐                 │
   │   │ Postgres │  │ Postgres Replica(s)  │                 │
   │   │ PRIMARY  │──│ (read-only)          │                 │
   │   │ (writer) │  │   replica 1          │                 │
   │   │          │  │   replica 2          │                 │
   │   └──────────┘  │   replica 3          │                 │
   │        ▲        └──────────────────────┘                 │
   │        │              ▲                                  │
   │        │              │ async replication                │
   │        └──────────────┘ (WAL streaming)                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Replication lag:
   ────────────────
   
   t=0: write to primary
   t=10ms: replica catches up
   
   Window: read from replica = stale!
   
   t=5ms: app reads replica → OLD value
   
   
   Mitigation patterns:
   ────────────────────
   
   • READ AFTER WRITE: read from primary for X seconds after user wrote
   • Tolerate eventual consistency for most reads
   • Synchronous replication: commit only when replica confirmed (slow)
   
   
   When replicas help:
   ───────────────────
   
   ✅ Read-heavy (90% read, 10% write) — big throughput win
   ❌ Write-heavy — bottleneck still primary
```

---

## 16. Sharding architectures

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   HASH SHARDING                                          │
   │                                                          │
   │   shard = hash(user_id) % num_shards                     │
   │                                                          │
   │   ┌──────────┐                                           │
   │   │  Client  │                                           │
   │   └────┬─────┘                                           │
   │        │                                                 │
   │        ▼                                                 │
   │   ┌──────────────────┐                                   │
   │   │  App with router  │                                  │
   │   │  pool_for(user_id)│                                  │
   │   └────┬──────────────┘                                  │
   │        │                                                 │
   │   ┌────┼───────────┬───────────┬───────────┐             │
   │   ▼    ▼           ▼           ▼           ▼             │
   │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐             │
   │  │Shard 0 │ │Shard 1 │ │Shard 2 │ │Shard 3 │             │
   │  │ users  │ │ users  │ │ users  │ │ users  │             │
   │  │ 0,4,8..│ │ 1,5,9..│ │ 2,6,10.│ │ 3,7,11.│             │
   │  └────────┘ └────────┘ └────────┘ └────────┘             │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   PROS:                                                  │
   │   ✅ Linear scale writes                                 │
   │   ✅ Smaller per-shard data                              │
   │                                                          │
   │   CONS:                                                  │
   │   ❌ Cross-shard query = fan-out (slow)                  │
   │   ❌ Distributed transactions hard                       │
   │   ❌ Resharding painful                                  │
   │   ❌ Complex routing logic                               │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   WHEN to shard:                                         │
   │   • Single DB hit write throughput limit                 │
   │   • Data > TB                                            │
   │   • > 10M-100M users typically                           │
   │                                                          │
   │   WHEN NOT to shard:                                     │
   │   • Vertical scaling still works                         │
   │   • Replicas cover read needs                            │
   │   • Premature → regret                                   │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 17. Multi-tenancy approaches

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   OPTION 1: Shared schema + tenant_id                    │
   │   ───────────────────────────────                        │
   │                                                          │
   │   Single DB → Single schema → all tenants               │
   │                                                          │
   │   ┌────────────────────────────────────┐                │
   │   │ users table                        │                │
   │   ├──────┬───────┬─────────┬──────────┤                │
   │   │ id   │ email │ tenant_id│ ...      │                │
   │   ├──────┼───────┼─────────┼──────────┤                │
   │   │ 1    │ a@... │ 100      │          │                │
   │   │ 2    │ b@... │ 100      │ tenant 100│               │
   │   │ 3    │ c@... │ 200      │ tenant 200│               │
   │   │ 4    │ d@... │ 100      │          │                │
   │   └──────┴───────┴─────────┴──────────┘                │
   │                                                          │
   │   Every query: WHERE tenant_id = ?                       │
   │                                                          │
   │   ✅ Cheap, simple                                       │
   │   ❌ Risk: forget WHERE → data leak                      │
   │   ❌ Noisy neighbor                                      │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   OPTION 2: Schema per tenant                            │
   │   ──────────────────                                     │
   │                                                          │
   │   Single DB → schemas: tenant_100, tenant_200, ...      │
   │                                                          │
   │   ┌─────────────────────────────────┐                   │
   │   │ Postgres DB                     │                   │
   │   ├─────────────────────────────────┤                   │
   │   │ schema tenant_100               │                   │
   │   │   tables: users, orders, ...    │                   │
   │   ├─────────────────────────────────┤                   │
   │   │ schema tenant_200               │                   │
   │   │   tables: users, orders, ...    │                   │
   │   ├─────────────────────────────────┤                   │
   │   │ schema tenant_300               │                   │
   │   │   ...                           │                   │
   │   └─────────────────────────────────┘                   │
   │                                                          │
   │   App sets: SET search_path = tenant_100                 │
   │                                                          │
   │   ✅ Better isolation                                    │
   │   ❌ Limit ~ 1000s schemas                               │
   │   ❌ Migration complexity                                │
   │                                                          │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │   OPTION 3: DB per tenant                                │
   │   ──────────────                                         │
   │                                                          │
   │   Multiple Postgres instances → 1 per tenant            │
   │                                                          │
   │   ┌──────┐  ┌──────┐  ┌──────┐                           │
   │   │ DB   │  │ DB   │  │ DB   │  ...                      │
   │   │ 100  │  │ 200  │  │ 300  │                           │
   │   └──────┘  └──────┘  └──────┘                           │
   │                                                          │
   │   ✅ Best isolation                                      │
   │   ✅ Per-tenant scale / backup / delete                  │
   │   ❌ Many connections (limit)                            │
   │   ❌ Cross-tenant queries hard                           │
   │   ❌ Cost                                                │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   Row-Level Security (Postgres):
   ──────────────────────────────
   
   ALTER TABLE users ENABLE ROW LEVEL SECURITY;
   CREATE POLICY tenant_isolation ON users
     USING (tenant_id = current_setting('app.tenant_id')::BIGINT);
   
   App sets per session:
     SET app.tenant_id = 100;
   
   Postgres ENFORCES: only sees tenant 100 rows.
   Defense in depth — even if app code forgot WHERE.
```

---

## 18. Time-series + partitioning

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   PROBLEM: 1B events table — queries slow, drops slow   │
   │                                                          │
   │   SOLUTION: Partition by time                            │
   │                                                          │
   │   ┌─────────────────────────────────────────────────┐    │
   │   │ events (logical table)                          │    │
   │   │  PARTITION BY RANGE (timestamp)                 │    │
   │   ├─────────────────────────────────────────────────┤    │
   │   │ events_2024_01 → physical: rows Jan 2024        │    │
   │   ├─────────────────────────────────────────────────┤    │
   │   │ events_2024_02 → physical: rows Feb 2024        │    │
   │   ├─────────────────────────────────────────────────┤    │
   │   │ events_2024_03 → physical: rows Mar 2024        │    │
   │   ├─────────────────────────────────────────────────┤    │
   │   │ events_2024_04 → physical: rows Apr 2024        │    │
   │   └─────────────────────────────────────────────────┘    │
   │                                                          │
   │   Query: SELECT * FROM events WHERE timestamp > yesterday│
   │   → Postgres ONLY scans recent partition                 │
   │   → 10-100x faster                                       │
   │                                                          │
   │   Drop old: DROP TABLE events_2024_01;                   │
   │   → INSTANT delete                                       │
   │   vs DELETE FROM events WHERE ... → hours + bloat        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   OLTP vs OLAP separation:
   ────────────────────────
   
   ┌────────────────────┐         ┌────────────────────┐
   │ OLTP (Postgres)    │ replicate│ OLAP (ClickHouse / │
   │                    │ →        │  BigQuery)         │
   │ Real-time          │ (CDC,    │                    │
   │ ms latency          │ Kafka,   │ Batch analytics    │
   │ Indexed lookups    │ Debezium)│ Full table scans   │
   │ Transactions       │          │ Aggregates          │
   │                    │          │ Seconds latency OK │
   └────────────────────┘         └────────────────────┘
   
   ⟹ Don't run BI queries on production OLTP DB
```

---

## 19. Testing strategy matrix

```
   ┌────────────────────────┬───────┬────────────────────────┐
   │ Strategy               │ Speed │ Realism                │
   ├────────────────────────┼───────┼────────────────────────┤
   │ Mock DB layer          │ ⚡⚡⚡ │ ❌ (no real SQL test)   │
   │ (mockall)              │       │                         │
   │                                                          │
   │ In-memory SQLite       │ ⚡⚡   │ ⚠️ (different from PG)  │
   │                                                          │
   │ sqlx::test (each test  │ ⚡    │ ✅ (real Postgres)      │
   │ has fresh DB)          │       │                         │
   │                                                          │
   │ Shared DB + tx rollback│ ⚡⚡   │ ✅ (but test bleed risk)│
   │                                                          │
   │ testcontainers          │ 🐢    │ ✅✅ (production-like)  │
   │ (Docker Postgres)       │       │                         │
   └────────────────────────┴───────┴────────────────────────┘
   
   
   sqlx::test pattern:
   ───────────────────
   
   #[sqlx::test(migrations = "./migrations")]
   async fn test_create_user(pool: PgPool) {
       // 1. Create fresh test DB
       // 2. Run migrations
       // 3. Pass pool to test
       // 4. Drop DB after test
       
       let user = create_user(&pool, "test@test.com").await.unwrap();
       assert_eq!(user.email, "test@test.com");
   }
   
   ⟹ Each test isolated. No leaking state.
   ⟹ Slower than mock but ~1s/test acceptable.
   
   
   Layer testing strategy:
   ───────────────────────
   
   Unit tests (domain):    mock repository (test logic)
                                    │
                                    ▼
   Repo tests:              real DB (sqlx::test)
                                    │
                                    ▼
   Integration (handler):  oneshot Router + test DB
                                    │
                                    ▼
   E2E tests:              real HTTP + real DB (testcontainers)
```

---

## 20. Common pitfalls visualization

```
   ❌ 1. N+1 queries
   ──────────────────
   for user in users {
       let orders = fetch_orders(user.id).await?;  // N queries!
   }
   ✅ Fix: JOIN or ANY()
   
   
   ❌ 2. Missing WHERE on UPDATE/DELETE
   ────────────────────────────────────
   UPDATE users SET email = 'x';   ❌ updates ALL!
   ✅ Always check rows_affected
   
   
   ❌ 3. Long transaction with external call
   ─────────────────────────────────────────
   let mut tx = pool.begin().await?;
   sqlx::query!("...").execute(&mut *tx).await?;
   call_external_api().await?;     ❌ tx held during 10s call!
   tx.commit().await?;
   
   ✅ Minimize tx scope, external calls OUTSIDE
   
   
   ❌ 4. SELECT * in production
   ────────────────────────────
   SELECT * FROM users;            ❌ schema changes break code
   ✅ Explicit columns
   
   
   ❌ 5. Storing JSON when relational works
   ────────────────────────────────────────
   ❌ emails JSONB    (can't index efficiently)
   ✅ Separate user_emails table with proper FK + index
   
   
   ❌ 6. Missing pool monitoring
   ─────────────────────────────
   No metrics on pool usage → silent saturation
   ✅ Export db_pool_used, alert > 80%
   
   
   ❌ 7. No retry for transient errors
   ───────────────────────────────────
   Single attempt → fails on momentary DB hiccup
   ✅ Retry with exponential backoff for is_transient()
   
   
   ❌ 8. Forgetting indexes
   ────────────────────────
   pg_stat_statements shows slow queries → EXPLAIN → add index
   
   
   ❌ 9. Cache invalidation issues
   ───────────────────────────────
   Update DB but forget to invalidate cache → stale data
   ✅ Either short TTL OR explicit invalidation on write
   
   
   ❌ 10. Manual SQL injection
   ───────────────────────────
   format!("SELECT * WHERE x = '{}'", user_input)  ❌ INJECTION!
   ✅ Use $1 parameterized queries always
```

---

## 21. Mind map cuối

```
                              DATABASE
                                  │
        ┌────────────┬────────────┼────────────┬─────────────┐
        ▼            ▼            ▼            ▼             ▼
   LIBRARIES    CONNECTION    TRANSACTIONS  PERFORMANCE   SCALING
        │            │            │            │             │
   sqlx          Pool         ACID         EXPLAIN       Replicas
   sea-orm       sizing       Isolation    Indexes       Sharding
   diesel        PgBouncer    Locks        N+1 fix       Multi-tenant
                 Monitoring   Retry        Caching       Partitioning
                                           Query opt
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. sqlx default. Compile-time SQL   │
                │                                      │
                │  2. Pool = throughput × query_time   │
                │     Monitor pool usage               │
                │                                      │
                │  3. Transactions short. No external  │
                │     API calls inside.                │
                │                                      │
                │  4. READ COMMITTED default.          │
                │     SERIALIZABLE when needed (retry) │
                │                                      │
                │  5. Migrations reversible if can.    │
                │     Multi-step for big changes.      │
                │                                      │
                │  6. ALWAYS EXPLAIN slow queries.     │
                │     Index based on query patterns.   │
                │                                      │
                │  7. N+1: JOIN, ANY(), or JSON_AGG.   │
                │                                      │
                │  8. Cache hot data. Invalidate care. │
                │                                      │
                │  9. Replicas for reads. Mind lag.    │
                │                                      │
                │  10. Sharding LAST resort. Hard.     │
                │                                      │
                │  11. Soft delete + audit for compl.  │
                │                                      │
                │  12. Test with REAL DB (sqlx::test)  │
                └──────────────────────────────────────┘
```

---

## 22. 🎓 Hoàn thành bộ tài liệu 18 chương

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
   │  17. axum-project            — Realistic web project    │
   │  18. database                — Database deep dive       │
   │      database-visual         ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   🎓 HOÀN THÀNH bộ tài liệu Rust senior                  │
   │                                                          │
   │   18 chủ đề × 2 files = 36 files                         │
   │   ~63,000 dòng, ~2 MB tài liệu                           │
   │                                                          │
   │   Đủ để build bất kỳ project Rust nào!                  │
   └──────────────────────────────────────────────────────────┘
   
   
   Khả năng giờ có:
   ────────────────
   
   🌐 Production web services
   🗄️ Database-heavy applications
   🔌 Embedded systems (MCU)
   🚀 High-performance services
   🧪 Well-tested codebases
   🔍 Observable production code
   ⚙️ System programming (unsafe, FFI)
   📊 Concurrent data processing
   🎮 Game engines
   🖥️ CLI tools
   📡 Network protocols
   🤖 WASM applications
```

---

## Bước tiếp theo — Áp dụng vào project thực tế

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   1. BUILD SIDE PROJECT                                  │
   │      Small but production-quality                        │
   │      Apply all 18 chapters                               │
   │                                                          │
   │   2. CONTRIBUTE TO OPEN SOURCE                           │
   │      sqlx, axum, tokio, embedded-hal, ...               │
   │      Read mature code                                    │
   │                                                          │
   │   3. READ EXEMPLARY CODE                                 │
   │      rustc (compiler)                                    │
   │      tokio (runtime)                                     │
   │      axum (web)                                          │
   │      sqlx (database)                                     │
   │      serde (serialization)                               │
   │      embedded-hal (embedded)                             │
   │                                                          │
   │   4. FOLLOW RUST NEWS                                    │
   │      This Week in Rust                                   │
   │      Rust blog                                           │
   │      r/rust                                              │
   │                                                          │
   │   5. CONFERENCES & TALKS                                 │
   │      RustConf, EuroRust, RustFest                        │
   │      Watch YouTube talks                                 │
   │                                                          │
   │   6. WRITE & SHARE                                       │
   │      Blog posts                                          │
   │      OSS libraries                                       │
   │      Help others (Discord, forum)                        │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   🦀 Chúc bạn senior Rust journey thành công!
```
