module wall.render

import wall.models.(WallNote)
import wall.helpers.(escapeHtml)

public func renderPage(notes: Array[WallNote]) -> String {
    var html = String();
    html.append(#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Kestrel Wall</title>
    <link rel="icon" href="/favicon.ico" type="image/svg+xml">
    <link rel="stylesheet" href="/style.css?v=6">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link href="https://fonts.googleapis.com/css2?family=Caveat:wght@400;700&display=swap" rel="stylesheet">
</head>
<body>
    <div id="toast" class="toast"></div>
    <div id="viewport" class="viewport">
    <div id="wall" class="wall">
    <div class="wall-title"><h1>Kestrel Wall</h1><p>Built with Kestrel</p></div>
"#);

    var i: Int64 = 0;
    while i < notes.count {
        let note = notes(unchecked: i);
        html.append(renderNote(note));
        i = i + 1
    };

    html.append(#"    </div>
    </div>
    <form id="post-form" class="post-form">
        <input type="text" name="username" placeholder="Name" maxlength="30" required>
        <input type="text" name="message" placeholder="Write something..." maxlength="280" required>
        <button type="submit">Pin it!</button>
    </form>
    <script src="/script.js?v=11"></script>
</body>
</html>"#);
    html
}

func renderNote(note: WallNote) -> String {
    let username = escapeHtml(note.username);
    let message = escapeHtml(note.message);

    var s = String();
    s.append("        <div class=\"note\" data-id=\"\(note.id)\" style=\"background: ");
    s.append(note.color);
    s.append("\">\n");
    s.append("            <div class=\"note-message\">");
    s.append(message);
    s.append("</div>\n");
    s.append("            <div class=\"note-author\">&mdash; ");
    s.append(username);
    s.append("</div>\n");
    s.append("        </div>\n");
    s
}
