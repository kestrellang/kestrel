// skip: included file, not a standalone test
module Prelude

@builtin(.Matchable)
public protocol Matchable {
    func matches(other: Self) -> lang.i1
}
