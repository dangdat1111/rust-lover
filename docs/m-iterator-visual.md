# Iterator Rust — Minh Hoạ Trực Quan

> Companion visual cho [iterator.md](./iterator.md). Đọc song song.

---

## 1. Bức tranh lớn — Iterator Universe

```
                          ITERATOR TRONG RUST
       ┌────────────────────────────────────────────────────────┐
       │                                                        │
       │   Iterator = LAZY STATE MACHINE pull-based             │
       │                                                        │
       │   ┌──────────┐    ┌──────────┐    ┌──────────────┐     │
       │   │ Source    │ ─► │ Adapters │ ─► │ Consumer     │    │
       │   │ (iter,    │    │ (lazy)   │    │ (eager,      │    │
       │   │  range,   │    │          │    │  triggers)   │    │
       │   │  custom)  │    │ map      │    │              │    │
       │   │           │    │ filter   │    │ collect      │    │
       │   │           │    │ take     │    │ sum          │    │
       │   │           │    │ chain    │    │ fold         │    │
       │   │           │    │ zip      │    │ for_each     │    │
       │   │           │    │ ...      │    │ ...           │    │
       │   └──────────┘    └──────────┘    └──────────────┘     │
       │                         │                              │
       │                         ▼                              │
       │            ┌────────────────────────────┐              │
       │            │ Implementations:            │              │
       │            │ • Iterator (sync sequential)│              │
       │            │ • ParallelIterator (rayon)  │              │
       │            │ • Stream (async)            │              │
       │            └────────────────────────────┘              │
       │                                                        │
       │   Zero-cost: compile thành loop optimized               │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Iterator trait — Định nghĩa

```
   ┌──────────────────────────────────────────────────────────┐
   │  pub trait Iterator {                                    │
   │      type Item;                                          │
   │                                                          │
   │      fn next(&mut self) -> Option<Self::Item>;           │
   │      //                                                  │
   │      //   Some(value) — còn item                         │
   │      //   None        — hết                              │
   │                                                          │
   │      // ... 70+ default methods                          │
   │      // map, filter, fold, sum, ...                      │
   │  }                                                       │
   └──────────────────────────────────────────────────────────┘
   
   
   Implement = chỉ next(). Mọi method khác FREE.
   ────────────────────────────────────────────────
   
   struct Counter { count: u32, max: u32 }
   
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
   
   // Bây giờ có ngay 70+ methods:
   let v: Vec<u32> = Counter { count: 0, max: 5 }.collect();
   let sum: u32 = Counter { count: 0, max: 5 }.sum();
   Counter { count: 0, max: 5 }.for_each(|x| println!("{}", x));
   // ...
```

---

## 3. for loop = sugar cho next()

```
   ┌────────────────────────────────────────────────────────┐
   │ Code bạn viết:                                         │
   │                                                        │
   │   for x in vec![1, 2, 3] {                             │
   │       println!("{}", x);                               │
   │   }                                                    │
   │                                                        │
   ├────────────────────────────────────────────────────────┤
   │ Compiler expand:                                       │
   │                                                        │
   │   let mut iter = vec![1, 2, 3].into_iter();            │
   │   loop {                                               │
   │       match iter.next() {                              │
   │           Some(x) => { println!("{}", x); }            │
   │           None => break,                               │
   │       }                                                │
   │   }                                                    │
   │                                                        │
   └────────────────────────────────────────────────────────┘
   
   ⟹ Bất cứ type nào impl Iterator → dùng for được.
```

---

## 4. iter / iter_mut / into_iter

```
   ┌──────────────────────────────────────────────────────────┐
   │                                                          │
   │   let v = vec![1, 2, 3];                                 │
   │                                                          │
   │   v.iter()       → Iter<'_, T>      yields &T           │
   │                    (borrow, v stays alive)               │
   │                                                          │
   │   v.iter_mut()   → IterMut<'_, T>   yields &mut T       │
   │                    (mut borrow)                          │
   │                                                          │
   │   v.into_iter()  → IntoIter<T>      yields T            │
   │                    (consume v)                           │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
   
   
   for loop chọn theo expression type:
   ───────────────────────────────────
   
   for x in v          → into_iter()  (T)
   for x in &v         → iter()       (&T)
   for x in &mut v     → iter_mut()   (&mut T)
   
   
   Visual:
   ───────
   
   Vec<T> v = [1, 2, 3]
       │
       ├── v.iter() ──────► gives you &1, &2, &3 (v stays)
       │
       ├── v.iter_mut() ──► gives you &mut 1, &mut 2, &mut 3
       │
       └── v.into_iter() ─► gives you 1, 2, 3 (v gone!)
