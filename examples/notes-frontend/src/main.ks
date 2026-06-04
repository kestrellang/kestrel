module notes.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import perch.middleware.(Logger)
import http.content.(Html)
import http.cookie.(Cookie)
import quill.value.(Value)
import html.builder.(Document)

import notes.api.(
    apiLogin, apiRegister,
    apiListNotes, apiGetNote, apiCreateNote, apiUpdateNote, apiDeleteNote, apiMoveNote,
    apiListFolders, apiCreateFolder
)
import notes.helpers.(getToken, parseForm, formField)
import notes.ui.(
    loginPage, registerPage,
    appShell, noteListView, noteDetailView, noteEditorView,
    folderSidebar
)

struct Ctx: Cloneable {
    var x: Int64
    func clone() -> Ctx { Ctx(x: self.x) }
}

func render(doc: Document) -> Html { Html(doc.render()) }

func main() {
    var app = App[Ctx](Ctx(x: 0));
    app.use(Logger[Ctx]());

    // --- Auth pages ---

    app.route(get: "/login", handleLoginPage);
    app.route(post: "/login", handleLoginSubmit);
    app.route(get: "/register", handleRegisterPage);
    app.route(post: "/register", handleRegisterSubmit);
    app.route(get: "/logout", handleLogout);

    // --- App pages (full page loads) ---

    app.route(get: "/", handleHome);
    app.route(get: "/new", handleNewNote);
    app.route(get: "/note/:id", handleViewNote);
    app.route(get: "/note/:id/edit", handleEditNote);
    app.route(get: "/folder/:id", handleFolder);

    // --- HTMX fragment endpoints ---

    app.route(get: "/fragments/notes", handleNotesFragment);
    app.route(post: "/fragments/notes", handleCreateNoteFragment);
    app.route(get: "/fragments/note/:id", handleNoteFragment);
    app.route(get: "/fragments/note/:id/edit", handleEditNoteFragment);
    app.route(post: "/fragments/note/:id", handleUpdateNoteFragment);
    app.route(post: "/fragments/note/:id/folder", handleMoveNoteFragment);
    app.route(delete: "/fragments/note/:id", handleDeleteNoteFragment);
    app.route(post: "/fragments/folders", handleCreateFolderFragment);
    app.route(get: "/fragments/sidebar", handleSidebarFragment);

    let port: UInt16 = 3001;
    println("Notes frontend on http://localhost:3001");
    println("(Requires notes-backend running on http://localhost:8080)");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: \(e.description())");
        }
    }
}

// ============================================================================
// Auth handlers
// ============================================================================

func handleLoginPage(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    if token.byteCount > 0 { return Response.redirect(to: "/") };
    Response.ok(render(doc: loginPage("")))
}

func handleLoginSubmit(req: Request, ctx: Ctx) -> Response {
    let fields = parseForm(req.body);
    let email = formField(fields, "email");
    let password = formField(fields, "password");

    guard let .Ok(apiRes) = apiLogin(email, password) else {
        return Response.ok(render(doc: loginPage("Could not connect to the API")))
    }
    guard apiRes.status.isSuccess() else {
        return Response.ok(render(doc: loginPage("Invalid email or password")))
    }
    guard let .Ok(json) = apiRes.json() else {
        return Response.ok(render(doc: loginPage("Unexpected response")))
    }

    let token = match json.value(forKey: "token") {
        .Some(t) => match t {
            .Str(s) => s,
            _ => return Response.ok(render(doc: loginPage("Unexpected response")))
        },
        .None => return Response.ok(render(doc: loginPage("Unexpected response")))
    };

    var cookie = Cookie("token", token);
    cookie.httpOnly = true;
    cookie.path = "/";
    cookie.maxAge = 86400;
    Response.redirect(to: "/").withCookie(cookie)
}

func handleRegisterPage(req: Request, ctx: Ctx) -> Response {
    Response.ok(render(doc: registerPage("")))
}

