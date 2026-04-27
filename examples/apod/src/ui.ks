// UI layer: HTML rendering for APOD.

module apod.ui

import quill.value.(Value)
import plume.plume.(Template)
import apod.data.(getStringField)

// ============================================================================
// SHARED CSS
// ============================================================================

func baseCss() -> String {
    var s = String();
    s.append("*{box-sizing:border-box;margin:0;padding:0}html,body{background:#000;color:#e8e8f0;min-height:100vh}body{font-family:'Inter',system-ui,-apple-system,sans-serif;-webkit-font-smoothing:antialiased;line-height:1.6;overflow-x:hidden}a{color:inherit;text-decoration:none}");

    // Hero: full-viewport edge-to-edge image, fades to black at the bottom.
    s.append(".hero{position:relative;height:100vh;width:100vw;overflow:hidden;background:#000}.hero-img{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;object-position:center;animation:kenBurns 60s ease-in-out infinite alternate;opacity:0;transition:opacity 1.4s ease;will-change:transform}.hero-img.loaded{opacity:1}.hero-fade{position:absolute;inset:0;pointer-events:none;background:linear-gradient(to bottom,rgba(0,0,0,0.45) 0%,transparent 18%,transparent 50%,rgba(0,0,0,0.55) 78%,#000 100%)}.hero-frame{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;background:#000}.hero-frame iframe{width:100%;max-width:1280px;aspect-ratio:16/9;border:0;box-shadow:0 40px 120px rgba(0,0,0,0.7)}");

    // Brand mark (top-left of hero).
    s.append(".brand{position:absolute;top:32px;left:48px;z-index:3;display:flex;align-items:center;gap:10px;font-size:0.7rem;letter-spacing:0.4em;text-transform:uppercase;font-weight:600;color:rgba(255,255,255,0.85);text-shadow:0 1px 12px rgba(0,0,0,0.6)}.brand-mark{font-size:1rem;letter-spacing:0;color:#fff}");

    // Hero meta block (overline, title, date) — bottom of hero.
    s.append(".hero-meta{position:absolute;left:0;right:0;bottom:120px;padding:0 48px;text-align:center;z-index:2;text-shadow:0 2px 24px rgba(0,0,0,0.7)}.overline{font-size:0.72rem;letter-spacing:0.4em;text-transform:uppercase;color:rgba(255,255,255,0.7);font-weight:600;margin-bottom:24px;opacity:0;animation:rise 1.1s cubic-bezier(0.16,1,0.3,1) 0.3s forwards}.title{font-family:'Fraunces',Georgia,serif;font-weight:400;font-size:clamp(2.4rem,6vw,5.4rem);letter-spacing:-0.02em;line-height:1.02;color:#fff;max-width:18ch;margin:0 auto 28px;opacity:0;transform:translateY(24px);animation:rise 1.4s cubic-bezier(0.16,1,0.3,1) 0.45s forwards}.date-chip{display:inline-block;font-size:0.7rem;letter-spacing:0.36em;text-transform:uppercase;color:rgba(255,255,255,0.65);font-weight:600;padding:8px 16px;border:1px solid rgba(255,255,255,0.18);border-radius:999px;background:rgba(0,0,0,0.2);backdrop-filter:blur(8px);-webkit-backdrop-filter:blur(8px);opacity:0;animation:rise 1.1s cubic-bezier(0.16,1,0.3,1) 0.65s forwards}");

    // Prev/next nav arrows on left/right edges.
    s.append(".nav{position:absolute;top:50%;transform:translateY(-50%);z-index:3;width:56px;height:56px;display:flex;align-items:center;justify-content:center;background:none;border:0;color:rgba(255,255,255,0.7);cursor:pointer;filter:drop-shadow(0 2px 12px rgba(0,0,0,0.55));transition:color 0.2s,transform 0.2s;opacity:0;animation:fadeIn 0.9s ease 1s forwards;padding:0}.nav:hover{color:#fff;transform:translateY(-50%) scale(1.12)}.nav:disabled{opacity:0.25;cursor:not-allowed;pointer-events:none}.nav-prev{left:24px}.nav-next{right:24px}.nav svg{width:32px;height:32px}");

    // Scroll cue.
    s.append(".scroll-cue{position:absolute;bottom:32px;left:50%;transform:translateX(-50%);z-index:2;font-size:0.62rem;letter-spacing:0.36em;text-transform:uppercase;color:rgba(255,255,255,0.55);font-weight:600;display:flex;flex-direction:column;align-items:center;gap:10px;opacity:0;animation:fadeIn 1s ease 1.4s forwards}.scroll-cue .chev{width:18px;height:18px;animation:bob 2s ease-in-out infinite}");

    // Content section below the fold.
    s.append(".story{background:#000;padding:120px 32px 96px;position:relative}.story::before{content:\"\";position:absolute;top:-1px;left:0;right:0;height:200px;background:linear-gradient(to bottom,#000,transparent);pointer-events:none}.container{max-width:64ch;margin:0 auto}.story-head{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:24px;margin-bottom:48px;padding-bottom:32px;border-bottom:1px solid rgba(255,255,255,0.08)}.date-form{display:flex;align-items:center;gap:12px}.date-form label{font-size:0.66rem;color:rgba(255,255,255,0.55);text-transform:uppercase;letter-spacing:0.32em;font-weight:600}.date-form input{background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.14);color:#f1f0f5;padding:10px 14px;border-radius:2px;font-family:inherit;font-size:0.9rem;outline:none;transition:border-color 0.2s,background 0.2s;color-scheme:dark}.date-form input:focus{border-color:rgba(255,255,255,0.45);background:rgba(255,255,255,0.1)}.hd-link{font-size:0.66rem;letter-spacing:0.32em;text-transform:uppercase;color:rgba(255,255,255,0.55);font-weight:600;transition:color 0.2s}.hd-link:hover{color:#fff}");

    // Explanation with drop cap.
    s.append(".explanation{font-size:1.08rem;color:rgba(232,232,240,0.84);font-weight:400}.explanation p{margin-bottom:1em}.explanation p:first-of-type::first-letter{font-family:'Fraunces',Georgia,serif;font-size:4.2rem;float:left;line-height:0.88;padding:0.08em 0.14em 0 0;color:#fff;font-weight:500}.copyright{margin-top:64px;padding-top:24px;border-top:1px solid rgba(255,255,255,0.08);font-size:0.66rem;letter-spacing:0.28em;text-transform:uppercase;color:rgba(255,255,255,0.4);font-weight:600}");

    // Error page.
    s.append(".error-shell{min-height:100vh;display:flex;align-items:center;justify-content:center;padding:32px}.error{max-width:54ch;text-align:center;padding:48px 32px;border:1px solid rgba(239,68,68,0.22);border-radius:2px;background:rgba(20,20,28,0.55);color:#fca5a5;font-family:'Fraunces',Georgia,serif;font-weight:400;font-size:1.4rem;line-height:1.4}");

    // Animations.
    s.append("@keyframes kenBurns{from{transform:scale(1.05) translate(0,0)}to{transform:scale(1.18) translate(-2%,1.5%)}}@keyframes rise{to{opacity:1;transform:translateY(0)}}@keyframes fadeIn{to{opacity:1}}@keyframes bob{0%,100%{transform:translateY(0)}50%{transform:translateY(6px)}}");

    // Responsive + reduced motion.
    s.append("@media(max-width:720px){.brand{top:20px;left:20px}.nav{width:44px;height:44px}.nav-prev{left:14px}.nav-next{right:14px}.hero-meta{padding:0 24px;bottom:96px}.title{font-size:2.2rem}.story{padding:80px 20px 64px}.story-head{flex-direction:column;align-items:flex-start;gap:16px}.explanation p:first-of-type::first-letter{font-size:3rem}}@media(prefers-reduced-motion:reduce){.hero-img{animation:none!important}.scroll-cue .chev{animation:none!important}*{animation-duration:0.01ms!important;transition-duration:0.01ms!important}}");

    s
}

