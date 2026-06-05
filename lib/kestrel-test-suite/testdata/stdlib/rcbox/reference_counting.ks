// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Create RcBox
            let box1 = std.memory.RcBox[std.numeric.Int64](42);

            // Test getValue
            if box1.getValue() != 42 { return 1 }

            // Test initial refCount is 1
            if box1.refCount() != 1 { return 2 }

            // Test isUnique
            if box1.isUnique() == false { return 3 }

            // Test clone increments refCount
            let box2 = box1.clone();
            if box1.refCount() != 2 { return 4 }
            if box1.isUnique() { return 5 }

            // Both share the same value
            if box2.getValue() != 42 { return 6 }

            0
        }
