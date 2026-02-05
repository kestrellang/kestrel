use kestrel_test_suite::*;

// TODO: Fails -- memory module type paths or pointer operations may not resolve correctly
#[test]
fn memory_raw_pointer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test nilPointer
            let nil = std.memory.RawPointer.nilPointer();
            if nil.isNull == false { return 1 }

            // Test address of nil pointer is 0
            if nil.address != std.num.UInt64(intLiteral: 0) { return 2 }

            // Create a non-null pointer from an array
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(42);
            let ptr = arr.asPointer();
            let raw = ptr.asRaw();

            // Non-null pointer
            if raw.isNull { return 3 }

            // Address should be non-zero
            if raw.address == std.num.UInt64(intLiteral: 0) { return 4 }

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
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- memory module type paths or pointer operations may not resolve correctly
#[test]
fn memory_typed_pointer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test nullPointer
            let null = std.memory.Pointer[std.num.Int64].nullPointer();
            if null.isNull == false { return 1 }

            // Test address of null pointer is 0
            if null.address != std.num.UInt64(intLiteral: 0) { return 2 }

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
            ptr1.write(value: 999);
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
            if addr == std.num.UInt64(intLiteral: 0) { return 13 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn memory_slice() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create slice from array's asSlice()
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);
            arr.append(40);
            arr.append(50);

            let slice = arr.asSlice();

            // Test count
            if slice.count != 5 { return 1 }

            // Test isEmpty
            if slice.isEmpty { return 2 }

            // Test isEmpty on empty slice
            let emptyArr = std.collections.Array[std.num.Int64]();
            let emptySlice = emptyArr.asSlice();
            if emptySlice.isEmpty == false { return 3 }
            if emptySlice.count != 0 { return 4 }

            // Test subscript(unchecked:) get
            if slice(unchecked: 0) != 10 { return 5 }
            if slice(unchecked: 1) != 20 { return 6 }
            if slice(unchecked: 2) != 30 { return 7 }
            if slice(unchecked: 3) != 40 { return 8 }
            if slice(unchecked: 4) != 50 { return 9 }

            // Test subscript(safe:) - valid index
            let safe1 = slice(safe: 2);
            if safe1.isNone() { return 10 }
            if safe1.unwrap() != 30 { return 11 }

            // Test subscript(safe:) - out of bounds
            let safeOob = slice(safe: 10);
            if safeOob.isSome() { return 12 }

            // Test subscript(safe:) - negative index
            let safeNeg = slice(safe: -1);
            if safeNeg.isSome() { return 13 }

            // Test first()
            let f = slice.first();
            if f.isNone() { return 14 }
            if f.unwrap() != 10 { return 15 }

            // Test last()
            let l = slice.last();
            if l.isNone() { return 16 }
            if l.unwrap() != 50 { return 17 }

            // Test first() and last() on empty slice
            if emptySlice.first().isSome() { return 18 }
            if emptySlice.last().isSome() { return 19 }

            // Test slice(from:to:) - valid sub-slice
            let sub = slice.slice(from: 1, to: 4);
            if sub.isNone() { return 20 }
            let subSlice = sub.unwrap();
            if subSlice.count != 3 { return 21 }
            if subSlice(unchecked: 0) != 20 { return 22 }
            if subSlice(unchecked: 1) != 30 { return 23 }
            if subSlice(unchecked: 2) != 40 { return 24 }

            // Test slice(from:to:) - empty sub-slice
            let emptySub = slice.slice(from: 2, to: 2);
            if emptySub.isNone() { return 25 }
            if emptySub.unwrap().count != 0 { return 26 }

            // Test slice(from:to:) - full range
            let fullSub = slice.slice(from: 0, to: 5);
            if fullSub.isNone() { return 27 }
            if fullSub.unwrap().count != 5 { return 28 }

            // Test slice(from:to:) - invalid range returns None
            let invalidSub = slice.slice(from: 3, to: 1);
            if invalidSub.isSome() { return 29 }

            // Test slice(from:to:) - out of bounds returns None
            let oobSub = slice.slice(from: 0, to: 10);
            if oobSub.isSome() { return 30 }

            // Test iter()
            var iter = slice.iter();
            var sum: std.num.Int64 = 0;
            var done: std.core.Bool = false;
            while done == false {
                let next = iter.next();
                if next.isSome() {
                    sum = sum + next.unwrap()
                } else {
                    done = true
                }
            }
            if sum != 150 { return 31 }

            // Test pointer property
            let ptr = slice.pointer;
            if ptr.isNull { return 32 }
            if ptr.read() != 10 { return 33 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- Buffer requires Allocator generic which may hit codegen issues
#[test]
fn memory_buffer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a buffer with SystemAllocator
            var alloc = std.memory.SystemAllocator();
            var buf = std.memory.Buffer[std.num.Int64, std.memory.SystemAllocator](capacity: 10, allocator: alloc);

            // Test capacity
            if buf.capacity != 10 { return 1 }

            // Test pointer is non-null
            if buf.pointer.isNull { return 2 }

            // Test write(unchecked:value:) and read(unchecked:)
            buf.write(unchecked: 0, value: 42);
            buf.write(unchecked: 1, value: 99);
            buf.write(unchecked: 2, value: 77);
            if buf.read(unchecked: 0) != 42 { return 3 }
            if buf.read(unchecked: 1) != 99 { return 4 }
            if buf.read(unchecked: 2) != 77 { return 5 }

            // Test write(at:value:) with bounds checking
            let ok1 = buf.write(at: 5, value: 55);
            if ok1 == false { return 6 }
            if buf.read(unchecked: 5) != 55 { return 7 }

            // Test write(at:value:) out of bounds
            let oob = buf.write(at: 100, value: 0);
            if oob { return 8 }

            // Test write(at:value:) negative index
            let neg = buf.write(at: -1, value: 0);
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
            buf.write(unchecked: 15, value: 123);
            if buf.read(unchecked: 15) != 123 { return 25 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- Layout/Allocator types may not resolve correctly in test harness
#[test]
fn memory_allocator() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test SystemAllocator init
            var alloc = std.memory.SystemAllocator();

            // Test allocate
            let layout = std.memory.Layout(size: 64, alignment: 8);
            let result = alloc.allocate(layout);
            if result.isNone() { return 1 }
            let ptr = result.unwrap();
            if ptr.isNull { return 2 }

            // Write to and read from the allocated memory
            let typedPtr = ptr.cast[std.num.Int64]();
            typedPtr.write(value: 12345);
            if typedPtr.read() != 12345 { return 3 }

            // Write at an offset
            typedPtr.offset(by: 1).write(value: 67890);
            if typedPtr.offset(by: 1).read() != 67890 { return 4 }

            // Test reallocate
            let newLayout = std.memory.Layout(size: 128, alignment: 8);
            let reallocResult = alloc.reallocate(ptr, layout, newLayout);
            if reallocResult.isNone() { return 5 }
            let newPtr = reallocResult.unwrap();
            if newPtr.isNull { return 6 }

            // Data should be preserved after realloc
            let newTyped = newPtr.cast[std.num.Int64]();
            if newTyped.read() != 12345 { return 7 }
            if newTyped.offset(by: 1).read() != 67890 { return 8 }

            // Test deallocate
            alloc.deallocate(newPtr, newLayout);

            // Test Layout.of
            let i64Layout = std.memory.Layout.of[std.num.Int64]();
            if i64Layout.size != 8 { return 9 }
            if i64Layout.alignment != 8 { return 10 }

            // Test Layout.array
            let arrLayout = std.memory.Layout.array[std.num.Int64](count: 4);
            if arrLayout.size != 32 { return 11 }
            if arrLayout.alignment != 8 { return 12 }

            // Test Layout.equals
            let l1 = std.memory.Layout(size: 8, alignment: 8);
            let l2 = std.memory.Layout(size: 8, alignment: 8);
            let l3 = std.memory.Layout(size: 16, alignment: 8);
            if l1.equals(l2) == false { return 13 }
            if l1.equals(l3) { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
