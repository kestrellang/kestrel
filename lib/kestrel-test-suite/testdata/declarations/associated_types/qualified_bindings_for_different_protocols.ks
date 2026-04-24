// test: diagnostics
// stdlib: false
module Test

protocol Addable[Rhs = Self] {
    type Output;
    func add(other: Rhs) -> Output
}

protocol RangeConstructible[Rhs = Self] {
    type Output;
    func exclusiveRange(to end: Rhs) -> Output
}

struct Range[T] {
    init() { }
}

struct MyInt: Addable, RangeConstructible {
    type Addable.Output = MyInt;
    type RangeConstructible.Output = Range[MyInt];

    init() { }

    func add(other: MyInt) -> MyInt { self }
    func exclusiveRange(to end: MyInt) -> Range[MyInt] { Range[MyInt]() }
}
