// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create an Empty reader
            var empty = std.io.read.Empty();

            // Create a buffer to read into
            let zeroByte: std.num.UInt8 = 0;
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(zeroByte);
            buf.append(zeroByte);
            buf.append(zeroByte);
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 3);

            // Read should return Ok(0) immediately (EOF)
            let result = empty.read(into: slice);
            match result {
                .Ok(n) => if n != 0 { return 1 },
                .Err(_) => return 2
            }

            // Read again should still return Ok(0)
            let result2 = empty.read(into: slice);
            match result2 {
                .Ok(n) => if n != 0 { return 3 },
                .Err(_) => return 4
            }

            0
        }
