//! Report sync backend routing.

use crate::config::YamlConfig;
use crate::constants::{config_key, report_backend, section};
use crate::{error, info};
use std::io::{self, Write};

pub fn handle_push(args: &[String], config: &mut YamlConfig) {
    match ensure_backend(config) {
        Some(backend) if backend == report_backend::GIT => {
            info!("Using Git report sync backend for push.");
            let message = args
                .iter()
                .find(|arg| arg.as_str() != "-f" && arg.as_str() != "--force")
                .map(String::as_str);
            super::git::handle_push(message, config)
        }
        Some(backend) if backend == report_backend::LARK => {
            info!("Using Lark report sync backend for push.");
            super::lark::handle_push(args, config)
        }
        Some(backend) => error!("unsupported report sync backend: {}", backend),
        None => {}
    }
}

pub fn handle_pull(args: &[String], config: &mut YamlConfig) {
    match ensure_backend(config) {
        Some(backend) if backend == report_backend::GIT => {
            info!("Using Git report sync backend for pull.");
            super::git::handle_pull(config)
        }
        Some(backend) if backend == report_backend::LARK => {
            info!("Using Lark report sync backend for pull.");
            super::lark::handle_pull(args, config)
        }
        Some(backend) => error!("unsupported report sync backend: {}", backend),
        None => {}
    }
}

pub fn handle_use(backend: Option<&str>, config: &mut YamlConfig) {
    let backend = match backend {
        Some(value) => value.trim().to_ascii_lowercase(),
        None => {
            show_current_backend(config);
            return;
        }
    };

    if !is_supported_backend(&backend) {
        error!("unsupported backend `{}`. available: git, lark", backend);
        return;
    }

    if backend == report_backend::LARK && !super::lark::ensure_lark_ready(config) {
        return;
    }

    if let Err(e) = config.set_property(section::REPORT, config_key::REPORT_SYNC_BACKEND, &backend)
    {
        error!("failed to save report sync backend: {}", e);
        return;
    }

    info!("report sync backend set to: {}", backend);
}

fn ensure_backend(config: &mut YamlConfig) -> Option<String> {
    if let Some(backend) = config.get_property(section::REPORT, config_key::REPORT_SYNC_BACKEND)
        && is_supported_backend(backend)
    {
        info!("Report sync backend already configured: {}", backend);
        return Some(backend.clone());
    }

    // Backward compatibility: existing git_repo users continue using Git.
    if config
        .get_property(section::REPORT, config_key::GIT_REPO)
        .is_some_and(|url| !url.trim().is_empty())
    {
        let backend = report_backend::GIT.to_string();
        info!("Detected existing git_repo; using Git report sync backend.");
        if let Err(e) =
            config.set_property(section::REPORT, config_key::REPORT_SYNC_BACKEND, &backend)
        {
            error!("failed to save report sync backend: {}", e);
            return None;
        }
        return Some(backend);
    }

    prompt_backend(config)
}

fn prompt_backend(config: &mut YamlConfig) -> Option<String> {
    info!("Report sync backend is not configured.");
    info!("Choose where `j reportctl push/pull` should sync reports:");
    info!("  1) git  - existing Git repository flow");
    info!("  2) lark - Lark/Feishu document flow");

    print!("Select backend [git/lark]: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if let Err(e) = io::stdin().read_line(&mut input) {
        error!("failed to read backend selection: {}", e);
        return None;
    }

    let backend = normalize_backend(&input)?;
    if backend == report_backend::LARK && !super::lark::ensure_lark_ready(config) {
        return None;
    }

    if let Err(e) = config.set_property(section::REPORT, config_key::REPORT_SYNC_BACKEND, backend) {
        error!("failed to save report sync backend: {}", e);
        return None;
    }

    info!("report sync backend set to: {}", backend);
    Some(backend.to_string())
}

pub(crate) fn normalize_backend(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "1" | "git" | "g" => Some(report_backend::GIT),
        "2" | "lark" | "feishu" | "f" => Some(report_backend::LARK),
        other => {
            error!("unsupported backend `{}`. available: git, lark", other);
            None
        }
    }
}

fn show_current_backend(config: &YamlConfig) {
    match config.get_property(section::REPORT, config_key::REPORT_SYNC_BACKEND) {
        Some(backend) if !backend.trim().is_empty() => {
            info!("current report sync backend: {}", backend)
        }
        _ => info!("report sync backend is not configured"),
    }
}

pub(crate) fn is_supported_backend(value: &str) -> bool {
    value == report_backend::GIT || value == report_backend::LARK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_backend_accepts_git_aliases() {
        assert_eq!(normalize_backend("git"), Some(report_backend::GIT));
        assert_eq!(normalize_backend("g"), Some(report_backend::GIT));
        assert_eq!(normalize_backend("1"), Some(report_backend::GIT));
        assert_eq!(normalize_backend(" GIT "), Some(report_backend::GIT));
    }

    #[test]
    fn normalize_backend_accepts_lark_aliases() {
        assert_eq!(normalize_backend("lark"), Some(report_backend::LARK));
        assert_eq!(normalize_backend("feishu"), Some(report_backend::LARK));
        assert_eq!(normalize_backend("f"), Some(report_backend::LARK));
        assert_eq!(normalize_backend("2"), Some(report_backend::LARK));
    }

    #[test]
    fn normalize_backend_rejects_unknown_values() {
        assert_eq!(normalize_backend(""), None);
        assert_eq!(normalize_backend("github"), None);
        assert_eq!(normalize_backend("docs"), None);
    }

    #[test]
    fn supported_backend_is_strict() {
        assert!(is_supported_backend(report_backend::GIT));
        assert!(is_supported_backend(report_backend::LARK));
        assert!(!is_supported_backend("feishu"));
        assert!(!is_supported_backend("git "));
    }
}
