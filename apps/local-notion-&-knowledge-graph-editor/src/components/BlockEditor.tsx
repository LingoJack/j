import React, { useMemo, useState } from 'react';
import { Document, Block, BlockType } from '../types';
import BlockItem from './BlockItem';
import {
  Plus,
  Link2,
  BookOpen,
  Clock,
  Layers,
  Sparkles,
  MessageSquare,
  Heading2,
  Table as TableIcon,
  Palette,
  Code,
  FileCode,
  ChevronDown,
  FileText
} from 'lucide-react';

interface BlockEditorProps {
  document: Document;
  documents: Document[];
  onSelectDocument: (id: string) => void;
  onUpdateDocument: (updatedFields: Partial<Document>) => void;
}

export default function BlockEditor({
  document: activeDoc,
  documents,
  onSelectDocument,
  onUpdateDocument,
}: BlockEditorProps) {
  
  const [newlyCreatedBlockId, setNewlyCreatedBlockId] = useState<string | null>(null);
  const backlinks = useMemo(() => {
    if (!activeDoc) return [];
    return documents.filter((doc) => {
      if (doc.id === activeDoc.id) return false;
      return doc.blocks.some((block) => {
        return (
          block.content &&
          block.content.toLowerCase().includes(`[[${activeDoc.title.toLowerCase()}]]`)
        );
      });
    });
  }, [documents, activeDoc]);

  const wordCount = useMemo(() => {
    if (!activeDoc) return 0;
    return activeDoc.blocks.reduce((acc, block) => {
      const contentStr = block.content || '';
      // Approximate Chinese & English word bounds
      const count = contentStr.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, ' ').split(/\s+/).filter(Boolean).length;
      return acc + count;
    }, 0);
  }, [activeDoc]);

  const readTimeMinutes = useMemo(() => {
    return Math.max(1, Math.ceil(wordCount / 300)); // Average 300 words/minute
  }, [wordCount]);

  // Add a new block at the end of the document
  const appendBlockAtEnd = (type: BlockType) => {
    const newBlock: Block = {
      id: `block-${Date.now()}-${Math.random().toString(36).substr(2, 5)}`,
      type,
      content: '',
      properties: {},
    };

    setNewlyCreatedBlockId(newBlock.id);

    onUpdateDocument({
      blocks: [...activeDoc.blocks, newBlock],
    });
  };

  // Insert a block directly below a given block ID
  const insertBlockBelowIndex = (targetBlockId: string, type: BlockType) => {
    const targetIdx = activeDoc.blocks.findIndex((b) => b.id === targetBlockId);
    if (targetIdx === -1) return;

    const newBlock: Block = {
      id: `block-${Date.now()}-${Math.random().toString(36).substr(2, 5)}`,
      type,
      content: '',
      properties: {},
    };

    const blocksCopy = [...activeDoc.blocks];
    blocksCopy.splice(targetIdx + 1, 0, newBlock);

    setNewlyCreatedBlockId(newBlock.id);

    onUpdateDocument({
      blocks: blocksCopy,
    });
  };

  // Modify a single block's values
  const updateBlockContent = (blockId: string, updatedFields: Partial<Block>) => {
    const updatedBlocks = activeDoc.blocks.map((b) => {
      if (b.id === blockId) {
        return { ...b, ...updatedFields };
      }
      return b;
    });

    onUpdateDocument({
      blocks: updatedBlocks,
    });
  };

  // Delete a block
  const deleteBlockInline = (blockId: string, mergeContent?: string) => {
    const targetIdx = activeDoc.blocks.findIndex(b => b.id === blockId);
    let nextFocusId: string | null = null;
    let updatedBlocks = [...activeDoc.blocks];

    if (targetIdx > 0) {
      const prevBlock = updatedBlocks[targetIdx - 1];
      nextFocusId = prevBlock.id;
      
      if (mergeContent !== undefined) {
         updatedBlocks[targetIdx - 1] = {
           ...prevBlock,
           content: prevBlock.content + mergeContent
         };
      }
    } else if (targetIdx === 0 && activeDoc.blocks.length > 1) {
      nextFocusId = activeDoc.blocks[1].id;
    }

    // Keep at least one empty block if user clears everything
    updatedBlocks = updatedBlocks.filter((b) => b.id !== blockId);
    if (updatedBlocks.length === 0) {
      const fallbackId = `block-fallback-${Date.now()}`;
      updatedBlocks = [
        {
          id: fallbackId,
          type: 'text',
          content: '',
          properties: {},
        },
      ];
      nextFocusId = fallbackId;
    }

    if (nextFocusId) {
      setNewlyCreatedBlockId(nextFocusId);
    }

    onUpdateDocument({
      blocks: updatedBlocks,
    });
  };

  // Handle emoji quick collection
  const emojiOptions = ['📝', '🧪', '🎨', '📚', '📆', '🚀', '🧠', '💡', '🏁', '⭐'];

  return (
    <div className="flex flex-col h-full bg-transparent overflow-hidden">
      
      {/* 1. Minimal Word-style Toolbar Ribbon Area */}
      <div className="flex items-center gap-1 py-2 shrink-0 select-none border-b border-slate-100 dark:border-slate-800">
        {[
          { type: 'text', icon: MessageSquare, title: '插入文本' },
          { type: 'heading-2', icon: Heading2, title: '插入副标题' },
          { type: 'table', icon: TableIcon, title: '插入表格' },
          { type: 'canvas', icon: Palette, title: '插入白板' },
          { type: 'code', icon: Code, title: '插入代码' },
          { type: 'html-render', icon: FileCode, title: '插入HTML沙盒' },
          { type: 'toggle', icon: ChevronDown, title: '插入折叠块' },
        ].map((op) => {
          const IconComp = op.icon;
          return (
            <button
              key={op.type}
              onClick={() => appendBlockAtEnd(op.type as BlockType)}
              className="cursor-pointer p-1.5 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800 dark:text-slate-400 rounded transition-colors"
              title={op.title}
            >
              <IconComp className="w-4 h-4" />
            </button>
          );
        })}
        <div className="h-4 w-[1px] bg-slate-200 dark:bg-slate-800 mx-2" />
        <div className="text-xs text-slate-400 dark:text-slate-500">
          词数: {wordCount} &nbsp;|&nbsp; 预计阅读 {readTimeMinutes} 分钟
        </div>
      </div>

      {/* 2. Editor Canvas Area */}
      <div className="flex-1 overflow-y-auto px-4 md:px-12 py-10 space-y-6 bg-white dark:bg-[#0d0d0d] select-text">
        {/* Document Title header */}
        <div className="pb-4">
          <input
            type="text"
            value={activeDoc.title}
            onChange={(e) => onUpdateDocument({ title: e.target.value })}
            placeholder="文档标题"
            className="text-3xl font-bold text-slate-800 dark:text-slate-200 bg-transparent border-none focus:outline-none w-full placeholder-slate-300"
          />
        </div>

        {/* Blocks rendering stream */}
        <div className="space-y-1 min-h-[50vh]" id="blocks-container">
          {activeDoc.blocks.map((block) => (
            <BlockItem
              key={block.id}
              block={block}
              documents={documents}
              onUpdateBlock={(fields) => updateBlockContent(block.id, fields)}
              onDeleteBlock={(content) => deleteBlockInline(block.id, content)}
              onNavigateToDoc={onSelectDocument}
              onInsertBlockBelow={(type) => insertBlockBelowIndex(block.id, type)}
              autoFocus={newlyCreatedBlockId === block.id}
            />
          ))}
        </div>

        {/* Dynamic Backlinks View Section - Extremely minimal */}
        {backlinks.length > 0 && (
          <div className="pt-10 border-t border-slate-100 dark:border-slate-800 mt-10" id="backlinks-section">
            <div className="text-xs font-semibold text-slate-400 mb-2">引用链接 ({backlinks.length})</div>
            <div className="flex flex-wrap gap-2">
              {backlinks.map((link) => (
                <span
                  key={link.id}
                  onClick={() => onSelectDocument(link.id)}
                  className="cursor-pointer px-2 py-1 bg-slate-50 dark:bg-slate-800 text-slate-600 dark:text-slate-400 rounded text-xs transition-colors hover:text-indigo-500"
                >
                  {link.title}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
