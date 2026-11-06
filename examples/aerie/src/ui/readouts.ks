module Aerie

/// Frames per second, measured over half a second at a time.
struct FpsCounter {
    var shown: Bool = false
    /// The last measured rate, and frames and seconds toward the next.
    var rate: Int64 = 0
    var frames: Int64 = 0
    var seconds: Float32 = 0.0

    init() {}

    /// Count a frame of `dt` seconds; `toggle` shows or hides the readout.
    mutating func count(dt: Float32, toggle toggle: Bool) {
        if toggle {
            self.shown = not self.shown;
        }
        self.frames = self.frames + 1;
        self.seconds = self.seconds + dt;
        if self.seconds < 0.5 {
            return;
        }
        self.rate = (Float32(from: self.frames) / self.seconds + 0.5).toInt64() ?? 0;
        self.frames = 0;
        self.seconds = 0.0;
    }

    /// The readout in the corner, or nothing while it's hidden.
    func quads() -> [OverlayQuad] {
        guard self.shown else {
            return [];
        }
        cornerLabel("\(self.rate) FPS", row: 0, alpha: 1.0)
    }
}

/// A short message in the corner, shown for a few seconds and fading out
/// over its last one.
struct Notice {
    var text: String = ""
    /// Seconds left to show it.
    var remaining: Float32 = 0.0

    init() {}

    mutating func show(text: String) {
        self.text = text;
        self.remaining = 3.0;
    }

    mutating func update(dt: Float32) {
        self.remaining = self.remaining - dt;
    }

    /// The notice `row` boards down the corner, or nothing once it's gone.
    func quads(row row: Int64) -> [OverlayQuad] {
        guard self.remaining > 0.0 else {
            return [];
        }
        cornerLabel(self.text, row: row, alpha: self.remaining.clamp(0.0, 1.0))
    }
}
