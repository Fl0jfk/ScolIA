import "server-only";

import { and, eq, inArray, ne, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { messagingParticipant, messagingPresence } from "@/db/schema";
import { publish } from "@/app/lib/messaging/events";
import type {
  MessagingMyPresenceDto,
  MessagingPresenceStatus,
} from "@/app/lib/messaging/types";

/** Au-delà : considéré hors ligne même si le statut n’a pas été basculé. */
export const PRESENCE_STALE_MS = 90_000;

const VALID: ReadonlySet<MessagingPresenceStatus> = new Set([
  "online",
  "away",
  "busy",
  "dnd",
  "offline",
]);

const MANUAL_ALLOWED: ReadonlySet<MessagingPresenceStatus> = new Set([
  "online",
  "away",
  "busy",
  "dnd",
]);

export function isPresenceStatus(value: unknown): value is MessagingPresenceStatus {
  return typeof value === "string" && VALID.has(value as MessagingPresenceStatus);
}

export function isManualPresenceStatus(
  value: unknown,
): value is Exclude<MessagingPresenceStatus, "offline"> {
  return typeof value === "string" && MANUAL_ALLOWED.has(value as MessagingPresenceStatus);
}

type PresenceRow = {
  status: MessagingPresenceStatus;
  connections: number;
  lastSeenAt: Date;
  manualStatus: MessagingPresenceStatus | null;
  manualUntil: Date | null;
};

function activeManual(
  row: Pick<PresenceRow, "manualStatus" | "manualUntil">,
  now = Date.now(),
): MessagingPresenceStatus | null {
  if (!row.manualStatus || !row.manualUntil) return null;
  if (row.manualUntil.getTime() <= now) return null;
  if (row.manualStatus === "offline") return null;
  return row.manualStatus;
}

function effectiveStatus(row: PresenceRow, now = Date.now()): MessagingPresenceStatus {
  const manual = activeManual(row, now);
  // Manuel visible même si l’utilisateur a encore une session (prioritaire).
  if (manual && row.connections > 0 && now - row.lastSeenAt.getTime() <= PRESENCE_STALE_MS) {
    return manual;
  }
  if (row.connections <= 0) return "offline";
  if (now - row.lastSeenAt.getTime() > PRESENCE_STALE_MS) return "offline";
  if (manual) return manual;
  if (row.status === "offline") return "online";
  return row.status;
}

/** Destinataires d’un événement présence : collègues partageant au moins une conversation. */
export async function listPresenceBroadcastTargets(
  etablissementId: string,
  userId: string,
): Promise<string[]> {
  const db = getDb();
  const myConvs = await db
    .select({ conversationId: messagingParticipant.conversationId })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        eq(messagingParticipant.userId, userId),
      ),
    );
  const convIds = [...new Set(myConvs.map((r) => r.conversationId))];
  if (convIds.length === 0) return [];

  const peers = await db
    .select({ userId: messagingParticipant.userId })
    .from(messagingParticipant)
    .where(
      and(
        eq(messagingParticipant.etablissementId, etablissementId),
        inArray(messagingParticipant.conversationId, convIds),
        ne(messagingParticipant.userId, userId),
      ),
    );
  return [...new Set(peers.map((p) => p.userId))];
}

async function broadcastPresence(
  etablissementId: string,
  userId: string,
  status: MessagingPresenceStatus,
  manualUntil: string | null = null,
): Promise<void> {
  const targets = await listPresenceBroadcastTargets(etablissementId, userId);
  if (targets.length === 0) return;
  publish({
    type: "presence",
    etablissementId,
    userIds: targets,
    payload: { userId, status, manualUntil },
  });
}

async function loadPresenceRow(userId: string): Promise<PresenceRow | null> {
  const db = getDb();
  const rows = await db
    .select({
      status: messagingPresence.status,
      connections: messagingPresence.connections,
      lastSeenAt: messagingPresence.lastSeenAt,
      manualStatus: messagingPresence.manualStatus,
      manualUntil: messagingPresence.manualUntil,
    })
    .from(messagingPresence)
    .where(eq(messagingPresence.userId, userId))
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  return {
    status: row.status,
    connections: row.connections,
    lastSeenAt: row.lastSeenAt,
    manualStatus: row.manualStatus ?? null,
    manualUntil: row.manualUntil ?? null,
  };
}

function toMyDto(row: PresenceRow | null, etablissementMatch = true): MessagingMyPresenceDto {
  if (!row || !etablissementMatch) {
    return { status: "offline", manualStatus: null, manualUntil: null };
  }
  const now = Date.now();
  const manual = activeManual(row, now);
  // Pour « mon statut » : on affiche le choix manuel même si le flux SSE a sauté un instant.
  if (manual) {
    return {
      status: manual,
      manualStatus: manual,
      manualUntil: row.manualUntil ? row.manualUntil.toISOString() : null,
    };
  }
  return {
    status: effectiveStatus(row, now),
    manualStatus: null,
    manualUntil: null,
  };
}

