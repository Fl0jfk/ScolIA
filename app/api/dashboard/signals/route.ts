import { NextResponse } from "next/server";
import { safeCurrentUser, isOrgAdminFromPublicMetadata } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getDashboardSignals } from "@/app/lib/dashboard-signals";
import {
  canViewCalendar,
  isAbsencePendingForManager,
  isAbsenceVisibleOnCalendar,
  type AbsenceRecord,
} from "@/app/lib/absences-types";
import { getAbsenceIndex } from "@/app/lib/absences-storage";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { getJson } from "@/app/lib/s3-storage";
import { loadAppConfig } from "@/app/lib/app-config";
import { loadWeekSheetData } from "@/app/lib/dashboard-week-sheet-storage";
import { listPendingSignaturesForUser } from "@/app/lib/stage-pending-signatures";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import { resolveStageViewerRole } from "@/app/lib/stage-access";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { INTRANET_MODULES, rolesAllowModule } from "@/app/lib/intranet-modules";
import { canAccessHseModule, canViewHseDemand } from "@/app/lib/demandes-hse-access";
import { canAccessRequestsStaffBoard } from "@/app/lib/requests-staff-access";
import { getRequestsIndex, isLeaderForRequestBranch } from "@/app/lib/requests";
import { getAllBranchStaffEmailsFromRouting } from "@/app/lib/requests-routing-config";
import { isVisibleOnStaffBoard } from "@/app/lib/requests-board";
import { todayDateParis } from "@/app/lib/internat-stats";
import {
  hasVotedMoodPulse,
  moodPulseTodayKey,
  readMoodPulseDay,
} from "@/app/lib/rh/mood-pulse-storage";
import { getInternatRollCall } from "@/app/lib/internat-storage";
import { canAccessInternatModule, canSeeInternatRollCallSignal } from "@/app/lib/internat-rbac";
import type { TripIndexRow } from "@/app/lib/dashboard-trips";
import { defaultProfRoomModule } from "@/app/lib/app-config-defaults";
import { parseProfRoomModule } from "@/app/lib/app-config-schemas";
import { withDefaultProfRoomSubjects } from "@/app/lib/prof-room-defaults";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
import { getPersonnelLeaveRequests } from "@/app/lib/personnel-leave-storage";
import { PERSONNEL_LEAVE_TYPE_LABELS } from "@/app/lib/personnel-types";
import {
  findCurrentActivity,
  schoolWeekParity,
  type LeaveSpan,
} from "@/app/lib/rh/planning-calendar";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import { hasRole } from "@/app/lib/intranet-role-utils";
import type { RhPlanningDoc } from "@/app/lib/rh/planning-types";
import { listUnseenSharedFolderInvites } from "@/app/lib/documents-cloud";

async function safeJson<T>(path: string, timeoutMs = 8_000): Promise<T | null> {
  try {
    const hit = await Promise.race([
      getJson<T>(path),
      new Promise<null>((resolve) => {
        setTimeout(() => resolve(null), timeoutMs);
      }),
    ]);
    if (!hit || typeof hit !== "object" || !("data" in hit)) return null;
    return (hit.data as T) ?? null;
  } catch {
    return null;
  }
}

type RoomReservationSignal = {
  id: string;
  roomId: string;
  startsAt: string;
  endsAt?: string;
  subject?: string;
  className?: string;
  status?: string;
  userId?: string;
  email?: string;
  bookedForOther?: boolean;
  bookedByUserId?: string;
  firstName?: string;
  lastName?: string;
  bookedByFirstName?: string;
  bookedByLastName?: string;
};

