// test: diagnostics
// stdlib: true
//
// E145: requirements that can't go in a function table can't be called on an
// existential: static, generic, `Self` outside the receiver, or an unbound
// associated type (design §5).

module Test

import std.numeric.(Int64)
import std.core.(Bool)
import std.iter.(Iterator)

protocol Thing {
    static func make() -> Int64
    func pick[T](x: T) -> T
    func same(other: Self) -> Bool
    func name() -> Int64
}

struct A {}
extend A: Thing {
    public static func make() -> Int64 { 1 }
    public func pick[T](x: T) -> T { x }
    public func same(other: A) -> Bool { true }
    public func name() -> Int64 { 2 }
}

struct Counter { var n: Int64; }
extend Counter: Iterator {
    type Item = Int64
    public mutating func next() -> Int64? {
        self.n = self.n + 1;
        .Some(self.n)
    }
}

func test(t: any Thing, other: any Thing) {
    let ok = t.name();
    let a = t.make(); // ERROR: 'make' cannot be called on 'any Thing': it is a static requirement
    let b = t.pick(1); // ERROR: 'pick' cannot be called on 'any Thing': it is generic
    let c = t.same(other); // ERROR: 'same' cannot be called on 'any Thing': it uses 'Self' as a parameter or return type
}

func iter() {
    var it: any Iterator = Counter(n: 0);
    let x = it.next(); // ERROR: 'next' cannot be called on 'any Iterator': it mentions associated type 'Item', which 'any Iterator' leaves unbound
}
