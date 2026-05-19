/* html-ppt :: editor.js — Visual WYSIWYG editor mode
 *
 * Activated by pressing E in runtime.js.
 * Provides PowerPoint-like editing experience:
 *   - Click-to-edit text (contenteditable)
 *   - Slide sidebar with thumbnails
 *   - Property panel for quick style tweaks
 *   - Save/download edited HTML
 *
 * Keyboard shortcuts in edit mode:
 *   E / Esc       — exit edit mode
 *   Ctrl+S        — save (download HTML)
 *   Ctrl+Z        — undo
 *   Ctrl+Shift+Z  — redo
 *   ← →           — navigate between slides
 */
(function () {
  'use strict';

  /* ===== State ===== */
  let active = false;
  let currentSlideIdx = 0;
  let selectedEl = null;
  let undoStack = [];
  let redoStack = [];
  let snapshotTimer = null;

  /* ===== DOM refs (set in enter()) ===== */
  let deck, slides, toolbar, sidebar, propPanel, statusBar;

  /* ===== Text element selector ===== */
  const EDITABLE_TAGS = new Set([
    'P', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6',
    'LI', 'SPAN', 'A', 'TD', 'TH', 'FIGCAPTION',
    'DIV', 'LABEL', 'STRONG', 'EM', 'B', 'I', 'TIME'
  ]);

  /* Elements inside these containers should NOT be contenteditable */
  const NO_EDIT_CLASSES = ['term-body', 'notes', 'speaker-notes', 'notes-overlay'];
  const NO_EDIT_TAGS = new Set(['SVG', 'CODE', 'PRE', 'SCRIPT', 'STYLE', 'IMG', 'VIDEO', 'IFRAME']);

  function isEditable(el) {
    if (!el) return false;
    if (NO_EDIT_TAGS.has(el.tagName)) return false;
    if (el.closest) {
      /* Skip editor UI controls (but NOT deck content which is inside editor-ui during edit) */
      const inEditorUI = el.closest('.editor-ui');
      const inDeck = el.closest('.deck');
      if (inEditorUI && !inDeck) return false;
      /* Skip notes (they have their own edit panel) */
      if (el.closest('aside.notes')) return false;
      for (const cls of NO_EDIT_CLASSES) {
        if (el.closest('.' + cls)) return false;
      }
    }
    return EDITABLE_TAGS.has(el.tagName);
  }

  /* ===== Snapshot for undo ===== */
  function takeSnapshot() {
    if (!active) return;
    const html = deck.innerHTML;
    undoStack.push(html);
    if (undoStack.length > 50) undoStack.shift();
    redoStack = [];
  }

  function scheduleSnapshot() {
    clearTimeout(snapshotTimer);
    snapshotTimer = setTimeout(takeSnapshot, 600);
  }

  function doUndo() {
    if (!undoStack.length) return;
    redoStack.push(deck.innerHTML);
    deck.innerHTML = undoStack.pop();
    refreshSlideList();
    /* Re-apply contenteditable */
    slides = Array.from(deck.querySelectorAll('.slide'));
    enableEditingOnCurrentSlide();
  }

  function doRedo() {
    if (!redoStack.length) return;
    undoStack.push(deck.innerHTML);
    deck.innerHTML = redoStack.pop();
    refreshSlideList();
    slides = Array.from(deck.querySelectorAll('.slide'));
    enableEditingOnCurrentSlide();
  }

  /* ===== Enter / Exit ===== */
  function enter() {
    if (active) return;
    deck = document.querySelector('.deck');
    if (!deck) return;
    slides = Array.from(deck.querySelectorAll('.slide'));
    if (!slides.length) return;

    active = true;
    /* Take initial snapshot */
    takeSnapshot();

    /* Build editor UI */
    buildUI();
    document.body.classList.add('editor-active');
    document.documentElement.setAttribute('data-editor', '1');

    /* Show first slide */
    currentSlideIdx = 0;
    showSlide(0);
  }

  function exit() {
    if (!active) return;
    active = false;

    /* Disable all contenteditable */
    deck.querySelectorAll('[contenteditable]').forEach(el => {
      el.removeAttribute('contenteditable');
    });

    /* Remove selection */
    if (selectedEl) {
      selectedEl.classList.remove('editor-selected');
      selectedEl = null;
    }

    /* Restore slide visibility */
    slides.forEach((s, i) => {
      s.style.display = '';
      s.classList.remove('is-active');
      s.classList.remove('is-prev');
      s.style.opacity = '';
      s.style.transform = '';
      s.style.pointerEvents = '';
    });

    /* Remove editor UI (this also removes slideArea which contains the deck) */
    const ui = document.querySelector('.editor-ui');
    if (ui) {
      /* Move deck back to body before removing editor UI */
      const slideArea = ui.querySelector('.ed-slide-area');
      if (slideArea && deck.parentNode === slideArea) {
        document.body.appendChild(deck);
      }
      ui.remove();
    }

    /* Restore deck inline styles added by editor */
    deck.style.transform = '';
    deck.style.width = '';
    deck.style.height = '';

    document.body.classList.remove('editor-active');
    document.documentElement.removeAttribute('data-editor');

    /* Let runtime restore normal state */
    window.dispatchEvent(new CustomEvent('editor-exit'));
  }

  /* ===== Build Editor UI ===== */
  function buildUI() {
    const ui = document.createElement('div');
    ui.className = 'editor-ui';

    /* --- Toolbar --- */
    toolbar = document.createElement('div');
    toolbar.className = 'ed-toolbar';
    toolbar.innerHTML = `
      <div class="ed-toolbar-left">
        <button class="ed-btn ed-btn-icon" data-action="exit" title="退出编辑 (E/Esc)">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M12 4L4 12M4 4l8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
        <span class="ed-toolbar-title">编辑模式</span>
      </div>
      <div class="ed-toolbar-center">
        <button class="ed-btn" data-action="undo" title="撤销 (Ctrl+Z)">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M4 7h6a3 3 0 110 6H7" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><path d="M7 4L4 7l3 3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
          撤销
        </button>
        <button class="ed-btn" data-action="redo" title="重做 (Ctrl+Shift+Z)">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M12 7H6a3 3 0 100 6h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><path d="M9 4l3 3-3 3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
          重做
        </button>
        <span class="ed-divider"></span>
        <button class="ed-btn" data-action="bold" title="加粗">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M4 3h4.5a2.5 2.5 0 010 5H4m0-5v10m4.5-5a2.5 2.5 0 010 5H4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
        <button class="ed-btn" data-action="italic" title="斜体">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M10 3H6m4 10H6m2.5-10l-1 10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
        </button>
        <button class="ed-btn" data-action="underline" title="下划线">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M4 3v5a4 4 0 008 0V3m-8 10h8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
        </button>
        <span class="ed-divider"></span>
        <button class="ed-btn" data-action="align-left" title="左对齐">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M2 3h12M2 6.5h8M2 10h10M2 13.5h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
        </button>
        <button class="ed-btn" data-action="align-center" title="居中">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M2 3h12M4 6.5h8M3 10h10M5 13.5h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
        </button>
        <button class="ed-btn" data-action="align-right" title="右对齐">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M2 3h12M6 6.5h8M4 10h10M8 13.5h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
        </button>
      </div>
      <div class="ed-toolbar-right">
        <button class="ed-btn ed-btn-save" data-action="save" title="保存 (Ctrl+S)">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none"><path d="M3 2h8l3 3v8a1 1 0 01-1 1H3a1 1 0 01-1-1V3a1 1 0 011-1z" stroke="currentColor" stroke-width="1.3"/><path d="M5 2v3h5V2m-2 8v3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/></svg>
          保存 HTML
        </button>
      </div>
    `;

    /* --- Sidebar (slide list) --- */
    sidebar = document.createElement('div');
    sidebar.className = 'ed-sidebar';
    buildSlideList();

    /* --- Property Panel --- */
    propPanel = document.createElement('div');
    propPanel.className = 'ed-props';
    propPanel.innerHTML = `
      <div class="ed-props-title">属性</div>
      <div class="ed-props-empty">点击幻灯片中的文字开始编辑</div>
      <div class="ed-props-content" style="display:none">
        <div class="ed-prop-group">
          <label>字体大小</label>
          <div class="ed-prop-row">
            <input type="range" min="10" max="120" value="16" id="ed-font-size">
            <span id="ed-font-size-val">16px</span>
          </div>
        </div>
        <div class="ed-prop-group">
          <label>文字颜色</label>
          <input type="color" id="ed-color" value="#000000">
        </div>
        <div class="ed-prop-group">
          <label>行高</label>
          <div class="ed-prop-row">
            <input type="range" min="10" max="30" value="16" id="ed-line-height">
            <span id="ed-line-height-val">1.6</span>
          </div>
        </div>
        <div class="ed-prop-group">
          <label>幻灯片标题 (data-title)</label>
          <input type="text" id="ed-slide-title" placeholder="未设置">
        </div>
      </div>
    `;

    /* --- Status Bar --- */
    statusBar = document.createElement('div');
    statusBar.className = 'ed-statusbar';
    statusBar.innerHTML = `
      <span>按 <kbd>E</kbd> / <kbd>Esc</kbd> 退出编辑</span>
      <span>点击文字直接编辑</span>
      <span class="ed-status-right">Ctrl+S 保存 · Ctrl+Z 撤销</span>
    `;

    /* --- Assemble --- */
    ui.appendChild(toolbar);
    ui.appendChild(sidebar);

    const slideArea = document.createElement('div');
    slideArea.className = 'ed-slide-area';
    ui.appendChild(slideArea);

    ui.appendChild(propPanel);
    ui.appendChild(statusBar);
    document.body.appendChild(ui);

    /* Move deck into slide area */
    slideArea.appendChild(deck);

    /* Wire up events */
    wireEvents();

    /* Auto-scale deck to fit slide area */
    function rescaleDeck() {
      if (!active || !deck) return;
      const area = document.querySelector('.ed-slide-area');
      if (!area) return;
      const areaW = area.clientWidth;
      const areaH = area.clientHeight;
      /* Design resolution: 1920x1080 (from base.css) */
      const scale = Math.min(areaW / 1920, areaH / 1080);
      deck.style.transform = `scale(${scale})`;
      deck.style.width = '1920px';
      deck.style.height = '1080px';
    }
    window.addEventListener('resize', rescaleDeck);
    rescaleDeck();
  }

  function buildSlideList() {
    sidebar.innerHTML = '<div class="ed-sidebar-title">幻灯片</div>';
    slides.forEach((s, i) => {
      const title = s.getAttribute('data-title') ||
        (s.querySelector('h1,h2,h3') || {}).textContent || ('Slide ' + (i + 1));
      const item = document.createElement('div');
      item.className = 'ed-slide-item' + (i === currentSlideIdx ? ' active' : '');
      item.setAttribute('data-idx', i);

      /* Mini thumbnail using CSS scale */
      const thumb = document.createElement('div');
      thumb.className = 'ed-thumb';
      const mini = document.createElement('div');
      mini.className = 'ed-mini-slide';
      const clone = s.cloneNode(true);
      clone.classList.add('is-active');
      clone.style.position = 'absolute';
      clone.style.inset = '0';
      clone.style.transform = 'none';
      clone.style.opacity = '1';
      clone.style.pointerEvents = 'none';
      /* Remove notes from thumbnail */
      clone.querySelectorAll('aside.notes, .notes').forEach(n => n.remove());
      mini.appendChild(clone);

      /* Calculate scale to fit thumb (width ~200px) */
      requestAnimationFrame(() => {
        const thumbW = thumb.offsetWidth;
        if (thumbW > 0) {
          mini.style.setProperty('--thumb-scale', (thumbW / 1920).toFixed(4));
        }
      });

      thumb.appendChild(mini);
      item.appendChild(thumb);

      const label = document.createElement('div');
      label.className = 'ed-slide-label';
      label.textContent = (i + 1) + '. ' + title.trim().slice(0, 30);
      item.appendChild(label);

      item.addEventListener('click', () => showSlide(i));
      sidebar.appendChild(item);
    });
  }

  function refreshSlideList() {
    if (!sidebar) return;
    slides = Array.from(deck.querySelectorAll('.slide'));
    buildSlideList();
  }

  /* ===== Show Slide ===== */
  function showSlide(idx) {
    idx = Math.max(0, Math.min(slides.length - 1, idx));
    currentSlideIdx = idx;

    /* Disable editing on previous slide */
    slides.forEach((s, i) => {
      s.style.display = i === idx ? '' : 'none';
      s.classList.toggle('is-active', i === idx);
      s.style.opacity = i === idx ? '1' : '0';
      s.style.transform = 'none';
      s.style.pointerEvents = i === idx ? 'auto' : 'none';
    });

    enableEditingOnCurrentSlide();

    /* Update sidebar */
    sidebar.querySelectorAll('.ed-slide-item').forEach((item, i) => {
      item.classList.toggle('active', i === idx);
    });

    /* Update slide title in props */
    const titleInput = document.getElementById('ed-slide-title');
    if (titleInput) {
      titleInput.value = slides[idx].getAttribute('data-title') || '';
    }

    /* Deselect element */
    deselectElement();
  }

  function enableEditingOnCurrentSlide() {
    /* Remove old contenteditable */
    deck.querySelectorAll('[contenteditable]').forEach(el => {
      el.removeAttribute('contenteditable');
    });

    const slide = slides[currentSlideIdx];
    if (!slide) return;

    /* Simple approach: make all text-containing editable tags contenteditable */
    /* We target leaf-ish elements that directly contain text */
    const candidates = slide.querySelectorAll('p, h1, h2, h3, h4, h5, h6, li, span, a, td, th, figcaption, label, time, strong, em, b, i');
    candidates.forEach(el => {
      /* Skip if inside notes (but NOT skip just because inside editor-ui — deck is inside editor-ui during edit) */
      if (el.closest('aside.notes') || el.closest('.notes')) return;
      /* Skip if inside editor UI controls (sidebar, toolbar, etc.) but allow if inside deck */
      const inEditorUI = el.closest('.editor-ui');
      const inDeck = el.closest('.deck');
      if (inEditorUI && !inDeck) return;
      /* Skip if contains only other editable elements (not direct text) */
      const hasDirectText = Array.from(el.childNodes).some(
        n => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0
      );
      if (hasDirectText) {
        el.setAttribute('contenteditable', 'true');
      }
    });

    /* Also handle divs that act as text containers (like term-def, term-desc) */
    slide.querySelectorAll('div.term-def, div.term-desc, div.term-head, div.text, div.caption').forEach(el => {
      if (el.closest('aside.notes') || el.closest('.notes')) return;
      const inEditorUI = el.closest('.editor-ui');
      const inDeck = el.closest('.deck');
      if (inEditorUI && !inDeck) return;
      const hasDirectText = Array.from(el.childNodes).some(
        n => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0
      );
      if (hasDirectText) {
        el.setAttribute('contenteditable', 'true');
      }
    });
  }

  /* ===== Element Selection ===== */
  function selectElement(el) {
    if (selectedEl === el) return;
    deselectElement();
    selectedEl = el;
    el.classList.add('editor-selected');
    updatePropsPanel();
  }

  function deselectElement() {
    if (selectedEl) {
      selectedEl.classList.remove('editor-selected');
      selectedEl = null;
    }
    const content = document.querySelector('.ed-props-content');
    const empty = document.querySelector('.ed-props-empty');
    if (content) content.style.display = 'none';
    if (empty) empty.style.display = '';
  }

  function updatePropsPanel() {
    if (!selectedEl) return;
    const content = document.querySelector('.ed-props-content');
    const empty = document.querySelector('.ed-props-empty');
    if (content) content.style.display = '';
    if (empty) empty.style.display = 'none';

    const cs = window.getComputedStyle(selectedEl);
    const fontSizeInput = document.getElementById('ed-font-size');
    const fontSizeVal = document.getElementById('ed-font-size-val');
    const colorInput = document.getElementById('ed-color');
    const lineHeightInput = document.getElementById('ed-line-height');
    const lineHeightVal = document.getElementById('ed-line-height-val');

    if (fontSizeInput) {
      const px = parseFloat(cs.fontSize);
      fontSizeInput.value = px;
      if (fontSizeVal) fontSizeVal.textContent = Math.round(px) + 'px';
    }
    if (colorInput) {
      colorInput.value = rgbToHex(cs.color);
    }
    if (lineHeightInput) {
      const lh = parseFloat(cs.lineHeight) / parseFloat(cs.fontSize);
      const val = Math.round(lh * 10) / 10;
      lineHeightInput.value = Math.round(val * 10);
      if (lineHeightVal) lineHeightVal.textContent = val.toFixed(1);
    }
  }

  /* ===== Wire Events ===== */
  function wireEvents() {
    /* Toolbar buttons */
    toolbar.addEventListener('click', (e) => {
      const btn = e.target.closest('[data-action]');
      if (!btn) return;
      const action = btn.getAttribute('data-action');
      switch (action) {
        case 'exit': exit(); break;
        case 'save': saveHTML(); break;
        case 'undo': doUndo(); break;
        case 'redo': doRedo(); break;
        case 'bold': document.execCommand('bold'); break;
        case 'italic': document.execCommand('italic'); break;
        case 'underline': document.execCommand('underline'); break;
        case 'align-left': document.execCommand('justifyLeft'); break;
        case 'align-center': document.execCommand('justifyCenter'); break;
        case 'align-right': document.execCommand('justifyRight'); break;
      }
    });

    /* Click on slide content to select element */
    deck.addEventListener('click', (e) => {
      if (!active) return;
      const el = e.target;
      /* Skip clicks on editor UI controls (buttons, inputs etc.) */
      if (el.closest('.ed-toolbar, .ed-sidebar, .ed-props, .ed-statusbar')) return;

      if (isEditable(el)) {
        selectElement(el);
      } else {
        deselectElement();
      }
    });

    /* Track content changes for undo */
    deck.addEventListener('input', () => {
      scheduleSnapshot();
      /* Update slide title if data-title element was edited */
      refreshSlideList();
    });

    /* Property panel controls */
    document.addEventListener('input', (e) => {
      if (!selectedEl) return;
      const id = e.target.id;
      if (id === 'ed-font-size') {
        selectedEl.style.fontSize = e.target.value + 'px';
        document.getElementById('ed-font-size-val').textContent = e.target.value + 'px';
        scheduleSnapshot();
      } else if (id === 'ed-color') {
        selectedEl.style.color = e.target.value;
        scheduleSnapshot();
      } else if (id === 'ed-line-height') {
        const val = (parseInt(e.target.value, 10) / 10).toFixed(1);
        selectedEl.style.lineHeight = val;
        document.getElementById('ed-line-height-val').textContent = val;
        scheduleSnapshot();
      } else if (id === 'ed-slide-title') {
        slides[currentSlideIdx].setAttribute('data-title', e.target.value);
        /* Update sidebar label */
        const items = sidebar.querySelectorAll('.ed-slide-item');
        if (items[currentSlideIdx]) {
          const label = items[currentSlideIdx].querySelector('.ed-slide-label');
          if (label) label.textContent = (currentSlideIdx + 1) + '. ' + e.target.value.trim().slice(0, 30);
        }
        scheduleSnapshot();
      }
    });

    /* Keyboard shortcuts */
    document.addEventListener('keydown', handleKeyDown);
  }

  function handleKeyDown(e) {
    if (!active) {
      document.removeEventListener('keydown', handleKeyDown);
      return;
    }

    /* Don't intercept when typing in input fields */
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

    /* Exit */
    if (e.key === 'e' || e.key === 'E') {
      if (!e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        exit();
        return;
      }
    }
    if (e.key === 'Escape') {
      /* If editing a contenteditable, defocus first */
      if (document.activeElement && document.activeElement.getAttribute('contenteditable') === 'true') {
        document.activeElement.blur();
        return;
      }
      e.preventDefault();
      exit();
      return;
    }

    /* Save */
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      saveHTML();
      return;
    }

    /* Undo */
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
      e.preventDefault();
      doUndo();
      return;
    }

    /* Redo */
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'z' || e.key === 'Z')) {
      e.preventDefault();
      doRedo();
      return;
    }

    /* Navigate slides */
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      if (!e.ctrlKey && !e.metaKey) {
        if (document.activeElement && document.activeElement.getAttribute('contenteditable') === 'true') return;
        e.preventDefault();
        showSlide(currentSlideIdx + 1);
      }
    }
    if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      if (!e.ctrlKey && !e.metaKey) {
        if (document.activeElement && document.activeElement.getAttribute('contenteditable') === 'true') return;
        e.preventDefault();
        showSlide(currentSlideIdx - 1);
      }
    }
  }

  /* ===== Save HTML ===== */
  function saveHTML() {
    /* Clone the entire document */
    const clonedDoc = document.documentElement.cloneNode(true);

    /* Remove editor artifacts */
    clonedDoc.querySelectorAll('.editor-ui').forEach(el => el.remove());
    clonedDoc.querySelectorAll('[contenteditable]').forEach(el => {
      el.removeAttribute('contenteditable');
    });
    clonedDoc.querySelectorAll('.editor-selected').forEach(el => {
      el.classList.remove('editor-selected');
    });
    clonedDoc.querySelectorAll('[data-editor]').forEach(el => {
      el.removeAttribute('data-editor');
    });
    clonedDoc.body.classList.remove('editor-active');

    /* Move deck back to body level (undo the slide-area move) */
    const slideArea = clonedDoc.querySelector('.ed-slide-area');
    if (slideArea) {
      const deckEl = slideArea.querySelector('.deck');
      if (deckEl) {
        slideArea.parentNode.insertBefore(deckEl, slideArea);
      }
      slideArea.remove();
    }

    /* Restore slide state to normal (remove editor overrides) */
    clonedDoc.querySelectorAll('.slide').forEach(s => {
      s.style.display = '';
      s.style.opacity = '';
      s.style.transform = '';
      s.style.pointerEvents = '';
    });

    /* Serialize */
    const html = '<!DOCTYPE html>\n' + clonedDoc.outerHTML;

    /* Download */
    const blob = new Blob([html], { type: 'text/html;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'index.html';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

    /* Visual feedback */
    showToast('已保存 index.html（下载到本地）');
  }

  /* ===== Toast ===== */
  function showToast(msg) {
    const existing = document.querySelector('.ed-toast');
    if (existing) existing.remove();

    const toast = document.createElement('div');
    toast.className = 'ed-toast';
    toast.textContent = msg;
    document.body.appendChild(toast);

    requestAnimationFrame(() => toast.classList.add('show'));
    setTimeout(() => {
      toast.classList.remove('show');
      setTimeout(() => toast.remove(), 300);
    }, 2000);
  }

  /* ===== Helpers ===== */
  function rgbToHex(rgb) {
    if (!rgb) return '#000000';
    if (rgb.startsWith('#')) return rgb;
    const match = rgb.match(/\d+/g);
    if (!match || match.length < 3) return '#000000';
    return '#' + match.slice(0, 3).map(n => parseInt(n, 10).toString(16).padStart(2, '0')).join('');
  }

  /* ===== Public API ===== */
  window.htmlPptEditor = {
    enter: enter,
    exit: exit,
    isActive: function () { return active; }
  };

})();
