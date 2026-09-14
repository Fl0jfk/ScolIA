import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { getJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import {
  sendCuisineOrderForTrip,
  type CuisineTripRecord,
} from "@/app/lib/travels-cuisine-send";
import type { TravelsTrip } from "@/app/lib/travels-types";

export async function POST(req: Request) {
  try {
    const body = await req.json();
    const mode: "initial" | "amendment" = body.mode === "amendment" ? "amendment" : "initial";
    const tripId = String(body.tripId || body.tripData?.id || "");

    let trip: CuisineTripRecord | null = body.tripData || null;
    if (tripId) {
      const hit = await getJson<CuisineTripRecord>(`travels/${tripId}.json`);
      if (hit?.data) trip = hit.data;
    }
    if (!trip?.data) {
      return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
    }

    const access = await assertTravelsTripAccess(trip as TravelsTrip, {
      requireOwnerOrDirection: true,
    });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const user = await safeCurrentUser();
    const userName = body.userName || trip.ownerName || user?.fullName || "La Providence";
    const userEmail = body.userEmail || user?.primaryEmailAddress?.emailAddress;
    const organizerEmail = body.organizerEmail || trip.ownerEmail;

    const result = await sendCuisineOrderForTrip({
      trip,
      tripId: tripId || String(trip.id || ""),
      mode,
      userName,
      userEmail,
      organizerEmail,
      requireListeElevesConfirmed: mode === "initial",
    });

    if (!result.ok) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json({ success: true, trip: result.trip, mode: result.mode });
  } catch (error) {
    console.error("Erreur envoi mail cuisine:", error);
    return NextResponse.json({ error: "Erreur lors de l'envoi du mail" }, { status: 500 });
  }
}
