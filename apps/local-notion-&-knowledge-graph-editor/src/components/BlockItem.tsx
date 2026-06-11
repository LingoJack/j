import React, { useState, useEffect, useRef, useImperativeHandle, forwardRef, useCallback } from "react";
import { Block, BlockType, CanvasPath, Document } from "../types";
import { Tldraw } from "tldraw";
import "tldraw/tldraw.css";
import getCaretCoordinates from "textarea-caret";

const ContentEditableBlock = forwardRef<HTMLDivElement, {
  html: string;
  onChange: (html: string, text: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onBlur?: (e: React.FocusEvent<HTMLDivElement>) => void;
  placeholder?: string;
  className?: string;
  tagName?: 'div' | 'h1' | 'h2' | 'h3';
}>(({ html, onChange, onKeyDown, onBlur, placeholder, className, tagName = 'div' }, ref) => {
  const localRef = useRef<HTMLDivElement>(null);
  
  useImperativeHandle(ref, () => localRef.current!);

  useEffect(() => {
    if (localRef.current && localRef.current.innerHTML !== html) {
      localRef.current.innerHTML = html;
    }
  }, [html]);

  const handleInput = (e: React.FormEvent<HTMLDivElement>) => {
    onChange(e.currentTarget.innerHTML, e.currentTarget.innerText);
  };

  const Tag = tagName as any;

  return (
    <Tag
      ref={localRef}
      contentEditable
      suppressContentEditableWarning
      onInput={handleInput}
      onKeyDown={onKeyDown}
      onBlur={onBlur}
      className={`outline-none break-words whitespace-pre-wrap empty:before:content-[attr(data-placeholder)] empty:before:text-slate-350 dark:empty:before:text-slate-650 ${className}`}
      data-placeholder={placeholder}
    />
  );
});
import {
  Code,
  Table as TableIcon,
  Palette,
  Image as ImageIcon,
  MessageSquare,
  ChevronDown,
  ChevronRight,
  Trash2,
  Settings,
  CornerDownRight,
  Play,
  Edit2,
  Check,
  Plus,
  ArrowUpDown,
  FileCode,
  Sparkles,
  Eye,
  Sun,
  Moon,
  RefreshCw,
  FileText,
} from "lucide-react";

function HeadingIcon(props: { className?: string }) {
  return <span className={`text-[10px] font-black ${props.className}`}>H</span>;
}

const SLASH_COMMANDS = [
  { type: "text", label: "文本", icon: MessageSquare },
  { type: "heading-1", label: "标题1", icon: HeadingIcon },
  { type: "heading-2", label: "标题2", icon: HeadingIcon },
  { type: "toggle", label: "清单", icon: ChevronRight },
  { type: "code", label: "代码块", icon: Code },
  { type: "whiteboard", label: "画板", icon: Edit2 },
];

interface BlockItemProps {
  key?: string | number;
  block: Block;
  documents: Document[];
  onUpdateBlock: (updatedFields: Partial<Block>) => void;
  onDeleteBlock: (mergeContent?: string) => void;
  onNavigateToDoc: (docId: string) => void;
  onInsertBlockBelow: (type: BlockType) => void;
  autoFocus?: boolean;
}

export default function BlockItem({
  block,
  documents,
  onUpdateBlock,
  onDeleteBlock,
  onNavigateToDoc,
  onInsertBlockBelow,
  autoFocus,
}: BlockItemProps) {
  const [showSlashMenu, setShowSlashMenu] = useState(false);
  const [slashMenuIndex, setSlashMenuIndex] = useState(0);
  const [slashMenuCoords, setSlashMenuCoords] = useState<{
    top: number;
    left: number;
  } | null>(null);
  const [rawText, setRawText] = useState(block.content);

  // Advanced States for Upgraded HTML Sandbox
  const [sandboxTab, setSandboxTab] = useState<
    "preview" | "html" | "css" | "js" | "split"
  >("split");
  const [splitCodingTab, setSplitCodingTab] = useState<"html" | "css" | "js">(
    "html",
  );
  const [sandboxHtml, setSandboxHtml] = useState(block.content || "");
  const [sandboxCss, setSandboxCss] = useState(block.properties?.cssCode || "");
  const [sandboxJs, setSandboxJs] = useState(block.properties?.jsCode || "");
  const [sandboxTheme, setSandboxTheme] = useState<"light" | "dark">(
    block.properties?.sandboxTheme || "light",
  );
  const [sandboxDebouncedSrcDoc, setSandboxDebouncedSrcDoc] = useState("");
  const [runIndicator, setRunIndicator] = useState(0); // For forcing reload

  // Preset templates for quick load list
  const SANDBOX_PRESETS = [
    {
      name: "磨砂玻璃卡片",
      html: `<div class="max-w-sm mx-auto p-8 rounded-2xl bg-white/20 dark:bg-white/10 backdrop-blur-md border border-white/20 shadow-xl flex flex-col items-center justify-center text-center transition-all duration-300 hover:shadow-2xl">
  <div class="w-14 h-14 rounded-full bg-gradient-to-tr from-emerald-400 to-indigo-500 flex items-center justify-center text-white text-2xl shadow-lg mb-4 animate-bounce">
    ✨
  </div>
  <h3 class="text-lg font-extrabold text-slate-800 dark:text-white mb-1">交互玻璃计算器</h3>
  <p class="text-xs text-slate-500 dark:text-slate-300 mb-6 leading-relaxed">Tailwind CDN 及 FontAwesome 已经预加载。体验纯粹的前端快速原型设计！</p>
  
  <!-- Counter Widget -->
  <div class="flex items-center gap-6 bg-slate-900/5 dark:bg-black/20 px-6 py-2.5 rounded-full mb-6">
    <button id="btn-dec" class="w-8 h-8 rounded-full bg-white dark:bg-slate-800 text-slate-800 dark:text-white shadow hover:bg-slate-50 active:scale-90 flex items-center justify-center font-bold font-mono transition-transform">-</button>
    <span id="counter" class="text-2xl font-black text-indigo-600 dark:text-indigo-400 font-mono w-12 text-center">0</span>
    <button id="btn-inc" class="w-8 h-8 rounded-full bg-white dark:bg-slate-800 text-slate-800 dark:text-white shadow hover:bg-slate-50 active:scale-90 flex items-center justify-center font-bold font-mono transition-transform">+</button>
  </div>

  <span class="text-[10px] text-slate-400 dark:text-slate-500 font-mono">Reacting completely local & instant</span>
</div>`,
      css: `/* 你可以在此附加任何自定义的 CSS 属性。Tailwind 类已经开箱原生可用！ */
body {
  background: linear-gradient(135deg, #f5f7fa 0%, #e4e8f0 100%);
}
body.dark {
  background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%);
}`,
      js: `const counterEl = document.getElementById('counter');
let count = 0;

document.getElementById('btn-inc').addEventListener('click', () => {
  count++;
  counterEl.textContent = count;
  counterEl.classList.add('scale-110');
  setTimeout(() => counterEl.classList.remove('scale-110'), 150);
});

document.getElementById('btn-dec').addEventListener('click', () => {
  count--;
  counterEl.textContent = count;
  counterEl.classList.add('scale-90');
  setTimeout(() => counterEl.classList.remove('scale-90'), 150);
});`,
    },
    {
      name: "物理粒子圆环",
      html: `<div class="flex flex-col items-center justify-center gap-4 text-center">
  <canvas id="sand-canvas" class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-[#0b0f19] shadow-2xl"></canvas>
  <div class="flex items-center gap-3">
    <button id="btn-add-p" class="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-semibold shadow-md active:scale-95 transition-all">追加 20 颗粒子</button>
    <button id="btn-clear-p" class="px-3 py-1.5 bg-slate-200 hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-lg text-xs font-medium active:scale-95 transition-all">重设画布</button>
  </div>
</div>`,
      css: `#sand-canvas {
  width: 100%;
  max-width: 480px;
  height: 200px;
}`,
      js: `const canvas = document.getElementById('sand-canvas');
const ctx = canvas.getContext('2d');
canvas.width = canvas.offsetWidth;
canvas.height = canvas.offsetHeight;

let particles = [];

function createParticle(x, y) {
  return {
    x: x || canvas.width / 2,
    y: y || canvas.height / 2,
    vx: (Math.random() - 0.5) * 4,
    vy: (Math.random() - 0.5) * 4,
    size: Math.random() * 3 + 1.5,
    color: \`hsl(\${Math.random() * 360}, 90%, 65%)\`,
    life: 1.0,
    decay: Math.random() * 0.01 + 0.005
  };
}

// Seed initial particles
for(let i = 0; i < 50; i++) particles.push(createParticle());

document.getElementById('btn-add-p').addEventListener('click', () => {
  for(let i = 0; i < 20; i++) particles.push(createParticle());
});

document.getElementById('btn-clear-p').addEventListener('click', () => {
  particles = [];
});

function draw() {
  ctx.fillStyle = 'rgba(11, 15, 25, 0.2)'; // Tail effect
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  particles.forEach((p, idx) => {
    p.x += p.vx;
    p.y += p.vy;
    p.life -= p.decay;

    // Bounce bounds
    if(p.x < 0 || p.x > canvas.width) p.vx *= -1;
    if(p.y < 0 || p.y > canvas.height) p.vy *= -1;

    ctx.beginPath();
    ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
    ctx.fillStyle = p.color;
    ctx.shadowBlur = 6;
    ctx.shadowColor = p.color;
    ctx.fill();
    ctx.shadowBlur = 0;

    if(p.life <= 0) {
      particles[idx] = createParticle(); // Respawn
    }
  });

  requestAnimationFrame(draw);
}
draw();`,
    },
    {
      name: "新丑撞色事件板",
      html: `<div class="max-w-xs mx-auto p-6 bg-[#ffe4e6] border-4 border-slate-950 rounded-none shadow-[6px_6px_0px_0px_rgba(0,0,0,1)] flex flex-col gap-3">
  <div class="flex justify-between items-center">
    <span class="text-[9.5px] font-mono font-black border-2 border-slate-950 px-2 py-0.5 bg-yellow-300 text-slate-950">NEO-BRUTALISM</span>
    <span class="text-xs font-bold text-slate-900">⚡ ACTIVE</span>
  </div>
  <h2 class="text-xl font-black text-slate-950 tracking-tight">像素微件反应堆</h2>
  <p class="text-xs text-slate-705 leading-normal font-medium">拒绝圆角！采用纯粹的黑度硬核边框和平面色彩块。让纯端排版焕发出别具格调的设计张力。</p>
  
  <div id="reactor-box" class="p-3 bg-white border-2 border-slate-950 text-xs font-mono font-semibold text-slate-900 text-center transition-all">
    状态：等待指令载入
  </div>
  
  <button id="trigger-react" class="cursor-pointer text-xs font-black border-2 border-slate-950 py-2 bg-indigo-400 hover:bg-slate-950 hover:text-white transition-all shadow-[3px_3px_0px_0px_rgba(0,0,0,1)] active:translate-x-0.5 active:translate-y-0.5 active:shadow-[1px_1px_0px_0px_rgba(0,0,0,1)]">试一下触发点击</button>
</div>`,
      css: ``,
      js: `const logTags = [
  "⚡ 核反应堆稳定",
  "🧩 CSS 样式层叠正常",
  "🔮 沙盒内原子扩散完毕",
  "🔋 电池系统充电 98%",
  "✨ 离线数据同步完美"
];
const box = document.getElementById('reactor-box');
document.getElementById('trigger-react').addEventListener('click', () => {
  const chosen = logTags[Math.floor(Math.random() * logTags.length)];
  box.textContent = chosen;
  box.style.backgroundColor = 'hsl(' + (Math.random() * 360) + ', 85%, 90%)';
});`,
    },
  ];

  // Load block content when initialized to prevent leaks
  useEffect(() => {
    if (block.type === "html-render") {
      setSandboxHtml(block.content || "");
      setSandboxCss(block.properties?.cssCode || "");
      setSandboxJs(block.properties?.jsCode || "");
      setSandboxTheme(block.properties?.sandboxTheme || "light");
    }
  }, [block.id, block.type]);

  // Real-time sandboxed debouncer updates (so typing doesn't compile on every keystroke)
  useEffect(() => {
    if (block.type !== "html-render") return;
    const timer = setTimeout(() => {
      setSandboxDebouncedSrcDoc(
        compileSandboxSrcDoc(sandboxHtml, sandboxCss, sandboxJs, sandboxTheme),
      );
    }, 550); // 550ms debounce
    return () => clearTimeout(timer);
  }, [
    sandboxHtml,
    sandboxCss,
    sandboxJs,
    sandboxTheme,
    runIndicator,
    block.type,
  ]);

  // Auto-saves fields to document state when they change
  const handleSandboxChange = (
    htmlVal: string,
    cssVal: string,
    jsVal: string,
    themeVal: "light" | "dark",
  ) => {
    setSandboxHtml(htmlVal);
    setSandboxCss(cssVal);
    setSandboxJs(jsVal);
    setSandboxTheme(themeVal);
    onUpdateBlock({
      content: htmlVal,
      properties: {
        ...block.properties,
        cssCode: cssVal,
        jsCode: jsVal,
        sandboxTheme: themeVal,
      },
    });
  };

  const loadPresetIntoSandbox = (preset: {
    html: string;
    css: string;
    js: string;
  }) => {
    handleSandboxChange(preset.html, preset.css, preset.js, sandboxTheme);
    setRunIndicator((prev) => prev + 1); // trigger refresh
  };

  // Canvas Drawing States
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [brushColor, setBrushColor] = useState("#4f46e5");
  const [brushWidth, setBrushWidth] = useState(4);
  const [isDrawing, setIsDrawing] = useState(false);
  const [paths, setPaths] = useState<CanvasPath[]>(
    block.properties?.drawingPaths || [],
  );

  const elementRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (autoFocus && elementRef.current) {
      elementRef.current.focus();
      if (
        elementRef.current instanceof HTMLTextAreaElement ||
        elementRef.current instanceof HTMLInputElement
      ) {
        const len = elementRef.current.value.length;
        elementRef.current.setSelectionRange(len, len);
      } else if (elementRef.current.isContentEditable) {
        const range = document.createRange();
        range.selectNodeContents(elementRef.current);
        range.collapse(false); // move to end
        const sel = window.getSelection();
        sel?.removeAllRanges();
        sel?.addRange(range);
      }
    }
  }, [autoFocus]);

  useEffect(() => {
    // Only used for textarea types like callout, code, etc. if needed later
    // but not for contentEditable
  }, [rawText, block.type]);

  const handleKeyDown = (
    e: React.KeyboardEvent<HTMLElement>,
  ) => {
    if (showSlashMenu) {
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashMenuIndex((prev) =>
          prev > 0 ? prev - 1 : SLASH_COMMANDS.length - 1,
        );
        return;
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashMenuIndex((prev) =>
          prev < SLASH_COMMANDS.length - 1 ? prev + 1 : 0,
        );
        return;
      } else if (e.key === "Enter") {
        e.preventDefault();
        executeSlashCommand(SLASH_COMMANDS[slashMenuIndex].type as BlockType);
        return;
      } else if (e.key === "Escape") {
        setShowSlashMenu(false);
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onInsertBlockBelow("text");
    } else if (e.key === "Backspace") {
      const el = e.currentTarget;
      if (
        (el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement) &&
        el.selectionStart === 0 &&
        el.selectionEnd === 0
      ) {
        e.preventDefault();
        onDeleteBlock(rawText);
      } else if (el.isContentEditable) {
        const sel = window.getSelection();
        if (sel && sel.rangeCount > 0) {
          const range = sel.getRangeAt(0);
          const preCaretRange = range.cloneRange();
          preCaretRange.selectNodeContents(el);
          preCaretRange.setEnd(range.startContainer, range.startOffset);
          if (preCaretRange.toString().length === 0) {
             e.preventDefault();
             onDeleteBlock(el.innerHTML);
          }
        }
      }
    }
  };

  useEffect(() => {
    setRawText(block.content);
  }, [block.content]);

  const handleBlur = () => {
    // Light Markdown auto-formatting on blur (so cursor doesn't jump during typing)
    let formatted = rawText;
    
    // **bold** to <b>bold</b>
    formatted = formatted.replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>');
    // `code` to <code>code</code>
    formatted = formatted.replace(/`([^`]+)`/g, '<code class="px-1 py-0.5 mx-0.5 rounded bg-slate-100 dark:bg-white/10 text-pink-500 dark:text-pink-400 font-mono text-[13px]">$1</code>');
    // [[Wiki]] to link
    formatted = formatted.replace(/\[\[([^\]]+)\]\]/g, (match, titleStr) => {
      const title = titleStr.trim();
      const matchedDoc = documents.find(
        (d) => d.title.toLowerCase() === title.toLowerCase()
      );
      if (matchedDoc) {
        return `<a href="#" data-doc-id="${matchedDoc.id}" class="wiki-link px-1.5 py-0.5 mx-0.5 rounded bg-indigo-50 hover:bg-indigo-100 dark:bg-indigo-950/50 dark:hover:bg-indigo-900 border-b border-indigo-500 font-semibold text-indigo-700 dark:text-indigo-400 cursor-pointer text-xs inline-flex items-center gap-1 transition-colors"><span>${title}</span></a>`;
      }
      return match;
    });

    if (formatted !== rawText) {
      setRawText(formatted);
      onUpdateBlock({ content: formatted });
    }
  };

  // Handle local text editing + Slash `/` command detector
  const handleTextChange = (val: string, element?: HTMLElement, plainText?: string) => {
    setRawText(val);
    onUpdateBlock({ content: val });

    const checkText = plainText !== undefined ? plainText : val;
    // Show command menu if text ends with /
    if (checkText.replace(/\n$/, "").endsWith("/")) {
      setShowSlashMenu(true);
      setSlashMenuIndex(0);
      if (element) {
        if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {
          const coords = getCaretCoordinates(
            element,
            element.selectionEnd || 0,
          );
          setSlashMenuCoords({
            top: coords.top + 24,
            left: Math.min(coords.left, 400),
          });
        } else {
          const sel = window.getSelection();
          if (sel && sel.rangeCount > 0) {
            const range = sel.getRangeAt(0);
            const rect = range.getBoundingClientRect();
            const parentRect = element.getBoundingClientRect();
            setSlashMenuCoords({
              top: rect.bottom - parentRect.top + 24,
              left: Math.min(rect.left - parentRect.left, 400),
            });
          }
        }
      }
    } else {
      setShowSlashMenu(false);
    }
  };

  const executeSlashCommand = (type: BlockType) => {
    // Remove the trailing slash from content
    const sanitized = rawText.replace(/\/((?:\s*<[^>]+>)*\s*)$/, "$1");
    setRawText(sanitized);

    // Convert current block type or insert below
    if (sanitized === "" || sanitized.replace(/<[^>]*>/g, '').trim() === "") {
      onUpdateBlock({
        type,
        content: "",
        properties: getDefaultProperties(type),
      });
    } else {
      // update text block to remove its slash in parent component too
      onUpdateBlock({ content: sanitized });
      onInsertBlockBelow(type);
    }
    setShowSlashMenu(false);
  };

  const getDefaultProperties = (type: BlockType) => {
    switch (type) {
      case "table":
        return {
          tableData: [
            ["标题 A", "标题 B", "标题 C"],
            ["数据 1", "数据 2", "数据 3"],
          ],
        };
      case "callout":
        return { emoji: "" };
      case "image":
        return { caption: "示例插图" };
      case "canvas":
        return { drawingPaths: [] };
      case "html-render":
        return {
          cssCode: "h1 { color: #6366f1; }",
          jsCode: 'console.log("Demo loaded!");',
        };
      case "toggle":
        return { isOpen: true };
      default:
        return {};
    }
  };

  // Parsing [[Wiki Links]] in regular markdown-like text blocks
  const renderFormattedText = () => {
    const text = rawText;
    if (!text)
      return (
        <span className="text-slate-300 dark:text-slate-600">
          输入回车另起一行，或输入 / 快速唤出组件...
        </span>
      );

    const regex = /(\[\[[^\]]+\]\]|\*\*[^*]+\*\*|`[^`]+`)/g;
    const parts = text.split(regex);

    return parts.map((part, index) => {
      if (part.startsWith("[[") && part.endsWith("]]")) {
        const titleStr = part.slice(2, -2).trim();
        // Match with known docs
        const matchedDoc = documents.find(
          (d) => d.title.toLowerCase() === titleStr.toLowerCase(),
        );

        if (matchedDoc) {
          return (
            <span
              key={index}
              onClick={() => onNavigateToDoc(matchedDoc.id)}
              className="px-1.5 py-0.5 mx-0.5 rounded bg-indigo-50 hover:bg-indigo-100 dark:bg-indigo-950/50 dark:hover:bg-indigo-900 border-b border-indigo-500 font-semibold text-indigo-700 dark:text-indigo-400 cursor-pointer text-xs inline-flex items-center gap-1 transition-colors"
              title={`点击跳转至：${matchedDoc.title}`}
            >
              <FileText className="w-3 h-3 text-indigo-500 shrink-0" />
              <span>{matchedDoc.title}</span>
            </span>
          );
        } else {
          return (
            <span
              key={index}
              className="px-1.5 py-0.5 mx-0.5 rounded bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border-b border-amber-300 dark:border-amber-800 text-xs inline-flex items-center gap-1"
              title="此文档未在本地创建，暂时无法点击跳转。"
            >
              <FileText className="w-3 h-3 text-amber-500 shrink-0" />
              <span>{titleStr} (未创建)</span>
            </span>
          );
        }
      } else if (part.startsWith("**") && part.endsWith("**")) {
        return (
          <strong
            key={index}
            className="font-bold text-slate-900 dark:text-slate-100"
          >
            {part.slice(2, -2)}
          </strong>
        );
      } else if (part.startsWith("`") && part.endsWith("`")) {
        return (
          <code
            key={index}
            className="px-1 py-0.5 mx-0.5 rounded bg-slate-100 dark:bg-white/10 text-pink-500 dark:text-pink-400 font-mono text-[13px]"
          >
            {part.slice(1, -1)}
          </code>
        );
      }
      return (
        <span key={index} className="whitespace-pre-wrap">
          {part}
        </span>
      );
    });
  };

  // --- DRAWING CANVAS LOGIC ---
  useEffect(() => {
    if (block.type !== "canvas" || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Clear and redraw vector lines
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    paths.forEach((path) => {
      if (path.points.length === 0) return;
      ctx.beginPath();
      ctx.strokeStyle = path.color;
      ctx.lineWidth = path.width;

      const start = path.points[0];
      ctx.moveTo(start.x, start.y);
      for (let i = 1; i < path.points.length; i++) {
        ctx.lineTo(path.points[i].x, path.points[i].y);
      }
      ctx.stroke();
    });
  }, [paths, block.type]);

  const startCanvasDrawing = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setIsDrawing(true);
    const newPath: CanvasPath = {
      points: [{ x, y }],
      color: brushColor,
      width: brushWidth,
    };

    const updatedPaths = [...paths, newPath];
    setPaths(updatedPaths);
    onUpdateBlock({
      properties: { ...block.properties, drawingPaths: updatedPaths },
    });
  };

  const drawOnCanvas = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!isDrawing) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const updatedPaths = [...paths];
    const lastPath = updatedPaths[updatedPaths.length - 1];
    if (lastPath) {
      lastPath.points.push({ x, y });
      setPaths(updatedPaths);
    }
  };

  const endCanvasDrawing = () => {
    if (!isDrawing) return;
    setIsDrawing(false);
    onUpdateBlock({ properties: { ...block.properties, drawingPaths: paths } });
  };

  const clearCanvasAll = () => {
    setPaths([]);
    onUpdateBlock({ properties: { ...block.properties, drawingPaths: [] } });
  };

  // --- HTML SANDBOX SANDBOXED IFRAME COMPILATION ---
  const compileSandboxSrcDoc = (
    htmlVal = sandboxHtml,
    cssVal = sandboxCss,
    jsVal = sandboxJs,
    themeVal = sandboxTheme,
  ) => {
    return `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <script src="https://cdn.tailwindcss.com"></script>
  <script>
    tailwind.config = {
      darkMode: 'class',
      theme: {
        extend: {
          colors: {
            brand: '#6366f1',
          }
        }
      }
    }
  </script>
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
  <style>
    body { 
      margin: 0; 
      padding: 16px; 
      font-family: system-ui, -apple-system, sans-serif; 
      background: transparent; 
      color: #334155;
      min-height: 100vh;
      transition: background-color 0.2s, color 0.2s;
    }
    body.dark {
      background-color: #0f172a;
      color: #f1f5f9;
    }
    /* Simple Custom Scrollbars */
    ::-webkit-scrollbar {
      width: 6px;
      height: 6px;
    }
    ::-webkit-scrollbar-track {
      background: transparent;
    }
    ::-webkit-scrollbar-thumb {
      background: rgba(156, 163, 175, 0.3);
      border-radius: 9999px;
    }
    ${cssVal}
  </style>
</head>
<body class="${themeVal === "dark" ? "dark" : ""}">
  ${htmlVal}
  <script>
    try {
      ${jsVal}
    } catch(err) {
      document.body.insertAdjacentHTML('beforeend', '<div style="color:#ef4444; margin-top:12px; padding:10px; border-radius:8px; border: 1px solid rgba(239, 68, 68, 0.2); background: rgba(239, 68, 68, 0.05); font-family: monospace; font-size:12px;">⚠️ JS Error: ' + err.message + '</div>');
    }
  </script>
</body>
</html>`;
  };

  // --- TABLE EDITING HANDLERS ---
  const handleTableCellEdit = (rowIdx: number, colIdx: number, val: string) => {
    const tableData = block.properties?.tableData
      ? [...block.properties.tableData]
      : [[]];
    tableData[rowIdx] = [...tableData[rowIdx]];
    tableData[rowIdx][colIdx] = val;
    onUpdateBlock({ properties: { ...block.properties, tableData } });
  };

  const addTableColumn = () => {
    const tableData = block.properties?.tableData
      ? [...block.properties.tableData]
      : [["", ""]];
    const updated = tableData.map((row) => [...row, ""]);
    onUpdateBlock({ properties: { ...block.properties, tableData: updated } });
  };

  const removeTableColumn = () => {
    const tableData = block.properties?.tableData
      ? [...block.properties.tableData]
      : [["", ""]];
    if (tableData[0].length <= 1) return;
    const updated = tableData.map((row) => row.slice(0, -1));
    onUpdateBlock({ properties: { ...block.properties, tableData: updated } });
  };

  const addTableRow = () => {
    const tableData = block.properties?.tableData
      ? [...block.properties.tableData]
      : [[""]];
    const numCols = tableData[0].length;
    const newRow = Array(numCols).fill("");
    onUpdateBlock({
      properties: { ...block.properties, tableData: [...tableData, newRow] },
    });
  };

  const removeTableRow = () => {
    const tableData = block.properties?.tableData
      ? [...block.properties.tableData]
      : [[""]];
    if (tableData.length <= 1) return;
    onUpdateBlock({
      properties: { ...block.properties, tableData: tableData.slice(0, -1) },
    });
  };

  // Drag and drop base64 image extractor
  const handleImageDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (!file || !file.type.startsWith("image/")) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const base64 = event.target?.result as string;
      onUpdateBlock({
        content: base64,
        properties: { ...block.properties, imageType: "base64" },
      });
    };
    reader.readAsDataURL(file);
  };

  return (
    <div
      className="group/block relative flex items-start gap-1 py-0.5"
      id={`block-row-${block.id}`}
    >
      {/* 1. Left controls (Plus, Delete - Top Aligned for great UX) */}
      <div className="absolute left-[-32px] md:left-[-40px] top-1.5 flex items-center gap-1 opacity-0 group-hover/block:opacity-100 transition-opacity duration-150 z-10">
        <button
          onClick={() => onInsertBlockBelow("text")}
          className="cursor-pointer text-slate-300 hover:text-slate-600 dark:text-slate-600 dark:hover:text-slate-300 p-0.5 rounded transition-colors"
          title="在此行下方添加"
        >
          <Plus className="w-4 h-4" />
        </button>

        <button
          onClick={() => onDeleteBlock()}
          className="cursor-pointer text-slate-300 hover:text-rose-500 dark:text-slate-600 dark:hover:text-rose-400 p-0.5 rounded transition-colors"
          title="删除此块"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* 2. Main content container mapped by block types */}
      <div className="flex-1 min-w-0" id={`block-body-${block.id}`}>
        {/* TYPE 1: HEADING 1 */}
        {block.type === "heading-1" && (
          <ContentEditableBlock
            ref={elementRef as React.RefObject<HTMLDivElement>}
            tagName="h1"
            onKeyDown={handleKeyDown}
            html={rawText}
            onChange={(val, text) => handleTextChange(val, elementRef.current!, text)}
            placeholder="主标题 1"
            className="w-full text-2xl font-bold text-slate-900 dark:text-slate-50 tracking-tight bg-transparent border-none focus:outline-none focus:ring-0 placeholder-slate-300"
          />
        )}

        {/* TYPE 2: HEADING 2 */}
        {block.type === "heading-2" && (
          <ContentEditableBlock
            ref={elementRef as React.RefObject<HTMLDivElement>}
            tagName="h2"
            onKeyDown={handleKeyDown}
            html={rawText}
            onChange={(val, text) => handleTextChange(val, elementRef.current!, text)}
            placeholder="主题分类 2"
            className="w-full text-xl font-bold text-slate-900 dark:text-slate-150 tracking-tight bg-transparent border-none focus:outline-none focus:ring-0 placeholder-slate-300"
          />
        )}

        {/* TYPE 3: HEADING 3 */}
        {block.type === "heading-3" && (
          <ContentEditableBlock
            ref={elementRef as React.RefObject<HTMLDivElement>}
            tagName="h3"
            onKeyDown={handleKeyDown}
            html={rawText}
            onChange={(val, text) => handleTextChange(val, elementRef.current!, text)}
            placeholder="小标题 3"
            className="w-full text-lg font-semibold text-slate-800 dark:text-slate-200 tracking-tight bg-transparent border-none focus:outline-none focus:ring-0 placeholder-slate-300"
          />
        )}

        {/* TYPE 4: TEXT block */}
        {block.type === "text" && (
          <div 
            className="relative group/text"
            onClick={(e) => {
              const target = e.target as HTMLElement;
              const link = target.closest('.wiki-link');
              if (link) {
                e.preventDefault();
                const docId = link.getAttribute('data-doc-id');
                if (docId) onNavigateToDoc(docId);
              }
            }}
          >
            <ContentEditableBlock
              ref={elementRef as React.RefObject<HTMLDivElement>}
              tagName="div"
              onKeyDown={handleKeyDown}
              onBlur={handleBlur}
              html={rawText}
              onChange={(val, text) => handleTextChange(val, elementRef.current!, text)}
              placeholder="输入文本，输入 / 快捷唤出菜单..."
              className="w-full text-sm text-slate-700 dark:text-slate-300 bg-transparent border-none focus:outline-none focus:ring-0 leading-relaxed block"
            />
          </div>
        )}

        {/* TYPE 5: CALLOUT block */}
        {block.type === "callout" && (
          <div className="flex items-start gap-3 p-3 bg-slate-50 dark:bg-[#111] rounded-r-md border-l-2 border-indigo-500 border-y border-r border-y-transparent border-r-transparent">
            <FileText className="w-4 h-4 text-slate-500 mt-0.5 shrink-0" />
            <div className="flex-1">
              <input
                ref={elementRef as React.RefObject<HTMLInputElement>}
                onKeyDown={handleKeyDown}
                type="text"
                value={rawText}
                onChange={(e) => handleTextChange(e.target.value)}
                placeholder="在此输入高亮提示卡内容..."
                className="w-full text-sm font-medium text-slate-800 dark:text-slate-200 bg-transparent border-none focus:outline-none focus:ring-0"
              />
            </div>
          </div>
        )}

        {/* TYPE 6: CODE block */}
        {block.type === "code" && (
          <div className="border border-slate-200 dark:border-slate-800 rounded-md overflow-hidden bg-slate-50 dark:bg-[#0b0c10]">
            <div className="bg-slate-100 dark:bg-[#151720] px-3 py-1.5 text-xs text-slate-500 dark:text-slate-400 flex items-center justify-between border-b border-slate-200 dark:border-slate-800">
              <div className="flex items-center gap-1.5">
                <Code className="w-3.5 h-3.5 text-indigo-500" />
                <span className="font-mono">
                  {block.properties?.language || "JavaScript"}
                </span>
              </div>
              <select
                value={block.properties?.language || "javascript"}
                onChange={(e) =>
                  onUpdateBlock({
                    properties: {
                      ...block.properties,
                      language: e.target.value,
                    },
                  })
                }
                className="bg-transparent border-none text-[11px] text-slate-600 dark:text-slate-400 focus:outline-none pl-1 cursor-pointer"
              >
                <option value="javascript">JavaScript</option>
                <option value="typescript">TypeScript</option>
                <option value="html">HTML Code</option>
                <option value="css">CSS Syntax</option>
                <option value="python">Python</option>
                <option value="rust">Rust</option>
              </select>
            </div>
            <textarea
              value={rawText}
              onChange={(e) => handleTextChange(e.target.value)}
              placeholder="// 在此输入代码..."
              className="w-full bg-transparent font-mono text-xs text-slate-800 dark:text-indigo-250 p-4 border-none resize-y min-h-[100px] focus:outline-none focus:ring-0"
            />
          </div>
        )}

        {/* TYPE 7: IMAGE block */}
        {block.type === "image" && (
          <div
            onDragOver={(e) => e.preventDefault()}
            onDrop={handleImageDrop}
            className="border border-dashed border-slate-300 dark:border-slate-700 rounded-md p-3 flex flex-col items-center justify-center gap-2 bg-slate-50 dark:bg-[#111]"
          >
            {block.content ? (
              <div className="max-w-md w-full">
                <img
                  src={block.content}
                  alt={block.properties?.caption || "Image content"}
                  referrerPolicy="no-referrer"
                  className="rounded-lg object-contain w-full max-h-72 border border-slate-200 dark:border-slate-800 mx-auto"
                />
                <input
                  type="text"
                  value={block.properties?.caption || ""}
                  onChange={(e) =>
                    onUpdateBlock({
                      properties: {
                        ...block.properties,
                        caption: e.target.value,
                      },
                    })
                  }
                  placeholder="添加说明文字 ( Caption )..."
                  className="w-full mt-2 text-center text-xs text-slate-400 bg-transparent border-none focus:outline-none"
                />
              </div>
            ) : (
              <div className="text-center py-4 space-y-2">
                <ImageIcon className="w-8 h-8 text-slate-400 mx-auto animate-pulse" />
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  可拖放本地图片至此 或
                </p>
                <input
                  type="text"
                  placeholder="粘贴在线图片 URL 以加载..."
                  onChange={(e) => onUpdateBlock({ content: e.target.value })}
                  className="px-3 py-1.5 text-xs rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-100 focus:outline-none w-64 text-center"
                />
              </div>
            )}
          </div>
        )}

        {/* TYPE 8: DRAWING CANVAS BLOCK (FREE BRUSH SKETCHING) */}
        {block.type === "canvas" && (
          <div className="border border-slate-200 dark:border-slate-800 rounded-md overflow-hidden bg-white dark:bg-black">
            {/* Header controls */}
            <div className="bg-slate-50 dark:bg-[#151720] px-4 py-2 border-b border-slate-200 dark:border-slate-800 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <Palette className="w-4 h-4 text-indigo-500" />
                <span className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                  手绘涂鸦 / 脑图画布
                </span>
              </div>

              <div className="flex items-center gap-4">
                {/* Brush size */}
                <div className="flex items-center gap-1.5 text-xs text-slate-500">
                  <span>粗细:</span>
                  <select
                    value={brushWidth}
                    onChange={(e) => setBrushWidth(parseInt(e.target.value))}
                    className="bg-transparent border border-slate-200 dark:border-slate-800 rounded px-1 text-[11px]"
                  >
                    <option value="2">极细 (2px)</option>
                    <option value="4">常规 (4px)</option>
                    <option value="8">中等 (8px)</option>
                    <option value="12">极粗 (12px)</option>
                  </select>
                </div>

                {/* Color Palette */}
                <div className="flex items-center gap-1.5">
                  {[
                    "#4f46e5",
                    "#34d399",
                    "#ef4444",
                    "#f59e0b",
                    "#000000",
                    "#94a3b8",
                  ].map((color) => (
                    <button
                      key={color}
                      onClick={() => setBrushColor(color)}
                      className={`w-4 h-4 rounded-full transition-transform cursor-pointer ${
                        brushColor === color
                          ? "ring-2 ring-indigo-500 scale-110"
                          : ""
                      }`}
                      style={{ backgroundColor: color }}
                    />
                  ))}
                </div>

                {/* Reset button */}
                <button
                  onClick={clearCanvasAll}
                  className="cursor-pointer text-[10px] text-rose-500 hover:text-rose-600 font-medium px-2 py-0.5 rounded border border-rose-200/50 dark:border-rose-900/30 hover:bg-rose-50/50"
                >
                  清空画布
                </button>
              </div>
            </div>

            {/* Canvas body */}
            <div className="relative bg-slate-50/50 dark:bg-slate-950/20 py-2 flex justify-center">
              <canvas
                ref={canvasRef}
                width={550}
                height={260}
                onMouseDown={startCanvasDrawing}
                onMouseMove={drawOnCanvas}
                onMouseUp={endCanvasDrawing}
                onMouseLeave={endCanvasDrawing}
                className="bg-white dark:bg-slate-950 border border-slate-150 dark:border-slate-850 rounded-lg cursor-crosshair shadow-inner"
              />
            </div>

            <div className="p-2 border-t border-slate-100 dark:border-slate-850/50 text-[10px] text-center text-slate-400">
              提示：鼠标按住并拖动即可书写，绘制路径将自动同步保存。
            </div>
          </div>
        )}

        {/* TYPE 9: TABLE block (Custom fully editable grid) */}
        {block.type === "table" && (
          <div className="overflow-x-auto border border-slate-200 dark:border-slate-800 rounded-md p-3 bg-white dark:bg-[#111]">
            <div className="text-xs font-semibold text-indigo-600 dark:text-indigo-400 flex items-center gap-1 mb-2">
              <TableIcon className="w-3.5 h-3.5" />
              <span>交互式数据表格 (Table Grid)</span>
            </div>

            <table className="w-full text-xs text-left text-slate-500 border-collapse">
              <tbody>
                {(block.properties?.tableData || [["A", "B"]]).map(
                  (row, rowIdx) => (
                    <tr
                      key={rowIdx}
                      className={`${
                        rowIdx === 0
                          ? "bg-slate-50 dark:bg-slate-950 font-semibold text-slate-700 dark:text-slate-350"
                          : "border-b border-slate-100 dark:border-slate-850"
                      }`}
                    >
                      {row.map((cell, colIdx) => (
                        <td
                          key={colIdx}
                          className="p-1.5 border border-slate-200/20 dark:border-white/5"
                        >
                          <input
                            type="text"
                            value={cell}
                            onChange={(e) =>
                              handleTableCellEdit(
                                rowIdx,
                                colIdx,
                                e.target.value,
                              )
                            }
                            className="w-full bg-transparent border-none text-xs text-slate-800 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-500/50 px-1 py-0.5 rounded"
                          />
                        </td>
                      ))}
                    </tr>
                  ),
                )}
              </tbody>
            </table>

            {/* Grid control bar */}
            <div className="flex flex-wrap items-center gap-3 mt-3 text-[10px] text-slate-500 justify-end border-t border-slate-100 dark:border-slate-850 pt-2">
              <div className="flex gap-1.5">
                <button
                  onClick={addTableRow}
                  className="cursor-pointer bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300 px-2 py-0.5 rounded hover:bg-indigo-50"
                >
                  + 新增行
                </button>
                <button
                  onClick={removeTableRow}
                  className="cursor-pointer bg-slate-100 dark:bg-slate-800 text-rose-600 px-2 py-0.5 rounded hover:bg-rose-50"
                >
                  - 裁减行
                </button>
              </div>

              <div className="h-3 w-[1px] bg-slate-200 dark:bg-slate-850" />

              <div className="flex gap-1.5">
                <button
                  onClick={addTableColumn}
                  className="cursor-pointer bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300 px-2 py-0.5 rounded hover:bg-indigo-50"
                >
                  + 新增列
                </button>
                <button
                  onClick={removeTableColumn}
                  className="cursor-pointer bg-slate-100 dark:bg-slate-800 text-rose-600 px-2 py-0.5 rounded hover:bg-rose-50"
                >
                  - 裁减列
                </button>
              </div>
            </div>
          </div>
        )}

        {/* TYPE 10: TOGGLE FOLDABLE BLOCK */}
        {block.type === "toggle" && (
          <div className="border border-slate-200 dark:border-slate-800 rounded-md p-3 bg-white dark:bg-[#111]">
            <div className="flex items-center gap-2 cursor-pointer">
              <button
                onClick={() =>
                  onUpdateBlock({
                    properties: {
                      ...block.properties,
                      isOpen: !block.properties?.isOpen,
                    },
                  })
                }
                className="cursor-pointer text-slate-400 hover:text-slate-600"
              >
                {block.properties?.isOpen ? (
                  <ChevronDown className="w-4 h-4" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
              </button>

              <input
                ref={elementRef as React.RefObject<HTMLInputElement>}
                onKeyDown={handleKeyDown}
                type="text"
                value={rawText}
                onChange={(e) => handleTextChange(e.target.value)}
                placeholder="折叠区主题..."
                className="w-full text-sm font-semibold text-slate-800 dark:text-slate-200 bg-transparent border-none focus:outline-none"
              />
            </div>

            {block.properties?.isOpen && (
              <div className="pl-6 mt-2 pt-2 border-t border-slate-150 dark:border-slate-850 animate-in slide-in-from-top-1 duration-150">
                <div className="flex items-start gap-2">
                  <CornerDownRight className="w-4 h-4 text-slate-300 mt-1" />
                  <textarea
                    value={block.properties?.caption || ""}
                    onChange={(e) =>
                      onUpdateBlock({
                        properties: {
                          ...block.properties,
                          caption: e.target.value,
                        },
                      })
                    }
                    placeholder="折叠详情与附加段落，可输入并在此存储您的折叠展开数据..."
                    className="w-full text-xs text-slate-600 dark:text-slate-400 bg-transparent border-none resize-none focus:outline-none"
                    rows={2}
                  />
                </div>
              </div>
            )}
          </div>
        )}

        {/* TYPE 11: LIVE HTML PLAYBOARD & SANDBOX RENDER */}
        {block.type === "html-render" && (
          <div className="border border-slate-200 dark:border-slate-800 rounded-md overflow-hidden bg-white dark:bg-[#111111] shadow-sm">
            {/* Header Toolbar */}
            <div className="bg-slate-50 dark:bg-[#1a1c23] px-3 py-2 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <FileCode className="w-3.5 h-3.5 text-slate-500" />
                <span className="text-xs font-medium text-slate-700 dark:text-slate-300">
                  代码沙盒
                </span>
              </div>
              <div className="flex items-center gap-1">
                {[
                  { id: "preview", label: "预览" },
                  { id: "html", label: "HTML" },
                  { id: "css", label: "CSS" },
                  { id: "js", label: "JS" },
                ].map((tab) => (
                  <button
                    key={tab.id}
                    onClick={() => setSandboxTab(tab.id as any)}
                    className={`cursor-pointer px-2 py-1 rounded text-[10px] transition-colors ${
                      sandboxTab === tab.id ||
                      (sandboxTab === "split" && tab.id === "preview")
                        ? "bg-slate-200 dark:bg-slate-700 text-slate-800 dark:text-slate-200"
                        : "text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"
                    }`}
                  >
                    {tab.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Workspaces */}
            {(sandboxTab === "preview" || sandboxTab === "split") && (
              <iframe
                title={`Sandbox Preview ${block.id}`}
                srcDoc={sandboxDebouncedSrcDoc}
                sandbox="allow-scripts allow-modals"
                className="w-full h-96 border-none bg-white dark:bg-slate-900"
              />
            )}

            {["html", "css", "js"].includes(sandboxTab) && (
              <textarea
                value={
                  sandboxTab === "html"
                    ? sandboxHtml
                    : sandboxTab === "css"
                      ? sandboxCss
                      : sandboxJs
                }
                onChange={(e) => {
                  const val = e.target.value;
                  if (sandboxTab === "html")
                    handleSandboxChange(
                      val,
                      sandboxCss,
                      sandboxJs,
                      sandboxTheme,
                    );
                  else if (sandboxTab === "css")
                    handleSandboxChange(
                      sandboxHtml,
                      val,
                      sandboxJs,
                      sandboxTheme,
                    );
                  else
                    handleSandboxChange(
                      sandboxHtml,
                      sandboxCss,
                      val,
                      sandboxTheme,
                    );
                }}
                className={`w-full h-80 font-mono text-xs bg-slate-50 dark:bg-[#0b0c10] p-4 border-none resize-y focus:outline-none ${
                  sandboxTab === "html"
                    ? "text-slate-800 dark:text-amber-200"
                    : sandboxTab === "css"
                      ? "text-slate-800 dark:text-sky-200"
                      : "text-slate-800 dark:text-emerald-200"
                }`}
              />
            )}
          </div>
        )}

        {/* TYPE 12: TLDRAW WHITEBOARD */}
        {block.type === "whiteboard" && (
          <div className="h-[500px] w-full border border-slate-200 dark:border-slate-800 rounded-md overflow-hidden relative bg-white dark:bg-black">
            <Tldraw persistenceKey={`tldraw-${block.id}`} />
          </div>
        )}

        {/* 3. Slash Menu `/` Floating Commands Menu Popover */}
        {showSlashMenu && (
          <div
            className="absolute z-50 mt-1 w-48 rounded-md bg-white dark:bg-[#1a1c23] border border-slate-200 dark:border-white/10 shadow-xl overflow-hidden animate-in fade-in slide-in-from-top-1 duration-100 text-slate-700 dark:text-slate-300"
            style={{
              top: slashMenuCoords ? slashMenuCoords.top : "100%",
              left: slashMenuCoords
                ? Math.max(slashMenuCoords.left - 24, 0)
                : 16,
            }}
          >
            <div className="py-1 max-h-56 overflow-y-auto">
              {SLASH_COMMANDS.map((cmd, idx) => {
                const IconComp = cmd.icon;
                const isSelected = idx === slashMenuIndex;
                return (
                  <button
                    key={cmd.type}
                    onClick={() => executeSlashCommand(cmd.type as BlockType)}
                    className={`cursor-pointer w-full text-left px-3 py-1.5 flex items-center gap-2.5 transition-colors text-xs font-medium ${
                      isSelected
                        ? "bg-indigo-50 dark:bg-indigo-500/10 text-indigo-700 dark:text-indigo-300"
                        : "hover:bg-slate-100 dark:hover:bg-white/5"
                    }`}
                    id={`slash-cmd-${cmd.type}`}
                  >
                    <IconComp
                      className={`w-3.5 h-3.5 ${isSelected ? "text-indigo-500 dark:text-indigo-400" : "text-slate-400"}`}
                    />
                    <span>{cmd.label}</span>
                    {isSelected && (
                      <span className="ml-auto text-[9px] text-indigo-400 dark:text-indigo-500">
                        ↵
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
