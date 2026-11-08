// test: execution
// stdlib: true
// expect-stdout: computed\n42\n42\n1\n
// backends: cranelift,llvm

// `modifying get` lets a getter change `self` (here: fill a cache) while the
// caller still gets a plain value. Reads need a `var` receiver.
module Test

import std.numeric.(Int64)
import std.streams.(println)

struct Cache {
    var cached: Int64?
    var hits: Int64

    var value: Int64 {
        modifying get {
            if let .Some(v) = self.cached {
                self.hits += 1;
                return v;
            }
            let _ = println("computed");
            self.cached = .Some(42);
            42
        }
    }
}

@main
func main() -> lang.i64 {
    var c = Cache(cached: .None, hits: 0);
    let _ = println("\(c.value)");
    let _ = println("\(c.value)");
    let _ = println("\(c.hits)");
    0
}
