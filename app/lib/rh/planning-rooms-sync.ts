import "server-only";

import { ensureReservationRoomsExist } from "@/app/lib/reservation-rooms-storage";
import type { TeacherPlanningDoc } from "@/app/lib/rh/planning-types";
import { collectRoomNamesFromTeacherPlanning } from "@/app/lib/rh/planning-room-suggest";

/** Collecte les libellés salle d’un EDT prof et les crée en salles de classe (non réservables). */
export async function syncClassroomRoomsFromTeacherPlanning(
  doc: TeacherPlanningDoc,
): Promise<void> {
  const names = collectRoomNamesFromTeacherPlanning(doc);
  if (names.length === 0) return;
  await ensureReservationRoomsExist(
    names.map((name) => ({
      name,
      kind: "classroom" as const,
      bookable: false,
    })),
  );
}
