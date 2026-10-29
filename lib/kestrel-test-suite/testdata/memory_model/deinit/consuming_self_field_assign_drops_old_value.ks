// test: execution
// stdlib: true
// expect-stdout: drop res 1drop res 2deinit new 3drop res 3
//
// Assigning heap-owning fields on a `consuming self`: each replaced field
// drops exactly once, returning `self` carries the new fields, and the value
// that finally drops (running `deinit`) has the last-assigned state.

module Test
import std.numeric.(Int64)
import std.text.(String)
import std.io.stdio.(print)
struct Res: not Copyable {
    var id: Int64
    deinit { print("drop res \(self.id)") }
}
struct Tok: not Copyable {
    var name: String
    var res: Res
    consuming func renamed() -> Tok { self.name = "new"; self.res = Res(id: 2); self }
    consuming func finish() { self.res = Res(id: 3); }
    deinit { print("deinit \(self.name) \(self.res.id)") }
}
@main func main() -> lang.i64 {
    let t = Tok(name: "old", res: Res(id: 1));
    let u = t.renamed();
    u.finish();
    0
}