```

---

## 5. Lazy evaluation — Bí mật quan trọng nhất

```
   ┌────────────────────────────────────────────────────────────┐
   │  Code:                                                     │
   │                                                            │
   │    let iter = v.iter()                                     │
   │        .map(|x| {                                          │
   │            println!("mapping {}", x);                      │
   │            x * 2                                           │
   │        });                                                 │
   │                                                            │
   │    println!("Created iterator");   ← print này TRƯỚC      │
   │                                                            │
   │    for x in iter { println!("Got {}", x); }                │
   │                                                            │
   ├────────────────────────────────────────────────────────────┤
   │  Output:                                                   │
   │                                                            │
   │    Created iterator          ← .map() KHÔNG chạy!         │
   │    mapping 1                                               │
   │    Got 2                                                   │
   │    mapping 2                                               │
   │    Got 4                                                   │
   │    mapping 3                                               │
   │    Got 6                                                   │
   │                                                            │
   └────────────────────────────────────────────────────────────┘
   
   
   Adapters CHỈ TẠO struct describe transformation,
   không apply cho đến khi consume.
   
   
   Pull-based execution:
   ─────────────────────
   
   collect() asks  ←── pull
       take asks   ←── pull
           filter asks
               map asks
                   source.next() ───► output value
                   
                   ▲ value flows up through transformations
```

---

## 6. Lazy chain visualization

```
   let result: Vec<i32> = (1..)
       .map(|x| x * 2)
       .filter(|&x| x > 5)
       .take(3)
       .collect();
   
   
   Execution trace:
   ────────────────
   
   collect: "give me Vec"
       │
       ▼
   take(3): "I need 3 items"
       │
       ▼
   filter: "find next > 5"
       │
       ▼
   map: "transform * 2"
       │
       ▼
   (1..): yield 1 → map: 2 → filter: 2 > 5? no, retry
                                                  │
   (1..): yield 2 → map: 4 → filter: 4 > 5? no, retry
                                                  │
   (1..): yield 3 → map: 6 → filter: 6 > 5? YES
                                                  │
   take: have 1/3, output 6                       │
       │                                          │
       ▼                                          │
   collect: push 6                                │
                                                  │
   ... continue with 4 → 8 → output 8             │
   ... continue with 5 → 10 → output 10           │
   take: 3/3 done → return None                   │
       │                                          │
   collect: return [6, 8, 10]                     │
   
   
   ⟹ Compiler INLINE tất cả → loop tương đương:
   
   let mut result = Vec::new();
   let mut count = 0;
   let mut x = 1;
   while count < 3 {
       let y = x * 2;
       if y > 5 {
           result.push(y);
           count += 1;
       }
       x += 1;
   }
   
   Zero alloc, no struct overhead. Pure loop.
```

---

## 7. Adapters — Categorized cheatsheet

```
   ┌──────────────────────────────────────────────────────────┐
   │              ADAPTERS (return iterator)                  │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  TRANSFORM:                                              │
   │    .map(F)              transform each element           │
   │    .filter(F)           keep matching                    │
   │    .filter_map(F)       map + filter (F → Option<T>)     │
   │    .flat_map(F)         map then flatten                 │
   │    .flatten()           flatten nested iter              │
   │    .scan(init, F)       accumulate + yield each          │
   │    .cloned()            &T → T (clone)                   │
   │    .copied()            &T → T (copy only)               │
   │                                                          │
   │  LIMIT:                                                  │
   │    .take(n)             first n items                    │
   │    .skip(n)             after first n                    │
   │    .take_while(F)       while true                       │
   │    .skip_while(F)       until first true                 │
   │    .step_by(n)          every nth                        │
   │    .map_while(F)        map until None                   │
   │                                                          │
   │  COMBINE:                                                │
   │    .chain(iter)         concatenate                      │
   │    .zip(iter)           pair up                          │
   │    .enumerate()         add index                        │
   │                                                          │
   │  ORDER:                                                  │
   │    .rev()               reverse (needs DoubleEnded)      │
   │    .cycle()             infinite repeat                  │
   │                                                          │
   │  HELPERS:                                                │
   │    .peekable()          add .peek()                      │
   │    .fuse()              stop forever after None          │
   │    .inspect(F)          side-effect, pass-through        │
   │    .by_ref()            borrow iterator                  │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 8. Consumers — Categorized cheatsheet

