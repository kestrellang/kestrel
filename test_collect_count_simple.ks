module Test

func main() -> lang.i64 {
    let arr2 = [1, 2, 3].iter().collect();
    let c: std.num.Int64 = arr2.count;
    c
}
