# Failing Tests After Rerun

Generated from the rerun list of 21 historically failing tests in `--release`.
Current rerun status (2026-02-06): 8 PASS, 13 FAIL.
Updated on 2026-02-06: three `stdlib::uint64::*` tests below were re-run and now pass.
Updated on 2026-02-06: `stdlib::iterator::{intersperse_adapter, peekable_adapter}` and `stdlib::memory::memory_buffer` were re-run and now pass.
Updated on 2026-02-06: `stdlib::iterator::flatten_iterator` now passes.

## Root Cause Categories (2026-02-06)

### A) Type inference / unresolved placeholders at compile time
- `stdlib::iterator::is_sorted_by_comparator`
- `stdlib::iterator::try_fold_adapter`

### B) MIR lowering generic type argument bug
- `stdlib::memory::memory_typed_pointer`
  - `missing type arguments for generic call to nullPointer`
  - `unsupported type: type parameter 'T' not in scope`

### C) Codegen: associated type projection not fully lowered
- `stdlib::dictionary::dictionary_merge_from_pairs`
- `stdlib::iterator::unzip_iterator`
  - both fail with unsupported index access on `AssociatedTypeProjection`

### D) Codegen: monomorphization failures
- `stdlib::array::init_repeating`
- `stdlib::iterator::fuse_and_cycle`
- `stdlib::rcbox::set_value_and_deep_clone`

### E) Codegen: backend verifier failures
- `stdlib::dictionary::dictionary_sum_values`
- `stdlib::iterator::is_sorted_by_key`
- `stdlib::set::set_sum`

### F) Runtime behavior mismatch (program exits non-zero)
- `stdlib::iterator::intersperse_with_adapter` (exit 3)
- `stdlib::views::chars_view_substring` (exit 6)

### G) Runtime memory fault
- `stdlib::memory::memory_raw_pointer` (exit 11 / segfault)

### H) Currently fixed in this rerun set
- `stdlib::iterator::flatten_iterator`
- `stdlib::iterator::intersperse_adapter`
- `stdlib::iterator::peekable_adapter`
- `stdlib::memory::memory_buffer`
- `stdlib::uint64::uint64_bitwidth_and_conversion`
- `stdlib::uint64::uint64_boundaries_and_constants`
- `stdlib::uint64::uint64_overflow_behavior`

## stdlib::array::init_repeating

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/array.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::array::init_repeating' (89297258) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            let arr = std.collections.Array[std.num.Int64](repeating: 7, 5);
            if arr.count != 5 { return 1 }
            if arr(0) != 7 { return 2 }
            if arr(1) != 7 { return 3 }
            if arr(4) != 7 { return 4 }

            // repeating with count 0
            let empty = std.collections.Array[std.num.Int64](repeating: 42, 0);
            if empty.count != 0 { return 5 }

            // repeating with count 1
            let single = std.collections.Array[std.num.Int64](repeating: 99, 1);
            if single.count != 1 { return 6 }
            if single(0) != 99 { return 7 }

            0
        }
    
```

## stdlib::dictionary::dictionary_merge_from_pairs

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/dictionary.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::dictionary::dictionary_merge_from_pairs' (89297415) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // mergeFrom with another dictionary's iter
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);

            var other = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = other.insert(2, 200);
            let _ = other.insert(3, 300);

            dict.mergeFrom(other, uniquingKeysWith: { (old, new) in old + new });
            if dict.count != 3 { return 1 }
            if dict(1).unwrap() != 10 { return 2 }
            if dict(2).unwrap() != 220 { return 3 }
            if dict(3).unwrap() != 300 { return 4 }

            // mergeFrom with "take new" strategy
            var dict2 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict2.insert(1, 10);

            var src = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = src.insert(1, 99);
            let _ = src.insert(2, 200);

            dict2.mergeFrom(src, uniquingKeysWith: { (old, new) in new });
            if dict2.count != 2 { return 5 }
            if dict2(1).unwrap() != 99 { return 6 }
            if dict2(2).unwrap() != 200 { return 7 }

            // mergeFrom with empty source - no change
            var dict3 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict3.insert(1, 10);
            let emptySrc = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            dict3.mergeFrom(emptySrc, uniquingKeysWith: { (old, new) in new });
            if dict3.count != 1 { return 8 }
            if dict3(1).unwrap() != 10 { return 9 }

            0
        }
    
```

