// test: diagnostics
// stdlib: false

module Test

public struct Wrapper8 {
    var raw: lang.i8
    public init(raw: lang.i8) { self.raw = raw }
}

public struct Wrapper32 {
    var raw: lang.i32
    public init(raw: lang.i32) { self.raw = raw }
}

// Target struct with differently-labeled inits
public struct Target {
    var value: lang.i64

    public init(from8 value: Wrapper8) {
        self.value = lang.cast_i8_i64(value.raw)
    }

    public init(from32 value: Wrapper32) {
        self.value = lang.cast_i32_i64(value.raw)
    }
}

public func test() {
    let w8 = Wrapper8(lang.cast_i64_i8(1));
    let w32 = Wrapper32(lang.cast_i64_i32(42));

    // These should work - different labels select correct init
    let t1 = Target(from8: w8);
    let t2 = Target(from32: w32);
}
