import "server-only";

import { getDashboardSignals, type DashboardNotification } from "@/app/lib/dashboard-signals";
import { loadAppConfig } from "@/app/lib/app-config";
import { isOrgAdminFromAppUser } from "@/app/lib/auth-roles-db";
import { INTRANET_MODULES, rolesAllowModule } from "@/app/lib/intranet-modules";
import { canAccessHseModule } from "@/app/lib/demandes-hse-access";
import {
  buildLoadContextFromViewUser,
  loadDashboardSignalSources,
  type DashboardSignalsViewUser,
} from "@/app/lib/dashboard-signals-load";
import { getDashboardSignalsBundleGeneration } from "@/app/lib/dashboard-signals-bundle-generation-server";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import { loadAppUserById } from "./signals-user";

/**
 * Calcul serveur des notifications du tableau de bord pour un utilisateur, SANS requête HTTP.
 * À APPELER DANS `runWithTenant(<tenant de l'établissement>, …)` (cf. signals-job.ts).
 *
 * ⚠ À garder aligné avec `app/api/dashboard/signals/route.ts` (accès modules, HSE, photocopies,
 * portes ouvertes) : la logique de droits y est volontairement dupliquée pour ne pas modifier la route.
 * Aucune écriture de cache Valkey des signaux ici.
 */
export async function computeDashboardNotificationsForUser(
  authUserId: string,
  etablissementId: string,
): Promise<DashboardNotification[] | null> {
  const user = await loadAppUserById(authUserId, etablissementId);
  if (!user) return null;

  const roles = user.roles;
  const userId = user.businessUserId;
  const businessUserId = user.businessUserId;
  const email = user.email ?? "";
  const isOrgAdmin = isOrgAdminFromAppUser(user);
  const signalsEtabId = etablissementId;

  const moduleAccess = await loadModuleAccess();
  const accessibleModuleIds = new Set(
    INTRANET_MODULES.filter((m) =>
      rolesAllowModule(roles, m, isOrgAdmin, moduleAccess, { userId: authUserId, businessUserId }),
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

  const establishments = appBundle?.establishments ?? [];
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
    signalsBundleGeneration: await getDashboardSignalsBundleGeneration(signalsEtabId),
    isOrgAdmin,
    accessibleModuleIds,
    moduleAccess,
    isPhotoOps,
    photocopiesOpsEmails,
    appBundle,
    establishments,
    absenceNotifications: appBundle?.notifications ?? null,
  });

  const { bundle, sourceStatus } = await loadDashboardSignalSources(loadCtx);
  // Une source en échec renverrait des compteurs à 0 : ne pas enregistrer de fausses baisses.
  if (Object.values(sourceStatus).some((s) => s !== "ok")) return null;
  const { internat, rh, demandes, absences, documents, stages, others } = bundle;

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
  return signals.notifications;
}