```
   ┌──────────────────────────────────────────────────────────┐
   │              CONSUMERS (return value)                    │
   ├──────────────────────────────────────────────────────────┤
   │                                                          │
   │  COLLECT INTO COLLECTION:                                │
   │    .collect::<T>()      build Vec/HashMap/String/Result  │
   │                                                          │
   │  REDUCE:                                                 │
   │    .sum::<T>()          total                            │
   │    .product::<T>()      multiply                         │
   │    .count()             element count                    │
   │    .fold(init, F)       general reduce                   │
   │    .reduce(F)           fold without init (→ Option)     │
   │    .try_fold(init, F)   fold with ?                      │
   │                                                          │
   │  FIND / MAX / MIN:                                       │
   │    .max() / .min()                                       │
   │    .max_by_key(F) / .min_by_key(F)                       │
   │    .max_by(F) / .min_by(F)                               │
   │    .find(F)             first match                      │
   │    .find_map(F)         first map result                 │
   │    .position(F)         index of first match             │
   │    .last() / .nth(n)                                     │
   │                                                          │
   │  BOOLEAN:                                                │
   │    .all(F)              all match?                       │
   │    .any(F)              any match?                       │
   │                                                          │
   │  SIDE EFFECT:                                            │
   │    .for_each(F)         apply each                       │
   │    .try_for_each(F)     apply with ?                     │
   │                                                          │
   │  SPLIT:                                                  │
   │    .partition(F)        (Vec, Vec)                       │
   │    .unzip()             (Vec, Vec) from tuples           │
   │                                                          │
   │  COMPARE:                                                │
   │    .eq(other)           sequence equal                   │
   │    .cmp(other)          lexicographic compare            │
   │                                                          │
   └──────────────────────────────────────────────────────────┘
```

---

## 9. map / filter / filter_map flow

```
   map:
   ────
   [1, 2, 3, 4, 5]
        │
        │ |x| x * 2
        ▼
   [2, 4, 6, 8, 10]
   
   ⟹ same length, transformed
   
   
   filter:
   ───────
   [1, 2, 3, 4, 5]
        │
        │ |&x| x % 2 == 0
        ▼
   [2, 4]
   
   ⟹ kept matching, length may decrease
   
   
   filter_map:
   ───────────
   ["1", "2", "abc", "4"]
        │
        │ |s| s.parse().ok()    (Option<i32>)
        ▼
   [Some(1), Some(2), None, Some(4)]
        │
        │ keep only Some
        ▼
   [1, 2, 4]
   
   ⟹ filter + map in one step. Useful for parse + skip.
   
   
   flat_map:
   ─────────
   [[1, 2], [3, 4], [5]]
        │
        │ |v| v.into_iter()
        ▼
   [1, 2, 3, 4, 5]
   
   ⟹ map then flatten nested structure
```

---

## 10. zip / chain / enumerate

```
   zip:
   ────
   ["Alice", "Bob", "Charlie"]
   [30, 25]
        │
        │ zip
        ▼
   [("Alice", 30), ("Bob", 25)]
   
   ⟹ pairs from 2 iterators, stops at SHORTER
   
   
   chain:
   ──────
   [1, 2, 3]
   [4, 5, 6]
        │
        │ chain
        ▼
   [1, 2, 3, 4, 5, 6]
   
   ⟹ concatenate
   
   
   enumerate:
   ──────────
   ['a', 'b', 'c']
        │
        │ enumerate
        ▼
   [(0, 'a'), (1, 'b'), (2, 'c')]
   
   ⟹ add index
```

---

## 11. take / skip / take_while / skip_while

```
   Input:   [1, 2, 3, 4, 5, 6]
   
   .take(3):           [1, 2, 3]
                       └── first 3
   
   .skip(2):           [3, 4, 5, 6]
                       └── skip first 2
   
   .take_while(|&x| x < 4):
                       [1, 2, 3]
                       └── take while true (STOPS at first false)
   
   .skip_while(|&x| x < 4):
                       [4, 5, 6]
                       └── skip while true (STOPS skipping at first false)
   
   .step_by(2):        [1, 3, 5]
                       └── every nth
```

---

## 12. fold visualization

