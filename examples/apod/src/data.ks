// Data layer: small JSON helpers shared with the UI.

module apod.data

import quill.value.(Value)

public func getString(v: Value) -> String {
    match v.asString() {
        .Some(s) => s,
        .None => ""
    }
}

public func getField(obj: Value, key: String) -> Value {
    match obj.value(forKey: key) {
        .Some(v) => v,
        .None => Value.Null
    }
}

/// Returns the field as a string, or an empty string if the key is missing.
public func getStringField(obj: Value, key: String) -> String {
    getString(getField(obj, key))
}
