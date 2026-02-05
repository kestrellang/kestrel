use kestrel_test_suite::*;

// TODO: Fails -- IO module type paths may not resolve correctly in test harness
#[test]
fn io_error_types() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test Error constructor with code
            let err = std.io.error.Error(code: std.num.Int32(intLiteral: 2));
            if err.errno() != std.num.Int32(intLiteral: 2) { return 1 }

            // Test description for known codes
            let desc = err.description();
            if desc.equals("no such file or directory") == false { return 2 }

            // Test notFound() convenience constructor
            let nf = std.io.error.notFound();
            if nf.errno() != std.num.Int32(intLiteral: 2) { return 3 }
            if nf.description().equals("no such file or directory") == false { return 4 }

            // Test permissionDenied()
            let pd = std.io.error.permissionDenied();
            if pd.errno() != std.num.Int32(intLiteral: 13) { return 5 }
            if pd.description().equals("permission denied") == false { return 6 }

            // Test alreadyExists()
            let ae = std.io.error.alreadyExists();
            if ae.errno() != std.num.Int32(intLiteral: 17) { return 7 }
            if ae.description().equals("file exists") == false { return 8 }

            // Test invalidInput()
            let ii = std.io.error.invalidInput();
            if ii.errno() != std.num.Int32(intLiteral: 22) { return 9 }
            if ii.description().equals("invalid argument") == false { return 10 }

            // Test wouldBlock()
            let wb = std.io.error.wouldBlock();
            if wb.errno() != std.num.Int32(intLiteral: 11) { return 11 }
            if wb.description().equals("would block") == false { return 12 }

            // Test interrupted()
            let intr = std.io.error.interrupted();
            if intr.errno() != std.num.Int32(intLiteral: 4) { return 13 }
            if intr.description().equals("interrupted") == false { return 14 }

            // Test brokenPipe()
            let bp = std.io.error.brokenPipe();
            if bp.errno() != std.num.Int32(intLiteral: 32) { return 15 }
            if bp.description().equals("broken pipe") == false { return 16 }

            // Test unknown error code
            let unk = std.io.error.Error(code: std.num.Int32(intLiteral: 999));
            if unk.description().equals("unknown error") == false { return 17 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_empty_reader() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create an Empty reader
            var empty = std.io.Empty();

            // Create a buffer to read into
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
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
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_repeat_reader() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Repeat reader that yields byte 42
            var rep = std.io.Repeat(byte: std.num.UInt8(intLiteral: 42));

            // Create a buffer to read into
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 5);

            // Read should fill the entire buffer with byte 42
            let result = rep.read(into: slice);
            match result {
                .Ok(n) => if n != 5 { return 1 },
                .Err(_) => return 2
            }

            // Verify all bytes are 42
            if buf(unchecked: 0) != std.num.UInt8(intLiteral: 42) { return 3 }
            if buf(unchecked: 1) != std.num.UInt8(intLiteral: 42) { return 4 }
            if buf(unchecked: 2) != std.num.UInt8(intLiteral: 42) { return 5 }
            if buf(unchecked: 3) != std.num.UInt8(intLiteral: 42) { return 6 }
            if buf(unchecked: 4) != std.num.UInt8(intLiteral: 42) { return 7 }

            // Read again with a different size buffer
            var buf2 = std.collections.Array[std.num.UInt8]();
            buf2.append(std.num.UInt8(intLiteral: 0));
            buf2.append(std.num.UInt8(intLiteral: 0));
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: buf2.asPointer(), count: 2);
            let result2 = rep.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != std.num.UInt8(intLiteral: 42) { return 10 }
            if buf2(unchecked: 1) != std.num.UInt8(intLiteral: 42) { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_cursor() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create data for cursor
            var data = std.collections.Array[std.num.UInt8]();
            data.append(std.num.UInt8(intLiteral: 10));
            data.append(std.num.UInt8(intLiteral: 20));
            data.append(std.num.UInt8(intLiteral: 30));
            data.append(std.num.UInt8(intLiteral: 40));
            data.append(std.num.UInt8(intLiteral: 50));

            // Create cursor
            var cursor = std.io.Cursor(data: data);

            // Initial position should be 0
            if cursor.position() != 0 { return 1 }

            // Read first 3 bytes
            var buf = std.collections.Array[std.num.UInt8]();
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            buf.append(std.num.UInt8(intLiteral: 0));
            let slice = std.memory.Slice[std.num.UInt8](pointer: buf.asPointer(), count: 3);
            let result = cursor.read(into: slice);
            match result {
                .Ok(n) => if n != 3 { return 2 },
                .Err(_) => return 3
            }

            // Verify bytes read
            if buf(unchecked: 0) != std.num.UInt8(intLiteral: 10) { return 4 }
            if buf(unchecked: 1) != std.num.UInt8(intLiteral: 20) { return 5 }
            if buf(unchecked: 2) != std.num.UInt8(intLiteral: 30) { return 6 }

            // Position should be 3
            if cursor.position() != 3 { return 7 }

            // Read remaining 2 bytes (request 5 but only 2 available)
            var buf2 = std.collections.Array[std.num.UInt8]();
            buf2.append(std.num.UInt8(intLiteral: 0));
            buf2.append(std.num.UInt8(intLiteral: 0));
            buf2.append(std.num.UInt8(intLiteral: 0));
            buf2.append(std.num.UInt8(intLiteral: 0));
            buf2.append(std.num.UInt8(intLiteral: 0));
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: buf2.asPointer(), count: 5);
            let result2 = cursor.read(into: slice2);
            match result2 {
                .Ok(n) => if n != 2 { return 8 },
                .Err(_) => return 9
            }
            if buf2(unchecked: 0) != std.num.UInt8(intLiteral: 40) { return 10 }
            if buf2(unchecked: 1) != std.num.UInt8(intLiteral: 50) { return 11 }

            // Position should be 5
            if cursor.position() != 5 { return 12 }

            // Read at EOF should return 0
            let result3 = cursor.read(into: slice);
            match result3 {
                .Ok(n) => if n != 0 { return 13 },
                .Err(_) => return 14
            }

            // Test setPosition
            cursor.setPosition(to: 1);
            if cursor.position() != 1 { return 15 }

            // Read after setPosition
            var buf3 = std.collections.Array[std.num.UInt8]();
            buf3.append(std.num.UInt8(intLiteral: 0));
            buf3.append(std.num.UInt8(intLiteral: 0));
            let slice3 = std.memory.Slice[std.num.UInt8](pointer: buf3.asPointer(), count: 2);
            let result4 = cursor.read(into: slice3);
            match result4 {
                .Ok(n) => if n != 2 { return 16 },
                .Err(_) => return 17
            }
            if buf3(unchecked: 0) != std.num.UInt8(intLiteral: 20) { return 18 }
            if buf3(unchecked: 1) != std.num.UInt8(intLiteral: 30) { return 19 }

            // setPosition clamps negative to 0
            cursor.setPosition(to: -5);
            if cursor.position() != 0 { return 20 }

            // setPosition clamps beyond end to data count
            cursor.setPosition(to: 100);
            if cursor.position() != 5 { return 21 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_sink_writer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Sink writer
            var sink = std.io.Sink();

            // Create data to write
            var data = std.collections.Array[std.num.UInt8]();
            data.append(std.num.UInt8(intLiteral: 1));
            data.append(std.num.UInt8(intLiteral: 2));
            data.append(std.num.UInt8(intLiteral: 3));
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 3);

            // Write should succeed and report all bytes as written
            let result = sink.write(from: slice);
            match result {
                .Ok(n) => if n != 3 { return 1 },
                .Err(_) => return 2
            }

            // Flush should succeed
            let flushResult = sink.flush();
            match flushResult {
                .Ok(_) => 0,
                .Err(_) => return 3
            }

            // Write with a larger buffer
            var big = std.collections.Array[std.num.UInt8]();
            var i: std.num.Int64 = 0;
            while i < 100 {
                big.append(std.num.UInt8(intLiteral: 255));
                i = i + 1
            }
            let bigSlice = std.memory.Slice[std.num.UInt8](pointer: big.asPointer(), count: 100);
            let result2 = sink.write(from: bigSlice);
            match result2 {
                .Ok(n) => if n != 100 { return 4 },
                .Err(_) => return 5
            }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_buffer_writer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Create a Buffer writer
            var buf = std.io.Buffer();

            // Initially empty
            if buf.isEmpty() == false { return 1 }
            if buf.count() != 0 { return 2 }

            // Write some bytes
            var data = std.collections.Array[std.num.UInt8]();
            data.append(std.num.UInt8(intLiteral: 72));  // 'H'
            data.append(std.num.UInt8(intLiteral: 101)); // 'e'
            data.append(std.num.UInt8(intLiteral: 108)); // 'l'
            data.append(std.num.UInt8(intLiteral: 108)); // 'l'
            data.append(std.num.UInt8(intLiteral: 111)); // 'o'
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 5);
            let result = buf.write(from: slice);
            match result {
                .Ok(n) => if n != 5 { return 3 },
                .Err(_) => return 4
            }

            // Check count
            if buf.count() != 5 { return 5 }
            if buf.isEmpty() { return 6 }

            // Check toString
            let s = buf.toString();
            if s.equals("Hello") == false { return 7 }

            // Check toArray
            let arr = buf.toArray();
            if arr.count != 5 { return 8 }
            if arr(unchecked: 0) != std.num.UInt8(intLiteral: 72) { return 9 }

            // Check asSlice
            let sl = buf.asSlice();
            if sl.count != 5 { return 10 }
            if sl(unchecked: 0) != std.num.UInt8(intLiteral: 72) { return 11 }

            // Flush should succeed (no-op for Buffer)
            let flushResult = buf.flush();
            match flushResult {
                .Ok(_) => 0,
                .Err(_) => return 12
            }

            // Write more data
            var data2 = std.collections.Array[std.num.UInt8]();
            data2.append(std.num.UInt8(intLiteral: 33)); // '!'
            let slice2 = std.memory.Slice[std.num.UInt8](pointer: data2.asPointer(), count: 1);
            let result2 = buf.write(from: slice2);
            match result2 {
                .Ok(n) => if n != 1 { return 13 },
                .Err(_) => return 14
            }
            if buf.count() != 6 { return 15 }
            if buf.toString().equals("Hello!") == false { return 16 }

            // Test clear
            buf.clear();
            if buf.count() != 0 { return 17 }
            if buf.isEmpty() == false { return 18 }

            // Test init with capacity
            var buf2 = std.io.Buffer(capacity: 64);
            if buf2.isEmpty() == false { return 19 }
            if buf2.count() != 0 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_read_helpers() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test readByte using a Cursor
            var data = std.collections.Array[std.num.UInt8]();
            data.append(std.num.UInt8(intLiteral: 65)); // 'A'
            data.append(std.num.UInt8(intLiteral: 66)); // 'B'
            data.append(std.num.UInt8(intLiteral: 67)); // 'C'

            var cursor = std.io.Cursor(data: data);

            // readByte should return first byte
            let rb1 = std.io.readByte(reader: cursor);
            match rb1 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != std.num.UInt8(intLiteral: 65) { return 1 },
                    .None => return 2
                },
                .Err(_) => return 3
            }

            // readByte should return second byte
            let rb2 = std.io.readByte(reader: cursor);
            match rb2 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != std.num.UInt8(intLiteral: 66) { return 4 },
                    .None => return 5
                },
                .Err(_) => return 6
            }

            // readByte should return third byte
            let rb3 = std.io.readByte(reader: cursor);
            match rb3 {
                .Ok(opt) => match opt {
                    .Some(b) => if b != std.num.UInt8(intLiteral: 67) { return 7 },
                    .None => return 8
                },
                .Err(_) => return 9
            }

            // readByte at EOF should return None
            let rb4 = std.io.readByte(reader: cursor);
            match rb4 {
                .Ok(opt) => match opt {
                    .Some(_) => return 10,
                    .None => 0
                },
                .Err(_) => return 11
            }

            // Test readAll using a Cursor
            var data2 = std.collections.Array[std.num.UInt8]();
            data2.append(std.num.UInt8(intLiteral: 1));
            data2.append(std.num.UInt8(intLiteral: 2));
            data2.append(std.num.UInt8(intLiteral: 3));
            var cursor2 = std.io.Cursor(data: data2);
            var dest = std.collections.Array[std.num.UInt8]();
            let raResult = std.io.readAll(reader: cursor2, into: dest);
            match raResult {
                .Ok(n) => if n != 3 { return 12 },
                .Err(_) => return 13
            }
            if dest.count != 3 { return 14 }
            if dest(unchecked: 0) != std.num.UInt8(intLiteral: 1) { return 15 }
            if dest(unchecked: 1) != std.num.UInt8(intLiteral: 2) { return 16 }
            if dest(unchecked: 2) != std.num.UInt8(intLiteral: 3) { return 17 }

            // Test readAll on Empty reader
            var empty = std.io.Empty();
            var emptyDest = std.collections.Array[std.num.UInt8]();
            let raEmpty = std.io.readAll(reader: empty, into: emptyDest);
            match raEmpty {
                .Ok(n) => if n != 0 { return 18 },
                .Err(_) => return 19
            }
            if emptyDest.count != 0 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails -- IO module type paths may not resolve correctly
