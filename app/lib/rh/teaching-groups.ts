/**
 * Groupes pédagogiques EDT — composition multi-classes (ex. latinistes 3A+3B+3C).
 * Stockage créneau : classes[] + groupId/groupLabel optionnels.
 */

export type TeachingGroup = {
  id: string;
  label: string;
  classNames: string[];
  establishmentId?: string | null;
};

export type TeachingGroupsConfig = {
  groups: TeachingGroup[];
};

function newId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

export function defaultTeachingGroupsConfig(): TeachingGroupsConfig {
  return { groups: [] };
}

export function parseTeachingGroup(raw: unknown): TeachingGroup {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const id = typeof o.id === "string" && o.id.trim() ? o.id.trim() : newId("grp");
  const label = typeof o.label === "string" && o.label.trim() ? o.label.trim() : id;
  const classNames = Array.isArray(o.classNames)
    ? o.classNames
        .map((c) => (typeof c === "string" ? c.trim() : ""))
        .filter(Boolean)
        .slice(0, 24)
    : [];
  const establishmentId =
    typeof o.establishmentId === "string" && o.establishmentId.trim()
      ? o.establishmentId.trim()
      : null;
  return { id, label, classNames, establishmentId };
}

export function parseTeachingGroupsConfig(raw: unknown): TeachingGroupsConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const groups = Array.isArray(o.groups) ? o.groups.map(parseTeachingGroup) : [];
  return { groups };
}

export function emptyTeachingGroup(partial?: Partial<TeachingGroup>): TeachingGroup {
  return {
    id: newId("grp"),
    label: "Nouveau groupe",
    classNames: [],
    establishmentId: null,
    ...partial,
  };
}

function sortedKey(names: string[]): string {
  return [...names]
    .map((c) => c.trim().toLowerCase())
    .filter(Boolean)
    .sort()
    .join("|");
}

/** Retrouve un groupe dont les classes correspondent exactement au créneau. */
export function findTeachingGroupByClasses(
  groups: TeachingGroup[],
  classes: string[],
): TeachingGroup | null {
  const key = sortedKey(classes);
  if (!key) return null;
  return groups.find((g) => sortedKey(g.classNames) === key) || null;
}

export function applyTeachingGroupToSlotFields(group: TeachingGroup): {
  classes: string[];
  groupId: string;
  groupLabel: string;
} {
  return {
    classes: [...group.classNames],
    groupId: group.id,
    groupLabel: group.label,
  };
}

/** Libellé affiché sur le calendrier : groupe si présent, sinon classes. */
export function slotAudienceLabel(slot: {
  classes?: string[] | null;
  groupLabel?: string | null;
}): string {
  const group = (slot.groupLabel || "").trim();
  if (group) return group;
  const classes = (slot.classes || []).map((c) => c.trim()).filter(Boolean);
  return classes.join(", ") || "Classe ?";
}
