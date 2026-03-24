// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(T)
    case None
}

struct Container[T] {
    var ptr: lang.ptr[T]

    init(maybeValue: Option[lang.ptr[T]]) {
        match maybeValue {
            .Some(rawPtr) => {
                self.ptr = rawPtr;
            },
            .None => {
                // This branch doesn't initialize ptr and doesn't diverge
            }
        }
    } // ERROR: does not initialize all fields
}
