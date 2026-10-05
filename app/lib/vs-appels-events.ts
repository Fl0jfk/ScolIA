import "server-only";

import { recordMetierEvent } from "@/app/lib/eleve-core/journal";

export const VS_APPEL_EVENT_TYPES = {
  ATTENDANCE_CALL_COMPLETED: "attendance.call_completed",
} as const;

export async function emitAttendanceCallCompleted(opts: {
  etablissementId: string;
  appelId: string;
  dateAppel: string;
  creneauId?: string | null;
  classe: string;
  absentEleveIds: string[];
  actorUserId?: string | null;
}): Promise<void> {
  await recordMetierEvent({
    etablissementId: opts.etablissementId,
    type: VS_APPEL_EVENT_TYPES.ATTENDANCE_CALL_COMPLETED,
    aggregate: "appel",
    aggregateId: opts.appelId,
    eleveId: opts.absentEleveIds[0] ?? null,
    actorUserId: opts.actorUserId ?? null,
    payload: {
      dateAppel: opts.dateAppel,
      creneauId: opts.creneauId ?? null,
      classe: opts.classe,
      absentCount: opts.absentEleveIds.length,
      absentEleveIds: opts.absentEleveIds,
    },
  });
}
