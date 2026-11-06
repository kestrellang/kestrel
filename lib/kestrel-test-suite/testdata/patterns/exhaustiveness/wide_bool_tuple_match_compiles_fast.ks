// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Once the first remaining row matches anything, the decision tree stops
// splitting. Each extra boolean column used to double the tree (15 columns
// took minutes to compile); 18 columns must compile quickly.

module Test

func classify(t: (Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool, Bool)) -> Int64 {
    match t {
        (true, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => 1,
        (_, true, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => 2,
        (_, _, true, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => 3,
        (_, _, _, true, _, _, _, _, _, _, _, _, _, _, _, _, _, _) => 4,
        (_, _, _, _, true, _, _, _, _, _, _, _, _, _, _, _, _, _) => 5,
        (_, _, _, _, _, true, _, _, _, _, _, _, _, _, _, _, _, _) => 6,
        (_, _, _, _, _, _, true, _, _, _, _, _, _, _, _, _, _, _) => 7,
        (_, _, _, _, _, _, _, true, _, _, _, _, _, _, _, _, _, _) => 8,
        (_, _, _, _, _, _, _, _, true, _, _, _, _, _, _, _, _, _) => 9,
        (_, _, _, _, _, _, _, _, _, true, _, _, _, _, _, _, _, _) => 10,
        (_, _, _, _, _, _, _, _, _, _, true, _, _, _, _, _, _, _) => 11,
        (_, _, _, _, _, _, _, _, _, _, _, true, _, _, _, _, _, _) => 12,
        (_, _, _, _, _, _, _, _, _, _, _, _, true, _, _, _, _, _) => 13,
        (_, _, _, _, _, _, _, _, _, _, _, _, _, true, _, _, _, _) => 14,
        (_, _, _, _, _, _, _, _, _, _, _, _, _, _, true, _, _, _) => 15,
        (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, true, _, _) => 16,
        (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, true, _) => 17,
        (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, true) => 18,
        _ => 0
    }
}

@main
func main() -> lang.i64 {
    if classify((false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true)) != 18 { return 1 }
    if classify((false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false)) != 0 { return 2 }
    0
}
