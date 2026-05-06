module Life

struct GameState: Cloneable {
    var grid: Grid
    var paused: Bool
    var running: Bool
    var selectedPattern: Pattern
    var seedCounter: UInt64
    var fps: Int64
    var stepDelayMs: Int64
    var generation: Int64
    var frameCount: Int64
    var lastFpsMs: Int64

    init(fromConfig cfg: Config) {
        self.grid = Grid(width: cfg.width, height: cfg.height);
        self.paused = false;
        self.running = true;
        self.selectedPattern = .Glider;
        self.seedCounter = 12648430;
        self.fps = 0;
        self.stepDelayMs = cfg.stepDelayMs;
        self.generation = 0;
        self.frameCount = 0;
        self.lastFpsMs = 0;
        self.grid.randomize(seed: self.seedCounter);
    }

    mutating func randomize() {
        self.seedCounter = self.seedCounter + 1;
        self.grid.randomize(seed: self.seedCounter);
        self.paused = false;
    }

    mutating func clear() {
        self.grid.clear();
        self.paused = true;
    }

    mutating func stamp(px: Int64, py: Int64, cellSize: Int64) {
        let gx = px / cellSize;
        let gy = py / cellSize;
        self.selectedPattern.stamp(on: self.grid, centerX: gx, centerY: gy);
    }

    mutating func updateFps(elapsed: Int64) {
        self.frameCount = self.frameCount + 1;
        let sinceFps = elapsed - self.lastFpsMs;
        if sinceFps >= 500 {
            self.fps = self.frameCount * 1000 / sinceFps;
            self.frameCount = 0;
            self.lastFpsMs = elapsed;
        }
    }
}
