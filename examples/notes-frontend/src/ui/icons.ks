module notes.ui

import html.builder.(raw, Document)

public func icon(name: String) -> Document {
    raw("<i data-lucide=\"\(name)\"></i>")
}

public func iconSized(name: String, size: Int64) -> Document {
    raw("<i data-lucide=\"\(name)\" style=\"width:\(size)px;height:\(size)px\"></i>")
}
