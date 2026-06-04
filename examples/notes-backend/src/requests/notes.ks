module notes.requests

import quill.value.(Value)
import quill.deserialize.(Deserialize, extractString, findKeyOpt)
import quill.error.(DeserializeError)

public struct CreateNoteRequest: Deserialize, Cloneable {
    public var title: String
    public var body: String
    public var folderId: Int64?

    public static func fromValue(value: Value) -> Result[CreateNoteRequest, DeserializeError] {
        let title = try extractString(from: value, "title");
        let body = try extractString(from: value, "body");
        let folderIdVal = try findKeyOpt(from: value, "folderId");
        let folderId: Int64? = match folderIdVal {
            .Some(v) => match v {
                .Int(n) => .Some(n),
                .Null => .None,
                _ => return .Err(DeserializeError.typeMismatch(expected: "int or null", got: v.typeName()))
            },
            .None => .None
        };
        .Ok(CreateNoteRequest(title: title, body: body, folderId: folderId))
    }

    public func clone() -> CreateNoteRequest {
        CreateNoteRequest(
            title: self.title.clone(),
            body: self.body.clone(),
            folderId: self.folderId
        )
    }
}

public struct UpdateNoteRequest: Deserialize, Cloneable {
    public var title: String
    public var body: String

    public static func fromValue(value: Value) -> Result[UpdateNoteRequest, DeserializeError] {
        let title = try extractString(from: value, "title");
        let body = try extractString(from: value, "body");
        .Ok(UpdateNoteRequest(title: title, body: body))
    }

    public func clone() -> UpdateNoteRequest {
        UpdateNoteRequest(
            title: self.title.clone(),
            body: self.body.clone()
        )
    }
}

public struct MoveFolderRequest: Deserialize, Cloneable {
    public var folderId: Int64?

    public static func fromValue(value: Value) -> Result[MoveFolderRequest, DeserializeError] {
        let folderIdVal = try findKeyOpt(from: value, "folderId");
        let folderId: Int64? = match folderIdVal {
            .Some(v) => match v {
                .Int(n) => .Some(n),
                .Null => .None,
                _ => return .Err(DeserializeError.typeMismatch(expected: "int or null", got: v.typeName()))
            },
            .None => .None
        };
        .Ok(MoveFolderRequest(folderId: folderId))
    }

    public func clone() -> MoveFolderRequest {
        MoveFolderRequest(folderId: self.folderId)
    }
}
