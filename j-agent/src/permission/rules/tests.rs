use super::*;

#[test]
fn test_wildcard_rule() {
    assert!(matches_rule("*", "Bash", "{}"));
    assert!(matches_rule("*", "Read", "{}"));
}

#[test]
fn test_tool_name_rule() {
    assert!(matches_rule("Read", "Read", "{}"));
    assert!(matches_rule("Grep", "Grep", "{}"));
    assert!(!matches_rule("Read", "Write", "{}"));
}

#[test]
fn test_bash_command_prefix() {
    let args = r#"{"command": "cargo build --release"}"#;
    assert!(matches_rule("Bash(cargo build:*)", "Bash", args));
    assert!(!matches_rule("Bash(cargo test:*)", "Bash", args));

    let args2 = r#"{"command": "ls -la"}"#;
    assert!(matches_rule("Bash(ls:*)", "Bash", args2));

    // 精确匹配（无参数）
    let args3 = r#"{"command": "cargo fmt"}"#;
    assert!(matches_rule("Bash(cargo fmt:*)", "Bash", args3));
}

#[test]
fn test_path_rule() {
    let args = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
    assert!(matches_rule(
        "Write(path:/Users/jack/projects/*)",
        "Write",
        args
    ));
    assert!(!matches_rule("Write(path:/Users/other/*)", "Write", args));
}

#[test]
fn test_domain_rule() {
    let args = r#"{"url": "https://docs.rs/serde/latest"}"#;
    assert!(matches_rule("WebFetch(domain:docs.rs)", "WebFetch", args));
    assert!(!matches_rule(
        "WebFetch(domain:github.com)",
        "WebFetch",
        args
    ));
}

#[test]
fn test_deny_priority() {
    let config = JcliConfig {
        permissions: PermissionConfig {
            allow_all: false,
            allow: vec!["Bash(cargo:*)".to_string()],
            deny: vec!["Bash(cargo build:*)".to_string()],
        },
    };
    // cargo test 被 allow 的 cargo:* 覆盖
    let args_test = r#"{"command": "cargo test"}"#;
    assert!(config.is_allowed("Bash", args_test));

    // cargo build 被 deny 拦截
    let args_build = r#"{"command": "cargo build"}"#;
    assert!(!config.is_allowed("Bash", args_build));
    assert!(config.is_denied("Bash", args_build));
}

#[test]
fn test_allow_all() {
    let config = JcliConfig {
        permissions: PermissionConfig {
            allow_all: true,
            allow: vec![],
            deny: vec![],
        },
    };
    assert!(config.is_allowed("Bash", r#"{"command": "rm -rf /"}"#));

    // deny 仍然优先
    let config2 = JcliConfig {
        permissions: PermissionConfig {
            allow_all: true,
            allow: vec![],
            deny: vec!["Bash(rm -rf:*)".to_string()],
        },
    };
    assert!(!config2.is_allowed("Bash", r#"{"command": "rm -rf /"}"#));
}

#[test]
fn test_url_domain_matching() {
    assert!(url_matches_domain("https://docs.rs/foo", "docs.rs"));
    assert!(url_matches_domain(
        "https://api.github.com/repos",
        "github.com"
    ));
    assert!(!url_matches_domain("https://evil.com/docs.rs", "docs.rs"));
}

#[test]
fn test_generate_allow_rule_bash() {
    let args = r#"{"command": "cargo build --release"}"#;
    assert_eq!(generate_allow_rule("Bash", args), "Bash(cargo build:*)");

    let args2 = r#"{"command": "ls -la"}"#;
    assert_eq!(generate_allow_rule("Bash", args2), "Bash(ls -la:*)");

    let args3 = r#"{"command": "ls"}"#;
    assert_eq!(generate_allow_rule("Bash", args3), "Bash(ls:*)");
}

#[test]
fn test_generate_allow_rule_write() {
    let args = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
    assert_eq!(
        generate_allow_rule("Write", args),
        "Write(path:/Users/jack/projects/foo/*)"
    );

    let args2 = r#"{"file_path": "/Users/jack/projects/foo/src/main.rs"}"#;
    assert_eq!(
        generate_allow_rule("Edit", args2),
        "Edit(path:/Users/jack/projects/foo/src/*)"
    );
}

#[test]
fn test_generate_allow_rule_webfetch() {
    let args = r#"{"url": "https://docs.rs/serde/latest"}"#;
    assert_eq!(
        generate_allow_rule("WebFetch", args),
        "WebFetch(domain:docs.rs)"
    );
}

#[test]
fn test_generate_allow_rule_other() {
    assert_eq!(generate_allow_rule("Read", "{}"), "Read");
    assert_eq!(generate_allow_rule("Grep", "{}"), "Grep");
    assert_eq!(generate_allow_rule("Glob", "{}"), "Glob");
}

// ========== Regex 匹配测试 ==========

#[test]
fn test_regex_bash_command() {
    // Bash(/^cargo (build|test)/) 匹配 cargo build 和 cargo test
    let args_build = r#"{"command": "cargo build --release"}"#;
    let args_test = r#"{"command": "cargo test --lib"}"#;
    let args_run = r#"{"command": "cargo run"}"#;
    assert!(matches_rule(
        "Bash(/^cargo (build|test)/)",
        "Bash",
        args_build
    ));
    assert!(matches_rule(
        "Bash(/^cargo (build|test)/)",
        "Bash",
        args_test
    ));
    assert!(!matches_rule(
        "Bash(/^cargo (build|test)/)",
        "Bash",
        args_run
    ));
}

#[test]
fn test_regex_path_pattern() {
    // Write(path:/\.rs$/) 匹配所有 .rs 文件
    let args_rs = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
    let args_ts = r#"{"file_path": "/Users/jack/projects/foo/bar.ts"}"#;
    assert!(matches_rule("Write(path:/\\.rs$/)", "Write", args_rs));
    assert!(!matches_rule("Write(path:/\\.rs$/)", "Write", args_ts));
}

#[test]
fn test_regex_domain_pattern() {
    // WebFetch(domain:/.*\.google\.com$/) 匹配所有 google.com 子域名
    let args_google = r#"{"url": "https://maps.google.com/api"}"#;
    let args_docs = r#"{"url": "https://docs.google.com/document"}"#;
    let args_other = r#"{"url": "https://evil.com/google.com"}"#;
    assert!(matches_rule(
        "WebFetch(domain:/.*\\.google\\.com$/)",
        "WebFetch",
        args_google
    ));
    assert!(matches_rule(
        "WebFetch(domain:/.*\\.google\\.com$/)",
        "WebFetch",
        args_docs
    ));
    assert!(!matches_rule(
        "WebFetch(domain:/.*\\.google\\.com$/)",
        "WebFetch",
        args_other
    ));
}

#[test]
fn test_regex_invalid_pattern() {
    // 无效的 regex 不匹配
    let args = r#"{"command": "anything"}"#;
    assert!(!matches_rule("Bash(/[invalid/)", "Bash", args));
    // 空 regex 不匹配
    assert!(!matches_rule("Bash(//)", "Bash", args));
}
