import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getJson } from "@/app/lib/s3-storage";
import { sendPanierRepasListForTrip } from "@/app/lib/travels-panier-repas-send";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import type { TravelsParticipantEleve, TravelsTrip } from "@/app/lib/travels-types";

/**
 * Envoie la liste nominative des élèves avec panier repas (+ PDF commande cuisine)
 * aux destinataires configurés (notifications.travelsCuisineListePaniers).
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const user = await safeCurrentUser();
    const userName =
      user?.fullName || access.user.fullName || trip.ownerName || "Organisateur";
    const userEmail =
      user?.primaryEmailAddress?.emailAddress ||
      access.user.primaryEmailAddress?.emailAddress ||
      trip.ownerEmail;

    const result = await sendPanierRepasListForTrip({
      trip,
      tripId,
      participants: Array.isArray(body.participantEleves)
        ? (body.participantEleves as TravelsParticipantEleve[])
        : undefined,
      userName,
      userEmail,
      attachCuisineOrderPdf: true,
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({
      ok: true,
      trip: result.trip,
      sentTo: result.sentTo,
      count: result.count,
      mealsOrdered: result.mealsOrdered,
      remaining: Math.max(0, result.mealsOrdered - result.count),
    });
  } catch (e) {
    console.error("[send-panier-repas-list]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Envoi impossible" },
      { status: 500 },
    );
  }
}
