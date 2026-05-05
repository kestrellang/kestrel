// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create a Buffer writer
            var buf = std.io.write.Buffer();

            // Initially empty
            if buf.isEmpty == false { return 1 }
            if buf.count != 0 { return 2 }

            // Write some bytes
            let byte72: std.numeric.UInt8 = 72;   // 'H'
            let byte101: std.numeric.UInt8 = 101; // 'e'
            let byte108: std.numeric.UInt8 = 108; // 'l'
            let byte111: std.numeric.UInt8 = 111; // 'o'
            var data = std.collections.Array[std.numeric.UInt8]();
            data.append(byte72);
            data.append(byte101);
            data.append(byte108);
            data.append(byte108);
            data.append(byte111);
            let slice = std.memory.ArraySlice[std.numeric.UInt8](pointer: data.asPointer(), count: 5);
            let result = buf.write(from: slice);
            match result {
                .Ok(n) => if n != 5 { return 3 },
                .Err(_) => return 4
            }

            // Check count
            if buf.count != 5 { return 5 }
            if buf.isEmpty { return 6 }

            // Check toString
            let s = buf.toString();
            if s.isEqual(to: "Hello") == false { return 7 }

            // Check toArray
            let arr = buf.toArray();
            if arr.count != 5 { return 8 }
            if arr(unchecked: 0) != byte72 { return 9 }

            // Check asSlice
            let sl = buf.asSlice();
            if sl.count != 5 { return 10 }
            if sl(unchecked: 0) != byte72 { return 11 }

            // Flush should succeed (no-op for Buffer)
            let flushResult = buf.flush();
            match flushResult {
                .Ok(_) => 0,
                .Err(_) => return 12
            }

            // Write more data
            let byte33: std.numeric.UInt8 = 33; // '!'
            var data2 = std.collections.Array[std.numeric.UInt8]();
            data2.append(byte33);
            let slice2 = std.memory.ArraySlice[std.numeric.UInt8](pointer: data2.asPointer(), count: 1);
            let result2 = buf.write(from: slice2);
            match result2 {
                .Ok(n) => if n != 1 { return 13 },
                .Err(_) => return 14
            }
            if buf.count != 6 { return 15 }
            if buf.toString().isEqual(to: "Hello!") == false { return 16 }

            // Test clear
            buf.clear();
            if buf.count != 0 { return 17 }
            if buf.isEmpty == false { return 18 }

            // Test init with capacity
            var buf2 = std.io.write.Buffer( 64);
            if buf2.isEmpty == false { return 19 }
            if buf2.count != 0 { return 20 }

            0
        }
