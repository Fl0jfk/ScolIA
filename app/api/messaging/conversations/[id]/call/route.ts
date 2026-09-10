import { NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import { publish } from "@/app/lib/messaging/events";
import {
  MessagingError,
  assertParticipant,
  listConversationParticipantIds,
} from "@/app/lib/messaging/service";
import {
  MESSAGING_CALL_MAX_PEERS,
  type MessagingCallSignalAction,
  type MessagingCallSseType,
  type MessagingCallPeerInfo,
} from "@/app/lib/messaging/call-types";
import { getDb } from "@/db";
import { user as authUser } from "@/db/schema";
import { inArray } from "drizzle-orm";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

type CallRoom = {
  callId: string;
  conversationId: string;
  etablissementId: string;
  hostId: string;
  title: string;
  kind: "dm" | "group";
  participantIds: string[];
  joinedIds: Set<string>;
  createdAt: number;
};

type CallBusState = {
  roomsByCallId: Map<string, CallRoom>;
  activeCallByConversation: Map<string, string>;
};

const GLOBAL_KEY = "__scola_messaging_calls__" as const;

function getCallState(): CallBusState {
  const g = globalThis as typeof globalThis & { [GLOBAL_KEY]?: CallBusState };
  if (!g[GLOBAL_KEY]) {
    g[GLOBAL_KEY] = {
      roomsByCallId: new Map(),
      activeCallByConversation: new Map(),
    };
  }
  return g[GLOBAL_KEY];
}

function actionToSse(action: MessagingCallSignalAction): MessagingCallSseType {
  switch (action) {
    case "invite":
      return "call_invite";
    case "accept":
      return "call_accept";
    case "reject":
      return "call_reject";
    case "join":
      return "call_join";
    case "offer":
      return "call_offer";
    case "answer":
      return "call_answer";
    case "ice":
      return "call_ice";
    case "leave":
      return "call_leave";
    case "end":
      return "call_end";
    default: {
      const _exhaustive: never = action;
      return _exhaustive;
    }
  }
}

async function loadPeerInfos(userIds: string[]): Promise<MessagingCallPeerInfo[]> {
  if (userIds.length === 0) return [];
  const db = getDb();
  const rows = await db
    .select({
      id: authUser.id,
      name: authUser.name,
      firstName: authUser.firstName,
      lastName: authUser.lastName,
    })
    .from(authUser)
    .where(inArray(authUser.id, userIds));
  const byId = new Map(
    rows.map((r) => {
      const composed = [r.firstName, r.lastName].filter(Boolean).join(" ").trim();
      return [r.id, { userId: r.id, name: composed || r.name || "Collègue" }] as const;
    }),
  );
  return userIds.map((id) => byId.get(id) ?? { userId: id, name: "Collègue" });
}

function cleanupStaleRooms(state: CallBusState) {
  const maxAgeMs = 2 * 60 * 60 * 1000;
  const now = Date.now();
  for (const [callId, room] of state.roomsByCallId) {
    if (now - room.createdAt > maxAgeMs || room.joinedIds.size === 0) {
      state.roomsByCallId.delete(callId);
      if (state.activeCallByConversation.get(room.conversationId) === callId) {
        state.activeCallByConversation.delete(room.conversationId);
      }
    }
  }
}

export async function POST(
  req: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  try {
    const { id: conversationId } = await ctx.params;
    await assertParticipant(gate.ctx.etablissementId, conversationId, gate.ctx.userId);

    const body = (await req.json()) as {
      action?: MessagingCallSignalAction;
      callId?: string;
      toUserId?: string;
      sdp?: RTCSessionDescriptionInit;
      candidate?: RTCIceCandidateInit;
      title?: string;
      kind?: "dm" | "group";
    };

    const action = body.action;
    if (
      action !== "invite" &&
      action !== "accept" &&
      action !== "reject" &&
      action !== "join" &&
      action !== "offer" &&
      action !== "answer" &&
      action !== "ice" &&
      action !== "leave" &&
      action !== "end"
    ) {
      return NextResponse.json({ error: "action invalide." }, { status: 400 });
    }

    const state = getCallState();
    cleanupStaleRooms(state);

    const allParticipantIds = await listConversationParticipantIds(
      gate.ctx.etablissementId,
      conversationId,
    );
    if (allParticipantIds.length < 2) {
      return NextResponse.json(
        { error: "Pas assez de participants pour un appel." },
        { status: 400 },
      );
    }
    if (allParticipantIds.length > MESSAGING_CALL_MAX_PEERS) {
      return NextResponse.json(
        {
          error: `Visio limitée à ${MESSAGING_CALL_MAX_PEERS} personnes (mesh).`,
          code: "CALL_TOO_LARGE",
        },
        { status: 400 },
      );
    }

    const fromName =
      [gate.ctx.user.firstName, gate.ctx.user.lastName].filter(Boolean).join(" ").trim() ||
      gate.ctx.user.name ||
      "Collègue";

    const publishTo = (
      userIds: string[],
      sseType: MessagingCallSseType,
      payload: Record<string, unknown>,
    ) => {
      publish({
        type: sseType,
        etablissementId: gate.ctx.etablissementId,
        userIds,
        payload: {
          conversationId,
          fromUserId: gate.ctx.userId,
          fromName,
          ...payload,
        },
      });
    };

    if (action === "invite") {
      const existingCallId = state.activeCallByConversation.get(conversationId);
      if (existingCallId) {
        const existing = state.roomsByCallId.get(existingCallId);
        if (existing && existing.joinedIds.size > 0) {
          return NextResponse.json(
            {
              error: "Un appel est déjà en cours sur cette conversation.",
              code: "CALL_ALREADY_ACTIVE",
              callId: existingCallId,
            },
            { status: 409 },
          );
        }
      }

      const callId = crypto.randomUUID();
      const peers = await loadPeerInfos(allParticipantIds);
      const room: CallRoom = {
        callId,
        conversationId,
        etablissementId: gate.ctx.etablissementId,
        hostId: gate.ctx.userId,
        title: typeof body.title === "string" && body.title.trim() ? body.title.trim() : "Appel",
        kind: body.kind === "group" ? "group" : "dm",
        participantIds: allParticipantIds,
        joinedIds: new Set([gate.ctx.userId]),
        createdAt: Date.now(),
      };
      state.roomsByCallId.set(callId, room);
      state.activeCallByConversation.set(conversationId, callId);

      const others = allParticipantIds.filter((id) => id !== gate.ctx.userId);
      publishTo(others, "call_invite", {
        callId,
        title: room.title,
        kind: room.kind,
        participants: peers,
      });

      return NextResponse.json({
        ok: true,
        callId,
        participants: peers,
        iceServers: defaultIceServers(),
      });
    }

    const callId = typeof body.callId === "string" ? body.callId.trim() : "";
    if (!callId) {
      return NextResponse.json({ error: "callId requis." }, { status: 400 });
    }
    const room = state.roomsByCallId.get(callId);
    if (!room || room.conversationId !== conversationId) {
      return NextResponse.json({ error: "Appel introuvable." }, { status: 404 });
    }
    if (!room.participantIds.includes(gate.ctx.userId)) {
      return NextResponse.json({ error: "Non autorisé sur cet appel." }, { status: 403 });
    }

    if (action === "accept" || action === "join") {
      room.joinedIds.add(gate.ctx.userId);
      const peers = await loadPeerInfos([...room.joinedIds]);
      const others = [...room.joinedIds].filter((id) => id !== gate.ctx.userId);
      // Les déjà présents créent une offre vers le nouvel arrivant
      publishTo(others, "call_join", {
        callId,
        participants: peers,
        joinerId: gate.ctx.userId,
        joinerName: fromName,
      });
      return NextResponse.json({
        ok: true,
        callId,
        participants: peers,
        iceServers: defaultIceServers(),
      });
    }

    if (action === "reject") {
      const targets = [room.hostId].filter((id) => id !== gate.ctx.userId);
      publishTo(targets, "call_reject", { callId });
      return NextResponse.json({ ok: true });
    }

    if (action === "offer" || action === "answer" || action === "ice") {
      const toUserId = typeof body.toUserId === "string" ? body.toUserId.trim() : "";
      if (!toUserId || !room.participantIds.includes(toUserId)) {
        return NextResponse.json({ error: "toUserId invalide." }, { status: 400 });
      }
      if (action === "offer" || action === "answer") {
        if (!body.sdp || typeof body.sdp !== "object") {
          return NextResponse.json({ error: "sdp requis." }, { status: 400 });
        }
      }
      if (action === "ice" && !body.candidate) {
        return NextResponse.json({ error: "candidate requis." }, { status: 400 });
      }
      publishTo([toUserId], actionToSse(action), {
        callId,
        toUserId,
        sdp: body.sdp,
        candidate: body.candidate,
      });
      return NextResponse.json({ ok: true });
    }

    if (action === "leave" || action === "end") {
      room.joinedIds.delete(gate.ctx.userId);
      const targets = room.participantIds.filter((id) => id !== gate.ctx.userId);
      if (action === "end" || room.joinedIds.size === 0 || gate.ctx.userId === room.hostId) {
        publishTo(targets, "call_end", { callId, reason: action });
        state.roomsByCallId.delete(callId);
        if (state.activeCallByConversation.get(conversationId) === callId) {
          state.activeCallByConversation.delete(conversationId);
        }
      } else {
        publishTo(targets, "call_leave", { callId });
      }
      return NextResponse.json({ ok: true });
    }

    return NextResponse.json({ error: "Non géré." }, { status: 400 });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/call]", error);
    return NextResponse.json({ error: "Erreur serveur." }, { status: 500 });
  }
}

function defaultIceServers(): RTCIceServer[] {
  const servers: RTCIceServer[] = [
    { urls: "stun:stun.l.google.com:19302" },
    { urls: "stun:stun1.l.google.com:19302" },
  ];
  const turnUrl = process.env.MESSAGING_TURN_URL?.trim();
  const turnUser = process.env.MESSAGING_TURN_USERNAME?.trim();
  const turnPass = process.env.MESSAGING_TURN_CREDENTIAL?.trim();
  if (turnUrl && turnUser && turnPass) {
    servers.push({
      urls: turnUrl,
      username: turnUser,
      credential: turnPass,
    });
  }
  return servers;
}
