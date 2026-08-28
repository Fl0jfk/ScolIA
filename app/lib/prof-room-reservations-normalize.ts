/** Normalise la liste des réservations salles (données EAV parfois incomplètes). */

export type RoomReservationRow = {
  id: string;
  roomId: string;
  startsAt: string;
  endsAt?: string;
  status?: string;
  groupId?: string | null;
  userId?: string;
  email?: string;
  subject?: string;
  className?: string;
  comment?: string;
  firstName?: string;
  lastName?: string;
  bookedByFirstName?: string;
  bookedByLastName?: string;
  bookedForOther?: boolean;
  bookedByUserId?: string;
  createdAt?: string;
  [key: string]: unknown;
};

export function normalizeRoomReservationsList(raw: unknown): RoomReservationRow[] {
  if (!Array.isArray(raw)) return [];
  const out: RoomReservationRow[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const r = item as Record<string, unknown>;
    const id = typeof r.id === "string" ? r.id : "";
    const roomId = typeof r.roomId === "string" ? r.roomId : "";
    const startsAt = typeof r.startsAt === "string" ? r.startsAt : "";
    if (!id || !roomId || !startsAt) continue;
    out.push({
      ...r,
      id,
      roomId,
      startsAt,
      endsAt: typeof r.endsAt === "string" ? r.endsAt : undefined,
      status: typeof r.status === "string" ? r.status : undefined,
    });
  }
  return out;
}

/** Préfixe créneau `YYYY-MM-DDTHH` pour matcher une réservation. */
export function reservationMatchesHourPrefix(
  reservation: { startsAt?: string | null; roomId?: string | null; status?: string | null },
  roomId: string,
  hourPrefix: string,
): boolean {
  const startsAt = typeof reservation.startsAt === "string" ? reservation.startsAt : "";
  return (
    Boolean(startsAt) &&
    reservation.roomId === roomId &&
    startsAt.startsWith(hourPrefix) &&
    reservation.status !== "CANCELLED"
  );
}
