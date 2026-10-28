// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) facets a3 + a4 — `[first, ..rest]` must match arrays of length
// >= 1, bind `first` via `matchGet(0)`, and bind `rest` to the tail slice via
// `matchSlice(1, matchLength())`. Currently `first` reads garbage (a3) and
// `rest.count` is a garbage count (a4). Pins the minimum-length switch
// (`matchLength() >= 1`), prefix binding, and the rest-slice extraction.

module Test

func head(arr: [Int64]) -> Int64 {
    match arr {
        [first, ..rest] => first,
        [] => -1
    }
}

func tailLen(arr: [Int64]) -> Int64 {
    match arr {
        [_, ..rest] => rest.count,
        [] => -1
    }
}

@main
func main() -> lang.i64 {
    if head([10, 20, 30]) != 10 { return 1 }   // first bound correctly (a3)
    if head([7]) != 7 { return 2 }             // single element: rest is empty
    if head([]) != -1 { return 3 }             // empty hits the `[]` arm
    if tailLen([1, 2, 3, 4]) != 3 { return 4 } // rest = [2,3,4], count 3 (a4)
    if tailLen([9]) != 0 { return 5 }          // rest = [], count 0
    0
}
