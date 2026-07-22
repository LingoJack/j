//! Lark/Feishu report sync.
//!
//! The backend uses lark-cli with the user's identity:
//! - push: append only when the local Markdown report extends the remote doc; use -f to overwrite.
//! - pull: update the local report only when the remote doc extends it; use -f to overwrite.

use crate::config::YamlConfig;
use crate::constants::{config_key, section};
use crate::{error, info};
use chrono::Local;
use serde_json::Value;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};
use url::Url;

const LARK_CLI_PACKAGE: &str = "@larksuite/cli";

pub fn handle_push(args: &[String], config: &mut YamlConfig) {
    let options = parse_push_options(args);
    if options.ignored_message.is_some() {
        info!("Lark report backend ignores commit messages.");
    }

    if ensure_lark_ready(config) && push_report_to_lark(config, options.force) {
        info!("Report pushed to the configured Lark document.");
    }
}

pub fn handle_pull(args: &[String], config: &mut YamlConfig) {
    let options = parse_force_options(args);
    if options.ignored_message.is_some() {
        info!("Lark report pull ignores extra arguments except -f/--force.");
    }

    if ensure_lark_ready(config) && pull_report_from_lark(config, options.force) {
        info!("Report pulled from the configured Lark document.");
    }
}

pub fn ensure_lark_ready(config: &mut YamlConfig) -> bool {
    if !ensure_lark_cli() {
        return false;
    }
    if !ensure_lark_auth() {
        return false;
    }
    ensure_lark_document(config)
}

fn ensure_lark_cli() -> bool {
    if command_exists("lark-cli") {
        info!("lark-cli is installed and executable.");
        return true;
    }

    info!(
        "lark-cli not found. Installing {} globally with npm...",
        LARK_CLI_PACKAGE
    );
    let status = Command::new("npm")
        .args(["install", "-g", LARK_CLI_PACKAGE])
        .status();

    match status {
        Ok(status) if status.success() => {
            info!("lark-cli installed successfully.");
            true
        }
        Ok(status) => {
            error!("failed to install lark-cli, exit status: {}", status);
            false
        }
        Err(e) => {
            error!("failed to run npm install: {}", e);
            false
        }
    }
}

fn ensure_lark_auth() -> bool {
    info!("Checking lark-cli user authentication with `lark-cli doctor`.");
    let doctor = Command::new("lark-cli").arg("doctor").output();
    match doctor {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() && user_identity_ready(&stdout) {
                info!("lark-cli user authentication is available.");
                return true;
            }
            info!(
                "lark-cli doctor did not report a ready user identity, exit status: {}",
                output.status
            );
            if !stdout.trim().is_empty() {
                info!("{}", stdout.trim());
            }
            if !stderr.trim().is_empty() {
                info!("{}", stderr.trim());
            }
        }
        Err(e) => {
            error!("failed to run lark-cli doctor: {}", e);
            return false;
        }
    }

    info!("Starting lark-cli login for docs/drive permissions.");
    let status = Command::new("lark-cli")
        .args(["auth", "login", "--domain", "docs,drive,markdown,wiki"])
        .status();

    match status {
        Ok(status) if status.success() => true,
        Ok(status) => {
            error!("lark-cli auth login failed, exit status: {}", status);
            false
        }
        Err(e) => {
            error!("failed to run lark-cli auth login: {}", e);
            false
        }
    }
}

fn user_identity_ready(doctor_stdout: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(doctor_stdout) else {
        return false;
    };

    value
        .get("checks")
        .and_then(Value::as_array)
        .is_some_and(|checks| {
            checks.iter().any(|check| {
                check.get("name").and_then(Value::as_str) == Some("user_identity")
                    && check.get("status").and_then(Value::as_str) == Some("pass")
            })
        })
}

