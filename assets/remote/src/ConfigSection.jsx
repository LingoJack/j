import { useState } from 'react'

const TABS = [
  { key: 'model', label: '模型' },
  { key: 'global', label: '全局' },
  { key: 'tools', label: '工具' },
  { key: 'skills', label: '技能' },
]

export default function ConfigSection({ configData, modelList, themeList, send, onCollapse }) {
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
      send({ type: 'config_edit_submit', value: editValue })
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
      {/* Section header */}
      <div className="sidebar-section-header">
        <span className="font-semibold text-[13px]">配置</span>
        <button
          className="text-fg3 hover:text-fg p-1 rounded-md hover:bg-bg3 transition-colors"
          onClick={onCollapse}
          title="收起侧边栏"
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </div>

      {/* Tab bar */}
      <div className="flex border-b border-border shrink-0">
        {TABS.map(t => (
          <button
            key={t.key}
            className={`flex-1 px-2 py-2 text-[11px] font-medium transition-colors ${activeTab === t.key ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-fg3 hover:text-fg'}`}
            onClick={() => requestTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {/* Model tab */}
        {activeTab === 'model' && (
          <div>
            {/* Model list */}
            {modelList && modelList.models && modelList.models.length > 0 && (
              <div className="px-3 py-2">
                <div className="text-[11px] text-fg3 mb-2">切换模型</div>
                {modelList.models.map((m, i) => (
                  <button
                    key={i}
                    className={`w-full text-left px-3 py-2 rounded-lg mb-1 text-[12px] transition-colors ${i === modelList.active_index ? 'bg-accent/15 text-accent border border-accent/30' : 'bg-bg3/50 text-fg hover:bg-bg3 border border-transparent'}`}
                    onClick={() => selectModel(i)}
                  >
                    <div className="font-medium">{m.name}</div>
                    <div className="text-[10px] text-fg3 mt-0.5">{m.model} · {m.provider}</div>
                  </button>
                ))}
              </div>
            )}

            {/* Theme list */}
            {themeList && themeList.themes && themeList.themes.length > 0 && (
              <div className="px-3 py-2 border-t border-border">
                <div className="text-[11px] text-fg3 mb-2">切换主题</div>
                {themeList.themes.map((t, i) => (
                  <button
                    key={i}
                    className={`w-full text-left px-3 py-2 rounded-lg mb-1 text-[12px] transition-colors ${i === themeList.active_index ? 'bg-accent/15 text-accent border border-accent/30' : 'bg-bg3/50 text-fg hover:bg-bg3 border border-transparent'}`}
                    onClick={() => selectTheme(i)}
                  >
                    {t.display_name}
                  </button>
                ))}
              </div>
            )}

            {(!modelList?.models?.length) && (
              <div className="px-4 py-6 text-center text-fg3 text-[12px]">点击上方 tab 加载配置</div>
            )}
          </div>
        )}

        {/* Global / Tools / Skills tabs - use config_data fields */}
        {activeTab !== 'model' && (
          <div className="px-3 py-2">
            {fields.length === 0 ? (
              <div className="text-center text-fg3 text-[12px] py-4">加载中...</div>
            ) : (
              fields.map((f, i) => (
                <div key={f.key} className="mb-2">
                  {f.field_type === 'bool' ? (
                    <div
                      className="flex items-center justify-between px-3 py-2.5 rounded-lg bg-bg3/50 cursor-pointer hover:bg-bg3 transition-colors"
                      onClick={() => toggleField(i)}
                    >
                      <span className="text-[12px] text-fg">{f.label}</span>
                      <span className={`w-9 h-5 rounded-full transition-colors relative ${f.value === 'true' ? 'bg-accent' : 'bg-border'}`}>
                        <span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${f.value === 'true' ? 'left-[18px]' : 'left-0.5'}`} />
                      </span>
                    </div>
                  ) : editingField === f.key ? (
                    <div className="px-3 py-2 rounded-lg bg-bg3/50">
                      <div className="text-[11px] text-fg3 mb-1">{f.label}</div>
                      <div className="flex gap-1">
                        <input
                          className="flex-1 bg-bg border border-border rounded px-2 py-1 text-[12px] text-fg outline-none focus:border-accent"
                          value={editValue}
                          onChange={e => setEditValue(e.target.value)}
                          onKeyDown={e => e.key === 'Enter' && submitEdit()}
                          autoFocus
                        />
                        <button className="px-2 py-1 rounded bg-accent text-white text-[11px]" onClick={submitEdit}>确定</button>
                        <button className="px-2 py-1 rounded bg-bg3 text-fg3 text-[11px]" onClick={() => setEditingField(null)}>取消</button>
                      </div>
                    </div>
                  ) : (
                    <div
                      className={`px-3 py-2 rounded-lg bg-bg3/50 ${f.editable ? 'cursor-pointer hover:bg-bg3 transition-colors' : ''}`}
                      onClick={() => f.editable && startEdit(f)}
                    >
                      <div className="text-[11px] text-fg3">{f.label}</div>
                      <div className="text-[12px] text-fg mt-0.5">{f.value || '(空)'}</div>
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  )
}