export async function touchPresenceConnection(
  etablissementId: string,
  userId: string,
  delta: 1 | -1,
  preferredStatus?: MessagingPresenceStatus,
): Promise<MessagingPresenceStatus> {
  const db = getDb();
  const now = new Date();
  const existing = await loadPresenceRow(userId);
  const manual = existing ? activeManual(existing) : null;

  if (delta === 1) {
    const nextStatus: MessagingPresenceStatus =
      manual ??
      (preferredStatus && preferredStatus !== "offline" ? preferredStatus : "online");
    await db
      .insert(messagingPresence)
      .values({
        userId,
        etablissementId,
        status: nextStatus,
        connections: 1,
        manualStatus: existing?.manualStatus ?? null,
        manualUntil: existing?.manualUntil ?? null,
        lastSeenAt: now,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: messagingPresence.userId,
        set: {
          etablissementId,
          connections: sql`${messagingPresence.connections} + 1`,
          status: sql`CASE
            WHEN ${messagingPresence.manualUntil} IS NOT NULL
              AND ${messagingPresence.manualUntil} > now()
              AND ${messagingPresence.manualStatus} IS NOT NULL
            THEN ${messagingPresence.manualStatus}
            WHEN ${messagingPresence.status} IN ('busy', 'dnd') THEN ${messagingPresence.status}
            ELSE ${nextStatus}
          END`,
          lastSeenAt: now,
          updatedAt: now,
        },
      });
  } else {
    await db
      .insert(messagingPresence)
      .values({
        userId,
        etablissementId,
        status: "offline",
        connections: 0,
        lastSeenAt: now,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: messagingPresence.userId,
        set: {
          connections: sql`GREATEST(0, ${messagingPresence.connections} - 1)`,
          status: sql`CASE
            WHEN ${messagingPresence.connections} <= 1 THEN 'offline'
            WHEN ${messagingPresence.manualUntil} IS NOT NULL
              AND ${messagingPresence.manualUntil} > now()
              AND ${messagingPresence.manualStatus} IS NOT NULL
            THEN ${messagingPresence.manualStatus}
            ELSE ${messagingPresence.status}
          END`,
          lastSeenAt: now,
          updatedAt: now,
        },
      });
  }

  const status = await getUserPresence(etablissementId, userId);
  await broadcastPresence(etablissementId, userId, status);
  return status;
}

/**
 * Statut automatique (visibilité onglet / appel).
 * N’écrase pas un statut manuel encore valide (busy / dnd / away programmés).
 */
export async function setPresenceStatus(
  etablissementId: string,
  userId: string,
  status: MessagingPresenceStatus,
): Promise<MessagingPresenceStatus> {
  const db = getDb();
  const now = new Date();
  const current = await loadPresenceRow(userId);
  const connections = current?.connections ?? 0;
  const manual = current ? activeManual(current) : null;

  if (manual) {
    // Auto sync ignoré tant que le manuel est actif.
    return effectiveStatus({
      status: current!.status,
      connections,
      lastSeenAt: current!.lastSeenAt,
      manualStatus: current!.manualStatus,
      manualUntil: current!.manualUntil,
    });
  }

  if (status === "offline" || connections <= 0) {
    await db
      .insert(messagingPresence)
      .values({
        userId,
        etablissementId,
        status: "offline",
        connections: Math.max(0, connections),
        lastSeenAt: now,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: messagingPresence.userId,
        set: {
          etablissementId,
          status: "offline",
          lastSeenAt: now,
          updatedAt: now,
        },
      });
    await broadcastPresence(etablissementId, userId, "offline");
    return "offline";
  }

  const next: MessagingPresenceStatus =
    current?.status === "busy" && status === "away"
      ? "busy"
      : current?.status === "dnd" && status === "away"
        ? "dnd"
        : status;

  await db
    .insert(messagingPresence)
    .values({
      userId,
      etablissementId,
      status: next,
      connections,
      lastSeenAt: now,
      updatedAt: now,
    })
    .onConflictDoUpdate({
      target: messagingPresence.userId,
      set: {
        etablissementId,
        status: next,
        lastSeenAt: now,
        updatedAt: now,
      },
    });

  await broadcastPresence(etablissementId, userId, next);
  return next;
}

/**
 * Choix utilisateur : Occupé / Ne pas déranger / Absent pour X heures (0 = jusqu’à changement manuel).
 */
