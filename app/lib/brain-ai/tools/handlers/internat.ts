import { canAccessInternatModule } from "@/app/lib/internat-rbac";
import { loadAppConfig } from "@/app/lib/app-config";
import { buildDashboardStats, todayDateParis } from "@/app/lib/internat-stats";
import {
  countStudentsInRoom,
  getInternatIncidents,
  getInternatRollCall,
  getInternatRooms,
  getInternatStudents,
  listValidatedRollCalls,
  saveInternatStudents,
  validateRoomCapacity,
} from "@/app/lib/internat-storage";
import type { BrainClientAction, BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const ROLL_STATUS_LABELS: Record<string, string> = {
  non_demarre: "non démarré",
  en_cours: "en cours",
  validee: "validé",
  non_applicable: "non concerné (hors lun–jeu)",
};

function fold(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

export async function handleGetInternatStatus(ctx: BrainToolCtx): Promise<BrainToolResult> {
  if (!ctx.userId) {
    return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  }

  if (!ctx.isOrgAdmin && !canAccessInternatModule(ctx.roles)) {
    return { ok: false, error: "Accès internat réservé.", code: "MODULE_FORBIDDEN" };
  }

  const date = todayDateParis();
  const [students, rooms, tonightRollCall, recentRollCalls, config, incidents] = await Promise.all([
    getInternatStudents(),
    getInternatRooms(),
    getInternatRollCall(date),
    listValidatedRollCalls(30),
    loadAppConfig(),
    getInternatIncidents(),
  ]);

  const stats = buildDashboardStats({
    students,
    rooms,
    tonightRollCall,
    recentRollCalls,
    incidents,
    weeklySummaryEnabled: config.internat.weeklySummaryEnabled,
  });

  const rollLabel = ROLL_STATUS_LABELS[stats.tonightRollCall.status] || stats.tonightRollCall.status;
  const fill =
    stats.occupancy.fillRate != null ? `${stats.occupancy.fillRate} %` : "n/a";

  const brief = {
    date,
    activeStudents: stats.activeStudents,
    occupancy: {
      occupiedBeds: stats.occupancy.occupiedBeds,
      totalBeds: stats.occupancy.totalBeds,
      fillRate: stats.occupancy.fillRate,
    },
    tonightRollCall: {
      status: stats.tonightRollCall.status,
      applicable: stats.tonightRollCall.applicable,
      statusLabel: rollLabel,
      present: stats.tonightRollCall.presentCount,
      absent: stats.tonightRollCall.absentCount,
      excused: stats.tonightRollCall.excusedCount,
    },
    incidents30d: stats.incidents30d,
    underWatchCount: stats.studentsUnderWatch.length,
    roomsOverCapacity: stats.roomsOverCapacity.length,
    presenceRate7d: stats.presenceRate7d,
    ctas: [
      { label: "Ouvrir Internat", href: "/gestion-internat" },
      { label: "Appel du soir", href: "/gestion-internat?tab=appel" },
    ],
  };

  return {
    ok: true,
    data: brief,
    summaryFr:
      `Internat ${date} : ${stats.activeStudents} interne(s), occupation ${fill}. ` +
      `Appel du soir : ${rollLabel}` +
      (stats.tonightRollCall.absentCount
        ? ` (${stats.tonightRollCall.absentCount} absent(s))`
        : "") +
      `. Incidents 30j : ${stats.incidents30d.total}.`,
  };
}

/** Ouvre l’écran d’appel internat. */
export async function handleOpenInternatAppel(
  ctx: BrainToolCtx,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!ctx.isOrgAdmin && !canAccessInternatModule(ctx.roles)) {
    return { ok: false, error: "Accès internat réservé.", code: "MODULE_FORBIDDEN" };
  }
  const href = "/gestion-internat?tab=appel";
  const action: BrainClientAction = { type: "open_route", href, label: "Appel internat" };
  return {
    ok: true,
    data: { clientActions: [action], ctas: [{ label: "Appel du soir", href }] },
    summaryFr: "J’ouvre l’appel internat.",
  };
}

/** Affecte (ou retire) une chambre à un interne. */
export async function handleAssignInternatRoom(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  if (!ctx.isOrgAdmin && !canAccessInternatModule(ctx.roles)) {
    return { ok: false, error: "Accès internat réservé.", code: "MODULE_FORBIDDEN" };
  }

  const students = await getInternatStudents();
  const rooms = await getInternatRooms();
  const active = students.filter((s) => s.actif);

  let studentId = String(args.studentId || args.id || "").trim();
  const query = String(args.query || args.q || args.name || "").trim();

  if (!studentId && query) {
    const folded = fold(query);
    const hits = active.filter((s) => {
      const label = fold(`${s.eleveRef.prenom} ${s.eleveRef.nom} ${s.eleveRef.folderName || ""}`);
      return label.includes(folded);
    });
    if (hits.length === 0) {
      return { ok: false, error: `Aucun interne pour « ${query} ».` };
    }
    if (hits.length > 1) {
      return {
        ok: false,
        needsChoices: true,
        tool: "assign_internat_room",
        field: "studentId",
        promptFr: "Quel interne ?",
        options: hits.slice(0, 12).map((s) => ({
          value: s.id,
          label: `${s.eleveRef.prenom} ${s.eleveRef.nom} (${s.classe})`,
        })),
        draftArgs: { ...args, query },
        selectionType: "single",
      };
    }
    studentId = hits[0]!.id;
  }

  if (!studentId) {
    return {
      ok: false,
      needsChoices: true,
      tool: "assign_internat_room",
      field: "query",
      promptFr: "Nom de l’interne ?",
      options: [],
      draftArgs: { ...args },
      selectionType: "text",
    };
  }

  const student = active.find((s) => s.id === studentId);
  if (!student) return { ok: false, error: "Interne introuvable." };

  let roomIdRaw = args.roomId;
  let roomId: string | null | undefined =
    roomIdRaw === undefined
      ? undefined
      : roomIdRaw === null || roomIdRaw === "" || roomIdRaw === "__clear__"
        ? null
        : String(roomIdRaw).trim();

  if (roomId === undefined) {
    const roomQuery = String(args.roomQuery || args.room || args.chambre || "").trim();
    const freeRooms = rooms
      .map((r) => ({
        room: r,
        free: Math.max(0, r.capacity - countStudentsInRoom(students, r.id)),
      }))
      .filter((x) => x.free > 0);

    if (roomQuery) {
      const fq = fold(roomQuery);
      const match = freeRooms.filter(
        (x) => fold(x.room.label).includes(fq) || fold(x.room.id).includes(fq),
      );
      if (match.length === 1) {
        roomId = match[0]!.room.id;
      } else if (match.length > 1) {
        return {
          ok: false,
          needsChoices: true,
          tool: "assign_internat_room",
          field: "roomId",
          promptFr: "Quelle chambre ?",
          options: match.map((x) => ({
            value: x.room.id,
            label: `${x.room.label} (${x.free} place(s))`,
          })),
          draftArgs: { studentId, query },
          selectionType: "single",
        };
      }
    }

    if (roomId === undefined) {
      if (freeRooms.length === 0) {
        return { ok: false, error: "Aucune chambre avec place libre." };
      }
      return {
        ok: false,
        needsChoices: true,
        tool: "assign_internat_room",
        field: "roomId",
        promptFr: `Chambre pour ${student.eleveRef.prenom} ${student.eleveRef.nom} ?`,
        options: [
          ...freeRooms.slice(0, 20).map((x) => ({
            value: x.room.id,
            label: `${x.room.label} (${x.free} place(s))`,
          })),
          { value: "__clear__", label: "Retirer de la chambre" },
        ],
        draftArgs: { studentId },
        selectionType: "single",
      };
    }
  }

  const finalRoomId = roomId === null ? null : roomId;
  const roomLabel =
    finalRoomId == null
      ? "aucune"
      : rooms.find((r) => r.id === finalRoomId)?.label || finalRoomId;

  if (!ctx.confirmed) {
    return {
      ok: false,
      needsConfirmation: true,
      tool: "assign_internat_room",
      args: { studentId, roomId: finalRoomId },
      summaryFr:
        `Affecter ${student.eleveRef.prenom} ${student.eleveRef.nom} ` +
        `→ chambre « ${roomLabel} »` +
        (student.roomId
          ? ` (actuelle : ${rooms.find((r) => r.id === student.roomId)?.label || student.roomId})`
          : "") +
        ".",
    };
  }

  const cap = validateRoomCapacity(students, rooms, studentId, finalRoomId);
  if (!cap.ok) return { ok: false, error: cap.error };

  const now = new Date().toISOString();
  const next = students.map((s) =>
    s.id === studentId
      ? {
          ...s,
          roomId: finalRoomId,
          updatedAt: now,
          history: [
            ...(s.history || []),
            {
              at: now,
              by: ctx.userId || "scolia",
              action: "AFFECTATION_CHAMBRE",
              note: `Chambre → ${roomLabel}`,
            },
          ],
        }
      : s,
  );
  await saveInternatStudents(next);

  const href = "/gestion-internat";
  return {
    ok: true,
    data: {
      studentId,
      roomId: finalRoomId,
      clientActions: [{ type: "open_route" as const, href, label: "Internat" }],
      ctas: [{ label: "Ouvrir Internat", href }],
    },
    summaryFr: `${student.eleveRef.prenom} ${student.eleveRef.nom} → chambre « ${roomLabel} ».`,
  };
}