#[test]
fn io_write_helpers() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test writeByte using Buffer
            var buf = std.io.Buffer();
            let wb1 = std.io.writeByte(writer: buf, byte: std.num.UInt8(intLiteral: 65));
            match wb1 {
                .Ok(_) => 0,
                .Err(_) => return 1
            }
            if buf.count() != 1 { return 2 }

            // Test writeStr using Buffer
            var buf2 = std.io.Buffer();
            let ws = std.io.writeStr(writer: buf2, s: "Hello");
            match ws {
                .Ok(_) => 0,
                .Err(_) => return 3
            }
            if buf2.count() != 5 { return 4 }
            if buf2.toString().equals("Hello") == false { return 5 }

            // Test writeLine using Buffer
            var buf3 = std.io.Buffer();
            let wl = std.io.writeLine(writer: buf3, s: "Hi");
            match wl {
                .Ok(_) => 0,
                .Err(_) => return 6
            }
            // "Hi" + newline = 3 bytes
            if buf3.count() != 3 { return 7 }

            // Test writeAll using Buffer
            var buf4 = std.io.Buffer();
            var data = std.collections.Array[std.num.UInt8]();
            data.append(std.num.UInt8(intLiteral: 1));
            data.append(std.num.UInt8(intLiteral: 2));
            data.append(std.num.UInt8(intLiteral: 3));
            let slice = std.memory.Slice[std.num.UInt8](pointer: data.asPointer(), count: 3);
            let wa = std.io.writeAll(writer: buf4, from: slice);
            match wa {
                .Ok(_) => 0,
                .Err(_) => return 8
            }
            if buf4.count() != 3 { return 9 }
            let arr = buf4.toArray();
            if arr(unchecked: 0) != std.num.UInt8(intLiteral: 1) { return 10 }
            if arr(unchecked: 1) != std.num.UInt8(intLiteral: 2) { return 11 }
            if arr(unchecked: 2) != std.num.UInt8(intLiteral: 3) { return 12 }

            // Test writeStr with empty string
            var buf5 = std.io.Buffer();
            let wsEmpty = std.io.writeStr(writer: buf5, s: "");
            match wsEmpty {
                .Ok(_) => 0,
                .Err(_) => return 13
            }
            if buf5.count() != 0 { return 14 }

            // Test multiple writes accumulate
            var buf6 = std.io.Buffer();
            let _ = std.io.writeStr(writer: buf6, s: "Hello");
            let _ = std.io.writeStr(writer: buf6, s: " ");
            let _ = std.io.writeStr(writer: buf6, s: "World");
            if buf6.toString().equals("Hello World") == false { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