func handleRegisterSubmit(req: Request, ctx: Ctx) -> Response {
    let fields = parseForm(req.body);
    let email = formField(fields, "email");
    let firstName = formField(fields, "firstName");
    let lastName = formField(fields, "lastName");
    let password = formField(fields, "password");

    guard let .Ok(apiRes) = apiRegister(email, firstName, lastName, password) else {
        return Response.ok(render(doc: registerPage("Could not connect to the API")))
    }
    guard apiRes.status.isSuccess() else {
        let msg = match apiRes.json() {
            .Ok(j) => match j.value(forKey: "error") {
                .Some(e) => match e { .Str(s) => s, _ => "Registration failed" },
                .None => "Registration failed"
            },
            .Err(_) => "Registration failed"
        };
        return Response.ok(render(doc: registerPage(msg)))
    }

    Response.redirect(to: "/login")
}

func handleLogout(req: Request, ctx: Ctx) -> Response {
    var cookie = Cookie("token", "");
    cookie.maxAge = 0;
    cookie.path = "/";
    Response.redirect(to: "/login").withCookie(cookie)
}

// ============================================================================
// Full page handlers
// ============================================================================

func handleHome(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.redirect(to: "/login") }
    renderAppPage(token, 0, "All Notes")
}

func handleFolder(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.redirect(to: "/login") }
    let folderId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.redirect(to: "/")
        },
        .None => return Response.redirect(to: "/")
    };
    renderAppPage(token, folderId, "Folder")
}

func handleNewNote(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.redirect(to: "/login") }
    let folderId = match req.query("folderId") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => 0
        },
        .None => 0
    };
    let folders = loadFolders(token);
    Response.ok(render(doc: appShell("New Note — Notes", folderSidebar(folders, folderId), noteEditorView(.None, folderId))))
}

func handleViewNote(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.redirect(to: "/login") }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.redirect(to: "/")
        },
        .None => return Response.redirect(to: "/")
    };
    guard let .Ok(apiRes) = apiGetNote(token, noteId) else {
        return Response.redirect(to: "/")
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.redirect(to: "/")
    }
    let folders = loadFolders(token);
    Response.ok(render(doc: appShell("Note — Notes", folderSidebar(folders, 0), noteDetailView(note, folders))))
}

func handleEditNote(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.redirect(to: "/login") }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.redirect(to: "/")
        },
        .None => return Response.redirect(to: "/")
    };
    guard let .Ok(apiRes) = apiGetNote(token, noteId) else {
        return Response.redirect(to: "/")
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.redirect(to: "/")
    }
    let noteFolderId = match note.value(forKey: "folderId") {
        .Some(v) => match v { .Int(n) => n, _ => 0 },
        .None => 0
    };
    let folders = loadFolders(token);
    Response.ok(render(doc: appShell("Edit Note — Notes", folderSidebar(folders, 0), noteEditorView(.Some(note), noteFolderId))))
}

// ============================================================================
// HTMX fragment handlers
// ============================================================================

func handleNotesFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let allNotes = loadNotes(token);
    let folderId = match req.query("folderId") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => 0
        },
        .None => 0
    };
    if folderId == 0 {
        Response.ok(render(doc: noteListView(allNotes, "All Notes")))
    } else {
        let filtered = filterByFolder(allNotes, folderId);
        Response.ok(render(doc: noteListView(filtered, "Folder")))
    }
}

func handleNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.badRequest(Html("Invalid id"))
        },
        .None => return Response.badRequest(Html("Missing id"))
    };
    guard let .Ok(apiRes) = apiGetNote(token, noteId) else {
        return Response.internalServerError()
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.internalServerError()
    }
    let folders = loadFolders(token);
    Response.ok(render(doc: noteDetailView(note, folders)))
}

func handleEditNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.badRequest(Html("Invalid id"))
        },
        .None => return Response.badRequest(Html("Missing id"))
    };
    guard let .Ok(apiRes) = apiGetNote(token, noteId) else {
        return Response.internalServerError()
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.internalServerError()
    }
    let noteFolderId = match note.value(forKey: "folderId") {
        .Some(v) => match v { .Int(n) => n, _ => 0 },
        .None => 0
    };
    Response.ok(render(doc: noteEditorView(.Some(note), noteFolderId)))
}

func handleCreateNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let fields = parseForm(req.body);
    let title = formField(fields, "title");
    let body = formField(fields, "body");
    let folderId: Int64? = match Int64(parsing: formField(fields, "folderId")) {
        .Some(n) => if n > 0 { .Some(n) } else { .None },
        .None => .None
    };
    guard let .Ok(apiRes) = apiCreateNote(token, title, body, folderId) else {
        return Response.internalServerError()
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.internalServerError()
    }
    let folders = loadFolders(token);
    Response.ok(render(doc: noteDetailView(note, folders)))
}

func handleUpdateNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.badRequest(Html("Invalid id"))
        },
        .None => return Response.badRequest(Html("Missing id"))
    };
    let fields = parseForm(req.body);
    let title = formField(fields, "title");
    let body = formField(fields, "body");
    guard let .Ok(apiRes) = apiUpdateNote(token, noteId, title, body) else {
        return Response.internalServerError()
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.internalServerError()
    }
    let folders = loadFolders(token);
    Response.ok(render(doc: noteDetailView(note, folders)))
}

func handleMoveNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.badRequest(Html("Invalid id"))
        },
        .None => return Response.badRequest(Html("Missing id"))
    };
    let fields = parseForm(req.body);
    let folderId: Int64? = match Int64(parsing: formField(fields, "folderId")) {
        .Some(n) => if n > 0 { .Some(n) } else { .None },
        .None => .None
    };
    guard let .Ok(apiRes) = apiMoveNote(token, noteId, folderId) else {
        return Response.internalServerError()
    }
    guard let .Ok(note) = apiRes.json() else {
        return Response.internalServerError()
    }
    let folders = loadFolders(token);
    Response.ok(render(doc: noteDetailView(note, folders)))
}

func handleDeleteNoteFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let noteId = match req.param("id") {
        .Some(idStr) => match Int64(parsing: idStr) {
            .Some(n) => n,
            .None => return Response.badRequest(Html("Invalid id"))
        },
        .None => return Response.badRequest(Html("Missing id"))
    };
    guard let .Ok(_) = apiDeleteNote(token, noteId) else {
        return Response.internalServerError()
    }
    let notes = loadNotes(token);
    Response.ok(render(doc: noteListView(notes, "All Notes")))
}

func handleCreateFolderFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    let fields = parseForm(req.body);
    let name = formField(fields, "name");
    guard name.byteCount > 0 else {
        return Response.ok(render(doc: folderSidebar(loadFolders(token), 0)))
    }
    guard let .Ok(_) = apiCreateFolder(token, name) else {
        return Response.internalServerError()
    }
    Response.ok(render(doc: folderSidebar(loadFolders(token), 0)))
}

func handleSidebarFragment(req: Request, ctx: Ctx) -> Response {
    let token = getToken(req);
    guard token.byteCount > 0 else { return Response.unauthorized() }
    Response.ok(render(doc: folderSidebar(loadFolders(token), 0)))
}

// ============================================================================
// Helpers
// ============================================================================

func renderAppPage(token: String, folderId: Int64, title: String) -> Response {
    let notes = loadNotes(token);
    let folders = loadFolders(token);
    Response.ok(render(doc: appShell("\(title) — Notes", folderSidebar(folders, folderId), noteListView(notes, title))))
}

func loadNotes(token: String) -> Array[Value] {
    guard let .Ok(apiRes) = apiListNotes(token, 1) else { return Array[Value]() }
    guard let .Ok(json) = apiRes.json() else { return Array[Value]() }
    match json.value(forKey: "data") {
        .Some(arr) => match arr.asArray() {
            .Some(items) => items,
            .None => Array[Value]()
        },
        .None => Array[Value]()
    }
}

func filterByFolder(notes: Array[Value], folderId: Int64) -> Array[Value] {
    var result = Array[Value]();
    var i: Int64 = 0;
    while i < notes.count {
        let note = notes(unchecked: i);
        let noteFolderId = match note.value(forKey: "folderId") {
            .Some(v) => match v {
                .Int(n) => n,
                _ => 0
            },
            .None => 0
        };
        if noteFolderId == folderId {
            result.append(note)
        };
        i = i + 1
    };
    result
}

func loadFolders(token: String) -> Array[Value] {
    guard let .Ok(apiRes) = apiListFolders(token) else { return Array[Value]() }
    guard let .Ok(json) = apiRes.json() else { return Array[Value]() }
    match json.value(forKey: "data") {
        .Some(arr) => match arr.asArray() {
            .Some(items) => items,
            .None => Array[Value]()
        },
        .None => Array[Value]()
    }
}
