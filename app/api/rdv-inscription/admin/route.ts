import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManageRdvInscription } from "@/app/lib/rdv-inscription-access";
import {
  getRdvInscriptionConfig,
  listRdvInscriptionBookings,
  listRdvInscriptionDirections,
  updateRdvInscriptionConfig,
  upsertRdvInscriptionDirection,
} from "@/app/lib/rdv-inscription-db";
import {
  clearRdvInscriptionGoogleLinkSecret,
  getRdvInscriptionGoogleLinkStatus,
} from "@/app/lib/rdv-inscription-google";
import { listAvailableInscriptionSlots } from "@/app/lib/rdv-inscription-gcal";
import { getRdvInscriptionOAuthRedirectUri } from "@/app/lib/rdv-inscription-oauth";
import { getTenantAppUrl } from "@/app/lib/tenant-context";

async function requireRdvAdmin() {
  const gate = await requireAuth();
  if (!gate.ok) return { ok: false as const, response: gate.response };
  const user = await safeCurrentUser();
  if (!user || !canManageRdvInscription(rolesFromUserLike(user))) {
    return {
      ok: false as const,
      response: NextResponse.json({ error: "Accès refusé." }, { status: 403 }),
    };
  }
  return { ok: true as const, user };
}

/** État admin : config, directions, Google, réservations. */
export async function GET(req: Request) {
  const auth = await requireRdvAdmin();
  if (!auth.ok) return auth.response;

  try {
    const url = new URL(req.url);
    const view = url.searchParams.get("view") || "all";

    if (view === "bookings") {
      const directionSlug = url.searchParams.get("direction") || undefined;
      const bookings = await listRdvInscriptionBookings({ directionSlug, limit: 150 });
      return NextResponse.json({ bookings });
    }

    const [config, directions, google, appUrl, oauthRedirectUri] = await Promise.all([
      getRdvInscriptionConfig(),
      listRdvInscriptionDirections(),
      getRdvInscriptionGoogleLinkStatus(),
      getTenantAppUrl(),
      getRdvInscriptionOAuthRedirectUri().catch(() => null),
    ]);

    const publicLinks = directions
      .filter((d) => d.active)
      .map((d) => ({
        slug: d.slug,
        label: d.label,
        url: `${appUrl}/rdv-inscription/${d.slug}`,
      }));

    return NextResponse.json({
      config,
      directions,
      google,
      publicLinks,
      oauthRedirectUri,
      oauthStartPath: "/api/rdv-inscription/oauth/start",
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}

/** Mise à jour config / directions / test listing / unlink Google. */
export async function PUT(req: Request) {
  const auth = await requireRdvAdmin();
  if (!auth.ok) return auth.response;

  try {
    const body = (await req.json()) as Record<string, unknown>;
    const action = String(body.action || "save").trim();

    if (action === "unlink-google") {
      await clearRdvInscriptionGoogleLinkSecret();
      return NextResponse.json({ success: true, google: await getRdvInscriptionGoogleLinkStatus() });
    }

    if (action === "test-slots") {
      const directionId = String(body.directionId || "").trim();
      const directions = await listRdvInscriptionDirections();
      const dir = directions.find((d) => d.id === directionId);
      if (!dir?.googleCalendarId.trim()) {
        return NextResponse.json({ error: "Direction ou calendarId manquant." }, { status: 400 });
      }
      const config = await getRdvInscriptionConfig();
      const slots = await listAvailableInscriptionSlots({
        calendarId: dir.googleCalendarId,
        titlePattern: config.eventTitlePattern,
        horizonDays: config.horizonDays,
      });
      return NextResponse.json({
        success: true,
        count: slots.length,
        slots: slots.slice(0, 20),
      });
    }

    if (action === "save-direction") {
      const dir = body.direction && typeof body.direction === "object"
        ? (body.direction as Record<string, unknown>)
        : null;
      if (!dir) {
        return NextResponse.json({ error: "direction requis." }, { status: 400 });
      }
      const saved = await upsertRdvInscriptionDirection({
        id: typeof dir.id === "string" ? dir.id : undefined,
        slug: String(dir.slug || ""),
        label: String(dir.label || ""),
        googleCalendarId: String(dir.googleCalendarId || ""),
        directriceDisplayName:
          dir.directriceDisplayName === null || dir.directriceDisplayName === undefined
            ? null
            : String(dir.directriceDisplayName),
        active: dir.active === true || dir.active === 1 || dir.active === "true",
        sortOrder: typeof dir.sortOrder === "number" ? dir.sortOrder : undefined,
      });
      return NextResponse.json({ success: true, direction: saved });
    }

    if (action === "save-config") {
      const patch = body.config && typeof body.config === "object"
        ? (body.config as Record<string, unknown>)
        : body;
      const config = await updateRdvInscriptionConfig({
        enabled: typeof patch.enabled === "boolean" ? patch.enabled : undefined,
        title: typeof patch.title === "string" ? patch.title : undefined,
        intro: typeof patch.intro === "string" ? patch.intro : undefined,
        eventTitlePattern:
          typeof patch.eventTitlePattern === "string" ? patch.eventTitlePattern : undefined,
        notifyEmail:
          patch.notifyEmail === null
            ? null
            : typeof patch.notifyEmail === "string"
              ? patch.notifyEmail
              : undefined,
        location: typeof patch.location === "string" ? patch.location : undefined,
        consentLabel: typeof patch.consentLabel === "string" ? patch.consentLabel : undefined,
        horizonDays: typeof patch.horizonDays === "number" ? patch.horizonDays : undefined,
      });
      return NextResponse.json({ success: true, config });
    }

    return NextResponse.json({ error: `Action inconnue : ${action}` }, { status: 400 });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : String(e) },
      { status: 500 },
    );
  }
}
