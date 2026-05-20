module notes.ui

import quill.value.(Value)
import notes.html.(
    raw, text, nothing,
    div, span, h1, h2, p, anchor, button, form, label, input, textarea,
    cls, id, href, attr, boolAttr,
    hxGet, hxPost, hxPut, hxDelete, hxTarget, hxSwap, hxConfirm
)

public func noteListView(notes: Array[Value], listTitle: String) -> String {
    var s = String(capacity: 4096);
    s.append("<div>");
    s.append("<div class=\"note-list-header\">");
    s.append("<h1 class=\"note-list-title\">");
    s.append(text(listTitle));
    s.append("</h1>");
    s.append("<a class=\"btn btn-primary btn-sm\" href=\"/new\">");
    s.append(iconSized("plus", 14));
    s.append("<span>New Note</span></a>");
    s.append("</div>");
    if notes.count == 0 {
        s.append(emptyState())
    } else {
        s.append(noteList(notes))
    };
    s.append("</div>");
    s
}

func noteList(notes: Array[Value]) -> String {
    var s = String(capacity: 4096);
    s.append("<div class=\"note-list\">");
    var i: Int64 = 0;
    while i < notes.count {
        s.append(noteCard(notes(unchecked: i)));
        i = i + 1
    };
    s.append("</div>");
    s
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

    var s = String(capacity: 512);
    s.append("<a class=\"note-card\" href=\"/note/\(nid)\" hx-get=\"/fragments/note/\(nid)\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/note/\(nid)\">");
    s.append("<div class=\"note-card-title\">");
    s.append(text(noteTitle));
    s.append("</div>");
    s.append("<div class=\"note-card-preview\">");
    s.append(text(preview));
    s.append("</div>");
    s.append("<div class=\"note-card-meta\">");
    s.append(iconSized("clock", 12));
    s.append("<time datetime=\"\(updatedAt)\">");
    s.append(text(updatedAt));
    s.append("</time></div>");
    s.append("</a>");
    s
}

public func noteDetailView(note: Value) -> String {
    let nid = getInt(note, "id");
    let noteTitle = getStr(note, "title");
    let noteBody = getStr(note, "body");
    let updatedAt = getStr(note, "updatedAt");

    var s = String(capacity: 2048);
    s.append("<div class=\"editor\">");

    // Toolbar
    s.append("<div class=\"editor-toolbar\">");
    s.append("<a class=\"btn btn-ghost btn-sm\" href=\"/\" hx-get=\"/fragments/notes\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/\">");
    s.append(iconSized("arrow-left", 14));
    s.append("<span>Back</span></a>");
    s.append("<a class=\"btn btn-ghost btn-sm\" href=\"/note/\(nid)/edit\" hx-get=\"/fragments/note/\(nid)/edit\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/note/\(nid)/edit\">");
    s.append(iconSized("pencil", 14));
    s.append("<span>Edit</span></a>");
    s.append("<button class=\"btn btn-danger btn-sm\" hx-delete=\"/fragments/note/\(nid)\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-confirm=\"Delete this note?\">");
    s.append(iconSized("trash-2", 14));
    s.append("<span>Delete</span></button>");
    s.append("<span class=\"spacer\"></span>");
    s.append("<span class=\"note-meta\">");
    s.append(iconSized("clock", 12));
    s.append("<time datetime=\"\(updatedAt)\">");
    s.append(text(updatedAt));
    s.append("</time></span>");
    s.append("</div>");

    // Content
    s.append("<h1 class=\"note-title\">");
    s.append(text(noteTitle));
    s.append("</h1>");
    s.append("<div class=\"note-body\">");
    s.append(text(noteBody));
    s.append("</div>");

    s.append("</div>");
    s
}

public func noteEditorView(note: Value?) -> String {
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

    var s = String(capacity: 2048);
    s.append("<div class=\"editor\">");

    // Toolbar
    s.append("<div class=\"editor-toolbar\">");
    s.append("<a class=\"btn btn-ghost btn-sm\" href=\"/\" hx-get=\"/fragments/notes\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/\">");
    s.append(iconSized("arrow-left", 14));
    s.append("<span>Back</span></a>");
    s.append("<span class=\"spacer\"></span>");
    s.append("</div>");

    // Form
    s.append("<form hx-post=\"\(formAction)\" hx-target=\"#content\" hx-swap=\"innerHTML\">");
    s.append("<input class=\"editor-title\" type=\"text\" name=\"title\" placeholder=\"Note title...\" value=\"");
    s.append(text(currentTitle));
    s.append("\" required>");
    s.append("<textarea class=\"editor-body\" name=\"body\" placeholder=\"Start writing...\" rows=\"20\">");
    s.append(text(currentBody));
    s.append("</textarea>");
    s.append("<div style=\"margin-top:20px\">");
    s.append("<button class=\"btn btn-primary\" type=\"submit\">");
    s.append(iconSized(submitIcon, 14));
    s.append("<span>");
    s.append(submitLabel);
    s.append("</span></button>");
    s.append("</div>");
    s.append("</form>");

    s.append("</div>");
    s
}

func emptyState() -> String {
    var s = String(capacity: 256);
    s.append("<div class=\"empty\">");
    s.append(iconSized("file-text", 48));
    s.append("<div class=\"empty-text\">No notes yet. Create one to get started.</div>");
    s.append("<a class=\"btn btn-primary\" href=\"/new\">");
    s.append(iconSized("plus", 14));
    s.append("<span>New Note</span></a>");
    s.append("</div>");
    s
}
