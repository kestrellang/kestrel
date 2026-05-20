module notes.ui

import notes.html.(
    raw, text, nothing,
    div, span, anchor, button, h1, p, form, label, input,
    cls, id, href, attr, boolAttr,
    hxPost, hxTarget, hxSwap
)

public func loginPage(error: String) -> String {
    var c = String(capacity: 4096);
    c.append("<div class=\"auth-logo\">");
    c.append(iconSized("feather", 22));
    c.append("<span>Notes</span></div>");
    c.append(h1([cls("auth-title")]) { text("Welcome back") });
    c.append(p([cls("auth-subtitle")]) { text("Sign in to continue") });
    c.append(errorAlert(error));
    c.append(loginForm());
    authPage("Login — Notes", c)
}

func loginForm() -> String {
    var f = String(capacity: 2048);
    f.append("<form action=\"/login\" method=\"POST\">");
    f.append(fieldGroup("Email", "email", "email", "you@example.com"));
    f.append(fieldGroup("Password", "password", "password", "Your password"));
    f.append("<button class=\"btn btn-primary auth-submit\" type=\"submit\">");
    f.append(iconSized("arrow-right", 14));
    f.append("<span>Sign In</span></button>");
    f.append(authLink("Don't have an account? ", "/register", "Sign up"));
    f.append("</form>");
    f
}

public func registerPage(error: String) -> String {
    var c = String(capacity: 4096);
    c.append("<div class=\"auth-logo\">");
    c.append(iconSized("feather", 22));
    c.append("<span>Notes</span></div>");
    c.append(h1([cls("auth-title")]) { text("Create an account") });
    c.append(p([cls("auth-subtitle")]) { text("Start organizing your thoughts") });
    c.append(errorAlert(error));
    c.append(registerForm());
    authPage("Register — Notes", c)
}

func registerForm() -> String {
    var f = String(capacity: 2048);
    f.append("<form action=\"/register\" method=\"POST\">");
    f.append("<div style=\"display:grid;grid-template-columns:1fr 1fr;gap:10px\">");
    f.append(fieldGroup("First Name", "text", "firstName", "Ada"));
    f.append(fieldGroup("Last Name", "text", "lastName", "Lovelace"));
    f.append("</div>");
    f.append(fieldGroup("Email", "email", "email", "you@example.com"));
    f.append(fieldGroup("Password", "password", "password", "Choose a password"));
    f.append("<button class=\"btn btn-primary auth-submit\" type=\"submit\">");
    f.append(iconSized("arrow-right", 14));
    f.append("<span>Create Account</span></button>");
    f.append(authLink("Already have an account? ", "/login", "Sign in"));
    f.append("</form>");
    f
}

func fieldGroup(labelText: String, inputType: String, name: String, placeholder: String) -> String {
    var s = String(capacity: 512);
    s.append("<div class=\"field\">");
    s.append(label([cls("field-label"), attr("for", name)]) { text(labelText) });
    s.append(input([cls("field-input"), attr("type", inputType), attr("name", name), attr("id", name), attr("placeholder", placeholder), boolAttr("required")]));
    s.append("</div>");
    s
}

func authLink(message: String, url: String, linkText: String) -> String {
    var s = String(capacity: 256);
    s.append("<p class=\"auth-link\">");
    s.append(message);
    s.append(anchor([href(url)]) { text(linkText) });
    s.append("</p>");
    s
}

func errorAlert(message: String) -> String {
    if message.byteCount == 0 {
        ""
    } else {
        var s = String(capacity: 256);
        s.append("<div class=\"alert alert-error\">");
        s.append(iconSized("alert-circle", 14));
        s.append("<span>");
        s.append(text(message));
        s.append("</span></div>");
        s
    }
}