export async function setManualPresence(
  etablissementId: string,
  userId: string,
  status: Exclude<MessagingPresenceStatus, "offline">,
  durationHours: number,
): Promise<MessagingMyPresenceDto> {
  const db = getDb();
  const now = new Date();
  const current = await loadPresenceRow(userId);
  const connections = Math.max(1, current?.connections ?? 1);

  let manualUntil: Date | null = null;
  if (status === "online") {
    // « Disponible » = clear manuel
    await db
      .insert(messagingPresence)
      .values({
        userId,
        etablissementId,
        status: "online",
        connections,
        manualStatus: null,
        manualUntil: null,
        lastSeenAt: now,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: messagingPresence.userId,
        set: {
          etablissementId,
          status: "online",
          manualStatus: null,
          manualUntil: null,
          lastSeenAt: now,
          updatedAt: now,
          connections: sql`GREATEST(1, ${messagingPresence.connections})`,
        },
      });
    await broadcastPresence(etablissementId, userId, "online", null);
    return {
      status: "online",
      manualStatus: null,
      manualUntil: null,
    };
  }

  if (durationHours <= 0) {
    // Jusqu’à nouvel ordre : 365 jours (plafond pratique)
    manualUntil = new Date(now.getTime() + 365 * 24 * 60 * 60 * 1000);
  } else {
    manualUntil = new Date(now.getTime() + durationHours * 60 * 60 * 1000);
  }

  await db
    .insert(messagingPresence)
    .values({
      userId,
      etablissementId,
      status,
      connections,
      manualStatus: status,
      manualUntil,
      lastSeenAt: now,
      updatedAt: now,
    })
    .onConflictDoUpdate({
      target: messagingPresence.userId,
      set: {
        etablissementId,
        status,
        manualStatus: status,
        manualUntil,
        lastSeenAt: now,
        updatedAt: now,
        connections: sql`GREATEST(1, ${messagingPresence.connections})`,
      },
    });

  const untilIso = manualUntil.toISOString();
  await broadcastPresence(etablissementId, userId, status, untilIso);
  return {
    status,
    manualStatus: status,
    manualUntil: untilIso,
  };
}

export async function heartbeatPresence(
  etablissementId: string,
  userId: string,
): Promise<void> {
  const db = getDb();
  const now = new Date();
  await db
    .update(messagingPresence)
    .set({ lastSeenAt: now, updatedAt: now })
    .where(
      and(
        eq(messagingPresence.userId, userId),
        eq(messagingPresence.etablissementId, etablissementId),
      ),
    );
}

export async function getUserPresence(
  etablissementId: string,
  userId: string,
): Promise<MessagingPresenceStatus> {
  const db = getDb();
  const rows = await db
    .select({
      status: messagingPresence.status,
      connections: messagingPresence.connections,
      lastSeenAt: messagingPresence.lastSeenAt,
      manualStatus: messagingPresence.manualStatus,
      manualUntil: messagingPresence.manualUntil,
      etablissementId: messagingPresence.etablissementId,
    })
    .from(messagingPresence)
    .where(eq(messagingPresence.userId, userId))
    .limit(1);
  const row = rows[0];
  if (!row || row.etablissementId !== etablissementId) return "offline";
  return effectiveStatus({
    status: row.status,
    connections: row.connections,
    lastSeenAt: row.lastSeenAt,
    manualStatus: row.manualStatus ?? null,
    manualUntil: row.manualUntil ?? null,
  });
}

export async function getMyPresence(
  etablissementId: string,
  userId: string,
): Promise<MessagingMyPresenceDto> {
  const db = getDb();
  const rows = await db
    .select({
      status: messagingPresence.status,
      connections: messagingPresence.connections,
      lastSeenAt: messagingPresence.lastSeenAt,
      manualStatus: messagingPresence.manualStatus,
      manualUntil: messagingPresence.manualUntil,
      etablissementId: messagingPresence.etablissementId,
    })
    .from(messagingPresence)
    .where(eq(messagingPresence.userId, userId))
    .limit(1);
  const row = rows[0];
  if (!row) return toMyDto(null);
  return toMyDto(
    {
      status: row.status,
      connections: row.connections,
      lastSeenAt: row.lastSeenAt,
      manualStatus: row.manualStatus ?? null,
      manualUntil: row.manualUntil ?? null,
    },
    row.etablissementId === etablissementId,
  );
}

export async function getPresenceMap(
  etablissementId: string,
  userIds: string[],
): Promise<Record<string, MessagingPresenceStatus>> {
  const unique = [...new Set(userIds.map((id) => id.trim()).filter(Boolean))];
  const out: Record<string, MessagingPresenceStatus> = {};
  for (const id of unique) out[id] = "offline";
  if (unique.length === 0) return out;

  const db = getDb();
  const rows = await db
    .select({
      userId: messagingPresence.userId,
      status: messagingPresence.status,
      connections: messagingPresence.connections,
      lastSeenAt: messagingPresence.lastSeenAt,
      manualStatus: messagingPresence.manualStatus,
      manualUntil: messagingPresence.manualUntil,
    })
    .from(messagingPresence)
    .where(
      and(
        eq(messagingPresence.etablissementId, etablissementId),
        inArray(messagingPresence.userId, unique),
      ),
    );

  const now = Date.now();
  for (const row of rows) {
    out[row.userId] = effectiveStatus(
      {
        status: row.status,
        connections: row.connections,
        lastSeenAt: row.lastSeenAt,
        manualStatus: row.manualStatus ?? null,
        manualUntil: row.manualUntil ?? null,
      },
      now,
    );
  }
  return out;
}
