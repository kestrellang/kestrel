module Life

import sdl.(Color, Rectangle, Renderer, SDLApp, Event)

protocol GameRenderer {
    mutating func render(state: GameState)
    func finish(state: GameState, elapsed elapsedMs: Int64)
}

struct SdlGameRenderer: GameRenderer, InputManager {
    var app: SDLApp
    var cellSize: Int64

    init(config cfg: Config) {
        let windowW = cfg.width * cfg.cellSize;
        let windowH = cfg.height * cfg.cellSize;
        self.app = SDLApp(title: "Game of Life", width: windowW, height: windowH);
        self.cellSize = cfg.cellSize;
    }

    mutating func getEvent() -> Optional[Event] {
        self.app.pollEvent()
    }

    mutating func render(state: GameState) {
        let grid = state.grid;
        let cell = self.cellSize;
        let patternText = "PATTERN: " + state.selectedPattern.formatted();
        let fpsText = "FPS: " + state.fps.formatted();
        self.app.render { (renderer) in
            renderer.clear(Color.black());

            renderer.setColor(Color.green());
            let gutter = if cell >= 4 { 1 } else { 0 };
            for y in 0..<grid.height {
                for x in 0..<grid.width {
                    if grid.cellAt(x: x, y: y) {
                        let rect = Rectangle(
                            x: x * cell,
                            y: y * cell,
                            width: cell - gutter,
                            height: cell - gutter
                        );
                        renderer.fillRect(rect);
                    }
                }
            }

            renderer.drawText(patternText, 8, 8, 2);
            renderer.drawText(fpsText, 8, 28, 2);
        };
    }

    func finish(state: GameState, elapsed elapsedMs: Int64) {}
}

struct HeadlessRenderer: GameRenderer {
    var width: Int64
    var height: Int64

    init(config cfg: Config) {
        self.width = cfg.width;
        self.height = cfg.height;
    }

    mutating func render(state: GameState) {}

    func finish(state: GameState, elapsed elapsedMs: Int64) {
        let denom = if elapsedMs > 0 { elapsedMs } else { 1 };
        let gensPerSec = state.generation * 1000 / denom;
        println("\(self.width)x\(self.height)  gens=\(state.generation)  elapsed_ms=\(elapsedMs)  gens_per_sec=\(gensPerSec)");
    }
}
