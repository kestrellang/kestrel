module notes.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import perch.middleware.(Logger)
import perch.router.(GroupBuilder)
import talon.sqlite.shared_database.(SharedDatabase)
import notes.context.(AppCtx)
import notes.db.(initSchema)
import notes.middleware.(AuthMiddleware)
import notes.handlers.(
    handleRegister, handleLogin,
    handleListNotes, handleCreateNote, handleGetNote, handleUpdateNote, handleDeleteNote, handleMoveNote,
    handleListFolders, handleCreateFolder, handleGetFolder, handleUpdateFolder, handleDeleteFolder
)

func main() {
    match initSchema("notes.db") {
        .Ok(_) => {},
        .Err(e) => {
            println("Failed to initialize database: " + e.description());
            return
        }
    };

    let db = match SharedDatabase("notes.db") {
        .Ok(d) => d,
        .Err(e) => {
            println("Failed to open database: " + e.description());
            return
        }
    };
    let ctx = AppCtx(db: db);

    var app = App[AppCtx](ctx);
    app.use(Logger[AppCtx]());

    // Public auth routes
    app.route(post: "/auth/create", handleRegister);
    app.route(post: "/auth/login", handleLogin);

    // Authenticated note routes
    var noteRoutes = GroupBuilder[AppCtx]("/notes");
    noteRoutes.use(AuthMiddleware());
    noteRoutes.route(get: "", handleListNotes);
    noteRoutes.route(put: "", handleCreateNote);
    noteRoutes.route(get: "/:id", handleGetNote);
    noteRoutes.route(post: "/:id", handleUpdateNote);
    noteRoutes.route(delete: "/:id", handleDeleteNote);
    noteRoutes.route(post: "/:id/folder", handleMoveNote);
    app.addGroup(noteRoutes);

    // Authenticated folder routes
    var folderRoutes = GroupBuilder[AppCtx]("/folders");
    folderRoutes.use(AuthMiddleware());
    folderRoutes.route(get: "", handleListFolders);
    folderRoutes.route(put: "", handleCreateFolder);
    folderRoutes.route(get: "/:id", handleGetFolder);
    folderRoutes.route(post: "/:id", handleUpdateFolder);
    folderRoutes.route(delete: "/:id", handleDeleteFolder);
    app.addGroup(folderRoutes);

    let port: UInt16 = 8080;
    println("Notes API listening on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: " + e.description());
        }
    }
}
