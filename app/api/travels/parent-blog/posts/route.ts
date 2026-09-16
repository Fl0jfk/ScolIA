import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getJson } from "@/app/lib/s3-storage";
import { travelsDbReady } from "@/app/lib/travel-db";
import {
  assertParentBlogEditAccess,
  createParentBlogPost,
  deleteParentBlogPost,
  PARENT_BLOG_MAX_PHOTOS,
  type PhotoUploadPayload,
} from "@/app/lib/travels-parent-blog";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** POST — publier un message (+ photos) sur le blog. */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    const message = String(body.body || body.message || "").trim();
    const photos: PhotoUploadPayload[] = Array.isArray(body.photos) ? body.photos : [];

    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    if (!message) return NextResponse.json({ error: "Message requis" }, { status: 400 });
    if (photos.length > PARENT_BLOG_MAX_PHOTOS) {
      return NextResponse.json(
        { error: `Maximum ${PARENT_BLOG_MAX_PHOTOS} photos.` },
        { status: 400 },
      );
    }

    const etabId = await travelsDbReady();
    if (!etabId) return NextResponse.json({ error: "Base voyages indisponible." }, { status: 503 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertParentBlogEditAccess(trip);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const post = await createParentBlogPost({
      etablissementId: etabId,
      trip,
      authorUserId: access.user.id,
      authorName: access.user.fullName || "Équipe pédagogique",
      body: message,
      photos,
    });

    const refreshed = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    return NextResponse.json({
      success: true,
      post,
      trip: refreshed?.data || trip,
    });
  } catch (e) {
    console.error("[parent-blog/posts POST]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Publication impossible." },
      { status: 500 },
    );
  }
}

/** DELETE — supprimer une publication. */
export async function DELETE(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "").trim();
    const postId = String(body.postId || "").trim();
    if (!tripId || !postId) {
      return NextResponse.json({ error: "tripId et postId requis" }, { status: 400 });
    }

    const etabId = await travelsDbReady();
    if (!etabId) return NextResponse.json({ error: "Base voyages indisponible." }, { status: 503 });

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertParentBlogEditAccess(trip);
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    await deleteParentBlogPost({
      etablissementId: etabId,
      trip,
      postId,
      actorName: access.user.fullName || "Équipe",
    });

    const refreshed = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    return NextResponse.json({ success: true, trip: refreshed?.data || trip });
  } catch (e) {
    console.error("[parent-blog/posts DELETE]", e);
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Suppression impossible." },
      { status: 500 },
    );
  }
}
