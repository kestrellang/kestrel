// Rosetta Code: Temperature conversion
// https://rosettacode.org/wiki/Temperature_conversion
module Main

@main
func main() {
    let kelvin = 21.0;
    let celsius = kelvin - 273.15;
    let fahrenheit = celsius * 9.0 / 5.0 + 32.0;
    let rankine = kelvin * 9.0 / 5.0;
    println("K  \(kelvin:.2)");
    println("C  \(celsius:.2)");
    println("F  \(fahrenheit:.2)");
    println("R  \(rankine:.2)");
}
