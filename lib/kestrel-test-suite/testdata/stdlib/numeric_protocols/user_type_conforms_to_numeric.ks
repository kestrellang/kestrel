// test: execution
// stdlib: true
// expect-stdout: 7\n

module Test

import std.numeric.(Numeric)

// A user type can join the family by binding each operator's `Output` to
// itself and converting exactly from `Int64`; `sum` then works on it like
// on the built-in numbers.
struct Cents: Numeric {
    var value: Int64

    init(intLiteral value: lang.i64) { self.value = Int64(intLiteral: value) }
    init(value: Int64) { self.value = value }
    init(exactly value: Int64)? { self.value = value }

    type Addable.Output = Cents
    type Subtractable.Output = Cents
    type Multipliable.Output = Cents
    type Divisible.Output = Cents

    static var zero: Cents { Cents(0) }
    static var one: Cents { Cents(1) }

    consuming func add(consuming other: Cents) -> Cents { Cents(self.value + other.value) }
    consuming func subtract(consuming other: Cents) -> Cents { Cents(self.value - other.value) }
    consuming func multiply(consuming other: Cents) -> Cents { Cents(self.value * other.value) }
    consuming func divide(consuming other: Cents) -> Cents { Cents(self.value / other.value) }

    func isEqual(to other: Cents) -> Bool { self.value == other.value }
    func compare(other: Cents) -> Ordering { self.value.compare(other.value) }
}

func sum[T](xs: &Slice[T]) -> T where T: Numeric {
    var total = T.zero;
    for x in xs { total = total + x; }
    total
}

@main
func main() {
    let xs: [Cents] = [Cents(3), Cents(4)];
    println(sum(xs.asSlice()).value);
}
