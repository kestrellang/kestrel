// UI layer: HTML rendering for the pokedex

module pokedex.ui

import quill.value.(Value)
import plume.plume.(Template)
import pokedex.data.(PokemonEntry, kantoPokedex, kantoEntryById,
                    typeColor, typeColorDark, typeGlow, typeEmoji, allTypes,
                    statLabel, statPercent, statColor, statShortLabel,
                    unitX, unitY, statAngleIdx, angleToStatIdx, roundInt,
                    getField, getArrayField, getString, getInt)
import pokedex.util.(padId, formatMeters, formatKilos, capitalize, containsLower, toLower)

// ============================================================================
// SHARED CSS
// ============================================================================

func baseCss() -> String {
    var s = String(capacity: 4096);
    s.append("""*{box-sizing:border-box;margin:0;padding:0}body{font-family:'Inter',system-ui,-apple-system,sans-serif;color:#f1f0f5;min-height:100vh;-webkit-font-smoothing:antialiased;background:linear-gradient(160deg,#000 0%,#0a0a0a 40%,#141414 100%);background-attachment:fixed}a{color:inherit;text-decoration:none}.header{display:flex;align-items:center;justify-content:space-between;padding:16px 24px;border-bottom:1px solid rgba(255,255,255,0.06);position:sticky;top:0;background:rgba(0,0,0,0.85);backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);z-index:50}.logo{display:flex;align-items:center;gap:10px;font-weight:700;font-size:1.05rem;color:#f1f0f5;letter-spacing:-0.01em}.logo-dot{width:14px;height:14px;border-radius:50%;background:radial-gradient(circle at 30% 30%,#fee2e2,#dc2626 60%,#7f1d1d);box-shadow:0 0 0 2px #0a0a0a,0 0 0 3px #fee2e2,0 0 12px rgba(220,38,38,0.6)}.search-wrap{position:relative;flex:1;max-width:340px;margin-left:24px}.search-wrap input{width:100%;padding:10px 16px 10px 38px;border-radius:12px;border:1px solid rgba(255,255,255,0.1);background:rgba(255,255,255,0.06);color:#f1f0f5;font-size:0.9rem;font-family:inherit;outline:none;transition:border-color 0.3s,box-shadow 0.3s,background 0.3s}.search-wrap input::placeholder{color:#6b6784}.search-wrap input:focus{border-color:rgba(255,255,255,0.5);box-shadow:0 0 0 3px rgba(255,255,255,0.15);background:rgba(255,255,255,0.09)}.search-icon{position:absolute;left:14px;top:50%;transform:translateY(-50%);color:#6b6784;pointer-events:none;font-size:0.95rem}.htmx-indicator{display:none}.htmx-request .htmx-indicator{display:inline-block}.search-spinner{position:absolute;right:14px;top:50%;transform:translateY(-50%);width:14px;height:14px;border:2px solid rgba(255,255,255,0.2);border-top-color:#888;border-radius:50%;animation:spin 0.7s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}@keyframes fadeSlideIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}@keyframes float{0%,100%{transform:translateY(0)}50%{transform:translateY(-8px)}}@keyframes pop{0%{opacity:0;transform:scale(0.85)}100%{opacity:1;transform:scale(1)}}""");
    s
}

