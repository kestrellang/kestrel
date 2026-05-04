module Life

import clutch.command.(Command)
import clutch.arg.(Arg)
import clutch.matches.(ArgMatches)
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

        guard let .Ok(matches) = cmd.parse(tokens: args) else {
            return Config(
                width: 80,
                height: 60,
                cellSize: autoCellSize(width: 80, height: 60),
                stepDelayMs: 80,
                headlessIters: 0
            )
        }

        let width = clampParse(matches.getValue(name: "width"), min: 5, max: 2000, default: 80);
        let height = clampParse(matches.getValue(name: "height"), min: 5, max: 2000, default: 60);

        let headlessIters = if matches.hasFlag(name: "headless") {
            clampParse(matches.getValue(name: "iters"), min: 1, max: 999999999, default: 1000)
        } else {
            0
        };

        let cellSize = if headlessIters > 0 {
            1
        } else {
            clampParse(matches.getValue(name: "cell-size"), min: 1, max: 40, default: 0)
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
    var cmd = Command(name: "life");
    cmd.setAbout(text: "Conway's Game of Life — an SDL example");

    var width = Arg(name: "width");
    width.short(flag: "W");
    width.help(text: "Board width (5..2000)");
    width.defaultsTo(value: "80");
    cmd.addArg(arg: width);

    var height = Arg(name: "height");
    height.short(flag: "H");
    height.help(text: "Board height (5..2000)");
    height.defaultsTo(value: "60");
    cmd.addArg(arg: height);

    var cellSize = Arg(name: "cell-size");
    cellSize.short(flag: "c");
    cellSize.help(text: "Cell size in pixels (1..40, auto if omitted)");
    cmd.addArg(arg: cellSize);

    var headless = Arg(name: "headless");
    headless.asFlag();
    headless.help(text: "Run headless benchmark (no window)");
    cmd.addArg(arg: headless);

    var iters = Arg(name: "iters");
    iters.short(flag: "n");
    iters.help(text: "Number of generations for headless mode");
    iters.defaultsTo(value: "1000");
    cmd.addArg(arg: iters);

    cmd
}

func clampParse(value: Optional[String], min lo: Int64, max hi: Int64, default fallback: Int64) -> Int64 {
    guard let .Some(s) = value else {
        return fallback;
    }

    guard let .Some(n) = Int64.parse(s) else {
        return fallback;
    }

    if n < lo or n > hi {
        return fallback
    }

    return n;
}
