# 🪶 Kestrel Launch Checklist

## 🔴 Blockers — must be done before launch

**Website**
- [ ] Fill empty sections in the launch post: both `Getting Started`, `Community`, **`Kestrel Wall` blurb**, `LSP`, + 4 empty bugfix lines
- [ ] Reframe benchmarks → **efficiency-forward** (lead with size/memory/startup wins; demote throughput; add the "raw throughput ~3000 req/s, HTML render is the pre-LLVM bottleneck" context)
- [ ] Verify OG/social meta tags + a good share image (drives click-through on HN/Reddit/Twitter)
- [ ] Run one clean prod build (`build-search-index` + `build-llms` + `next build`)

**Install (the one thing that must not break)**
- [ ] Smoke-test `curl … | sh` on a *clean* macOS box → `kestrel --version`, `jessup list`
- [ ] Same on a clean Linux box (Ubuntu/Debian)

**Community homes (set up BEFORE opening the firehoses)**
- [ ] Discord server + **non-expiring invite** + channels (`#announcements #general #help #showcase #contributing #bugs`)
- [x] Twitter
- [ ] Enable **GitHub Discussions**
- [ ] Wire both links into site footer/nav **and** the blog post's Community section

**Email**
- [x] Cloudflare Email Routing: `hello@`, `security@`, `john@` → your inbox
- [ ] Gmail "Send mail as" for replies
- [ ] **SPF + DKIM + DMARC** DNS records (Cloudflare)
- [ ] `SECURITY.md` → `security@`; put `hello@` in footer/blog/repo

**Launch posts (drafted + ready to fire)**
- [ ] **Decide narrative angle** — AI-assisted story vs. "6 months solo" (changes all copy)
- [ ] Show HN post (technical + humble)
- [ ] r/ProgrammingLanguages post (design-curious)
- [ ] Launch thread for Twitter/X + Bluesky

## 🟠 High-impact (do if time allows)
- [ ] Home-page proof strip: **`2.3 MB binary · 2.7 MB RAM · 9 ms cold start`**
- [ ] "60-second start" path on the home page (install → hello world → run)
- [ ] Email-capture box (owned audience that compounds)
- [ ] Tag a few **good-first-issues**; confirm `CONTRIBUTING.md` is current
- [ ] Fill 3 placeholder index pages (`tooling/`, `concepts/`, `reference/`)
- [ ] Basic Discord rules / `CODE_OF_CONDUCT.md`

## ⚪ Post-launch
- [ ] Replace stale repo README (still the Vite template)
- [ ] Deeper guides + steady blog cadence

## 🚀 Launch morning (run order)
1. Final build deployed + install verified ✅
2. Post **Show HN** (~8–10am US Pacific, weekday)
3. Post **r/ProgrammingLanguages**
4. Launch thread on Twitter/Bluesky
5. **Sit in the HN + Reddit threads for 6+ hours** answering everything — this is the single highest-leverage thing you'll do all day
6. (Stagger r/rust, r/programming, Lobsters for later, not same-minute)
