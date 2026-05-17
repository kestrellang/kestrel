module Life

import clutch.command.(Command)
import clutch.argument.(Argument)
import clutch.matches.(ArgumentMatches)
import clutch.os.(getArgv)

struct Config {
    var width: Int64
    var height: Int64
    var cellSize: Int64
    var stepDelayMs: Int64
    // 0 means interactive; >0 runs N headless generations and exits.
    var headlessIters: Int64

    static func fromArgs() -> Config {
        let cmd = buildCommand();
        let args = getArgv();

        guard let .Ok(matches) = cmd.parse(from: args) else {
            return Config(
                width: 80,
                height: 60,
                cellSize: autoCellSize(width: 80, height: 60),
                stepDelayMs: 80,
                headlessIters: 0
            )
        }

        let width = clampParse(matches.value(of: "width"), min: 5, max: 2000, default: 80);
        let height = clampParse(matches.value(of: "height"), min: 5, max: 2000, default: 60);

        let headlessIters = if matches.hasFlag("headless") {
            clampParse(matches.value(of: "iters"), min: 1, max: 999999999, default: 1000)
        } else {
            0
        };

        let cellSize = if headlessIters > 0 {
            1
        } else {
            clampParse(matches.value(of: "cell-size"), min: 1, max: 40, default: 0)
        };

        let finalCell = if cellSize > 0 { cellSize } else { autoCellSize(width: width, height: height) };

        Config(
            width: width,
            height: height,
            cellSize: finalCell,
            stepDelayMs: 80,
            headlessIters: headlessIters
        )
    }
}

// Picks the largest cell size (in px) that keeps the board inside ~800x600,
// clamped to [2, 40]. Smaller boards stay readable, bigger ones still fit.
func autoCellSize(width w: Int64, height h: Int64) -> Int64 {
    let byW = 800 / w;
    let byH = 600 / h;
    var c = if byW < byH { byW } else { byH };
    if c < 1 { c = 1; }
    if c > 40 { c = 40; }
    c
}

func buildCommand() -> Command {
    var cmd = Command("life");
    cmd = cmd.about("Conway's Game of Life — an SDL example");
    cmd = cmd.argument(Argument("width").short("W").help("Board width (5..2000)").defaultsTo("80"));
    cmd = cmd.argument(Argument("height").short("H").help("Board height (5..2000)").defaultsTo("60"));
    cmd = cmd.argument(Argument("cell-size").short("c").help("Cell size in pixels (1..40, auto if omitted)"));
    cmd = cmd.argument(Argument("headless").toFlag().help("Run headless benchmark (no window)"));
    cmd = cmd.argument(Argument("iters").short("n").help("Number of generations for headless mode").defaultsTo("1000"));
    cmd
}

func clampParse(value: Optional[String], min lo: Int64, max hi: Int64, default fallback: Int64) -> Int64 {
    guard let .Some(s) = value else {
        return fallback;
    }

    guard let .Some(n) = Int64(parsing: s) else {
        return fallback;
    }

    if n < lo or n > hi {
        return fallback
    }

    return n;
}
