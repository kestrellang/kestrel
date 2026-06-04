// test: execution
// stdlib: true

module Test

        @main
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
            let typedPtr = ptr.cast[std.numeric.Int64]();
            typedPtr.write(12345);
            if typedPtr.read() != 12345 { return 3 }

            // Write at an offset
            typedPtr.offset(by: 1).write(67890);
            if typedPtr.offset(by: 1).read() != 67890 { return 4 }

            // Test reallocate
            let newLayout = std.memory.Layout(size: 128, alignment: 8);
            let reallocResult = alloc.reallocate(ptr, layout, newLayout);
            if reallocResult.isNone() { return 5 }
            let newPtr = reallocResult.unwrap();
            if newPtr.isNull { return 6 }

            // Data should be preserved after realloc
            let newTyped = newPtr.cast[std.numeric.Int64]();
            if newTyped.read() != 12345 { return 7 }
            if newTyped.offset(by: 1).read() != 67890 { return 8 }

            // Test deallocate
            alloc.deallocate(newPtr, newLayout);

            // Test Layout.of
            let i64Layout = std.memory.Layout.of[std.numeric.Int64]();
            if i64Layout.size != 8 { return 9 }
            if i64Layout.alignment != 8 { return 10 }

            // Test Layout.array
            let arrLayout = std.memory.Layout.array[std.numeric.Int64](4);
            if arrLayout.size != 32 { return 11 }
            if arrLayout.alignment != 8 { return 12 }

            // Test Layout.equals
            let l1 = std.memory.Layout(size: 8, alignment: 8);
            let l2 = std.memory.Layout(size: 8, alignment: 8);
            let l3 = std.memory.Layout(size: 16, alignment: 8);
            if l1.isEqual(to: l2) == false { return 13 }
            if l1.isEqual(to: l3) { return 14 }

            0
        }
