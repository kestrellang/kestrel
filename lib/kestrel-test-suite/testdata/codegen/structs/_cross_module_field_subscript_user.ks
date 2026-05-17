// skip: helper module for cross_module_field_subscript.ks

module UserSide

import Test.(Bag)
import std.text.(String)

public func firstItem(b: Bag) -> String {
    b.items(unchecked: 0)
}
