module Aerie

/// What the environment asks of a run, read once at startup:
/// `AERIE_VIEW` (view distance in chunks), `AERIE_TIME` (time of day, 0 to
/// 1), `AERIE_PLAY=1` (skip the title), `AERIE_SAVE` (a save path, or
/// `none`), `AERIE_FRAMES` (quit after n frames) and `AERIE_BENCH` (time
/// generation and meshing over n runs instead of playing).
struct Settings {
    /// How many chunk columns around the player are loaded and drawn.
    var viewRadius: Int64
    var startTime: Float32?
    var skipTitle: Bool
    var savePath: String?
    /// 0 for no limit.
    var maxFrames: Int64
    var benchRuns: Int64?

    static func fromEnvironment() -> Settings {
        let view = Int64(parsing: getenv("AERIE_VIEW") ?? "") ?? defaultViewRadius;
        let save = getenv("AERIE_SAVE") ?? defaultSavePath();
        Settings(
            viewRadius: view.clamp(minViewRadius, maxViewRadius),
            startTime: Float32(parsing: getenv("AERIE_TIME") ?? ""),
            skipTitle: getenv("AERIE_PLAY").isSome(),
            savePath: if save == "none" { .None } else { .Some(save) },
            maxFrames: Int64(parsing: getenv("AERIE_FRAMES") ?? "") ?? 0,
            benchRuns: match getenv("AERIE_BENCH") {
                .Some(runs) => .Some(Int64(parsing: runs) ?? 10),
                .None => .None
            }
        )
    }

    /// How far chunks are drawn, in blocks.
    var viewDistance: Float32 { Float32(from: self.viewRadius * chunkSize) }
}
