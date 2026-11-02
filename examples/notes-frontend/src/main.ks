module notes.main

import perch.(
    App, Logger, Get, Post, Delete, Request, Response, HttpError, IntoHttpError, IntoResponse,
    Path, Query, Middleware, MiddlewareResult
)
import http.content.(Html)
import http.cookie.(Cookie)
import http.status.(StatusCode)
import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import swoop.response.(Response as ApiResponse)
import swoop.error.(SwoopError)
import html.builder.(Document)

import notes.api.(
    apiLogin, apiRegister,
    apiListNotes, apiGetNote, apiCreateNote, apiUpdateNote, apiDeleteNote, apiMoveNote,
    apiListFolders, apiCreateFolder
)
import notes.helpers.(Form)
import notes.ui.(
    loginPage, registerPage,
    appShell, noteListView, noteDetailView, noteEditorView,
    folderSidebar
)

// ============================================================================
// Contexts and middleware
// ============================================================================

/// The app-wide context: the frontend keeps no state of its own.
struct AppCtx: Cloneable {
    func clone() -> AppCtx = AppCtx()
}

/// The context behind `Session`: the signed-in user's API token.
struct SessionCtx {
    var token: String
}

/// Reads the `token` cookie into a `SessionCtx`. Without one the request
/// is answered with `otherwise`: a redirect to /login for full pages, a
/// 401 for htmx fragments.
struct Session: Middleware[AppCtx] {
    type Output = SessionCtx
    var otherwise: Response

    func handle(request: Request, ctx: AppCtx) -> MiddlewareResult[SessionCtx] {
        guard let some token = request.cookie("token") else {
            return .Respond(self.otherwise.clone())
        }
        guard token.bytes.count > 0 else {
            return .Respond(self.otherwise.clone())
        }
        .Continue(SessionCtx(token: token))
    }
}

// ============================================================================
// Errors: a failed API call is a 502 for fragment handlers
// ============================================================================

extend SwoopError: IntoHttpError {
    public consuming func intoHttpError() -> HttpError =
        HttpError(StatusCode(502), "Could not reach the notes API")
}

extend DeserializeError: IntoHttpError {
    public consuming func intoHttpError() -> HttpError =
        HttpError(StatusCode(502), "Unexpected response from the notes API")
}

/// The JSON body of an API call.
func apiJson(result: Result[ApiResponse, SwoopError]) -> Value throws HttpError {
    let response = try result;
    try response.json()
}

/// `?folderId=<n>`; 0 (all notes) when absent or not a number.
struct FolderFilter: Deserialize {
    var folderId: Int64

    static func fromValue(value: Value) -> FolderFilter throws DeserializeError {
        guard let some .Str(raw) = value.value(for: "folderId") else { return FolderFilter(folderId: 0) }
        FolderFilter(folderId: Int64(parsing: raw) ?? 0)
    }
}

func render(doc doc: Document) -> Html { Html(doc.render()) }