fn ensure_lark_document(config: &mut YamlConfig) -> bool {
    let existing_url = config
        .get_property(section::REPORT, config_key::LARK_DOC_URL)
        .cloned()
        .unwrap_or_default();

    let doc_url = if existing_url.trim().is_empty() {
        match prompt_line("Lark/Feishu document URL for report sync: ") {
            Some(url) if !url.trim().is_empty() => url.trim().to_string(),
            _ => {
                error!("Lark document URL is required.");
                return false;
            }
        }
    } else if has_lark_document_metadata(config) {
        let title = config
            .get_property(section::REPORT, config_key::LARK_DOC_TITLE)
            .map(String::as_str)
            .unwrap_or_default();
        let doc_type = config
            .get_property(section::REPORT, config_key::LARK_DOC_TYPE)
            .map(String::as_str)
            .unwrap_or_default();
        info!(
            "Using configured Lark document: {} ({})",
            empty_as_unknown(title),
            empty_as_unknown(doc_type)
        );
        return true;
    } else {
        info!("Lark document URL is configured but metadata is incomplete.");
        info!(
            "Re-inspecting configured Lark document URL: {}",
            existing_url
        );
        existing_url
    };

    let Some(doc) = inspect_lark_document(&doc_url) else {
        return false;
    };

    save_lark_document(config, &doc_url, &doc)
}

fn push_report_to_lark(config: &YamlConfig, force: bool) -> bool {
    let Some(doc_ref) = lark_document_ref(config) else {
        error!("Lark document is not configured.");
        return false;
    };

    let report_path = match super::io::get_report_path(config) {
        Some(path) => path,
        None => return false,
    };

    let content = match fs::read_to_string(&report_path) {
        Ok(content) => content,
        Err(e) => {
            error!("failed to read report file {}: {}", report_path, e);
            return false;
        }
    };

    if content.trim().is_empty() {
        info!("Local report file is empty; pushing an empty document body.");
    }

    info!("Pushing local report to Lark document.");
    info!("  source: {}", report_path);
    info!("  target: {}", lark_document_display(config));

    if force {
        info!("Force push enabled; overwriting the configured Lark document.");
        let Some(remote) = fetch_lark_document_markdown(&doc_ref) else {
            return false;
        };
        if let Err(e) = backup_content_to_lark_child(
            config,
            BackupKind::RemoteBeforePush {
                revision_id: remote.revision_id,
            },
            &remote.content,
        ) {
            error!(
                "failed to backup remote Lark document before force push: {}",
                e
            );
            return false;
        }
        return overwrite_lark_document(&doc_ref, &content);
    }

    let Some(remote) = fetch_lark_document_markdown(&doc_ref) else {
        return false;
    };
    let decision = decide_push_mode(&remote.content, &content);
    match decision {
        PushDecision::NoChange => {
            info!("Remote Lark document is already up to date.");
            true
        }
        PushDecision::Append { tail } => {
            info!(
                "Safe append-only push detected (remote revision {}, appending {} bytes).",
                remote
                    .revision_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
                tail.len()
            );
            append_lark_document(&doc_ref, tail)
        }
        PushDecision::Destructive => {
            error!(
                "Lark push aborted: local report is not an append-only extension of the remote document."
            );
            error!(
                "Run `j reportctl push -f` to overwrite the configured Lark document intentionally."
            );
            false
        }
    }
}

fn overwrite_lark_document(doc_ref: &str, content: &str) -> bool {
    let Some(output) = run_lark_with_stdin(
        &[
            "docs",
            "+update",
            "--doc",
            doc_ref,
            "--command",
            "overwrite",
            "--doc-format",
            "markdown",
            "--content",
            "-",
            "--json",
        ],
        content.as_bytes(),
    ) else {
        return false;
    };

    if !output.status.success() {
        log_lark_failure("lark-cli docs +update failed", &output);
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(summary) = parse_lark_update_result(&stdout) {
        info!(
            "Lark document updated: result={}, revision={}",
            empty_as_unknown(&summary.result),
            summary
                .revision_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "(unknown)".to_string())
        );
    } else if !stdout.trim().is_empty() {
        info!("Lark document updated.");
    }

    true
}

