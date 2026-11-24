/* RPlayer v2 · Listening Room — prototype behaviour */

(() => {
  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

  const COVERS = {
    jay: {
      cls: 'cover-jay',
      html: `<span class="cv-ring"></span><span class="cv-title">JAY</span><span class="cv-meta">2000 · 周杰伦</span>`,
    },
    '89757': {
      cls: 'cover-89757',
      html: `<span class="cv-meta">林俊杰 · 2005</span><span class="cv-barcode"></span><span class="cv-number">89757</span>`,
    },
    lastdance: {
      cls: 'cover-lastdance',
      html: `<span class="cv-ball"></span><span class="cv-title">LAST<br>DANCE</span><span class="cv-meta">徐薇 · 2020</span>`,
    },
    city: {
      cls: 'cover-city',
      html: `<span class="cv-sun"></span><span class="cv-title">城市夜航</span><span class="cv-meta">林间电台 · 2024</span>`,
    },
    summer: {
      cls: 'cover-summer',
      html: `<span class="cv-sun"></span><span class="cv-title">夏日回声</span><span class="cv-meta">橘子汽水 · 2025</span>`,
    },
    '2am': {
      cls: 'cover-2am',
      html: `<span class="cv-moon"></span><span class="cv-title">凌晨两点</span><span class="cv-meta">漫游 · 2024</span>`,
    },
    tide: {
      cls: 'cover-tide',
      html: `<span class="cv-wave w1"></span><span class="cv-wave w2"></span><span class="cv-title">潮汐来信</span><span class="cv-meta">苏晚 · 2023</span>`,
    },
    train: {
      cls: 'cover-train',
      html: `<span class="cv-window"></span><span class="cv-title">慢行列车</span><span class="cv-meta">陈屿 · 2022</span>`,
    },
  };

  function renderCover(el) {
    const key = el.dataset.cover;
    const def = COVERS[key];
    if (!def) return;
    el.classList.add('cover');
    [...el.classList].filter((c) => c.startsWith('cover-')).forEach((c) => el.classList.remove(c));
    el.classList.add(def.cls);
    el.innerHTML = def.html;
    el.setAttribute('aria-hidden', 'true');
  }

  $$('[data-cover]')
    .filter((el) => !el.classList.contains('track-row'))
    .forEach(renderCover);

  /* ── screens ──────────────────────────────────────────────── */

  const CRUMBS = {
    library: '本地曲库',
    albums: '专辑',
    artists: '艺术家',
    favorites: '我的收藏',
    recent: '最近播放',
    folders: '音乐目录',
    playlist: '歌单',
    now: '正在播放',
    overlay: '桌面歌词',
  };

  function closeAll() {
    document.body.classList.remove('queue-open');
    $$('.modal-layer').forEach((m) => m.classList.remove('is-open'));
  }

  function showScreen(name, title) {
    if (!name) return;
    closeAll();
    document.body.dataset.screen = name;
    const isOverlay = name === 'overlay';
    $('.desktop-scene').classList.toggle('is-active', isOverlay);
    $$('.screen').forEach((s) => s.classList.toggle('is-active', s.dataset.screen === name));
    if (name === 'playlist') {
      const nameLabel = title || '深夜歌单';
      $('#playlistTitle').textContent = nameLabel;
      $('#crumbCurrent').textContent = nameLabel;
    } else {
      $('#crumbCurrent').textContent = CRUMBS[name] || '本地曲库';
    }

    $$('.nav-item').forEach((n) => {
      const active = name === 'playlist'
        ? n.dataset.go === 'playlist' && n.querySelector('.nav-text')?.textContent?.trim() === (title || '深夜歌单')
        : n.dataset.go === name;
      n.classList.toggle('is-active', active);
      if (active) n.setAttribute('aria-current', 'page');
      else n.removeAttribute('aria-current');
    });
    $$('.tab[data-go]').forEach((t) => {
      const active = t.dataset.go === name;
      t.classList.toggle('is-active', active);
      t.setAttribute('aria-selected', String(active));
    });

    if (name === 'albums') {
      $$('.tab').forEach((t) => t.classList.toggle('is-active', t.textContent.includes('专辑')));
    }
    if (name === 'artists') {
      $$('.tab').forEach((t) => t.classList.toggle('is-active', t.textContent.includes('艺术家')));
    }

    document.documentElement.scrollTop = 0;
    window.scrollTo(0, 0);
  }

  function openModal(name) {
    if (name === 'queue') {
      const willOpen = !document.body.classList.contains('queue-open');
      closeAll();
      document.body.classList.toggle('queue-open', willOpen);
      return;
    }
    const layer = name === 'settings' ? $('#settingsLayer') : name === 'command' ? $('#commandLayer') : null;
    if (!layer) return;
    closeAll();
    layer.classList.add('is-open');
    if (name === 'command') {
      const input = $('#commandInput');
      setTimeout(() => input.focus(), 40);
    }
  }

  /* ── delegation ───────────────────────────────────────────── */

  document.addEventListener('click', (e) => {
    const go = e.target.closest('[data-go]');
    if (go) {
      if (go.dataset.go === 'now') closeAll();
      showScreen(go.dataset.go, go.querySelector?.('.nav-text')?.textContent?.trim());
      return;
    }

    const open = e.target.closest('[data-open]');
    if (open) {
      openModal(open.dataset.open);
      return;
    }

    const close = e.target.closest('[data-close]');
    if (close) {
      if (close.dataset.close === 'queue') document.body.classList.remove('queue-open');
      else $$('.modal-layer').forEach((m) => m.classList.remove('is-open'));
      return;
    }

    const themeChoice = e.target.closest('[data-theme-choice]');
    if (themeChoice) {
      setTheme(themeChoice.dataset.themeChoice);
      $$('.theme-card').forEach((c) => c.classList.toggle('is-active', c === themeChoice));
      return;
    }

    const pick = e.target.closest('[data-pick]');
    if (pick) {
      const row = $(`.track-row[data-cover="${pick.dataset.pick}"]`);
      if (row) setTrack(row, false);
      closeAll();
      toast('已载入到播放器');
      return;
    }

    const playAll = e.target.closest('[data-play-all]');
    if (playAll) {
      toast('从第 1 首开始播放');
      return;
    }

    const heart = e.target.closest('.heart-btn, .row-btn');
    if (heart && (heart.classList.contains('heart-btn') || heart.getAttribute('aria-label')?.includes('收藏'))) {
      const wasLiked = heart.classList.contains('is-liked') || heart.classList.contains('liked');
      heart.classList.toggle('is-liked', !wasLiked);
      heart.classList.toggle('liked', !wasLiked);
      heart.setAttribute('aria-label', wasLiked ? '收藏' : '取消收藏');
      toast(wasLiked ? '已取消收藏' : '已加入收藏');
      return;
    }

    const moreBtn = e.target.closest('.row-btn, [aria-label*="更多"]');
    if (moreBtn) {
      toast('更多操作');
      return;
    }

    const album = e.target.closest('[data-open-album]');
    if (album) {
      const name = album.querySelector('.album-copy b')?.textContent || '专辑';
      toast(`打开专辑 · ${name}`);
      return;
    }

    const track = e.target.closest('.track-row');
    if (track) {
      setTrack(track, true);
      return;
    }
  });

  $('#themeToggle')?.addEventListener('click', () => {
    setTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark');
    $$('.theme-card').forEach((c) => c.classList.toggle('is-active', c.dataset.themeChoice === document.documentElement.dataset.theme));
  });

  function setTheme(theme) {
    document.documentElement.dataset.theme = theme;
    try { localStorage.setItem('rplayer-theme', theme); } catch (_) {}
  }

  $('#playToggle')?.addEventListener('click', () => {
    document.body.classList.toggle('is-paused');
  });

  const EQ_ROWS = $$('.track-row .eq');
  function setTrack(row, play) {
    const rows = $$('.track-row');
    rows.forEach((r) => {
      r.classList.toggle('is-playing', r === row);
      r.classList.toggle('is-paused', false);
      if (r === row) r.setAttribute('aria-current', 'true');
      else r.removeAttribute('aria-current');
    });
    document.body.classList.remove('is-paused');

    const title = row.dataset.title;
    const artist = row.dataset.artist;
    const album = row.dataset.album;
    const cover = row.dataset.cover;
    const duration = row.dataset.duration || '3:48';

    $('#railTitle').textContent = title;
    $('#railArtist').innerHTML = `${artist} <span>·</span> ${album}`;
    $('#railCover').dataset.cover = cover;
    renderCover($('#railCover'));
    $('#railCover').dataset.go = 'now';

    $('#playerTitle').textContent = title;
    $('#playerArtist').textContent = `${artist} · ${album}`;
    const pc = $('.player-cover');
    pc.dataset.cover = cover;
    renderCover(pc);
    $('#playerTotal').textContent = duration;
    $('#railTotal').textContent = duration;

    $('#nowTitle').textContent = title;
    $('#nowArtist').innerHTML = `${artist} <span>·</span> ${album}`;
    const nc = $('.now-cover');
    nc.dataset.cover = cover;
    renderCover(nc);

    const index = rows.indexOf(row) + 1;
    $('.now-position').textContent = `${String(index).padStart(2, '0')} / 08`;
    const dc = $('.drawer-cover');
    if (dc) { dc.dataset.cover = cover; renderCover(dc); }
    $('.drawer-now-copy b').textContent = title;
    $('.drawer-now-copy small').textContent = `${artist} · ${album}`;
    if (play) toast(`正在播放 · ${title}`);
  }

  /* ── command palette ──────────────────────────────────────── */

  const commandInput = $('#commandInput');
  commandInput?.addEventListener('input', () => {
    const q = commandInput.value.trim().toLowerCase();
    $$('.command-row').forEach((row) => {
      row.style.display = !q || row.textContent.toLowerCase().includes(q) ? '' : 'none';
    });
    const first = $$('.command-row').find((r) => r.style.display !== 'none');
    $$('.command-row').forEach((r) => r.classList.toggle('is-selected', r === first));
  });

  /* ── toast ────────────────────────────────────────────────── */

  let toastTimer;
  function toast(message) {
    const el = $('#toast');
    el.querySelector('span').textContent = message;
    el.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.remove('show'), 2300);
  }

  /* ── keyboard ─────────────────────────────────────────────── */

  document.addEventListener('keydown', (e) => {
    const typing = ['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName);
    const commandOpen = $('#commandLayer').classList.contains('is-open');

    if (commandOpen && ['ArrowDown', 'ArrowUp', 'Enter'].includes(e.key)) {
      const visible = $$('.command-row').filter((r) => r.style.display !== 'none');
      if (!visible.length) return;
      let index = visible.findIndex((r) => r.classList.contains('is-selected'));
      if (index < 0) index = 0;
      if (e.key === 'ArrowDown') index = (index + 1) % visible.length;
      if (e.key === 'ArrowUp') index = (index - 1 + visible.length) % visible.length;
      if (e.key === 'Enter') {
        e.preventDefault();
        visible[index]?.click();
        return;
      }
      e.preventDefault();
      visible.forEach((r, i) => r.classList.toggle('is-selected', i === index));
      visible[index].scrollIntoView({ block: 'nearest' });
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      openModal('command');
      return;
    }
    if ((e.ctrlKey || e.metaKey) && ['1', '2', '3'].includes(e.key)) {
      e.preventDefault();
      showScreen({ '1': 'library', '2': 'albums', '3': 'now' }[e.key]);
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      openModal('settings');
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      showScreen('now');
      return;
    }
    if (e.key === 'Escape') {
      closeAll();
      return;
    }
    if (typing) return;

    const focusedTrack = document.activeElement?.closest?.('.track-row');
    if (focusedTrack && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      setTrack(focusedTrack, true);
      return;
    }
    if (e.key === ' ' && !e.target.closest('button')) {
      e.preventDefault();
      document.body.classList.toggle('is-paused');
    }
    if (e.key.toLowerCase() === 'f') {
      e.preventDefault();
      $('#railHeart')?.classList.toggle('is-liked');
      toast('收藏状态已更新');
    }
    if (e.key.toLowerCase() === 'q') openModal('queue');
  });

  $('#searchTrigger')?.addEventListener('click', () => openModal('command'));

  /* ── URL state for screenshots ────────────────────────────── */

  const params = new URLSearchParams(location.search);
  const theme = params.get('theme');
  if (theme === 'dark' || theme === 'light') setTheme(theme);
  else {
    try {
      const saved = localStorage.getItem('rplayer-theme');
      if (saved) setTheme(saved);
    } catch (_) {}
  }

  const screen = params.get('screen');
  if (screen) showScreen(screen);
  const modal = params.get('modal');
  if (modal) setTimeout(() => openModal(modal), 30);
})();
