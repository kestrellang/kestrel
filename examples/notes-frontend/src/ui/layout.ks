module notes.ui

import notes.html.(
    raw, text, nothing,
    div, span, anchor, button, aside,
    el, cls, id, href, attr, boolAttr
)

public func page(pageTitle: String, content: String) -> String {
    var s = String(capacity: 16384);
    s.append("<!DOCTYPE html><html lang=\"en\"><head>");
    s.append("<meta charset=\"utf-8\">");
    s.append("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    s.append("<title>");
    s.append(pageTitle);
    s.append("</title>");
    s.append("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
    s.append("<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap\">");
    s.append("<script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>");
    s.append("<script src=\"https://unpkg.com/lucide@latest\"></script>");
    s.append("<style>");
    s.append(appCss());
    s.append("</style>");
    // Folder active toggle + Lucide icon init after every HTMX swap
    s.append(##"<script>"##);
    s.append(##"document.addEventListener('click',function(e){var f=e.target.closest('.folder-item');if(!f)return;document.querySelectorAll('.folder-item').forEach(function(el){el.classList.remove('active')});f.classList.add('active')});"##);
    s.append(##"function initPage(){lucide.createIcons();document.querySelectorAll('time[datetime]').forEach(function(el){var d=new Date(el.getAttribute('datetime'));if(isNaN(d))return;var now=new Date();var diff=now-d;var s=Math.floor(diff/1000);if(s<60){el.textContent='just now'}else if(s<3600){el.textContent=Math.floor(s/60)+'m ago'}else if(s<86400){el.textContent=Math.floor(s/3600)+'h ago'}else if(s<604800){el.textContent=Math.floor(s/86400)+'d ago'}else{el.textContent=d.toLocaleDateString(undefined,{month:'short',day:'numeric',year:d.getFullYear()!==now.getFullYear()?'numeric':undefined})}})}"##);
    s.append(##"document.addEventListener('DOMContentLoaded',initPage);"##);
    s.append(##"document.addEventListener('htmx:afterSettle',initPage);"##);
    s.append(##"</script>"##);
    s.append("</head><body>");
    s.append(content);
    s.append("</body></html>");
    s
}

public func authPage(pageTitle: String, content: String) -> String {
    var body = String(capacity: 4096);
    body.append("<div class=\"auth-page\"><div class=\"auth-card\">");
    body.append(content);
    body.append("</div></div>");
    page(pageTitle, body)
}

public func appShell(pageTitle: String, sidebar: String, content: String) -> String {
    var body = String(capacity: 8192);
    body.append("<div class=\"app\">");
    body.append(topbar());
    body.append("<aside class=\"sidebar\" id=\"sidebar\">");
    body.append(sidebar);
    body.append("</aside>");
    body.append("<main class=\"content\" id=\"content\">");
    body.append(content);
    body.append("</main>");
    body.append("</div>");
    page(pageTitle, body)
}

func topbar() -> String {
    var s = String(capacity: 512);
    s.append("<div class=\"topbar\">");
    s.append("<a href=\"/\" class=\"topbar-brand\">");
    s.append(iconSized("feather", 18));
    s.append("<span>Notes</span></a>");
    s.append("<div class=\"topbar-actions\">");
    s.append("<a class=\"btn btn-primary btn-sm\" href=\"/new\">");
    s.append(iconSized("plus", 14));
    s.append("<span>New Note</span></a>");
    s.append("<a class=\"btn btn-ghost btn-sm\" href=\"/logout\">");
    s.append(iconSized("log-out", 14));
    s.append("<span>Logout</span></a>");
    s.append("</div></div>");
    s
}
