module ReturnTests

@extern(.C, mangleName: "putchar")
func putchar(consuming c: lang.i32) -> lang.i32

func print_char(c: lang.i32) {
    let _ = putchar(c);
}

func test_nested_if(n: lang.i64) {
    if lang.i64_signed_gt(n, 0) {
        if lang.i64_signed_gt(n, 10) {
            print_char(lang.cast_i64_i32(66)); // 'B'
        } else {
            print_char(lang.cast_i64_i32(83)); // 'S'
        }
    } else {
        print_char(lang.cast_i64_i32(90)); // 'Z'
    }
}

func main() -> lang.i64 {
    test_nested_if(15);
    test_nested_if(5);
    test_nested_if(0);
    print_char(lang.cast_i64_i32(10)); // '\n'
    0
}
