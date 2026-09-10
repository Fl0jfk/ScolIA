"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  MessagingMyPresenceDto,
  MessagingPresenceStatus,
  MessagingSseEvent,
} from "@/app/lib/messaging/types";

function collectPeerIds(
  conversations: Array<{
    peer?: { id: string } | null;
    membersPreview?: Array<{ id: string }>;
  }>,
  extraUserIds: string[],
  selfId: string,
): string[] {
  const ids = new Set<string>();
  for (const c of conversations) {
    if (c.peer?.id) ids.add(c.peer.id);
    for (const m of c.membersPreview ?? []) {
      if (m.id) ids.add(m.id);
    }
  }
  for (const id of extraUserIds) {
    if (id.trim()) ids.add(id.trim());
  }
  ids.delete(selfId);
  return [...ids];
}

function isStatus(value: string): value is MessagingPresenceStatus {
  return (
    value === "online" ||
    value === "away" ||
    value === "busy" ||
    value === "dnd" ||
    value === "offline"
  );
}

/**
 * Heartbeat de mon statut + carte de présence des collègues visibles.
 */
export function useMessagingPresence({
  enabled,
  currentUserId,
  conversations,
  extraUserIds = [],
  callBusy = false,
}: {
  enabled: boolean;
  currentUserId: string;
  conversations: Array<{
    peer?: { id: string } | null;
    membersPreview?: Array<{ id: string }>;
  }>;
  extraUserIds?: string[];
  callBusy?: boolean;
}) {
  const [presenceByUser, setPresenceByUser] = useState<
    Record<string, MessagingPresenceStatus>
  >({});
  const [myPresence, setMyPresence] = useState<MessagingMyPresenceDto>({
    status: "online",
    manualStatus: null,
    manualUntil: null,
  });
  const peerIds = useMemo(
    () => collectPeerIds(conversations, extraUserIds, currentUserId),
    [conversations, extraUserIds, currentUserId],
  );
  const peerKey = peerIds.slice().sort().join(",");
  const lastPosted = useRef<MessagingPresenceStatus | null>(null);
  const manualActive = Boolean(myPresence.manualStatus && myPresence.manualUntil);

  const postAutoStatus = useCallback(async (status: MessagingPresenceStatus) => {
    if (status === "offline") return;
    if (lastPosted.current === status) return;
    lastPosted.current = status;
    try {
      await fetch("/api/messaging/presence", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status, mode: "auto" }),
      });
    } catch {
      lastPosted.current = null;
    }
  }, []);

  const setManualStatus = useCallback(
    async (
      status: Exclude<MessagingPresenceStatus, "offline">,
      durationHours: 0 | 1 | 4 | 24 = 24,
    ) => {
      const res = await fetch("/api/messaging/presence", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status, mode: "manual", durationHours }),
      });
      if (!res.ok) throw new Error("Impossible de changer le statut");
      const data = (await res.json()) as { me?: MessagingMyPresenceDto };
      if (data.me) {
        setMyPresence(data.me);
        lastPosted.current = data.me.status;
      }
      return data.me;
    },
    [],
  );

  const refreshMe = useCallback(async () => {
    if (!enabled) return;
    try {
      const res = await fetch("/api/messaging/presence?me=1", { cache: "no-store" });
      if (!res.ok) return;
      const data = (await res.json()) as { me?: MessagingMyPresenceDto };
      if (data.me) setMyPresence(data.me);
    } catch {
      /* ignore */
    }
  }, [enabled]);

  const refresh = useCallback(async () => {
    if (!enabled || peerIds.length === 0) return;
    try {
      const res = await fetch(
        `/api/messaging/presence?userIds=${encodeURIComponent(peerIds.join(","))}`,
        { cache: "no-store" },
      );
      if (!res.ok) return;
      const data = (await res.json()) as {
        presence?: Record<string, MessagingPresenceStatus>;
      };
      if (data.presence) {
        setPresenceByUser((prev) => ({ ...prev, ...data.presence }));
      }
    } catch {
      /* ignore */
    }
  }, [enabled, peerIds]);

  useEffect(() => {
    if (!enabled) return;
    void refresh();
    void refreshMe();
    const timer = setInterval(() => {
      void refresh();
      void refreshMe();
    }, 120_000);
    return () => clearInterval(timer);
  }, [enabled, refresh, refreshMe, peerKey]);

  /** Visibilité onglet + occupation appel — ignoré si statut manuel actif. */
  useEffect(() => {
    if (!enabled) return;

    const sync = () => {
      if (manualActive) return;
      if (callBusy) {
        void postAutoStatus("busy");
        return;
      }
      if (document.visibilityState === "hidden") {
        void postAutoStatus("away");
        return;
      }
      void postAutoStatus("online");
    };

    sync();
    document.addEventListener("visibilitychange", sync);
    window.addEventListener("focus", sync);
    return () => {
      document.removeEventListener("visibilitychange", sync);
      window.removeEventListener("focus", sync);
    };
  }, [enabled, callBusy, postAutoStatus, manualActive]);

  const applySseEvent = useCallback((event: MessagingSseEvent) => {
    if (event.type !== "presence") return;
    const payload = event.payload;
    if (!payload || typeof payload !== "object") return;
    const userId =
      "userId" in payload && typeof (payload as { userId: unknown }).userId === "string"
        ? (payload as { userId: string }).userId
        : null;
    const status =
      "status" in payload && typeof (payload as { status: unknown }).status === "string"
        ? (payload as { status: string }).status
        : null;
    if (!userId || !status || !isStatus(status)) return;
    setPresenceByUser((prev) => ({ ...prev, [userId]: status }));
  }, []);

  const statusOf = useCallback(
    (userId: string | null | undefined): MessagingPresenceStatus => {
      if (!userId) return "offline";
      if (userId === currentUserId) return myPresence.status;
      return presenceByUser[userId] ?? "offline";
    },
    [presenceByUser, currentUserId, myPresence.status],
  );

  const statusForConversation = useCallback(
    (conv: {
      kind: string;
      peer?: { id: string } | null;
      membersPreview?: Array<{ id: string }>;
    }): MessagingPresenceStatus => {
      if (conv.kind === "dm") return statusOf(conv.peer?.id);
      const members = (conv.membersPreview ?? []).map((m) => statusOf(m.id));
      if (members.some((s) => s === "online")) return "online";
      if (members.some((s) => s === "dnd")) return "dnd";
      if (members.some((s) => s === "busy")) return "busy";
      if (members.some((s) => s === "away")) return "away";
      return "offline";
    },
    [statusOf],
  );

  return {
    presenceByUser,
    myPresence,
    setManualStatus,
    statusOf,
    statusForConversation,
    applySseEvent,
    refreshPresence: refresh,
    refreshMyPresence: refreshMe,
  };
}