fn append_lark_document(doc_ref: &str, content: &str) -> bool {
    if content.is_empty() {
        info!("No appended content to push.");
        return true;
    }

    let Some(output) = run_lark_with_stdin(
        &[
            "docs",
            "+update",
            "--doc",
            doc_ref,
            "--command",
            "append",
            "--doc-format",
            "markdown",
            "--content",
            "-",
            "--json",
        ],
        content.as_bytes(),
    ) else {
        return false;
    };

    if !output.status.success() {
        log_lark_failure("lark-cli docs +update append failed", &output);
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(summary) = parse_lark_update_result(&stdout) {
        info!(
            "Lark document appended: result={}, revision={}",
            empty_as_unknown(&summary.result),
            summary
                .revision_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "(unknown)".to_string())
        );
    }

    true
}

fn pull_report_from_lark(config: &YamlConfig, force: bool) -> bool {
    let Some(doc_ref) = lark_document_ref(config) else {
        error!("Lark document is not configured.");
        return false;
    };

    let report_path = match super::io::get_report_path(config) {
        Some(path) => path,
        None => return false,
    };

    info!("Pulling Lark document into local report.");
    info!("  source: {}", lark_document_display(config));
    info!("  target: {}", report_path);

    let Some(fetched) = fetch_lark_document_markdown(&doc_ref) else {
        return false;
    };

    let local_content = match fs::read_to_string(&report_path) {
        Ok(content) => content,
        Err(e) => {
            error!("failed to read local report file {}: {}", report_path, e);
            return false;
        }
    };

    if !force {
        match decide_pull_mode(&local_content, &fetched.content) {
            PullDecision::NoChange => {
                info!("Local report is already up to date.");
                return true;
            }
            PullDecision::Append { tail } => {
                info!(
                    "Safe append-only pull detected (remote revision {}, appending {} bytes locally).",
                    fetched
                        .revision_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "(unknown)".to_string()),
                    tail.len()
                );
            }
            PullDecision::Destructive => {
                error!(
                    "Lark pull aborted: remote document is not an append-only extension of the local report."
                );
                error!("Run `j reportctl pull -f` to overwrite the local report intentionally.");
                return false;
            }
        }
    } else {
        info!("Force pull enabled; overwriting the local report file.");
        if let Err(e) =
            backup_content_to_lark_child(config, BackupKind::LocalBeforePull, &local_content)
        {
            error!("failed to backup local report before force pull: {}", e);
            return false;
        }
    }

    if let Err(e) = fs::write(&report_path, fetched.content.as_bytes()) {
        error!("failed to write local report file {}: {}", report_path, e);
        return false;
    }

    info!(
        "Local report updated from Lark document revision {} ({} bytes).",
        fetched
            .revision_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "(unknown)".to_string()),
        fetched.content.len()
    );

    true
}

fn fetch_lark_document_markdown(doc_ref: &str) -> Option<LarkFetchContent> {
    let output = Command::new("lark-cli")
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1")
        .args([
            "docs",
            "+fetch",
            "--doc",
            doc_ref,
            "--doc-format",
            "markdown",
            "--json",
        ])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(e) => {
            error!("failed to run lark-cli docs +fetch: {}", e);
            return None;
        }
    };

    if !output.status.success() {
        log_lark_failure("lark-cli docs +fetch failed", &output);
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_lark_fetch_content(&stdout)
}

