module notes.ui

import html.builder.(
    raw, text, nothing, el, Document, Attr,
    div, span, anchor, button,
    htmlDoc, headEl, bodyEl, title, style, script, meta, linkEl,
    spacer,
    cls, id, href, attr, boolAttr
)

public func page(pageTitle: String, content: Document) -> Document {
    raw("<!DOCTYPE html>")
    + htmlDoc([attr("lang", "en")]) {
        headEl {
            meta([attr("charset", "utf-8")])
            + meta([attr("name", "viewport"), attr("content", "width=device-width, initial-scale=1")])
            + title { text(pageTitle) }
            + linkEl([attr("rel", "preconnect"), href("https://fonts.googleapis.com")])
            + linkEl([attr("rel", "stylesheet"), href("https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap")])
            + script([attr("src", "https://unpkg.com/htmx.org@1.9.10")]) { nothing() }
            + script([attr("src", "https://unpkg.com/lucide@latest")]) { nothing() }
            + style { appCss() }
            + script { clientJs() }
        }
        + bodyEl { content }
    }
}

func clientJs() -> Document {
    raw(
        ##"document.addEventListener('click',function(e){var f=e.target.closest('.folder-item');if(!f)return;document.querySelectorAll('.folder-item').forEach(function(el){el.classList.remove('active')});f.classList.add('active')});"##
        + ##"function initPage(){lucide.createIcons();document.querySelectorAll('time[datetime]').forEach(function(el){var d=new Date(el.getAttribute('datetime'));if(isNaN(d))return;var now=new Date();var diff=now-d;var s=Math.floor(diff/1000);if(s<60){el.textContent='just now'}else if(s<3600){el.textContent=Math.floor(s/60)+'m ago'}else if(s<86400){el.textContent=Math.floor(s/3600)+'h ago'}else if(s<604800){el.textContent=Math.floor(s/86400)+'d ago'}else{el.textContent=d.toLocaleDateString(undefined,{month:'short',day:'numeric',year:d.getFullYear()!==now.getFullYear()?'numeric':undefined})}})}"##
        + ##"document.addEventListener('DOMContentLoaded',initPage);"##
        + ##"document.addEventListener('htmx:afterSettle',initPage);"##
    )
}

public func authPage(pageTitle: String, content: Document) -> Document {
    page(pageTitle,
        div([cls("auth-page")]) {
            div([cls("auth-card")]) { content }
        }
    )
}

public func appShell(pageTitle: String, sidebar: Document, content: Document) -> Document {
    page(pageTitle,
        div([cls("app")]) {
            topbar()
            + el("aside", [cls("sidebar"), id("sidebar")]) { sidebar }
            + el("main", [cls("content"), id("content")]) { content }
        }
    )
}

func topbar() -> Document {
    div([cls("topbar")]) {
        anchor([href("/"), cls("topbar-brand")]) {
            iconSized("feather", 18) + span { text("Notes") }
        }
        + div([cls("topbar-actions")]) {
            anchor([cls("btn btn-primary btn-sm"), href("/new")]) {
                iconSized("plus", 14) + span { text("New Note") }
            }
            + anchor([cls("btn btn-ghost btn-sm"), href("/logout")]) {
                iconSized("log-out", 14) + span { text("Logout") }
            }
        }
    }
}
