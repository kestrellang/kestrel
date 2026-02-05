module Test

func main() -> lang.i64 {
    // Test stepBy
    let everyOther = [0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();
    if everyOther.count != 4 { return 1 }

    0
}
