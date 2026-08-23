import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import {
  loadAllEstablishments,
  loadAppConfig,
  saveEstablishments,
  saveIntegrations,
  saveNotifications,
  saveProfRoomModule,
  saveSiteIdentity,
  saveTravelsModule,
  saveOnboardingStep,
} from "@/app/lib/app-config";
import {
  parseEstablishmentsFile,
  parseIntegrations,
  parseNotifications,
  parseSiteIdentity,
  parseTravelsModule,
} from "@/app/lib/app-config-schemas";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { normalizeProfRoomAdminIds } from "@/app/lib/prof-room-auth";
import { ensureSiteAddressCoordinates } from "@/app/lib/site-address-coordinates";
import {
  syncEstablishmentDirectorRoles,
  withDerivedRoleSlugs,
} from "@/app/lib/establishment-director-sync";

const ALLOWED = new Set([
  "site",
  "establishments",
  "notifications",
  "prof-room",
  "integrations",
  "travels",
]);

export async function PUT(req: Request, ctx: { params: Promise<{ section: string }> }) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;
  const { section } = await ctx.params;
  if (!ALLOWED.has(section)) {
    return NextResponse.json({ error: "Section inconnue." }, { status: 400 });
  }
  try {
    const body = await req.json();
    const user = await safeCurrentUser();
    const audit = { updatedAt: new Date().toISOString(), updatedBy: user?.fullName || user?.id || "admin" };

    if (section === "site") {
      const parsed = await ensureSiteAddressCoordinates(parseSiteIdentity(body));
      await saveSiteIdentity(parsed);
      if (typeof body.onboardingStep === "number") {
        await saveOnboardingStep(body.onboardingStep);
      }
    } else if (section === "establishments") {
      const previous = await loadAllEstablishments();
      const parsed = parseEstablishmentsFile(body).map(withDerivedRoleSlugs);
      await saveEstablishments(parsed);
      await syncEstablishmentDirectorRoles(previous, parsed);
      try {
        const { getRequestsRoutingConfig, saveRequestsRoutingConfig } = await import(
          "@/app/lib/requests-routing-config"
        );
        const { syncRequestsRoutingWithEstablishments } = await import(
          "@/app/lib/requests-routing-defaults"
        );
        const routing = await getRequestsRoutingConfig();
        await saveRequestsRoutingConfig(syncRequestsRoutingWithEstablishments(routing, parsed));
      } catch (e) {
        console.error("[settings] sync requests routing with establishments", e);
      }
    } else if (section === "notifications") {
      await saveNotifications(parseNotifications(body));
    } else if (section === "prof-room") {
      const current = await loadAppConfig();
      const adminExternalUserIds = Array.isArray(body?.adminExternalUserIds)
        ? normalizeProfRoomAdminIds(body.adminExternalUserIds)
        : current.profRoom.adminExternalUserIds || [];
      await saveProfRoomModule({ ...current.profRoom, adminExternalUserIds });
    } else if (section === "integrations") {
      await saveIntegrations(parseIntegrations(body));
    } else if (section === "travels") {
      await saveTravelsModule(parseTravelsModule(body));
    }

    const config = await loadAppConfig();
    return NextResponse.json({ success: true, audit, config });
  } catch (e) {
    console.error("[settings] PUT", section, e);
    const msg = e instanceof Error ? e.message : "Erreur enregistrement";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
