import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import { resolveTravelsCuisineEmails } from "@/app/lib/app-config-schemas";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { buildTravelsMailPreviewFromConfig, type MailPreviewType } from "@/app/lib/travels-mail-preview";
import type { TravelsTrip } from "@/app/lib/travels-types";

const VALID_TYPES: MailPreviewType[] = [
  "transport_amendment",
  "transport_initial",
  "cuisine_initial",
  "cuisine_amendment",
  "cancel_trip_transport",
  "cancel_trip_cuisine",
];

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    const type = body.type as MailPreviewType;
    if (!tripId || !VALID_TYPES.includes(type)) {
      return NextResponse.json({ error: "tripId et type valides requis" }, { status: 400 });
    }

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const config = await loadAppConfig();
    const chefEmails = resolveTravelsCuisineEmails(config.notifications, "");

    const preview = await buildTravelsMailPreviewFromConfig(trip, type, {
      userName: body.userName || trip.ownerName,
      chefEmails,
    });

    return NextResponse.json({ preview });
  } catch (e) {
    console.error("[mail-preview]", e);
    return NextResponse.json({ error: "Prévisualisation impossible" }, { status: 500 });
  }
}
