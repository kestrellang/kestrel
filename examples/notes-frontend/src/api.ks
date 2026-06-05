module notes.api

import swoop.swoop.(Swoop)
import swoop.response.(Response)
import swoop.error.(SwoopError)
import swoop.content.(JsonBody)
import quill.value.(Value)

func baseUrl() -> String { "http://localhost:8080" }

func client(token: String) -> Swoop {
    if token.byteCount > 0 {
        Swoop(baseUrl: baseUrl())
            .header("Authorization", "Bearer \(token)")
    } else {
        Swoop(baseUrl: baseUrl())
    }
}

// --- Auth ---

public func apiLogin(email: String, password: String) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("email", Value.Str(email));
    obj.insert("password", Value.Str(password));
    Swoop(baseUrl: baseUrl()).post("/auth/login", JsonBody(fromRaw: Value.Obj(obj)))
}

public func apiRegister(email: String, firstName: String, lastName: String, password: String) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("email", Value.Str(email));
    obj.insert("firstName", Value.Str(firstName));
    obj.insert("lastName", Value.Str(lastName));
    obj.insert("password", Value.Str(password));
    Swoop(baseUrl: baseUrl()).post("/auth/create", JsonBody(fromRaw: Value.Obj(obj)))
}

// --- Notes ---

public func apiListNotes(token: String, page: Int64) -> Result[Response, SwoopError] {
    client(token).fetch("/notes?page=\(page)&per_page=25")
}

public func apiGetNote(token: String, id: Int64) -> Result[Response, SwoopError] {
    client(token).fetch("/notes/\(id)")
}

public func apiCreateNote(token: String, title: String, body: String, folderId: Int64?) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("title", Value.Str(title));
    obj.insert("body", Value.Str(body));
    match folderId {
        .Some(fid) => obj.insert("folderId", Value.Int(fid)),
        .None => obj.insert("folderId", Value.Null),
    };
    client(token).put("/notes", JsonBody(fromRaw: Value.Obj(obj)))
}

public func apiUpdateNote(token: String, id: Int64, title: String, body: String) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("title", Value.Str(title));
    obj.insert("body", Value.Str(body));
    client(token).post("/notes/\(id)", JsonBody(fromRaw: Value.Obj(obj)))
}

public func apiDeleteNote(token: String, id: Int64) -> Result[Response, SwoopError] {
    client(token).delete("/notes/\(id)")
}

public func apiMoveNote(token: String, id: Int64, folderId: Int64?) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    match folderId {
        .Some(fid) => obj.insert("folderId", Value.Int(fid)),
        .None => obj.insert("folderId", Value.Null),
    };
    client(token).post("/notes/\(id)/folder", JsonBody(fromRaw: Value.Obj(obj)))
}

// --- Folders ---

public func apiListFolders(token: String) -> Result[Response, SwoopError] {
    client(token).fetch("/folders?per_page=100")
}

public func apiCreateFolder(token: String, name: String) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("name", Value.Str(name));
    client(token).put("/folders", JsonBody(fromRaw: Value.Obj(obj)))
}

public func apiUpdateFolder(token: String, id: Int64, name: String) -> Result[Response, SwoopError] {
    var obj = Dictionary[String, Value]();
    obj.insert("name", Value.Str(name));
    client(token).post("/folders/\(id)", JsonBody(fromRaw: Value.Obj(obj)))
}

public func apiDeleteFolder(token: String, id: Int64) -> Result[Response, SwoopError] {
    client(token).delete("/folders/\(id)")
}