func gridCss() -> String {
    var s = String(capacity: 4096);
    s.append("""main.grid-main{max-width:1240px;margin:0 auto;padding:36px 24px 32px;animation:fadeSlideIn 0.5s ease both}.grid-hero{text-align:center;margin-bottom:28px;position:relative}.grid-hero h1{font-size:2.4rem;font-weight:900;letter-spacing:-0.03em;margin-bottom:6px;background:linear-gradient(90deg,#fff 0%,#666 50%,#fff 100%);background-size:200% auto;-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;animation:shimmer 6s linear infinite}.grid-hero p{color:#8b87a0;font-size:0.95rem;letter-spacing:0.01em}.grid-count{display:inline-flex;align-items:center;gap:6px;margin-top:10px;padding:5px 12px;border-radius:999px;background:rgba(255,255,255,0.1);border:1px solid rgba(255,255,255,0.2);font-size:0.78rem;color:#ddd;font-weight:600;font-variant-numeric:tabular-nums;letter-spacing:0.04em}.grid-count b{color:#fff;font-weight:800}.type-filter{display:flex;flex-wrap:wrap;gap:8px;justify-content:center;margin-bottom:24px;padding:0 4px}.type-filter button{cursor:pointer;border:none;font-family:inherit;padding:7px 14px;border-radius:999px;font-size:0.76rem;font-weight:700;letter-spacing:0.04em;color:#fff;text-transform:capitalize;text-shadow:0 1px 1px rgba(0,0,0,0.35);transition:transform 0.15s ease,filter 0.15s ease,box-shadow 0.15s ease;display:inline-flex;align-items:center;gap:5px;opacity:0.85}.type-filter button:hover{opacity:1;transform:translateY(-1px);filter:brightness(1.1)}.type-filter button.active{opacity:1;box-shadow:0 0 0 2px #fff,0 0 16px var(--btn-color,rgba(255,255,255,0.6));transform:translateY(-1px)}.type-filter button.all-btn{background:rgba(255,255,255,0.06);color:#e8e6f0;border:1px solid rgba(255,255,255,0.12);text-shadow:none;font-weight:600}.type-filter button.all-btn.active{background:rgba(255,255,255,0.12);border-color:rgba(255,255,255,0.5)}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(140px,1fr));gap:14px;perspective:600px}.grid:empty::after{content:"\01F614  No matches in this corner of Kanto.";display:block;grid-column:1/-1;text-align:center;color:#8b87a0;padding:64px 0;font-size:0.95rem;letter-spacing:0.02em}.card{position:relative;background:linear-gradient(160deg,color-mix(in srgb,var(--card-color,#888) 14%,#0d0d0d 86%) 0%,#0d0d0d 60%);border:2px solid color-mix(in srgb,var(--card-color,#888) 55%,transparent 45%);border-radius:18px;padding:14px 10px 12px;text-align:center;animation:pop 0.3s ease both;display:flex;flex-direction:column;align-items:center;cursor:pointer;transform-style:preserve-3d;transition:transform 0.18s ease-out,border-color 0.2s ease,box-shadow 0.2s ease;will-change:transform;text-decoration:none;color:inherit}.card:hover{border-color:var(--card-color,#888);box-shadow:0 14px 30px rgba(0,0,0,0.5),0 0 28px color-mix(in srgb,var(--card-color,#888) 45%,transparent 55%)}.card::after{content:"";position:absolute;inset:0;border-radius:inherit;background:radial-gradient(circle at var(--mx,50%) var(--my,50%),rgba(255,255,255,0.35) 0%,rgba(255,255,255,0.0) 32%),conic-gradient(from calc(var(--mx-deg,90deg)) at var(--mx,50%) var(--my,50%),rgba(255,0,180,0.22),rgba(0,200,255,0.22),rgba(180,255,0,0.22),rgba(255,150,0,0.22),rgba(255,0,180,0.22));opacity:0;transition:opacity 0.3s ease;pointer-events:none;mix-blend-mode:color-dodge}.card:hover::after{opacity:0.9}.card-sprite{width:96px;height:96px;image-rendering:pixelated;image-rendering:-moz-crisp-edges;filter:drop-shadow(0 4px 6px rgba(0,0,0,0.45));transition:transform 0.25s ease}.card:hover .card-sprite{transform:translateZ(35px) scale(1.08)}.card-id{font-size:0.7rem;color:#6b6784;font-weight:700;letter-spacing:0.06em;margin-top:2px;font-variant-numeric:tabular-nums}.card-name{font-size:0.92rem;font-weight:700;color:#f1f0f5;margin-top:1px;text-transform:capitalize}.grid-footer{margin-top:48px;padding-top:24px;border-top:1px solid rgba(255,255,255,0.05);text-align:center;font-size:0.78rem;color:#6b6784}.grid-footer a{color:#888;text-decoration:none;font-weight:600;transition:color 0.2s}.grid-footer a:hover{color:#ddd}.grid-footer .sep{margin:0 10px;opacity:0.4}@keyframes shimmer{to{background-position:200% center}}@media(max-width:640px){.search-wrap{max-width:none;margin-left:12px}.grid{grid-template-columns:repeat(auto-fill,minmax(110px,1fr));gap:10px}.card-sprite{width:80px;height:80px}.grid-hero h1{font-size:1.8rem}.type-filter button{padding:6px 11px;font-size:0.72rem}}@media(hover:none){.card{transform:none!important}}""");
    s
}

