import React, { useState } from 'react';
import { Document, Plugin } from '../types';
import {
  Search,
  Plus,
  BookOpen,
  FolderDot,
  Settings,
  RefreshCw,
  Package,
  Sun,
  Moon,
  Trash,
  Star,
  FileText
} from 'lucide-react';

interface DocumentListProps {
  documents: Document[];
  activeDocId: string;
  onSelectDocument: (id: string) => void;
  onDeleteDocument: (id: string) => void;
  onToggleFavorite: (id: string) => void;
  isDarkMode: boolean;
  onToggleDarkMode: () => void;
  onRestoreDefaults: () => void;
  onClearAll: () => void;
  onImportData: (importedText: string) => boolean;
}

export default function DocumentList({
  documents,
  activeDocId,
  onSelectDocument,
  onDeleteDocument,
  onToggleFavorite,
  isDarkMode,
  onToggleDarkMode,
  onRestoreDefaults,
  onClearAll,
  onImportData,
}: DocumentListProps) {
  const [search, setSearch] = useState('');
  const [showSettingsPopover, setShowSettingsPopover] = useState(false);
  const [importText, setImportText] = useState('');
  const [showImporter, setShowImporter] = useState(false);

  const filteredDocs = documents.filter((doc) =>
    doc.title.toLowerCase().includes(search.toLowerCase())
  );

  const favoriteDocs = filteredDocs.filter((doc) => doc.isFavorite);
  const otherDocs = filteredDocs.filter((doc) => !doc.isFavorite);

  return (
    <div className="w-full md:w-60 h-full bg-white dark:bg-[#0f0f11] border-r border-slate-100 dark:border-white/5 flex flex-col p-3 select-none z-10">
      
      {/* Search Input */}
      <div className="relative mb-4">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400" />
        <input
          type="text"
          placeholder="查找文档..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full text-xs pl-8 pr-3 py-2 rounded border border-slate-100 dark:border-white/5 bg-slate-50 dark:bg-[#151720] text-slate-800 dark:text-white placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-indigo-500/50"
        />
      </div>

      {/* Docs List viewports */}
      <div className="flex-1 overflow-y-auto space-y-4 pr-1">
        
        {/* Favorite section */}
        {favoriteDocs.length > 0 && (
          <div className="space-y-1">
            <h4 className="text-[10px] font-semibold text-slate-400 flex items-center gap-1.5 px-2 mb-2">
              <Star className="w-3 h-3 text-amber-500" />
              <span>常用 {favoriteDocs.length}</span>
            </h4>
            
            <div className="space-y-0.5">
              {favoriteDocs.map((doc) => (
                <div
                  key={doc.id}
                  onClick={() => onSelectDocument(doc.id)}
                  className={`group flex items-center justify-between px-2 py-1.5 rounded cursor-pointer transition-colors ${
                    doc.id === activeDocId
                      ? 'bg-slate-100 dark:bg-white/10 text-slate-900 dark:text-slate-100 font-medium'
                      : 'hover:bg-slate-50 dark:hover:bg-white/5 text-slate-600 dark:text-slate-400'
                  }`}
                  id={`sidebar-doc-${doc.id}`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <FileText className="w-3.5 h-3.5 opacity-60 shrink-0" />
                    <span className="text-xs truncate">{doc.title || '无标题'}</span>
                  </div>

                  {/* Sidebar item controls */}
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleFavorite(doc.id);
                      }}
                      className="cursor-pointer text-amber-500 p-0.5 hover:text-amber-600 rounded"
                      id={`unfav-${doc.id}`}
                    >
                      <Star className="w-3 h-3 fill-amber-500" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteDocument(doc.id);
                      }}
                      className="cursor-pointer text-slate-400 hover:text-rose-500 p-0.5 rounded"
                      id={`delete-${doc.id}`}
                    >
                      <Trash className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Local Documents collection */}
        <div className="space-y-1">
          <h4 className="text-[10px] font-semibold text-slate-400 flex items-center gap-1.5 px-2 mb-2 mt-4">
            <FolderDot className="w-3 h-3" />
            <span>全部知识本 {otherDocs.length}</span>
          </h4>
          
          <div className="space-y-0.5">
            {otherDocs.length === 0 && favoriteDocs.length === 0 ? (
              <p className="text-[10px] text-slate-400 px-2 py-2">
                暂无匹配文档
              </p>
            ) : (
              otherDocs.map((doc) => (
                <div
                  key={doc.id}
                  onClick={() => onSelectDocument(doc.id)}
                  className={`group flex items-center justify-between px-2 py-1.5 rounded cursor-pointer transition-colors ${
                    doc.id === activeDocId
                      ? 'bg-slate-100 dark:bg-white/10 text-slate-900 dark:text-slate-100 font-medium'
                      : 'hover:bg-slate-50 dark:hover:bg-white/5 text-slate-600 dark:text-slate-400'
                  }`}
                  id={`sidebar-doc-${doc.id}`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <FileText className="w-3.5 h-3.5 opacity-60 shrink-0" />
                    <span className="text-xs truncate">{doc.title || '无标题'}</span>
                  </div>

                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleFavorite(doc.id);
                      }}
                      className="cursor-pointer text-slate-400 hover:text-amber-500 p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-800"
                      title="添加常用"
                      id={`fav-${doc.id}`}
                    >
                      <Star className="w-3 h-3" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteDocument(doc.id);
                      }}
                      className="cursor-pointer text-slate-400 hover:text-rose-500 p-0.5 rounded"
                      id={`delete-${doc.id}`}
                    >
                      <Trash className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

      </div>

      {/* Footer controls dock */}
      <div className="pt-3 border-t border-slate-100 dark:border-white/5 flex items-center justify-between gap-2 text-xs relative shrink-0">
        
        {/* Settings Popover wrapper */}
        {showSettingsPopover && (
          <div className="absolute bottom-10 left-0 w-64 bg-white dark:bg-[#1a1c23] border border-slate-200 dark:border-white/10 rounded-md p-3 shadow-lg z-50 animate-in fade-in slide-in-from-bottom-2 duration-100">
            <div className="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-white/5 mb-2">
              <span className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 text-xs">
                <Settings className="w-3 h-3 text-slate-500" />
                <span>偏好设置</span>
              </span>
              <button
                onClick={() => {
                  setShowSettingsPopover(false);
                  setShowImporter(false);
                }}
                className="text-xs text-slate-400 hover:text-slate-600 dark:hover:text-white cursor-pointer px-1"
              >
                收起
              </button>
            </div>

            <div className="space-y-1.5">
              <button
                onClick={() => {
                  onRestoreDefaults();
                  setShowSettingsPopover(false);
                }}
                className="cursor-pointer w-full text-left p-2 rounded bg-slate-50 dark:bg-white/5 hover:bg-slate-100 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 transition-colors text-xs flex items-center justify-between font-medium"
              >
                <span>恢复初始数据</span>
                <span className="text-[10px] text-slate-400 font-mono">Rest</span>
              </button>

              <button
                onClick={() => {
                  if(window.confirm('确定清空所有本地数据吗？')){
                     onClearAll();
                     setShowSettingsPopover(false);
                  }
                }}
                className="cursor-pointer w-full text-left p-2 rounded bg-slate-50 dark:bg-white/5 hover:bg-rose-50 dark:hover:bg-rose-900/20 text-rose-600 dark:text-rose-400 transition-colors text-xs flex items-center justify-between font-medium"
              >
                <span>清空本地文档</span>
                <span className="text-[10px] text-rose-400/50 font-mono">Warn</span>
              </button>

              <div className="h-[1px] bg-slate-100 dark:bg-white/5 my-2" />

              <div className="space-y-1 text-slate-600 dark:text-slate-400">
                <div className="flex items-center justify-between gap-1 text-[10px] px-1">
                  <span className="font-medium text-slate-500">主备份副本 (JSON)</span>
                  <button
                    onClick={() => {
                      try {
                        navigator.clipboard.writeText(JSON.stringify(documents, null, 2));
                        alert('已复制至剪贴板');
                      } catch (err) {
                        alert('导出失败: ' + err);
                      }
                    }}
                    className="cursor-pointer text-slate-500 font-medium hover:text-slate-700 dark:hover:text-slate-300"
                  >
                    备份整库
                  </button>
                </div>

                <button
                  type="button"
                  onClick={() => setShowImporter(!showImporter)}
                  className="cursor-pointer w-full py-1 text-center rounded border border-dashed border-slate-200 dark:border-white/10 text-[9.5px] text-slate-500 hover:text-indigo-500 bg-transparent transition-colors mt-1"
                >
                  {showImporter ? '隐藏导入输入区' : '📂 置入外部备份副本'}
                </button>

                {showImporter && (
                  <div className="space-y-1.5 mt-1.5 pt-1.5 border-t border-slate-100 dark:border-white/5">
                    <textarea
                      placeholder="粘贴备份数组 JSON..."
                      value={importText}
                      onChange={(e) => setImportText(e.target.value)}
                      rows={3}
                      className="w-full text-[9px] p-1.5 bg-slate-50 dark:bg-black/30 border border-slate-200 dark:border-white/5 rounded text-slate-800 dark:text-slate-350 font-mono focus:outline-none"
                    />
                    <button
                      type="button"
                      onClick={() => {
                        if (!importText) return;
                        const ok = onImportData(importText);
                        if (ok) {
                          setShowSettingsPopover(false);
                          setShowImporter(false);
                          setImportText('');
                        }
                      }}
                      className="cursor-pointer w-full py-1 bg-indigo-600 hover:bg-indigo-700 text-white rounded text-[10px] font-bold"
                    >
                      安全加载此副本
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Action Toggle Gear Button: Settings */}
        <button
          onClick={() => setShowSettingsPopover(!showSettingsPopover)}
          className="cursor-pointer flex-1 text-slate-650 dark:text-slate-350 hover:bg-white/30 dark:hover:bg-white/5 hover:text-indigo-600 dark:hover:text-indigo-400 p-1.5 rounded-lg flex items-center justify-center gap-1.5 transition-colors text-[11px] font-semibold border border-transparent hover:border-slate-200/50 dark:hover:border-white/10"
          id="btn-sidebar-settings"
        >
          <Settings className="w-4 h-4 text-indigo-500 animate-spin-slow" />
          <span>库设置</span>
        </button>

        {/* Theme select option next of settings */}
        <button
          onClick={onToggleDarkMode}
          className="cursor-pointer w-8 h-8 text-slate-650 dark:text-slate-350 hover:bg-white/30 dark:hover:bg-white/5 p-1.5 rounded-lg flex items-center justify-center transition-colors border border-transparent hover:border-slate-200/50 dark:hover:border-white/10 shrink-0"
          id="btn-toggle-theme"
          title="切换暗度/明亮外观"
        >
          {isDarkMode ? <Sun className="w-4 h-4 text-amber-500" /> : <Moon className="w-4 h-4 text-indigo-400" />}
        </button>
      </div>

    </div>
  );
}
