// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test appendChar
            var s = std.text.String();
            s.appendChar('H');
            s.appendChar('i');
            if s.byteCount != 2 { return 1 }
            if s.equals("Hi") == false { return 2 }

            // Test appendByte
            var s2 = std.text.String();
            // Append ASCII 'A' (65)
            s2.appendByte(65);
            s2.appendByte(66);
            if s2.byteCount != 2 { return 3 }
            if s2.equals("AB") == false { return 4 }

            // Test clear()
            var s3: std.text.String = "hello world";
            if s3.isEmpty { return 5 }
            s3.clear();
            if s3.isEmpty == false { return 6 }
            if s3.byteCount != 0 { return 7 }

            // Test init(capacity:)
            var s4 = std.text.String(capacity: 64);
            if s4.capacity < 64 { return 8 }
            if s4.isEmpty == false { return 9 }
            if s4.byteCount != 0 { return 10 }

            // After appending, capacity should still be >= 64
            s4.append("test");
            if s4.byteCount != 4 { return 11 }
            if s4.capacity < 64 { return 12 }

            // Test that clear preserves capacity
            let capBefore = s4.capacity;
            s4.clear();
            if s4.capacity != capBefore { return 13 }

            0
        }
