import { useEffect, useRef, useCallback } from 'react'

/**
 * ECDH P-256 + AES-256-GCM 加密 WebSocket Hook
 *
 * 握手流程:
 *   ws.onopen →
 *     收到 server_hello (明文 JSON, 含 server_pk) →
 *     生成客户端 P-256 密钥对 →
 *     发送 key_exchange (明文 JSON, 含 client_pk) →
 *     ECDH deriveBits → HKDF deriveKey → AES-256-GCM CryptoKey →
 *     收到加密的 key_exchange_ok →
 *     协商完成，进入加密通信
 *
 * 后续通信:
 *   send: JSON → TextEncoder → AES-GCM encrypt → [IV(12) + ciphertext] → ws.send(ArrayBuffer)
 *   recv: ArrayBuffer → 拆 IV + ciphertext → AES-GCM decrypt → TextDecoder → JSON.parse
 */

// base64url 编解码 (无 padding)
function b64urlEncode(buf) {
  const bytes = new Uint8Array(buf)
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
  return bytes.buffer
}

// 从未压缩 P-256 公钥字节 (65B) 提取 x, y (各 32B)
function parseUncompressedP256(rawBuf) {
  const raw = new Uint8Array(rawBuf)
  if (raw[0] !== 0x04 || raw.length !== 65) throw new Error('Invalid uncompressed P-256 key')
  return {
    x: b64urlEncode(raw.slice(1, 33)),
    y: b64urlEncode(raw.slice(33, 65)),
  }
}

// 导入 P-256 公钥 (base64url 编码的未压缩格式)
async function importServerPk(b64) {
  const rawBuf = b64urlDecode(b64)
  const { x, y } = parseUncompressedP256(rawBuf)
  return crypto.subtle.importKey(
    'jwk',
    { kty: 'EC', crv: 'P-256', x, y },
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    []
  )
}

// 生成客户端 P-256 密钥对
async function generateClientKeyPair() {
  return crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true, // exportable
    ['deriveBits']
  )
}

// 导出公钥为未压缩格式 (65B) → base64url
async function exportPublicKey(key) {
  const raw = await crypto.subtle.exportKey('raw', key)
  return b64urlEncode(raw)
}

// ECDH deriveBits → HKDF → AES-256-GCM CryptoKey
async function deriveAesKey(clientPrivateKey, serverPublicKey) {
  // ECDH → 256-bit shared secret
  const sharedBits = await crypto.subtle.deriveBits(
    { name: 'ECDH', public: serverPublicKey },
    clientPrivateKey,
    256
  )

  // 导入 shared secret 作为 HKDF 输入
  const hkdfKey = await crypto.subtle.importKey(
    'raw', sharedBits, { name: 'HKDF' }, false, ['deriveKey']
  )

  // HKDF-SHA256 → AES-256-GCM key (info = "j-remote-aes256gcm", 与 Rust 端一致)
  const info = new TextEncoder().encode('j-remote-aes256gcm')
  return crypto.subtle.deriveKey(
    { name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(0), info },
    hkdfKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  )
}

// AES-256-GCM 加密: 返回 ArrayBuffer = [IV(12) | ciphertext+tag]
async function aesEncrypt(aesKey, plaintext) {
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const encoded = new TextEncoder().encode(plaintext)
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    aesKey,
    encoded
  )
  // 拼接 [IV(12) | ciphertext]
  const result = new Uint8Array(12 + ciphertext.byteLength)
  result.set(iv, 0)
  result.set(new Uint8Array(ciphertext), 12)
  return result.buffer
}

// AES-256-GCM 解密: 输入 ArrayBuffer = [IV(12) | ciphertext+tag]
async function aesDecrypt(aesKey, data) {
  const buf = new Uint8Array(data)
  const iv = buf.slice(0, 12)
  const ciphertext = buf.slice(12)
  const plainBuf = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv },
    aesKey,
    ciphertext
  )
  return new TextDecoder().decode(plainBuf)
}