## stdlib::dictionary::dictionary_sum_values

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/dictionary.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::dictionary::dictionary_sum_values' (89297842) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // sumValues() returns sum of all values
            let total = dict.sumValues();
            if total != 60 { return 1 }

            // sumValues on empty dictionary returns default (0 for Int64)
            let emptyDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let emptySum = emptyDict.sumValues();
            if emptySum != 0 { return 2 }

            // sumValues on single entry
            var singleDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = singleDict.insert(1, 42);
            if singleDict.sumValues() != 42 { return 3 }

            // sumValues after mutation
            let _ = dict.insert(4, 40);
            if dict.sumValues() != 100 { return 4 }

            0
        }
    
```

## stdlib::iterator::flatten_iterator

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::iterator::flatten_iterator -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // Flatten nested iterators
            var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
            var inner1 = std.collections.Array[std.num.Int64]();
            inner1.append(1);
            inner1.append(2);
            var inner2 = std.collections.Array[std.num.Int64]();
            inner2.append(3);
            inner2.append(4);
            var inner3 = std.collections.Array[std.num.Int64]();
            inner3.append(5);
            nested.append(inner1);
            nested.append(inner2);
            nested.append(inner3);

            let flat = nested.iter().map({ (arr) in arr.iter() }).flatten().collect();
            if flat.count != 5 { return 1 }
            if flat(unchecked: 0) != 1 { return 2 }
            if flat(unchecked: 4) != 5 { return 3 }

            0
        }
    
```

## stdlib::iterator::fuse_and_cycle

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::iterator::fuse_and_cycle' (89298194) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // ---- fuse() ----
            let fused: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().fuse().collect();
            if fused.count != 3 { return 1 }
            if fused(unchecked: 0) != 1 { return 2 }
            if fused(unchecked: 2) != 3 { return 3 }

            // ---- cycle() + take() ----
            let cycled: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().cycle().take(7).collect();
            if cycled.count != 7 { return 4 }
            if cycled(unchecked: 0) != 1 { return 5 }
            if cycled(unchecked: 3) != 1 { return 6 }
            if cycled(unchecked: 6) != 1 { return 7 }

            0
        }
    
```

## stdlib::iterator::intersperse_adapter

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::iterator::intersperse_adapter -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // ---- intersperse() ----
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().intersperse(0).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator
            let single: std.collections.Array[std.num.Int64] = [42].iter().intersperse(0).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty - stays empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().intersperse(0).collect();
            if emptyResult.count != 0 { return 9 }

            0
        }
    
```

## stdlib::iterator::intersperse_with_adapter

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::iterator::intersperse_with_adapter' (89298663) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // intersperseWith: lazy separator via closure
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().intersperseWith({ () in 0 }).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator generated
            let single: std.collections.Array[std.num.Int64] = [42].iter().intersperseWith({ () in 0 }).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty iterator - stays empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().intersperseWith({ () in 99 }).collect();
            if emptyResult.count != 0 { return 9 }

            // intersperseWith with varying separator (counter-based)
            // Note: cannot use mutable closure captures, so use a constant separator
            let result2: std.collections.Array[std.num.Int64] = [10, 20, 30].iter().intersperseWith({ () in -1 }).collect();
            if result2.count != 5 { return 10 }
            if result2(unchecked: 0) != 10 { return 11 }
            if result2(unchecked: 1) != -1 { return 12 }
            if result2(unchecked: 2) != 20 { return 13 }
            if result2(unchecked: 3) != -1 { return 14 }
            if result2(unchecked: 4) != 30 { return 15 }

            0
        }
    
