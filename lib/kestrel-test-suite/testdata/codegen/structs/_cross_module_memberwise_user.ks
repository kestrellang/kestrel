// skip: helper module for cross_module_memberwise_construct.ks

module UserSide

import Test.(Point)

public func buildPoint(x: lang.i64, y: lang.i64) -> Point {
    Point(x: x, y: y)
}
