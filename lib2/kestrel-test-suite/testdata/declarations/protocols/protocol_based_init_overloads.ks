// test: diagnostics
// stdlib: false

module Test

public protocol Convertible[T] {
    init(from other: T)
}

public struct Small {
    var x: lang.i8
    public init() { self.x = lang.cast_i64_i8(0) }
}

public struct Large {
    var x: lang.i32
    public init() { self.x = lang.cast_i64_i32(0) }
}

public struct Target: Convertible[Small], Convertible[Large] {
    var value: lang.i64

    public init(from other: Small) {
        self.value = lang.cast_i8_i64(other.x)
    }

    public init(from other: Large) {
        self.value = lang.cast_i32_i64(other.x)
    }
}

public func test() {
    let s = Small();
    let l = Large();

    // Type-directed conformance: selects init based on argument type
    let t1 = Target(from: s);  // Should call init(from: Small)
    let t2 = Target(from: l);  // Should call init(from: Large)
}
