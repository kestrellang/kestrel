// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test writeByte using Buffer
            var buf = std.io.write.Buffer();
            let byte65: std.numeric.UInt8 = 65;
            let wb1 = std.io.write.writeByte( buf, byte65);
            match wb1 {
                .Ok(_) => 0,
                .Err(_) => return 1
            }
            if buf.count != 1 { return 2 }

            // Test writeString using Buffer
            var buf2 = std.io.write.Buffer();
            let ws = std.io.write.writeString( buf2, "Hello");
            match ws {
                .Ok(_) => 0,
                .Err(_) => return 3
            }
            if buf2.count != 5 { return 4 }
            if buf2.toString().isEqual(to: "Hello") == false { return 5 }

            // Test writeLine using Buffer
            var buf3 = std.io.write.Buffer();
            let wl = std.io.write.writeLine( buf3, "Hi");
            match wl {
                .Ok(_) => 0,
                .Err(_) => return 6
            }
            // "Hi" + newline = 3 bytes
            if buf3.count != 3 { return 7 }

            // Test writeAll using Buffer
            var buf4 = std.io.write.Buffer();
            let byte1: std.numeric.UInt8 = 1;
            let byte2: std.numeric.UInt8 = 2;
            let byte3: std.numeric.UInt8 = 3;
            var data = std.collections.Array[std.numeric.UInt8]();
            data.append(byte1);
            data.append(byte2);
            data.append(byte3);
            let slice = std.memory.ArraySlice[std.numeric.UInt8](pointer: data.asPointer(), count: 3);
            let wa = std.io.write.writeAll( buf4, from: slice);
            match wa {
                .Ok(_) => 0,
                .Err(_) => return 8
            }
            if buf4.count != 3 { return 9 }
            let arr = buf4.toArray();
            if arr(unchecked: 0) != byte1 { return 10 }
            if arr(unchecked: 1) != byte2 { return 11 }
            if arr(unchecked: 2) != byte3 { return 12 }

            // Test writeString with empty string
            var buf5 = std.io.write.Buffer();
            let wsEmpty = std.io.write.writeString( buf5, "");
            match wsEmpty {
                .Ok(_) => 0,
                .Err(_) => return 13
            }
            if buf5.count != 0 { return 14 }

            // Test multiple writes accumulate
            var buf6 = std.io.write.Buffer();
            let _ = std.io.write.writeString( buf6, "Hello");
            let _ = std.io.write.writeString( buf6, " ");
            let _ = std.io.write.writeString( buf6, "World");
            if buf6.toString().isEqual(to: "Hello World") == false { return 15 }

            0
        }
