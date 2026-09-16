import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson } from "@/app/lib/s3-storage";
import { travelsDbReady } from "@/app/lib/travel-db";
import {
  activateParentBlog,
  assertParentBlogEditAccess,
  getParentBlogForTrip,
  isParentBlogPublicExpired,
  listParentBlogPosts,
  metaFromBlogRow,
} from "@/app/lib/travels-parent-blog";
import type { TravelsParentCalendar, TravelsTrip } from "@/app/lib/travels-types";

/** GET ?tripId= — état blog + posts (auth). */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const tripId = new URL(req.url).searchParams.get("tripId")?.trim() || "";
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const etabId = await travelsDbReady();
    if (!etabId) return NextResponse.json({ error: "Base voyages indisponible." }, { status: 503 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertParentBlogEditAccess(trip);
    if (!access.ok) {
      // Lecture : accès dossier suffit (owner/direction/admin déjà via assertTravelsTripAccess dans edit).
      // Si délégué seulement pour écriture — pour GET on autorise aussi consultation dossier classique.
      const { assertTravelsTripAccess } = await import("@/app/lib/travels-rbac-server");
      const view = await assertTravelsTripAccess(trip);
      if (!view.ok) return NextResponse.json({ error: view.error }, { status: view.status });
    }

    const blog = await getParentBlogForTrip(etabId, tripId);
    const posts = blog ? await listParentBlogPosts(etabId, tripId) : [];
    const expired = blog ? isParentBlogPublicExpired(blog.expiresAt) : false;

    return NextResponse.json({
      blog: blog ? metaFromBlogRow(blog) : null,
      expired,
      canEdit: access.ok && Boolean(blog?.enabled) && !expired,
      posts,
      delegates: trip.data.parentBlogDelegates || [],
    });
  } catch (e) {
    console.error("[parent-blog GET]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Impossible de charger le blog." },
      { status: 500 },
    );
  }
}

/** POST — activer le blog parents (+ mail si première notification). */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const etabId = await travelsDbReady();
    if (!etabId) return NextResponse.json({ error: "Base voyages indisponible." }, { status: 503 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertParentBlogEditAccess(trip);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const participants = trip.data.participantEleves || [];
    if (participants.length === 0) {
      return NextResponse.json(
        { error: "Composez d’abord la liste des élèves avant d’activer la page parents." },
        { status: 400 },
      );
    }

    const notifyParents = body.notifyParents !== false;
    const attachIcs = body.attachIcs !== false;
    const calendarPatch = body.parentCalendar as TravelsParentCalendar | undefined;

    const result = await activateParentBlog({
      etablissementId: etabId,
      trip,
      activatedByUserId: access.user.id,
      activatedByName: access.user.fullName || "Équipe pédagogique",
      notifyParents,
      attachIcs,
      parentCalendar: calendarPatch,
    });

    return NextResponse.json({
      success: true,
      trip: result.trip,
      blog: result.meta,
      created: result.created,
      parentsNotified: result.parentsNotified,
      parentsSkippedReason: result.parentsSkippedReason,
    });
  } catch (e) {
    console.error("[parent-blog activate]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Activation impossible." },
      { status: 500 },
    );
  }
}
