// Rosetta Code: Mandelbrot set
// https://rosettacode.org/wiki/Mandelbrot_set
module Main

@main
func main() {
    let palette = " .:-=+*#%@".chars.iter().collect();
    let maxIter = 50;
    for row in 0..<25 {
        var line = StringBuilder();
        for col in 0..<78 {
            let cr = -2.2 + Float64(from: col) * 0.035;
            let ci = -1.2 + Float64(from: row) * 0.1;
            var (zr, zi) = (0.0, 0.0);
            var i: Int64 = 0;
            while i < maxIter and zr * zr + zi * zi <= 4.0 {
                let t = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = t;
                i = i + 1;
            }
            let shade = if i == maxIter { palette.count - 1 } else { i % (palette.count - 1) };
            line.append(char: palette(shade));
        }
        println(line.build().trimmedEnd().toOwned());
    }
}
