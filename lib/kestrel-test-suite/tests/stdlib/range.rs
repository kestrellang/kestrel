use kestrel_test_suite::*;

#[test]
fn iteration() {
    Test::new(
        r#"module Test

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
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn range_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let r = std.core.Range[std.num.Int64](2, 8);

            // contains - value in range
            if r.contains(2) == false { return 1 }
            if r.contains(5) == false { return 2 }
            if r.contains(7) == false { return 3 }

            // contains - end is exclusive
            if r.contains(8) { return 4 }

            // contains - value below range
            if r.contains(1) { return 5 }

            // contains - value above range
            if r.contains(9) { return 6 }

            // isEmpty - non-empty range
            if r.isEmpty() { return 7 }

            // isEmpty - empty range (start >= end)
            let emptyRange = std.core.Range[std.num.Int64](5, 5);
            if emptyRange.isEmpty() == false { return 8 }

            let reverseRange = std.core.Range[std.num.Int64](8, 2);
            if reverseRange.isEmpty() == false { return 9 }

            // equals
            let r2 = std.core.Range[std.num.Int64](2, 8);
            if r.equals(r2) == false { return 10 }

            let r3 = std.core.Range[std.num.Int64](2, 9);
            if r.equals(r3) { return 11 }

            let r4 = std.core.Range[std.num.Int64](3, 8);
            if r.equals(r4) { return 12 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn closed_range() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // init and basic properties
            let r = std.core.ClosedRange[std.num.Int64](2, 5);

            // contains - value in range (inclusive of both endpoints)
            if r.contains(2) == false { return 1 }
            if r.contains(3) == false { return 2 }
            if r.contains(5) == false { return 3 }

            // contains - value outside range
            if r.contains(1) { return 4 }
            if r.contains(6) { return 5 }

            // isEmpty - non-empty range
            if r.isEmpty() { return 6 }

            // isEmpty - single element range (start == end)
            let singleRange = std.core.ClosedRange[std.num.Int64](5, 5);
            if singleRange.isEmpty() { return 7 }

            // isEmpty - empty range (start > end)
            let emptyRange = std.core.ClosedRange[std.num.Int64](8, 2);
            if emptyRange.isEmpty() == false { return 8 }

            // equals
            let r2 = std.core.ClosedRange[std.num.Int64](2, 5);
            if r.equals(r2) == false { return 9 }

            let r3 = std.core.ClosedRange[std.num.Int64](2, 6);
            if r.equals(r3) { return 10 }

            // iter - iterate and sum 2+3+4+5 = 14
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
            if sum != 14 { return 11 }

            // iter - single element range should yield one element
            var singleSum: std.num.Int64 = 0;
            var singleIter = singleRange.iter();
            var singleDone: std.core.Bool = false;
            while singleDone == false {
                let next = singleIter.next();
                if next.isSome() {
                    singleSum = singleSum + next.unwrap()
                } else {
                    singleDone = true
                }
            }
            if singleSum != 5 { return 12 }

            // iter - empty range should yield nothing
            var emptySum: std.num.Int64 = 0;
            var emptyIter = emptyRange.iter();
            var emptyDone: std.core.Bool = false;
            while emptyDone == false {
                let next = emptyIter.next();
                if next.isSome() {
                    emptySum = emptySum + next.unwrap()
                } else {
                    emptyDone = true
                }
            }
            if emptySum != 0 { return 13 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