// TCG-card-styled detail page. The body gets `--type-color` / `--type-color-2`
// / `--type-glow` CSS variables so every type-derived color in the layout
// (frame, hp badge, radar fill, ambient body glow) flows from the same source.
func tcgCss() -> String {
    var s = String(capacity: 8192);
    s.append("""body.tcg-body{background:radial-gradient(ellipse at 20% -10%,var(--type-glow,rgba(255,255,255,0.18)) 0%,transparent 55%),radial-gradient(ellipse at 80% 110%,var(--type-glow,rgba(255,255,255,0.18)) 0%,transparent 55%),linear-gradient(160deg,#000 0%,#0d0d0d 45%,#141414 100%);background-attachment:fixed}main.tcg-main{max-width:1080px;margin:0 auto;padding:48px 24px 80px;display:grid;grid-template-columns:minmax(320px,420px) 1fr;gap:48px;align-items:start;animation:fadeSlideIn 0.5s ease both;perspective:1400px}.tcg-card{position:relative;border-radius:22px;padding:5px;background:linear-gradient(135deg,var(--type-color,#888) 0%,var(--type-color-2,#444) 50%,var(--type-color,#888) 100%);box-shadow:0 30px 60px -15px rgba(0,0,0,0.55),0 0 0 1px rgba(255,255,255,0.08),0 0 60px -10px var(--type-color,#888);transform-style:preserve-3d;transition:transform 0.35s cubic-bezier(0.2,0.7,0.2,1);will-change:transform}.tcg-card::before{content:"";position:absolute;inset:0;border-radius:inherit;background:repeating-linear-gradient(115deg,rgba(255,255,255,0.08) 0%,rgba(255,255,255,0.08) 1px,transparent 1px,transparent 5px);opacity:0.5;pointer-events:none;mix-blend-mode:overlay}.tcg-card::after{content:"";position:absolute;inset:0;border-radius:inherit;background:radial-gradient(circle at var(--mx,50%) var(--my,40%),rgba(255,255,255,0.45) 0%,rgba(255,255,255,0.0) 22%),conic-gradient(from calc(var(--mx-deg,90deg)) at var(--mx,50%) var(--my,50%),rgba(255,0,180,0.18),rgba(0,200,255,0.18),rgba(180,255,0,0.18),rgba(255,150,0,0.18),rgba(255,0,180,0.18));mix-blend-mode:color-dodge;border-radius:inherit;pointer-events:none;opacity:var(--foil-opacity,0);transition:opacity 0.4s ease}.tcg-inner{position:relative;background:linear-gradient(160deg,#0d0d0d 0%,#181818 100%);border-radius:18px;overflow:hidden;display:flex;flex-direction:column;min-height:560px}.tcg-header{display:flex;align-items:flex-end;justify-content:space-between;padding:14px 18px 10px;background:linear-gradient(180deg,var(--type-color,#888) 0%,color-mix(in srgb,var(--type-color,#888) 70%,#000 30%) 100%);position:relative}.tcg-header::after{content:"";position:absolute;left:0;right:0;bottom:-1px;height:2px;background:linear-gradient(90deg,transparent,rgba(255,255,255,0.4),transparent)}.tcg-name{font-size:1.7rem;font-weight:800;color:#fff;letter-spacing:-0.02em;text-shadow:0 2px 4px rgba(0,0,0,0.35);text-transform:capitalize;line-height:1}.tcg-stage{font-size:0.7rem;color:rgba(255,255,255,0.7);font-weight:600;letter-spacing:0.1em;text-transform:uppercase;margin-bottom:4px}.tcg-hp{display:flex;align-items:baseline;gap:4px;color:#fff;text-shadow:0 1px 2px rgba(0,0,0,0.4)}.tcg-hp-prefix{font-size:0.78rem;font-weight:700;letter-spacing:0.05em}.tcg-hp-val{font-size:1.9rem;font-weight:900;letter-spacing:-0.04em;color:#fff5f5}.tcg-hp-unit{font-size:0.78rem;font-weight:800;letter-spacing:0.05em}.tcg-art{position:relative;margin:14px;border-radius:14px;background:radial-gradient(ellipse at 50% 35%,color-mix(in srgb,var(--type-color,#888) 38%,#000 62%) 0%,#000 75%);min-height:300px;display:flex;align-items:center;justify-content:center;overflow:hidden;border:1px solid rgba(255,255,255,0.08)}.tcg-art::before{content:"";position:absolute;inset:0;background:repeating-linear-gradient(45deg,rgba(255,255,255,0.02) 0,rgba(255,255,255,0.02) 1px,transparent 1px,transparent 6px);pointer-events:none}.tcg-art img{position:relative;width:90%;max-width:300px;filter:drop-shadow(0 14px 26px rgba(0,0,0,0.6));animation:float 4.5s ease-in-out infinite;transition:transform 0.25s ease;will-change:transform}.tcg-corner{position:absolute;font-size:0.62rem;color:rgba(255,255,255,0.45);font-weight:600;letter-spacing:0.08em;font-variant-numeric:tabular-nums}.tcg-corner.tl{top:8px;left:12px}.tcg-corner.tr{top:8px;right:12px}.tcg-corner.bl{bottom:8px;left:12px}.tcg-corner.br{bottom:8px;right:12px}.tcg-info{display:flex;justify-content:space-between;gap:12px;padding:10px 18px;border-top:1px solid rgba(255,255,255,0.06);border-bottom:1px solid rgba(255,255,255,0.06);background:rgba(255,255,255,0.02)}.tcg-types{display:flex;gap:6px;flex-wrap:wrap}.type-chip{display:inline-flex;align-items:center;gap:5px;padding:5px 11px;border-radius:999px;font-size:0.74rem;font-weight:700;color:#fff;text-shadow:0 1px 2px rgba(0,0,0,0.35);text-transform:capitalize;letter-spacing:0.02em;box-shadow:0 1px 0 rgba(255,255,255,0.2) inset,0 3px 8px rgba(0,0,0,0.3)}.tcg-measures{display:flex;gap:14px;font-size:0.72rem;color:#9b97b0;font-weight:600;letter-spacing:0.04em;font-variant-numeric:tabular-nums}.tcg-measures span{display:flex;flex-direction:column;text-align:right}.tcg-measures b{color:#f1f0f5;font-size:0.85rem;font-weight:700;letter-spacing:-0.01em}.tcg-section{padding:18px}.tcg-section + .tcg-section{padding-top:0}.tcg-section-title{font-size:0.68rem;color:#8b87a0;text-transform:uppercase;letter-spacing:0.14em;font-weight:700;margin-bottom:10px;display:flex;align-items:center;gap:8px}.tcg-section-title::before{content:"";flex:0 0 14px;height:1px;background:var(--type-color,#888);opacity:0.6}.tcg-section-title::after{content:"";flex:1;height:1px;background:linear-gradient(90deg,rgba(255,255,255,0.12),transparent)}.tcg-radar-wrap{display:flex;justify-content:center}.tcg-radar{width:100%;max-width:300px;height:auto;display:block}.radar-axis{stroke:rgba(255,255,255,0.06);stroke-width:1}.radar-grid{stroke:rgba(255,255,255,0.07);stroke-width:1;fill:none}.radar-fill{fill:var(--type-color,#888);fill-opacity:0.22;stroke:var(--type-color,#888);stroke-width:2;stroke-linejoin:round;filter:drop-shadow(0 0 6px var(--type-color,#888))}.radar-dot{fill:var(--type-color,#888);stroke:#000;stroke-width:1.5}.radar-label{fill:#9b97b0;font-size:9px;font-weight:700;letter-spacing:0.08em;text-anchor:middle;font-family:'Inter',system-ui,sans-serif}.radar-value{fill:#f1f0f5;font-size:8px;font-weight:600;text-anchor:middle;font-family:'Inter',system-ui,sans-serif}.tcg-abilities{display:flex;gap:8px;flex-wrap:wrap}.ability-chip{padding:6px 12px;border-radius:10px;background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.08);font-size:0.82rem;color:#e8e6f0;font-weight:600;text-transform:capitalize;letter-spacing:0.01em}.ability-chip.hidden{background:linear-gradient(135deg,rgba(244,114,182,0.15),rgba(255,255,255,0.15));border-color:rgba(244,114,182,0.3);color:#f9a8d4}.ability-chip.hidden::after{content:" ✨";font-size:0.7rem}.tcg-footer{margin-top:auto;padding:10px 18px;background:rgba(0,0,0,0.25);border-top:1px solid rgba(255,255,255,0.06);font-size:0.66rem;color:#6b6784;font-weight:600;letter-spacing:0.16em;text-transform:uppercase;display:flex;justify-content:space-between;align-items:center}.tcg-footer .dot{width:4px;height:4px;background:var(--type-color,#888);border-radius:50%;display:inline-block;margin:0 6px;opacity:0.7}.tcg-side{display:flex;flex-direction:column;gap:24px;padding-top:8px;animation:fadeSlideIn 0.6s ease 0.1s both;animation-fill-mode:both;opacity:0;animation-name:fadeSlideIn;animation-duration:0.6s;animation-delay:0.1s;animation-fill-mode:forwards}.tcg-back{display:inline-flex;align-items:center;gap:6px;color:#8b87a0;font-size:0.85rem;font-weight:600;transition:color 0.2s ease}.tcg-back:hover{color:#fff}.tcg-side-id{font-size:0.85rem;color:var(--type-color,#888);font-weight:700;letter-spacing:0.12em;font-variant-numeric:tabular-nums}.tcg-side-name{font-size:3.4rem;font-weight:900;letter-spacing:-0.03em;color:#fff;line-height:0.95;text-transform:capitalize;margin:4px 0 6px;background:linear-gradient(135deg,#fff 0%,color-mix(in srgb,var(--type-color,#888) 40%,#fff 60%) 100%);-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent}.tcg-side-tagline{font-size:0.95rem;color:#9b97b0;line-height:1.45;max-width:38ch}.tcg-side-stats{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}.side-stat{padding:12px;border-radius:12px;background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.06);text-align:center}.side-stat-label{font-size:0.65rem;color:#6b6784;text-transform:uppercase;letter-spacing:0.1em;font-weight:700;margin-bottom:2px}.side-stat-val{font-size:1.3rem;color:#f1f0f5;font-weight:800;letter-spacing:-0.02em;font-variant-numeric:tabular-nums}.tcg-side-types{display:flex;gap:8px;flex-wrap:wrap}.error{background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.2);border-radius:14px;padding:16px 20px;color:#fca5a5;text-align:center;font-size:0.95rem;animation:fadeSlideIn 0.3s ease both;max-width:480px;margin:48px auto}@media(max-width:880px){main.tcg-main{grid-template-columns:1fr;gap:32px;padding:32px 16px 56px}.tcg-side-name{font-size:2.4rem}.tcg-side-stats{grid-template-columns:repeat(3,1fr)}}@media(hover:none){.tcg-card{transform:none!important}}""");
    s
}

