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

    function connect() {
      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        onStatusChange(true)
        ws.send(JSON.stringify({ type: 'sync' }))
      }

      ws.onclose = () => {
        onStatusChange(false)
        reconnectTimer = setTimeout(connect, 3000)
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

    const pingInterval = setInterval(() => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: 'ping' }))
      }
    }, 30000)

    return () => {
      clearInterval(pingInterval)
      clearTimeout(reconnectTimer)
      wsRef.current?.close()
    }
  }, [url, onMessage, onStatusChange])

  return send
}
