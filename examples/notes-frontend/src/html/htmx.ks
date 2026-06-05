module notes.html

import html.builder.(attr, Attr)

public func hxGet(url: String) -> Attr { attr("hx-get", url) }
public func hxPost(url: String) -> Attr { attr("hx-post", url) }
public func hxPut(url: String) -> Attr { attr("hx-put", url) }
public func hxDelete(url: String) -> Attr { attr("hx-delete", url) }
public func hxTarget(selector: String) -> Attr { attr("hx-target", selector) }
public func hxSwap(mode: String) -> Attr { attr("hx-swap", mode) }
public func hxTrigger(event: String) -> Attr { attr("hx-trigger", event) }
public func hxPushUrl(url: String) -> Attr { attr("hx-push-url", url) }
public func hxConfirm(message: String) -> Attr { attr("hx-confirm", message) }
public func hxIndicator(selector: String) -> Attr { attr("hx-indicator", selector) }
public func hxInclude(selector: String) -> Attr { attr("hx-include", selector) }
public func hxVals(json: String) -> Attr { attr("hx-vals", json) }
