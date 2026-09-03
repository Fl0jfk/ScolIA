import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { requireProfRoomModuleAdmin } from "@/app/lib/prof-room-auth";
import {
  listReservationRooms,
  saveReservationRooms,
} from "@/app/lib/reservation-rooms-storage";

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const rooms = await listReservationRooms();
    return NextResponse.json({ rooms });
  } catch (err: unknown) {
    console.error("Erreur route /rooms:", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Erreur serveur lors du chargement des salles" },
      { status: 500 },
    );
  }
}

export async function PUT(req: Request) {
  const gate = await requireProfRoomModuleAdmin();
  if (!gate.ok) return gate.response;
  try {
    const body = await req.json();
    const rooms = Array.isArray(body?.rooms) ? body.rooms : body;
    if (!Array.isArray(rooms)) {
      return NextResponse.json({ error: "Format invalide : attendu { rooms: [...] }" }, { status: 400 });
    }
    const saved = await saveReservationRooms(rooms);
    return NextResponse.json({ success: true, rooms: saved });
  } catch (err: unknown) {
    console.error("PUT /rooms:", err);
    return NextResponse.json({ error: "Impossible d'enregistrer les salles." }, { status: 500 });
  }
}
