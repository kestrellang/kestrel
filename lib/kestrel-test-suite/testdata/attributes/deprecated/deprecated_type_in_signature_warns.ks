// test: diagnostics
// stdlib: false

module Test

@deprecated("use 'NewPoint'")
struct OldPoint {}

@deprecated
protocol OldShape {}

func takes(p: OldPoint) {} // WARN: 'OldPoint' is deprecated: use 'NewPoint'

func gives() -> OldPoint { OldPoint() } // WARN: 'OldPoint' is deprecated

struct Holder {
    var p: OldPoint // WARN: 'OldPoint' is deprecated
}

struct Square: OldShape {} // WARN: 'OldShape' is deprecated

func generic[T]() where T: OldShape {} // WARN: 'OldShape' is deprecated

type Alias = OldPoint // WARN: 'OldPoint' is deprecated