function normalizeRoomReservations(raw: unknown): RoomReservationSignal[] {
  if (!Array.isArray(raw)) return [];
  const out: RoomReservationSignal[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const r = item as Record<string, unknown>;
    const startsAt = typeof r.startsAt === "string" ? r.startsAt : "";
    const roomId = typeof r.roomId === "string" ? r.roomId : "";
    const id = typeof r.id === "string" ? r.id : "";
    if (!id || !roomId || !startsAt) continue;
    out.push({
      id,
      roomId,
      startsAt,
      endsAt: typeof r.endsAt === "string" ? r.endsAt : undefined,
      subject: typeof r.subject === "string" ? r.subject : undefined,
      className: typeof r.className === "string" ? r.className : undefined,
      status: typeof r.status === "string" ? r.status : undefined,
      userId: typeof r.userId === "string" ? r.userId : undefined,
      email: typeof r.email === "string" ? r.email : undefined,
      bookedForOther: r.bookedForOther === true,
      bookedByUserId: typeof r.bookedByUserId === "string" ? r.bookedByUserId : undefined,
      firstName: typeof r.firstName === "string" ? r.firstName : undefined,
      lastName: typeof r.lastName === "string" ? r.lastName : undefined,
      bookedByFirstName:
        typeof r.bookedByFirstName === "string" ? r.bookedByFirstName : undefined,
      bookedByLastName: typeof r.bookedByLastName === "string" ? r.bookedByLastName : undefined,
    });
  }
  return out;
}