export function useWebSocket(url, onMessage, onStatusChange) {
  const wsRef = useRef(null)
  // aesKey 引用在闭包中共享
  const aesKeyRef = useRef(null)
  // 协商完成标志
  const readyRef = useRef(false)
  // 协商完成前的发送队列
  const pendingRef = useRef([])

  const send = useCallback((data) => {
    const ws = wsRef.current
    const aesKey = aesKeyRef.current
    if (!ws || ws.readyState !== WebSocket.OPEN) return

    const json = JSON.stringify(data)

    if (!readyRef.current || !aesKey) {
      // 协商未完成，排队
      pendingRef.current.push(json)
      return
    }

    // 加密发送
    aesEncrypt(aesKey, json).then(buf => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(buf)
      }
    }).catch(err => console.error('加密发送失败', err))
  }, [])

  useEffect(() => {
    let reconnectTimer = null
    let pingInterval = null
    let destroyed = false

    function connect() {
      if (destroyed) return

      // 重置加密状态
      aesKeyRef.current = null
      readyRef.current = false
      pendingRef.current = []

      const ws = new WebSocket(url)
      ws.binaryType = 'arraybuffer'
      wsRef.current = ws

      ws.onopen = () => {
        // 等待 server_hello（通过 onmessage 处理）
      }

      ws.onclose = () => {
        onStatusChange(false)
        readyRef.current = false
        aesKeyRef.current = null
        clearInterval(pingInterval)
        // 自动重连，1.5 秒间隔
        if (!destroyed) {
          reconnectTimer = setTimeout(connect, 1500)
        }
      }

      ws.onerror = () => {}

      ws.onmessage = async (e) => {
        try {
          // 协商阶段：处理明文 JSON 消息
          if (!readyRef.current) {
            // server_hello 是明文 Text
            if (typeof e.data === 'string') {
              const msg = JSON.parse(e.data)
              if (msg.type === 'server_hello' && msg.server_pk) {
                await handleServerHello(ws, msg.server_pk)
                return
              }
            }
            // key_exchange_ok 是加密的 Binary
            if (e.data instanceof ArrayBuffer && aesKeyRef.current) {
              const text = await aesDecrypt(aesKeyRef.current, e.data)
              const msg = JSON.parse(text)
              if (msg.type === 'key_exchange_ok') {
                readyRef.current = true
                onStatusChange(true)

                // 发送排队的消息
                const pending = pendingRef.current.splice(0)
                for (const json of pending) {
                  const buf = await aesEncrypt(aesKeyRef.current, json)
                  if (ws.readyState === WebSocket.OPEN) ws.send(buf)
                }

                // 发送 sync 请求
                const syncJson = JSON.stringify({ type: 'sync' })
                const syncBuf = await aesEncrypt(aesKeyRef.current, syncJson)
                if (ws.readyState === WebSocket.OPEN) ws.send(syncBuf)

                // 启动客户端 ping 间隔 10 秒
                clearInterval(pingInterval)
                pingInterval = setInterval(async () => {
                  if (ws.readyState === WebSocket.OPEN && aesKeyRef.current && readyRef.current) {
                    const pingJson = JSON.stringify({ type: 'ping' })
                    const buf = await aesEncrypt(aesKeyRef.current, pingJson)
                    if (ws.readyState === WebSocket.OPEN) ws.send(buf)
                  }
                }, 10000)

                return
              }
            }
            return
          }

          // 加密通信阶段：所有消息都是 Binary
          if (e.data instanceof ArrayBuffer && aesKeyRef.current) {
            const text = await aesDecrypt(aesKeyRef.current, e.data)
            const msg = JSON.parse(text)
            onMessage(msg)
          }
        } catch (err) {
          console.error('消息处理错误', err)
        }
      }
    }

    async function handleServerHello(ws, serverPkB64) {
      try {
        // 1. 导入服务端公钥
        const serverPk = await importServerPk(serverPkB64)

        // 2. 生成客户端密钥对
        const clientKeyPair = await generateClientKeyPair()
        const clientPkB64 = await exportPublicKey(clientKeyPair.publicKey)

        // 3. 发送 key_exchange（明文 JSON）
        ws.send(JSON.stringify({ type: 'key_exchange', client_pk: clientPkB64 }))

        // 4. ECDH + HKDF 派生 AES 密钥
        const aesKey = await deriveAesKey(clientKeyPair.privateKey, serverPk)
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
