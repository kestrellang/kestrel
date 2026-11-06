// test: diagnostics
// stdlib: true

// Reading a `modifying` member mutates the receiver, so it needs a mutable
// place, like calling a `mutating` method.
module Test

import std.numeric.(Int64)

struct Cache {
    var n: Int64

    var next: Int64 {
        modifying get {
            self.n += 1;
            self.n
        }
    }

    subscript(i: Int64) -> Int64 {
        modifying get {
            self.n += i;
            self.n
        }
    }

    func peek() -> Int64 {
        self.next // ERROR(E203): cannot read 'modifying' member 'next' through immutable binding 'self'
    }
}

func use(c: Cache) -> Int64 {
    c.next // ERROR(E203): cannot read 'modifying' member 'next' through immutable binding 'c'
}

func main() {
    let c = Cache(n: 0);
    let a = c.next; // ERROR(E203): cannot read 'modifying' member 'next' through immutable binding 'c'
    let b = c(2); // ERROR(E203): cannot read 'modifying' member 'subscript' through immutable binding 'c'
    var d = Cache(n: 0);
    let e = d.next;
    let f = d(1);
}