fn inspect_lark_document(url: &str) -> Option<LarkDocumentInfo> {
    info!("Inspecting Lark document metadata with lark-cli.");
    let output = Command::new("lark-cli")
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1")
        .args(["drive", "+inspect", "--url", url, "--json"])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(e) => {
            error!("failed to inspect Lark document: {}", e);
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!(
            "lark-cli drive +inspect failed: {}{}",
            stdout.trim(),
            stderr.trim()
        );
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let doc = match parse_lark_document_info(&stdout) {
        Some(doc) => doc,
        None => return None,
    };

    info!("Lark document inspected:");
    info!("  title: {}", empty_as_unknown(&doc.title));
    info!("  type: {}", empty_as_unknown(&doc.doc_type));
    info!("  token: {}", empty_as_unknown(&doc.token));

    Some(doc)
}

fn parse_lark_document_info(json: &str) -> Option<LarkDocumentInfo> {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(e) => {
            error!("failed to parse lark-cli inspect output: {}", e);
            return None;
        }
    };

    let doc = LarkDocumentInfo {
        doc_type: first_string(&value, &["type", "obj_type", "file_type"]).unwrap_or_default(),
        token: first_string(
            &value,
            &[
                "token",
                "obj_token",
                "doc_token",
                "file_token",
                "node_token",
            ],
        )
        .unwrap_or_default(),
        title: first_string(&value, &["title", "name"]).unwrap_or_default(),
    };

    if doc.token.is_empty() {
        error!("lark-cli inspect output does not include a document token.");
        return None;
    }

    Some(doc)
}

fn parse_lark_fetch_content(json: &str) -> Option<LarkFetchContent> {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(e) => {
            error!("failed to parse lark-cli fetch output: {}", e);
            return None;
        }
    };

    let content = value
        .pointer("/data/document/content")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let Some(content) = content else {
        error!("lark-cli fetch output does not include document content.");
        return None;
    };

    Some(LarkFetchContent {
        content,
        revision_id: value
            .pointer("/data/document/revision_id")
            .and_then(Value::as_i64),
    })
}

fn parse_lark_update_result(json: &str) -> Option<LarkUpdateSummary> {
    let value: Value = serde_json::from_str(json).ok()?;

    Some(LarkUpdateSummary {
        result: value
            .pointer("/data/result")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        revision_id: value
            .pointer("/data/document/revision_id")
            .and_then(Value::as_i64),
    })
}

fn parse_lark_create_result(json: &str, title: String) -> Option<LarkBackupLocation> {
    let value: Value = serde_json::from_str(json).ok()?;

    Some(LarkBackupLocation {
        title,
        url: first_string(&value, &["url"]),
        token: first_string(
            &value,
            &[
                "node_token",
                "obj_token",
                "document_id",
                "token",
                "doc_token",
            ],
        ),
    })
}

fn parse_push_options(args: &[String]) -> ForceOptions {
    parse_force_options(args)
}

fn parse_force_options(args: &[String]) -> ForceOptions {
    let mut force = false;
    let mut ignored_message = None;

    for arg in args {
        match arg.as_str() {
            "-f" | "--force" => force = true,
            value if ignored_message.is_none() => ignored_message = Some(value.to_string()),
            _ => {}
        }
    }

    ForceOptions {
        force,
        ignored_message,
    }
}

fn decide_push_mode<'a>(remote: &str, local: &'a str) -> PushDecision<'a> {
    if local == remote {
        return PushDecision::NoChange;
    }

    if is_append_only_extension(remote, local) {
        return PushDecision::Append {
            tail: &local[remote.len()..],
        };
    }

    PushDecision::Destructive
}

fn decide_pull_mode<'a>(local: &str, remote: &'a str) -> PullDecision<'a> {
    if remote == local {
        return PullDecision::NoChange;
    }

    if is_append_only_extension(local, remote) {
        return PullDecision::Append {
            tail: &remote[local.len()..],
        };
    }

    PullDecision::Destructive
}

fn is_append_only_extension(base: &str, next: &str) -> bool {
    if !next.starts_with(base) {
        return false;
    }

    let tail = &next[base.len()..];
    !tail.is_empty() && (base.is_empty() || tail.starts_with('\n'))
}

fn save_lark_document(config: &mut YamlConfig, url: &str, doc: &LarkDocumentInfo) -> bool {
    let entries = [
        (config_key::LARK_DOC_URL, url),
        (config_key::LARK_DOC_TYPE, doc.doc_type.as_str()),
        (config_key::LARK_DOC_TOKEN, doc.token.as_str()),
        (config_key::LARK_DOC_TITLE, doc.title.as_str()),
    ];

    for (key, value) in entries {
        if let Err(e) = config.set_property(section::REPORT, key, value) {
            error!("failed to save Lark report config {}: {}", key, e);
            return false;
        }
    }

    info!("Lark report document config saved.");
    true
}

