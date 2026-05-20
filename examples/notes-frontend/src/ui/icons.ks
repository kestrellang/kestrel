module notes.ui

// Lucide icon element — rendered client-side by the lucide script.
public func icon(name: String) -> String {
    "<i data-lucide=\"\(name)\"></i>"
}

public func iconSized(name: String, size: Int64) -> String {
    "<i data-lucide=\"\(name)\" style=\"width:\(size)px;height:\(size)px\"></i>"
}
