## Overview

File encryption and decryption using **AES-256-GCM** symmetric encryption, supporting single files or batch directory operations.

## Basic Usage

### Encrypt a file

```bash
j lock <password> <file-or-directory-path>
```

### Decrypt a file

```bash
j unlock <password> <file-or-directory-path>
```

## Parameters

| Parameter | Description |
|-----------|-------------|
| password | Used to derive the AES-256 key; not stored on disk |
| file-or-directory-path | Target file or directory; directories are processed recursively |

## Examples

### Encrypt a single file

```bash
j lock mypassword secret.txt
# Output: secret.txt.lock (original file is deleted)
```

### Decrypt a single file

```bash
j unlock mypassword secret.txt.lock
# Output: secret.txt (.lock file is deleted)
```

### Batch encrypt a directory

```bash
j lock mypassword ./documents/
# Recursively encrypts all files, generating .lock files for each
```

### Batch decrypt a directory

```bash
j unlock mypassword ./documents/
# Recursively decrypts all .lock files in the directory
```

## Encryption Details

| Item | Description |
|------|-------------|
| Algorithm | AES-256-GCM (authenticated encryption, tamper-proof) |
| Key derivation | HKDF-SHA256, derives 256-bit key from password + random Salt |
| Randomness | Each encryption uses an independent random Salt (32 bytes) and Nonce (12 bytes) |

### Encrypted File Format

```
MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG(16)
```

| Field | Length | Description |
|-------|--------|-------------|
| MAGIC | 4 bytes | `JLCK`, identifies j-cli encrypted files |
| VERSION | 1 byte | Format version (currently `0x01`) |
| SALT | 32 bytes | HKDF salt |
| NONCE | 12 bytes | AES-GCM random nonce |
| CIPHERTEXT+TAG | variable + 16 bytes | Encrypted data + GCM authentication tag |

## Behavior

| Behavior | Description |
|----------|-------------|
| After encryption | **Deletes the original file**, keeping only the `.lock` encrypted file |
| After decryption | **Deletes the `.lock` file**, keeping only the restored original file |
| Skip rule | Skips files already having the `.lock` suffix during encryption |
| Hidden files | Skips hidden files and directories (starting with `.`) |
| Symlinks | Skips symbolic links |

## Important Notes

- **Remember your password** — lost passwords cannot recover file contents
- Passwords are not stored in the system; must be entered each time
- Original files are deleted after encryption; verify encryption succeeded before cleanup