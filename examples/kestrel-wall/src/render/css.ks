module wall.render

public func wallCss() -> String {
    #"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #2c2c2c;
    color: #333;
    overflow: hidden;
    height: 100vh;
}

.post-form {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    max-width: 700px;
    margin: 0 auto;
    padding: 0.6rem 1rem;
    background: #FEFF9C;
    border-radius: 8px 8px 0 0;
    box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.3);
    z-index: 10;
    display: flex;
    gap: 0.5rem;
    align-items: center;
}

.post-form input {
    border: none;
    background: transparent;
    font-family: 'Caveat', 'Segoe Print', cursive, sans-serif;
    font-size: 1.1rem;
    padding: 0.3rem 0;
    outline: none;
    color: #333;
}

.post-form input::placeholder {
    color: rgba(0, 0, 0, 0.35);
}

.post-form input[name=username] {
    width: 100px;
    flex-shrink: 0;
    border-right: 1px dashed rgba(0, 0, 0, 0.2);
    padding-right: 0.5rem;
}

.post-form input[name=message] {
    flex: 1;
}

.post-form button {
    padding: 0.3rem 1rem;
    background: rgba(0, 0, 0, 0.08);
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-radius: 3px;
    font-family: inherit;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.15s;
    flex-shrink: 0;
    white-space: nowrap;
}

.post-form button:hover {
    background: rgba(0, 0, 0, 0.15);
}

.toast {
    position: fixed;
    top: 1rem;
    left: 50%;
    transform: translateX(-50%);
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 500;
    z-index: 100;
    opacity: 0;
    transition: opacity 0.3s;
    pointer-events: none;
}

.toast.show { opacity: 1; }
.toast.error { background: #ff4444; color: white; }
.toast.success { background: #44bb44; color: white; }

.viewport {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 40px;
    overflow: hidden;
    cursor: grab;
    background: #2c2c2c;
    background-image:
        radial-gradient(circle at 20% 50%, rgba(120, 80, 60, 0.15) 0%, transparent 50%),
        radial-gradient(circle at 80% 20%, rgba(60, 80, 120, 0.1) 0%, transparent 50%);
}

.viewport.dragging { cursor: grabbing; }

.wall {
    position: absolute;
    top: 0;
    left: 0;
    width: 2400px;
    height: 1500px;
}

.wall-title {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
    pointer-events: none;
    user-select: none;
    z-index: 0;
}

.wall-title h1 {
    font-size: 4rem;
    font-weight: 800;
    letter-spacing: -0.03em;
    color: rgba(255, 255, 255, 0.06);
}

.wall-title p {
    font-size: 1.2rem;
    color: rgba(255, 255, 255, 0.04);
    margin-top: 0.25rem;
}

.note {
    position: absolute;
    width: 200px;
    background: #FEFF9C;
    padding: 1rem;
    border-radius: 2px;
    box-shadow: 2px 3px 8px rgba(0, 0, 0, 0.25);
    word-break: break-word;
    opacity: 0;
    transition: opacity 0.3s, box-shadow 0.15s;
    user-select: none;
}

.note.placed {
    opacity: 1;
}

.note:hover {
    box-shadow: 3px 5px 14px rgba(0, 0, 0, 0.35);
    z-index: 2;
}

.note-message {
    font-family: 'Caveat', 'Segoe Print', cursive, sans-serif;
    font-size: 1.2rem;
    line-height: 1.4;
    color: #333;
}

.note-author {
    margin-top: 0.75rem;
    font-size: 0.8rem;
    opacity: 0.6;
    text-align: right;
    font-style: italic;
}

@media (max-width: 600px) {
    .note { width: 150px; }
}
"#
}
