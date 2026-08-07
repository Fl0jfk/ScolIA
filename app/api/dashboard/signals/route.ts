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
import { getJson } from "@/app/lib/s3-storage";
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

async function safeJson<T>(path: string): Promise<T | null> {
  try {
    const hit = await getJson<T>(path);
    return (hit?.data as T) ?? null;
  } catch {
    return null;
  }
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    const userId = gate.ctx.userId;
    const email = user?.primaryEmailAddress?.emailAddress ?? "";
    const isOrgAdmin = isOrgAdminFromPublicMetadata(user?.publicMetadata);

    const accessibleModuleIds = new Set(
      INTRANET_MODULES.filter((m) => rolesAllowModule(roles, m, isOrgAdmin)).map((m) => m.id),
    );
    if (accessibleModuleIds.has("rh")) {
      accessibleModuleIds.add("absences");
      if (canAccessHseModule(roles)) accessibleModuleIds.add("demandes-hse");
    }

    const tripsPromise = accessibleModuleIds.has("travels")
      ? safeJson<TripIndexRow[]>("travels/index.json")
      : Promise.resolve(null);

    const absencesPromise =
      accessibleModuleIds.has("rh") && canViewCalendar(roles)
        ? getAbsenceIndex().catch(() => [] as AbsenceRecord[])
        : Promise.resolve([] as AbsenceRecord[]);

    const roomsPromise = accessibleModuleIds.has("prof-room")
      ? safeJson<{ rooms?: { id: string; name: string }[] } | { id: string; name: string }[]>(
          "reservation-rooms/rooms.json",
        )
      : Promise.resolve(null);

    const reservationsPromise = accessibleModuleIds.has("prof-room")
      ? safeJson<
          Array<{
            id: string;
            roomId: string;
            startsAt: string;
            endsAt?: string;
            subject?: string;
            className?: string;
            status?: string;
          }>
        >("reservation-rooms/reservations.json")
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
              etablissement: "École" | "Collège" | "Lycée";
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
      rooms = roomsRaw;
    } else if (roomsRaw && typeof roomsRaw === "object" && Array.isArray(roomsRaw.rooms)) {
      rooms = roomsRaw.rooms;
    }

    const reservations = Array.isArray(reservationsRaw) ? reservationsRaw : [];
    const roomSubjectColors = withDefaultProfRoomSubjects(
      profRoomRaw ? parseProfRoomModule(profRoomRaw) : defaultProfRoomModule(),
    ).subjectColors;
    const absences = absencesRaw.filter(
      (a) =>
        isAbsenceVisibleOnCalendar(a, userId, roles) ||
        isAbsencePendingForManager(a, userId, roles),
    );
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
        stagesPendingSignatures = listPendingSignaturesForUser(
          conventions,
          userEmail,
          userId,
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
    });

    return NextResponse.json(signals);
  } catch (e) {
    console.error("[dashboard/signals]", e);
    return NextResponse.json({ error: "Impossible de charger les signaux." }, { status: 500 });
  }
}
