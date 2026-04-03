//! ECDH P-256 密钥协商 + AES-256-GCM 加密工具

use aes_gcm::aead::Aead;
use aes_gcm::aead::generic_array::typenum::U12;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use p256::PublicKey;
use p256::ecdh::EphemeralSecret;
use rand::rngs::OsRng;
use sha2::Sha256;

/// AES-256-GCM nonce 长度（12 字节）
const NONCE_LEN: usize = 12;

/// 生成 P-256 密钥对
pub fn generate_keypair() -> (EphemeralSecret, PublicKey) {
    let secret = EphemeralSecret::random(&mut OsRng);
    let public = secret.public_key();
    (secret, public)
}

/// 从 ECDH shared_secret 经 HKDF-SHA256 派生 32 字节 AES-256 密钥
pub fn derive_aes_key(shared_secret: &p256::ecdh::SharedSecret) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, shared_secret.raw_secret_bytes());
    let mut key = [0u8; 32];
    hk.expand(b"j-remote-aes256gcm", &mut key)
        .expect("HKDF expand should not fail for 32 bytes");
    key
}

/// AES-256-GCM 加密
///
/// 返回 `[nonce(12) | ciphertext | tag(16)]`
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    use rand::RngCore;
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce: &Nonce<U12> = (&nonce_bytes).into();

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("AES-GCM encrypt should not fail");

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

/// AES-256-GCM 解密
///
/// 输入 `[nonce(12) | ciphertext | tag(16)]`
pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, &'static str> {
    if data.len() < NONCE_LEN + 16 {
        return Err("密文太短");
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(key.into());
    let nonce: &Nonce<U12> = nonce_bytes.into();

    cipher.decrypt(nonce, ciphertext).map_err(|_| "解密失败")
}

/// 导出公钥为 base64url（未压缩 65 字节）
pub fn export_public_key(pk: &PublicKey) -> String {
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    let point = pk.to_encoded_point(false); // uncompressed
    URL_SAFE_NO_PAD.encode(point.as_bytes())
}

/// 从 base64url 编码的未压缩公钥导入 P-256 PublicKey
pub fn import_public_key(b64: &str) -> Result<PublicKey, &'static str> {
    let bytes = URL_SAFE_NO_PAD.decode(b64).map_err(|_| "base64 解码失败")?;
    PublicKey::from_sec1_bytes(&bytes).map_err(|_| "无效的 P-256 公钥")
}
