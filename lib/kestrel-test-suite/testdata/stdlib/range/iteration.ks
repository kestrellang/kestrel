// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Create range 0..<5
            let r = std.core.Range[std.num.Int64](0, 5);

            // Iterate and sum
            var sum: std.num.Int64 = 0;
            var iter = r.iter();
            var done: std.core.Bool = false;
            while done == false {
                let next = iter.next();
                if next.isSome() {
                    sum = sum + next.unwrap()
                } else {
                    done = true
                }
            }

            // 0 + 1 + 2 + 3 + 4 = 10
            if sum != 10 { return 1 }

            0
        }
