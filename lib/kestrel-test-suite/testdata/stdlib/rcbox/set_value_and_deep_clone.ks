// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- setValue() ----
            let box1 = std.memory.RcBox[std.num.Int64](10);
            box1.setValue(42);
            if box1.getValue() != 42 { return 1 }

            // setValue on shared box affects both references
            let box2 = box1.clone();
            box1.setValue(99);
            if box2.getValue() != 99 { return 2 }

            // ---- deepClone() ----
            let box3 = std.memory.RcBox[std.num.Int64](50);
            let box4 = box3.deepClone();

            // Deep clone creates independent storage
            if box4.getValue() != 50 { return 3 }
            if box3.refCount() != 1 { return 4 }
            if box4.refCount() != 1 { return 5 }

            // Mutating deep clone doesn't affect original
            box4.setValue(100);
            if box3.getValue() != 50 { return 6 }
            if box4.getValue() != 100 { return 7 }

            0
        }
