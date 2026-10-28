// test: diagnostics
// stdlib: true

// A struct pattern destructures storage, so a computed property is not
// bindable. The pattern roster used a bare `NodeKind::Field` filter and
// included computed properties; binding one made the sub-pattern index read
// past the end of the struct, producing an OSSA block-arg type mismatch ICE
// instead of a diagnostic.

module Main

struct P {
    var x: std.numeric.Int64;
    var doubled: std.numeric.Int64 { get { return self.x * 2; } }
}

@main
func main() -> lang.i64 {
    let p = P(x: 4);
    let r = match p {
        P { x, doubled } => doubled // ERROR: no field `doubled`
    };
    return 0;
}
