import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { complexNeedsBus } from "@/app/lib/travels-trip-helpers";
import {
  mergeTransportRequestForRequalify,
  normalizeDatesForComplexTrip,
  resolveRequalifyToBusStatus,
} from "@/app/lib/travels-requalify-to-bus";
import {
  buildInitialTransportQuoteSnapshot,
  sendInitialTransportQuotes,
} from "@/app/lib/travels-send-initial-transport";
import type { TravelsTrip } from "@/app/lib/travels-types";

const BLOCKED_STATUSES = new Set(["ANNULE", "SEANCE_ANNULEE", "REJETE"]);

type TransportRequestBody = {
  pickupPoint?: string;
  stayOnSite?: boolean;
  freeText?: string;
};

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    if (!tripId) {
      return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    }

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) {
      return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
    }

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) {
      return NextResponse.json({ error: access.error }, { status: access.status });
    }

    if (BLOCKED_STATUSES.has(String(trip.status))) {
      return NextResponse.json(
        { error: "Impossible de requalifier un dossier annulé ou refusé." },
        { status: 400 },
      );
    }

    if (complexNeedsBus(trip)) {
      return NextResponse.json(
        { error: "Ce dossier est déjà un voyage / sortie bus avec demande de devis." },
        { status: 400 },
      );
    }

    if (trip.type !== "SIMPLE" && trip.type !== "COMPLEX") {
      return NextResponse.json({ error: "Type de dossier non pris en charge." }, { status: 400 });
    }

    const transportRequest = mergeTransportRequestForRequalify(
      trip.data.transportRequest,
      body.transportRequest as TransportRequestBody | undefined,
    );

    const now = new Date().toISOString();
    const actor = access.user.fullName || "Administration";
    const fromLabel = trip.type === "SIMPLE" ? "sortie de proximité" : "voyage sans bus";

    const {
      transportPhaseBypassedAt: _bypassAt,
      transportPhaseBypassedBy: _bypassBy,
      transportPhaseBypassNote: _bypassNote,
      ...dataWithoutBypass
    } = trip.data;

    let nextData = normalizeDatesForComplexTrip({
      ...dataWithoutBypass,
      needsBus: true,
      transportRequest,
    });

    const statusResolve = resolveRequalifyToBusStatus(
      String(trip.status),
      typeof trip.data.previousStatus === "string" ? trip.data.previousStatus : undefined,
    );
    if (statusResolve.previousStatus) {
      nextData = { ...nextData, previousStatus: statusResolve.previousStatus };
    }

    const mailResult = await sendInitialTransportQuotes({
      tripId,
      data: nextData,
      userName: actor,
    });
    if (!mailResult.ok) {
      return NextResponse.json({ error: mailResult.error }, { status: mailResult.status });
    }

    nextData = {
      ...nextData,
      transportQuoteSnapshot: buildInitialTransportQuoteSnapshot(nextData, now),
    };

    const updatedTrip: TravelsTrip = {
      ...trip,
      type: "COMPLEX",
      status: statusResolve.status,
      updatedAt: now,
      data: nextData,
      history: [
        ...(trip.history || []),
        {
          date: now,
          user: actor,
          action: "REQUALIFIE_VOYAGE_BUS",
          note: `Requalifié ${fromLabel} → voyage / sortie bus (demande de devis envoyée)`,
        },
        ...(statusResolve.status !== trip.status
          ? [
              {
                date: now,
                user: actor,
                action: statusResolve.status,
                note: "Statut ajusté pour le circuit devis transport",
              },
            ]
          : []),
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);

    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    if (index.length > 0) {
      await putJson(
        "travels/index.json",
        index.map((t) =>
          t.id === tripId
            ? {
                ...t,
                type: "COMPLEX",
                status: updatedTrip.status,
                data: {
                  ...t.data,
                  needsBus: true,
                  startDate: nextData.startDate,
                  endDate: nextData.endDate,
                  transportRequest: nextData.transportRequest,
                },
              }
            : t,
        ),
      );
    }

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      emailsAttempted: mailResult.emailsAttempted,
      emailsFailed: mailResult.emailsFailed,
    });
  } catch (e) {
    console.error("[requalify-to-bus]", e);
    return NextResponse.json({ error: "Requalification impossible" }, { status: 500 });
  }
}
