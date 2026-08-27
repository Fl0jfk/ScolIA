import { NextResponse } from "next/server";
import { getJson } from "@/app/lib/s3-storage";
import { requireAuth } from "@/app/lib/intranet-auth";
import { normalizeRoomReservationsList } from "@/app/lib/prof-room-reservations-normalize";

const RESERVATIONS_KEY = "reservation-rooms/reservations.json";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const hit = await getJson<unknown>(RESERVATIONS_KEY);
    const reservations = normalizeRoomReservationsList(hit?.data);
    return NextResponse.json({ reservations });
  } catch (err: unknown) {
    console.error(err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Erreur serveur" },
      { status: 500 },
    );
  }
}
