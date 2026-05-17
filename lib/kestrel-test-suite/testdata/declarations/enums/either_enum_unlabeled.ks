// test: diagnostics
// stdlib: false

module Test

enum Either[L, R] {
    case Left(L)
    case Right(R)
}

func fold[L, R, Out](
    either: Either[L, R],
    onLeft: (L) -> Out,
    onRight: (R) -> Out
) -> Out {
    match either {
        .Left(l) => onLeft(l),
        .Right(r) => onRight(r)
    }
}
