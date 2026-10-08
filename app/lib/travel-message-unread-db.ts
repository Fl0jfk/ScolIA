import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { travelMessage, travelMessageRead } from "@/db/schema";
import type { Establishment } from "@/app/lib/app-config-schemas";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  countUnreadTravelMessages,
  type TravelThreadViewer,
  viewerIsTravelThreadAudience,
} from "@/app/lib/travels-thread-unread";

export async function markTravelThreadRead(params: {
  etablissementId: string;
  travelId: string;
  userId: string;
  at?: Date;
}): Promise<void> {
  const userId = params.userId.trim();
  const travelId = params.travelId.trim();
  if (!userId || !travelId) return;
  const lastReadAt = params.at ?? new Date();
  try {
    const db = getDb();
    await db
      .insert(travelMessageRead)
      .values({
        etablissementId: params.etablissementId,
        travelId,
        userId,
        lastReadAt,
      })
      .onConflictDoUpdate({
        target: [
          travelMessageRead.etablissementId,
          travelMessageRead.travelId,
          travelMessageRead.userId,
        ],
        set: { lastReadAt },
      });
  } catch (err) {
    console.warn("[travels] markTravelThreadRead:", err);
  }
}

export async function insertTravelInternalMessage(params: {
  etablissementId: string;
  travelId: string;
  authorUserId: string;
  userLabel: string;
  role: string;
  text: string;
}): Promise<{
  id: string;
  user: string;
  role: string;
  text: string;
  date: string;
  authorUserId: string;
}> {
  const db = getDb();
  const travelId = params.travelId.trim();
  const [maxRow] = await db
    .select({
      maxOrder: sql<number>`coalesce(max(${travelMessage.sortOrder}), -1)`,
    })
    .from(travelMessage)
    .where(
      and(
        eq(travelMessage.etablissementId, params.etablissementId),
        eq(travelMessage.travelId, travelId),
      ),
    );
  const sortOrder = Number(maxRow?.maxOrder ?? -1) + 1;
  const date = new Date().toISOString();
  const id = `${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  await db.insert(travelMessage).values({
    id,
    etablissementId: params.etablissementId,
    travelId,
    userLabel: params.userLabel,
    role: params.role,
    body: params.text,
    at: date,
    sortOrder,
  });
  try {
    await markTravelThreadRead({
      etablissementId: params.etablissementId,
      travelId,
      userId: params.authorUserId,
      at: new Date(date),
    });
  } catch (err) {
    console.warn("[travels] mark thread read after post:", err);
  }
  void import("@/app/lib/valkey")
    .then(async ({ valkeyDel }) => {
      const { valkeyKeyTravelsIndex, valkeyKeyTravelTrip } = await import(
        "@/app/lib/valkey-keys"
      );
      await valkeyDel(
        valkeyKeyTravelsIndex(params.etablissementId),
        valkeyKeyTravelTrip(params.etablissementId, travelId),
      );
    })
    .catch(() => undefined);
  return {
    id,
    user: params.userLabel,
    role: params.role,
    text: params.text,
    date,
    authorUserId: params.authorUserId,
  };
}

async function lastReadByTripForUser(params: {
  etablissementId: string;
  userId: string;
}): Promise<Map<string, Date>> {
  const db = getDb();
  const rows = await db
    .select({
      travelId: travelMessageRead.travelId,
      lastReadAt: travelMessageRead.lastReadAt,
    })
    .from(travelMessageRead)
    .where(
      and(
        eq(travelMessageRead.etablissementId, params.etablissementId),
        eq(travelMessageRead.userId, params.userId),
      ),
    );
  const map = new Map<string, Date>();
  for (const row of rows) {
    if (row.lastReadAt) map.set(row.travelId, row.lastReadAt);
  }
  return map;
}

export async function unreadCountsByTravelId(params: {
  etablissementId: string;
  trips: TravelsTrip[];
  viewer: TravelThreadViewer;
  establishments: Establishment[];
}): Promise<Map<string, number>> {
  const userId = String(params.viewer.user?.id || "").trim();
  const out = new Map<string, number>();
  if (!userId) return out;

  const audienceIds = params.trips
    .filter((trip) => viewerIsTravelThreadAudience(trip, params.viewer, params.establishments))
    .map((trip) => trip.id);
  if (audienceIds.length === 0) return out;

  try {
    const audience = new Set(audienceIds);
    const db = getDb();
    const [messages, reads] = await Promise.all([
      db
        .select({
          travelId: travelMessage.travelId,
          at: travelMessage.at,
        })
        .from(travelMessage)
        .where(eq(travelMessage.etablissementId, params.etablissementId)),
      lastReadByTripForUser({ etablissementId: params.etablissementId, userId }),
    ]);

    const byTrip = new Map<string, Array<{ date?: string }>>();
    for (const row of messages) {
      if (!audience.has(row.travelId)) continue;
      const list = byTrip.get(row.travelId);
      const item = { date: row.at };
      if (list) list.push(item);
      else byTrip.set(row.travelId, [item]);
    }

    const viewerUserIds = [
      userId,
      ...(params.viewer.extraUserIds ?? []).filter((id: string) => Boolean(id.trim())),
    ];
    for (const travelId of audienceIds) {
      const n = countUnreadTravelMessages({
        messages: byTrip.get(travelId) ?? [],
        lastReadAt: reads.get(travelId) ?? null,
        viewerUserIds,
      });
      if (n > 0) out.set(travelId, n);
    }
  } catch (err) {
    console.warn("[travels] unread counts unavailable:", err);
  }
  return out;
}

export async function attachTravelUnreadCounts(params: {
  etablissementId: string;
  trips: TravelsTrip[];
  viewer: TravelThreadViewer;
  establishments: Establishment[];
}): Promise<TravelsTrip[]> {
  const counts = await unreadCountsByTravelId(params);
  return params.trips.map((trip) => ({
    ...trip,
    unreadInternalCount: counts.get(trip.id) ?? 0,
  }));
}

export type TravelUnreadTripSummary = {
  tripId: string;
  messageCount: number;
  title: string;
  etablissement: string | null;
};

export type TravelUnreadSummary = {
  tripCount: number;
  messageCount: number;
  firstTripId: string | null;
  /** Un entrée par séjour concerné (pour notifs dashboard par voyage). */
  trips: TravelUnreadTripSummary[];
};

export async function summarizeViewerTravelUnread(params: {
  etablissementId: string;
  trips: TravelsTrip[];
  viewer: TravelThreadViewer;
  establishments: Establishment[];
}): Promise<TravelUnreadSummary> {
  const counts = await unreadCountsByTravelId(params);
  const byId = new Map(params.trips.map((t) => [t.id, t]));
  const trips: TravelUnreadTripSummary[] = [];
  let messageCount = 0;
  for (const [tripId, n] of counts) {
    if (n <= 0) continue;
    messageCount += n;
    const trip = byId.get(tripId);
    trips.push({
      tripId,
      messageCount: n,
      title: String(trip?.data?.title || trip?.data?.destination || "Séjour").trim() || "Séjour",
      etablissement: trip?.data?.etablissement?.trim() || null,
    });
  }
  trips.sort((a, b) => b.messageCount - a.messageCount || a.title.localeCompare(b.title, "fr"));
  return {
    tripCount: trips.length,
    messageCount,
    firstTripId: trips[0]?.tripId ?? null,
    trips,
  };
}
