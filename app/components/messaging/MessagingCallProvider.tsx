"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import type { MessagingCallPeerInfo } from "@/app/lib/messaging/call-types";
import type { MessagingSseEvent } from "@/app/lib/messaging/types";

type CallPhase = "idle" | "outgoing" | "incoming" | "connecting" | "active";

export type CallViewMode = "split" | "pip";

type RemotePeer = {
  userId: string;
  name: string;
  stream: MediaStream | null;
};

type IncomingInvite = {
  callId: string;
  conversationId: string;
  title: string;
  kind: "dm" | "group";
  fromUserId: string;
  fromName: string;
  participants: MessagingCallPeerInfo[];
};

type ActiveCall = {
  callId: string;
  conversationId: string;
  title: string;
  kind: "dm" | "group";
  phase: CallPhase;
  viewMode: CallViewMode;
  localStream: MediaStream | null;
  remotes: RemotePeer[];
  muted: boolean;
  camOff: boolean;
  error: string | null;
};

type MessagingCallContextValue = {
  call: ActiveCall | null;
  incoming: IncomingInvite | null;
  startCall: (args: {
    conversationId: string;
    title: string;
    kind: "dm" | "group";
  }) => Promise<void>;
  acceptIncoming: () => Promise<void>;
  rejectIncoming: () => Promise<void>;
  hangUp: () => Promise<void>;
  toggleMute: () => void;
  toggleCam: () => void;
  setViewMode: (mode: CallViewMode) => void;
  handleSseEvent: (event: MessagingSseEvent) => void;
};

const MessagingCallContext = createContext<MessagingCallContextValue | null>(null);

function defaultIce(): RTCIceServer[] {
  return [
    { urls: "stun:stun.l.google.com:19302" },
    { urls: "stun:stun1.l.google.com:19302" },
  ];
}

