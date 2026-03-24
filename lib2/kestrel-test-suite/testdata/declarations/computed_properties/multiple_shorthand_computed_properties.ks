// test: diagnostics
// stdlib: false

module Test

struct Box {
    var width: lang.i64
    var height: lang.i64
    var depth: lang.i64

    var volume: lang.i64 {
        lang.i64_mul(lang.i64_mul(self.width, self.height), self.depth)
    }

    var surfaceArea: lang.i64 {
        lang.i64_mul(2, lang.i64_add(
            lang.i64_add(
                lang.i64_mul(self.width, self.height),
                lang.i64_mul(self.height, self.depth)
            ),
            lang.i64_mul(self.width, self.depth)
        ))
    }
}
