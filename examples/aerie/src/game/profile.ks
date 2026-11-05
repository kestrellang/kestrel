module Aerie

import sdl.(monotonicUs)

/// A part of a frame that `FrameProfile` times.
enum FrameStage {
    /// Player, birds, sky and the HUD's state.
    case Update
    /// Generating, meshing and uploading chunks as the player moves.
    case Stream
    /// Gathering the boxes, quads and icons to draw.
    case Scene
    /// Culling, encoding and submitting, and waiting for a frame to draw into.
    case Draw

    static let all: [FrameStage] = [.Update, .Stream, .Scene, .Draw];

    var index: Int64 {
        match self { .Update => 0, .Stream => 1, .Scene => 2, .Draw => 3 }
    }

    var name: String {
        match self { .Update => "update", .Stream => "stream", .Scene => "scene", .Draw => "draw" }
    }
}

/// Where frame time goes over a run, in microseconds, for the run's report.
struct FrameProfile {
    /// Time spent in each stage, indexed by `FrameStage.index`.
    var totals: [Int64]
    /// The slowest whole frame so far.
    var worst: Int64
    var frames: Int64
    /// When the current frame, and the stage now running, began.
    var frameStart: Int64
    var stageStart: Int64

    init() {
        self.totals = [0, 0, 0, 0];
        self.worst = 0;
        self.frames = 0;
        self.frameStart = 0;
        self.stageStart = 0;
    }

    /// Start timing a frame.
    mutating func begin() {
        self.frameStart = unsafe monotonicUs();
        self.stageStart = self.frameStart;
    }

    /// Charge the time since the last mark to `stage`.
    mutating func mark(stage: FrameStage) {
        let now = unsafe monotonicUs();
        self.totals(stage.index) = self.totals(stage.index) + now - self.stageStart;
        self.stageStart = now;
    }

    /// Finish timing a frame.
    mutating func end() {
        self.frames = self.frames + 1;
        let took = self.stageStart - self.frameStart;
        self.worst = if took > self.worst { took } else { self.worst };
    }

    /// Average milliseconds per frame in each stage, and the worst frame.
    func summary() -> String {
        guard self.frames > 0 else {
            return "no frames timed";
        }
        var out = "ms per frame:";
        for stage in FrameStage.all {
            out = out + " \(stage.name) \(milliseconds(self.totals(stage.index) / self.frames)),";
        }
        out + " worst frame \(milliseconds(self.worst))"
    }
}

/// `micros` microseconds as milliseconds, for printing.
func milliseconds(micros: Int64) -> Float64 {
    Float64(from: micros) / 1000.0
}
