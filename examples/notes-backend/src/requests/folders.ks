module notes.requests

import quill.value.(Value)
import quill.deserialize.(Deserialize, extractString)
import quill.error.(DeserializeError)

public struct CreateFolderRequest: Deserialize, Cloneable {
    public var name: String

    public static func fromValue(value: Value) -> Result[CreateFolderRequest, DeserializeError] {
        let name = try extractString(from: value, "name");
        .Ok(CreateFolderRequest(name: name))
    }

    public func clone() -> CreateFolderRequest {
        CreateFolderRequest(name: self.name.clone())
    }
}

public struct UpdateFolderRequest: Deserialize, Cloneable {
    public var name: String

    public static func fromValue(value: Value) -> Result[UpdateFolderRequest, DeserializeError] {
        let name = try extractString(from: value, "name");
        .Ok(UpdateFolderRequest(name: name))
    }

    public func clone() -> UpdateFolderRequest {
        UpdateFolderRequest(name: self.name.clone())
    }
}
