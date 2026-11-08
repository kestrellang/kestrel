// Rosetta Code: Show ASCII table
// https://rosettacode.org/wiki/Show_ASCII_table
module Main

func name(code: Int64) -> String {
    if code == 32 { return "Spc" }
    if code == 127 { return "Del" }
    Char(unchecked: UInt32(truncating: code)).toString()
}

@main
func main() {
    for row in 0..<16 {
        var line = StringBuilder();
        for col in 0..<6 {
            let code = 32 + row + col * 16;
            line.append("\(code:>3) : \(name(code):<5)");
        }
        println(line.build().trimmedEnd().toOwned());
    }
}
