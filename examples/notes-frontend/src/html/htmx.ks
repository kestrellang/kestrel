module notes.html

public func hxGet(url: String) -> String { attr("hx-get", url) }
public func hxPost(url: String) -> String { attr("hx-post", url) }
public func hxPut(url: String) -> String { attr("hx-put", url) }
public func hxDelete(url: String) -> String { attr("hx-delete", url) }
public func hxTarget(selector: String) -> String { attr("hx-target", selector) }
public func hxSwap(mode: String) -> String { attr("hx-swap", mode) }
public func hxTrigger(event: String) -> String { attr("hx-trigger", event) }
public func hxPushUrl(url: String) -> String { attr("hx-push-url", url) }
public func hxConfirm(message: String) -> String { attr("hx-confirm", message) }
public func hxIndicator(selector: String) -> String { attr("hx-indicator", selector) }
public func hxInclude(selector: String) -> String { attr("hx-include", selector) }
public func hxVals(json: String) -> String { attr("hx-vals", json) }
