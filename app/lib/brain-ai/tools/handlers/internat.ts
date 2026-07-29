import { canAccessInternatModule } from "@/app/lib/internat-rbac";
import { loadAppConfig } from "@/app/lib/app-config";
import { buildDashboardStats, todayDateParis } from "@/app/lib/internat-stats";
import {
  getInternatIncidents,
  getInternatRollCall,
  getInternatRooms,
  getInternatStudents,
  listValidatedRollCalls,
} from "@/app/lib/internat-storage";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const ROLL_STATUS_LABELS: Record<string, string> = {
  non_demarre: "non démarré",
  en_cours: "en cours",
  validee: "validé",
};

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
      statusLabel: rollLabel,
      present: stats.tonightRollCall.presentCount,
      absent: stats.tonightRollCall.absentCount,
      excused: stats.tonightRollCall.excusedCount,
    },
    incidents30d: stats.incidents30d,
    underWatchCount: stats.studentsUnderWatch.length,
    roomsOverCapacity: stats.roomsOverCapacity.length,
    presenceRate7d: stats.presenceRate7d,
    /** Pas de liste nominative d'élèves sous surveillance — uniquement le compte. */
    ctas: [{ label: "Ouvrir Internat", href: "/gestion-internat" }],
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