fn has_lark_document_metadata(config: &YamlConfig) -> bool {
    config
        .get_property(section::REPORT, config_key::LARK_DOC_URL)
        .is_some_and(|url| !url.trim().is_empty())
        && config
            .get_property(section::REPORT, config_key::LARK_DOC_TOKEN)
            .is_some_and(|token| !token.trim().is_empty())
}

fn lark_document_ref(config: &YamlConfig) -> Option<String> {
    config
        .get_property(section::REPORT, config_key::LARK_DOC_URL)
        .filter(|url| !url.trim().is_empty())
        .cloned()
        .or_else(|| {
            config
                .get_property(section::REPORT, config_key::LARK_DOC_TOKEN)
                .filter(|token| !token.trim().is_empty())
                .cloned()
        })
}

fn lark_document_display(config: &YamlConfig) -> String {
    let title = config
        .get_property(section::REPORT, config_key::LARK_DOC_TITLE)
        .map(String::as_str)
        .unwrap_or_default();
    let doc_type = config
        .get_property(section::REPORT, config_key::LARK_DOC_TYPE)
        .map(String::as_str)
        .unwrap_or_default();

    format!(
        "{} ({})",
        empty_as_unknown(title),
        empty_as_unknown(doc_type)
    )
}

fn backup_content_to_lark_child(
    config: &YamlConfig,
    kind: BackupKind,
    content: &str,
) -> Result<LarkBackupLocation, String> {
    let Some(parent_token) = lark_wiki_parent_token(config) else {
        return Err(
            "configured Lark document is not a wiki URL; cannot create a backup child document"
                .to_string(),
        );
    };

    let title = backup_document_title(kind);
    info!("Creating Lark backup child document before force overwrite.");
    info!("  parent wiki node: {}", parent_token);
    info!("  backup title: {}", title);

    let args = [
        "docs",
        "+create",
        "--parent-token",
        parent_token.as_str(),
        "--title",
        title.as_str(),
        "--doc-format",
        "markdown",
        "--content",
        "-",
        "--json",
    ];

    let Some(output) = run_lark_with_stdin(&args, content.as_bytes()) else {
        return Err("failed to run lark-cli docs +create".to_string());
    };

    if !output.status.success() {
        log_lark_failure("lark-cli docs +create backup failed", &output);
        return Err(format!(
            "lark-cli docs +create exited with {}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(location) = parse_lark_create_result(&stdout, title.clone()) else {
        return Err("failed to parse lark-cli docs +create backup output".to_string());
    };

    info!("Backup child document created: {}", location.display());
    Ok(location)
}

fn lark_wiki_parent_token(config: &YamlConfig) -> Option<String> {
    config
        .get_property(section::REPORT, config_key::LARK_DOC_URL)
        .and_then(|url| extract_wiki_token(url))
}

fn extract_wiki_token(input: &str) -> Option<String> {
    let parsed = Url::parse(input).ok()?;
    let mut segments = parsed.path_segments()?;
    while let Some(segment) = segments.next() {
        if segment == "wiki" {
            return segments
                .next()
                .filter(|token| !token.trim().is_empty())
                .map(ToString::to_string);
        }
    }
    None
}

fn backup_document_title(kind: BackupKind) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    match kind {
        BackupKind::RemoteBeforePush { revision_id } => {
            let revision = revision_id
                .map(|id| format!(" rev {}", id))
                .unwrap_or_default();
            format!(
                "j report backup before force push{} - {}",
                revision, timestamp
            )
        }
        BackupKind::LocalBeforePull => {
            format!("j report backup before force pull - {}", timestamp)
        }
    }
}

fn run_lark_with_stdin(args: &[&str], stdin: &[u8]) -> Option<Output> {
    let mut child = match Command::new("lark-cli")
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            error!("failed to run lark-cli: {}", e);
            return None;
        }
    };

    if let Some(mut child_stdin) = child.stdin.take()
        && let Err(e) = child_stdin.write_all(stdin)
    {
        error!("failed to write stdin to lark-cli: {}", e);
        return None;
    }

    match child.wait_with_output() {
        Ok(output) => Some(output),
        Err(e) => {
            error!("failed to wait for lark-cli: {}", e);
            None
        }
    }
}

