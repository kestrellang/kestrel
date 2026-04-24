// test: execution
// stdlib: true

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
