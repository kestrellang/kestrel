module Test

func main() -> lang.i64 {
    let arr1 = [1, 2, 3];
    if arr1.count != 3 { return 1 }
    
    let arr2 = [1, 2, 3].iter().collect();
    if arr2.count != 3 { return 2 }
    
    0
}
