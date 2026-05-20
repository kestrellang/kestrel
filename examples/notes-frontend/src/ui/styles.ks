module notes.ui

public func appCss() -> String {
    var s = String(capacity: 12288);

    // Reset & base
    s.append(##"*{box-sizing:border-box;margin:0;padding:0}"##);
    s.append(##"body{font-family:'Inter',system-ui,-apple-system,sans-serif;background:#09090b;color:#e4e4e7;min-height:100vh;-webkit-font-smoothing:antialiased}"##);
    s.append(##"a{color:inherit;text-decoration:none}"##);

    // App layout
    s.append(##".app{display:grid;grid-template-columns:260px 1fr;grid-template-rows:auto 1fr;min-height:100vh}"##);

    // Topbar
    s.append(##".topbar{grid-column:1/-1;display:flex;align-items:center;justify-content:space-between;padding:0 24px;height:52px;background:#09090b;border-bottom:1px solid rgba(255,255,255,0.06)}"##);
    s.append(##".topbar-brand{display:flex;align-items:center;gap:8px;font-size:0.9rem;font-weight:700;color:#fafafa;letter-spacing:-0.02em}"##);
    s.append(##".topbar-brand i{color:#3b82f6}"##);
    s.append(##".topbar-actions{display:flex;gap:6px;align-items:center}"##);

    // Sidebar
    s.append(##".sidebar{background:#09090b;border-right:1px solid rgba(255,255,255,0.06);padding:16px 0;overflow-y:auto}"##);
    s.append(##".sidebar-section{padding:0 12px;margin-bottom:20px}"##);
    s.append(##".sidebar-label{font-size:0.65rem;text-transform:uppercase;letter-spacing:0.1em;color:#52525b;font-weight:600;padding:0 8px;margin-bottom:8px}"##);
    s.append(##".folder-item{display:flex;align-items:center;gap:8px;padding:7px 10px;border-radius:6px;font-size:0.83rem;color:#a1a1aa;cursor:pointer;transition:all 0.12s ease;margin-bottom:1px}"##);
    s.append(##".folder-item i{flex-shrink:0;width:16px;height:16px;opacity:0.5}"##);
    s.append(##".folder-item:hover{background:rgba(255,255,255,0.04);color:#d4d4d8}"##);
    s.append(##".folder-item:hover i{opacity:0.7}"##);
    s.append(##".folder-item.active{background:rgba(59,130,246,0.1);color:#93bbfd}"##);
    s.append(##".folder-item.active i{opacity:1;color:#3b82f6}"##);
    s.append(##".folder-name{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}"##);

    // Main content
    s.append(##".content{padding:28px 36px;overflow-y:auto;background:#0c0c0f}"##);

    // Note list
    s.append(##".note-list-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:24px}"##);
    s.append(##".note-list-title{font-size:1.15rem;font-weight:600;color:#fafafa;letter-spacing:-0.02em}"##);
    s.append(##".note-list{display:flex;flex-direction:column;gap:6px}"##);

    // Note card
    s.append(##".note-card{display:block;padding:14px 16px;background:rgba(255,255,255,0.02);border:1px solid rgba(255,255,255,0.05);border-radius:10px;cursor:pointer;transition:all 0.15s ease}"##);
    s.append(##".note-card:hover{background:rgba(255,255,255,0.04);border-color:rgba(255,255,255,0.1)}"##);
    s.append(##".note-card-title{font-size:0.88rem;font-weight:600;color:#e4e4e7;margin-bottom:3px}"##);
    s.append(##".note-card-preview{font-size:0.78rem;color:#71717a;line-height:1.45;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}"##);
    s.append(##".note-card-meta{display:flex;align-items:center;gap:4px;font-size:0.68rem;color:#3f3f46;margin-top:8px}"##);
    s.append(##".note-card-meta i{width:12px;height:12px}"##);

    // Editor / detail view
    s.append(##".editor{max-width:680px}"##);
    s.append(##".editor-toolbar{display:flex;align-items:center;gap:6px;margin-bottom:24px;padding-bottom:16px;border-bottom:1px solid rgba(255,255,255,0.06)}"##);
    s.append(##".editor-toolbar .spacer{flex:1}"##);
    s.append(##".editor-title{width:100%;background:transparent;border:none;color:#fafafa;font-size:1.6rem;font-weight:700;font-family:inherit;outline:none;padding:0;margin-bottom:20px;letter-spacing:-0.03em}"##);
    s.append(##".editor-title::placeholder{color:#27272a}"##);
    s.append(##".editor-body{width:100%;background:transparent;border:none;color:#d4d4d8;font-size:0.9rem;font-family:inherit;outline:none;padding:0;line-height:1.75;resize:none;min-height:400px}"##);
    s.append(##".editor-body::placeholder{color:#27272a}"##);
    s.append(##".note-title{font-size:1.6rem;font-weight:700;color:#fafafa;margin-bottom:20px;letter-spacing:-0.03em}"##);
    s.append(##".note-body{color:#a1a1aa;font-size:0.9rem;line-height:1.75;white-space:pre-wrap}"##);
    s.append(##".note-meta{font-size:0.72rem;color:#3f3f46;display:flex;align-items:center;gap:4px}"##);
    s.append(##".note-meta i{width:12px;height:12px}"##);

    // Buttons
    s.append(##".btn{display:inline-flex;align-items:center;gap:5px;padding:6px 12px;border-radius:6px;font-size:0.78rem;font-weight:500;font-family:inherit;border:none;cursor:pointer;transition:all 0.12s ease;white-space:nowrap}"##);
    s.append(##".btn i{flex-shrink:0}"##);
    s.append(##".btn-primary{background:#3b82f6;color:#fff}.btn-primary:hover{background:#2563eb}"##);
    s.append(##".btn-ghost{background:transparent;color:#71717a}.btn-ghost:hover{background:rgba(255,255,255,0.05);color:#a1a1aa}"##);
    s.append(##".btn-danger{background:transparent;color:#71717a}.btn-danger:hover{background:rgba(239,68,68,0.08);color:#ef4444}"##);
    s.append(##".btn-sm{padding:5px 8px;font-size:0.75rem}"##);

    // Auth pages
    s.append(##".auth-page{display:flex;align-items:center;justify-content:center;min-height:100vh;background:#09090b}"##);
    s.append(##".auth-card{width:100%;max-width:360px;padding:40px 32px}"##);
    s.append(##".auth-logo{display:flex;align-items:center;justify-content:center;gap:8px;margin-bottom:32px;font-size:1.1rem;font-weight:700;color:#fafafa}"##);
    s.append(##".auth-logo i{color:#3b82f6}"##);
    s.append(##".auth-title{font-size:1.25rem;font-weight:700;color:#fafafa;margin-bottom:4px;text-align:center;letter-spacing:-0.02em}"##);
    s.append(##".auth-subtitle{font-size:0.82rem;color:#52525b;margin-bottom:24px;text-align:center}"##);
    s.append(##".field{margin-bottom:14px}"##);
    s.append(##".field-label{display:block;font-size:0.75rem;font-weight:500;color:#71717a;margin-bottom:5px}"##);
    s.append(##".field-input{width:100%;padding:9px 12px;background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.08);border-radius:6px;color:#e4e4e7;font-size:0.85rem;font-family:inherit;outline:none;transition:border-color 0.15s}"##);
    s.append(##".field-input:focus{border-color:rgba(59,130,246,0.5)}"##);
    s.append(##".field-input::placeholder{color:#3f3f46}"##);
    s.append(##".auth-submit{width:100%;margin-top:6px;padding:9px 0;justify-content:center;font-size:0.85rem}"##);
    s.append(##".auth-link{display:block;text-align:center;margin-top:20px;font-size:0.78rem;color:#52525b}"##);
    s.append(##".auth-link a{color:#3b82f6;font-weight:500}"##);
    s.append(##".auth-link a:hover{text-decoration:underline}"##);
    s.append(##".auth-divider{height:1px;background:rgba(255,255,255,0.06);margin:20px 0}"##);

    // Alerts
    s.append(##".alert{padding:10px 12px;border-radius:6px;font-size:0.78rem;margin-bottom:16px;display:flex;align-items:center;gap:8px}"##);
    s.append(##".alert i{flex-shrink:0;width:14px;height:14px}"##);
    s.append(##".alert-error{background:rgba(239,68,68,0.06);border:1px solid rgba(239,68,68,0.12);color:#fca5a5}"##);

    // Empty state
    s.append(##".empty{text-align:center;padding:80px 24px;color:#3f3f46}"##);
    s.append(##".empty i{margin-bottom:16px;opacity:0.3}"##);
    s.append(##".empty-text{font-size:0.88rem;margin-bottom:20px;color:#52525b}"##);

    // New folder input
    s.append(##".new-folder-input{width:100%;padding:6px 10px;background:rgba(255,255,255,0.02);border:1px solid rgba(255,255,255,0.06);border-radius:6px;color:#a1a1aa;font-size:0.78rem;font-family:inherit;outline:none;transition:border-color 0.15s}"##);
    s.append(##".new-folder-input:focus{border-color:rgba(59,130,246,0.4);background:rgba(255,255,255,0.04)}"##);
    s.append(##".new-folder-input::placeholder{color:#3f3f46}"##);

    // HTMX
    s.append(##".htmx-indicator{display:none}.htmx-request .htmx-indicator{display:inline-block}"##);

    // Transitions
    s.append(##"@keyframes fadeIn{from{opacity:0;transform:translateY(4px)}to{opacity:1;transform:translateY(0)}}"##);
    s.append(##".note-list .note-card{animation:fadeIn 0.2s ease both}"##);

    // Folder picker
    s.append(##".folder-picker{background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.08);border-radius:6px;color:#a1a1aa;font-size:0.75rem;font-family:inherit;padding:4px 8px;outline:none;cursor:pointer;margin-left:6px}.folder-picker:hover{border-color:rgba(255,255,255,0.15)}.folder-picker:focus{border-color:rgba(59,130,246,0.5)}.folder-picker option{background:#18181b;color:#e4e4e7}"##);

    // Responsive
    s.append(##"@media(max-width:768px){.app{grid-template-columns:1fr}.sidebar{display:none}.content{padding:20px 16px}}"##);

    s
}
