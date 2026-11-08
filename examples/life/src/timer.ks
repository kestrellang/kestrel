module Life

import sdl.(monotonicMs)

struct Timer {
    var startMs: Int64
    var lastTickMs: Int64

    static func start() -> Timer {
        let now = unsafe monotonicMs();
        Timer(startMs: now, lastTickMs: now)
    }

    mutating func tick() -> Int64 {
        let now = unsafe monotonicMs();
        let dt = now - self.lastTickMs;
        self.lastTickMs = now;
        dt
    }

    func elapsed() -> Int64 {
        unsafe monotonicMs() - self.startMs
    }
}
