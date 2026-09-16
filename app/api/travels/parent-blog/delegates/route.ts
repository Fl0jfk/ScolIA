import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson } from "@/app/lib/s3-storage";
import {
  assertParentBlogEditAccess,
  setParentBlogDelegates,
} from "@/app/lib/travels-parent-blog";
import type { TravelsParentBlogDelegate, TravelsTrip } from "@/app/lib/travels-types";
import { isTripOwnerOrCreator } from "@/app/lib/travels-direction-permissions";
import { canSignTravelsDirectionForEtab } from "@/app/lib/establishments";

/** POST — définir les délégués (owner / direction uniquement). */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertParentBlogEditAccess(trip);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const isOwner = isTripOwnerOrCreator(trip, access.user);
    const canSign = await canSignTravelsDirectionForEtab(access.user, trip.data?.etablissement);
    if (!isOwner && !canSign) {
      return NextResponse.json(
        { error: "Seuls l’organisateur ou la direction peuvent gérer les délégations." },
        { status: 403 },
      );
    }

    const raw: TravelsParentBlogDelegate[] = Array.isArray(body.delegates) ? body.delegates : [];
    const updated = await setParentBlogDelegates({
      trip,
      delegates: raw,
      actorName: access.user.fullName || "Équipe",
    });

    return NextResponse.json({
      success: true,
      trip: updated,
      delegates: updated.data.parentBlogDelegates || [],
    });
  } catch (e) {
    console.error("[parent-blog/delegates]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Enregistrement impossible." },
      { status: 500 },
    );
  }
}
