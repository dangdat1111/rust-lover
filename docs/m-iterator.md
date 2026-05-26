# Iterator trong Rust — Deep Dive

> Tài liệu thứ 13 trong bộ Rust nền tảng. Đọc trước:
> - [trait.md](./trait.md) — Iterator là 1 trait
> - [generic.md](./generic.md) — trait bounds, associated types
> - [closure.md](./closure.md) — map/filter/fold dùng closure
> - [lifetime.md](./lifetime.md) — iterator borrow rules
> - [performance.md](./performance.md) — iterator chains zero-cost
>
> **Iterator** là một trong những abstraction **mạnh nhất** trong Rust. Nó:
> - Cho phép viết code declarative (mô tả "what", không phải "how")
> - **Zero-cost** — compile thành code tương đương loop manual
> - **Lazy** — không compute cho đến khi cần
> - Composable — chain `map`, `filter`, `fold`... thoải mái
>
> Tài liệu này dạy bạn:
> - Cơ chế bên trong Iterator trait
> - 70+ methods quan trọng + khi nào dùng
> - Implement custom Iterator
> - Lazy evaluation deep
> - Parallel với `rayon`
> - Async với `Stream`
> - Patterns và antipatterns senior

---

# Mục lục

- [Tầng 1: Iterator là gì?](#tầng-1-iterator-là-gì)
- [Tầng 2: Iterator trait — Định nghĩa cốt lõi](#tầng-2-iterator-trait--định-nghĩa-cốt-lõi)
- [Tầng 3: IntoIterator — `for` loop magic](#tầng-3-intoiterator--for-loop-magic)
- [Tầng 4: iter, iter_mut, into_iter — Chọn đúng](#tầng-4-iter-iter_mut-into_iter--chọn-đúng)
- [Tầng 5: Lazy evaluation — Bí mật quan trọng nhất](#tầng-5-lazy-evaluation--bí-mật-quan-trọng-nhất)
- [Tầng 6: Adapters — Transformers trong chain](#tầng-6-adapters--transformers-trong-chain)
- [Tầng 7: Consumers — Kết thúc chain](#tầng-7-consumers--kết-thúc-chain)
- [Tầng 8: Implement Iterator thủ công](#tầng-8-implement-iterator-thủ-công)
- [Tầng 9: Sub-traits — DoubleEndedIterator, ExactSizeIterator, FusedIterator](#tầng-9-sub-traits--doubleendediterator-exactsizeiterator-fusediterator)
- [Tầng 10: 70+ methods — Cheatsheet đầy đủ](#tầng-10-70-methods--cheatsheet-đầy-đủ)
- [Tầng 11: itertools — Power-ups extra](#tầng-11-itertools--power-ups-extra)
- [Tầng 12: Parallel iterator — rayon](#tầng-12-parallel-iterator--rayon)
- [Tầng 13: Stream — Async iterator](#tầng-13-stream--async-iterator)
- [Tầng 14: Performance — Zero-cost trong thực tế](#tầng-14-performance--zero-cost-trong-thực-tế)
- [Tầng 15: Patterns và Antipatterns](#tầng-15-patterns-và-antipatterns)

---

# Tầng 1: Iterator là gì?

## 1.1 Vấn đề lập trình thông thường

Tính tổng bình phương của số chẵn trong 1 vec:

### C-style (imperative)
```rust
let v = vec![1, 2, 3, 4, 5];
let mut sum = 0;
for i in 0..v.len() {
    if v[i] % 2 == 0 {
        sum += v[i] * v[i];
    }
}
```

### Rust idiomatic (functional / iterator)
```rust
let sum: i32 = v.iter()
    .filter(|&&x| x % 2 == 0)
    .map(|&x| x * x)
    .sum();
```

So sánh:
- **Imperative**: chi tiết HOW, dễ bug (off-by-one, mutable state)
- **Functional**: mô tả WHAT, ngắn, composable

Iterator giúp viết **declarative code** mà KHÔNG mất performance — compile thành code tương đương loop manual.

## 1.2 Iterator trong các ngôn ngữ

| Ngôn ngữ | Iterator |
|----------|----------|
| **C** | Manual: index + condition |
| **C++** | STL iterators (begin/end), Ranges (C++20) |
| **Java** | Iterator interface, Stream API (Java 8+) |
| **Python** | `__iter__`/`__next__`, generators |
| **JS** | `Symbol.iterator`, generators |
| **Rust** | `Iterator` trait + lazy adapters |

Rust mạnh hơn vì:
- Zero-cost (compile to same code as loop)
- Type-safe + ownership-aware
- Lazy by default
- Trait-based — extensible

## 1.3 Tại sao Iterator quan trọng?

```rust
// Without iterator:
let mut total_age = 0;
let mut count = 0;
for user in users {
    if user.is_active {
        total_age += user.age;
        count += 1;
    }
}
let avg = total_age / count;

// With iterator:
let avg: u32 = users.iter()
    .filter(|u| u.is_active)
    .map(|u| u.age)
    .sum::<u32>() / users.iter().filter(|u| u.is_active).count() as u32;

// Cleaner with fold:
let (total, count) = users.iter()
    .filter(|u| u.is_active)
    .fold((0u32, 0u32), |(t, c), u| (t + u.age, c + 1));
let avg = total / count;
```

Iterator code:
- ✅ Ngắn
- ✅ Express intent rõ ràng
- ✅ Khó bug off-by-one
- ✅ Parallel hóa được (rayon)
- ✅ Compose dễ

## 1.4 Khám phá đầu tiên

```rust
let v = vec![1, 2, 3];

// Get iterator
let mut iter = v.iter();

// Manual next()
println!("{:?}", iter.next());  // Some(&1)
println!("{:?}", iter.next());  // Some(&2)
println!("{:?}", iter.next());  // Some(&3)
println!("{:?}", iter.next());  // None
println!("{:?}", iter.next());  // None (after exhausted)
```

Iterator = stream giá trị. `next()` trả `Option<T>`:
- `Some(value)` — còn item
- `None` — hết

## 1.5 Iterator vs Container

```rust
let v: Vec<i32> = vec![1, 2, 3];      // Container (Vec)
let iter = v.iter();                    // Iterator over Vec

// Container stores data
// Iterator yields data on demand (lazy)
```

Hiểu rõ: **Vec** lưu data, **Iterator** describe how to traverse.

---

# Tầng 2: Iterator trait — Định nghĩa cốt lõi

## 2.1 Trait definition

```rust
pub trait Iterator {
    type Item;
    
    fn next(&mut self) -> Option<Self::Item>;
    
    // ... 70+ default methods (map, filter, fold, ...)
}
```

Quan trọng:
- **Chỉ 1 method required**: `next()`
- **Associated type `Item`**: kiểu yielded
- **70+ default methods** built trên `next()`

→ Implement Iterator trait = implement `next()`. 70 methods khác **free**.

## 2.2 Ví dụ implement thủ công

```rust
struct Counter {
    count: u32,
    max: u32,
}

impl Iterator for Counter {
    type Item = u32;
    
    fn next(&mut self) -> Option<u32> {
        if self.count < self.max {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

let counter = Counter { count: 0, max: 5 };
let v: Vec<u32> = counter.collect();
println!("{:?}", v);  // [1, 2, 3, 4, 5]
```

Implement chỉ `next()`. Có ngay `collect()`, `sum()`, `map()`, `filter()`...

## 2.3 Cơ chế `for` loop

```rust
for x in vec![1, 2, 3] {
    println!("{}", x);
}

// Equivalent (compiler expand):
let mut iter = vec![1, 2, 3].into_iter();
loop {
    match iter.next() {
        Some(x) => { println!("{}", x); }
        None => break,
    }
}
```

`for` loop = sugar cho `loop + next()`. Bất cứ type nào implement Iterator → dùng `for` được.

## 2.4 Method chain return Iterator

```rust
let iter = v.iter()       // returns Iter<'_, i32>
    .map(|x| x * 2)        // returns Map<Iter<...>, ...>
    .filter(|&x| x > 5);    // returns Filter<Map<...>, ...>
```

Mỗi adapter trả type mới (zero-size struct wrap iterator trước). Iterator wraps iterator wraps iterator → forms 1 type complex tại compile time.

Khi `for` loop hoặc `collect()` chạy, **tất cả flatten** thành code optimized — không có heap alloc per step.

## 2.5 Iterator là pull-based

```
Consumer: for x in iter { ... }
              │
              │ "give me next item"
              ▼
        iter.next()
              │
              │ "give me from upstream"
              ▼
        upstream.next()
              │
              ...
```

Consumer **pull** từ source. Lazy: chỉ compute khi pulled.

Khác **push-based** (RxJava observable): source push → subscriber. Iterator pull, Stream pull (async), Channel push.

---

# Tầng 3: IntoIterator — `for` loop magic

## 3.1 IntoIterator trait

```rust
pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    
    fn into_iter(self) -> Self::IntoIter;
}
```

`IntoIterator`: "có thể turn thành iterator". `for` loop call `into_iter()`:

```rust
for x in collection {
    // ...
}
// = for x in collection.into_iter() { ... }
```

## 3.2 Vec implements IntoIterator 3 cách

```rust
let v = vec![1, 2, 3];

// 1. Consume Vec
for x in v {           // v.into_iter() — yields i32 (move)
    println!("{}", x);
}
// v không dùng được nữa

// 2. Borrow Vec
for x in &v {          // v.iter() = (&v).into_iter() — yields &i32
    println!("{}", x);
}
// v vẫn dùng được

// 3. Mut borrow Vec
for x in &mut v {      // v.iter_mut() — yields &mut i32
    *x *= 2;
}
```

Vec implements 3 versions của `IntoIterator`:
- `impl IntoIterator for Vec<T>` → owned (yields `T`)
- `impl IntoIterator for &Vec<T>` → borrowed (yields `&T`)
- `impl IntoIterator for &mut Vec<T>` → mut borrowed (yields `&mut T`)

## 3.3 Implementing IntoIterator

```rust
struct MyCollection {
    items: Vec<i32>,
}

impl IntoIterator for MyCollection {
    type Item = i32;
    type IntoIter = std::vec::IntoIter<i32>;
    
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

let coll = MyCollection { items: vec![1, 2, 3] };
for x in coll {  // works!
    println!("{}", x);
}
```

## 3.4 `from_iter` — Collect ngược lại

```rust
pub trait FromIterator<A>: Sized {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}
```

Implement `FromIterator` cho type → `collect::<Type>()` work:

```rust
struct Bag(Vec<i32>);

impl FromIterator<i32> for Bag {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Bag {
        Bag(iter.into_iter().collect())
    }
}

let bag: Bag = vec![1, 2, 3].into_iter().collect();
```

Hữu ích cho custom collection types.

---

# Tầng 4: iter, iter_mut, into_iter — Chọn đúng

## 4.1 3 hình thức

```rust
let v = vec![1, 2, 3];

let _: std::slice::Iter<i32> = v.iter();       // yields &T (borrow)
let _: std::slice::IterMut<i32> = v.iter_mut(); // yields &mut T
let _: std::vec::IntoIter<i32> = v.into_iter(); // yields T (consume)
```

| Method | Yields | Effect on collection |
|--------|--------|----------------------|
| `.iter()` | `&T` | borrow (collection stays) |
| `.iter_mut()` | `&mut T` | mut borrow |
| `.into_iter()` | `T` | consume (collection gone) |

## 4.2 Examples

### iter — read only
```rust
let v = vec![1, 2, 3];
let sum: i32 = v.iter().sum();   // borrow, v stays
println!("{:?}", v);              // OK
```

### iter_mut — modify in place
```rust
let mut v = vec![1, 2, 3];
for x in v.iter_mut() {
    *x *= 2;
}
// v now [2, 4, 6]
```

### into_iter — consume to transform
```rust
let v = vec![1, 2, 3];
let doubled: Vec<i32> = v.into_iter().map(|x| x * 2).collect();
// v consumed, doubled = [2, 4, 6]
```

## 4.3 Khi for loop dùng cái nào?

```rust
let v = vec![1, 2, 3];

for x in v          { /* T   */ }  // into_iter — consumes
for x in &v         { /* &T  */ }  // iter
for x in &mut v     { /* &mut T */ }  // iter_mut
for x in v.iter()   { /* &T  */ }  // explicit iter
```

Rust biết dựa trên type:
- `v: Vec<T>` → `into_iter()`
- `&v` → `iter()`
- `&mut v` → `iter_mut()`

## 4.4 Array hành xử khác

```rust
let arr = [1, 2, 3];

for x in arr {              // Yields T (i32 — Copy)
    println!("{}", x);
}
// arr vẫn dùng được vì i32 Copy

for x in arr.iter() {       // Yields &T
    println!("{}", x);
}
```

Array (`[T; N]`) implement `IntoIterator` từ Rust 1.53+. Trước đó phải `iter()`.

## 4.5 String — iter chars, bytes, hoặc lines

```rust
let s = "hello world";

for c in s.chars() { /* char */ }            // Unicode characters
for b in s.bytes() { /* u8 */ }              // UTF-8 bytes
for w in s.split_whitespace() { /* &str */ } // words
for l in s.lines() { /* &str */ }            // lines
```

String **không** implement `Iterator` directly. Phải pick: chars, bytes, lines, ... mỗi cái different semantics.

## 4.6 HashMap iteration

```rust
let mut map: HashMap<String, i32> = ...;

for (k, v) in &map           { /* (&String, &i32) */ }
for (k, v) in &mut map       { /* (&String, &mut i32) */ }
for (k, v) in map            { /* (String, i32) — consumes */ }

for k in map.keys()          { /* &String */ }
for v in map.values()        { /* &i32 */ }
for v in map.values_mut()    { /* &mut i32 */ }
```

---

# Tầng 5: Lazy evaluation — Bí mật quan trọng nhất

## 5.1 Iterator KHÔNG chạy cho đến khi tiêu thụ

```rust
let v = vec![1, 2, 3];

let iter = v.iter()
    .map(|x| {
        println!("mapping {}", x);   // ← KHÔNG chạy ở đây
        x * 2
    });

println!("Created iterator");   // Print này ra trước

// Bây giờ tiêu thụ:
for x in iter {
    println!("Got {}", x);
}

// Output:
// Created iterator
// mapping 1
// Got 2
// mapping 2
// Got 4
// mapping 3
// Got 6
```

`map` chỉ **tạo** struct describe transformation. Không apply cho đến khi `for`/`collect`/`sum`.

## 5.2 Tại sao lazy?

### Lợi ích 1: Compose without alloc

```rust
let v: Vec<i32> = (1..1_000_000)        // không alloc 1M items
    .map(|x| x * 2)                       // không transform
    .filter(|&x| x % 3 == 0)              // không filter
    .take(10)                              // chỉ pull 10 items khi consume
    .collect();
```

Lazy → chỉ compute 10 items đầu thoả mãn. Eager (như Python list comp) sẽ compute 1M.

### Lợi ích 2: Infinite iterator

```rust
let first_10_squares: Vec<u32> = (1u32..)
    .map(|x| x * x)
    .take(10)
    .collect();
// [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]
```

`1u32..` infinite. Lazy OK. Eager → infinite loop.

### Lợi ích 3: Stream processing

```rust
read_file_lines("huge.txt")
    .map(parse_line)
    .filter(|line| line.is_valid())
    .take_while(|line| line.timestamp < cutoff)
    .for_each(process);
```

Stream qua file mà không load all vào RAM.

## 5.3 must_use warning

```rust
let _ = v.iter().map(|x| x * 2);   // warning: unused iterator
```

Compiler warn `must_use` — iterator created nhưng không tiêu thụ. Dấu hiệu bug.

```rust
v.iter().map(|x| println!("{}", x));   // ❌ KHÔNG print gì
v.iter().for_each(|x| println!("{}", x));  // ✅ for_each tiêu thụ
```

## 5.4 Consumer methods kích hoạt iterator

```rust
// Lazy adapters (không tiêu thụ):
.map(), .filter(), .take(), .skip(), .chain(), .zip(), .enumerate()...

// Consumer (tiêu thụ + return final value):
.collect(), .sum(), .count(), .max(), .min(), .fold(), .reduce()
.for_each(), .all(), .any(), .find(), .last(), .nth(), .position()
.next() (manual)
```

Khi gặp consumer, iterator chạy thực sự.

## 5.5 Visualize lazy chain

```rust
let result: Vec<i32> = (1..)
    .map(|x| x * 2)
    .filter(|&x| x > 5)
    .take(3)
    .collect();
// → [6, 8, 10]
```

Execution flow (pull-based):
```
collect() asks take for next
  take asks filter for next
    filter asks map for next
      map asks (1..) for next → 1
      map outputs 2
    filter: 2 > 5? No, ask again
      map asks (1..) → 2
      map outputs 4
    filter: 4 > 5? No, ask again
      map asks (1..) → 3
      map outputs 6
    filter: 6 > 5? Yes → output 6
  take: 1/3, output 6
collect appends 6

(repeat for 8, 10)

take exhausts after 3 → outputs None
collect returns Vec
```

Compiler **inline tất cả** → loop tối ưu tương đương manual loop.

---

# Tầng 6: Adapters — Transformers trong chain

## 6.1 map — Transform each element

```rust
let doubled: Vec<i32> = vec![1, 2, 3].iter().map(|&x| x * 2).collect();
// [2, 4, 6]

// map_while: take elements while closure returns Some
let v: Vec<i32> = vec!["1", "2", "abc", "4"].iter()
    .map_while(|s| s.parse().ok())
    .collect();
// [1, 2] (stops at "abc")
```

## 6.2 filter — Keep matching

```rust
let evens: Vec<i32> = vec![1, 2, 3, 4, 5].into_iter()
    .filter(|&x| x % 2 == 0)
    .collect();
// [2, 4]

// filter_map: combine filter + map
let nums: Vec<i32> = vec!["1", "abc", "3"].iter()
    .filter_map(|s| s.parse().ok())
    .collect();
// [1, 3]
```

`filter_map` cực kỳ hữu ích — parse + skip failures trong 1 step.

## 6.3 take, skip, take_while, skip_while

```rust
let v = vec![1, 2, 3, 4, 5];

v.iter().take(3);      // [1, 2, 3]
v.iter().skip(2);      // [3, 4, 5]

v.iter().take_while(|&&x| x < 4);  // [1, 2, 3] (stop at first false)
v.iter().skip_while(|&&x| x < 3);  // [3, 4, 5] (skip until first false)
```

## 6.4 chain — Combine iterators

```rust
let a = vec![1, 2, 3];
let b = vec![4, 5, 6];
let combined: Vec<_> = a.iter().chain(b.iter()).collect();
// [&1, &2, &3, &4, &5, &6]
```

## 6.5 zip — Pair iterators

```rust
let names = vec!["Alice", "Bob"];
let ages = vec![30, 25];

for (name, age) in names.iter().zip(ages.iter()) {
    println!("{} is {}", name, age);
}
// "Alice is 30"
// "Bob is 25"

// zip stops at shorter:
vec![1, 2, 3].iter().zip(vec!['a', 'b'].iter())
// yields (&1, &'a'), (&2, &'b'), nothing more
```

## 6.6 enumerate — Pair with index

```rust
for (i, item) in vec!['a', 'b', 'c'].iter().enumerate() {
    println!("{}: {}", i, item);
}
// 0: a
// 1: b
// 2: c
```

## 6.7 rev — Reverse

```rust
let v: Vec<i32> = (1..=5).rev().collect();
// [5, 4, 3, 2, 1]

// rev only for DoubleEndedIterator (Tầng 9)
```

## 6.8 flat_map — Map + flatten

```rust
let nested = vec![vec![1, 2], vec![3, 4]];
let flat: Vec<i32> = nested.into_iter().flat_map(|v| v).collect();
// [1, 2, 3, 4]

// Or flatten() directly:
let flat: Vec<i32> = vec![vec![1, 2], vec![3, 4]].into_iter().flatten().collect();
```

`flat_map(f)` = `map(f).flatten()`.

Use case: split each string into words:
```rust
let texts = vec!["hello world", "foo bar"];
let words: Vec<&str> = texts.iter().flat_map(|s| s.split_whitespace()).collect();
// ["hello", "world", "foo", "bar"]
```

## 6.9 step_by — Skip by N

```rust
let evens: Vec<i32> = (0..10).step_by(2).collect();
// [0, 2, 4, 6, 8]
```

## 6.10 inspect — Side effect for debugging

```rust
let sum: i32 = (1..=5)
    .inspect(|x| println!("got {}", x))   // print each, không consume
    .filter(|&x| x > 2)
    .inspect(|x| println!("after filter {}", x))
    .sum();
```

Inspect pass-through nhưng cho phép side effect. Debug iterator chain.

## 6.11 peekable — Peek next without consuming

```rust
let mut iter = vec![1, 2, 3].into_iter().peekable();

if iter.peek() == Some(&1) {
    iter.next();   // consume
}
```

Useful cho parser.

## 6.12 cycle — Infinite repeat

```rust
let mut iter = vec![1, 2, 3].iter().cycle();
iter.next();  // Some(&1)
iter.next();  // Some(&2)
iter.next();  // Some(&3)
iter.next();  // Some(&1) -- starts over!
```

## 6.13 fuse — Stop forever after first None

```rust
let mut iter = some_iterator.fuse();
// Sau khi return None lần đầu, mọi lần sau return None
// (some iterators có thể return Some sau None — fuse prevents that)
```

## 6.14 scan — Map with state

```rust
let v: Vec<i32> = (1..=5).scan(0, |state, x| {
    *state += x;
    Some(*state)
}).collect();
// [1, 3, 6, 10, 15] (running sum)
```

Like fold but yields intermediate values.

## 6.15 by_ref — Borrow iterator

```rust
let mut iter = (1..=10);
let first_3: Vec<i32> = iter.by_ref().take(3).collect();
// first_3 = [1, 2, 3]
// iter vẫn dùng được, position 4

let rest: Vec<i32> = iter.collect();
// rest = [4, 5, 6, 7, 8, 9, 10]
```

`by_ref` lấy `&mut Iterator` thay vì consume. Quan trọng để partial-consume.

---

# Tầng 7: Consumers — Kết thúc chain

## 7.1 collect — Versatile collector

```rust
let v: Vec<i32> = (1..=5).collect();
let s: String = vec!['h', 'i'].into_iter().collect();
let m: HashMap<i32, i32> = vec![(1, 2), (3, 4)].into_iter().collect();
let set: HashSet<i32> = vec![1, 2, 2, 3].into_iter().collect();

// Collect Result<Vec, E>:
let r: Result<Vec<i32>, _> = vec!["1", "2", "abc"].iter().map(|s| s.parse()).collect();
// Err(ParseIntError) — fails fast at first Err
```

`collect()` works với mọi type implement `FromIterator`. Type annotation cần (turbofish hoặc let type).

```rust
let v = (1..=5).collect::<Vec<i32>>();   // turbofish
let v: Vec<i32> = (1..=5).collect();      // let type
```

## 7.2 sum, product

```rust
let sum: i32 = (1..=10).sum();        // 55
let product: i32 = (1..=5).product();  // 120

// Float OK:
let s: f64 = vec![1.5, 2.5, 3.5].iter().sum();
```

Yêu cầu `T: Sum`. Standard numeric types built-in.

## 7.3 count, max, min, last

```rust
(1..=10).count();                       // 10
vec![3, 1, 4, 1, 5].iter().max();       // Some(&5)
vec![3, 1, 4, 1, 5].iter().min();       // Some(&1)
(1..=10).last();                        // Some(10)

// max/min by closure:
vec!["foo", "barbaz", "x"].iter().max_by_key(|s| s.len());  // Some(&"barbaz")

// max by custom comparator:
vec![3.0, 1.0, 4.0].iter().max_by(|a, b| a.partial_cmp(b).unwrap());
```

## 7.4 fold — General accumulator

```rust
let sum: i32 = (1..=10).fold(0, |acc, x| acc + x);  // 55

// Build string:
let s: String = (1..=5).fold(String::new(), |mut s, n| {
    s.push_str(&n.to_string());
    s
});
// "12345"

// Build tuple:
let (sum, product) = (1..=4).fold((0, 1), |(s, p), x| (s + x, p * x));
// (10, 24)
```

`fold(initial, |acc, x| new_acc)` — general-purpose. Sum, product, count, build collection ... all fold cases.

## 7.5 reduce — Fold without initial

```rust
let max: Option<i32> = vec![3, 1, 4, 1, 5].into_iter().reduce(i32::max);
// Some(5)

// Empty → None:
let empty: Vec<i32> = vec![];
empty.into_iter().reduce(|a, b| a + b);  // None
```

`reduce` like fold but use first element as initial. Returns `Option` because empty.

## 7.6 for_each — Side-effect iteration

```rust
(1..=5).for_each(|x| println!("{}", x));
```

= `for x in (1..=5) { println!("{}", x); }`. Cleaner trong chains:

```rust
data.iter()
    .filter(|x| x.is_valid())
    .for_each(|x| process(x));
```

## 7.7 all, any — Boolean reduce

```rust
vec![2, 4, 6].iter().all(|&x| x % 2 == 0);   // true
vec![1, 2, 3].iter().any(|&x| x > 2);         // true

// Short-circuit:
vec![1, 2, 3, 4].iter().any(|&x| {
    println!("checking {}", x);
    x == 2
});
// Output:
// checking 1
// checking 2
// (returns true, stops here)
```

Both short-circuit.

## 7.8 find, position

```rust
vec![1, 2, 3].iter().find(|&&x| x > 1);     // Some(&2)
vec![1, 2, 3].iter().position(|&x| x > 1);  // Some(1) — index

vec![1, 2, 3].iter().find_map(|x| {
    if *x > 1 { Some(x * 10) } else { None }
});  // Some(20) — find + transform
```

## 7.9 nth — Get nth element

```rust
let mut iter = (1..=10);
iter.nth(2);   // Some(3) — 0-indexed
iter.nth(0);   // Some(4) — iter advanced
```

Consume up to nth + 1 elements.

## 7.10 partition — Split into two

```rust
let (evens, odds): (Vec<i32>, Vec<i32>) = (1..=10)
    .partition(|&x| x % 2 == 0);
// evens = [2, 4, 6, 8, 10]
// odds = [1, 3, 5, 7, 9]
```

## 7.11 unzip — Split tuple iterator

```rust
let pairs = vec![(1, 'a'), (2, 'b'), (3, 'c')];
let (nums, chars): (Vec<i32>, Vec<char>) = pairs.into_iter().unzip();
// nums = [1, 2, 3]
// chars = ['a', 'b', 'c']
```

Inverse of `zip`.

## 7.12 collect into specific type

```rust
// Vec<T>
let v: Vec<i32> = (1..=5).collect();

// HashMap<K, V>
let m: HashMap<i32, i32> = (1..=5).map(|i| (i, i * 2)).collect();

// String
let s: String = vec!['h', 'i'].into_iter().collect();

// Result<Vec<_>, _>
let r: Result<Vec<i32>, _> = strs.iter().map(|s| s.parse()).collect();

// Option<Vec<_>>
let r: Option<Vec<i32>> = strs.iter().map(|s| s.parse().ok()).collect();
// → None if any element is None

// Custom collection via FromIterator
let bag: Bag = (1..=5).collect();
```

---

# Tầng 8: Implement Iterator thủ công

## 8.1 Counter — Simple example

```rust
struct Counter {
    count: u32,
    limit: u32,
}

impl Iterator for Counter {
    type Item = u32;
    
    fn next(&mut self) -> Option<u32> {
        if self.count < self.limit {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

let c = Counter { count: 0, limit: 5 };
println!("{:?}", c.collect::<Vec<_>>());  // [1, 2, 3, 4, 5]
```

## 8.2 Fibonacci iterator

```rust
struct Fib {
    curr: u64,
    next: u64,
}

impl Iterator for Fib {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        let n = self.curr;
        self.curr = self.next;
        self.next = n + self.curr;
        Some(n)   // Infinite — never None
    }
}

let fibs: Vec<u64> = Fib { curr: 0, next: 1 }.take(10).collect();
// [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
```

Infinite iterator — combine với `take` để limit.

## 8.3 Iterator borrowing slice

```rust
struct Windows<'a, T> {
    slice: &'a [T],
    size: usize,
    pos: usize,
}

impl<'a, T> Iterator for Windows<'a, T> {
    type Item = &'a [T];
    
    fn next(&mut self) -> Option<&'a [T]> {
        if self.pos + self.size > self.slice.len() {
            return None;
        }
        let w = &self.slice[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

let v = vec![1, 2, 3, 4, 5];
let iter = Windows { slice: &v, size: 3, pos: 0 };
for w in iter {
    println!("{:?}", w);
}
// [1, 2, 3]
// [2, 3, 4]
// [3, 4, 5]
```

(`slice::windows(3)` đã có sẵn — đây chỉ demo.)

## 8.4 Stateful iterator — Running average

```rust
struct RunningAverage<I: Iterator<Item = f64>> {
    inner: I,
    sum: f64,
    count: u64,
}

impl<I: Iterator<Item = f64>> Iterator for RunningAverage<I> {
    type Item = f64;
    
    fn next(&mut self) -> Option<f64> {
        let x = self.inner.next()?;
        self.sum += x;
        self.count += 1;
        Some(self.sum / self.count as f64)
    }
}

trait RunningAvgExt: Iterator<Item = f64> + Sized {
    fn running_avg(self) -> RunningAverage<Self> {
        RunningAverage { inner: self, sum: 0.0, count: 0 }
    }
}

impl<I: Iterator<Item = f64>> RunningAvgExt for I {}

let avgs: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0].into_iter()
    .running_avg()
    .collect();
// [1.0, 1.5, 2.0, 2.5]
```

Extension trait pattern: `MyIter: Iterator + Sized` → add custom methods on existing iterators.

## 8.5 Generator-like iterator

```rust
// Yield: 1, 1, 2, 3, 5, 8, ... (Fibonacci)
fn fibonacci() -> impl Iterator<Item = u64> {
    let mut state = (0u64, 1u64);
    std::iter::from_fn(move || {
        let v = state.0;
        state = (state.1, state.0 + state.1);
        Some(v)
    })
}

let v: Vec<u64> = fibonacci().take(8).collect();
// [0, 1, 1, 2, 3, 5, 8, 13]
```

`std::iter::from_fn(|| Some(value) | None)` — quick way to create iterator without defining struct.

## 8.6 std::iter::successors

```rust
let powers_of_2 = std::iter::successors(Some(1u64), |&n| Some(n * 2));
let v: Vec<u64> = powers_of_2.take(10).collect();
// [1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
```

`successors(initial, |prev| next)` — chain values.

## 8.7 std::iter::repeat, once, empty

```rust
std::iter::once(42);           // [42]
std::iter::empty::<i32>();     // []
std::iter::repeat(1).take(3);  // [1, 1, 1]
```

Useful as building blocks.

---

# Tầng 9: Sub-traits — DoubleEndedIterator, ExactSizeIterator, FusedIterator

## 9.1 DoubleEndedIterator

```rust
pub trait DoubleEndedIterator: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;
}
```

Iterator có thể consume từ **đầu** hoặc **cuối**:

```rust
let v = vec![1, 2, 3, 4, 5];
let mut iter = v.iter();
iter.next();      // Some(&1)
iter.next_back(); // Some(&5)
iter.next();      // Some(&2)
iter.next_back(); // Some(&4)
iter.next();      // Some(&3)
iter.next();      // None
```

Enables `.rev()`:
```rust
let v: Vec<i32> = (1..=5).rev().collect();  // [5, 4, 3, 2, 1]
```

Available for: Vec iter, slice iter, range, ...

## 9.2 ExactSizeIterator

```rust
pub trait ExactSizeIterator: Iterator {
    fn len(&self) -> usize;
}
```

Iterator biết chính xác số element còn lại:
```rust
let v = vec![1, 2, 3];
let iter = v.iter();
assert_eq!(iter.len(), 3);
```

Enables optimization (preallocate Vec when collect):
```rust
let v: Vec<i32> = (1..=1000).collect();  // alloc 1000 capacity upfront
```

Available for: Vec/slice iter, range (bounded).

## 9.3 FusedIterator

```rust
pub trait FusedIterator: Iterator {}
```

Iterator đã return `None` thì **mãi mãi** return `None`. Marker trait — không có method.

Some weird iterators có thể alternate `Some`/`None` — FusedIterator guarantees no.

Most std iterators implement FusedIterator. `.fuse()` adapter forces this.

## 9.4 Iterator size_hint

```rust
fn size_hint(&self) -> (usize, Option<usize>) {
    // (lower_bound, optional upper_bound)
}
```

Default method — returns expected count. Used by `collect()` để preallocate.

```rust
let v: Vec<i32> = (1..=100).collect();
// size_hint = (100, Some(100)) → Vec::with_capacity(100)
```

Implement đúng cho custom iterator → faster collect.

---

# Tầng 10: 70+ methods — Cheatsheet đầy đủ

## 10.1 Adapter (return iterator)

| Method | Effect |
|--------|--------|
| `map(F)` | Transform each |
| `filter(F)` | Keep matching |
| `filter_map(F)` | Map + filter (`-> Option<T>`) |
| `flat_map(F)` | Map then flatten |
| `flatten()` | Flatten nested iter |
| `take(n)` | First n |
| `skip(n)` | After first n |
| `take_while(F)` | While true |
| `skip_while(F)` | Until first true |
| `step_by(n)` | Every nth |
| `chain(iter)` | Concat |
| `zip(iter)` | Pair up |
| `enumerate()` | Add index |
| `rev()` | Reverse |
| `cycle()` | Infinite repeat |
| `peekable()` | Add peek() |
| `fuse()` | Stop after first None |
| `inspect(F)` | Side-effect, pass-through |
| `scan(init, F)` | Accumulate, yield each |
| `map_while(F)` | Map until None |
| `cloned()` | `&T` → `T` (clone) |
| `copied()` | `&T` → `T` (copy, Copy types) |
| `by_ref()` | Borrow iter |

## 10.2 Consumer (return value)

| Method | Returns |
|--------|---------|
| `collect::<T>()` | Build collection |
| `sum::<T>()` | Total |
| `product::<T>()` | Multiply |
| `count()` | Element count |
| `max()` / `min()` | Max/min |
| `max_by_key(F)` / `min_by_key(F)` | By key |
| `max_by(F)` / `min_by(F)` | By comparator |
| `last()` | Last element |
| `nth(n)` | nth element |
| `first()` | (on slice) |
| `fold(init, F)` | Reduce |
| `reduce(F)` | Fold without init |
| `for_each(F)` | Side-effect loop |
| `all(F)` | All match |
| `any(F)` | Any match |
| `find(F)` | First match |
| `find_map(F)` | First map result |
| `position(F)` | Index of first match |
| `rposition(F)` | Index from end |
| `partition(F)` | Split into 2 |
| `unzip()` | Split tuple iter |
| `try_fold(init, F)` | Fold with `?` |
| `try_for_each(F)` | for_each with `?` |
| `eq(other)` | Sequence equal |
| `cmp(other)` | Compare |
| `sum_by(F)` | (via fold) |

## 10.3 Common combinations

```rust
// Group-like (no built-in, use fold or itertools):
items.iter()
    .fold(HashMap::<String, Vec<_>>::new(), |mut map, item| {
        map.entry(item.category.clone()).or_default().push(item);
        map
    });

// Count occurrences:
let counts: HashMap<&str, usize> = items.iter()
    .fold(HashMap::new(), |mut m, x| { *m.entry(*x).or_insert(0) += 1; m });

// Unique elements (preserve order requires Vec, no order use HashSet):
use std::collections::HashSet;
let unique: Vec<i32> = items.into_iter()
    .scan(HashSet::new(), |seen, x| Some(seen.insert(x).then_some(x)))
    .flatten()
    .collect();
```

---

# Tầng 11: itertools — Power-ups extra

## 11.1 itertools crate

```toml
[dependencies]
itertools = "0.13"
```

```rust
use itertools::Itertools;

let v = vec![1, 2, 3, 4, 5];
```

Adds 100+ methods on top of `Iterator`.

## 11.2 chunks và tuples

```rust
let v: Vec<Vec<i32>> = (1..=10).chunks(3).into_iter()
    .map(|c| c.collect())
    .collect();
// [[1,2,3], [4,5,6], [7,8,9], [10]]

let pairs: Vec<(i32, i32)> = (1..=6).tuples().collect();
// [(1,2), (3,4), (5,6)]
```

## 11.3 group_by

```rust
use itertools::Itertools;

let data = vec![1, 1, 2, 2, 2, 3, 1, 1];
let groups: Vec<(i32, Vec<i32>)> = data.into_iter()
    .group_by(|&x| x)
    .into_iter()
    .map(|(k, g)| (k, g.collect()))
    .collect();
// [(1, [1, 1]), (2, [2, 2, 2]), (3, [3]), (1, [1, 1])]
```

(Consecutive grouping. Need sort first for true grouping.)

## 11.4 dedup

```rust
let v: Vec<i32> = vec![1, 1, 2, 3, 3, 1].into_iter().dedup().collect();
// [1, 2, 3, 1] — consecutive duplicates removed
```

## 11.5 sorted, sorted_by

```rust
let sorted: Vec<i32> = vec![3, 1, 4, 1, 5].into_iter().sorted().collect();
// [1, 1, 3, 4, 5]

let sorted_desc: Vec<i32> = vec![3, 1, 4].into_iter()
    .sorted_by(|a, b| b.cmp(a))
    .collect();
// [4, 3, 1]
```

## 11.6 unique

```rust
let unique: Vec<i32> = vec![1, 2, 1, 3, 2].into_iter().unique().collect();
// [1, 2, 3] — preserves first occurrence order
```

## 11.7 cartesian_product

```rust
let cross: Vec<(i32, char)> = (1..=3)
    .cartesian_product("ab".chars())
    .collect();
// [(1,'a'),(1,'b'),(2,'a'),(2,'b'),(3,'a'),(3,'b')]
```

## 11.8 itertools::izip

```rust
use itertools::izip;

let a = vec![1, 2, 3];
let b = vec!['a', 'b', 'c'];
let c = vec![true, false, true];

for (x, y, z) in izip!(&a, &b, &c) {
    println!("{} {} {}", x, y, z);
}
```

Zip 3+ iterators.

## 11.9 minmax

```rust
use itertools::Itertools;

if let itertools::MinMaxResult::MinMax(min, max) = vec![3, 1, 4, 1, 5].iter().minmax() {
    println!("min: {}, max: {}", min, max);
}
// min: 1, max: 5
```

1 pass thay 2 passes.

## 11.10 fold1

```rust
let max: Option<i32> = vec![3, 1, 4].into_iter().fold1(|a, b| a.max(b));
// Some(4)
```

Like `reduce`.

## 11.11 Khi nào dùng itertools?

- Code lặp dùng chunks, tuples, groups → use itertools
- Cần unique, dedup, sorted in 1 step
- Multi-iterator combinations (cartesian, izip)

itertools = "iterator standard library extension". Recommended for non-trivial iterator work.

---

# Tầng 12: Parallel iterator — rayon

## 12.1 rayon crate

```toml
[dependencies]
rayon = "1"
```

```rust
use rayon::prelude::*;

let v: Vec<i32> = (1..=1_000_000).collect();
let sum: i64 = v.par_iter().map(|&x| x as i64).sum();
```

`par_iter()` thay `iter()` → tự động chia work across CPU cores.

## 12.2 ParallelIterator trait

Most Iterator methods available on parallel version:
- `par_iter()`, `into_par_iter()`, `par_iter_mut()`
- `par_chunks()`, `par_windows()` (slice)
- Methods: `map`, `filter`, `fold`, `reduce`, `sum`, `collect`, ...

## 12.3 Auto work-stealing

```
Thread pool (1 per CPU core):
  Worker 0  Worker 1  Worker 2  Worker 3
     │         │         │         │
     │  Each gets chunk of work    │
     │  Steal work from busier ones│
```

Rayon dynamic load balancing → 8 cores → ~7-8x speedup cho CPU-bound.

## 12.4 Conversion sequential ↔ parallel

```rust
// Sequential:
let result: Vec<i32> = (0..1_000_000)
    .map(expensive_compute)
    .filter(|x| x > 0)
    .collect();

// Parallel (1 line change!):
let result: Vec<i32> = (0..1_000_000)
    .into_par_iter()        // ← parallel
    .map(expensive_compute)
    .filter(|&x| x > 0)
    .collect();
```

Beautiful — minimal code change for parallelism.

## 12.5 Khi nào dùng rayon?

✅ Tốt:
- CPU-bound work mỗi item (parsing, hash, math)
- Large collections (>1k items)
- Independent items (no shared mutable state)

❌ Xấu:
- Small data (parallel overhead > work)
- I/O bound (cần async)
- Items dependent on each other (synchronization overhead)

Rule of thumb: per-item work > ~1µs → parallel benefit.

## 12.6 Performance considerations

```rust
// ❌ Tiny work — parallel overhead dominates
let sum: i32 = (1..=100).into_par_iter().sum();   // SLOWER than sequential

// ✅ Heavy work
let result: Vec<_> = files.par_iter()
    .map(|f| process_large_file(f))   // each takes ms
    .collect();
```

Test với criterion để verify parallel actually faster.

## 12.7 Mutable shared state — Anti-pattern

```rust
// ❌ Race condition risk
let mut total = 0;
v.par_iter().for_each(|x| total += x);   // compile error: not Sync

// ✅ Use sum/reduce
let total: i32 = v.par_iter().sum();

// ✅ Or atomic
let total = AtomicI32::new(0);
v.par_iter().for_each(|x| { total.fetch_add(*x, Ordering::Relaxed); });
let result = total.load(Ordering::Relaxed);
```

Compile-time prevent data race trong rayon.

## 12.8 par_iter trên collections

```rust
use rayon::prelude::*;

let v: Vec<i32> = vec![1, 2, 3, 4];
v.par_iter();          // &i32
v.par_iter_mut();      // &mut i32
v.into_par_iter();     // i32

// HashMap, HashSet, BTreeMap, etc. all support par_iter
```

## 12.9 par_bridge — Sequential iter → parallel

```rust
use rayon::prelude::*;
use rayon::iter::ParallelBridge;

let iter = (1..=100).filter(|x| x % 2 == 0);  // sequential iter
let sum: i32 = iter.par_bridge().sum();        // run in parallel
```

`par_bridge` wraps regular Iterator to parallelize. Less efficient than native par_iter — only when source isn't parallel-native.

---

# Tầng 13: Stream — Async iterator

## 13.1 Stream là gì?

`Stream` = async version of Iterator. Each `next()` returns a Future.

```rust
use futures::stream::{Stream, StreamExt};

pub trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
}
```

(Iterator returns `Option<T>` sync. Stream returns `Poll<Option<T>>`.)

## 13.2 Sử dụng

```rust
use futures::stream::{self, StreamExt};

let mut s = stream::iter(vec![1, 2, 3]);
while let Some(item) = s.next().await {
    println!("{}", item);
}
```

## 13.3 Stream combinators

Most Iterator methods có async equivalent:
- `map`, `filter`, `take`, `skip`, ...
- `for_each(F: FnMut -> Future)`
- `collect::<Vec<_>>()`
- `fold(init, F: FnMut -> Future)`

```rust
let result: Vec<i32> = stream::iter(1..=10)
    .filter(|&x| async move { x % 2 == 0 })
    .map(|x| async move { x * 2 })
    .buffer_unordered(4)   // run 4 at once
    .collect()
    .await;
```

`buffer_unordered(n)` concurrent execution của n futures.

## 13.4 Use cases

- Reading file lines async (`tokio::io::BufReader::lines()`)
- WebSocket messages
- Kafka/PubSub consumer
- HTTP server requests
- Database query rows

## 13.5 async-stream crate

```rust
use async_stream::stream;

let s = stream! {
    for i in 0..3 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        yield i;
    }
};

futures::pin_mut!(s);
while let Some(item) = s.next().await {
    println!("{}", item);
}
```

Generator-like syntax for streams.

## 13.6 Stream vs Iterator vs ParallelIterator

```
   ┌──────────────────┬───────────────────────────────────┐
   │ Iterator         │ Sync, sequential                  │
   │                  │ for x in iter { ... }             │
   ├──────────────────┼───────────────────────────────────┤
   │ ParallelIterator │ Sync, parallel (rayon)            │
   │                  │ items.par_iter().map(...)         │
   ├──────────────────┼───────────────────────────────────┤
   │ Stream           │ Async, sequential                 │
   │                  │ while let Some(x) = s.next().await│
   └──────────────────┴───────────────────────────────────┘
```

Choose based on:
- CPU-bound + parallel → `rayon`
- I/O-bound + concurrent → `Stream`
- Simple sequential → `Iterator`

---

# Tầng 14: Performance — Zero-cost trong thực tế

## 14.1 Iterator chains compile to same asm as loops

```rust
fn iter_sum(v: &[i32]) -> i32 {
    v.iter().sum()
}

fn loop_sum(v: &[i32]) -> i32 {
    let mut s = 0;
    for x in v { s += x; }
    s
}
```

Cả 2 compile thành **same assembly** (with `--release`):
- Vectorize (SIMD)
- Loop unroll
- Bounds check elimination

→ Iterator chains "zero-cost".

## 14.2 Compiler tricks

```rust
// Bounds check elimination:
v.iter()           // ← guaranteed in bounds → no check
    .map(|&x| x * 2)
    .collect()

// vs
for i in 0..v.len() {
    result.push(v[i] * 2);  // ← may have bounds check
}
```

Iter often **faster** than index-based loop because of bounds check elimination.

## 14.3 Vectorize-friendly patterns

```rust
// Good — compiler auto-vectorize
let sum: i32 = v.iter().sum();
let result: Vec<f32> = v.iter().map(|x| x * 2.0).collect();

// Bad — early exit prevents vectorize
v.iter().take_while(|&&x| x > 0).sum::<i32>();

// Mediocre — closure captures local var, harder to optimize
let factor = 2;
v.iter().map(|x| x * factor).sum::<i32>();
```

Test với `cargo asm` để verify.

## 14.4 collect() with size_hint

```rust
// With ExactSizeIterator (Vec, range):
let v: Vec<i32> = (1..=1000).collect();   // 1 alloc, no realloc

// Without:
let v: Vec<i32> = (1..).take_while(|&x| x <= 1000).collect();
// May realloc (size unknown), but usually 1-2 reallocs only
```

## 14.5 Iterator chain reuses memory

```rust
// ❌ Alloc Vec mỗi step
let v1: Vec<i32> = (1..=100).map(|x| x * 2).collect();
let v2: Vec<i32> = v1.iter().filter(|&&x| x > 50).collect();
let v3: Vec<i32> = v2.iter().take(10).collect();
// 3 allocs

// ✅ Chain in 1 pass
let v: Vec<i32> = (1..=100)
    .map(|x| x * 2)
    .filter(|&x| x > 50)
    .take(10)
    .collect();
// 1 alloc, lazy compute
```

## 14.6 Avoid collect intermediate

```rust
// ❌
let lengths: Vec<usize> = words.iter().map(|w| w.len()).collect();
let total: usize = lengths.iter().sum();

// ✅
let total: usize = words.iter().map(|w| w.len()).sum();
```

Don't `collect` unless needed.

## 14.7 cloned vs copied

```rust
// .cloned() works for any T: Clone (potentially expensive)
let v: Vec<String> = vec!["a".to_string()].iter().cloned().collect();

// .copied() only for T: Copy (always cheap, just memcpy)
let v: Vec<i32> = vec![1, 2, 3].iter().copied().collect();
```

Prefer `.copied()` cho primitive types — clearer intent + slightly faster.

## 14.8 fold cheaper than reduce sometimes

Differences subtle. Both compile efficiently. Use whichever clearer:
- `fold(init, |acc, x| ...)` when initial state matters
- `reduce(|a, b| ...)` when accumulator type = item type

## 14.9 Benchmark!

Always measure. Examples expected zero-cost may not be in your context. Use `criterion`:

```rust
fn bench(c: &mut Criterion) {
    let v: Vec<i32> = (0..1_000_000).collect();
    
    c.bench_function("iter sum", |b| b.iter(|| {
        v.iter().sum::<i32>()
    }));
    
    c.bench_function("loop sum", |b| b.iter(|| {
        let mut s = 0;
        for x in &v { s += x; }
        s
    }));
}
```

Often: iter == loop, sometimes iter faster, occasionally loop faster.

---

# Tầng 15: Patterns và Antipatterns

## 15.1 ✅ Pattern: Use iterator over index-based loop

```rust
// ❌
for i in 0..v.len() {
    process(&v[i]);
}

// ✅
for item in &v {
    process(item);
}

// ✅✅
v.iter().for_each(process);
```

## 15.2 ✅ Pattern: filter_map for parse + skip

```rust
let nums: Vec<i32> = inputs.iter()
    .filter_map(|s| s.parse().ok())
    .collect();
```

## 15.3 ✅ Pattern: Collect Result<Vec<T>, E>

```rust
let parsed: Result<Vec<i32>, _> = inputs.iter()
    .map(|s| s.parse::<i32>())
    .collect();
// Err nếu bất kỳ parse fails
```

## 15.4 ✅ Pattern: Try-fold for error short-circuit

```rust
let total: Result<i32, _> = nums.iter()
    .try_fold(0, |acc, &x| {
        if x < 0 { return Err("negative"); }
        Ok(acc + x)
    });
```

## 15.5 ✅ Pattern: Chain to combine

```rust
let all_items: Vec<&Item> = group_a.iter()
    .chain(group_b.iter())
    .chain(group_c.iter())
    .collect();
```

## 15.6 ✅ Pattern: enumerate when need index

```rust
for (i, item) in items.iter().enumerate() {
    if i % 100 == 0 { println!("progress {}", i); }
    process(item);
}
```

## 15.7 ✅ Pattern: peekable for parser

```rust
let mut iter = tokens.iter().peekable();
while let Some(t) = iter.next() {
    if iter.peek() == Some(&&Token::Semicolon) {
        // end of statement
    }
}
```

## 15.8 ❌ Antipattern: Collect intermediate

```rust
// ❌
let evens: Vec<i32> = v.iter().filter(|&&x| x % 2 == 0).cloned().collect();
let sum: i32 = evens.iter().sum();

// ✅
let sum: i32 = v.iter().filter(|&&x| x % 2 == 0).sum();
```

## 15.9 ❌ Antipattern: clone instead of borrow

```rust
// ❌
let doubled: Vec<i32> = v.iter().cloned().map(|x| x * 2).collect();

// ✅
let doubled: Vec<i32> = v.iter().map(|&x| x * 2).collect();
```

## 15.10 ❌ Antipattern: into_iter() khi cần &T

```rust
let v = vec![1, 2, 3];

// ❌ consumes v unnecessarily
let sum: i32 = v.into_iter().sum();
println!("{:?}", v);   // ERROR

// ✅ borrows
let sum: i32 = v.iter().sum();
println!("{:?}", v);   // OK
```

## 15.11 ❌ Antipattern: collect rồi for_each

```rust
// ❌ Allocates Vec wastefully
v.iter().map(|x| x * 2).collect::<Vec<_>>().iter().for_each(print);

// ✅ for_each directly
v.iter().map(|x| x * 2).for_each(|x| print!("{}", x));
```

## 15.12 ❌ Antipattern: iter mutable state outside

```rust
// ❌ Hard to read, side effects
let mut sum = 0;
v.iter().for_each(|x| sum += x);

// ✅ Use sum/fold
let sum: i32 = v.iter().sum();
```

## 15.13 ❌ Antipattern: Premature par_iter

```rust
// ❌ Tiny data, parallel overhead > benefit
let v: Vec<i32> = (1..=100).collect();
let sum: i32 = v.par_iter().sum();

// ✅ Sequential faster for tiny data
let sum: i32 = v.iter().sum();
```

Benchmark to verify rayon helps.

---

# Tổng kết — 12 nguyên tắc senior

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Iterator = lazy state machine. Không chạy đến khi consume.   │
│                                                                 │
│ 2. Implement Iterator = chỉ định nghĩa next(). 70 methods free. │
│                                                                 │
│ 3. iter (borrow) / iter_mut (mut) / into_iter (consume).        │
│    Chọn theo intent.                                            │
│                                                                 │
│ 4. for loop = sugar cho into_iter() + loop + next().            │
│                                                                 │
│ 5. Iterator chains zero-cost — compile thành loop tối ưu.       │
│                                                                 │
│ 6. filter_map cho parse + skip failures.                        │
│                                                                 │
│ 7. collect::<Result<Vec<_>, E>>() để bail-on-first-err.         │
│                                                                 │
│ 8. fold cho generic reduce, try_fold cho error propagation.     │
│                                                                 │
│ 9. itertools cho chunks, group_by, dedup, sorted, ...           │
│                                                                 │
│ 10. rayon par_iter chỉ khi work > overhead. Test với criterion. │
│                                                                 │
│ 11. Stream cho async, ParallelIterator cho parallel CPU.        │
│                                                                 │
│ 12. Tránh collect intermediate, prefer chain xuyên suốt.        │
└─────────────────────────────────────────────────────────────────┘
```

---

# Crates senior toolkit

| Crate | Mục đích |
|-------|----------|
| `itertools` | Extra iterator methods |
| `rayon` | Parallel iterators |
| `futures` | Stream trait, combinators |
| `tokio-stream` | Tokio Stream extensions |
| `async-stream` | Generator-syntax for Stream |
| `ndarray` | N-dimensional arrays with iter |
| `rayon-hash` | Parallel HashMap iteration |

---

# Lộ trình tiếp theo

Bạn đã có 13 chủ đề:

```
1. memory-model
2. ownership-borrowing
3. trait
4. generic
5. closure
6. async
7. error-handling
8. macros
9. smart-pointers
10. lifetime
11. performance
12. observability
13. iterator           ← MỚI
```

Còn các topic chuyên sâu:

- **Unsafe Rust** — raw pointer, UnsafeCell, atomic ordering, FFI, soundness
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 13 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
