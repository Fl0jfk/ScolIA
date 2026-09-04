import type { TeacherPlanningDoc } from "@/app/lib/rh/planning-types";

/** Noms de salles / classes à synchroniser depuis un EDT. */
export function collectRoomNamesFromTeacherPlanning(doc: TeacherPlanningDoc): string[] {
  const names = new Set<string>();
  for (const slot of [...doc.weekA, ...doc.weekB, ...(doc.replacements || [])]) {
    const room = (slot.room || "").trim();
    if (room) names.add(room);
    for (const c of slot.classes || []) {
      const cls = c.trim();
      if (cls && cls.length <= 12 && !/\s/.test(cls)) names.add(cls);
    }
  }
  return [...names];
}

/** Propose une salle pour une sélection de classes (1 classe → salle homonyme). */
export function suggestRoomForClasses(
  classes: string[],
  catalogRooms: string[],
  currentRoom?: string | null,
): string | undefined {
  if (classes.length !== 1) return undefined;
  const cls = classes[0]?.trim();
  if (!cls) return undefined;
  const hit = catalogRooms.find((r) => r.trim().toLowerCase() === cls.toLowerCase());
  const suggested = hit || cls;
  const current = (currentRoom || "").trim();
  if (!current) return suggested;
  if (
    /^[A-Za-z0-9À-ÿ._\-/]{1,12}$/.test(current) &&
    current.toLowerCase() !== suggested.toLowerCase()
  ) {
    return suggested;
  }
  return undefined;
}