```

## stdlib::iterator::is_sorted_by_comparator

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `error: could not infer type for 14 placeholder(s)`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // isSorted(by:) with a custom comparator
            // Check descending order: a >= b means "a comes before b"
            if [5, 4, 3, 2, 1].iter().isSorted(by: { (a, b) in a >= b }) == false { return 1 }

            // Ascending is not sorted in descending order
            if [1, 2, 3, 4, 5].iter().isSorted(by: { (a, b) in a >= b }) { return 2 }

            // Check sorted by absolute value
            if [-1, 2, -3, 4].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) == false { return 3 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) { return 4 }

            // Empty iterator is sorted by any comparator
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSorted(by: { (a, b) in false }) == false { return 5 }

            // Single element is sorted by any comparator
            if [42].iter().isSorted(by: { (a, b) in false }) == false { return 6 }

            // Equal elements - ascending comparator
            if [3, 3, 3].iter().isSorted(by: { (a, b) in a <= b }) == false { return 7 }

            0
        }
    
```

## stdlib::iterator::is_sorted_by_key

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::iterator::is_sorted_by_key' (89299214) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // isSortedBy(key:) checks if elements are sorted by extracted key ascending

            // Sorted by absolute value
            if [-1, 2, -3, 4].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) == false { return 1 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) { return 2 }

            // Sorted by negation (effectively descending by value)
            if [5, 4, 3, 2, 1].iter().isSortedBy({ (x) in 0 - x }) == false { return 3 }

            // Not sorted by negation
            if [1, 2, 3].iter().isSortedBy({ (x) in 0 - x }) { return 4 }

            // Empty - always sorted
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSortedBy({ (x) in x }) == false { return 5 }

            // Single element - always sorted
            if [42].iter().isSortedBy({ (x) in x }) == false { return 6 }

            // Identity key - same as isSorted()
            if [1, 2, 3, 4, 5].iter().isSortedBy({ (x) in x }) == false { return 7 }
            if [1, 3, 2].iter().isSortedBy({ (x) in x }) { return 8 }

            0
        }
    
```

## stdlib::iterator::peekable_adapter

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::iterator::peekable_adapter -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);

            // ---- peekable() ----
            var iter = arr.iter().peekable();

            // Peek doesn't consume
            let p1 = iter.peek();
            if p1.isNone() { return 1 }
            if p1.unwrap() != 1 { return 2 }

            // Peek again returns same value
            let p2 = iter.peek();
            if p2.unwrap() != 1 { return 3 }

            // next() consumes
            let n1 = iter.next();
            if n1.unwrap() != 1 { return 4 }

            // Peek now shows next element
            let p3 = iter.peek();
            if p3.unwrap() != 2 { return 5 }

            // Consume remaining
            iter.next();
            iter.next();
            let pEnd = iter.peek();
            if pEnd.isSome() { return 6 }

            0
        }
    
```

## stdlib::iterator::try_fold_adapter

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `error: could not infer type for 2 placeholder(s)`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // tryFold: fold where combine returns Result
            // Successful fold - all Ok
            let result = [1, 2, 3, 4].iter().tryFold(initial: 0, combine: { (acc, x) in
                .Ok(acc + x)
            });
            match result {
                .Ok(v) => { if v != 10 { return 1 } },
                .Err(_) => { return 2 }
            }

            // tryFold with early exit on error
            let earlyExit = [1, 2, 3, 4, 5].iter().tryFold(initial: 0, combine: { (acc, x) in
                if acc > 3 {
                    let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(acc);
                    err
                } else {
                    .Ok(acc + x)
                }
            });
            match earlyExit {
                .Ok(_) => { return 3 },
                .Err(e) => { if e != 6 { return 4 } }
            }

            // tryFold on empty iterator returns Ok(initial)
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().tryFold(initial: 42, combine: { (acc, x) in
                .Ok(acc + x)
            });
            match emptyResult {
                .Ok(v) => { if v != 42 { return 5 } },
                .Err(_) => { return 6 }
            }

            // tryFold that errors on first element
            let firstErr = [1, 2, 3].iter().tryFold(initial: 0, combine: { (acc, x) in
                let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(-1);
                err
            });
            match firstErr {
                .Ok(_) => { return 7 },
                .Err(e) => { if e != -1 { return 8 } }
            }

            0
        }
    