func headerHtml() -> String {
    var s = String(capacity: 768);
    s.append("""<header class="header"><a href="/" class="logo"><span class="logo-dot"></span>Kestrel Pokédex</a><div class="search-wrap"><span class="search-icon">🔍</span><input type="text" name="q" placeholder="Search Kanto..." autocomplete="off" hx-get="/search" hx-trigger="keyup changed delay:150ms, search" hx-target="#grid" hx-include="#type-filter" hx-push-url="false"><div class="htmx-indicator"><div class="search-spinner"></div></div></div></header>""");
    s
}

// ============================================================================
// LANDING PAGE (pre-rendered with full Kanto grid)
// ============================================================================

public func typeFilterRowHtml() -> String {
    var h = String(capacity: 2048);
    var t = Template();
    h.append("""<div class="type-filter"><button type="button" class="all-btn active" data-type="" onclick="setType(this)">All Types</button>""");
    let types = allTypes();
    var i: Int64 = 0;
    while i < types.count {
        let typeName = types(unchecked: i);
        t.setRaw("color", typeColor(typeName));
        t.setRaw("colorDark", typeColorDark(typeName));
        t.put("typeName", capitalize(typeName));
        t.setRaw("type", typeName);
        t.put("emoji", typeEmoji(typeName));
        h.append(t.render("""<button type="button" data-type="{type}" onclick="setType(this)" style="--btn-color:{color};background:linear-gradient(135deg,{color},{colorDark})">{emoji} {typeName}</button>"""));
        i = i + 1
    }
    h.append("</div>");
    h
}

