import "server-only";

import { and, eq, notInArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { reservationRoom, reservationRoomBooking } from "@/db/schema";
import {
  isEntCoreDbEnabled,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";

export type ReservationRoomRow = {
  id: string;
  name: string;
  building?: string | null;
  sortOrder?: number;
  [key: string]: unknown;
};

export async function reservationRoomsDbReady(): Promise<string | null> {
  if (!isEntCoreDbEnabled()) return null;
  return resolveCurrentEtablissementId();
}

function rowToBooking(r: typeof reservationRoomBooking.$inferSelect): RoomReservationRow {
  return {
    id: r.id,
    roomId: r.roomId,
    startsAt: r.startsAt,
    endsAt: r.endsAt,
    status: r.status,
    ...(r.groupId ? { groupId: r.groupId } : { groupId: null }),
    ...(r.userId ? { userId: r.userId } : {}),
    ...(r.email ? { email: r.email } : {}),
    ...(r.subject ? { subject: r.subject } : {}),
    ...(r.className ? { className: r.className } : {}),
    ...(r.comment ? { comment: r.comment } : {}),
    ...(r.firstName ? { firstName: r.firstName } : {}),
    ...(r.lastName ? { lastName: r.lastName } : {}),
    ...(r.bookedByFirstName ? { bookedByFirstName: r.bookedByFirstName } : {}),
    ...(r.bookedByLastName ? { bookedByLastName: r.bookedByLastName } : {}),
    ...(r.bookedByUserId ? { bookedByUserId: r.bookedByUserId } : {}),
    bookedForOther: Boolean(r.bookedForOther),
    ...(r.cancelledAt ? { cancelledAt: r.cancelledAt } : {}),
    ...(r.cancelledBy ? { cancelledBy: r.cancelledBy } : {}),
    ...(r.cancelReason ? { cancelReason: r.cancelReason } : {}),
    createdAt: r.createdAt.toISOString(),
  };
}

export async function listReservationRoomsFromDb(
  etablissementId: string,
): Promise<ReservationRoomRow[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(reservationRoom)
    .where(eq(reservationRoom.etablissementId, etablissementId));
  return [...rows]
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name, "fr"))
    .map((r) => ({
      id: r.id,
      name: r.name,
      ...(r.building ? { building: r.building } : {}),
      sortOrder: r.sortOrder,
    }));
}

/** Remplace le catalogue salles du tenant : upsert + suppression ciblée des absents. */
export async function replaceReservationRoomsInDb(
  etablissementId: string,
  rooms: ReservationRoomRow[],
): Promise<void> {
  const db = getDb();
  const now = new Date();
  const normalized = rooms
    .map((r, i) => ({
      etablissementId,
      id: String(r.id || "").trim(),
      name: String(r.name || "").trim(),
      building: r.building ? String(r.building) : null,
      sortOrder: typeof r.sortOrder === "number" ? r.sortOrder : i,
      updatedAt: now,
    }))
    .filter((r) => r.id && r.name);

  for (const row of normalized) {
    await db
      .insert(reservationRoom)
      .values({ ...row, createdAt: now })
      .onConflictDoUpdate({
        target: [reservationRoom.etablissementId, reservationRoom.id],
        set: {
          name: row.name,
          building: row.building,
          sortOrder: row.sortOrder,
          updatedAt: row.updatedAt,
        },
      });
  }

  const ids = normalized.map((r) => r.id);
  if (ids.length === 0) {
    await db.delete(reservationRoom).where(eq(reservationRoom.etablissementId, etablissementId));
    return;
  }
  await db
    .delete(reservationRoom)
    .where(
      and(eq(reservationRoom.etablissementId, etablissementId), notInArray(reservationRoom.id, ids)),
    );
}

export async function listReservationBookingsFromDb(
  etablissementId: string,
): Promise<RoomReservationRow[]> {
  const db = getDb();
  const rows = await db
    .select()
    .from(reservationRoomBooking)
    .where(eq(reservationRoomBooking.etablissementId, etablissementId));
  return rows.map(rowToBooking);
}

