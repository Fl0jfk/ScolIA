import { NextResponse, NextRequest } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";

const RESERVATIONS_KEY = "reservation-rooms/reservations.json";

type ReservationRow = {
  id: string;
  roomId?: string;
  groupId?: string;
  status?: string;
  startsAt: string;
  endsAt: string;
  subject?: string;
  className?: string;
  comment?: string;
};

export async function POST(req: NextRequest) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const body = (await req.json()) as {
      id?: string;
      newHour?: number;
      date?: string;
      updateAllSeries?: boolean;
      subject?: string;
      className?: string;
      comment?: string;
    };
    const { id, newHour, date, updateAllSeries, subject, className, comment } = body;
    if (!id || newHour === undefined || newHour === null || Number.isNaN(Number(newHour))) {
      return NextResponse.json({ error: "Identifiant et nouvel horaire requis." }, { status: 400 });
    }

    const hour = Number(newHour);
    const hit = await getJson<ReservationRow[]>(RESERVATIONS_KEY);
    const existing: ReservationRow[] = Array.isArray(hit?.data) ? hit.data : [];
    const originalRes = existing.find((r) => r.id === id);
    if (!originalRes) {
      return NextResponse.json({ error: "Réservation introuvable." }, { status: 404 });
    }

    const reservationsToUpdate =
      updateAllSeries && originalRes.groupId
        ? existing.filter((r) => r.groupId === originalRes.groupId && r.status !== "CANCELLED")
        : [originalRes];

    for (const res of reservationsToUpdate) {
      const baseDate = !updateAllSeries && date ? date : res.startsAt.split("T")[0];
      const tempStart = `${baseDate}T${hour.toString().padStart(2, "0")}:30:00`;
      const tempEnd = `${baseDate}T${(hour + 1).toString().padStart(2, "0")}:30:00`;
      const conflict = existing.some(
        (ext) =>
          !reservationsToUpdate.some((u) => u.id === ext.id) &&
          ext.roomId === res.roomId &&
          ext.status !== "CANCELLED" &&
          ext.startsAt.substring(0, 19) < tempEnd &&
          ext.endsAt.substring(0, 19) > tempStart,
      );
      if (conflict) {
        return NextResponse.json(
          { error: "Conflit d'horaire détecté pour un des créneaux." },
          { status: 409 },
        );
      }
    }

    for (const res of reservationsToUpdate) {
      const idx = existing.findIndex((r) => r.id === res.id);
      if (idx === -1) continue;
      const baseDate = !updateAllSeries && date ? date : res.startsAt.split("T")[0];
      existing[idx] = {
        ...existing[idx],
        startsAt: `${baseDate}T${hour.toString().padStart(2, "0")}:30:00`,
        endsAt: `${baseDate}T${(hour + 1).toString().padStart(2, "0")}:30:00`,
        ...(subject ? { subject } : {}),
        ...(className ? { className } : {}),
        ...(comment !== undefined ? { comment } : {}),
      };
    }

    await putJson(RESERVATIONS_KEY, existing);
    return NextResponse.json({ success: true });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Erreur serveur";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
