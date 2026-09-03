import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { normalizeRoomReservationsList } from "@/app/lib/prof-room-reservations-normalize";
import { listReservationBookings } from "@/app/lib/reservation-rooms-storage";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const reservations = normalizeRoomReservationsList(await listReservationBookings());
    return NextResponse.json({ reservations });
  } catch (err: unknown) {
    console.error(err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Erreur serveur" },
      { status: 500 },
    );
  }
}
