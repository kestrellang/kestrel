// Rosetta Code: Arithmetic-geometric mean
// https://rosettacode.org/wiki/Arithmetic-geometric_mean
module Main

func agm(a0: Float64, g0: Float64) -> Float64 {
    var a = a0;
    var g = g0;
    while (a - g).abs() > 1.0e-15 * a {
        let next = (a + g) / 2.0;
        g = (a * g).sqrt();
        a = next;
    }
    a
}

@main
func main() {
    println(agm(1.0, 1.0 / 2.0.sqrt()));
}
