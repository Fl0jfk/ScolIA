import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";

import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  canManageStageSettings,
  canModerateOffers,
  canReviewPreconvention,
  canViewAllConventions,
  canBrowseStageConventions,
  canViewReferentConventions,
  canFileConventionToOneDrive,
  resolveStageViewerRole,
} from "@/app/lib/stage-access";
import { ensureStageYearAutoPurge } from "@/app/lib/stage-auto-purge";
import {
  conventionVisibleToUser,
  indexEntryVisibleToUser,
} from "@/app/lib/stage-referent";
import { listPendingSignaturesForUser } from "@/app/lib/stage-pending-signatures";
import { listPrincipalClassesForUser } from "@/app/lib/stage-referents-config";
import {
  getStageWatchersConfig,
  listWatcherAssignmentsForUser,
} from "@/app/lib/stage-watchers-config";
import {
  classNameMatchesStageSecteurs,
  conventionMatchesStageSecteurs,
  resolveStageViewerSecteurs,
  stageViewerSecteurSummary,
} from "@/app/lib/stage-sector-scope";
import {
  loadHubBoardStageConventions,
  loadStageConventionsByIds,
} from "@/app/lib/stage-convention-load";
import { getConventionsIndex } from "@/app/lib/stage-storage";
import {
  STAGE_CONVENTION_STATUS_LABELS,
  currentStageSchoolYear,
  type StageConvention,
  type StageConventionIndexEntry,
  type StageConventionStatus,
} from "@/app/lib/stage-types";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import {
  loadElevePhotoIndex,
  resolveElevePhotoS3Key,
} from "@/app/lib/eleve-photos";
import { inferSecteurFromFolderName } from "@/app/lib/onedrive-eleves";
import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import { createPerfTimer } from "@/app/lib/perf-timer";

function stageElevePhotoPath(eleveId: string): string {
  return `/api/stages/eleve-photo?eleveId=${encodeURIComponent(eleveId)}`;
}