async function postCall(
  conversationId: string,
  body: Record<string, unknown>,
): Promise<Response> {
  return fetch(`/api/messaging/conversations/${conversationId}/call`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

export function MessagingCallProvider({
  currentUserId,
  children,
}: {
  currentUserId: string;
  children: ReactNode;
}) {
  const [call, setCall] = useState<ActiveCall | null>(null);
  const [incoming, setIncoming] = useState<IncomingInvite | null>(null);
  const iceServersRef = useRef<RTCIceServer[]>(defaultIce());
  const pcsRef = useRef<Map<string, RTCPeerConnection>>(new Map());
  const localStreamRef = useRef<MediaStream | null>(null);
  const callRef = useRef<ActiveCall | null>(null);
  const makingOfferRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    callRef.current = call;
  }, [call]);

  const stopLocal = useCallback(() => {
    localStreamRef.current?.getTracks().forEach((t) => t.stop());
    localStreamRef.current = null;
  }, []);

  const closeAllPcs = useCallback(() => {
    for (const pc of pcsRef.current.values()) {
      try {
        pc.close();
      } catch {
        /* ignore */
      }
    }
    pcsRef.current.clear();
    makingOfferRef.current.clear();
  }, []);

  const resetCallState = useCallback(() => {
    closeAllPcs();
    stopLocal();
    setCall(null);
    setIncoming(null);
  }, [closeAllPcs, stopLocal]);

  const ensureLocalStream = useCallback(async () => {
    if (localStreamRef.current) return localStreamRef.current;
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true },
      video: {
        facingMode: "user",
        width: { ideal: 1280 },
        height: { ideal: 720 },
      },
    });
    localStreamRef.current = stream;
    setCall((prev) => (prev ? { ...prev, localStream: stream } : prev));
    return stream;
  }, []);

  const upsertRemote = useCallback((userId: string, name: string, stream: MediaStream | null) => {
    setCall((prev) => {
      if (!prev) return prev;
      const others = prev.remotes.filter((r) => r.userId !== userId);
      return {
        ...prev,
        remotes: [...others, { userId, name, stream }],
        phase: "active",
      };
    });
  }, []);

  const removeRemote = useCallback((userId: string) => {
    const pc = pcsRef.current.get(userId);
    if (pc) {
      try {
        pc.close();
      } catch {
        /* ignore */
      }
      pcsRef.current.delete(userId);
    }
    setCall((prev) =>
      prev
        ? { ...prev, remotes: prev.remotes.filter((r) => r.userId !== userId) }
        : prev,
    );
  }, []);

  const getOrCreatePc = useCallback(
    async (peerId: string, peerName: string, conversationId: string, callId: string) => {
      const existing = pcsRef.current.get(peerId);
      if (existing) return existing;

      const pc = new RTCPeerConnection({ iceServers: iceServersRef.current });
      pcsRef.current.set(peerId, pc);

      const local = await ensureLocalStream();
      for (const track of local.getTracks()) {
        pc.addTrack(track, local);
      }

      pc.onicecandidate = (ev) => {
        if (!ev.candidate) return;
        void postCall(conversationId, {
          action: "ice",
          callId,
          toUserId: peerId,
          candidate: ev.candidate.toJSON(),
        });
      };

      pc.ontrack = (ev) => {
        const stream = ev.streams[0] ?? new MediaStream([ev.track]);
        upsertRemote(peerId, peerName, stream);
      };

      pc.onconnectionstatechange = () => {
        if (
          pc.connectionState === "failed" ||
          pc.connectionState === "closed" ||
          pc.connectionState === "disconnected"
        ) {
          // Soft leave: keep UI until peer leave signal
        }
      };

      return pc;
    },
    [ensureLocalStream, upsertRemote],
  );

  const createOfferTo = useCallback(
    async (peerId: string, peerName: string, conversationId: string, callId: string) => {
      if (peerId === currentUserId) return;
      if (makingOfferRef.current.has(peerId)) return;
      makingOfferRef.current.add(peerId);
      try {
        const pc = await getOrCreatePc(peerId, peerName, conversationId, callId);
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        await postCall(conversationId, {
          action: "offer",
          callId,
          toUserId: peerId,
          sdp: pc.localDescription,
        });
      } finally {
        makingOfferRef.current.delete(peerId);
      }
    },
    [currentUserId, getOrCreatePc],
  );

  const startCall = useCallback(
    async (args: { conversationId: string; title: string; kind: "dm" | "group" }) => {
      if (callRef.current || incoming) return;
      try {
        await ensureLocalStream();
        setCall({
          callId: "",
          conversationId: args.conversationId,
          title: args.title,
          kind: args.kind,
          phase: "outgoing",
          viewMode: "split",
          localStream: localStreamRef.current,
          remotes: [],
          muted: false,
          camOff: false,
          error: null,
        });
        const res = await postCall(args.conversationId, {
          action: "invite",
          title: args.title,
          kind: args.kind,
        });
        const data = (await res.json()) as {
          callId?: string;
          iceServers?: RTCIceServer[];
          error?: string;
        };
        if (!res.ok) {
          throw new Error(data.error || "Impossible de démarrer l’appel");
        }
        if (Array.isArray(data.iceServers) && data.iceServers.length) {
          iceServersRef.current = data.iceServers;
        }
        setCall((prev) =>
          prev
            ? {
                ...prev,
                callId: data.callId || "",
                phase: "active",
              }
            : prev,
        );
      } catch (e) {
        resetCallState();
        setCall({
          callId: "",
          conversationId: args.conversationId,
          title: args.title,
          kind: args.kind,
          phase: "idle",
          viewMode: "split",
          localStream: null,
          remotes: [],
          muted: false,
          camOff: false,
          error: e instanceof Error ? e.message : "Erreur d’appel",
        });
        // Clear error banner after showing briefly via hangUp-like reset
        setTimeout(() => setCall(null), 2500);
      }
    },
    [ensureLocalStream, incoming, resetCallState],
  );

  const acceptIncoming = useCallback(async () => {
    if (!incoming) return;
    const invite = incoming;
    setIncoming(null);
    try {
      await ensureLocalStream();
      setCall({
        callId: invite.callId,
        conversationId: invite.conversationId,
        title: invite.title,
        kind: invite.kind,
        phase: "connecting",
        viewMode: "split",
        localStream: localStreamRef.current,
        remotes: [],
        muted: false,
        camOff: false,
        error: null,
      });
      const res = await postCall(invite.conversationId, {
        action: "accept",
        callId: invite.callId,
      });
      const data = (await res.json()) as {
        iceServers?: RTCIceServer[];
        error?: string;
        participants?: MessagingCallPeerInfo[];
      };
      if (!res.ok) throw new Error(data.error || "Impossible de rejoindre");
      if (Array.isArray(data.iceServers) && data.iceServers.length) {
        iceServersRef.current = data.iceServers;
      }
      // Offers will come from already-joined peers via call_join on their side;
      // we wait for their offers. Also offer to host if polite (id comparison).
      const others = (data.participants || invite.participants).filter(
        (p) => p.userId !== currentUserId,
      );
      for (const p of others) {
        if (currentUserId < p.userId) {
          await createOfferTo(p.userId, p.name, invite.conversationId, invite.callId);
        }
      }
      setCall((prev) => (prev ? { ...prev, phase: "active" } : prev));
    } catch (e) {
      resetCallState();
      alert(e instanceof Error ? e.message : "Échec de la connexion");
    }
  }, [createOfferTo, currentUserId, ensureLocalStream, incoming, resetCallState]);

  const rejectIncoming = useCallback(async () => {
    if (!incoming) return;
    const invite = incoming;
    setIncoming(null);
    await postCall(invite.conversationId, {
      action: "reject",
      callId: invite.callId,
    }).catch(() => undefined);
  }, [incoming]);

  const hangUp = useCallback(async () => {
    const active = callRef.current;
    if (active?.callId && active.conversationId) {
      await postCall(active.conversationId, {
        action: "end",
        callId: active.callId,
      }).catch(() => undefined);
    }
    resetCallState();
  }, [resetCallState]);

  const toggleMute = useCallback(() => {
    const stream = localStreamRef.current;
    if (!stream) return;
    const next = !(callRef.current?.muted ?? false);
    for (const t of stream.getAudioTracks()) t.enabled = !next;
    setCall((prev) => (prev ? { ...prev, muted: next } : prev));
  }, []);

  const toggleCam = useCallback(() => {
    const stream = localStreamRef.current;
    if (!stream) return;
    const next = !(callRef.current?.camOff ?? false);
    for (const t of stream.getVideoTracks()) t.enabled = !next;
    setCall((prev) => (prev ? { ...prev, camOff: next } : prev));
  }, []);

  const setViewMode = useCallback((mode: CallViewMode) => {
    setCall((prev) => (prev ? { ...prev, viewMode: mode } : prev));
  }, []);

  const handleSseEvent = useCallback(
    (event: MessagingSseEvent) => {
      const payload = (event.payload ?? {}) as Record<string, unknown>;
      const type = event.type;

      if (type === "call_invite") {
        if (callRef.current) return;
        const callId = String(payload.callId ?? "");
        const conversationId = String(
          payload.conversationId ?? event.conversationId ?? "",
        );
        if (!callId || !conversationId) return;
        setIncoming({
          callId,
          conversationId,
          title: String(payload.title ?? "Appel vidéo"),
          kind: payload.kind === "group" ? "group" : "dm",
          fromUserId: String(payload.fromUserId ?? ""),
          fromName: String(payload.fromName ?? "Collègue"),
          participants: Array.isArray(payload.participants)
            ? (payload.participants as MessagingCallPeerInfo[])
            : [],
        });
        return;
      }

      const active = callRef.current;
      const callId = String(payload.callId ?? "");
      if (!active || !callId || active.callId !== callId) {
        // Incoming accept path may not have call yet for join from self — ignore
        if (type === "call_end" || type === "call_reject") {
          if (incoming && incoming.callId === callId) setIncoming(null);
        }
        return;
      }

      if (type === "call_join") {
        const joinerId = String(payload.joinerId ?? payload.fromUserId ?? "");
        const joinerName = String(payload.joinerName ?? payload.fromName ?? "Collègue");
        if (joinerId && joinerId !== currentUserId) {
          void createOfferTo(joinerId, joinerName, active.conversationId, callId);
        }
        return;
      }

      if (type === "call_accept") {
        // Host may also create offers toward accepter
        const fromUserId = String(payload.fromUserId ?? "");
        const fromName = String(payload.fromName ?? "Collègue");
        if (fromUserId && fromUserId !== currentUserId) {
          void createOfferTo(fromUserId, fromName, active.conversationId, callId);
        }
        return;
      }

      if (type === "call_offer") {
        const fromUserId = String(payload.fromUserId ?? "");
        const fromName = String(payload.fromName ?? "Collègue");
        const sdp = payload.sdp as RTCSessionDescriptionInit | undefined;
        if (!fromUserId || !sdp) return;
        void (async () => {
          const pc = await getOrCreatePc(
            fromUserId,
            fromName,
            active.conversationId,
            callId,
          );
          await pc.setRemoteDescription(sdp);
          const answer = await pc.createAnswer();
          await pc.setLocalDescription(answer);
          await postCall(active.conversationId, {
            action: "answer",
            callId,
            toUserId: fromUserId,
            sdp: pc.localDescription,
          });
        })();
        return;
      }

      if (type === "call_answer") {
        const fromUserId = String(payload.fromUserId ?? "");
        const sdp = payload.sdp as RTCSessionDescriptionInit | undefined;
        if (!fromUserId || !sdp) return;
        const pc = pcsRef.current.get(fromUserId);
        if (!pc) return;
        void pc.setRemoteDescription(sdp);
        return;
      }

      if (type === "call_ice") {
        const fromUserId = String(payload.fromUserId ?? "");
        const candidate = payload.candidate as RTCIceCandidateInit | undefined;
        if (!fromUserId || !candidate) return;
        const pc = pcsRef.current.get(fromUserId);
        if (!pc) return;
        void pc.addIceCandidate(candidate).catch(() => undefined);
        return;
      }

      if (type === "call_leave") {
        const fromUserId = String(payload.fromUserId ?? "");
        if (fromUserId) removeRemote(fromUserId);
        return;
      }

      if (type === "call_end" || type === "call_reject") {
        resetCallState();
      }
    },
    [
      createOfferTo,
      currentUserId,
      getOrCreatePc,
      incoming,
      removeRemote,
      resetCallState,
    ],
  );

  const value = useMemo<MessagingCallContextValue>(
    () => ({
      call,
      incoming,
      startCall,
      acceptIncoming,
      rejectIncoming,
      hangUp,
      toggleMute,
      toggleCam,
      setViewMode,
      handleSseEvent,
    }),
    [
      acceptIncoming,
      call,
      handleSseEvent,
      hangUp,
      incoming,
      rejectIncoming,
      setViewMode,
      startCall,
      toggleCam,
      toggleMute,
    ],
  );

  return (
    <MessagingCallContext.Provider value={value}>{children}</MessagingCallContext.Provider>
  );
}

export function useMessagingCall(): MessagingCallContextValue {
  const ctx = useContext(MessagingCallContext);
  if (!ctx) {
    throw new Error("useMessagingCall must be used within MessagingCallProvider");
  }
  return ctx;
}
