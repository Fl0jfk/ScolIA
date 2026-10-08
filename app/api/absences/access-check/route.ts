import { NextResponse } from "next/server";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  viewerCanSeeAbsenceDirectionQueue,
} from "@/app/lib/absences-admin-access";
import {
  canManageAbsence,
  canViewAbsence,
  isAbsencePendingForManager,
  resolveAbsenceScope,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { getAbsenceIndex } from "@/app/lib/absences-storage";
import { getEffectiveViewUser, safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { getAppSession } from "@/app/lib/app-session";
import { resolveActiveSupervision } from "@/app/lib/supervision";

/**
 * Diagnostic lecture seule : rôles / file Direction pour la vue effective
 * (utile en supervision sur le compte d’une directrice).
 */
export async function GET() {
  const actor = await getAppSession();
  if (!actor) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }
  const supervision = await resolveActiveSupervision({ actorUserId: actor.user.id });
  const viewUser = await getEffectiveViewUser();
  const user = await safeCurrentUser();
  if (!viewUser && !user) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  const roles = viewUser?.roles?.length
    ? viewUser.roles
    : rolesFromUserLike(user);
  const email = viewUser?.email || user?.primaryEmailAddress?.emailAddress || "";
  const userId = String(viewUser?.businessUserId || user?.id || "").trim();
  const userIds = [
    viewUser?.businessUserId,
    viewUser?.id,
    viewUser?.externalUserId,
    user?.id,
  ];
  let establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"] = [];
  let notifications: Awaited<ReturnType<typeof loadAppConfig>>["notifications"] | null = null;
  try {
    const bundle = await loadAppConfig();
    establishments = bundle.establishments;
    notifications = bundle.notifications;
  } catch (e) {
    console.error("[api/absences/access-check] loadAppConfig", e);
  }

  const dirCtx = {
    establishments,
    userId,
    userIds,
    email,
    notifications,
  };
  const canTreat = viewerCanSeeAbsenceDirectionQueue(
    { email, userId, roles },
    notifications,
    establishments,
  );

  const lycee = establishments.find(
    (e) =>
      String(e.kind || "").toLowerCase() === "lycee" ||
      /lyc/i.test(String(e.label || "")),
  );

  const index = await getAbsenceIndex();
  const pendingSamples: Array<{
    id: string;
    displayName: string;
    scope: string;
    etablissement: string | null;
    canView: boolean;
    canManage: boolean;
    pendingForManager: boolean;
    ogecValidator: unknown;
  }> = [];

  let visibleCount = 0;
  let pendingCount = 0;
  let ogecPending = 0;
  let profPending = 0;

  for (const raw of index) {
    const abs = raw as AbsenceRecord;
    if (abs.managerDecision !== "EN_ATTENTE" || abs.workflowStatus === "CLOTUREE") continue;
    if (abs.source === "admin_manual" || abs.source === "admin_pdf") continue;

    const canView = canViewAbsence(abs, userId, roles, dirCtx);
    const canManage = canManageAbsence(abs, roles, dirCtx);
    const pending = isAbsencePendingForManager(abs, userId, roles, dirCtx);
    if (canView) visibleCount += 1;
    if (pending) {
      pendingCount += 1;
      const scope = resolveAbsenceScope(abs);
      if (scope === "ogec") ogecPending += 1;
      else profPending += 1;
    }
    if (pendingSamples.length < 15) {
      pendingSamples.push({
        id: abs.id,
        displayName: abs.displayName,
        scope: resolveAbsenceScope(abs),
        etablissement: abs.data?.etablissement ?? null,
        canView,
        canManage,
        pendingForManager: pending,
        ogecValidator: abs.data?.ogecValidator ?? null,
      });
    }
  }

  return NextResponse.json({
    supervisionActive: Boolean(supervision),
    view: {
      userId,
      authUserId: viewUser?.id ?? null,
      businessUserId: viewUser?.businessUserId ?? null,
      userIds: userIds.filter(Boolean),
      email,
      roles,
      name: viewUser?.name || user?.fullName || null,
    },
    lyceeDirector: lycee
      ? {
          label: lycee.label,
          directorName: lycee.directorName ?? null,
          directorEmail: lycee.directorEmail ?? null,
          directorExternalUserId: lycee.directorExternalUserId ?? null,
        }
      : null,
    absencesValidatorsOgec: notifications?.absencesValidatorsOgec ?? [],
    canSeeDirectionTab: canTreat,
    counts: {
      enAttenteTotal: index.filter(
        (a) =>
          a.managerDecision === "EN_ATTENTE" &&
          a.workflowStatus !== "CLOTUREE" &&
          a.source !== "admin_manual" &&
          a.source !== "admin_pdf",
      ).length,
      visibleEnAttente: visibleCount,
      pendingForManager: pendingCount,
      ogecPendingForManager: ogecPending,
      profPendingForManager: profPending,
    },
    samples: pendingSamples,
  });
}
