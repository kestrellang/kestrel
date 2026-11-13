module Aerie

/// Seconds in one day.
let dayLength: Float32 = 480.0;

/// The time of day and the lairs' gloom, which together set the light.
struct Sky {
    /// 0 to 1 over a day; see `Daylight.at`.
    var timeOfDay: Float32
    /// How much the lairs' crimson sky shows, 0 to 1, easing toward how
    /// deep in lair country the camera is.
    var gloom: Float32 = 0.0

    init(timeOfDay timeOfDay: Float32) {
        self.timeOfDay = timeOfDay;
    }

    /// Run the clock `dt` seconds, and ease the gloom toward `lairness`
    /// over a couple of seconds.
    mutating func update(dt: Float32, lairness lairness: Float32) {
        let t = self.timeOfDay + dt / dayLength;
        self.timeOfDay = t - t.floor();
        self.gloom = self.gloom + (lairness - self.gloom) * (dt * 0.6).clamp(0.0, 1.0);
    }

    var daylight: Daylight { Daylight.at(self.timeOfDay, gloom: self.gloom) }
}

/// Counts down to the next autosave.
struct Autosave {
    /// Seconds between saves.
    var interval: Float32 = 60.0
    var remaining: Float32 = 60.0

    init() {}

    /// Count `dt` seconds of play; true when it's time to save.
    mutating func due(dt: Float32) -> Bool {
        self.remaining = self.remaining - dt;
        guard self.remaining <= 0.0 else {
            return false;
        }
        self.remaining = self.interval;
        true
    }
}
