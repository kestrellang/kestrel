module AbiTests

// Simple externs
@extern(.C, mangleName: "putchar")
func putchar(consuming c: lang.i32) -> lang.i32

func print_char(c: lang.i32) {
    let _ = putchar(c);
}

func print_ok() {
    print_char(lang.cast_i64_i32(79)); // 'O'
    print_char(lang.cast_i64_i32(75)); // 'K'
    print_char(lang.cast_i64_i32(10)); // '\n'
}

struct SimpleStruct {
    var a: lang.i64
    var b: lang.i64
}

enum SimpleEnum {
    case One(lang.i64)
    case Two
}

// Internal function: pass by ref (Kestrel ABI)
func internal_add(a: lang.i64, b: lang.i64) -> lang.i64 {
    lang.i64_add(a, b)
}

// Internal function: struct by ref
func struct_sum(s: SimpleStruct) -> lang.i64 {
    lang.i64_add(s.a, s.b)
}

// Internal function: enum by ref
func enum_val(e: SimpleEnum) -> lang.i64 {
    match e {
        .One(v) => v,
        .Two => 222
    }
}

func main() -> lang.i64 {
    print_char(lang.cast_i64_i32(83)); // 'S'
    print_char(lang.cast_i64_i32(84)); // 'T'
    print_char(lang.cast_i64_i32(65)); // 'A'
    print_char(lang.cast_i64_i32(82)); // 'R'
    print_char(lang.cast_i64_i32(84)); // 'T'
    print_char(lang.cast_i64_i32(10)); // '\n'

    // 1. Test primitives
    let a: lang.i64 = 10;
    let b: lang.i64 = 20;
    let sum = internal_add(a, b);
    if lang.i64_eq(sum, 30) {
        print_char(lang.cast_i64_i32(49)); // '1'
    } else {
        print_char(lang.cast_i64_i32(69)); // 'E'
    }

    // 2. Test struct
    var s = SimpleStruct(a: 100, b: 200);
    let s_sum = struct_sum(s);
    if lang.i64_eq(s_sum, 300) {
        print_char(lang.cast_i64_i32(50)); // '2'
    } else {
        print_char(lang.cast_i64_i32(69)); // 'E'
    }

    // 3. Test enum
    let e1: SimpleEnum = .One(555);
    let v1 = enum_val(e1);
    if lang.i64_eq(v1, 555) {
        print_char(lang.cast_i64_i32(51)); // '3'
    } else {
        print_char(lang.cast_i64_i32(69)); // 'E'
    }

    let e2: SimpleEnum = .Two;
    let v2 = enum_val(e2);
    if lang.i64_eq(v2, 222) {
        print_char(lang.cast_i64_i32(52)); // '4'
    } else {
        print_char(lang.cast_i64_i32(69)); // 'E'
    }

    print_char(lang.cast_i64_i32(10)); // '\n'
    print_ok();

    0
}