```

## stdlib::iterator::unzip_iterator

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/iterator.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::iterator::unzip_iterator' (89299768) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // Test unzip on iterator of tuples
            var pairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            pairs.append((1, 10));
            pairs.append((2, 20));
            pairs.append((3, 30));

            let (left, right) = pairs.iter().unzip();
            if left.count != 3 { return 1 }
            if right.count != 3 { return 2 }
            if left(unchecked: 0) != 1 { return 3 }
            if left(unchecked: 1) != 2 { return 4 }
            if left(unchecked: 2) != 3 { return 5 }
            if right(unchecked: 0) != 10 { return 6 }
            if right(unchecked: 1) != 20 { return 7 }
            if right(unchecked: 2) != 30 { return 8 }

            // Unzip empty
            let emptyPairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            let (emptyLeft, emptyRight) = emptyPairs.iter().unzip();
            if emptyLeft.count != 0 { return 9 }
            if emptyRight.count != 0 { return 10 }

            0
        }
    
```

## stdlib::memory::memory_buffer

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/memory.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::memory::memory_buffer -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // Create a buffer with SystemAllocator
            var alloc = std.memory.SystemAllocator();
            var buf = std.memory.Buffer[std.num.Int64, std.memory.SystemAllocator](10, alloc);

            // Test capacity
            if buf.capacity != 10 { return 1 }

            // Test pointer is non-null
            if buf.pointer.isNull { return 2 }

            // Test write(unchecked:value:) and read(unchecked:)
            buf.write(unchecked: 0, 42);
            buf.write(unchecked: 1, 99);
            buf.write(unchecked: 2, 77);
            if buf.read(unchecked: 0) != 42 { return 3 }
            if buf.read(unchecked: 1) != 99 { return 4 }
            if buf.read(unchecked: 2) != 77 { return 5 }

            // Test write(at:value:) with bounds checking
            let ok1 = buf.write(at: 5, 55);
            if ok1 == false { return 6 }
            if buf.read(unchecked: 5) != 55 { return 7 }

            // Test write(at:value:) out of bounds
            let oob = buf.write(at: 100, 0);
            if oob { return 8 }

            // Test write(at:value:) negative index
            let neg = buf.write(at: -1, 0);
            if neg { return 9 }

            // Test read(at:) with bounds checking
            let r1 = buf.read(at: 0);
            if r1.isNone() { return 10 }
            if r1.unwrap() != 42 { return 11 }

            // Test read(at:) out of bounds
            let rOob = buf.read(at: 100);
            if rOob.isSome() { return 12 }

            // Test read(at:) negative index
            let rNeg = buf.read(at: -1);
            if rNeg.isSome() { return 13 }

            // Test asSlice
            let sl = buf.asSlice();
            if sl.count != 10 { return 14 }
            if sl(unchecked: 0) != 42 { return 15 }
            if sl(unchecked: 1) != 99 { return 16 }

            // Test slice(from:to:) - valid range
            let sub = buf.slice(from: 0, to: 3);
            if sub.isNone() { return 17 }
            if sub.unwrap().count != 3 { return 18 }
            if sub.unwrap()(unchecked: 0) != 42 { return 19 }

            // Test slice(from:to:) - out of bounds
            let subOob = buf.slice(from: 0, to: 100);
            if subOob.isSome() { return 20 }

            // Test resize
            buf.resize(to: 20);
            if buf.capacity != 20 { return 21 }
            // Data should be preserved after resize
            if buf.read(unchecked: 0) != 42 { return 22 }
            if buf.read(unchecked: 1) != 99 { return 23 }
            if buf.read(unchecked: 2) != 77 { return 24 }

            // Can write to the expanded region
            buf.write(unchecked: 15, 123);
            if buf.read(unchecked: 15) != 123 { return 25 }

            0
        }
    
