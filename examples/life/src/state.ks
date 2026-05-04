module Life

struct GameState {
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
