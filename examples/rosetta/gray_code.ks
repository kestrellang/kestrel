// Rosetta Code: Gray code
// https://rosettacode.org/wiki/Gray_code
module Main

func grayEncode(n: Int64) -> Int64 { n ^ (n >> 1) }

func grayDecode(g: Int64) -> Int64 {
    var n = g;
    var shift = g >> 1;
    while shift != 0 {
        n = n ^ shift;
        shift = shift >> 1;
    }
    n
}

@main
func main() {
    println("dec  bin    gray   decoded");
    for i in 0..<32 {
        let g = grayEncode(i);
        println("\(i:>3)  \(i:05b)  \(g:05b)  \(grayDecode(g):>3)");
    }
}
