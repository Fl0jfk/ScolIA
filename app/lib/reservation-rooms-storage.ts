import "server-only";

import type { RoomReservationRow } from "@/app/lib/prof-room-reservations-normalize";
import {
  ensureReservationBookingsMigratedFromCollection,
  ensureReservationRoomsMigratedFromCollection,
  replaceReservationRoomsInDb,
  reservationRoomsDbReady,
  type ReservationRoomRow,
  upsertReservationBookingInDb,
  upsertReservationBookingsInDb,
} from "@/app/lib/reservation-rooms-db";

export async function listReservationRooms(): Promise<ReservationRoomRow[]> {
  const etabId = await reservationRoomsDbReady();
  if (!etabId) return [];
  return ensureReservationRoomsMigratedFromCollection(etabId);
}

export async function saveReservationRooms(rooms: ReservationRoomRow[]): Promise<ReservationRoomRow[]> {
  const etabId = await reservationRoomsDbReady();
  if (!etabId) throw new Error("[reservation-rooms] Postgres requis");
  await replaceReservationRoomsInDb(etabId, rooms);
  return listReservationRooms();
}

export async function listReservationBookings(): Promise<RoomReservationRow[]> {
  const etabId = await reservationRoomsDbReady();
  if (!etabId) return [];
  return ensureReservationBookingsMigratedFromCollection(etabId);
}

export async function saveReservationBooking(booking: RoomReservationRow): Promise<void> {
  const etabId = await reservationRoomsDbReady();
  if (!etabId) throw new Error("[reservation-rooms] Postgres requis");
  await upsertReservationBookingInDb(etabId, booking);
}

/** Upsert unitaire de chaque ligne — jamais de wipe de la table. */
export async function saveReservationBookings(bookings: RoomReservationRow[]): Promise<void> {
  const etabId = await reservationRoomsDbReady();
  if (!etabId) throw new Error("[reservation-rooms] Postgres requis");
  await upsertReservationBookingsInDb(etabId, bookings);
}
