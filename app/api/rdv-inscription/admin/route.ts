import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManageRdvInscription } from "@/app/lib/rdv-inscription-access";
import {
  getRdvInscriptionConfig,
  getRdvInscriptionDirectionById,
  listRdvInscriptionBookings,
  listRdvInscriptionDirections,
  updateRdvInscriptionConfig,
  upsertRdvInscriptionDirection,
} from "@/app/lib/rdv-inscription-db";
import {
  clearRdvInscriptionGoogleLinkSecret,
  getRdvInscriptionGoogleLinkStatus,
} from "@/app/lib/rdv-inscription-google";
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

    if (action === "confirm-booking") {
      const bookingId = String(body.bookingId || "").trim();
      if (!bookingId) {
        return NextResponse.json({ error: "bookingId requis." }, { status: 400 });
      }
      const { confirmRdvInscriptionBookingAsAdmin } = await import(
        "@/app/lib/rdv-inscription-service"
      );
      const result = await confirmRdvInscriptionBookingAsAdmin(bookingId);
      if (!result.ok) {
        return NextResponse.json({ error: result.error }, { status: result.status });
      }
      return NextResponse.json({
        success: true,
        booking: result.booking,
        already: result.already === true,
        mailWarning: result.mailWarning || undefined,
      });
    }

    if (action === "test-slots") {
      const directionId = String(body.directionId || "").trim();
      const dir = await getRdvInscriptionDirectionById(directionId);
      if (!dir?.googleCalendarId.trim()) {
        return NextResponse.json({ error: "Direction ou calendarId manquant." }, { status: 400 });
      }
      const { listAvailableInscriptionSlotsDetailed } = await import(
        "@/app/lib/rdv-inscription-gcal"
      );
      const result = await listAvailableInscriptionSlotsDetailed({
        calendarId: dir.googleCalendarId,
        titlePattern: dir.eventTitlePattern,
        horizonDays: dir.horizonDays,
      });
      return NextResponse.json({
        success: true,
        count: result.slots.length,
        slots: result.slots.slice(0, 20),
        titlePattern: dir.eventTitlePattern,
        upcomingEventCount: result.upcomingEventCount,
        sampleTitles: result.sampleTitles,
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
        title: typeof dir.title === "string" ? dir.title : undefined,
        intro: typeof dir.intro === "string" ? dir.intro : undefined,
        eventTitlePattern:
          typeof dir.eventTitlePattern === "string" ? dir.eventTitlePattern : undefined,
        notifyEmail:
          dir.notifyEmail === null
            ? null
            : typeof dir.notifyEmail === "string"
              ? dir.notifyEmail
              : undefined,
        location: typeof dir.location === "string" ? dir.location : undefined,
        consentLabel: typeof dir.consentLabel === "string" ? dir.consentLabel : undefined,
        horizonDays: typeof dir.horizonDays === "number" ? dir.horizonDays : undefined,
        active: dir.active === true || dir.active === 1 || dir.active === "true",
        sortOrder: typeof dir.sortOrder === "number" ? dir.sortOrder : undefined,
      });
      return NextResponse.json({
        success: true,
        direction: saved,
        config: await getRdvInscriptionConfig(),
      });
    }

    if (action === "save-config") {
      const patch = body.config && typeof body.config === "object"
        ? (body.config as Record<string, unknown>)
        : body;
      const config = await updateRdvInscriptionConfig({
        enabled: typeof patch.enabled === "boolean" ? patch.enabled : undefined,
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
