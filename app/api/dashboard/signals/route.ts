import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { getDashboardSignals } from "@/app/lib/dashboard-signals";
import { loadAppConfig } from "@/app/lib/app-config";
import { INTRANET_MODULES, rolesAllowModule } from "@/app/lib/intranet-modules";
import { canAccessHseModule } from "@/app/lib/demandes-hse-access";
import { createPerfTimer } from "@/app/lib/perf-timer";
import {
  buildLoadContextFromViewUser,
  loadDashboardSignalSources,
  type DashboardSignalsViewUser,
} from "@/app/lib/dashboard-signals-load";
import { DASHBOARD_SIGNAL_SOURCES } from "@/app/lib/dashboard-signals-parallel";

const EMPTY_SIGNALS = {
  shortcuts: [] as unknown[],
  todayNews: [] as unknown[],
  hasCurrentWeek: false,
  notifications: [] as unknown[],
  anneeScolaireLabel: null as string | null,
};

export async function GET() {
  const perf = createPerfTimer();
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  perf.mark("auth");

  try {
    const { requireViewUser } = await import("@/app/lib/app-session");
    const viewUser = await requireViewUser();
    if (!viewUser.ok) {
      return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
    }
    const user = viewUser.user;
    const roles = user.roles;
    const userId = user.businessUserId;
    const authUserId = user.id;
    const businessUserId = user.businessUserId;
    const email = user.email ?? "";
    const isOrgAdmin = isOrgAdminFromAppUser(user);
    perf.mark("session");

    let signalsEtabId = user.etablissementId?.trim() || "default";
    try {
      const { requireTenantId } = await import("@/app/lib/tenant-scope");
      const tenant = await requireTenantId();
      if (tenant.ok) signalsEtabId = tenant.ctx.etablissementId;
    } catch {
      /* ignore */
    }

    const { valkeyGetJson, valkeySetJson } = await import("@/app/lib/valkey");
    const { VALKEY_TTL, valkeyKeyDashboardSignals } = await import("@/app/lib/valkey-keys");
    const signalsCacheKey = valkeyKeyDashboardSignals(signalsEtabId, authUserId);
    const cachedSignals = await valkeyGetJson<Record<string, unknown>>(signalsCacheKey);
    perf.mark("cache_lookup");
    if (cachedSignals) {
      const snap = perf.snapshot();
      return NextResponse.json({
        ...cachedSignals,
        perf: {
          ...snap,
          stepsMs: { ...snap.stepsMs, aggregate: snap.totalMs },
          fromCache: true,
        },
      });
    }

    const { loadModuleAccess } = await import("@/app/lib/module-access-store");
    const moduleAccess = await loadModuleAccess();
    const accessibleModuleIds = new Set(
      INTRANET_MODULES.filter((m) =>
        rolesAllowModule(roles, m, isOrgAdmin, moduleAccess, {
          userId: authUserId,
          businessUserId,
        }),
      ).map((m) => m.id),
    );
    if (accessibleModuleIds.has("rh")) {
      accessibleModuleIds.add("absences");
      if (canAccessHseModule(roles)) accessibleModuleIds.add("demandes-hse");
    }

    const { resolvePhotocopiesOpsEmails } = await import("@/app/lib/photocopies-couleur-ops");
    const { isPhotocopiesOpsHandlerResolved } = await import(
      "@/app/lib/photocopies-couleur-ops-server"
    );
    const appBundle = await loadAppConfig().catch(() => null);
    const photocopiesOpsEmails = resolvePhotocopiesOpsEmails(appBundle?.notifications ?? null);
    const isPhotoOps = isPhotocopiesOpsHandlerResolved({
      email,
      opsEmails: photocopiesOpsEmails,
      moduleAccess,
      lookup: { userId: authUserId, businessUserId },
      roles,
    });
    if (isPhotoOps) accessibleModuleIds.add("photocopies-couleur");

    try {
      const { isPortesOuvertesToolEnabled } = await import("@/app/lib/toolbox-config");
      if (!(await isPortesOuvertesToolEnabled())) {
        accessibleModuleIds.delete("accueil-portes-ouvertes");
      }
    } catch {
      /* ignore */
    }

    let establishments: NonNullable<typeof appBundle>["establishments"] =
      appBundle?.establishments ?? [];
    if (!appBundle) {
      try {
        const appConfig = await loadAppConfig();
        establishments = appConfig.establishments;
      } catch (err) {
        console.error("[dashboard/signals] loadAppConfig", err);
      }
    }
    perf.mark("access_config");

    const view: DashboardSignalsViewUser = {
      id: authUserId,
      businessUserId,
      email: user.email,
      roles,
      firstName: user.firstName ?? null,
      lastName: user.lastName ?? null,
      name: user.name ?? null,
      imageUrl: user.imageUrl ?? null,
      platformAdmin: user.platformAdmin,
    };

    const loadCtx = buildLoadContextFromViewUser(view, {
      signalsEtabId,
      isOrgAdmin,
      accessibleModuleIds,
      moduleAccess,
      isPhotoOps,
      photocopiesOpsEmails,
      appBundle,
      establishments,
      absenceNotifications: appBundle?.notifications ?? null,
    });

    const { bundle, stepsMs: sourceStepsMs, sourceStatus } = await loadDashboardSignalSources(loadCtx);
    perf.mark("sources");

    const { internat, rh, demandes, absences, documents, stages, others } = bundle;

    try {
      const signals = getDashboardSignals({
        roles,
        userId,
        authUserId,
        email,
        firstName: user.firstName ?? undefined,
        lastName: user.lastName ?? undefined,
        accessibleModuleIds,
        trips: Array.isArray(others.tripsRaw) ? others.tripsRaw : [],
        absences: absences.absences,
        reservations: others.reservations,
        rooms: others.rooms,
        roomSubjectColors: others.roomSubjectColors,
        requestsBoard: demandes.requestsBoard,
        photocopies: demandes.photocopies,
        photocopiesOpsEmails,
        photocopiesOpsHandler: isPhotoOps,
        hse: demandes.hse,
        stagesPendingSignatures: stages.stagesPendingSignatures,
        internatRollCallStatus: internat.internatRollCallStatus,
        weekSheet: others.weekSheet,
        moodPulseSubmittedToday: rh.moodPulseSubmittedToday,
        sstEmargementPending: rh.sstEmargementPending,
        planningNow: rh.planningNow,
        establishments,
        absenceNotifications: appBundle?.notifications ?? null,
        unseenSharedFolders: documents.unseenSharedFolders,
        vsAbsencesATraiter: absences.vsAbsencesATraiter,
        vsAbsencesJustifFamille: absences.vsAbsencesJustifFamille,
        vsAppelsManquants: absences.vsAppelsManquants,
        vsSanctionsAujourdhui: others.vsSanctionsAujourdhui,
        vsCarnetNonSignees: others.vsCarnetNonSignees,
        facturesEnRetard: others.facturesEnRetard,
        anneeScolaireLabel: others.anneeScolaireLabel,
        unseenAccompagnementAlerts: others.unseenAccompagnementAlerts,
        creneauxVidesCount: others.creneauxVidesCount,
        creneauxVidesTravelId: others.creneauxVidesTravelId,
        travelsUnreadMessages: others.travelsUnreadMessages,
      });
      perf.mark("aggregate");
      const snap = perf.snapshot();
      const payload = {
        ...signals,
        perf: {
          ...snap,
          stepsMs: {
            ...snap.stepsMs,
            ...sourceStepsMs,
            aggregate: snap.totalMs,
          },
          sourceStatus,
        },
      };
      void valkeySetJson(signalsCacheKey, signals, VALKEY_TTL.dashboardSignals);
      return NextResponse.json(payload);
    } catch (err) {
      console.error("[dashboard/signals] getDashboardSignals", err);
      const snap = perf.snapshot();
      return NextResponse.json({
        ...EMPTY_SIGNALS,
        anneeScolaireLabel: others.anneeScolaireLabel,
        perf: { ...snap, stepsMs: { ...snap.stepsMs, ...sourceStepsMs }, sourceStatus },
      });
    }
  } catch (e) {
    console.error("[dashboard/signals]", e);
    return NextResponse.json({
      ...EMPTY_SIGNALS,
      perf: perf.snapshot(),
    });
  }
}