public func landingPageHtml() -> String {
    var h = String(capacity: 81920);
    h.append("""<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Kestrel Pokédex — Kanto</title><link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap" rel="stylesheet"><script src="https://unpkg.com/htmx.org@1.9.10"></script><style>""");
    h.append(baseCss());
    h.append(gridCss());
    h.append("""</style></head><body>""");
    h.append(headerHtml());
    h.append("""<input type="hidden" name="type" id="type-filter" value="">""");
    h.append("""<main class="grid-main"><div class="grid-hero"><h1>Kanto Pokédex</h1><p>The original 151. Tap one to peek at its trading card.</p><div class="grid-count">Showing <b>151</b> of 151 Pokémon</div></div>""");
    h.append(typeFilterRowHtml());
    h.append("""<div id="grid" class="grid">""");
    h.append(gridItemsHtml(kantoPokedex()));
    h.append("""</div>""");
    h.append("""<footer class="grid-footer">Built with <a href="https://github.com/anthropics/kestrel">Kestrel</a><span class="sep">•</span>Data from <a href="https://pokeapi.co">PokéAPI</a><span class="sep">•</span>Sprites by <a href="https://github.com/PokeAPI/sprites">PokeAPI/sprites</a></footer>""");
    h.append("""</main>""");
    h.append(tiltScript());
    h.append(landingScript());
    h.append("</body></html>");
    h
}

func landingScript() -> String {
    """<script>
(function(){
  function updateCount(){
    var c=document.querySelectorAll('#grid .card').length;
    var el=document.querySelector('.grid-count b');
    if(el){el.textContent=c;}
  }
  window.setType=function(btn){
    document.querySelectorAll('.type-filter button').forEach(function(b){b.classList.remove('active');});
    btn.classList.add('active');
    document.getElementById('type-filter').value=btn.dataset.type;
    var input=document.querySelector('input[name=q]');
    if(input){htmx.trigger(input,'search');}
  };
  document.body.addEventListener('htmx:afterSwap',function(e){
    if(e.target&&e.target.id==='grid'){updateCount();}
  });
  updateCount();
})();
</script>"""
}

// ============================================================================
// GRID ITEMS (used both for landing and search responses)
// ============================================================================

public func gridItemsHtml(entries: Array[PokemonEntry]) -> String {
    var h = String(capacity: 65536);
    var t = Template();
    var i: Int64 = 0;
    while i < entries.count {
        let e = entries(unchecked: i);
        t.setInt("id", e.id);
        t.setRaw("idPad", padId(e.id, 3));
        t.put("name", e.displayName);
        t.setInt("delay", i * 8);
        t.setRaw("color", typeColor(e.primaryType));
        t.setRaw("type", e.primaryType);
        h.append(t.render("""<a class="card" data-type="{type}" style="--card-color:{color};animation-delay:{delay}ms" href="/pokemon?id={id}"><img class="card-sprite" src="https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/{id}.png" alt="{name}" loading="lazy"><div class="card-id">#{idPad}</div><div class="card-name">{name}</div></a>"""));
        i = i + 1
    }
    h
}

// ============================================================================
// SEARCH (filter the Kanto list locally)
// ============================================================================

public func filterKanto(query: String, typeFilter: String) -> Array[PokemonEntry] {
    let q = toLower(query);
    var out = Array[PokemonEntry]();
    let all = kantoPokedex();
    var i: Int64 = 0;
    while i < all.count {
        let e = all(unchecked: i);
        let nameMatch = q.byteCount == 0 or containsLower(e.apiName, q) or containsLower(e.displayName, q);
        let typeMatch = typeFilter.byteCount == 0 or e.primaryType == typeFilter;
        if nameMatch and typeMatch {
            out.append(e)
        };
        i = i + 1
    }
    out
}

// ============================================================================
// RADAR CHART (SVG)
//
// Hexagonal radar with 3 concentric gridline hexagons, 6 axis lines, the
// stat-value polygon (filled with the type color at low alpha, stroked at
// full), a dot at each vertex, axis labels, and the actual stat values
// printed near each label.
// ============================================================================

