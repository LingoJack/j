import { useState } from 'react'

const TABS = [
  { key: 'model', label: '模型' },
  { key: 'global', label: '全局' },
  { key: 'tools', label: '工具' },
  { key: 'skills', label: '技能' },
]

export default function ConfigSection({ configData, modelList, themeList, send, onBack }) {
  const [activeTab, setActiveTab] = useState('model')
  const [editingField, setEditingField] = useState(null)
  const [editValue, setEditValue] = useState('')

  const requestTab = (tab) => {
    setActiveTab(tab)
    send({ type: 'request_config', tab })
  }

  const startEdit = (field) => {
    setEditingField(field.key)
    setEditValue(field.value)
  }

  const submitEdit = () => {
    if (editingField) {
      send({ type: 'config_edit_submit', field: editingField, value: editValue })
      setEditingField(null)
      setEditValue('')
    }
  }

  const toggleField = (index) => {
    send({ type: 'config_toggle', index })
  }

  const selectModel = (index) => {
    send({ type: 'select_model', index })
  }

  const selectTheme = (index) => {
    send({ type: 'select_theme', index })
  }

  const fields = configData?.fields || []

  return (
    <div className="flex flex-col h-full">
      {/* Header bar */}
      <div className="flex items-center gap-2 px-3 bg-bg2 border-b border-border shrink-0 h-[34px]">
        <button
          className="text-fg3 hover:text-fg active:text-accent active:scale-[0.92] text-[11px] px-1.5 py-0.5 rounded hover:bg-bg3 active:bg-bg3 transition-all duration-100 select-none"
          onClick={onBack}
        >← 返回</button>
        <div className="w-px h-4 bg-border" />
        <svg className="w-3.5 h-3.5 text-fg3 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        <span className="text-fg text-[12px] font-medium">配置</span>
      </div>

      {/* Tab bar */}
      <div className="flex border-b border-border shrink-0 bg-bg2/50">
        {TABS.map(t => (
          <button
            key={t.key}
            className={`px-5 py-2.5 text-[12px] font-medium transition-all duration-100 select-none ${activeTab === t.key ? 'text-fg border-b-2 border-fg bg-fg/5' : 'text-fg3 hover:text-fg active:text-fg'}`}
            onClick={() => requestTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Content - wider layout */}
      <div className="flex-1 overflow-y-auto">
        {/* Model tab */}
        {activeTab === 'model' && (
          <div className="max-w-2xl mx-auto p-4">
            {/* Model list */}
            {modelList && modelList.models && modelList.models.length > 0 && (
              <div className="mb-4">
                <div className="text-[11px] text-fg3 mb-2 uppercase tracking-wider">切换模型</div>
                <div className="grid gap-2">
                  {modelList.models.map((m, i) => (
                    <button
                      key={i}
                      className={`w-full text-left px-4 py-3 rounded-lg text-[13px] transition-all duration-100 select-none active:scale-[0.98] ${i === modelList.active_index ? 'bg-fg/10 text-fg border border-fg/20 ring-1 ring-fg/10' : 'bg-bg3/30 text-fg hover:bg-bg3 active:bg-border border border-transparent'}`}
                      onClick={() => selectModel(i)}
                    >
                      <div className="font-medium">{m.name}</div>
                      <div className="text-[11px] text-fg3 mt-0.5">{m.model} · {m.provider}</div>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Theme list */}
            {themeList && themeList.themes && themeList.themes.length > 0 && (
              <div className="mb-4 pt-4 border-t border-border">
                <div className="text-[11px] text-fg3 mb-2 uppercase tracking-wider">TUI 主题</div>
                <div className="grid gap-2">
                  {themeList.themes.map((t, i) => (
                    <button
                      key={i}
                      className={`w-full text-left px-4 py-3 rounded-lg text-[13px] transition-all duration-100 select-none active:scale-[0.98] ${i === themeList.active_index ? 'bg-fg/10 text-fg border border-fg/20 ring-1 ring-fg/10' : 'bg-bg3/30 text-fg hover:bg-bg3 active:bg-border border border-transparent'}`}
                      onClick={() => selectTheme(i)}
                    >
                      {t.display_name}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {(!modelList?.models?.length) && (
              <div className="py-8 text-center text-fg3 text-[12px]">加载中...</div>
            )}
          </div>
        )}

        {/* Global / Tools / Skills tabs */}
        {activeTab !== 'model' && (
          <div className="max-w-2xl mx-auto p-4">
            {fields.length === 0 ? (
              <div className="text-center text-fg3 text-[12px] py-8">加载中...</div>
            ) : (
              <div className="grid gap-2">
                {fields.map((f, i) => (
                  <div key={f.key}>
                    {f.field_type === 'bool' ? (
                      <div
                        className="flex items-center justify-between px-4 py-3 rounded-lg bg-bg3/30 cursor-pointer hover:bg-bg3 active:bg-border active:scale-[0.99] transition-all duration-100 select-none"
                        onClick={() => toggleField(i)}
                      >
                        <span className="text-[13px] text-fg">{f.label}</span>
                        <span className={`w-10 h-[22px] rounded-full transition-colors relative ${f.value === 'true' ? 'bg-ok' : 'bg-border-light'}`}>
                          <span className={`absolute top-[3px] w-4 h-4 rounded-full bg-white transition-transform shadow-sm ${f.value === 'true' ? 'left-[22px]' : 'left-[3px]'}`} />
                        </span>
                      </div>
                    ) : editingField === f.key ? (
                      <div className="px-4 py-3 rounded-lg bg-bg3/30">
                        <div className="text-[11px] text-fg3 mb-1.5">{f.label}</div>
                        <div className="flex gap-2">
                          <input
                            className="flex-1 bg-bg border border-border rounded-md px-3 py-1.5 text-[13px] text-fg outline-none focus:border-fg/50 transition-colors"
                            value={editValue}
                            onChange={e => setEditValue(e.target.value)}
                            onKeyDown={e => e.key === 'Enter' && submitEdit()}
                            autoFocus
                          />
                          <button className="px-3 py-1.5 rounded-md bg-fg text-bg text-[12px] font-medium active:scale-[0.95] transition-transform duration-100 select-none" onClick={submitEdit}>确定</button>
                          <button className="px-3 py-1.5 rounded-md bg-bg3 text-fg text-[12px] active:scale-[0.95] transition-transform duration-100 select-none" onClick={() => setEditingField(null)}>取消</button>
                        </div>
                      </div>
                    ) : (
                      <div
                        className={`px-4 py-3 rounded-lg bg-bg3/30 transition-all duration-100 select-none ${f.editable ? 'cursor-pointer hover:bg-bg3 active:bg-border active:scale-[0.99]' : ''}`}
                        onClick={() => f.editable && startEdit(f)}
                      >
                        <div className="text-[11px] text-fg3">{f.label}</div>
                        <div className="text-[13px] text-fg mt-0.5">{f.value || '(空)'}</div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
