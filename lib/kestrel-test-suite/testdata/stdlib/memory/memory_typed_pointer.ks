// test: execution
// stdlib: true

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
