// test: diagnostics
// stdlib: false

module Test

protocol Level3 {
    static func baseValue() -> lang.i64
}

protocol Level2 {
    type Next: Level3;
    func level2() -> lang.i64
}

protocol Level1 {
    type Next: Level2;
}

struct S3: Level3 {
    static func baseValue() -> lang.i64 { return 300; }
}

struct S2: Level2 {
    type Next = S3;
    func level2() -> lang.i64 { return 2; }
}

struct S1: Level1 {
    type Next = S2;
}

struct Wrapper[T] { var val: T }

extend Wrapper[T] where T: Level1 {
    func deepStatic() -> lang.i64 {
        return T.Next.Next.baseValue();
    }
}
