import { useEffect, useRef, useCallback } from 'react'

export function useWebSocket(url, onMessage, onStatusChange) {
  const wsRef = useRef(null)

  const send = useCallback((data) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(data))
    }
  }, [])

  useEffect(() => {
    let reconnectTimer = null
    let pingInterval = null
    let destroyed = false

    function connect() {
      if (destroyed) return
      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        onStatusChange(true)
        ws.send(JSON.stringify({ type: 'sync' }))

        // 客户端 ping 间隔 10 秒，配合服务端 15 秒 ping + 30 秒超时
        clearInterval(pingInterval)
        pingInterval = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'ping' }))
          }
        }, 10000)
      }

      ws.onclose = () => {
        onStatusChange(false)
        clearInterval(pingInterval)
        // 自动重连，1.5 秒间隔（加快重连速度）
        if (!destroyed) {
          reconnectTimer = setTimeout(connect, 1500)
        }
      }

      ws.onerror = () => {}

      ws.onmessage = (e) => {
        try {
          onMessage(JSON.parse(e.data))
        } catch (err) {
          console.error('消息解析错误', err)
        }
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