@main
func main() {
    let app = App(AppCtx()).using(Logger());

    // --- Auth pages ---

    app.on(Get("/login"), Auth.showLogin);
    app.on(Post("/login"), Auth.login);
    app.on(Get("/register"), Auth.showRegister);
    app.on(Post("/register"), Auth.register);
    app.on(Get("/logout"), Auth.logout);

    // --- App pages (full page loads); signed out → /login ---

    let pages = app.group("").using(Session(otherwise: Response.redirect(to: "/login")));
    pages.on(Get("/"), Pages.home);
    pages.on(Get("/new"), Pages.newNote);
    pages.on(Get("/note/:id"), Pages.viewNote);
    pages.on(Get("/note/:id/edit"), Pages.editNote);
    pages.on(Get("/folder/:id"), Pages.folder);

    // --- HTMX fragment endpoints; signed out → 401 ---

    let fragments = app.group("/fragments").using(Session(otherwise: Response.unauthorized()));
    fragments.on(Get("/notes"), Fragments.notes);
    fragments.on(Post("/notes"), Fragments.createNote);
    fragments.on(Get("/note/:id"), Fragments.note);
    fragments.on(Get("/note/:id/edit"), Fragments.editNote);
    fragments.on(Post("/note/:id"), Fragments.updateNote);
    fragments.on(Post("/note/:id/folder"), Fragments.moveNote);
    fragments.on(Delete("/note/:id"), Fragments.deleteNote);
    fragments.on(Post("/folders"), Fragments.createFolder);
    fragments.on(Get("/sidebar"), Fragments.sidebar);

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

struct Auth {
    /// The login form, or home when already signed in.
    static func showLogin(ctx: AppCtx, request: Request) -> Response throws HttpError {
        if let some token = request.cookie("token") {
            if token.bytes.count > 0 { return Response.redirect(to: "/") }
        }
        render(doc: loginPage("")).intoResponse()
    }

    /// Signs in through the API and stores its token in a cookie.
    static func login(ctx: AppCtx, form: Form) -> Response throws HttpError {
        guard let .Ok(apiRes) = apiLogin(form.field("email"), form.field("password")) else {
            return render(doc: loginPage("Could not connect to the API")).intoResponse()
        }
        guard apiRes.status.isSuccess() else {
            return render(doc: loginPage("Invalid email or password")).intoResponse()
        }
        guard let .Ok(json) = apiRes.json() else {
            return render(doc: loginPage("Unexpected response")).intoResponse()
        }
        guard let some .Str(token) = json.value(for: "token") else {
            return render(doc: loginPage("Unexpected response")).intoResponse()
        }

        var cookie = Cookie("token", token);
        cookie.httpOnly = true;
        cookie.path = "/";
        cookie.maxAge = 86400;
        Response.redirect(to: "/").withCookie(cookie)
    }

    static func showRegister(ctx: AppCtx, input: ()) -> Html throws HttpError = render(doc: registerPage(""))

    /// Creates the account, then sends the user to sign in.
    static func register(ctx: AppCtx, form: Form) -> Response throws HttpError {
        let result = apiRegister(form.field("email"), form.field("firstName"), form.field("lastName"), form.field("password"));
        guard let .Ok(apiRes) = result else {
            return render(doc: registerPage("Could not connect to the API")).intoResponse()
        }
        guard apiRes.status.isSuccess() else {
            let msg = match apiRes.json() {
                .Ok(j) => match j.value(for: "error") {
                    .Some(.Str(s)) => s,
                    _ => "Registration failed"
                },
                .Err(_) => "Registration failed"
            };
            return render(doc: registerPage(msg)).intoResponse()
        }

        Response.redirect(to: "/login")
    }

    /// Clears the token cookie.
    static func logout(ctx: AppCtx, input: ()) -> Response throws HttpError {
        var cookie = Cookie("token", "");
        cookie.maxAge = 0;
        cookie.path = "/";
        Response.redirect(to: "/login").withCookie(cookie)
    }
}

// ============================================================================
// Full page handlers: a note the API won't give us sends the user home.
// ============================================================================

struct Pages {
    static func home(ctx: SessionCtx, input: ()) -> Html throws HttpError =
        renderAppPage(ctx.token, 0, "All Notes")

    static func folder(ctx: SessionCtx, Path { value: folderId }: Path[Int64]) -> Html throws HttpError =
        renderAppPage(ctx.token, folderId, "Folder")

    static func newNote(ctx: SessionCtx, Query { value: filter }: Query[FolderFilter]) -> Html throws HttpError {
        let folders = loadFolders(ctx.token);
        render(doc: appShell("New Note — Notes", folderSidebar(folders, filter.folderId), noteEditorView(.None, filter.folderId)))
    }

    static func viewNote(ctx: SessionCtx, Path { value: noteId }: Path[Int64]) -> Response throws HttpError {
        guard let .Ok(note) = apiJson(apiGetNote(ctx.token, noteId)) else {
            return Response.redirect(to: "/")
        }
        let folders = loadFolders(ctx.token);
        render(doc: appShell("Note — Notes", folderSidebar(folders, 0), noteDetailView(note, folders))).intoResponse()
    }

    static func editNote(ctx: SessionCtx, Path { value: noteId }: Path[Int64]) -> Response throws HttpError {
        guard let .Ok(note) = apiJson(apiGetNote(ctx.token, noteId)) else {
            return Response.redirect(to: "/")
        }
        let noteFolderId = noteFolder(note);
        let folders = loadFolders(ctx.token);
        render(doc: appShell("Edit Note — Notes", folderSidebar(folders, 0), noteEditorView(.Some(note), noteFolderId))).intoResponse()
    }
}

// ============================================================================
// HTMX fragment handlers: a failed API call throws (502).
// ============================================================================

struct Fragments {
    static func notes(ctx: SessionCtx, Query { value: filter }: Query[FolderFilter]) -> Html throws HttpError {
        let allNotes = loadNotes(ctx.token);
        if filter.folderId == 0 {
            return render(doc: noteListView(allNotes, "All Notes"))
        };
        render(doc: noteListView(filterByFolder(allNotes, filter.folderId), "Folder"))
    }

    static func note(ctx: SessionCtx, Path { value: noteId }: Path[Int64]) -> Html throws HttpError {
        let note = try apiJson(apiGetNote(ctx.token, noteId));
        render(doc: noteDetailView(note, loadFolders(ctx.token)))
    }

    static func editNote(ctx: SessionCtx, Path { value: noteId }: Path[Int64]) -> Html throws HttpError {
        let note = try apiJson(apiGetNote(ctx.token, noteId));
        let noteFolderId = noteFolder(note);
        render(doc: noteEditorView(.Some(note), noteFolderId))
    }

    static func createNote(ctx: SessionCtx, form: Form) -> Html throws HttpError {
        let note = try apiJson(apiCreateNote(ctx.token, form.field("title"), form.field("body"), form.positiveId("folderId")));
        render(doc: noteDetailView(note, loadFolders(ctx.token)))
    }

    static func updateNote(ctx: SessionCtx, (Path { value: noteId }, form): (Path[Int64], Form)) -> Html throws HttpError {
        let note = try apiJson(apiUpdateNote(ctx.token, noteId, form.field("title"), form.field("body")));
        render(doc: noteDetailView(note, loadFolders(ctx.token)))
    }

    static func moveNote(ctx: SessionCtx, (Path { value: noteId }, form): (Path[Int64], Form)) -> Html throws HttpError {
        let note = try apiJson(apiMoveNote(ctx.token, noteId, form.positiveId("folderId")));
        render(doc: noteDetailView(note, loadFolders(ctx.token)))
    }

    static func deleteNote(ctx: SessionCtx, Path { value: noteId }: Path[Int64]) -> Html throws HttpError {
        let _deleted = try apiDeleteNote(ctx.token, noteId);
        render(doc: noteListView(loadNotes(ctx.token), "All Notes"))
    }

    /// Creates a folder (a blank name creates nothing) and re-renders the sidebar.
    static func createFolder(ctx: SessionCtx, form: Form) -> Html throws HttpError {
        let name = form.field("name");
        if name.bytes.count > 0 {
            let _created = try apiCreateFolder(ctx.token, name);
        };
        render(doc: folderSidebar(loadFolders(ctx.token), 0))
    }

    static func sidebar(ctx: SessionCtx, input: ()) -> Html throws HttpError =
        render(doc: folderSidebar(loadFolders(ctx.token), 0))
}

// ============================================================================
// Helpers
// ============================================================================

func renderAppPage(token: String, folderId: Int64, title: String) -> Html {
    let notes = loadNotes(token);
    let folders = loadFolders(token);
    render(doc: appShell("\(title) — Notes", folderSidebar(folders, folderId), noteListView(notes, title)))
}

/// The note's folder id; 0 when it's in none.
func noteFolder(note: Value) -> Int64 {
    guard let some .Int(n) = note.value(for: "folderId") else { return 0 }
    n
}

func loadNotes(token: String) -> Array[Value] {
    guard let .Ok(apiRes) = apiListNotes(token, 1) else { return Array[Value]() }
    guard let .Ok(json) = apiRes.json() else { return Array[Value]() }
    match json.value(for: "data") {
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
        if noteFolder(note) == folderId {
            result.append(note)
        };
        i = i + 1
    };
    result
}

func loadFolders(token: String) -> Array[Value] {
    guard let .Ok(apiRes) = apiListFolders(token) else { return Array[Value]() }
    guard let .Ok(json) = apiRes.json() else { return Array[Value]() }
    match json.value(for: "data") {
        .Some(arr) => match arr.asArray() {
            .Some(items) => items,
            .None => Array[Value]()
        },
        .None => Array[Value]()
    }
}
