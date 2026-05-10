//! 文件加密/解密命令 (`j lock` / `j unlock`)
//!
//! 使用 AES-256-GCM 对文件内容进行对称加密，密码通过参数传入，不持久化。
//!
//! 文件格式: `MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG`

use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

use aes_gcm::aead::generic_array::typenum::U12;

/// 文件头魔数，标识 j-cli lock 文件
const MAGIC: &[u8; 4] = b"JLCK";

/// 文件格式版本号
const VERSION: u8 = 0x01;

/// Salt 长度（字节）
const SALT_LEN: usize = 32;

/// Nonce 长度（字节）
const NONCE_LEN: usize = 12;

/// 文件头固定长度: MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) = 49
const HEADER_LEN: usize = 4 + 1 + SALT_LEN + NONCE_LEN;

/// AES-GCM 认证标签长度（字节）
const TAG_LEN: usize = 16;

/// 加密后的文件后缀
const LOCK_EXTENSION: &str = ".lock";

/// 加密命令入口
///
/// `password` 用于派生 AES-256 密钥（不存储）;
/// `target` 为文件路径或目录路径，默认为当前目录。
pub fn handle_lock(password: &str, target: &str) {
    let path = Path::new(target);
    if !path.exists() {
        crate::error!("文件或目录不存在: {}", target);
        return;
    }

    let files = collect_targets(path, false);
    if files.is_empty() {
        crate::info!("没有需要加密的文件");
        return;
    }

    let mut success_count: usize = 0;
    let mut fail_count: usize = 0;

    for file in &files {
        match encrypt_file(password, file) {
            Ok(output_path) => {
                // 加密成功后删除原文件
                if let Err(e) = fs::remove_file(file) {
                    crate::error!("删除原文件失败 {}: {}", file.display(), e);
                    // 回滚：删除已生成的加密文件
                    let _ = fs::remove_file(&output_path);
                    fail_count += 1;
                    continue;
                }
                crate::info!("已加密: {} -> {}", file.display(), output_path.display());
                success_count += 1;
            }
            Err(e) => {
                crate::error!("加密失败 {}: {}", file.display(), e);
                fail_count += 1;
            }
        }
    }

    print_summary("加密", success_count, fail_count);
}

/// 解密命令入口
///
/// `password` 用于派生 AES-256 密钥;
/// `target` 为 `.lock` 文件路径或目录路径。
pub fn handle_unlock(password: &str, target: &str) {
    let path = Path::new(target);
    if !path.exists() {
        crate::error!("文件或目录不存在: {}", target);
        return;
    }

    let files = collect_targets(path, true);
    if files.is_empty() {
        crate::info!("没有需要解密的文件");
        return;
    }

    let mut success_count: usize = 0;
    let mut fail_count: usize = 0;

    for file in &files {
        match decrypt_file(password, file) {
            Ok(output_path) => {
                // 解密成功后删除 .lock 文件
                if let Err(e) = fs::remove_file(file) {
                    crate::error!("删除加密文件失败 {}: {}", file.display(), e);
                    let _ = fs::remove_file(&output_path);
                    fail_count += 1;
                    continue;
                }
                crate::info!("已解密: {} -> {}", file.display(), output_path.display());
                success_count += 1;
            }
            Err(e) => {
                crate::error!("解密失败 {}: {}", file.display(), e);
                fail_count += 1;
            }
        }
    }

    print_summary("解密", success_count, fail_count);
}

/// 打印操作汇总信息
fn print_summary(action: &str, success: usize, fail: usize) {
    let total = success + fail;
    if fail == 0 {
        crate::info!("{}完成: {} 个文件全部成功", action, total);
    } else {
        crate::info!(
            "{}完成: 成功 {} 个, 失败 {} 个, 共 {} 个",
            action,
            success,
            fail,
            total
        );
    }
}

/// 收集待处理文件列表
///
/// - `for_decrypt=false` 时收集加密目标（跳过 `.lock` 文件）
/// - `for_decrypt=true` 时收集解密目标（仅 `.lock` 文件）
fn collect_targets(path: &Path, for_decrypt: bool) -> Vec<PathBuf> {
    if path.is_file() {
        // 单文件模式
        if for_decrypt {
            if path.extension().and_then(|e| e.to_str()) == Some("lock") {
                vec![path.to_path_buf()]
            } else {
                crate::error!("文件不是 .lock 格式: {}", path.display());
                vec![]
            }
        } else {
            // 加密时跳过已有 .lock 后缀的文件
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(LOCK_EXTENSION))
            {
                crate::error!("文件已经是加密状态: {}", path.display());
                vec![]
            } else {
                vec![path.to_path_buf()]
            }
        }
    } else if path.is_dir() {
        collect_files_recursive(path, for_decrypt)
    } else {
        vec![]
    }
}

