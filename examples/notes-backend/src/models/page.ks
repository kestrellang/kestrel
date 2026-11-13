module notes.models

import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)
import notes.requests.(Pagination)

// Paginated response envelope: {"data": [...], "page", "perPage", "total"}.
public struct Page[T]: Serialize where T: Serialize {
    public var items: Array[T]
    public var pagination: Pagination
    public var total: Int64

    public init(items: Array[T], pagination: Pagination, total: Int64) {
        self.items = items;
        self.pagination = pagination;
        self.total = total;
    }

    public func toValue() -> Result[Value, SerializeError] {
        // quill's Value isn't dictionary-literal expressible; build the object.
        var obj: Dictionary[String, Value] = [:];
        obj.insert("data", try self.items.toValue());
        obj.insert("page", .Int(self.pagination.page));
        obj.insert("perPage", .Int(self.pagination.perPage));
        obj.insert("total", .Int(self.total));
        .Ok(.Obj(obj))
    }
}
