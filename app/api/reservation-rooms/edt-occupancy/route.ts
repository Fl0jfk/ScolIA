import { NextRequest, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { listReservationRooms } from "@/app/lib/reservation-rooms-storage";
import {
  loadEdtRoomOccupancy,
  occupancyRangeForWeek,
} from "@/app/lib/rh/planning-room-occupancy";

export async function GET(req: NextRequest) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const { searchParams } = req.nextUrl;
    const roomId = searchParams.get("roomId")?.trim() || "";
    if (!roomId) {
      return NextResponse.json({ error: "roomId requis." }, { status: 400 });
    }

    const rooms = await listReservationRooms();
    const room = rooms.find((r) => r.id === roomId);
    if (!room) {
      return NextResponse.json({ error: "Salle introuvable." }, { status: 404 });
    }

    let from = searchParams.get("from")?.trim() || "";
    let to = searchParams.get("to")?.trim() || "";
    if (!from || !to) {
      const range = occupancyRangeForWeek(new Date());
      from = from || range.from;
      to = to || range.to;
    }

    const occupancy = await loadEdtRoomOccupancy({ room, from, to });
    return NextResponse.json({ occupancy });
  } catch (err) {
    console.error("[reservation-rooms/edt-occupancy]", err);
    return NextResponse.json(
      { error: "Impossible de charger l’occupation EDT." },
      { status: 500 },
    );
  }
}