/// 递归收集目录中的文件
fn collect_files_recursive(dir: &Path, for_decrypt: bool) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            crate::error!("无法读取目录 {}: {}", dir.display(), e);
            return result;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // 跳过符号链接
        if path.is_symlink() {
            continue;
        }

        // 跳过隐藏目录/文件（以 . 开头）
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            // 递归处理隐藏目录（但不处理隐藏目录下的文件）
            if path.is_dir() {
                // 跳过隐藏目录，不递归
            }
            continue;
        }

        if path.is_dir() {
            result.extend(collect_files_recursive(&path, for_decrypt));
        } else if path.is_file() {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if for_decrypt {
                if filename.ends_with(LOCK_EXTENSION) {
                    result.push(path);
                }
            } else {
                if !filename.ends_with(LOCK_EXTENSION) {
                    result.push(path);
                }
            }
        }
    }

    result
}

/// 从密码和 salt 派生 256-bit AES 密钥
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), password.as_bytes());
    let mut key = [0u8; 32];
    // info 长度 <= SHA-256 输出长度(32)，expand 不会失败
    hk.expand(b"j-lock-aes256gcm-v1", &mut key)
        .expect("HKDF expand for 32 bytes should not fail");
    key
}

/// 加密单个文件
///
/// 原文件保持不变，生成 `<原文件>.lock` 加密文件。
fn encrypt_file(password: &str, path: &Path) -> Result<PathBuf, String> {
    let plaintext = fs::read(path).map_err(|e| format!("读取文件失败: {e}"))?;

    // 生成随机 salt 和 nonce
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    // 派生密钥并加密
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new((&key).into());
    let nonce: &Nonce<U12> = (&nonce_bytes).into();

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|_| "AES-GCM 加密失败".to_string())?;

    // 组装输出: MAGIC + VERSION + SALT + NONCE + CIPHERTEXT+TAG
    let mut output = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    let output_path = get_encrypted_path(path);
    fs::write(&output_path, &output).map_err(|e| format!("写入加密文件失败: {e}"))?;

    Ok(output_path)
}

/// 解密单个 `.lock` 文件
///
/// `.lock` 文件保持不变，还原出原始文件。
fn decrypt_file(password: &str, path: &Path) -> Result<PathBuf, String> {
    let data = fs::read(path).map_err(|e| format!("读取加密文件失败: {e}"))?;

    if data.len() < HEADER_LEN + TAG_LEN {
        return Err("加密文件格式无效（数据过短）".to_string());
    }

    // 校验 magic
    if &data[0..4] != MAGIC {
        return Err("加密文件格式无效（非 j-cli lock 文件）".to_string());
    }

    // 校验版本
    if data[4] != VERSION {
        return Err(format!("不支持的文件格式版本: {}", data[4]));
    }

    let salt = &data[5..5 + SALT_LEN];
    let nonce_bytes = &data[5 + SALT_LEN..5 + SALT_LEN + NONCE_LEN];
    let ciphertext = &data[HEADER_LEN..];

    // 派生密钥并解密
    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new((&key).into());
    let nonce: &Nonce<U12> = nonce_bytes.into();

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "密码错误或文件已损坏".to_string())?;

    let output_path = get_decrypted_path(path)?;
    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    fs::write(&output_path, &plaintext).map_err(|e| format!("写入解密文件失败: {e}"))?;

    Ok(output_path)
}

/// 获取加密后的输出路径（追加 `.lock` 后缀）
fn get_encrypted_path(path: &Path) -> PathBuf {
    let mut result = path.as_os_str().to_owned();
    result.push(LOCK_EXTENSION);
    PathBuf::from(result)
}

/// 获取解密后的输出路径（去掉 `.lock` 后缀）
fn get_decrypted_path(path: &Path) -> Result<PathBuf, String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "无法获取文件名".to_string())?;

    if !filename.ends_with(LOCK_EXTENSION) {
        return Err("文件不是 .lock 后缀".to_string());
    }

    let trimmed = &filename[..filename.len() - LOCK_EXTENSION.len()];
    if trimmed.is_empty() {
        return Err("去掉 .lock 后文件名为空".to_string());
    }

    Ok(path.with_file_name(trimmed))
}
