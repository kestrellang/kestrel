// Rosetta Code: Maze generation
// https://rosettacode.org/wiki/Maze_generation
// Depth-first search with an explicit stack and a seeded generator.
module Main

import std.numeric.Lcg64

@main
func main() {
    let w = 12;
    let h = 8;
    var rng = Lcg64(seed: 7);
    // Walls: right(i) and down(i) for each cell i = y * w + x.
    var right = Array[Bool](repeating: true, count: w * h);
    var down = Array[Bool](repeating: true, count: w * h);
    var visited = Array[Bool](repeating: false, count: w * h);
    var stack: Array[Int64] = [0];
    visited(0) = true;
    while let .Some(cell) = stack.last() {
        let x = cell % w;
        let y = cell / w;
        var options = Array[Int64]();
        if x > 0 and visited(cell - 1) == false { options.append(cell - 1); }
        if x < w - 1 and visited(cell + 1) == false { options.append(cell + 1); }
        if y > 0 and visited(cell - w) == false { options.append(cell - w); }
        if y < h - 1 and visited(cell + w) == false { options.append(cell + w); }
        if options.isEmpty {
            stack.pop();
            continue
        }
        let next = options(Int64.random(in: 0..<options.count, using: rng));
        // Knock down the wall between cell and next.
        if next == cell + 1 { right(cell) = false; }
        else if next == cell - 1 { right(next) = false; }
        else if next == cell + w { down(cell) = false; }
        else { down(next) = false; }
        visited(next) = true;
        stack.append(next);
    }
    println("+\("---+".repeated(w))");
    for y in 0..<h {
        var top = StringBuilder();
        var bottom = StringBuilder();
        top.append("|");
        bottom.append("+");
        for x in 0..<w {
            let i = y * w + x;
            top.append(if right(i) == true { "   |" } else { "    " });
            bottom.append(if down(i) == true { "---+" } else { "   +" });
        }
        println(top.build());
        println(bottom.build());
    }
}
