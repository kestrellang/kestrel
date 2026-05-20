module notes.ui

import quill.value.(Value)
import notes.html.(
    raw, text, nothing,
    div, span, anchor, button, form, input,
    cls, id, href, attr,
    hxGet, hxPost, hxTarget, hxSwap
)

public func folderSidebar(folders: Array[Value], activeFolderId: Int64) -> String {
    var s = String(capacity: 2048);
    s.append("<div class=\"sidebar-section\">");
    s.append("<div class=\"sidebar-label\">Folders</div>");

    // All Notes
    let allCls = if activeFolderId == 0 { "folder-item active" } else { "folder-item" };
    s.append("<a class=\"\(allCls)\" href=\"/\" hx-get=\"/fragments/notes\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/\">");
    s.append(iconSized("file-text", 16));
    s.append("<span class=\"folder-name\">All Notes</span></a>");

    // Folders
    var i: Int64 = 0;
    while i < folders.count {
        let folder = folders(unchecked: i);
        let fid = getInt(folder, "id");
        let name = getStr(folder, "name");
        let itemCls = if fid == activeFolderId { "folder-item active" } else { "folder-item" };
        s.append("<a class=\"\(itemCls)\" href=\"/folder/\(fid)\" hx-get=\"/fragments/notes?folderId=\(fid)\" hx-target=\"#content\" hx-swap=\"innerHTML\" hx-push-url=\"/folder/\(fid)\">");
        s.append(iconSized("folder", 16));
        s.append("<span class=\"folder-name\">");
        s.append(text(name));
        s.append("</span></a>");
        i = i + 1
    };

    // New folder input
    s.append("<form style=\"padding:8px 10px;margin-top:4px\" hx-post=\"/fragments/folders\" hx-target=\"#sidebar\" hx-swap=\"innerHTML\">");
    s.append("<input class=\"new-folder-input\" type=\"text\" name=\"name\" placeholder=\"New folder...\">");
    s.append("</form>");

    s.append("</div>");
    s
}

func getStr(v: Value, key: String) -> String {
    match v.value(forKey: key) {
        .Some(s) => match s {
            .Str(val) => val,
            _ => ""
        },
        .None => ""
    }
}

func getInt(v: Value, key: String) -> Int64 {
    match v.value(forKey: key) {
        .Some(n) => match n {
            .Int(val) => val,
            _ => 0
        },
        .None => 0
    }
}