func hexPoints(cx: Float64, cy: Float64, r: Float64) -> String {
    var s = String();
    var i: Int64 = 0;
    while i < 6 {
        let x = cx + r * unitX(i);
        let y = cy + r * unitY(i);
        s.append(roundInt(x).format());
        s.append(",");
        s.append(roundInt(y).format());
        if i < 5 { s.append(" ") };
        i = i + 1
    }
    s
}

public func statsRadarSvg(statsArr: Array[Value]) -> String {
    var s = String(capacity: 2048);
    let cx: Float64 = 150.0;
    let cy: Float64 = 130.0;
    let r: Float64 = 84.0;
    let labelR: Float64 = 102.0;
    let valueR: Float64 = 116.0;

    s.append("""<svg viewBox="0 0 300 260" class="tcg-radar" xmlns="http://www.w3.org/2000/svg">""");

    // 3 concentric gridline hexagons (33%, 66%, 100% of radius)
    s.append("<polygon class=\"radar-grid\" points=\"");
    s.append(hexPoints(cx, cy, r * 0.33));
    s.append("\"/>");
    s.append("<polygon class=\"radar-grid\" points=\"");
    s.append(hexPoints(cx, cy, r * 0.66));
    s.append("\"/>");
    s.append("<polygon class=\"radar-grid\" points=\"");
    s.append(hexPoints(cx, cy, r));
    s.append("\"/>");

    // 6 axis lines from center to each vertex
    var ai: Int64 = 0;
    while ai < 6 {
        let ex = cx + r * unitX(ai);
        let ey = cy + r * unitY(ai);
        var t = Template();
        t.setInt("x1", roundInt(cx));
        t.setInt("y1", roundInt(cy));
        t.setInt("x2", roundInt(ex));
        t.setInt("y2", roundInt(ey));
        s.append(t.render("""<line class="radar-axis" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}"/>"""));
        ai = ai + 1
    }

    // Stat polygon — walk vertices clockwise so the edges don't cross
    s.append("<polygon class=\"radar-fill\" points=\"");
    var pi: Int64 = 0;
    while pi < 6 {
        let statIdx = angleToStatIdx(pi);
        let stat = statsArr(unchecked: statIdx);
        let value = getInt(getField(stat, "base_stat"));
        let cap = if value > 200 { 200 } else { value };
        let normalized = (Float64(from: cap) / 200.0) * r;
        let x = cx + normalized * unitX(pi);
        let y = cy + normalized * unitY(pi);
        s.append(roundInt(x).format());
        s.append(",");
        s.append(roundInt(y).format());
        if pi < 5 { s.append(" ") };
        pi = pi + 1
    }
    s.append("\"/>");

    // Dots + labels + values
    var di: Int64 = 0;
    while di < 6 {
        let stat = statsArr(unchecked: di);
        let value = getInt(getField(stat, "base_stat"));
        let cap = if value > 200 { 200 } else { value };
        let normalized = (Float64(from: cap) / 200.0) * r;
        let angIdx = statAngleIdx(di);
        let dotX = cx + normalized * unitX(angIdx);
        let dotY = cy + normalized * unitY(angIdx);
        let labX = cx + labelR * unitX(angIdx);
        let labY = cy + labelR * unitY(angIdx);
        let valX = cx + valueR * unitX(angIdx);
        let valY = cy + valueR * unitY(angIdx);
        var t = Template();
        t.setInt("dx", roundInt(dotX));
        t.setInt("dy", roundInt(dotY));
        t.setInt("lx", roundInt(labX));
        t.setInt("ly", roundInt(labY));
        t.setInt("vx", roundInt(valX));
        t.setInt("vy", roundInt(valY));
        t.put("label", statShortLabel(di));
        t.setInt("value", value);
        s.append(t.render("""<circle class="radar-dot" cx="{dx}" cy="{dy}" r="3.2"/><text class="radar-label" x="{lx}" y="{ly}">{label}</text><text class="radar-value" x="{vx}" y="{vy}">{value}</text>"""));
        di = di + 1
    }

    s.append("</svg>");
    s
}

// ============================================================================
// TILT + PARALLAX SCRIPT
//
// Event-delegated mousemove on the document. Grid cards get a perspective
// rotateX/Y up to 12°; the TCG detail card gets a gentler 8° plus
// cursor-tracked --mx/--my CSS vars driving the holographic foil overlay.
// Re-applies cleanly after every htmx grid swap (no listeners needed —
// delegation handles new nodes automatically).
// ============================================================================

