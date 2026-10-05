import { NextResponse } from "next/server";
import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import {
  buildInitialTransportQuoteSnapshot,
  sendInitialTransportQuotes,
} from "@/app/lib/travels-send-initial-transport";
import type { TravelsTrip } from "@/app/lib/travels-types";

/**
 * Relance les demandes de devis bus (mêmes documents que l’envoi initial)
 * auprès des transporteurs qui n’ont pas encore de devis rattaché au dossier.
 */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const session = await resolveSession();
  if (!session?.userId) {
    return NextResponse.json({ error: "Non autorisé" }, { status: 401 });
  }

  try {
    const body = await req.json().catch(() => ({}));
    const tripId = String((body as { tripId?: unknown }).tripId || "").trim();
    if (!tripId) {
      return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    }

    const forceAll = Boolean((body as { forceAll?: unknown }).forceAll);

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) {
      return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
    }

    // Créateur, direction établissement, administratif / admin général (et compta via le même garde).
    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) {
      return NextResponse.json({ error: access.error }, { status: access.status });
    }

    const data = trip.data;
    if (!data.needsBus) {
      return NextResponse.json({ error: "Ce voyage n'a pas de transport bus." }, { status: 400 });
    }

    if (data.signedQuoteUrl) {
      return NextResponse.json(
        {
          error:
            "Un devis est déjà signé. Utilisez « Devis rectifié (effectif) » pour un avenant, pas une relance initiale.",
        },
        { status: 400 },
      );
    }

    const user = await safeCurrentUser();
    const userName =
      String((body as { userName?: unknown }).userName || "").trim() ||
      user?.fullName ||
      "La Providence";

    const result = await sendInitialTransportQuotes({
      tripId,
      data,
      userName,
      kind: "reminder",
      forceAllProviders: forceAll,
      receivedDevis: trip.receivedDevis,
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    if (result.emailsAttempted > 0 && result.emailsFailed >= result.emailsAttempted) {
      return NextResponse.json(
        { error: "Échec d'envoi à tous les transporteurs (SMTP).", details: result },
        { status: 502 },
      );
    }

    const now = new Date().toISOString();
    const historyAction =
      result.skippedAlreadyQuoted > 0
        ? `Relance demande de devis transport envoyée (${result.sentTo.length} transporteur${result.sentTo.length > 1 ? "s" : ""}, ${result.skippedAlreadyQuoted} déjà devis ignoré${result.skippedAlreadyQuoted > 1 ? "s" : ""})`
        : `Relance demande de devis transport envoyée à ${result.sentTo.length} transporteur${result.sentTo.length > 1 ? "s" : ""}`;

    const updatedTrip: TravelsTrip = {
      ...trip,
      data: {
        ...data,
        transportQuoteSnapshot: buildInitialTransportQuoteSnapshot(data, now, "reminder"),
        transportDateSnapshot: {
          startDate: data.startDate || data.date,
          endDate: data.endDate,
          startTime: data.startTime,
          endTime: data.endTime,
          sentAt: now,
        },
      },
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: userName,
          action: historyAction,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);

    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    const nextIndex = index.map((t) => (t.id === tripId ? { ...t, ...updatedTrip } : t));
    await putJson("travels/index.json", nextIndex);

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      sentTo: result.sentTo,
      emailsAttempted: result.emailsAttempted,
      emailsFailed: result.emailsFailed,
      skippedAlreadyQuoted: result.skippedAlreadyQuoted,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    console.error("[remind-transport-quotes]", msg);
    return NextResponse.json(
      { error: "Échec de la relance des demandes de devis", details: msg },
      { status: 500 },
    );
  }
}