```

## stdlib::memory::memory_raw_pointer

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/memory.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::memory::memory_raw_pointer' (89300116) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // Test nilPointer
            let nil = std.memory.RawPointer.nilPointer();
            if nil.isNull == false { return 1 }

            // Test address of nil pointer is 0
            let zeroAddr: std.num.UInt64 = 0;
            if nil.address != zeroAddr { return 2 }

            // Create a non-null pointer from an array
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(42);
            let ptr = arr.asPointer();
            let raw = ptr.asRaw();

            // Non-null pointer
            if raw.isNull { return 3 }

            // Address should be non-zero
            let zeroCheck: std.num.UInt64 = 0;
            if raw.address == zeroCheck { return 4 }

            // Test equals - same pointer should be equal
            if raw.equals(raw) == false { return 5 }

            // Test equals - nil vs non-nil should not be equal
            if raw.equals(nil) { return 6 }

            // Test offset
            let offsetPtr = raw.offset(by: 8);
            if offsetPtr.isNull { return 7 }
            // The offset pointer should not equal the original
            if offsetPtr.equals(raw) { return 8 }

            // Test cast to typed pointer
            let typedPtr = raw.cast[std.num.Int64]();
            if typedPtr.isNull { return 9 }
            // Read through the casted pointer should give the array element
            if typedPtr.read() != 42 { return 10 }

            // Test init(address:) round-trip
            let addr = raw.address;
            let fromAddr = std.memory.RawPointer(address: addr);
            if fromAddr.equals(raw) == false { return 11 }

            0
        }
    
```

## stdlib::memory::memory_typed_pointer

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/memory.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::memory::memory_typed_pointer' (89300478) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // Test nullPointer
            let nullPtr = std.memory.Pointer[std.num.Int64].nullPointer();
            if nullPtr.isNull == false { return 1 }

            // Test address of null pointer is 0
            let zeroAddr: std.num.UInt64 = 0;
            if nullPtr.address != zeroAddr { return 2 }

            // Create a typed pointer from an array
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(100);
            arr.append(200);
            arr.append(300);
            let ptr = arr.asPointer();

            // Non-null pointer
            if ptr.isNull { return 3 }

            // Test read
            if ptr.read() != 100 { return 4 }

            // Test offset and read
            let ptr1 = ptr.offset(by: 1);
            if ptr1.read() != 200 { return 5 }

            let ptr2 = ptr.offset(by: 2);
            if ptr2.read() != 300 { return 6 }

            // Test write
            ptr1.write(999);
            if ptr1.read() != 999 { return 7 }
            // Verify the array was modified through the pointer
            if arr(unchecked: 1) != 999 { return 8 }

            // Test equals
            if ptr.equals(ptr) == false { return 9 }
            if ptr.equals(ptr1) { return 10 }

            // Test asRaw
            let raw = ptr.asRaw();
            if raw.isNull { return 11 }
            if raw.address != ptr.address { return 12 }

            // Test address round-trip
            let addr = ptr.address;
            let zeroCheck: std.num.UInt64 = 0;
            if addr == zeroCheck { return 13 }

            0
        }
    
```

## stdlib::rcbox::set_value_and_deep_clone

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/rcbox.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::rcbox::set_value_and_deep_clone' (89300939) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // ---- setValue() ----
            let box1 = std.memory.RcBox[std.num.Int64](10);
            box1.setValue(42);
            if box1.getValue() != 42 { return 1 }

            // setValue on shared box affects both references
            let box2 = box1.clone();
            box1.setValue(99);
            if box2.getValue() != 99 { return 2 }

            // ---- deepClone() ----
            let box3 = std.memory.RcBox[std.num.Int64](50);
            let box4 = box3.deepClone();

            // Deep clone creates independent storage
            if box4.getValue() != 50 { return 3 }
            if box3.refCount() != 1 { return 4 }
            if box4.refCount() != 1 { return 5 }

            // Mutating deep clone doesn't affect original
            box4.setValue(100);
            if box3.getValue() != 50 { return 6 }
            if box4.getValue() != 100 { return 7 }

            0
        }
    
```

