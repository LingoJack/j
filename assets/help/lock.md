---
name: 文件加密
order: 11
---

# 文件加密 / 解密

j-cli 提供 `j lock` 和 `j unlock` 命令，使用 **AES-256-GCM** 对文件进行对称加密/解密。

## 命令

### 加密文件

```bash
j lock <密码> <文件或目录路径>
```

### 解密文件

```bash
j unlock <密码> <文件或目录路径>
```

## 参数

| 参数 | 说明 |
|------|------|
| 密码 | 用于派生 AES-256 密钥，不会存储到磁盘 |
| 文件或目录路径 | 目标文件或目录；目录时会递归处理所有文件 |

## 用法示例

### 加密单个文件

```bash
j lock mypassword secret.txt
# 输出: secret.txt.lock（原文件被删除）
```

### 解密单个文件

```bash
j unlock mypassword secret.txt.lock
# 输出: secret.txt（.lock 文件被删除）
```

### 批量加密目录

```bash
j lock mypassword ./documents/
# 递归加密目录下所有文件，每个文件生成对应的 .lock 文件
```

### 批量解密目录

```bash
j unlock mypassword ./documents/
# 递归解密目录下所有 .lock 文件
```

## 加密原理

- **算法**: AES-256-GCM（认证加密，防篡改）
- **密钥派生**: HKDF-SHA256，从密码 + 随机 Salt 派生 256-bit 密钥
- **随机性**: 每次加密使用独立的随机 Salt（32 字节）和 Nonce（12 字节）

### 加密文件格式

```
MAGIC(4) + VERSION(1) + SALT(32) + NONCE(12) + CIPHERTEXT+TAG(16)
```

- `MAGIC`: `JLCK`，标识 j-cli 加密文件
- `VERSION`: 格式版本号（当前为 `0x01`）
- `SALT`: HKDF 盐值（32 字节）
- `NONCE`: AES-GCM 随机 nonce（12 字节）
- `CIPHERTEXT+TAG`: 加密数据 + GCM 认证标签（16 字节）

## 行为说明

- 加密成功后会**删除原文件**，只保留 `.lock` 加密文件
- 解密成功后会**删除 .lock 文件**，只保留还原后的原文件
- 加密时自动**跳过已有 `.lock` 后缀的文件**
- 解密时只处理 `.lock` 后缀的文件
- 目录处理时会**跳过隐藏文件和隐藏目录**（以 `.` 开头）
- 会**跳过符号链接**

## 注意事项

- 请务必**记住密码**，密码丢失无法恢复文件内容
- 每次加密的密码不存储在系统中
- 加密后原文件被删除，请确认加密成功后再清理
