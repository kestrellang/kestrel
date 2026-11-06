// test: execution
// stdlib: true
// expect-stdout: cow: 5 [9, 3] 7,3 b! | copy: 10 [1] 1,2 a\nrc: 5 7,2 a! | alias: 5 7 a!\ncomputed: 20\ncomputed: 210\ncomputed: 215\ncomputed: set:z\ncomposition: 11 7\n

// Writing a `{ get set }` property requirement through an existential calls
// the payload's setter through the function table: a plain store, a
// compound assignment, a mutating call on the property, a subscript store
// into it, and a store into one of its fields. `any[CowBox] P` forks first,
// so a copy keeps the old value; the default `any P` (RcBox) writes the
// shared payload, so every copy sees it. A computed witness's setter body
// runs. Before, a plain store wrote raw bytes into the box and a compound
// write failed post-mono ("does not implement 'n.set'").

module Test

protocol A { var n: Int64 { get set } }
protocol B { var m: Int64 { get set } }
struct AB: A, B { var n: Int64  var m: Int64 }
struct Pt { var x: Int64  var y: Int64 }
protocol Named { var name: String { get set } }
protocol HasN: Named, Cloneable { var n: Int64 { get set }  var items: [Int64] { get set }  var pt: Pt { get set } }
protocol Shared: Named { var n: Int64 { get set }  var pt: Pt { get set } }
struct T: HasN, Shared, Cloneable {
    var n: Int64
    var items: [Int64]
    var pt: Pt
    var name: String
    func clone() -> T { T(n: self.n, items: self.items, pt: self.pt, name: self.name) }
}
// A computed property witness: the setter body must run.
struct C: Shared {
    var raw: Int64
    var log: String
    var n: Int64 { get { self.raw } set { self.raw = newValue * 10 } }
    var pt: Pt { get { Pt(x: self.raw, y: 0) } set { self.raw = newValue.x + newValue.y } }
    var name: String { get { self.log } set { self.log = "set:" + newValue } }
}
@main
func main() -> Int64 {
    var c: any[CowBox] HasN = T(n: 10, items: [1], pt: Pt(x: 1, y: 2), name: "a");
    let copy = c;
    c.n = 4;
    c.n += 1;
    c.items.append(3);
    c.items(0) = 9;
    c.pt.x = 7;
    c.pt.y += 1;
    c.name = "b";
    c.name = c.name + "!";
    print("cow: \(c.n) \(c.items) \(c.pt.x),\(c.pt.y) \(c.name) | copy: \(copy.n) \(copy.items) \(copy.pt.x),\(copy.pt.y) \(copy.name)\n");

    var s: any Shared = T(n: 10, items: [1], pt: Pt(x: 1, y: 2), name: "a");
    let alias = s;
    s.n = 4;
    s.n += 1;
    s.pt.x = 7;
    s.name = s.name + "!";
    print("rc: \(s.n) \(s.pt.x),\(s.pt.y) \(s.name) | alias: \(alias.n) \(alias.pt.x) \(alias.name)\n");

    var k: any Shared = C(raw: 1, log: "");
    k.n = 2;
    print("computed: \(k.n)\n");
    k.n += 1;
    print("computed: \(k.n)\n");
    k.pt.y = 5;
    print("computed: \(k.n)\n");
    k.name = "z";
    print("computed: \(k.name)\n");
    var ab: any A and B = AB(n: 1, m: 2);
    ab.n += 10;
    ab.m = 7;
    print("composition: \(ab.n) \(ab.m)\n");
    0
}