## stdlib::set::set_sum

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/set.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::set::set_sum' (89301248) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);

            let total = s.sum();
            if total != 6 { return 1 }

            // Empty set sum
            let empty = std.collections.Set[std.num.Int64]();
            let emptySum = empty.sum();
            if emptySum != 0 { return 2 }

            // Single element
            var single = std.collections.Set[std.num.Int64]();
            let _ = single.insert(42);
            if single.sum() != 42 { return 3 }

            0
        }
    
```

## stdlib::uint64::uint64_bitwidth_and_conversion

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/uint64.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::uint64::uint64_bitwidth_and_conversion -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // byteSwapped — 8-byte swap
            // 1 as u64 (0x0000000000000001) -> 0x0100000000000000 (72057594037927936)
            let one: std.num.UInt64 = 1;
            let lit72057594037927936: std.num.UInt64 = 72057594037927936;
            if one.byteSwapped.equals(lit72057594037927936) == false { return 1 }

            // byteSwapped — double swap is identity
            let val: std.num.UInt64 = 123456789;
            if val.byteSwapped.byteSwapped.equals(val) == false { return 2 }

            // leadingZeros — relative to 64-bit width
            if one.leadingZeros != 63 { return 3 }

            let zero: std.num.UInt64 = 0;
            if zero.leadingZeros != 64 { return 4 }

            // Value with high bit set: 2^63 = 9223372036854775808
            let highBit: std.num.UInt64 = 9223372036854775808;
            if highBit.leadingZeros != 0 { return 5 }

            // rotateLeft — 64-bit rotation
            // rotateLeft(1, by: 1) = 2
            let rotTwo: std.num.UInt64 = 2;
            if one.rotateLeft(by: 1).equals(rotTwo) == false { return 6 }
            // rotateLeft(highBit, by: 1) = 1 (wraps around from bit 63 to bit 0)
            if highBit.rotateLeft(by: 1).equals(one) == false { return 7 }

            // rotateRight — 64-bit rotation
            // rotateRight(2, by: 1) = 1
            let two: std.num.UInt64 = 2;
            if two.rotateRight(by: 1).equals(one) == false { return 8 }
            // rotateRight(1, by: 1) = highBit (wraps from bit 0 to bit 63)
            if one.rotateRight(by: 1).equals(highBit) == false { return 9 }

            // rotateLeft and rotateRight are inverses
            let testVal: std.num.UInt64 = 123456789;
            if testVal.rotateLeft(by: 17).rotateRight(by: 17).equals(testVal) == false { return 10 }

            // init(from:) — from Int64
            let fromI64Val: std.num.Int64 = 1000000;
            let fromI64 = std.num.UInt64(from: fromI64Val);
            let lit1000000: std.num.UInt64 = 1000000;
            if fromI64.equals(lit1000000) == false { return 11 }

            // init(from:) — from UInt8
            let fromU8Val: std.num.UInt8 = 255;
            let fromU8 = std.num.UInt64(from: fromU8Val);
            let lit255: std.num.UInt64 = 255;
            if fromU8.equals(lit255) == false { return 12 }

            // parse — valid large value
            let parsed = std.num.UInt64.parse( "18446744073709551615");
            if parsed.isNone() { return 13 }
            if parsed.unwrap().equals(std.num.UInt64.maxValue) == false { return 14 }

            // parse — zero
            let parsedZero = std.num.UInt64.parse( "0");
            if parsedZero.isNone() { return 15 }
            if parsedZero.unwrap().equals(std.num.UInt64.zero) == false { return 16 }

            // parse — out of range (18446744073709551616 > max)
            let parsedOver = std.num.UInt64.parse( "18446744073709551616");
            if parsedOver.isSome() { return 17 }

            // parse — negative not allowed for unsigned
            let parsedNeg = std.num.UInt64.parse( "-1");
            if parsedNeg.isSome() { return 18 }

            // parse — empty string
            let parsedEmpty = std.num.UInt64.parse( "");
            if parsedEmpty.isSome() { return 19 }

            0
        }
    
```