export async function upsertReservationBookingInDb(
  etablissementId: string,
  booking: RoomReservationRow,
): Promise<void> {
  const db = getDb();
  const now = new Date();
  const createdAt = booking.createdAt ? new Date(String(booking.createdAt)) : now;
  const values = {
    etablissementId,
    id: String(booking.id),
    roomId: String(booking.roomId),
    groupId: booking.groupId ? String(booking.groupId) : null,
    userId: String(booking.userId ?? ""),
    firstName: String(booking.firstName ?? ""),
    lastName: String(booking.lastName ?? ""),
    bookedByFirstName: String(booking.bookedByFirstName ?? ""),
    bookedByLastName: String(booking.bookedByLastName ?? ""),
    bookedByUserId: booking.bookedByUserId ? String(booking.bookedByUserId) : null,
    bookedForOther: Boolean(booking.bookedForOther),
    email: booking.email ? String(booking.email) : null,
    subject: booking.subject ? String(booking.subject) : null,
    className: booking.className ? String(booking.className) : null,
    comment: booking.comment ? String(booking.comment) : null,
    startsAt: String(booking.startsAt),
    endsAt: String(booking.endsAt ?? booking.startsAt),
    status: String(booking.status ?? "CONFIRMED"),
    cancelledAt: booking.cancelledAt ? String(booking.cancelledAt) : null,
    cancelledBy: booking.cancelledBy ? String(booking.cancelledBy) : null,
    cancelReason: booking.cancelReason ? String(booking.cancelReason) : null,
    createdAt: Number.isNaN(createdAt.getTime()) ? now : createdAt,
    updatedAt: now,
  };

  await db
    .insert(reservationRoomBooking)
    .values(values)
    .onConflictDoUpdate({
      target: [reservationRoomBooking.etablissementId, reservationRoomBooking.id],
      set: {
        roomId: values.roomId,
        groupId: values.groupId,
        userId: values.userId,
        firstName: values.firstName,
        lastName: values.lastName,
        bookedByFirstName: values.bookedByFirstName,
        bookedByLastName: values.bookedByLastName,
        bookedByUserId: values.bookedByUserId,
        bookedForOther: values.bookedForOther,
        email: values.email,
        subject: values.subject,
        className: values.className,
        comment: values.comment,
        startsAt: values.startsAt,
        endsAt: values.endsAt,
        status: values.status,
        cancelledAt: values.cancelledAt,
        cancelledBy: values.cancelledBy,
        cancelReason: values.cancelReason,
        updatedAt: values.updatedAt,
      },
    });
}

/** Upsert unitaire — ne supprime jamais les absents de la liste. */
export async function upsertReservationBookingsInDb(
  etablissementId: string,
  bookings: RoomReservationRow[],
): Promise<void> {
  for (const b of bookings) {
    if (b?.id && b.roomId && b.startsAt) await upsertReservationBookingInDb(etablissementId, b);
  }
}

export async function ensureReservationRoomsMigratedFromCollection(
  etablissementId: string,
): Promise<ReservationRoomRow[]> {
  const existing = await listReservationRoomsFromDb(etablissementId);
  if (existing.length > 0) return existing;
  const { getCollectionSingleton } = await import("@/app/lib/ent-collection-db");
  const legacy = await getCollectionSingleton<{ rooms?: ReservationRoomRow[] } | ReservationRoomRow[]>(
    etablissementId,
    "reservation-rooms__rooms",
  );
  const rooms = Array.isArray(legacy)
    ? legacy
    : Array.isArray(legacy?.rooms)
      ? legacy.rooms
      : [];
  if (rooms.length === 0) return [];
  await replaceReservationRoomsInDb(etablissementId, rooms);
  return listReservationRoomsFromDb(etablissementId);
}

export async function ensureReservationBookingsMigratedFromCollection(
  etablissementId: string,
): Promise<RoomReservationRow[]> {
  const existing = await listReservationBookingsFromDb(etablissementId);
  if (existing.length > 0) return existing;
  const { getCollectionRecord, getCollectionSingleton } = await import(
    "@/app/lib/ent-collection-db"
  );
  const legacySingleton = await getCollectionSingleton<RoomReservationRow[] | Record<string, unknown>>(
    etablissementId,
    "reservation-rooms__reservations",
  );
  const legacyRecord = await getCollectionRecord<RoomReservationRow[] | Record<string, unknown>>(
    etablissementId,
    "reservation-rooms",
    "reservations",
  );
  const { normalizeRoomReservationsList } = await import(
    "@/app/lib/prof-room-reservations-normalize"
  );
  const rows = [
    ...normalizeRoomReservationsList(legacySingleton),
    ...normalizeRoomReservationsList(
      Array.isArray(legacyRecord)
        ? legacyRecord
        : legacyRecord && "__root" in legacyRecord
          ? (legacyRecord as { __root: unknown }).__root
          : legacyRecord,
    ),
  ];
  if (rows.length === 0) return [];
  await upsertReservationBookingsInDb(etablissementId, rows);
  return listReservationBookingsFromDb(etablissementId);
}
