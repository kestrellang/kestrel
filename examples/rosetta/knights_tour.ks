// Rosetta Code: Knight's tour
// https://rosettacode.org/wiki/Knight%27s_tour
module Main

let dx: Array[Int64] = [1, 2, 2, 1, -1, -2, -2, -1];
let dy: Array[Int64] = [-2, -1, 1, 2, 2, 1, -1, -2];

func onBoard(x: Int64, y: Int64) -> Bool { x >= 0 and x < 8 and y >= 0 and y < 8 }

func degree(board: Array[Int64], x: Int64, y: Int64) -> Int64 {
    (0..<8).iter().filter { (k) in
        let nx = x + dx(k);
        let ny = y + dy(k);
        onBoard(nx, ny) and board(ny * 8 + nx) == 0
    }.count()
}

// Warnsdorff's rule: always move to the square with the fewest onward moves.
@main
func main() {
    var board = Array[Int64](repeating: 0, count: 64);
    var (x, y) = (0, 0);
    board(0) = 1;
    for move in 2..=64 {
        var bestK: Int64 = -1;
        var bestDegree = 9;
        for k in 0..<8 {
            let nx = x + dx(k);
            let ny = y + dy(k);
            if onBoard(nx, ny) and board(ny * 8 + nx) == 0 {
                let d = degree(board, nx, ny);
                if d < bestDegree {
                    bestDegree = d;
                    bestK = k;
                }
            }
        }
        if bestK < 0 { println("stuck at move \(move)"); return }
        x = x + dx(bestK);
        y = y + dy(bestK);
        board(y * 8 + x) = move;
    }
    for row in 0..<8 {
        println((0..<8).iter().map { "\(board(row * 8 + it):>3)" }.collect().joined(""));
    }
}
