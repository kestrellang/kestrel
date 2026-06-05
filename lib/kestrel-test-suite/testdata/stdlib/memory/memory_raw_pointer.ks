// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test nullPointer
            let nullPtr = std.memory.RawPointer.nullPointer();
            if nullPtr.isNull == false { return 1 }

            // Test address of null pointer is 0
            let zeroAddr: std.numeric.UInt64 = 0;
            if nullPtr.address != zeroAddr { return 2 }

            // Create a non-null pointer from an array
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(42);
            let ptr = arr.asPointer();
            let raw = ptr.asRaw();

            // Non-null pointer
            if raw.isNull { return 3 }

            // Address should be non-zero
            let zeroCheck: std.numeric.UInt64 = 0;
            if raw.address == zeroCheck { return 4 }

            // Test equals - same pointer should be equal
            if raw.isEqual(to: raw) == false { return 5 }

            // Test equals - null vs non-null should not be equal
            if raw.isEqual(to: nullPtr) { return 6 }

            // Test offset
            let offsetPtr = raw.offset(by: 8);
            if offsetPtr.isNull { return 7 }
            // The offset pointer should not equal the original
            if offsetPtr.isEqual(to: raw) { return 8 }

            // Test cast to typed pointer
            let typedPtr = raw.cast[std.numeric.Int64]();
            if typedPtr.isNull { return 9 }
            // Read through the casted pointer should give the array element
            if typedPtr.read() != 42 { return 10 }

            // Test init(address:) round-trip
            let addr = raw.address;
            let fromAddr = std.memory.RawPointer(address: addr);
            if fromAddr.isEqual(to: raw) == false { return 11 }

            0
        }
