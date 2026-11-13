// Rosetta Code: Trigonometric functions
// https://rosettacode.org/wiki/Trigonometric_functions
module Main

@main
func main() {
    let radians = Float64.pi / 4.0;
    let degrees = 45.0;
    let toRadians = Float64.pi / 180.0;
    println("sin: \(radians.sin():.6) \((degrees * toRadians).sin():.6)");
    println("cos: \(radians.cos():.6) \((degrees * toRadians).cos():.6)");
    println("tan: \(radians.tan():.6) \((degrees * toRadians).tan():.6)");
    let asin = radians.sin().asin();
    println("asin: \(asin:.6) \(asin / toRadians:.6)");
    let acos = radians.cos().acos();
    println("acos: \(acos:.6) \(acos / toRadians:.6)");
    let atan = radians.tan().atan();
    println("atan: \(atan:.6) \(atan / toRadians:.6)");
}
