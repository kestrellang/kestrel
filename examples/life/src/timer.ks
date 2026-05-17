module Life

@extern(.C, mangleName: "Kestrel_MonotonicMs")
func monotonicMs() -> Int64

struct Timer {
    var startMs: Int64
    var lastTickMs: Int64

    static func start() -> Timer {
        let now = monotonicMs();
        Timer(startMs: now, lastTickMs: now)
    }

    mutating func tick() -> Int64 {
        let now = monotonicMs();
        let dt = now - self.lastTickMs;
        self.lastTickMs = now;
        dt
    }

    func elapsed() -> Int64 {
        monotonicMs() - self.startMs
    }
}
