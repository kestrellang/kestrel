module InitTests

@extern(.C, mangleName: "putchar")
func putchar(consuming c: lang.i32) -> lang.i32

func print_char(c: lang.i32) {
    let _ = putchar(c);
}

struct TestStruct {
    var x: lang.i64

    public init(n: lang.i64) {
        if lang.i64_signed_gt(n, 0) {
            self.x = 1;
        } else {
            self.x = 0;
        }
    }
}

func main() -> lang.i64 {
    let s1 = TestStruct(n: 1);
    if lang.i64_eq(s1.x, 1) {
        print_char(lang.cast_i64_i32(49)); // '1'
    }

    let s2 = TestStruct(n: 0);
    if lang.i64_eq(s2.x, 0) {
        print_char(lang.cast_i64_i32(50)); // '2'
    }

    print_char(lang.cast_i64_i32(10)); // '\n'
    0
}
