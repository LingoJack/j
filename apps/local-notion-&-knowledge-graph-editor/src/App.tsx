import React, { useState, useEffect } from 'react';
import { Document } from './types';
import { DEFAULT_DOCUMENTS } from './data/defaultData';
import DocumentList from './components/DocumentList';
import BlockEditor from './components/BlockEditor';
import LocalFolder from './components/LocalFolder';
import ArticleOutline from './components/ArticleOutline';
import {
  Sparkles,
  FileText,
  Info,
  FolderDot,
  PanelLeftClose,
  PanelLeft,
  PanelRightClose,
  PanelRight,
  Plus,
  Trash2,
  Star
} from 'lucide-react';

export default function App() {
  // 1. Core Reactive States
  const [documents, setDocuments] = useState<Document[]>([]);
  const [activeDocId, setActiveDocId] = useState<string>('');
  const [isDarkMode, setIsDarkMode] = useState<boolean>(true);
  const currentViewMode = 'editor';
  const [isFolderOpen, setIsFolderOpen] = useState(false);
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [isOutlineOpen, setIsOutlineOpen] = useState(true);

  // --- SYSTEM ADVANCED CONTROLS TRIGGERED BY SETTINGS ---
  const handleRestoreDefaults = () => {
    if (window.confirm('确定恢复初始教学脑图与示例项目吗？当前修改过的内容将会被覆盖。')) {
      setDocuments(DEFAULT_DOCUMENTS);
      setActiveDocId(DEFAULT_DOCUMENTS[0].id);
      localStorage.setItem('omninote_docs', JSON.stringify(DEFAULT_DOCUMENTS));
    }
  };

  const handleClearAll = () => {
    if (window.confirm('确认清空库下的所有文档吗？此操作不可逆。')) {
      const defaultEmpty: Document = {
        id: `doc-empty-${Date.now()}`,
        title: '我的新知识脑图',
        emoji: '',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        blocks: [
          {
            id: `block-empty-${Date.now()}`,
            type: 'text',
            content: '这里是您的新天地！开始记录吧。输入 / 呼出组件大礼包。',
            properties: {}
          }
        ]
      };
      setDocuments([defaultEmpty]);
      setActiveDocId(defaultEmpty.id);
      localStorage.setItem('omninote_docs', JSON.stringify([defaultEmpty]));
    }
  };

  const handleImportData = (importText: string): boolean => {
    try {
      const parsed = JSON.parse(importText);
      if (Array.isArray(parsed) && parsed.length > 0 && parsed[0].id && parsed[0].blocks) {
        setDocuments(parsed);
        setActiveDocId(parsed[0].id);
        localStorage.setItem('omninote_docs', JSON.stringify(parsed));
        alert('离线备份成功恢复并就绪！');
        return true;
      } else {
        alert('JSON 格式有误，需为一个包含 Document 的标准 Array 格式。');
        return false;
      }
    } catch (err) {
      alert('解析失败，请检查是否为合格的 JSON 文件文本：' + err);
      return false;
    }
  };

  const handleInsertAssetAsBlock = (asset: any) => {
    if (!activeDocId) {
      alert('请先选择或创建一个目标文档，然后再置入此本地附件。');
      return;
    }
    const targetDoc = documents.find(d => d.id === activeDocId);
    if (!targetDoc) return;

    let newBlock;
    if (asset.type.startsWith('image/')) {
      newBlock = {
        id: `block-${Date.now()}`,
        type: 'image' as const,
        content: asset.content || '',
        properties: { caption: asset.name, imageType: asset.content ? 'base64' : 'url' }
      };
    } else {
      newBlock = {
        id: `block-${Date.now()}`,
        type: 'callout' as const,
        content: `📁 **已关联本地存储物理附件**: [${asset.name}] (${asset.size})\n*创建时间: ${new Date(asset.createdAt).toLocaleString()} • 双链指令 [[ 建立语义索引*`,
        properties: { emoji: '📎' }
      };
    }

    const updatedBlocks = [...targetDoc.blocks, newBlock];
    const updated = documents.map((doc) => {
      if (doc.id === activeDocId) {
        return {
          ...doc,
          blocks: updatedBlocks,
          updatedAt: new Date().toISOString(),
        };
      }
      return doc;
    });
    setDocuments(updated);
    localStorage.setItem('omninote_docs', JSON.stringify(updated));
  };

  // --- INITIAL LOADING (LocalStorage Check) ---
  useEffect(() => {
    // A. Load documents
    const savedDocs = localStorage.getItem('omninote_docs');
    if (savedDocs) {
      try {
        const parsed = JSON.parse(savedDocs) as Document[];
        if (parsed.length > 0) {
          setDocuments(parsed);
          setActiveDocId(parsed[0].id);
        } else {
          setDocuments(DEFAULT_DOCUMENTS);
          setActiveDocId(DEFAULT_DOCUMENTS[0].id);
        }
      } catch (e) {
        setDocuments(DEFAULT_DOCUMENTS);
        setActiveDocId(DEFAULT_DOCUMENTS[0].id);
      }
    } else {
      setDocuments(DEFAULT_DOCUMENTS);
      setActiveDocId(DEFAULT_DOCUMENTS[0].id);
    }

    // B. Load theme preferences (Default to dark)
    const savedTheme = localStorage.getItem('omninote_theme');
    if (savedTheme === 'light') {
      setIsDarkMode(false);
      document.documentElement.classList.remove('dark');
    } else {
      setIsDarkMode(true);
      document.documentElement.classList.add('dark');
    }
  }, []);

  // --- DARK MODE TRIGGERS ---
  const handleToggleDarkMode = () => {
    const nextDark = !isDarkMode;
    setIsDarkMode(nextDark);
    if (nextDark) {
      document.documentElement.classList.add('dark');
      localStorage.setItem('omninote_theme', 'dark');
    } else {
      document.documentElement.classList.remove('dark');
      localStorage.setItem('omninote_theme', 'light');
    }
  };

  // --- PHYSICAL DOCUMENT ACTIONS ---
  const handleCreateDocument = () => {
    const newDoc: Document = {
      id: `doc-${Date.now()}`,
      title: '未命名文档',
      emoji: '',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      blocks: [
        {
          id: `block-${Date.now()}-initial`,
          type: 'text',
          content: '欢迎来到新页面！输入 [[ 可引用其它文档名称，输入 / 快速唤出交互块。',
          properties: {},
        },
      ],
    };

    const updated = [newDoc, ...documents];
    setDocuments(updated);
    setActiveDocId(newDoc.id);
    localStorage.setItem('omninote_docs', JSON.stringify(updated));
  };

  const handleDeleteDocument = (id: string) => {
    if (documents.length <= 1) {
      alert('抱歉，本地库中至少需要保留一篇文档，无法继续删除该项目。');
      return;
    }
    const updated = documents.filter((doc) => doc.id !== id);
    setDocuments(updated);
    if (activeDocId === id) {
      setActiveDocId(updated[0].id);
    }
    localStorage.setItem('omninote_docs', JSON.stringify(updated));
  };

  const handleToggleFavorite = (id: string) => {
    const updated = documents.map((doc) => {
      if (doc.id === id) {
        return { ...doc, isFavorite: !doc.isFavorite };
      }
      return doc;
    });
    setDocuments(updated);
    localStorage.setItem('omninote_docs', JSON.stringify(updated));
  };

  const handleUpdateDocument = (updatedFields: Partial<Document>) => {
    const updated = documents.map((doc) => {
      if (doc.id === activeDocId) {
        return {
          ...doc,
          ...updatedFields,
          updatedAt: new Date().toISOString(),
        };
      }
      return doc;
    });
    setDocuments(updated);
    localStorage.setItem('omninote_docs', JSON.stringify(updated));
  };

  // Extract selected active document
  const activeDocument = documents.find((doc) => doc.id === activeDocId);

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-[#09090b] text-slate-850 dark:text-slate-100 transition-colors flex items-center justify-center font-sans tracking-tight relative overflow-hidden md:p-2">
      
      {/* 💻 Wrapper Frame */}
      <div className="w-full h-full md:max-w-7xl md:h-[94vh] bg-white dark:bg-[#0f0f11] md:rounded-xl md:shadow-2xl md:border border-slate-200/50 dark:border-white/5 flex flex-col overflow-hidden animate-in fade-in duration-300 relative z-10">
        
        {/* Minimal Top Navigation */}
        <div className="h-12 border-b border-slate-100 dark:border-white/5 px-4 flex items-center justify-between shrink-0 select-none bg-white/50 dark:bg-black/20 backdrop-blur-md">
          {/* Left: Sidebar toggle & Title */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setIsSidebarOpen(!isSidebarOpen)}
              className="cursor-pointer p-1.5 rounded text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5 hover:text-slate-600 dark:hover:text-slate-300 transition-colors"
              title={isSidebarOpen ? "收拢左边栏" : "展开左边栏"}
            >
              {isSidebarOpen ? <PanelLeftClose className="w-4 h-4" /> : <PanelLeft className="w-4 h-4" />}
            </button>
            <div className="font-semibold text-sm text-slate-600 dark:text-slate-400 hidden sm:flex items-center gap-1.5 ml-1">
              <Sparkles className="w-3.5 h-3.5 text-indigo-400" />
              <span>OmniNote</span>
            </div>
          </div>

          {/* Right: Actions */}
          <div className="flex items-center gap-1">
            <button
              onClick={handleCreateDocument}
              className="cursor-pointer bg-slate-800 hover:bg-slate-700 dark:bg-white/10 dark:hover:bg-white/20 text-white rounded px-2.5 py-1.5 text-xs font-medium flex items-center gap-1 transition-colors mr-1"
            >
              <Plus className="w-3.5 h-3.5" />
              <span className="hidden md:inline">新建</span>
            </button>

            {activeDocument && (
              <>
                <button
                  onClick={() => handleToggleFavorite(activeDocument.id)}
                  className={`cursor-pointer p-1.5 rounded transition-colors ${
                    activeDocument.isFavorite
                      ? 'text-amber-500 hover:bg-amber-50 dark:hover:bg-amber-500/10'
                      : 'text-slate-400 hover:text-slate-600 hover:bg-slate-100 dark:hover:bg-white/5'
                  }`}
                >
                  <Star className={`w-4 h-4 ${activeDocument.isFavorite ? 'fill-amber-400 border-none' : ''}`} />
                </button>

                <button
                  onClick={() => handleDeleteDocument(activeDocument.id)}
                  className="cursor-pointer p-1.5 rounded text-slate-400 hover:text-rose-500 hover:bg-rose-50 dark:hover:bg-rose-500/10 transition-colors"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
                
                <div className="w-[1px] h-3.5 bg-slate-200 dark:bg-white/10 mx-1 hidden sm:block" />
              </>
            )}

            <button
              onClick={() => setIsFolderOpen(!isFolderOpen)}
              className={`cursor-pointer p-1.5 rounded transition-colors ${
                isFolderOpen
                  ? 'bg-slate-200 dark:bg-white/10 text-slate-800 dark:text-slate-200'
                  : 'text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5 hover:text-slate-600'
              }`}
            >
              <FolderDot className="w-4 h-4" />
            </button>

            {activeDocument && (
              <button
                onClick={() => setIsOutlineOpen(!isOutlineOpen)}
                className={`cursor-pointer p-1.5 rounded transition-colors ${
                  isOutlineOpen
                    ? 'bg-slate-200 dark:bg-white/10 text-slate-800 dark:text-slate-200'
                    : 'text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5 hover:text-slate-600'
                }`}
              >
                {isOutlineOpen ? <PanelRightClose className="w-4 h-4" /> : <PanelRight className="w-4 h-4" />}
              </button>
            )}
          </div>
        </div>

        {/* Workspace Shell Split-Pane */}
        <div className="flex-1 min-h-0 flex flex-col md:flex-row">
          
          {/* Collapsible Left Sidebar */}
          {isSidebarOpen && (
            <DocumentList
              documents={documents}
              activeDocId={activeDocId}
              onSelectDocument={setActiveDocId}
              onDeleteDocument={handleDeleteDocument}
              onToggleFavorite={handleToggleFavorite}
              isDarkMode={isDarkMode}
              onToggleDarkMode={handleToggleDarkMode}
              onRestoreDefaults={handleRestoreDefaults}
              onClearAll={handleClearAll}
              onImportData={handleImportData}
            />
          )}

          {/* Core dynamic Main Stage Container - edge to edge */}
          <div className="flex-1 min-w-0 h-full bg-white dark:bg-[#0f0f11] transition-all">
            {activeDocument ? (
              <div className="w-full h-full flex flex-col">
                <div className="w-full h-full flex animate-in fade-in duration-300">
                  <div className="flex-1 h-full min-w-0 flex flex-col">
                    <BlockEditor
                      document={activeDocument}
                      documents={documents}
                      onSelectDocument={setActiveDocId}
                      onUpdateDocument={handleUpdateDocument}
                    />
                  </div>
                  {isOutlineOpen && (
                    <div className="hidden lg:block w-56 shrink-0 h-full border-l border-slate-100 dark:border-white/5">
                      <ArticleOutline document={activeDocument} />
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div className="w-full h-full flex flex-col items-center justify-center text-center gap-3">
                <Info className="w-10 h-10 text-indigo-500 animate-bounce" />
                <div>
                  <h3 className="font-semibold text-slate-800 dark:text-slate-100">请选择左侧文档</h3>
                  <p className="text-xs text-slate-500 mt-1">创建或选择现有文档立即进行整理编写。</p>
                </div>
              </div>
            )}
          </div>
        </div>

        {isFolderOpen && (
          <LocalFolder
            onInsertAsset={handleInsertAssetAsBlock}
            onClose={() => setIsFolderOpen(false)}
          />
        )}

      </div>
    </div>
  );
}
