module notes.ui

import quill.value.(Value)
import notes.html.(
    raw, text, nothing, el,
    div, span, anchor, button, form, input,
    cls, id, href, attr,
    hxGet, hxPost, hxTarget, hxSwap, hxPushUrl
)

public func folderSidebar(folders: Array[Value], activeFolderId: Int64) -> String {
    let allCls = if activeFolderId == 0 { "folder-item active" } else { "folder-item" };

    var items = String();
    var i: Int64 = 0;
    while i < folders.count {
        let folder = folders(unchecked: i);
        let fid = getInt(folder, "id");
        let name = getStr(folder, "name");
        let itemCls = if fid == activeFolderId { "folder-item active" } else { "folder-item" };
        items.append(
            anchor([cls(itemCls), href("/folder/\(fid)"),
                    hxGet("/fragments/notes?folderId=\(fid)"), hxTarget("#content"),
                    hxSwap("innerHTML"), hxPushUrl("/folder/\(fid)")]) {
                iconSized("folder", 16)
                + span([cls("folder-name")]) { text(name) }
            }
        );
        i = i + 1
    };

    div([cls("sidebar-section")]) {
        div([cls("sidebar-label")]) { "Folders" }
        + anchor([cls(allCls), href("/"),
                  hxGet("/fragments/notes"), hxTarget("#content"),
                  hxSwap("innerHTML"), hxPushUrl("/")]) {
            iconSized("file-text", 16)
            + span([cls("folder-name")]) { "All Notes" }
        }
        + items
        + form([attr("style", "padding:8px 10px;margin-top:4px"),
                hxPost("/fragments/folders"), hxTarget("#sidebar"), hxSwap("innerHTML")]) {
            input([cls("new-folder-input"), attr("type", "text"),
                   attr("name", "name"), attr("placeholder", "New folder...")])
        }
    }
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
