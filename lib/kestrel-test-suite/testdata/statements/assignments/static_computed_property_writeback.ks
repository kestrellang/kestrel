// test: execution
// stdlib: true
// expect-stdout: 22 2 3\n

// Read-modify-write through a static get/set property: `S.calc.x += 10`,
// `S.cn -= 1`, and the same through a `static var { get set }` protocol
// requirement in a generic body (`T.count += 1`). Each gets the value,
// modifies it, and calls the setter (#140).
module Test

struct P { var x: Int64 }

struct S {
    static var stored: P = P(x: 1)
    static var calc: P { get { S.stored } set { S.stored = newValue; } }
    static var n: Int64 = 0
    static var cn: Int64 { get { S.n } set { S.n = newValue; } }
}

protocol Counted { static var count: Int64 { get set } }

struct A { static var stored: Int64 = 0 }

extend A: Counted {
    static var count: Int64 { get { A.stored } set { A.stored = newValue; } }
}

func bump[T](t: T) where T: Counted {
    T.count += 1;
    T.count += 2;
}

@main
func main() -> Int64 {
    S.calc.x += 10;
    S.calc.x = S.calc.x * 2;
    S.cn += 3;
    S.cn -= 1;
    bump(A());
    print("\(S.stored.x) \(S.n) \(A.stored)\n");
    0
}