function normalizePersonPart(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function depositKindForConvention(c: {
  status: string;
  tutorEmailChangeRequest?: unknown;
  scheduleChangeRequest?: unknown;
}): string {
  if (c.scheduleChangeRequest) return "Avenant";
  if (c.tutorEmailChangeRequest) return "E-mail tuteur";
  if (c.status === "convention_deposited") return "Convention";
  return "Stage";
}

function resolveConventionSecteur(params: {
  className?: string;
  level?: string;
  eleveSecteur?: string | null;
}): Secteur | null {
  const explicit = String(params.eleveSecteur || "")
    .trim()
    .toLowerCase();
  if (explicit === "ecole" || explicit === "college" || explicit === "lycee") {
    return explicit;
  }
  return (
    inferSecteurFromFolderName(params.className || "") ||
    inferSecteurFromFolderName(params.level || "") ||
    null
  );
}

const SKIP_INDEX: ReadonlySet<StageConventionStatus> = new Set([
  "archived",
  "draft",
  "cancelled",
]);

const HUB_BOARD_STATUSES: ReadonlySet<StageConventionStatus> = new Set([
  "admin_review",
  "preconvention_submitted",
  "convention_deposited",
  "convention_ready",
  "signatures_pending",
]);

function filterIndexForViewer(
  index: StageConventionIndexEntry[],
  roles: string[],
  userEmail: string,
  principalClassNames: string[],
  viewerSecteurs: Secteur[],
): StageConventionIndexEntry[] {
  return index.filter((e) => {
    if (SKIP_INDEX.has(e.status)) return false;
    if (!indexEntryVisibleToUser(e, roles, userEmail, principalClassNames)) return false;
    if (viewerSecteurs.length > 0 && !classNameMatchesStageSecteurs(e.className, viewerSecteurs)) {
      return false;
    }
    return true;
  });
}

export async function GET() {
  const perf = createPerfTimer();
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;
    perf.mark("auth");

    // Ne pas bloquer le hub sur la purge annuelle (1ère fois de l’année seulement).
    void ensureStageYearAutoPurge().catch(() => undefined);

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const viewer = resolveStageViewerRole(roles);
    const watchersCfg = await getStageWatchersConfig(currentStageSchoolYear());
    const watcherAssignments = listWatcherAssignmentsForUser(watchersCfg, gate.ctx.userId);
    if (!viewer && watcherAssignments.length === 0) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }
    perf.mark("session_watchers");

    const viewerSecteurs = await resolveStageViewerSecteurs(roles, gate.ctx.userId);
    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    const principalClassNames = canViewReferentConventions(roles)
      ? await listPrincipalClassesForUser(gate.ctx.userId)
      : [];
    perf.mark("secteurs_pp");

    const index = await getConventionsIndex();
    perf.mark("conventions_index");
    const watcherOnlyPath =
      watcherAssignments.length > 0 &&
      !canBrowseStageConventions(roles) &&
      !canViewReferentConventions(roles);

    /** Compteurs depuis l’index (pas besoin de charger toutes les conventions signées). */
    let visibleIndex = filterIndexForViewer(
      index,
      roles,
      userEmail,
      principalClassNames,
      viewerSecteurs,
    );

    let boardConventions: StageConvention[];

    if (watcherOnlyPath) {
      // Watchers : matching élève/classe nécessite les objets complets du hub.
      boardConventions = await loadHubBoardStageConventions();
      boardConventions = boardConventions.filter((c) =>
        conventionVisibleToUser(
          c,
          roles,
          userEmail,
          gate.ctx.userId,
          principalClassNames,
          watcherAssignments,
        ),
      );
      if (viewerSecteurs.length > 0) {
        boardConventions = boardConventions.filter((c) =>
          conventionMatchesStageSecteurs(c, viewerSecteurs),
        );
      }
      const visibleIds = new Set(boardConventions.map((c) => c.id));
      visibleIndex = index.filter(
        (e) => !SKIP_INDEX.has(e.status) && visibleIds.has(e.id),
      );
    } else {
      const boardIds = visibleIndex
        .filter((e) => HUB_BOARD_STATUSES.has(e.status))
        .map((e) => e.id);
      boardConventions = await loadStageConventionsByIds(boardIds);
      boardConventions = boardConventions.filter((c) =>
        conventionVisibleToUser(
          c,
          roles,
          userEmail,
          gate.ctx.userId,
          principalClassNames,
          watcherAssignments,
        ),
      );
    }
    perf.mark("board_conventions");

    const signedCount = visibleIndex.filter((e) => e.status === "signed").length;
    const activeCount = visibleIndex.length;
    const adminQueue = boardConventions.filter(
      (c) =>
        c.status === "admin_review" ||
        c.status === "preconvention_submitted" ||
        c.status === "convention_deposited" ||
        Boolean(c.tutorEmailChangeRequest) ||
        Boolean(c.scheduleChangeRequest),
    );
    const signaturesPending = boardConventions.filter((c) => c.status === "signatures_pending");
    const signaturesPendingCount = visibleIndex.filter(
      (e) => e.status === "signatures_pending",
    ).length;

    const referentOnly =
      canViewReferentConventions(roles) && !canBrowseStageConventions(roles);
    const watcherOnly =
      !canBrowseStageConventions(roles) &&
      !canViewReferentConventions(roles) &&
      !canReviewPreconvention(roles) &&
      watcherAssignments.length > 0;
    const canSeeAdminDepositQueue = roles.includes("administratif");

    const boardSlice = [
      ...adminQueue.slice(0, 30),
      ...signaturesPending.slice(0, 30),
    ];

    const [myPendingSignatures, eleves] = await Promise.all([
      listPendingSignaturesForUser(
        signaturesPending,
        userEmail,
        gate.ctx.userId,
        roles,
        { includePeriodAlignment: false },
      ),
      // Registre uniquement si on a des cartes à enrichir (secteur / photos).
      boardSlice.length > 0
        ? loadElevesRegistry().catch(() => [] as Awaited<ReturnType<typeof loadElevesRegistry>>)
        : Promise.resolve([] as Awaited<ReturnType<typeof loadElevesRegistry>>),
    ]);
    perf.mark("pending_sigs_eleves");

    const mapBoardCard = (
      c: StageConvention,
      photoByConventionId: Record<string, string>,
      secteurByConventionId: Record<string, Secteur | null>,
    ) => ({
      id: c.id,
      studentName: `${c.student.firstName} ${c.student.lastName}`.trim(),
      companyName: c.company.name,
      className: c.student.className,
      secteur: secteurByConventionId[c.id] ?? null,
      status: c.status,
      photoUrl: photoByConventionId[c.id] || null,
      depositKind: depositKindForConvention(c),
      tutorEmailChangePending: Boolean(c.tutorEmailChangeRequest),
      scheduleChangePending: Boolean(c.scheduleChangeRequest),
    });

    const byIne = new Map(
      eleves
        .filter((e) => e.ine?.trim())
        .map((e) => [e.ine!.trim().toUpperCase(), e] as const),
    );
    const byName = new Map<string, (typeof eleves)[number]>();
    for (const e of eleves) {
      const key = `${normalizePersonPart(e.nom)}§${normalizePersonPart(e.prenom)}`;
      if (!byName.has(key)) byName.set(key, e);
    }

    const secteurByConventionId: Record<string, Secteur | null> = {};
    const elevesForPhotos: Array<{
      id: string;
      nom: string;
      prenom: string;
      ine?: string | null;
      photoKey?: string | null;
      conventionId: string;
    }> = [];
    for (const c of boardSlice) {
      const ine = c.ocrMeta?.matchedEleveIne?.trim().toUpperCase() || "";
      const fromIne = ine ? byIne.get(ine) : undefined;
      const fromName = byName.get(
        `${normalizePersonPart(c.student.lastName)}§${normalizePersonPart(c.student.firstName)}`,
      );
      const eleve = fromIne || fromName;
      secteurByConventionId[c.id] = resolveConventionSecteur({
        className: c.student.className,
        level: c.student.level,
        eleveSecteur: eleve?.secteur,
      });
      if (!eleve?.id) continue;
      elevesForPhotos.push({
        id: eleve.id,
        nom: eleve.nom,
        prenom: eleve.prenom,
        ine: eleve.ine,
        photoKey: eleve.photoKey,
        conventionId: c.id,
      });
    }

    // Proxy auth Stages (pas de pré-signature S3 en masse) — le navigateur charge en parallèle.
    const photoIndex =
      elevesForPhotos.length > 0
        ? await loadElevePhotoIndex().catch(() => ({} as Awaited<ReturnType<typeof loadElevePhotoIndex>>))
        : ({} as Awaited<ReturnType<typeof loadElevePhotoIndex>>);
    const photoByConventionId: Record<string, string> = {};
    for (const e of elevesForPhotos) {
      const key = resolveElevePhotoS3Key(photoIndex, e);
      if (key) photoByConventionId[e.conventionId] = stageElevePhotoPath(e.id);
    }
    perf.mark("photos");

    // Statut Valkey léger (pas de probes TCP/TLS — trop coûteux sur le hot path).
    const { isValkeyConfigured, getValkey } = await import("@/app/lib/valkey");
    const vk = getValkey();
    const valkey = {
      configured: isValkeyConfigured(),
      ready: vk?.status === "ready",
      pingOk: vk?.status === "ready",
      status: vk?.status ?? (isValkeyConfigured() ? "connecting" : "absent"),
    };
    perf.mark("valkey_status");
    const perfSnapshot = perf.snapshot();

    return NextResponse.json({
      viewer: viewer || "staff",
      viewerSecteurLabel: stageViewerSecteurSummary(viewerSecteurs),
      cache: { valkey },
      perf: {
        ...perfSnapshot,
        boardLoaded: boardConventions.length,
        indexSize: index.length,
        visibleIndex: visibleIndex.length,
      },
      permissions: {
        canModerateOffers: canModerateOffers(roles),
        canReviewPreconvention: canReviewPreconvention(roles),
        canSeeAdminDepositQueue,
        canViewAllConventions: canViewAllConventions(roles),
        canViewReferentConventions: canViewReferentConventions(roles),
        canDepositOffer: roles.includes("parent"),
        canFileToOneDrive: canFileConventionToOneDrive(roles),
        canManageStageSettings: canManageStageSettings(roles),
        canManageReferents: canReviewPreconvention(roles),
        canViewRepasAbsences:
          canReviewPreconvention(roles) ||
          canViewAllConventions(roles) ||
          roles.includes("cpe") ||
          watcherAssignments.some((a) => a.kind === "restauration" || a.kind === "cpe"),
        referentOnly,
        watcherOnly,
        canViewClassRoster:
          canViewReferentConventions(roles) ||
          canBrowseStageConventions(roles) ||
          canViewAllConventions(roles),
        consultOnly:
          canViewReferentConventions(roles) &&
          !canReviewPreconvention(roles) &&
          !canViewAllConventions(roles),
      },
      counts: {
        pendingOffers: 0,
        conventions: activeCount,
        signed: signedCount,
        adminQueue: adminQueue.length,
        signaturesPending: signaturesPendingCount,
        myPendingSignatures: myPendingSignatures.length,
      },
      myPendingSignatures,
      pendingOffers: [],
      adminQueue: adminQueue
        .slice(0, 30)
        .map((c) => mapBoardCard(c, photoByConventionId, secteurByConventionId)),
      signaturesPending: signaturesPending
        .slice(0, 30)
        .map((c) => mapBoardCard(c, photoByConventionId, secteurByConventionId)),
      conventions: visibleIndex.slice(0, 100).map((e) => ({
        id: e.id,
        studentName: e.studentName,
        className: e.className,
        companyName: e.companyName,
        status: e.status,
        periodStart: e.periodStart,
        periodEnd: e.periodEnd,
      })),
      labels: {
        conventionStatuses: STAGE_CONVENTION_STATUS_LABELS,
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
