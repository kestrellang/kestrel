// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

struct Counter {
    var count: lang.i64

    init(start: lang.i64) {
        self.count = start;
    }

    mutating func next() -> Option[lang.i64] {
        if lang.i64_signed_gt(self.count, 0) {
            let v = self.count;
            self.count = lang.i64_sub(self.count, 1);
            Option[lang.i64].Some(value: v)
        } else {
            Option[lang.i64].None
        }
    }
}

func sumAll() -> lang.i64 {
    var counter = Counter(5);
    var sum: lang.i64 = 0;
    while let .Some(v) = counter.next() {
        sum = lang.i64_add(sum, v);
    }
    sum
}