```
   .fold(init, |acc, x| new_acc)
   
   
   Example: sum
   ────────────
   [1, 2, 3, 4].iter().fold(0, |acc, &x| acc + x)
   
   Step      acc       x        new acc
   1         0    +    1     =   1
   2         1    +    2     =   3
   3         3    +    3     =   6
   4         6    +    4     =   10
                                  ▲
                              final result
   
   
   Example: build string
   ─────────────────────
   [1, 2, 3].iter().fold(String::new(), |mut s, &n| {
       s.push_str(&n.to_string());
       s
   })
   
   Step      acc      x        new acc
   1         ""    +  1   =    "1"
   2         "1"   +  2   =    "12"
   3         "12"  +  3   =    "123"
   
   
   Example: tuple accumulator
   ──────────────────────────
   [1, 2, 3].iter().fold((0, 1), |(sum, product), &x| {
       (sum + x, product * x)
   })
   
   Step      acc          x       new acc
   1         (0, 1)    +  1   =   (1, 1)
   2         (1, 1)    +  2   =   (3, 2)
   3         (3, 2)    +  3   =   (6, 6)
                                    ▲
                            sum=6, product=6
```

---

## 13. Implement custom iterator

```
   ┌────────────────────────────────────────────────────────────┐
   │  struct Fib { curr: u64, next: u64 }                       │
   │                                                            │
   │  impl Iterator for Fib {                                   │
   │      type Item = u64;                                      │
   │      fn next(&mut self) -> Option<u64> {                   │
   │          let n = self.curr;                                │
   │          self.curr = self.next;                            │
   │          self.next = n + self.curr;                        │
   │          Some(n)   // infinite!                            │
   │      }                                                     │
   │  }                                                         │
   └────────────────────────────────────────────────────────────┘
   
   
   Usage:
   ──────
   
   let v: Vec<u64> = Fib { curr: 0, next: 1 }
       .take(10)              ← limit infinite to 10
       .collect();
   // [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
   
   
   Iterator pull flow:
   ───────────────────
   
   collect → take(10) → Fib { curr: 0, next: 1 }
                              │
                              ▼ next()
                         output 0; curr=1, next=1
                              │
                              ▼ next()
                         output 1; curr=1, next=2
                              │
                              ▼ next()
                         output 1; curr=2, next=3
                              ...
                              │
                              ▼ after 10 pulls, take returns None
                         (Fib still infinite but unreachable)
```

---

## 14. Extension trait pattern

```
   ┌──────────────────────────────────────────────────────────┐
   │  trait RunningAvgExt: Iterator<Item = f64> + Sized {     │
   │      fn running_avg(self) -> RunningAverage<Self> {      │
   │          RunningAverage {                                │
   │              inner: self,                                │
   │              sum: 0.0,                                   │
   │              count: 0,                                   │
   │          }                                               │
   │      }                                                   │
   │  }                                                       │
   │                                                          │
   │  // Blanket impl for all f64 iterators:                  │
   │  impl<I: Iterator<Item = f64>> RunningAvgExt for I {}    │
   └──────────────────────────────────────────────────────────┘
   
   
   Usage — chain with built-in methods:
   ────────────────────────────────────
   
   let avgs: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0]
       .into_iter()
       .running_avg()           ← custom method!
       .filter(|&x| x > 1.5)
       .collect();
   
   ⟹ Custom iterator method seamless với standard methods.
   ⟹ Pattern: itertools, rayon dùng cách này.
```

---

## 15. itertools — Power-ups

```
   ┌─────────────────────────────────────────────────────────┐
   │  itertools 100+ methods extra:                          │
   │                                                         │
   │  CHUNKING:                                              │
   │    .chunks(n)         group consecutive n               │
   │    .tuples()          [a,b,c,d] → [(a,b),(c,d)]         │
   │                                                         │
   │  GROUPING:                                              │
   │    .group_by(F)       consecutive groups                │
   │    .unique()          remove duplicates                 │
   │    .dedup()           remove consecutive dups           │
   │                                                         │
   │  SORTING:                                               │
   │    .sorted()          sort and yield                    │
   │    .sorted_by(F)      sorted with cmp                   │
   │                                                         │
   │  COMBINATIONS:                                          │
   │    .cartesian_product(other)                            │
   │    .combinations(k)                                     │
   │    .permutations(k)                                     │
   │                                                         │
   │  MULTI-ITER:                                            │
   │    izip!(a, b, c)     zip 3+ iterators                  │
   │    .interleave(other) alternate                         │
   │                                                         │
   │  STATS:                                                 │
   │    .minmax()          min AND max in 1 pass             │
   │    .counts()          HashMap<T, usize>                 │
   │                                                         │
   └─────────────────────────────────────────────────────────┘
   
   
   Example — chunks:
   ─────────────────
   
   (1..=10).chunks(3)
   →  [1, 2, 3] [4, 5, 6] [7, 8, 9] [10]
   
   
   Example — cartesian:
   ────────────────────
   
   (1..=2).cartesian_product("ab".chars())
   →  (1, 'a'), (1, 'b'), (2, 'a'), (2, 'b')
   
   
   Example — izip:
   ───────────────
   
   izip!(&names, &ages, &scores).for_each(|(n, a, s)| {
       println!("{} ({}) → {}", n, a, s);
   });
```

