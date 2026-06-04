---
name: kestrel-mutating-callsite
description: mutating access mode is declaration-only — not repeated at the call site
metadata:
  type: feedback
---

`mutating` on a parameter is declaration-side only. At the call site, just use the label.

**Why:** `mutating` is an access mode annotation, not part of the call-site syntax. e.g. `func format(mutating into writer: StringBuilder)` is called as `x.format(into: s)`, not `x.format(mutating into: s)`.

**How to apply:** When writing Kestrel call expressions with mutating parameters, omit `mutating`. See also [[kestrel_mutating_params]].
