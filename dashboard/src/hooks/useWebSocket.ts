'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import type { Alert, Detection } from '@/lib/api';

const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8000/ws';

export type WebSocketStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface WebSocketMessage {
  type: 'detection' | 'alert' | 'stats_update' | 'camera_status' | 'heartbeat';
  payload: unknown;
  timestamp: string;
}

interface UseWebSocketOptions {
  onDetection?: (detection: Detection) => void;
  onAlert?: (alert: Alert) => void;
  onStatsUpdate?: (stats: unknown) => void;
  onCameraStatus?: (status: { cameraId: string; status: string }) => void;
  autoReconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    onDetection,
    onAlert,
    onStatsUpdate,
    onCameraStatus,
    autoReconnect = true,
    reconnectInterval = 3000,
    maxReconnectAttempts = 10,
  } = options;

  const [status, setStatus] = useState<WebSocketStatus>('disconnected');
  const [lastMessage, setLastMessage] = useState<WebSocketMessage | null>(null);
  const [reconnectCount, setReconnectCount] = useState(0);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    setStatus('connecting');

    try {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.onopen = () => {
        setStatus('connected');
        setReconnectCount(0);
        console.log('[WebSocket] Connected to server');
      };

      ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          setLastMessage(message);

          switch (message.type) {
            case 'detection':
              onDetection?.(message.payload as Detection);
              break;
            case 'alert':
              onAlert?.(message.payload as Alert);
              break;
            case 'stats_update':
              onStatsUpdate?.(message.payload);
              break;
            case 'camera_status':
              onCameraStatus?.(message.payload as { cameraId: string; status: string });
              break;
            case 'heartbeat':
              // Heartbeat received, connection is alive
              break;
            default:
              console.log('[WebSocket] Unknown message type:', message.type);
          }
        } catch (error) {
          console.error('[WebSocket] Failed to parse message:', error);
        }
      };

      ws.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        setStatus('error');
      };

      ws.onclose = () => {
        setStatus('disconnected');
        wsRef.current = null;

        if (autoReconnect && reconnectCount < maxReconnectAttempts) {
          reconnectTimeoutRef.current = setTimeout(() => {
            setReconnectCount((prev) => prev + 1);
            connect();
          }, reconnectInterval);
        }
      };
    } catch (error) {
      console.error('[WebSocket] Failed to connect:', error);
      setStatus('error');
    }
  }, [
    onDetection,
    onAlert,
    onStatsUpdate,
    onCameraStatus,
    autoReconnect,
    reconnectInterval,
    maxReconnectAttempts,
    reconnectCount,
  ]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setStatus('disconnected');
    setReconnectCount(0);
  }, []);

  const send = useCallback((message: object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.warn('[WebSocket] Cannot send message: not connected');
    }
  }, []);

  const subscribe = useCallback((channels: string[]) => {
    send({ type: 'subscribe', channels });
  }, [send]);

  const unsubscribe = useCallback((channels: string[]) => {
    send({ type: 'unsubscribe', channels });
  }, [send]);

  useEffect(() => {
    connect();

    return () => {
      disconnect();
    };
  }, []);

  return {
    status,
    lastMessage,
    reconnectCount,
    connect,
    disconnect,
    send,
    subscribe,
    unsubscribe,
    isConnected: status === 'connected',
  };
}

// Hook for subscribing to specific camera feeds
export function useCameraFeed(cameraId: string) {
  const [detections, setDetections] = useState<Detection[]>([]);
  const [isActive, setIsActive] = useState(true);

  const { status, subscribe, unsubscribe } = useWebSocket({
    onDetection: (detection) => {
      if (detection.cameraId === cameraId) {
        setDetections((prev) => [...prev.slice(-50), detection]);
      }
    },
    onCameraStatus: (statusUpdate) => {
      if (statusUpdate.cameraId === cameraId) {
        setIsActive(statusUpdate.status === 'online');
      }
    },
  });

  useEffect(() => {
    if (status === 'connected') {
      subscribe([`camera:${cameraId}`]);
    }

    return () => {
      unsubscribe([`camera:${cameraId}`]);
    };
  }, [status, cameraId, subscribe, unsubscribe]);

  return { detections, isActive, connectionStatus: status };
}

// Hook for real-time alerts
export function useAlertStream() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);

  const { status, subscribe, unsubscribe } = useWebSocket({
    onAlert: (alert) => {
      setAlerts((prev) => [alert, ...prev.slice(0, 99)]);
      if (!alert.acknowledged) {
        setUnreadCount((prev) => prev + 1);
      }
    },
  });

  useEffect(() => {
    if (status === 'connected') {
      subscribe(['alerts']);
    }

    return () => {
      unsubscribe(['alerts']);
    };
  }, [status, subscribe, unsubscribe]);

  const markAsRead = useCallback(() => {
    setUnreadCount(0);
  }, []);

  return { alerts, unreadCount, markAsRead, connectionStatus: status };
}