---

## 16. rayon — Parallel iterators

```
   ┌─────────────────────────────────────────────────────────────┐
   │  Sequential:                                                │
   │  ───────────                                                │
   │  let sum: i64 = (1..=1_000_000)                             │
   │      .map(|x| x as i64 * x as i64)                          │
   │      .sum();                                                │
   │                                                             │
   │  Time: 1 thread, ~10ms                                      │
   │                                                             │
   ├─────────────────────────────────────────────────────────────┤
   │  Parallel (rayon):                                          │
   │  ──────────────────                                         │
   │  use rayon::prelude::*;                                     │
   │                                                             │
   │  let sum: i64 = (1..=1_000_000)                             │
   │      .into_par_iter()        ← THE CHANGE                   │
   │      .map(|x| x as i64 * x as i64)                          │
   │      .sum();                                                │
   │                                                             │
   │  Time: 8 cores, ~1.5ms (~7x speedup)                        │
   │                                                             │
   └─────────────────────────────────────────────────────────────┘
   
   
   Cách rayon hoạt động:
   ─────────────────────
   
   Iterator items: [1, 2, 3, ..., 1_000_000]
                        │
                        ▼ split
   ┌──────────┬──────────┬──────────┬──────────┐
   │ chunk 1  │ chunk 2  │ chunk 3  │ chunk 4  │
   │ items    │ items    │ items    │ items    │
   │ 0..250k  │ 250k..500k │ 500k..750k │ 750k..1M │
   └────┬─────┴────┬─────┴────┬─────┴────┬─────┘
        ▼          ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
   │Worker 0│ │Worker 1│ │Worker 2│ │Worker 3│  ← thread pool
   └────┬───┘ └────┬───┘ └────┬───┘ └────┬───┘
        │          │          │          │
        ▼          ▼          ▼          ▼
       sum_1     sum_2      sum_3      sum_4
        │          │          │          │
        └──────────┴──────────┴──────────┘
                          │
                          ▼ combine
                       final sum
   
   
   Work-stealing:
   ──────────────
   
   Worker 0:  [████████]  busy   
   Worker 1:  [██]         done early
   Worker 2:  [████████]  busy
   Worker 3:  [████]       done
              │
              │ Worker 1 STEALS from Worker 0 or 2
              ▼
   Worker 1:  [██]  +  [████]  ← stolen items
   
   ⟹ Dynamic load balancing → tốt cho non-uniform workload
```

---

## 17. Iterator vs ParallelIterator vs Stream

```
   ┌──────────────────────────────────────────────────────────────┐
   │                                                              │
   │   ITERATOR             PARALLEL ITERATOR    STREAM            │
   │   ────────             ──────────────────  ───────            │
   │                                                              │
   │   Sync                 Sync                 Async             │
   │   Sequential           Parallel              Sequential       │
   │                        (work-stealing)       (async)          │
   │                                                              │
   │   for x in iter        items.par_iter()      while let Some(x)│
   │   { ... }              .map(f)               = s.next().await │
   │                                                              │
   │   Trait: Iterator      Trait: ParallelIter   Trait: Stream    │
   │                                                              │
   │   Use:                 Use:                  Use:             │
   │   • CPU light          • CPU heavy           • I/O bound      │
   │   • Stream large data  • Independent items   • Network        │
   │   • Generators         • Sortable splits     • File async     │
   │                                                              │
   └──────────────────────────────────────────────────────────────┘
   
   
   Decision tree:
   ──────────────
   
                What's blocking?
                       │
            ┌──────────┴──────────┐
           CPU                   I/O
            │                     │
       Heavy or small?       Concurrent?
            │                     │
       ┌────┴────┐           ┌────┴────┐
      Heavy    Small        Yes        No
       │        │            │          │
   ParallelIter Iter      Stream     Iterator
   (rayon)                (futures)
```