## stdlib::uint64::uint64_boundaries_and_constants

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/uint64.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::uint64::uint64_boundaries_and_constants -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            // minValue is 0
            let minVal = std.num.UInt64.minValue;
            let lit0: std.num.UInt64 = 0;
            if minVal.equals(lit0) == false { return 1 }

            // maxValue is 18446744073709551615
            let maxVal = std.num.UInt64.maxValue;
            let lit18446744073709551615: std.num.UInt64 = 18446744073709551615;
            if maxVal.equals(lit18446744073709551615) == false { return 2 }

            // bitWidth is 64
            if std.num.UInt64.bitWidth != 64 { return 3 }

            // zero constant
            let z = std.num.UInt64.zero;
            let zeroLit: std.num.UInt64 = 0;
            if z.equals(zeroLit) == false { return 4 }

            // one constant
            let o = std.num.UInt64.one;
            let oneLit: std.num.UInt64 = 1;
            if o.equals(oneLit) == false { return 5 }

            // isZero
            if minVal.isZero == false { return 6 }
            if maxVal.isZero { return 7 }

            // isPositive
            if maxVal.isPositive == false { return 8 }
            if minVal.isPositive { return 9 }

            // isNegative is always false for unsigned
            if minVal.isNegative { return 10 }
            if maxVal.isNegative { return 11 }

            0
        }
    
```

## stdlib::uint64::uint64_overflow_behavior

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/uint64.rs`
- Status on rerun: FIXED (PASS)
- Verification: `cargo test -p kestrel-test-suite stdlib::uint64::uint64_overflow_behavior -- --exact`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt64.maxValue;
            let minVal = std.num.UInt64.minValue;
            let one: std.num.UInt64 = 1;
            let two: std.num.UInt64 = 2;
            let large: std.num.UInt64 = 10000000000000000000;

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            let three: std.num.UInt64 = 3;
            if addNorm.unwrap().equals(three) == false { return 2 }

            // addChecked — overflow at max
            let addOver = maxVal.addChecked(one);
            if addOver.isSome() { return 3 }

            // subtractChecked — normal case
            let subNorm = two.subtractChecked(one);
            if subNorm.isNone() { return 4 }
            if subNorm.unwrap().equals(one) == false { return 5 }

            // subtractChecked — underflow at 0
            let subUnder = minVal.subtractChecked(one);
            if subUnder.isSome() { return 6 }

            // multiplyChecked — normal case
            let mulThree: std.num.UInt64 = 3;
            let mulNorm = two.multiplyChecked(mulThree);
            if mulNorm.isNone() { return 7 }
            let six: std.num.UInt64 = 6;
            if mulNorm.unwrap().equals(six) == false { return 8 }

            // multiplyChecked — overflow near max
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let hundred: std.num.UInt64 = 100;
            let addSat = maxVal.addSaturating(hundred);
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            let addSatThree: std.num.UInt64 = 3;
            if addSatNorm.equals(addSatThree) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt64.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatThree: std.num.UInt64 = 3;
            let mulSatNorm = two.multiplySaturating(mulSatThree);
            let mulSatSix: std.num.UInt64 = 6;
            if mulSatNorm.equals(mulSatSix) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to maxValue
            let wrapped = minVal.subtract(one);
            if wrapped.equals(maxVal) == false { return 16 }

            0
        }
    
```

## stdlib::views::chars_view_substring

- Source: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/stdlib/views.rs`
- Status on rerun: FAIL
- Error: `thread 'stdlib::views::chars_view_substring' (89302209) panicked at /Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/lib.rs:673:13:`

### Source
```kestrel
module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- chars.substring(from:to:) ---- (character indices, not bytes)
            let sub = s.chars.substring(from: 0, to: 5);
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.chars.substring(from: 6, to: 11);
            if sub2.equals("world") == false { return 2 }

            // ---- chars.substring(checked:to:) ----
            let checked = s.chars.substring(checked: 0, to: 5);
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.chars.substring(checked: 0, to: 100);
            if oob.isSome() { return 5 }

            // Empty range
            let empty = s.chars.substring(checked: 3, to: 3);
            if empty.isNone() { return 6 }
            if empty.unwrap().isEmpty == false { return 7 }

            0
        }
    
```
