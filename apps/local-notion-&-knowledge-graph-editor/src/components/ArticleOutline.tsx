import React from 'react';
import { Document, Block } from '../types';
import { AlignLeft, ChevronRight, Hash } from 'lucide-react';

interface ArticleOutlineProps {
  document: Document;
}

export default function ArticleOutline({ document }: ArticleOutlineProps) {
  // Extract heading blocks
  const headings = document.blocks.filter(
    (block) =>
      block.type === 'heading-1' || block.type === 'heading-2' || block.type === 'heading-3'
  );

  const handleHeadingClick = (blockId: string) => {
    const el = window.document.getElementById(`block-row-${blockId}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      // Add a brief subtle flash to draw focus
      el.classList.add('bg-indigo-500/10');
      setTimeout(() => {
        el.classList.remove('bg-indigo-500/10');
      }, 1000);
    }
  };

  return (
    <div className="w-full h-full flex flex-col p-4 bg-transparent select-none animate-in fade-in duration-200">
      <div className="flex items-center gap-2 mb-4 text-xs font-semibold text-slate-400">
        <AlignLeft className="w-3.5 h-3.5" />
        <span>文章大纲</span>
      </div>

      <div className="flex-1 overflow-y-auto">
        {headings.length === 0 ? (
          <div className="py-10 text-center">
            <span className="text-xs text-slate-400">暂无标题</span>
          </div>
        ) : (
          <div className="space-y-3 text-[11px]">
            {headings.map((heading) => {
              // Indent according to heading type
              let indentClass = 'font-medium text-slate-700 dark:text-slate-300';
              if (heading.type === 'heading-2') {
                indentClass = 'pl-3 text-slate-500 dark:text-slate-400';
              } else if (heading.type === 'heading-3') {
                indentClass = 'pl-6 text-slate-400 dark:text-slate-500';
              }

              return (
                <button
                  key={heading.id}
                  onClick={() => handleHeadingClick(heading.id)}
                  className={`cursor-pointer w-full text-left truncate flex items-center transition-colors hover:text-slate-900 dark:hover:text-slate-100 ${indentClass}`}
                  title={heading.content || '未命名'}
                >
                  <span className="truncate">
                    {heading.content ? heading.content : <span className="opacity-50">未命名</span>}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
