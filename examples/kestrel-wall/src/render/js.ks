module wall.render

public func wallJs() -> String {
    #"
(function() {
    var form = document.getElementById('post-form');
    var viewport = document.getElementById('viewport');
    var wall = document.getElementById('wall');
    var toast = document.getElementById('toast');
    var toastTimeout = null;
    var placed = [];
    var PAD = 14;
    var WALL_W = 2400;
    var WALL_H = 1500;

    // --- pan state ---
    var panX = 0, panY = 0;
    var dragging = false;
    var dragStartX = 0, dragStartY = 0;
    var panStartX = 0, panStartY = 0;

    function clampPan() {
        var vw = viewport.clientWidth;
        var vh = viewport.clientHeight;
        var minX = vw - WALL_W;
        var minY = vh - WALL_H;
        if (panX > 0) panX = 0;
        if (panY > 0) panY = 0;
        if (panX < minX) panX = minX;
        if (panY < minY) panY = minY;
    }

    function applyPan() {
        clampPan();
        wall.style.transform = 'translate(' + panX + 'px, ' + panY + 'px)';
    }

    function panTo(x, y, smooth) {
        panX = x;
        panY = y;
        clampPan();
        if (smooth) {
            wall.style.transition = 'transform 0.4s ease';
            applyPan();
            setTimeout(function() { wall.style.transition = ''; }, 400);
        } else {
            applyPan();
        }
    }

    function scrollToNote(note) {
        var vw = viewport.clientWidth;
        var vh = viewport.clientHeight;
        var nx = parseFloat(note.style.left) || 0;
        var ny = parseFloat(note.style.top) || 0;
        var nw = note.offsetWidth;
        var nh = note.offsetHeight;
        var cx = -(nx + nw / 2 - vw / 2);
        var cy = -(ny + nh / 2 - vh / 2);
        panTo(cx, cy, true);
    }

    // --- mouse drag ---
    viewport.addEventListener('mousedown', function(e) {
        if (e.target.closest('.note')) return;
        dragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;
        panStartX = panX;
        panStartY = panY;
        viewport.classList.add('dragging');
        e.preventDefault();
    });

    window.addEventListener('mousemove', function(e) {
        if (!dragging) return;
        panX = panStartX + (e.clientX - dragStartX);
        panY = panStartY + (e.clientY - dragStartY);
        applyPan();
    });

    window.addEventListener('mouseup', function() {
        if (!dragging) return;
        dragging = false;
        viewport.classList.remove('dragging');
    });

    // --- touch drag ---
    viewport.addEventListener('touchstart', function(e) {
        if (e.target.closest('.note') || e.target.closest('.post-form')) return;
        if (e.touches.length !== 1) return;
        dragging = true;
        dragStartX = e.touches[0].clientX;
        dragStartY = e.touches[0].clientY;
        panStartX = panX;
        panStartY = panY;
    }, { passive: true });

    viewport.addEventListener('touchmove', function(e) {
        if (!dragging) return;
        panX = panStartX + (e.touches[0].clientX - dragStartX);
        panY = panStartY + (e.touches[0].clientY - dragStartY);
        applyPan();
        e.preventDefault();
    }, { passive: false });

    viewport.addEventListener('touchend', function() {
        dragging = false;
    });

    // --- toast ---
    function showToast(msg, type) {
        toast.textContent = msg;
        toast.className = 'toast show ' + type;
        if (toastTimeout) clearTimeout(toastTimeout);
        toastTimeout = setTimeout(function() {
            toast.className = 'toast';
        }, 3000);
    }

    // --- placement ---
    function overlaps(x, y, w, h) {
        for (var i = 0; i < placed.length; i++) {
            var p = placed[i];
            if (x < p.x + p.w + PAD && x + w + PAD > p.x &&
                y < p.y + p.h + PAD && y + h + PAD > p.y) {
                return true;
            }
        }
        return false;
    }

    // Reserve center area for the title text
    var titleEl = wall.querySelector('.wall-title');
    if (titleEl) {
        var tw = titleEl.offsetWidth + 40;
        var th = titleEl.offsetHeight + 40;
        var tx = (WALL_W - tw) / 2;
        var ty = (WALL_H - th) / 2;
        placed.push({ x: tx, y: ty, w: tw, h: th });
    }

    function placeNote(note) {
        var w = note.offsetWidth;
        var h = note.offsetHeight;
        var best = null;

        for (var attempt = 0; attempt < 120; attempt++) {
            var x = Math.random() * Math.max(0, WALL_W - w);
            var y = Math.random() * Math.max(0, WALL_H - h);
            if (!overlaps(x, y, w, h)) {
                best = { x: x, y: y };
                break;
            }
        }

        if (!best) {
            var maxY = 0;
            for (var i = 0; i < placed.length; i++) {
                var b = placed[i].y + placed[i].h;
                if (b > maxY) maxY = b;
            }
            best = { x: Math.random() * Math.max(0, WALL_W - w), y: maxY + PAD };
            if (best.y + h > WALL_H) WALL_H = best.y + h + PAD;
            wall.style.height = WALL_H + 'px';
        }

        var rot = ((Math.random() - 0.5) * 8).toFixed(1);
        note.style.left = best.x + 'px';
        note.style.top = best.y + 'px';
        note.style.transform = 'rotate(' + rot + 'deg)';
        note.classList.add('placed');
        placed.push({ x: best.x, y: best.y, w: w, h: h });
    }

    function placeAll() {
        var notes = wall.querySelectorAll('.note:not(.placed)');
        var arr = Array.prototype.slice.call(notes);
        arr.sort(function(a, b) { return b.offsetHeight - a.offsetHeight; });
        for (var i = 0; i < arr.length; i++) {
            placeNote(arr[i]);
        }
    }

    function centerWall() {
        var vw = viewport.clientWidth;
        var vh = viewport.clientHeight;
        panX = -(WALL_W - vw) / 2;
        panY = -(WALL_H - vh) / 2;
        clampPan();
        applyPan();
    }

    if (document.fonts && document.fonts.ready) {
        document.fonts.ready.then(function() { placeAll(); centerWall(); });
    } else {
        placeAll();
        centerWall();
    }

    // --- post note ---
    function createNoteElement(data) {
        var note = document.createElement('div');
        note.className = 'note';
        note.style.background = data.color;
        note.setAttribute('data-id', data.id);

        var msgDiv = document.createElement('div');
        msgDiv.className = 'note-message';
        msgDiv.textContent = data.message;

        var authorDiv = document.createElement('div');
        authorDiv.className = 'note-author';
        authorDiv.textContent = '— ' + data.username;

        note.appendChild(msgDiv);
        note.appendChild(authorDiv);
        return note;
    }

    form.addEventListener('submit', function(e) {
        e.preventDefault();
        var username = form.querySelector('[name=username]').value.trim();
        var message = form.querySelector('[name=message]').value.trim();
        if (!username || !message) return;

        var btn = form.querySelector('button');
        btn.disabled = true;

        fetch('/api/notes', {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            body: 'username=' + encodeURIComponent(username) + '&message=' + encodeURIComponent(message)
        })
        .then(function(res) {
            if (res.status === 201) return res.json();
            if (res.status === 429) throw new Error('Slow down! Wait 30 seconds between posts.');
            var ct = res.headers.get('content-type') || '';
            if (ct.indexOf('application/json') >= 0) {
                return res.json().then(function(data) {
                    throw new Error(data.error || 'Something went wrong');
                });
            }
            throw new Error('Something went wrong');
        })
        .then(function(data) {
            var note = createNoteElement(data);
            wall.appendChild(note);
            placeNote(note);
            scrollToNote(note);
            form.querySelector('[name=message]').value = '';
            showToast('Note posted!', 'success');
        })
        .catch(function(err) {
            showToast(err.message, 'error');
        })
        .finally(function() {
            btn.disabled = false;
        });
    });
})();
"#
}
