pub mod config;
pub mod persist;
pub mod session;
pub mod types;

pub use config::{
    AgentConfig, ModelProvider, agent_config_path, agent_data_dir, hooks_config_path,
    load_agent_config, load_memory, load_soul, load_style, load_system_prompt, memory_path,
    save_agent_config, save_memory, save_soul, save_style, save_system_prompt, soul_path,
    system_prompt_path,
};
pub use persist::{
    PlanStatePersist, SandboxStatePersist, SessionHookPersist, SubAgentSnapshotPersist,
    TeammateSnapshotPersist, load_hooks_state, load_plan_state, load_sandbox_state,
    load_skills_state, load_tasks_state, load_teammates_state, load_todos_state, sanitize_filename,
    save_hooks_state, save_plan_state, save_sandbox_state, save_skills_state, save_subagents_state,
    save_tasks_state, save_teammates_state, save_todos_state,
};
pub use session::{
    SessionMeta, SessionPaths, append_event_to_path, append_session_event, delete_session,
    find_latest_session_id, generate_session_id, list_sessions, load_session,
    read_transcript_with_timestamps, session_file_path, sessions_dir,
};
pub use types::{
    ChatMessage, ChatSession, DisplayType, ImageData, MessageRole, SessionEvent, ToolCallItem,
};

#[cfg(test)]
mod tests {
    use super::session::{SessionMetaFile, load_session_meta_file, save_session_meta_file};
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TempDataDir {
        root: PathBuf,
        prev: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TempDataDir {
        fn new() -> Self {
            let lock = test_lock().lock().unwrap_or_else(|e| e.into_inner());
            let pid = std::process::id();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("jcli-storage-test-{}-{}", pid, nanos));
            let _ = fs::create_dir_all(&root);
            let prev = std::env::var("J_DATA_PATH").ok();
            // SAFETY: 测试环境下单线程运行，通过 Mutex 保证互斥访问，
            // 修改 J_DATA_PATH 环境变量不会影响其他并发代码。
            unsafe {
                std::env::set_var("J_DATA_PATH", &root);
            }
            Self {
                root,
                prev,
                _lock: lock,
            }
        }
    }

    impl Drop for TempDataDir {
        fn drop(&mut self) {
            // SAFETY: 测试环境下单线程运行，由 TempDataDir 的 Mutex 保证互斥，
            // 仅在 drop 时恢复/清除测试环境变量，不会造成数据竞争。
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("J_DATA_PATH", v),
                    None => std::env::remove_var("J_DATA_PATH"),
                }
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn session_paths_construction() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("abc");
        assert_eq!(paths.id(), "abc");
        assert_eq!(paths.dir().file_name().unwrap(), "abc");
        assert_eq!(paths.transcript().file_name().unwrap(), "transcript.jsonl");
        assert_eq!(paths.meta_file().file_name().unwrap(), "session.json");
        assert!(paths.transcript().parent().unwrap().ends_with("abc"));
    }

    #[test]
    fn append_event_writes_to_new_layout() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("append-id");

        let msg = ChatMessage::text(MessageRole::User, "hello".to_string());
        assert!(append_session_event("append-id", &SessionEvent::msg(msg)));

        assert!(paths.transcript().exists());
    }

    #[test]
    fn load_session_round_trip() {
        let _tmp = TempDataDir::new();

        let msg = ChatMessage::text(MessageRole::User, "round trip test");
        assert!(append_session_event("rt-id", &SessionEvent::msg(msg)));

        let session = load_session("rt-id");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "round trip test");
    }

    #[test]
    fn list_sessions_finds_sessions() {
        let _tmp = TempDataDir::new();

        let paths = SessionPaths::new("ls-test");
        paths.ensure_dir().unwrap();
        let msg = ChatMessage::text(MessageRole::User, "list test");
        let line = serde_json::to_string(&SessionEvent::msg(msg)).unwrap();
        fs::write(paths.transcript(), format!("{}\n", line)).unwrap();

        let metas = list_sessions();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, "ls-test");
    }

    #[test]
    fn delete_session_removes_dir() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("del-id");

        paths.ensure_dir().unwrap();
        fs::write(paths.transcript(), b"").unwrap();

        assert!(delete_session("del-id"));
        assert!(!paths.dir().exists());
    }

    #[test]
    fn session_meta_file_round_trip() {
        let _tmp = TempDataDir::new();
        let meta = SessionMetaFile {
            id: "meta-test".to_string(),
            title: "你好世界".to_string(),
            message_count: 5,
            created_at: 1000,
            updated_at: 2000,
            model: Some("gpt-4o".to_string()),
        };
        assert!(save_session_meta_file(&meta));
        let loaded = load_session_meta_file("meta-test").expect("should load");
        assert_eq!(loaded.id, "meta-test");
        assert_eq!(loaded.title, "你好世界");
        assert_eq!(loaded.message_count, 5);
        assert_eq!(loaded.created_at, 1000);
        assert_eq!(loaded.updated_at, 2000);
        assert_eq!(loaded.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn append_event_updates_meta() {
        let _tmp = TempDataDir::new();
        let msg1 = ChatMessage::text(MessageRole::User, "hello world");
        assert!(append_session_event("meta-upd", &SessionEvent::msg(msg1)));

        let meta = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta.id, "meta-upd");
        assert_eq!(meta.message_count, 1);
        assert_eq!(meta.title, "hello world");
        assert!(meta.updated_at > 0);

        let msg2 = ChatMessage::text(MessageRole::Assistant, "hi there");
        assert!(append_session_event("meta-upd", &SessionEvent::msg(msg2)));

        let meta2 = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta2.message_count, 2);
        assert_eq!(meta2.title, "hello world");

        assert!(append_session_event("meta-upd", &SessionEvent::Clear));
        let meta3 = load_session_meta_file("meta-upd").expect("meta should exist");
        assert_eq!(meta3.message_count, 0);
    }

    #[test]
    fn list_sessions_lazy_generates_meta() {
        let _tmp = TempDataDir::new();

        let paths = SessionPaths::new("lazy-gen");
        paths.ensure_dir().unwrap();
        let msg = ChatMessage::text(MessageRole::User, "lazy generation test");
        let line = serde_json::to_string(&SessionEvent::msg(msg)).unwrap();
        fs::write(paths.transcript(), format!("{}\n", line)).unwrap();

        assert!(!paths.meta_file().exists());

        let sessions = list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "lazy-gen");
        assert_eq!(sessions[0].message_count, 1);
        assert_eq!(sessions[0].title.as_deref(), Some("lazy generation test"));

        assert!(paths.meta_file().exists());
    }

    #[test]
    fn session_paths_transcripts_dir() {
        let _tmp = TempDataDir::new();
        let paths = SessionPaths::new("tx-test");
        assert!(paths.transcripts_dir().ends_with(".transcripts"));
        assert_eq!(paths.transcripts_dir().parent().unwrap(), paths.dir());
    }
}