---

## 18. Performance — Iterator vs Loop

```
   fn iter_sum(v: &[i32]) -> i32 {
       v.iter().sum()
   }
   
   fn loop_sum(v: &[i32]) -> i32 {
       let mut s = 0;
       for x in v { s += x; }
       s
   }
   
   
   Compiled (--release):
   ─────────────────────
   
   ┌─────────────────────────────┐    ┌─────────────────────────────┐
   │ iter_sum asm:               │    │ loop_sum asm:               │
   │   (same instructions)        │    │   (same instructions)       │
   │                              │    │                              │
   │   - Vectorized (SIMD)        │    │   - Vectorized (SIMD)        │
   │   - Loop unrolled            │    │   - Loop unrolled            │
   │   - No bounds check          │    │   - No bounds check (often) │
   └─────────────────────────────┘    └─────────────────────────────┘
   
   Benchmark: identical or iter slightly faster (no bounds check)
   
   
   Iterator chain pipeline:
   ────────────────────────
   
   v.iter().map(f).filter(g).sum()
        │       │       │      │
        └───────┴───────┴──────┘
              ALL INLINED into:
   
   let mut sum = 0;
   for x in v {
       let y = f(x);
       if g(y) { sum += y; }
   }
   
   ⟹ Zero overhead. No intermediate Vec.
```

---

## 19. Common patterns visualized

```
   ✅ Pattern 1: filter_map for parse + skip
   ──────────────────────────────────────────
   
   strs ──► parse → Some/None ──► skip None ──► Vec<T>
        filter_map combines both steps
   
   let nums: Vec<i32> = strs.iter()
       .filter_map(|s| s.parse().ok())
       .collect();
   
   
   ✅ Pattern 2: collect Result<Vec, E>
   ────────────────────────────────────
   
   strs ──► parse → Result<i32, E> ──► fail fast on first Err
   
   let r: Result<Vec<i32>, _> = strs.iter()
       .map(|s| s.parse::<i32>())
       .collect();
   
   ⟹ Bail at first failure, return Err
   
   
   ✅ Pattern 3: try_fold for short-circuit
   ────────────────────────────────────────
   
   let total: Result<i32, _> = nums.iter()
       .try_fold(0, |acc, &x| {
           if x < 0 { return Err("negative"); }
           Ok(acc + x)
       });
   
   
   ✅ Pattern 4: enumerate for index
   ─────────────────────────────────
   
   for (i, item) in items.iter().enumerate() {
       if i % 100 == 0 { progress(i); }
       process(item);
   }
   
   
   ✅ Pattern 5: peekable for parser
   ─────────────────────────────────
   
   let mut iter = tokens.iter().peekable();
   while let Some(t) = iter.next() {
       if iter.peek() == Some(&&Token::Semicolon) {
           iter.next();   // consume semicolon
       }
   }
```

---

## 20. Antipatterns visualized

```
   ❌ 1. Collect intermediate Vec
   ──────────────────────────────
   
   // Bad: 2 allocs
   let evens: Vec<i32> = v.iter().filter(|&&x| x%2==0).cloned().collect();
   let sum: i32 = evens.iter().sum();
   
   // Good: chain
   let sum: i32 = v.iter().filter(|&&x| x%2==0).sum();
   
   
   ❌ 2. Clone when borrow OK
   ──────────────────────────
   
   // Bad: clone each item
   let doubled: Vec<i32> = v.iter().cloned().map(|x| x * 2).collect();
   
   // Good: borrow
   let doubled: Vec<i32> = v.iter().map(|&x| x * 2).collect();
   
   
   ❌ 3. into_iter() khi cần &T
   ────────────────────────────
   
   let v = vec![1, 2, 3];
   let sum: i32 = v.into_iter().sum();   // consumes v
   println!("{:?}", v);   // ERROR: v moved
   
   // Fix: iter()
   let sum: i32 = v.iter().sum();
   println!("{:?}", v);   // OK
   
   
   ❌ 4. collect → for_each
   ────────────────────────
   
   // Bad: alloc Vec just to iterate
   v.iter().map(|x| x*2).collect::<Vec<_>>().iter().for_each(print);
   
   // Good: for_each directly
   v.iter().map(|x| x*2).for_each(|x| print!("{}", x));
   
   
   ❌ 5. Mutable state outside iter
   ────────────────────────────────
   
   // Bad
   let mut sum = 0;
   v.iter().for_each(|x| sum += x);
   
   // Good
   let sum: i32 = v.iter().sum();
   
   
   ❌ 6. par_iter for tiny data
   ────────────────────────────
   
   // Bad — parallel overhead > work
   let sum: i32 = (1..=100).into_par_iter().sum();
   
   // Good — sequential faster
   let sum: i32 = (1..=100).sum();
   
   ⟹ par_iter chỉ benefit khi work > overhead
   ⟹ Test với criterion!
```