fn log_lark_failure(context: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    error!("{}: {}", context, output.status);
    if !stdout.trim().is_empty() {
        error!("{}", stdout.trim());
    }
    if !stderr.trim().is_empty() {
        error!("{}", stderr.trim());
    }
}

fn command_exists(name: &str) -> bool {
    match Command::new(name).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version = stdout.trim();
            let version = if version.is_empty() {
                stderr.trim()
            } else {
                version
            };
            if !version.is_empty() {
                info!("{} version: {}", name, version);
            }
            true
        }
        Ok(output) => {
            info!("{} --version failed, exit status: {}", name, output.status);
            false
        }
        Err(e) => {
            info!("{} is not available: {}", name, e);
            false
        }
    }
}

fn prompt_line(prompt: &str) -> Option<String> {
    print!("{}", prompt);
    let _ = io::stdout().flush();

    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(_) => Some(buf.trim_end_matches(['\r', '\n']).to_string()),
        Err(e) => {
            error!("failed to read input: {}", e);
            None
        }
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = find_string(value, key) {
            return Some(s);
        }
    }
    None
}

fn find_string(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(v) = map.get(key).and_then(Value::as_str) {
                return Some(v.to_string());
            }
            map.values().find_map(|v| find_string(v, key))
        }
        Value::Array(items) => items.iter().find_map(|v| find_string(v, key)),
        _ => None,
    }
}

fn empty_as_unknown(value: &str) -> &str {
    if value.is_empty() { "(unknown)" } else { value }
}

struct LarkDocumentInfo {
    doc_type: String,
    token: String,
    title: String,
}

struct LarkFetchContent {
    content: String,
    revision_id: Option<i64>,
}

struct LarkUpdateSummary {
    result: String,
    revision_id: Option<i64>,
}

struct LarkBackupLocation {
    title: String,
    url: Option<String>,
    token: Option<String>,
}

impl LarkBackupLocation {
    fn display(&self) -> String {
        match (&self.url, &self.token) {
            (Some(url), _) if !url.is_empty() => format!("{} ({})", self.title, url),
            (_, Some(token)) if !token.is_empty() => format!("{} ({})", self.title, token),
            _ => self.title.clone(),
        }
    }
}

struct ForceOptions {
    force: bool,
    ignored_message: Option<String>,
}

#[derive(Clone, Copy)]
enum BackupKind {
    RemoteBeforePush { revision_id: Option<i64> },
    LocalBeforePull,
}

#[derive(Debug, PartialEq, Eq)]
enum PushDecision<'a> {
    NoChange,
    Append { tail: &'a str },
    Destructive,
}

