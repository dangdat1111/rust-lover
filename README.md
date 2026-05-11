# rust-lover

Repo cá nhân để học và thực hành ngôn ngữ lập trình **Rust** — từ cơ bản đến nâng cao.

## Mục tiêu

- Nắm vững ownership, borrowing, và lifetime
- Hiểu hệ thống kiểu (type system) mạnh mẽ của Rust
- Thực hành viết code an toàn, hiệu năng cao
- Xây dựng các project nhỏ để củng cố kiến thức

## Cấu trúc (dự kiến)

```
rust-lover/
├── basics/          # Biến, kiểu dữ liệu, hàm, control flow
├── ownership/       # Ownership, borrowing, slices
├── structs/         # Struct, enum, pattern matching
├── collections/     # Vec, HashMap, String
├── error_handling/  # Result, Option, ? operator
├── generics/        # Generics, traits, lifetimes
├── closures/        # Closures, iterators
├── concurrency/     # Threads, async/await
└── projects/        # Các mini project thực hành
```

## Chủ đề theo lộ trình

| # | Chủ đề | Trạng thái |
|---|--------|------------|
| 1 | Variables & Data Types | |
| 2 | Functions & Control Flow | |
| 3 | Ownership & Borrowing | |
| 4 | Structs & Enums | |
| 5 | Pattern Matching | |
| 6 | Collections | |
| 7 | Error Handling | |
| 8 | Generics & Traits | |
| 9 | Lifetimes | |
| 10 | Closures & Iterators | |
| 11 | Smart Pointers | |
| 12 | Concurrency | |
| 13 | Async / Await | |

## Chạy code

Cài Rust qua [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Chạy một file bất kỳ:

```bash
cd <thư-mục>
cargo run
```

Chạy tests:

```bash
cargo test
```

## Tài nguyên học

- [The Rust Book](https://doc.rust-lang.org/book/) — tài liệu chính thức, miễn phí
- [Rustlings](https://github.com/rust-lang/rustlings) — bài tập tương tác
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) — học qua ví dụ
- [Exercism - Rust track](https://exercism.org/tracks/rust) — luyện tập có mentor

## License

[MIT](LICENSE)
