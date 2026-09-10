"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { MessagingConversationDto, MessagingSseEvent } from "@/app/lib/messaging/types";

type UseMessagingStreamOptions = {
  enabled: boolean;
  onEvent: (event: MessagingSseEvent) => void;
  onFallbackPoll?: () => void;
};

/**
 * SSE avec reconnect + fallback polling 5s si le flux tombe.
 */
export function useMessagingStream({ enabled, onEvent, onFallbackPoll }: UseMessagingStreamOptions) {
  const onEventRef = useRef(onEvent);
  const onFallbackRef = useRef(onFallbackPoll);
  onEventRef.current = onEvent;
  onFallbackRef.current = onFallbackPoll;
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    if (!enabled) {
      setConnected(false);
      return;
    }

    let es: EventSource | null = null;
    let pollTimer: ReturnType<typeof setInterval> | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    let closed = false;
    let failCount = 0;

    const startPoll = () => {
      if (pollTimer) return;
      pollTimer = setInterval(() => {
        onFallbackRef.current?.();
      }, 5000);
    };

    const stopPoll = () => {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    };

    const connect = () => {
      if (closed) return;
      es?.close();
      es = new EventSource("/api/messaging/stream");
      es.onopen = () => {
        failCount = 0;
        setConnected(true);
        stopPoll();
      };
      es.onmessage = (ev) => {
        try {
          const data = JSON.parse(ev.data) as MessagingSseEvent;
          if (data.type !== "heartbeat") onEventRef.current(data);
        } catch {
          /* ignore */
        }
      };
      es.onerror = () => {
        setConnected(false);
        es?.close();
        es = null;
        failCount += 1;
        startPoll();
        const delay = Math.min(30_000, 1000 * 2 ** Math.min(failCount, 5));
        reconnectTimer = setTimeout(connect, delay);
      };
    };

    connect();

    return () => {
      closed = true;
      stopPoll();
      if (reconnectTimer) clearTimeout(reconnectTimer);
      es?.close();
      setConnected(false);
    };
  }, [enabled]);

  return { connected };
}

export async function fetchConversations(): Promise<MessagingConversationDto[]> {
  const res = await fetch("/api/messaging/conversations", { cache: "no-store" });
  if (!res.ok) throw new Error("Impossible de charger les conversations");
  const data = (await res.json()) as { conversations?: MessagingConversationDto[] };
  return Array.isArray(data.conversations) ? data.conversations : [];
}

export function useMessagingConversations(enabled: boolean) {
  const [conversations, setConversations] = useState<MessagingConversationDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    setError(null);
    try {
      const list = await fetchConversations();
      setConversations(list);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur de chargement");
    } finally {
      setLoading(false);
    }
  }, [enabled]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const totalUnread = conversations.reduce((acc, c) => acc + (c.unreadCount || 0), 0);

  return { conversations, setConversations, loading, error, refresh, totalUnread };
}