#[derive(Debug, PartialEq, Eq)]
enum PullDecision<'a> {
    NoChange,
    Append { tail: &'a str },
    Destructive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_identity_ready_requires_pass_status() {
        let ready = r#"{
            "ok": true,
            "checks": [
                {"name": "bot_identity", "status": "pass"},
                {"name": "user_identity", "status": "pass"}
            ]
        }"#;
        let bot_only = r#"{
            "ok": true,
            "checks": [
                {"name": "bot_identity", "status": "pass"},
                {"name": "user_identity", "status": "warn"}
            ]
        }"#;

        assert!(user_identity_ready(ready));
        assert!(!user_identity_ready(bot_only));
        assert!(!user_identity_ready("not-json"));
    }

    #[test]
    fn first_string_finds_nested_values() {
        let value: Value = serde_json::from_str(
            r#"{
                "data": {
                    "node": {
                        "name": "Weekly Report",
                        "obj_token": "doc-token"
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(
            first_string(&value, &["title", "name"]).as_deref(),
            Some("Weekly Report")
        );
        assert_eq!(
            first_string(&value, &["token", "obj_token"]).as_deref(),
            Some("doc-token")
        );
    }

    #[test]
    fn parse_lark_document_info_accepts_top_level_schema() {
        let doc = parse_lark_document_info(
            r#"{
                "type": "docx",
                "token": "top-level-token",
                "title": "Daily Report"
            }"#,
        )
        .unwrap();

        assert_eq!(doc.doc_type, "docx");
        assert_eq!(doc.token, "top-level-token");
        assert_eq!(doc.title, "Daily Report");
    }

    #[test]
    fn parse_lark_document_info_accepts_nested_schema() {
        let doc = parse_lark_document_info(
            r#"{
                "data": {
                    "file": {
                        "file_type": "sheet",
                        "file_token": "nested-token",
                        "name": "Weekly Report"
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(doc.doc_type, "sheet");
        assert_eq!(doc.token, "nested-token");
        assert_eq!(doc.title, "Weekly Report");
    }

    #[test]
    fn parse_lark_document_info_rejects_missing_token() {
        assert!(parse_lark_document_info(r#"{"title":"No Token"}"#).is_none());
    }

    #[test]
    fn parse_lark_fetch_content_reads_markdown_and_revision() {
        let fetched = parse_lark_fetch_content(
            r##"{
                "ok": true,
                "data": {
                    "document": {
                        "revision_id": 53,
                        "content": "# Weekly Report\n\n- item"
                    }
                }
            }"##,
        )
        .unwrap();

        assert_eq!(fetched.revision_id, Some(53));
        assert_eq!(fetched.content, "# Weekly Report\n\n- item");
    }

    #[test]
    fn parse_lark_fetch_content_rejects_missing_content() {
        assert!(parse_lark_fetch_content(r#"{"ok":true,"data":{"document":{}}}"#).is_none());
    }

    #[test]
    fn parse_lark_update_result_reads_result_and_revision() {
        let summary = parse_lark_update_result(
            r#"{
                "ok": true,
                "data": {
                    "result": "success",
                    "document": {"revision_id": 54}
                }
            }"#,
        )
        .unwrap();

        assert_eq!(summary.result, "success");
        assert_eq!(summary.revision_id, Some(54));
    }

    #[test]
    fn parse_lark_create_result_reads_backup_location() {
        let location = parse_lark_create_result(
            r#"{
                "ok": true,
                "data": {
                    "document": {
                        "document_id": "docx-token",
                        "url": "https://example.feishu.cn/docx/docx-token"
                    }
                }
            }"#,
            "backup title".to_string(),
        )
        .unwrap();

        assert_eq!(location.title, "backup title");
        assert_eq!(
            location.url.as_deref(),
            Some("https://example.feishu.cn/docx/docx-token")
        );
        assert_eq!(location.token.as_deref(), Some("docx-token"));
        assert_eq!(
            location.display(),
            "backup title (https://example.feishu.cn/docx/docx-token)"
        );
    }

    #[test]
    fn extract_wiki_token_reads_path_segment() {
        assert_eq!(
            extract_wiki_token(
                "https://bytedance.larkoffice.com/wiki/Fm2TwXP6CiNaEVk8LLecsXYEnod?from=copy"
            )
            .as_deref(),
            Some("Fm2TwXP6CiNaEVk8LLecsXYEnod")
        );
        assert_eq!(
            extract_wiki_token("https://bytedance.larkoffice.com/docx/WMEkd7GgCoamDIxAx7tc6yApnWe"),
            None
        );
    }

    #[test]
    fn lark_wiki_parent_token_uses_configured_url() {
        let mut config = YamlConfig::default();
        config.report.insert(
            config_key::LARK_DOC_URL.to_string(),
            "https://bytedance.larkoffice.com/wiki/Fm2TwXP6CiNaEVk8LLecsXYEnod".to_string(),
        );

        assert_eq!(
            lark_wiki_parent_token(&config).as_deref(),
            Some("Fm2TwXP6CiNaEVk8LLecsXYEnod")
        );
    }

    #[test]
    fn backup_document_title_describes_force_operation() {
        let push_title = backup_document_title(BackupKind::RemoteBeforePush {
            revision_id: Some(42),
        });
        let pull_title = backup_document_title(BackupKind::LocalBeforePull);

        assert!(push_title.starts_with("j report backup before force push rev 42 - "));
        assert!(pull_title.starts_with("j report backup before force pull - "));
    }

    #[test]
    fn parse_push_options_detects_force_and_ignores_message() {
        let args = vec!["update report".to_string(), "-f".to_string()];
        let options = parse_push_options(&args);

        assert!(options.force);
        assert_eq!(options.ignored_message.as_deref(), Some("update report"));
    }

    #[test]
    fn parse_push_options_accepts_long_force() {
        let args = vec!["--force".to_string()];
        let options = parse_push_options(&args);

        assert!(options.force);
        assert_eq!(options.ignored_message, None);
    }

    #[test]
    fn decide_push_mode_detects_no_change() {
        assert_eq!(
            decide_push_mode("# report\n", "# report\n"),
            PushDecision::NoChange
        );
    }

    #[test]
    fn decide_push_mode_allows_empty_remote_as_append() {
        assert_eq!(
            decide_push_mode("", "# report\n"),
            PushDecision::Append { tail: "# report\n" }
        );
    }

    #[test]
    fn decide_push_mode_allows_append_only_update() {
        assert_eq!(
            decide_push_mode("# report\n", "# report\n\n- new item\n"),
            PushDecision::Append {
                tail: "\n- new item\n"
            }
        );
    }

    #[test]
    fn decide_push_mode_rejects_rewrite() {
        assert_eq!(
            decide_push_mode("# report\n- old\n", "# report\n- changed\n"),
            PushDecision::Destructive
        );
    }

    #[test]
    fn decide_push_mode_rejects_same_line_extension() {
        assert_eq!(
            decide_push_mode("- old", "- old text"),
            PushDecision::Destructive
        );
    }

    #[test]
    fn decide_pull_mode_detects_no_change() {
        assert_eq!(
            decide_pull_mode("# report\n", "# report\n"),
            PullDecision::NoChange
        );
    }

    #[test]
    fn decide_pull_mode_allows_empty_local_as_append() {
        assert_eq!(
            decide_pull_mode("", "# report\n"),
            PullDecision::Append { tail: "# report\n" }
        );
    }

    #[test]
    fn decide_pull_mode_allows_append_only_remote() {
        assert_eq!(
            decide_pull_mode("# report\n", "# report\n\n- remote item\n"),
            PullDecision::Append {
                tail: "\n- remote item\n"
            }
        );
    }

    #[test]
    fn decide_pull_mode_rejects_remote_rewrite() {
        assert_eq!(
            decide_pull_mode("# report\n- local\n", "# report\n- changed remotely\n"),
            PullDecision::Destructive
        );
    }

    #[test]
    fn decide_pull_mode_rejects_same_line_extension() {
        assert_eq!(
            decide_pull_mode("- local", "- local remote"),
            PullDecision::Destructive
        );
    }

    #[test]
    fn has_lark_document_metadata_requires_url_and_token() {
        let mut config = YamlConfig::default();
        assert!(!has_lark_document_metadata(&config));

        config.report.insert(
            config_key::LARK_DOC_URL.to_string(),
            "https://example.feishu.cn/docx/token".to_string(),
        );
        assert!(!has_lark_document_metadata(&config));

        config.report.insert(
            config_key::LARK_DOC_TOKEN.to_string(),
            "doc-token".to_string(),
        );
        assert!(has_lark_document_metadata(&config));
    }
}
