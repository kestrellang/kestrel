module notes.main

import perch.(App, Logger, Get, Put, Post, Delete, ServerError)
import talon.sqlite.shared_database.(SharedDatabase)
import notes.context.(AppCtx)
import notes.db.(initSchema)
import notes.middleware.(AuthMiddleware)
import notes.handlers.(Auth, Notes, Folders)

@main
func main() -> () throws ServerError {
    try initSchema("notes.db");

    let db = try SharedDatabase("notes.db");
    let app = App(AppCtx(db: db)).using(Logger());

    app.on(Post("/auth/create"), Auth.register);
    app.on(Post("/auth/login"), Auth.login);

    let noteRoutes = app.group("/notes").using(AuthMiddleware());
    noteRoutes.on(Get(), Notes.list);
    noteRoutes.on(Put(), Notes.create);
    noteRoutes.on(Get("/:id"), Notes.read);
    noteRoutes.on(Post("/:id"), Notes.update);
    noteRoutes.on(Delete("/:id"), Notes.delete);
    noteRoutes.on(Post("/:id/folder"), Notes.move);

    let folderRoutes = app.group("/folders").using(AuthMiddleware());
    folderRoutes.on(Get(), Folders.list);
    folderRoutes.on(Put(), Folders.create);
    folderRoutes.on(Get("/:id"), Folders.read);
    folderRoutes.on(Post("/:id"), Folders.update);
    folderRoutes.on(Delete("/:id"), Folders.delete);

    // PORT overrides the default 8080.
    let port: UInt16 = match Environment.variable("PORT") {
        .Some(raw) => UInt16(parsing: raw) ?? 8080,
        .None => 8080
    };
    try app.listen(port);
}
