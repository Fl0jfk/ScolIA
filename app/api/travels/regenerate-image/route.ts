import { NextResponse } from "next/server";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { selectTravelCoverImage } from "@/app/lib/travels-select-cover-image";
import { normalizeTripImageFields } from "@/app/lib/travels-image-url";
import type { TravelsTrip } from "@/app/lib/travels-types";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    if (!tripId) {
      return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    }

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip) {
      return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });
    }

    const title = String(trip.data?.title || "Titre introuvable");
    const destination = String(trip.data?.destination || "Destination introuvable");
    const previousConfigId =
      typeof trip.imageConfigId === "string" ? trip.imageConfigId : null;

    const selected = await selectTravelCoverImage({
      title,
      destination,
      excludeId: previousConfigId,
    });

    const me = await safeCurrentUser();
    const actor = me?.fullName || me?.primaryEmailAddress?.emailAddress || "Admin";
    const now = new Date().toISOString();

    const updatedTrip: TravelsTrip = {
      ...trip,
      imageUrl: selected.url,
      imageConfigId: selected.id,
      updatedAt: now,
      history: [
        ...(trip.history || []),
        {
          date: now,
          user: actor,
          action: "IMAGE_REGENEREE",
          note: `Image de présentation régénérée (IA) → ${selected.label || selected.id}`,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);

    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) => {
        if (t.id !== tripId) return t;
        return {
          ...t,
          imageUrl: selected.url,
          imageConfigId: selected.id,
          updatedAt: now,
          data: {
            ...t.data,
            imageUrl: selected.url,
          },
        };
      }),
    );

    return NextResponse.json({
      success: true,
      trip: normalizeTripImageFields(updatedTrip),
      imageUrl: selected.url,
      imageConfigId: selected.id,
      imageLabel: selected.label,
    });
  } catch (e) {
    console.error("[travels/regenerate-image]", e);
    return NextResponse.json({ error: "Régénération impossible" }, { status: 500 });
  }
}