func tiltScript() -> String {
    """<script>
(function(){
  function applyTilt(el,e,maxDeg,withPerspective,foilCard){
    var r=el.getBoundingClientRect();
    var x=(e.clientX-r.left)/r.width;
    var y=(e.clientY-r.top)/r.height;
    var rx=(0.5-y)*maxDeg;
    var ry=(x-0.5)*maxDeg;
    var t=withPerspective
      ?'perspective(900px) rotateX('+rx.toFixed(2)+'deg) rotateY('+ry.toFixed(2)+'deg)'
      :'rotateX('+rx.toFixed(2)+'deg) rotateY('+ry.toFixed(2)+'deg) scale(1.04)';
    el.style.transform=t;
    el.style.setProperty('--mx',(x*100).toFixed(1)+'%');
    el.style.setProperty('--my',(y*100).toFixed(1)+'%');
    el.style.setProperty('--mx-deg',(x*360).toFixed(1)+'deg');
    if(foilCard){el.style.setProperty('--foil-opacity','0.85');}
  }
  function reset(el,foilCard){
    el.style.transform='';
    el.style.setProperty('--mx','50%');
    el.style.setProperty('--my','50%');
    if(foilCard){el.style.setProperty('--foil-opacity','0');}
  }
  function tiltTarget(target){
    if(!target||!target.closest)return null;
    var c=target.closest('.card');
    if(c)return {el:c,deg:22,persp:false,foil:false};
    var t=target.closest('.tcg-card');
    if(t)return {el:t,deg:9,persp:true,foil:true};
    return null;
  }
  document.addEventListener('mousemove',function(e){
    var hit=tiltTarget(e.target);
    if(hit)applyTilt(hit.el,e,hit.deg,hit.persp,hit.foil);
  });
  // mouseout fires for every transition between elements; only reset when
  // we're actually leaving the tiltable element (relatedTarget is outside).
  document.addEventListener('mouseout',function(e){
    var hit=tiltTarget(e.target);
    if(!hit)return;
    if(hit.el.contains(e.relatedTarget))return;
    reset(hit.el,hit.foil);
  });
})();
</script>"""
}

// ============================================================================
// DETAIL PAGE
// ============================================================================

