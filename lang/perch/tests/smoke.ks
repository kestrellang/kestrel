module perch.tests

import perch.(App, Get)

struct Ctx: Cloneable { var n: Int64; func clone() -> Ctx { Ctx(n: self.n) } }

@main
func main() -> Int64 { 0 }
