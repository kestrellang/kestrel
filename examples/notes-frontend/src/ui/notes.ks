module notes.ui

import quill.value.(Value)
import notes.html.(
    raw, text, nothing, el,
    div, span, h1, h2, p, anchor, button, form, label, input, textarea, select, option,
    spacer,
    cls, id, href, attr, boolAttr,
    hxGet, hxPost, hxPut, hxDelete, hxTarget, hxSwap, hxPushUrl, hxConfirm
)

public func noteListView(notes: Array[Value], listTitle: String) -> String {
    let items = if notes.count == 0 { emptyState() } else { noteList(notes) };
    div {
        div([cls("note-list-header")]) {
            h1([cls("note-list-title")]) { text(listTitle) }
            + anchor([cls("btn btn-primary btn-sm"), href("/new")]) {
                iconSized("plus", 14) + span { "New Note" }
            }
        }
        + items
    }
}

func noteList(notes: Array[Value]) -> String {
    var cards = String(capacity: 4096);
    var i: Int64 = 0;
    while i < notes.count {
        cards.append(noteCard(notes(unchecked: i)));
        i = i + 1
    };
    div([cls("note-list")]) { cards }
}

func noteCard(note: Value) -> String {
    let nid = getInt(note, "id");
    let noteTitle = getStr(note, "title");
    let noteBody = getStr(note, "body");
    let updatedAt = getStr(note, "updatedAt");

    let preview = if noteBody.byteCount > 140 {
        noteBody.asSlice().subslice(from: 0, to: 140).toOwned() + "..."
    } else {
        noteBody
    };

    anchor([cls("note-card"), href("/note/\(nid)"),
            hxGet("/fragments/note/\(nid)"), hxTarget("#content"),
            hxSwap("innerHTML"), hxPushUrl("/note/\(nid)")]) {
        div([cls("note-card-title")]) { text(noteTitle) }
        + div([cls("note-card-preview")]) { text(preview) }
        + div([cls("note-card-meta")]) {
            iconSized("clock", 12)
            + el("time", [attr("datetime", updatedAt)]) { text(updatedAt) }
        }
    }
}

public func noteDetailView(note: Value, folders: Array[Value]) -> String {
    let nid = getInt(note, "id");
    let noteTitle = getStr(note, "title");
    let noteBody = getStr(note, "body");
    let updatedAt = getStr(note, "updatedAt");
    let noteFolderId = getOptInt(note, "folderId");

    div([cls("editor")]) {
        detailToolbar(nid, updatedAt, noteFolderId, folders)
        + h1([cls("note-title")]) { text(noteTitle) }
        + div([cls("note-body")]) { text(noteBody) }
    }
}

func detailToolbar(nid: Int64, updatedAt: String, folderId: Int64, folders: Array[Value]) -> String {
    div([cls("editor-toolbar")]) {
        navButton("/", "/fragments/notes", "/", "arrow-left", "Back")
        + navButton("/note/\(nid)/edit", "/fragments/note/\(nid)/edit",
                    "/note/\(nid)/edit", "pencil", "Edit")
        + button([cls("btn btn-danger btn-sm"),
                  hxDelete("/fragments/note/\(nid)"), hxTarget("#content"),
                  hxSwap("innerHTML"), hxConfirm("Delete this note?")]) {
            iconSized("trash-2", 14) + span { "Delete" }
        }
        + folderPicker(nid, folderId, folders)
        + spacer()
        + span([cls("note-meta")]) {
            iconSized("clock", 12)
            + el("time", [attr("datetime", updatedAt)]) { text(updatedAt) }
        }
    }
}

func folderPicker(nid: Int64, currentFolderId: Int64, folders: Array[Value]) -> String {
    let noneSelected = if currentFolderId == 0 { " selected" } else { "" };
    var opts = "<option value=\"0\"\(noneSelected)>No folder</option>";
    var i: Int64 = 0;
    while i < folders.count {
        let folder = folders(unchecked: i);
        let fid = getInt(folder, "id");
        let name = getStr(folder, "name");
        let sel = if fid == currentFolderId { " selected" } else { "" };
        opts.append("<option value=\"\(fid)\"\(sel)>");
        opts.append(text(name));
        opts.append("</option>");
        i = i + 1
    };
    select([cls("folder-picker"),
            attr("name", "folderId"),
            attr("hx-post", "/fragments/note/\(nid)/folder"),
            hxTarget("#content"), hxSwap("innerHTML"),
            attr("hx-include", "closest select"),
            attr("title", "Move to folder")]) { opts }
}

public func noteEditorView(note: Value?, folderId: Int64) -> String {
    let isNew = match note {
        .Some(n) => false,
        .None => true
    };
    let nid = match note {
        .Some(n) => getInt(n, "id"),
        .None => 0
    };
    let currentTitle = match note {
        .Some(n) => getStr(n, "title"),
        .None => ""
    };
    let currentBody = match note {
        .Some(n) => getStr(n, "body"),
        .None => ""
    };
    let formAction = if isNew { "/fragments/notes" } else { "/fragments/note/\(nid)" };
    let submitLabel = if isNew { "Create Note" } else { "Save Changes" };
    let submitIcon = if isNew { "plus" } else { "check" };

    div([cls("editor")]) {
        div([cls("editor-toolbar")]) {
            navButton("/", "/fragments/notes", "/", "arrow-left", "Back")
            + spacer()
        }
        + form([hxPost(formAction), hxTarget("#content"), hxSwap("innerHTML")]) {
            input([attr("type", "hidden"), attr("name", "folderId"),
                   attr("value", "\(folderId)")])
            + input([cls("editor-title"), attr("type", "text"), attr("name", "title"),
                     attr("placeholder", "Note title..."),
                     attr("value", text(currentTitle)), boolAttr("required")])
            + textarea([cls("editor-body"), attr("name", "body"),
                        attr("placeholder", "Start writing..."),
                        attr("rows", "20")]) {
                text(currentBody)
            }
            + div([attr("style", "margin-top:20px")]) {
                button([cls("btn btn-primary"), attr("type", "submit")]) {
                    iconSized(submitIcon, 14) + span { submitLabel }
                }
            }
        }
    }
}

func navButton(pageHref: String, fragmentUrl: String,
               pushUrl: String, iconName: String, labelText: String) -> String {
    anchor([cls("btn btn-ghost btn-sm"), href(pageHref),
            hxGet(fragmentUrl), hxTarget("#content"),
            hxSwap("innerHTML"), hxPushUrl(pushUrl)]) {
        iconSized(iconName, 14) + span { labelText }
    }
}

func emptyState() -> String {
    div([cls("empty")]) {
        iconSized("file-text", 48)
        + div([cls("empty-text")]) { "No notes yet. Create one to get started." }
        + anchor([cls("btn btn-primary"), href("/new")]) {
            iconSized("plus", 14) + span { "New Note" }
        }
    }
}

func getOptInt(v: Value, key: String) -> Int64 {
    match v.value(forKey: key) {
        .Some(n) => match n {
            .Int(val) => val,
            _ => 0
        },
        .None => 0
    }
}