public func detailPageHtml(json: Value, id: Int64) -> String {
    var h = String(capacity: 24576);
    var t = Template();

    let entry = kantoEntryById(id);
    let display = entry.displayName;

    let height = getInt(getField(json, "height"));
    let weight = getInt(getField(json, "weight"));

    let typesArr = getArrayField(json, "types");
    let statsArr = getArrayField(json, "stats");
    let abilitiesArr = getArrayField(json, "abilities");

    // Primary + secondary type (or duplicate primary if mono-type)
    let primaryType = if typesArr.count > 0 {
        getString(getField(getField(typesArr(unchecked: 0), "type"), "name"))
    } else { "normal" };
    let secondaryType = if typesArr.count > 1 {
        getString(getField(getField(typesArr(unchecked: 1), "type"), "name"))
    } else { primaryType };

    // HP is stat index 0 by PokeAPI convention.
    let hpValue = if statsArr.count > 0 {
        getInt(getField(statsArr(unchecked: 0), "base_stat"))
    } else { 0 };

    // BST = sum of all 6 base stats
    var bst: Int64 = 0;
    var bi: Int64 = 0;
    while bi < statsArr.count {
        bst = bst + getInt(getField(statsArr(unchecked: bi), "base_stat"));
        bi = bi + 1
    }

    h.append("""<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">""");

    t.put("name", display);
    h.append(t.render("""<title>{name} — Kestrel Pokédex</title>"""));

    h.append("""<link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap" rel="stylesheet"><script src="https://unpkg.com/htmx.org@1.9.10"></script><style>""");
    h.append(baseCss());
    h.append(tcgCss());
    h.append("</style></head>");

    // Body with type-derived CSS variables
    t.setRaw("c1", typeColor(primaryType));
    t.setRaw("c2", typeColor(secondaryType));
    t.setRaw("c1d", typeColorDark(primaryType));
    t.setRaw("glow", typeGlow(primaryType));
    h.append(t.render("""<body class="tcg-body" style="--type-color:{c1};--type-color-2:{c2};--type-color-dark:{c1d};--type-glow:{glow};">"""));

    h.append(headerHtml());

    h.append("""<main class="tcg-main">""");

    // ---- LEFT: TCG CARD ----
    h.append("""<div class="tcg-card">""");
    h.append("""<div class="tcg-inner">""");

    // Header strip: name + HP
    t.put("name", display);
    t.setInt("hp", hpValue);
    h.append(t.render("""<div class="tcg-header"><div><div class="tcg-stage">Basic Pokémon</div><div class="tcg-name">{name}</div></div><div class="tcg-hp"><span class="tcg-hp-prefix">HP</span><span class="tcg-hp-val">{hp}</span></div></div>"""));

    // Art window with corner decorations
    t.setInt("id", id);
    t.put("name", display);
    t.setRaw("idPad", padId(id, 3));
    t.put("primaryType", capitalize(primaryType));
    h.append(t.render("""<div class="tcg-art"><span class="tcg-corner tl">No. {idPad}</span><span class="tcg-corner tr">{primaryType}</span><img src="https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{id}.png" alt="{name}" loading="eager"><span class="tcg-corner bl">KANTO</span><span class="tcg-corner br">GEN I</span></div>"""));

    // Info row: type chips + measurements
    h.append("""<div class="tcg-info"><div class="tcg-types">""");
    var ti: Int64 = 0;
    while ti < typesArr.count {
        let typeName = getString(getField(getField(typesArr(unchecked: ti), "type"), "name"));
        t.put("typeName", capitalize(typeName));
        t.setRaw("typeC", typeColor(typeName));
        t.setRaw("typeCD", typeColorDark(typeName));
        t.setRaw("typeEmoji", typeEmoji(typeName));
        h.append(t.render("""<span class="type-chip" style="background:linear-gradient(135deg,{typeC},{typeCD})">{typeEmoji} {typeName}</span>"""));
        ti = ti + 1
    }
    h.append("</div>");
    t.setRaw("height", formatMeters(height));
    t.setRaw("weight", formatKilos(weight));
    h.append(t.render("""<div class="tcg-measures"><span>Height<b>{height}</b></span><span>Weight<b>{weight}</b></span></div></div>"""));

    // Stats radar
    h.append("""<div class="tcg-section"><div class="tcg-section-title">Base Stats</div><div class="tcg-radar-wrap">""");
    h.append(statsRadarSvg(statsArr));
    h.append("</div></div>");

    // Abilities
    h.append("""<div class="tcg-section"><div class="tcg-section-title">Abilities</div><div class="tcg-abilities">""");
    var aIdx: Int64 = 0;
    while aIdx < abilitiesArr.count {
        let a = abilitiesArr(unchecked: aIdx);
        let abName = getString(getField(getField(a, "ability"), "name"));
        let isHidden = match getField(a, "is_hidden").asBool() {
            .Some(b) => b,
            .None => false
        };
        let cls = if isHidden { "ability-chip hidden" } else { "ability-chip" };
        // Replace kebab-case dashes with spaces for display
        var displayAb = String();
        var k: Int64 = 0;
        while k < abName.byteCount {
            let b = abName.bytes(unchecked: k);
            if b == 45 { displayAb.appendByte(32) } else { displayAb.appendByte(b) };
            k = k + 1
        }
        t.setRaw("cls", cls);
        t.put("ab", displayAb);
        h.append(t.render("""<span class="{cls}">{ab}</span>"""));
        aIdx = aIdx + 1
    }
    h.append("</div></div>");

    // Footer
    t.setRaw("idPad", padId(id, 3));
    h.append(t.render("""<div class="tcg-footer"><span>Kestrel Pokédex<span class="dot"></span>Kanto</span><span>#{idPad} / 151</span></div>"""));

    h.append("""</div></div>"""); // close .tcg-inner, .tcg-card

    // ---- RIGHT: SIDE PANEL ----
    h.append("""<div class="tcg-side">""");
    h.append("""<a href="/" class="tcg-back">← Back to Kanto Pokédex</a>""");

    t.setRaw("idPad", padId(id, 3));
    t.put("name", display);
    h.append(t.render("""<div><div class="tcg-side-id">#{idPad}</div><div class="tcg-side-name">{name}</div></div>"""));

    // Type chips (bigger version on the side)
    h.append("""<div class="tcg-side-types">""");
    var tii: Int64 = 0;
    while tii < typesArr.count {
        let typeName = getString(getField(getField(typesArr(unchecked: tii), "type"), "name"));
        t.put("typeName", capitalize(typeName));
        t.setRaw("typeC", typeColor(typeName));
        t.setRaw("typeCD", typeColorDark(typeName));
        t.setRaw("typeEmoji", typeEmoji(typeName));
        h.append(t.render("""<span class="type-chip" style="background:linear-gradient(135deg,{typeC},{typeCD});padding:8px 16px;font-size:0.88rem">{typeEmoji} {typeName}</span>"""));
        tii = tii + 1
    }
    h.append("</div>");

    // Side stat tiles: BST, HP, Speed
    let speedValue = if statsArr.count > 5 {
        getInt(getField(statsArr(unchecked: 5), "base_stat"))
    } else { 0 };
    t.setInt("bst", bst);
    t.setInt("hp2", hpValue);
    t.setInt("spe", speedValue);
    t.setRaw("h", formatMeters(height));
    t.setRaw("w", formatKilos(weight));
    h.append(t.render("""<div class="tcg-side-stats"><div class="side-stat"><div class="side-stat-label">BST</div><div class="side-stat-val">{bst}</div></div><div class="side-stat"><div class="side-stat-label">HP</div><div class="side-stat-val">{hp2}</div></div><div class="side-stat"><div class="side-stat-label">Speed</div><div class="side-stat-val">{spe}</div></div></div>"""));

    h.append("""<div class="tcg-side-tagline">Move your cursor over the card to see the holo shimmer. Hover the grid back home to peek individual cards.</div>""");

    h.append("</div>"); // close .tcg-side

    h.append("</main>");
    h.append(tiltScript());
    h.append("</body></html>");
    h
}

// ============================================================================
// ERROR (used for failed fetches)
// ============================================================================

public func errorPageHtml(msg: String) -> String {
    var h = String(capacity: 4096);
    var t = Template();
    h.append("""<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Error — Kestrel Pokédex</title><link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap" rel="stylesheet"><script src="https://unpkg.com/htmx.org@1.9.10"></script><style>""");
    h.append(baseCss());
    h.append(tcgCss());
    h.append("</style></head><body>");
    h.append(headerHtml());
    t.put("msg", msg);
    h.append(t.render("""<main class="tcg-main" style="grid-template-columns:1fr"><div class="error">{msg}</div></main>"""));
    h.append("</body></html>");
    h
}