// ============================================================================
// HEAD (font + viewport boilerplate)
// ============================================================================

func headHtml(title: String) -> String {
    var h = String();
    var t = Template();
    h.append("<!DOCTYPE html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    t.put("title", title);
    h.append(t.render("<title>{title} &mdash; Kestrel APOD</title>"));
    h.append("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\"><link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin><link href=\"https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,500&family=Inter:wght@400;500;600;700&display=swap\" rel=\"stylesheet\"><style>");
    h.append(baseCss());
    h.append("</style></head>");
    h
}

// Inline JS for prev/next navigation and image fade-in.
// Kept braces-free where it conflicts with Plume Template; we never run this
// string through `t.render`.
func navScript() -> String {
    var s = String();
    s.append("<script>(function(){var img=document.querySelector('.hero-img');if(img){if(img.complete){img.classList.add('loaded')}else{img.addEventListener('load',function(){img.classList.add('loaded')})}}window.apodNav=function(delta){var u=new URL(location.href);var cur=u.searchParams.get('date');var d=cur?new Date(cur+'T00:00:00Z'):new Date();d.setUTCDate(d.getUTCDate()+delta);var today=new Date();today.setUTCHours(0,0,0,0);var min=new Date('1995-06-16T00:00:00Z');if(d>today||d<min)return;u.searchParams.set('date',d.toISOString().slice(0,10));location.href=u.toString()};})();</script>");
    s
}

// SVG chevron used by both nav buttons (rotated for the right one via CSS).
func chevronLeft() -> String {
    "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><polyline points=\"15 6 9 12 15 18\"/></svg>"
}

func chevronRight() -> String {
    "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><polyline points=\"9 6 15 12 9 18\"/></svg>"
}

func chevronDown() -> String {
    "<svg class=\"chev\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><polyline points=\"6 9 12 15 18 9\"/></svg>"
}

// ============================================================================
// FULL PAGE
// ============================================================================

public func pageHtml(json: Value, selectedDate: String) -> String {
    var h = String();
    var t = Template();

    let title = getStringField(json, "title");
    let date = getStringField(json, "date");
    let mediaType = getStringField(json, "media_type");
    let url = getStringField(json, "url");
    let hdurl = getStringField(json, "hdurl");
    let explanation = getStringField(json, "explanation");
    let copyright = getStringField(json, "copyright");

    h.append(headHtml(title));
    h.append("<body>");

    // ----- HERO -----
    h.append("<section class=\"hero\">");

    // Background media: image fills the viewport via object-fit:cover.
    // Video days center an iframe with letterbox.
    if mediaType == "image" and url.byteCount > 0 {
        let bgUrl = if hdurl.byteCount > 0 { hdurl } else { url };
        t.put("u", bgUrl);
        t.put("alt", title);
        h.append(t.render("<img class=\"hero-img\" src=\"{u}\" alt=\"{alt}\">"))
    } else if mediaType == "video" and url.byteCount > 0 {
        t.put("u", url);
        t.put("alt", title);
        h.append(t.render("<div class=\"hero-frame\"><iframe src=\"{u}\" title=\"{alt}\" allow=\"encrypted-media\" allowfullscreen></iframe></div>"))
    };

    // Vignette + bottom fade.
    h.append("<div class=\"hero-fade\"></div>");

    // Brand chip (top-left).
    h.append("<a class=\"brand\" href=\"/\"><span class=\"brand-mark\">&#x2726;</span><span>Kestrel APOD</span></a>");

    // Prev/next arrows (JS-driven; degrades to inert on no-JS).
    h.append("<button type=\"button\" class=\"nav nav-prev\" onclick=\"apodNav(-1)\" aria-label=\"Previous day\">");
    h.append(chevronLeft());
    h.append("</button>");
    h.append("<button type=\"button\" class=\"nav nav-next\" onclick=\"apodNav(1)\" aria-label=\"Next day\">");
    h.append(chevronRight());
    h.append("</button>");

    // Hero meta: overline + big serif title + date chip.
    h.append("<div class=\"hero-meta\"><div class=\"overline\">Astronomy Picture of the Day</div>");
    if title.byteCount > 0 {
        t.put("title", title);
        h.append(t.render("<h1 class=\"title\">{title}</h1>"))
    };
    let chipDate = if date.byteCount > 0 { date } else { selectedDate };
    if chipDate.byteCount > 0 {
        t.put("d", chipDate);
        h.append(t.render("<div class=\"date-chip\">{d}</div>"))
    };
    h.append("</div>");

    // Scroll cue.
    h.append("<a class=\"scroll-cue\" href=\"#story\"><span>Scroll</span>");
    h.append(chevronDown());
    h.append("</a>");

    h.append("</section>");

    // ----- STORY (below the fold) -----
    h.append("<main class=\"story\" id=\"story\"><div class=\"container\"><div class=\"story-head\">");

    // Date picker.
    let pickerDate = if date.byteCount > 0 { date } else { selectedDate };
    t.setRaw("date", pickerDate);
    h.append(t.render("<form class=\"date-form\" method=\"get\" action=\"/\"><label for=\"d\">Date</label><input id=\"d\" type=\"date\" name=\"date\" value=\"{date}\" min=\"1995-06-16\" onchange=\"this.form.submit()\"></form>"));

    // HD link (only when distinct and image).
    if mediaType == "image" and hdurl.byteCount > 0 and hdurl != url {
        t.put("hd", hdurl);
        h.append(t.render("<a class=\"hd-link\" href=\"{hd}\" target=\"_blank\" rel=\"noopener\">View HD &rarr;</a>"))
    };

    h.append("</div>");

    // Explanation.
    if explanation.byteCount > 0 {
        h.append("<div class=\"explanation\"><p>");
        t.put("text", explanation);
        h.append(t.render("{text}"));
        h.append("</p></div>")
    };

    if copyright.byteCount > 0 {
        t.put("c", copyright);
        h.append(t.render("<div class=\"copyright\">&copy; {c}</div>"))
    };

    h.append("</div></main>");

    h.append(navScript());
    h.append("</body></html>");
    h
}

// ============================================================================
// ERROR PAGE
// ============================================================================

public func errorPageHtml(msg: String, selectedDate: String) -> String {
    var h = String();
    var t = Template();

    h.append(headHtml("Error"));
    h.append("<body><div class=\"error-shell\"><div>");
    h.append("<a class=\"brand\" style=\"position:static;display:inline-flex;margin-bottom:32px\" href=\"/\"><span class=\"brand-mark\">&#x2726;</span><span>Kestrel APOD</span></a>");
    t.put("msg", msg);
    h.append(t.render("<div class=\"error\">{msg}</div>"));
    let _ = selectedDate;
    h.append("</div></div></body></html>");
    h
}