---

## 21. Decision tree — Iterator workflow

```
                  Cần process collection?
                          │
                  ┌───────┴────────┐
                 YES               NO → other approach
                  │
                  ▼
              Modify collection?
                  │
            ┌─────┴─────┐
           YES         NO
            │           │
        iter_mut    Need result or just iterate?
                        │
                  ┌─────┴─────┐
                 Result    Iterate only
                  │           │
              iter / into  iter / into → for_each
                  │
              Sync or async?
                  │
            ┌─────┴─────┐
           Sync       Async
            │           │
        CPU heavy?    Stream
        & parallel?
            │
       ┌────┴────┐
      Yes      No
       │        │
   par_iter   iter
   (rayon)
   
   
   Item ownership:
   ───────────────
   
   Need owned T?      ──► .into_iter()  hoặc .cloned()/.copied()
   Just read?          ──► .iter()
   Modify in place?    ──► .iter_mut()
```

---

## 22. Mind map cuối

```
                          ITERATOR
                              │
       ┌──────────┬───────────┼───────────┬────────────┐
       ▼          ▼           ▼           ▼            ▼
    CORE      LAZY          70+        PARALLEL    ASYNC
    TRAIT     EVAL         METHODS     (rayon)     (Stream)
       │          │           │           │            │
   next()      Adapters    Adapters    par_iter    poll_next
   IntoIter    chain       Consumers   work-       buffer_un-
   Item type   compose                  stealing     ordered
   for sugar
   iter vs
   into_iter
   
   
                ┌──────────────────────────────────────┐
                │  CORE INSIGHTS cho SENIOR            │
                │  ───────────────────────────         │
                │                                      │
                │  1. Iterator = lazy state machine   │
                │     Pull-based, không chạy đến      │
                │     khi consume                      │
                │                                      │
                │  2. Zero-cost: chain compile thành  │
                │     loop optimized                   │
                │                                      │
                │  3. Implement = chỉ next(), free 70+ │
                │     methods                          │
                │                                      │
                │  4. iter / iter_mut / into_iter      │
                │     pick by intent                   │
                │                                      │
                │  5. filter_map cho parse+skip        │
                │     collect<Result> cho fail-fast    │
                │     try_fold cho short-circuit       │
                │                                      │
                │  6. itertools cho chunks/group/sorted│
                │                                      │
                │  7. par_iter chỉ khi work > overhead │
                │                                      │
                │  8. Stream cho async, ParallelIter   │
                │     cho parallel CPU                 │
                │                                      │
                │  9. Tránh collect intermediate Vec   │
                │                                      │
                │  10. Benchmark zero-cost claims      │
                └──────────────────────────────────────┘
```

---

## 23. Bộ tài liệu Rust giờ có 13 chủ đề

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
   │      iterator-visual         ← VỪA HOÀN THÀNH           │
   │                                                          │
   │  ──────────────────────────────────────────────────────  │
   │                                                          │
   │   Tổng: 26 files, ~1.45 MB MD                            │
   │                                                          │
   │   🦀 Bộ kỹ năng functional + parallel + async hoàn chỉnh│
   └──────────────────────────────────────────────────────────┘
```

---

## Chủ đề tiếp theo gợi ý

- **Unsafe Rust** — raw pointer, UnsafeCell deep, atomic ordering, FFI, soundness
- **Testing patterns** — unit, integration, proptest, criterion, mocking, fuzz
- **Web framework realistic** — axum project apply 13 chủ đề
- **Database** — sqlx, sea-orm, transaction patterns, connection pool
- **Embedded Rust** — no_std, embassy, real-time

Báo cái nào muốn đào sâu! 🦀⚡
