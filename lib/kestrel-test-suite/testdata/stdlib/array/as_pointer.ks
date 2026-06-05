// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // asPointer() returns a pointer to the internal buffer
            let ptr = arr.asPointer();

            // Read through the pointer to verify it points to the array data
            let val0 = ptr.read();
            if val0 != 10 { return 1 }

            let val1 = ptr.offset(by: 1).read();
            if val1 != 20 { return 2 }

            let val2 = ptr.offset(by: 2).read();
            if val2 != 30 { return 3 }

            0
        }
