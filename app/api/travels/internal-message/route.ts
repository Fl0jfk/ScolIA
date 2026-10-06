import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import { requireTenantId } from "@/app/lib/tenant-scope";
import { getTravelTrip } from "@/app/lib/travels-storage";
import {
  insertTravelInternalMessage,
  markTravelThreadRead,
} from "@/app/lib/travel-message-unread-db";
import {
  canUseTravelInternalThread,
  travelThreadViewerFromStaff,
  viewerIsCompta,
} from "@/app/lib/travels-thread-unread";
import { userIsAnyDirection } from "@/app/lib/establishment-catalog";
import { travelsDbReady } from "@/app/lib/travel-db";

async function resolveThreadContext() {
  const gate = await requireModule("travels");
  if (!gate.ok) return gate;
  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant;
  return { ok: true as const, user: gate.ctx.user, etablissementId: tenant.ctx.etablissementId };
}

export async function POST(req: Request) {
  const ctx = await resolveThreadContext();
  if (!ctx.ok) return ctx.response;

  try {
    const body = (await req.json()) as { tripId?: string; text?: string };
    const tripId = String(body.tripId || "").trim();
    const text = String(body.text || "").trim();
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    if (!text) return NextResponse.json({ error: "Message requis" }, { status: 400 });

    const trip = await getTravelTrip(tripId);
    if (!trip) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const appConfig = await loadAppConfig();
    const viewer = travelThreadViewerFromStaff(ctx.user);
    if (!canUseTravelInternalThread(trip, viewer, appConfig.establishments ?? [])) {
      return NextResponse.json({ error: "Fil interne réservé au créateur, à la direction et à la compta." }, { status: 403 });
    }

    const etabId = (await travelsDbReady()) || ctx.etablissementId;
    const roleLabel = userIsAnyDirection(
      viewer.user,
      appConfig.establishments ?? [],
      viewer.roles,
      viewer.extraUserIds,
    )
      ? "Direction"
      : viewerIsCompta(ctx.user.roles)
        ? "Comptabilité"
        : "Créateur";
    const userLabel =
      ctx.user.name?.trim() ||
      [ctx.user.firstName, ctx.user.lastName].filter(Boolean).join(" ").trim() ||
      "Utilisateur";

    const message = await insertTravelInternalMessage({
      etablissementId: etabId,
      travelId: trip.id,
      authorUserId: ctx.user.id,
      userLabel,
      role: roleLabel,
      text,
    });

    const updated = await getTravelTrip(trip.id);
    return NextResponse.json({
      success: true,
      message,
      trip: updated ? { ...updated, unreadInternalCount: 0 } : { ...trip, unreadInternalCount: 0 },
    });
  } catch (e) {
    console.error("[travels/internal-message] POST", e);
    return NextResponse.json({ error: "Envoi impossible." }, { status: 500 });
  }
}

export async function PATCH(req: Request) {
  const ctx = await resolveThreadContext();
  if (!ctx.ok) return ctx.response;

  try {
    const body = (await req.json()) as { tripId?: string };
    const tripId = String(body.tripId || "").trim();
    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });

    const trip = await getTravelTrip(tripId);
    if (!trip) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const appConfig = await loadAppConfig();
    const viewer = travelThreadViewerFromStaff(ctx.user);
    if (!canUseTravelInternalThread(trip, viewer, appConfig.establishments ?? [])) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
    }

    const etabId = (await travelsDbReady()) || ctx.etablissementId;
    await markTravelThreadRead({
      etablissementId: etabId,
      travelId: trip.id,
      userId: ctx.user.id,
    });
    return NextResponse.json({ success: true, unreadInternalCount: 0 });
  } catch (e) {
    console.error("[travels/internal-message] PATCH", e);
    return NextResponse.json({ error: "Marquage lu impossible." }, { status: 500 });
  }
}