const EMPTY_SIGNALS = {
  shortcuts: [] as unknown[],
  todayNews: [] as unknown[],
  hasCurrentWeek: false,
  notifications: [] as unknown[],
  anneeScolaireLabel: null as string | null,
};

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const userId = gate.ctx.userId;
    const email = user?.primaryEmailAddress?.emailAddress ?? "";
    const isOrgAdmin = isOrgAdminFromPublicMetadata(user?.publicMetadata);

    const { loadModuleAccess } = await import("@/app/lib/module-access-store");
    const moduleAccess = await loadModuleAccess();
    let authUserId: string | null = null;
    let businessUserId: string | null = userId;
    try {
      const { requireAppUser } = await import("@/app/lib/app-session");
      const appUser = await requireAppUser();
      if (appUser.ok) {
        authUserId = appUser.user.id;
        businessUserId = appUser.user.businessUserId;
      }
    } catch {
      /* repli gate.ctx */
    }
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

    const tripsPromise = accessibleModuleIds.has("travels")
      ? safeJson<TripIndexRow[]>("travels/index.json")
      : Promise.resolve(null);

    const absencesPromise =
      accessibleModuleIds.has("rh") && (canViewCalendar(roles) || isAnyDirectionRole(roles))
        ? getAbsenceIndex().catch(() => [] as AbsenceRecord[])
        : Promise.resolve([] as AbsenceRecord[]);

    const roomsPromise = accessibleModuleIds.has("prof-room")
      ? safeJson<{ rooms?: { id: string; name: string }[] } | { id: string; name: string }[]>(
          "reservation-rooms/rooms.json",
        )
      : Promise.resolve(null);

    const reservationsPromise = accessibleModuleIds.has("prof-room")
      ? safeJson<unknown>("reservation-rooms/reservations.json", 5_000)
      : Promise.resolve(null);

    const profRoomConfigPromise = accessibleModuleIds.has("prof-room")
      ? safeJson<unknown>("settings/modules/prof-room.json")
      : Promise.resolve(null);

    const photocopiesPromise = accessibleModuleIds.has("photocopies-couleur")
      ? safeJson<
          Array<{
            id: string;
            status: string;
            etablissement?: string;
            createdBy?: { userId?: string };
          }>
        >("photocopies-couleur/index.json")
      : Promise.resolve(null);

    const hsePromise =
      accessibleModuleIds.has("demandes-hse") && canAccessHseModule(roles)
        ? safeJson<
            Array<{
              id: string;
              status: string;
              etablissement: string;
              createdBy: { userId: string };
            }>
          >("demandes-hse/index.json")
        : Promise.resolve(null);

    const [tripsRaw, absencesRaw, roomsRaw, reservationsRaw, photocopiesRaw, hseRaw, weekSheet, profRoomRaw] =
      await Promise.all([
        tripsPromise,
        absencesPromise,
        roomsPromise,
        reservationsPromise,
        photocopiesPromise,
        hsePromise,
        loadWeekSheetData().catch(() => null),
        profRoomConfigPromise,
      ]);

    let rooms: { id: string; name: string }[] = [];
    if (Array.isArray(roomsRaw)) {
      rooms = roomsRaw
        .filter((r): r is { id: string; name: string } =>
          Boolean(r && typeof r === "object" && typeof (r as { id?: unknown }).id === "string"),
        )
        .map((r) => ({
          id: r.id,
          name: typeof (r as { name?: unknown }).name === "string" ? (r as { name: string }).name : r.id,
        }));
    } else if (roomsRaw && typeof roomsRaw === "object" && Array.isArray(roomsRaw.rooms)) {
      rooms = roomsRaw.rooms
        .filter((r): r is { id: string; name: string } =>
          Boolean(r && typeof r === "object" && typeof (r as { id?: unknown }).id === "string"),
        )
        .map((r) => ({
          id: (r as { id: string }).id,
          name:
            typeof (r as { name?: unknown }).name === "string"
              ? (r as { name: string }).name
              : (r as { id: string }).id,
        }));
    }

    const reservations = normalizeRoomReservations(reservationsRaw);
    let roomSubjectColors: Record<string, string> = {};
    try {
      roomSubjectColors = withDefaultProfRoomSubjects(
        profRoomRaw ? parseProfRoomModule(profRoomRaw) : defaultProfRoomModule(),
      ).subjectColors;
    } catch (err) {
      console.error("[dashboard/signals] prof-room config", err);
      roomSubjectColors = withDefaultProfRoomSubjects(defaultProfRoomModule()).subjectColors;
    }
    let establishments: Awaited<ReturnType<typeof loadAppConfig>>["establishments"] = [];
    try {
      const appConfig = await loadAppConfig();
      establishments = appConfig.establishments;
    } catch (err) {
      console.error("[dashboard/signals] loadAppConfig", err);
    }
    const absenceDirCtx = { establishments, userId };
    let absences: AbsenceRecord[] = [];
    try {
      absences = absencesRaw.filter(
        (a) =>
          isAbsenceVisibleOnCalendar(a, userId, roles) ||
          isAbsencePendingForManager(a, userId, roles, absenceDirCtx),
      );
    } catch (err) {
      console.error("[dashboard/signals] absences filter", err);
      absences = [];
    }
    const hse = Array.isArray(hseRaw)
      ? hseRaw.filter((h) => canViewHseDemand(h, userId, roles))
      : [];
    const photocopies = Array.isArray(photocopiesRaw) ? photocopiesRaw : [];

    let requestsBoard: Array<{
      id: string;
      status: string;
      subject?: string;
      assignedTo?: {
        email?: string;
        claimedBy?: { email?: string; userId?: string | null } | null;
        routeId?: string;
        unit?: string;
        poolEmails?: string[];
      };
    }> = [];

    if (
      accessibleModuleIds.has("requests-staff") &&
      (await canAccessRequestsStaffBoard(roles, email))
    ) {
      try {
        const index = await getRequestsIndex();
        const allStaff = await getAllBranchStaffEmailsFromRouting();
        for (const r of index) {
          const isLeader = await isLeaderForRequestBranch(
            r.assignedTo.routeId,
            r.assignedTo.unit,
            email,
          );
          if (isVisibleOnStaffBoard(r.assignedTo, email, allStaff, isLeader)) {
            requestsBoard.push({
              id: r.id,
              status: r.status,
              subject: r.subject,
              assignedTo: r.assignedTo,
            });
          }
        }
      } catch {
        /* ignore */
      }
    }

    let stagesPendingSignatures = 0;
    if (accessibleModuleIds.has("stages") && resolveStageViewerRole(roles)) {
      try {
        const conventionsIndex = await getConventionsIndex();
        const all = await Promise.all(conventionsIndex.map((e) => getStageConvention(e.id)));
        const userEmail = email.trim().toLowerCase();
        const conventions = all
          .filter((c): c is NonNullable<typeof c> => Boolean(c))
          .filter((c) => conventionVisibleToUser(c, roles, userEmail, userId));
        stagesPendingSignatures = (
          await listPendingSignaturesForUser(conventions, userEmail, userId, roles)
        ).length;
      } catch {
        /* ignore */
      }
    }

    let internatRollCallStatus: "validee" | "en_cours" | "non_demarre" | null = null;
    if (
      accessibleModuleIds.has("internat") &&
      canAccessInternatModule(roles) &&
      canSeeInternatRollCallSignal(roles)
    ) {
      try {
        const roll = await getInternatRollCall(todayDateParis());
        if (roll.status === "validee") {
          internatRollCallStatus = "validee";
        } else {
          const marks = { ...roll.boys.marks, ...roll.girls.marks };
          internatRollCallStatus =
            Object.keys(marks).length > 0 || roll.boys.completed || roll.girls.completed
              ? "en_cours"
              : "non_demarre";
        }
      } catch {
        internatRollCallStatus = "non_demarre";
      }
    }

    let moodPulseSubmittedToday = false;
    if (accessibleModuleIds.has("rh")) {
      try {
        const dayDoc = await readMoodPulseDay(moodPulseTodayKey());
        moodPulseSubmittedToday = hasVotedMoodPulse(dayDoc, userId);
      } catch {
        moodPulseSubmittedToday = false;
      }
    }

    let planningNow: {
      title: string;
      detail: string;
      start: string;
      end: string;
    } | null = null;
    if (accessibleModuleIds.has("mon-planning") || accessibleModuleIds.has("rh")) {
      try {
        let doc: RhPlanningDoc | null = null;
        let leavePersonnelId: string | undefined;
        try {
          const index = await getPersonnelIndex();
          const self = index.find((e) => e.externalUserId === userId && e.active !== false);
          if (self) leavePersonnelId = self.id;
        } catch {
          leavePersonnelId = undefined;
        }

        if (hasRole(roles, "professeur")) {
          doc = await readRhPlanning("teacher", userId);
        } else {
          doc = await readRhPlanning("staff", userId);
        }
        // Si pas de dossier OGEC mais compte prof-like déjà géré ; sinon tente teacher id
        let needsTeacherFallback = !doc;
        if (doc?.kind === "teacher") {
          needsTeacherFallback =
            doc.weekA.length === 0 &&
            doc.weekB.length === 0 &&
            !(doc.replacements?.length ?? 0);
        }
        if (needsTeacherFallback) {
          const teacherTry = await readRhPlanning("teacher", userId);
          if (teacherTry.kind === "teacher") {
            if (
              teacherTry.weekA.length > 0 ||
              teacherTry.weekB.length > 0 ||
              (teacherTry.replacements?.length ?? 0) > 0
            ) {
              doc = teacherTry;
            }
          }
        }
        if (doc) {
          let leaves: LeaveSpan[] = [];
          if (leavePersonnelId) {
            const allLeaves = await getPersonnelLeaveRequests();
            leaves = allLeaves
              .filter((r) => r.status === "validee" && r.personnelId === leavePersonnelId)
              .map((r) => ({
                startDate: r.startDate,
                endDate: r.endDate,
                type: r.type,
                label: PERSONNEL_LEAVE_TYPE_LABELS[r.type] || r.type,
              }));
          }
          let zone: "A" | "B" | "C" | null = null;
          try {
            const cfg = await loadAppConfig();
            zone = cfg.identity.schoolHolidayZone ?? null;
          } catch {
            zone = null;
          }
          const act = findCurrentActivity(
            doc,
            new Date(),
            schoolWeekParity(new Date()),
            leaves,
            zone,
          );
          if (act) {
            planningNow = {
              title: act.title,
              detail: act.detail,
              start: act.start,
              end: act.end,
            };
          }
        }
      } catch {
        planningNow = null;
      }
    }

    let unseenSharedFolders: Array<{ id: string; name: string }> = [];
    if (accessibleModuleIds.has("documents")) {
      try {
        unseenSharedFolders = await listUnseenSharedFolderInvites(userId);
      } catch {
        unseenSharedFolders = [];
      }
    }

    let vsAbsencesATraiter = 0;
    let vsAbsencesJustifFamille = 0;
    let vsAppelsManquants = 0;
    let vsSanctionsAujourdhui = 0;
    let vsCarnetNonSignees = 0;
    let facturesEnRetard = 0;
    let anneeScolaireLabel: string | null = null;

    try {
      const { resolveCurrentEtablissementId } = await import("@/app/lib/ent-core-db");
      const { resolveAnneeCouranteMeta } = await import("@/app/lib/annees-scolaires-db");
      const etabId = await resolveCurrentEtablissementId();
      if (etabId) {
        anneeScolaireLabel = (await resolveAnneeCouranteMeta(etabId)).label;
      }
    } catch {
      anneeScolaireLabel = null;
    }

    if (
      accessibleModuleIds.has("vs-absences") ||
      accessibleModuleIds.has("vs-appels") ||
      accessibleModuleIds.has("vs-calendrier") ||
      accessibleModuleIds.has("vs-sanctions") ||
      accessibleModuleIds.has("vs-carnet")
    ) {
      try {
        const { resolveCurrentEtablissementId } = await import("@/app/lib/ent-core-db");
        const {
          countAbsencesATraiter,
          countAbsencesJustifFamilleEnAttente,
          countAppelsManquants,
        } = await import("@/app/lib/vs-absences-db");
        const { countSanctionsAujourdhui } = await import("@/app/lib/vs-sanctions-db");
        const { countCarnetNonSignees } = await import("@/app/lib/vs-carnet-db");
        const { parisDateKey } = await import("@/app/lib/paris-time");
        const etabId = await resolveCurrentEtablissementId();
        if (etabId) {
          const tasks: Promise<void>[] = [];
          if (accessibleModuleIds.has("vs-absences")) {
            tasks.push(
              countAbsencesATraiter(etabId).then((n) => {
                vsAbsencesATraiter = n;
              }),
            );
            tasks.push(
              countAbsencesJustifFamilleEnAttente(etabId).then((n) => {
                vsAbsencesJustifFamille = n;
              }),
            );
          }
          if (accessibleModuleIds.has("vs-appels") || accessibleModuleIds.has("vs-absences")) {
            tasks.push(
              countAppelsManquants(etabId).then((n) => {
                vsAppelsManquants = n;
              }),
            );
          }
          if (accessibleModuleIds.has("vs-sanctions")) {
            tasks.push(
              countSanctionsAujourdhui(etabId, parisDateKey(new Date())).then((n) => {
                vsSanctionsAujourdhui = n;
              }),
            );
          }
          if (accessibleModuleIds.has("vs-carnet")) {
            tasks.push(
              countCarnetNonSignees(etabId).then((n) => {
                vsCarnetNonSignees = n;
              }),
            );
          }
          await Promise.all(tasks);
        }
      } catch {
        vsAbsencesATraiter = 0;
        vsAppelsManquants = 0;
        vsSanctionsAujourdhui = 0;
        vsCarnetNonSignees = 0;
      }
    }

    try {
      const signals = getDashboardSignals({
        roles,
        userId,
        email,
        accessibleModuleIds,
        trips: Array.isArray(tripsRaw) ? tripsRaw : [],
        absences,
        reservations,
        rooms,
        roomSubjectColors,
        requestsBoard,
        photocopies,
        hse,
        stagesPendingSignatures,
        internatRollCallStatus,
        weekSheet,
        moodPulseSubmittedToday,
        planningNow,
        establishments,
        unseenSharedFolders,
        vsAbsencesATraiter,
        vsAbsencesJustifFamille,
        vsAppelsManquants,
        vsSanctionsAujourdhui,
        vsCarnetNonSignees,
        facturesEnRetard,
        anneeScolaireLabel,
      });
      return NextResponse.json(signals);
    } catch (err) {
      console.error("[dashboard/signals] getDashboardSignals", err);
      return NextResponse.json({ ...EMPTY_SIGNALS, anneeScolaireLabel });
    }
  } catch (e) {
    console.error("[dashboard/signals]", e);
    // Réponse dégradée 200 : un 500 videait aussi l’UI (pas de tuiles / signaux).
    return NextResponse.json(EMPTY_SIGNALS);
  }
}
