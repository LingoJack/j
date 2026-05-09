import { useEffect, useRef, useCallback } from 'react'
import { p256 } from '@noble/curves/nist.js'
import { gcm } from '@noble/ciphers/aes.js'
import { randomBytes } from '@noble/ciphers/utils.js'
import { hkdf } from '@noble/hashes/hkdf.js'
import { sha256 } from '@noble/hashes/sha2.js'

/**
 * ECDH P-256 + AES-256-GCM 加密 WebSocket Hook（纯 JS 实现）
 *
 * 使用 @noble/curves + @noble/ciphers，不依赖 crypto.subtle，
 * 因此在 HTTP（非 localhost）环境下也能工作。
 */

// base64url 编解码 (无 padding)
function b64urlEncode(bytes) {
  let str = ''
  for (let i = 0; i < bytes.length; i++) str += String.fromCharCode(bytes[i])
  return btoa(str).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}
function b64urlDecode(str) {
  str = str.replace(/-/g, '+').replace(/_/g, '/')
  while (str.length % 4) str += '='
  const bin = atob(str)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return bytes
}

// ECDH: 生成客户端密钥对，返回 { privateKey: Uint8Array(32), publicKey: Uint8Array(65) }
function generateKeyPair() {
  const privateKey = p256.utils.randomSecretKey()
  const publicKey = p256.getPublicKey(privateKey, false) // uncompressed (65 bytes)
  return { privateKey, publicKey }
}

// ECDH: 计算 shared secret → HKDF 派生 AES-256 密钥
function deriveAesKey(clientPrivateKey, serverPublicKeyBytes) {
  // ECDH shared secret (x coordinate, 32 bytes)
  const shared = p256.getSharedSecret(clientPrivateKey, serverPublicKeyBytes, false)
  // getSharedSecret returns uncompressed point (65 bytes), take x coordinate (bytes 1..33)
  const sharedX = shared.slice(1, 33)

  // HKDF-SHA256, info = "j-remote-aes256gcm" (与 Rust 端一致)
  const info = new TextEncoder().encode('j-remote-aes256gcm')
  return hkdf(sha256, sharedX, undefined, info, 32)
}

// AES-256-GCM 加密: 返回 Uint8Array = [nonce(12) | ciphertext+tag]
function aesEncrypt(key, plaintext) {
  const nonce = randomBytes(12)
  const encoded = new TextEncoder().encode(plaintext)
  const cipher = gcm(key, nonce)
  const ciphertext = cipher.encrypt(encoded)
  // 拼接 [nonce(12) | ciphertext]
  const result = new Uint8Array(12 + ciphertext.length)
  result.set(nonce, 0)
  result.set(ciphertext, 12)
  return result
}

// AES-256-GCM 解密: 输入 Uint8Array = [nonce(12) | ciphertext+tag]
function aesDecrypt(key, data) {
  const nonce = data.slice(0, 12)
  const ciphertext = data.slice(12)
  const cipher = gcm(key, nonce)
  const plaintext = cipher.decrypt(ciphertext)
  return new TextDecoder().decode(plaintext)
}

export function useWebSocket(url, onMessage, onStatusChange) {
  const wsRef = useRef(null)
  const aesKeyRef = useRef(null)
  const readyRef = useRef(false)
  const pendingRef = useRef([])

  const send = useCallback((data) => {
    const ws = wsRef.current
    const aesKey = aesKeyRef.current
    if (!ws || ws.readyState !== WebSocket.OPEN) return

    const json = JSON.stringify(data)

    if (!readyRef.current || !aesKey) {
      pendingRef.current.push(json)
      return
    }

    try {
      const buf = aesEncrypt(aesKey, json)
      ws.send(buf.buffer)
    } catch (err) {
      console.error('加密发送失败', err)
    }
  }, [])

  useEffect(() => {
    let reconnectTimer = null
    let pingInterval = null
    let retryCount = 0
    const MAX_RETRIES = 20
    const BASE_DELAY = 1000
    let destroyed = false

    function connect() {
      if (destroyed) return

      aesKeyRef.current = null
      readyRef.current = false
      pendingRef.current = []

      const ws = new WebSocket(url)
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onopen = () => {
        // 等待 server_hello
      }

      ws.onclose = () => {
        onStatusChange(false)
        readyRef.current = false
        aesKeyRef.current = null
        clearInterval(pingInterval)
        if (!destroyed && retryCount < MAX_RETRIES) {
          const delay = Math.min(BASE_DELAY * Math.pow(1.5, retryCount), 30000)
          retryCount++
          reconnectTimer = setTimeout(connect, delay)
        }
      }

      ws.onerror = () => {}

      ws.onmessage = (e) => {
        try {
          // 协商阶段
          if (!readyRef.current) {
            // server_hello 是明文 Text
            if (typeof e.data === 'string') {
              const msg = JSON.parse(e.data)
              if (msg.type === 'server_hello' && msg.server_pk) {
                handleServerHello(ws, msg.server_pk)
                return
              }
            }
            // key_exchange_ok 是加密的 Binary
            if (e.data instanceof ArrayBuffer && aesKeyRef.current) {
              const text = aesDecrypt(aesKeyRef.current, new Uint8Array(e.data))
              const msg = JSON.parse(text)
              if (msg.type === 'key_exchange_ok') {
                readyRef.current = true
                retryCount = 0
                onStatusChange(true)

                // 发送排队的消息
                const pending = pendingRef.current.splice(0)
                for (const json of pending) {
                  const buf = aesEncrypt(aesKeyRef.current, json)
                  if (ws.readyState === WebSocket.OPEN) ws.send(buf.buffer)
                }

                // 发送 sync 请求
                const syncBuf = aesEncrypt(aesKeyRef.current, JSON.stringify({ type: 'sync' }))
                if (ws.readyState === WebSocket.OPEN) ws.send(syncBuf.buffer)

                // 客户端 ping 10 秒间隔
                clearInterval(pingInterval)
                pingInterval = setInterval(() => {
                  if (ws.readyState === WebSocket.OPEN && aesKeyRef.current && readyRef.current) {
                    try {
                      const buf = aesEncrypt(aesKeyRef.current, JSON.stringify({ type: 'ping' }))
                      ws.send(buf.buffer)
                    } catch {}
                  }
                }, 10000)

                return
              }
            }
            return
          }

          // 加密通信阶段：Binary
          if (e.data instanceof ArrayBuffer && aesKeyRef.current) {
            const text = aesDecrypt(aesKeyRef.current, new Uint8Array(e.data))
            const msg = JSON.parse(text)
            onMessage(msg)
          }
        } catch (err) {
          console.error('消息处理错误', err)
        }
      }
    }

    function handleServerHello(ws, serverPkB64) {
      try {
        const serverPkBytes = b64urlDecode(serverPkB64)
        const { privateKey, publicKey } = generateKeyPair()
        const clientPkB64 = b64urlEncode(publicKey)

        // 发送 key_exchange（明文 JSON）
        ws.send(JSON.stringify({ type: 'key_exchange', client_pk: clientPkB64 }))

        // 派生 AES 密钥
        const aesKey = deriveAesKey(privateKey, serverPkBytes)
        aesKeyRef.current = aesKey

        // 等待 key_exchange_ok（在 onmessage 中处理）
      } catch (err) {
        console.error('密钥协商失败', err)
        ws.close()
      }
    }

    connect()

    return () => {
      destroyed = true
      clearInterval(pingInterval)
      clearTimeout(reconnectTimer)
      wsRef.current?.close()
    }
  }, [url, onMessage, onStatusChange])

  return send
}
