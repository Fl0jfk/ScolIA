import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  buildInitialTransportQuoteSnapshot,
  sendInitialTransportQuotes,
} from "@/app/lib/travels-send-initial-transport";

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) {
    return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }
  try {
    const body = await req.json();
    const { tripData, userName } = body;
    const data = tripData?.data;
    if (!tripData?.id || !data || typeof data !== "object") {
      return NextResponse.json({ error: "Données voyage invalides" }, { status: 400 });
    }

    // NJ-11 : sur un dossier existant, seuls le porteur, la direction, la comptabilité et l'admin déclenchent l'envoi.
    const stored = await getJson<TravelsTrip>(`travels/${String(tripData.id)}.json`);
    if (stored?.data) {
      const access = await assertTravelsTripAccess(stored.data, { requireOwnerOrDirection: true });
      if (!access.ok) {
        return NextResponse.json({ error: access.error }, { status: access.status });
      }
    }
    const result = await sendInitialTransportQuotes({
      tripId: String(tripData.id),
      data,
      userName: String(userName || "Administration"),
    });
    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    const tripId = String(tripData.id);
    try {
      const hit = await getJson<Record<string, unknown>>(`travels/${tripId}.json`);
      const existing = hit?.data;
      if (existing && typeof existing === "object") {
        const inner = (existing as { data?: Record<string, unknown> }).data || {};
        const now = new Date().toISOString();
        const updated = {
          ...existing,
          data: {
            ...inner,
            transportQuoteSnapshot: buildInitialTransportQuoteSnapshot(
              data,
              now,
            ),
          },
        };
        await putJson(`travels/${tripId}.json`, updated);
      }
    } catch (snapErr) {
      console.error("[send-transport] snapshot", snapErr);
    }

    return NextResponse.json({ success: true });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    console.error("Erreur API Gmail/Transport:", msg);
    return NextResponse.json({ error: "Échec de l'envoi", details: msg }, { status: 500 });
  }
}
