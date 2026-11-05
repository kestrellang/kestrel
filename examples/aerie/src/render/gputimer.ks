module Aerie

import wgpu.(Device, QuerySet, PassTimestamps, WgpuError)

/// A pass whose GPU time `GpuTimer` measures.
enum TimedPass {
    case Shadow
    case Main

    static let all: [TimedPass] = [.Shadow, .Main];

    var index: Int64 {
        match self { .Shadow => 0, .Main => 1 }
    }

    var name: String {
        match self { .Shadow => "shadow", .Main => "main" }
    }
}

/// How often `GpuTimer` reads its timestamps back, in frames. Each read
/// waits for the GPU to finish, so it's kept rare.
let gpuTimingInterval: Int64 = 30;

/// GPU time per pass, from timestamps the GPU writes at each pass's start
/// and end. The display caps how fast frames are shown, so this is the
/// only way to see how much headroom the GPU has.
struct GpuTimer: not Copyable {
    /// Two timestamps per `TimedPass`: its start, then its end. None
    /// until `start`, or when the device can't time passes.
    var queries: QuerySet?
    /// Nanoseconds spent in each pass over the frames sampled.
    var totals: [Int64]
    var samples: Int64
    var frame: Int64

    init() {
        self.queries = .None;
        self.totals = [0, 0];
        self.samples = 0;
        self.frame = 0;
    }

    /// Start timing passes, if `device` was made with timestamp queries.
    mutating func start(device: Device) {
        guard device.supports(.TimestampQuery) else {
            return;
        }
        self.queries = .Some(device.createQuerySet(kind: .Timestamp, count: TimedPass.all.count * 2, label: "pass times"));
    }

    /// Where `pass` writes its start and end times, while timing.
    func timestamps(pass: TimedPass) -> PassTimestamps? {
        match self.queries {
            .Some(q) => .Some(q.timestamps(beginning: .Some(pass.index * 2), end: .Some(pass.index * 2 + 1))),
            .None => .None
        }
    }

    /// After a frame is submitted: every `gpuTimingInterval` frames, wait
    /// for it and add up how long each pass took.
    mutating func collect(device: Device) -> () throws WgpuError {
        self.frame = self.frame + 1;
        guard self.frame % gpuTimingInterval == 0 else {
            return .Ok(());
        }
        let stamps = match self.queries {
            .Some(q) => try device.readQueries(q),
            .None => { return .Ok(()); }
        };
        for pass in TimedPass.all {
            let (start, end) = (stamps(pass.index * 2), stamps(pass.index * 2 + 1));
            self.totals(pass.index) = self.totals(pass.index) + Int64(from: end - start);
        }
        self.samples = self.samples + 1;
        .Ok(())
    }

    /// Average GPU milliseconds per pass, for the run's report.
    func summary() -> String {
        guard self.samples > 0 else {
            return "gpu: no samples";
        }
        var out = "gpu ms per frame:";
        for pass in TimedPass.all {
            out = out + " \(pass.name) \(Float64(from: self.totals(pass.index) / self.samples) / 1000000.0),";
        }
        out
    }
}
