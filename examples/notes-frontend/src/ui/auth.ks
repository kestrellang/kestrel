module notes.ui

import notes.html.(
    raw, text, nothing,
    div, span, anchor, button, h1, p, form, label, input,
    cls, id, href, attr, boolAttr,
    hxPost, hxTarget, hxSwap
)

public func loginPage(error: String) -> String {
    let c = div([cls("auth-logo")]) { iconSized("feather", 22) + span { "Notes" } }
        + h1([cls("auth-title")]) { text("Welcome back") }
        + p([cls("auth-subtitle")]) { text("Sign in to continue") }
        + errorAlert(error)
        + loginForm();
    authPage("Login — Notes", c)
}

func loginForm() -> String {
    form([attr("action", "/login"), attr("method", "POST")]) {
        fieldGroup("Email", "email", "email", "you@example.com")
        + fieldGroup("Password", "password", "password", "Your password")
        + button([cls("btn btn-primary auth-submit"), attr("type", "submit")]) {
            iconSized("arrow-right", 14) + span { "Sign In" }
        }
        + authLink("Don't have an account? ", "/register", "Sign up")
    }
}

public func registerPage(error: String) -> String {
    let c = div([cls("auth-logo")]) { iconSized("feather", 22) + span { "Notes" } }
        + h1([cls("auth-title")]) { text("Create an account") }
        + p([cls("auth-subtitle")]) { text("Start organizing your thoughts") }
        + errorAlert(error)
        + registerForm();
    authPage("Register — Notes", c)
}

func registerForm() -> String {
    form([attr("action", "/register"), attr("method", "POST")]) {
        div([attr("style", "display:grid;grid-template-columns:1fr 1fr;gap:10px")]) {
            fieldGroup("First Name", "text", "firstName", "Ada")
            + fieldGroup("Last Name", "text", "lastName", "Lovelace")
        }
        + fieldGroup("Email", "email", "email", "you@example.com")
        + fieldGroup("Password", "password", "password", "Choose a password")
        + button([cls("btn btn-primary auth-submit"), attr("type", "submit")]) {
            iconSized("arrow-right", 14) + span { "Create Account" }
        }
        + authLink("Already have an account? ", "/login", "Sign in")
    }
}

func fieldGroup(labelText: String, inputType: String, name: String, placeholder: String) -> String {
    div([cls("field")]) {
        label([cls("field-label"), attr("for", name)]) { text(labelText) }
        + input([cls("field-input"), attr("type", inputType), attr("name", name),
                 attr("id", name), attr("placeholder", placeholder), boolAttr("required")])
    }
}

func authLink(message: String, url: String, linkText: String) -> String {
    p([cls("auth-link")]) {
        message + anchor([href(url)]) { text(linkText) }
    }
}

func errorAlert(message: String) -> String {
    if message.byteCount == 0 {
        ""
    } else {
        div([cls("alert alert-error")]) {
            iconSized("alert-circle", 14)
            + span { text(message) }
        }
    }
}
