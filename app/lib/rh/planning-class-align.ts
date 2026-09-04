import "server-only";

import { foldSchoolClass, schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { loadOfficialSchoolClasses, resolveCanonicalSiecleClass } from "@/app/lib/nomenclature-classes";
import { resolveTeacherPlanningEtabId } from "@/app/lib/rh/planning-teacher-db";
import { humanizePronoteClassCode } from "@/app/lib/rh/planning-pronote-parse";
import type { TeacherPlanningDoc, TeacherPlanningSlot, TeacherReplacementSlot } from "@/app/lib/rh/planning-types";

export type PlanningClassAlignResult = {
  planning: TeacherPlanningDoc;
  /** Libellés uniques après alignement (pour dossiers / notifs). */
  classes: string[];
  remapped: Array<{ from: string; to: string }>;
  unmatched: string[];
};

function resolveAgainstCatalog(raw: string, catalog: string[]): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  for (const official of catalog) {
    if (schoolClassesMatch(trimmed, official)) return official;
  }
  const fold = foldSchoolClass(trimmed);
  for (const official of catalog) {
    if (foldSchoolClass(official) === fold) return official;
  }
  return null;
}

function alignOneClass(
  raw: string,
  official: Awaited<ReturnType<typeof loadOfficialSchoolClasses>>,
  catalog: string[],
): { value: string; remappedFrom?: string; matched: boolean } {
  const trimmed = String(raw || "").trim();
  if (!trimmed) return { value: trimmed, matched: false };

  const humanized = humanizePronoteClassCode(trimmed);
  const candidates = humanized !== trimmed ? [trimmed, humanized] : [trimmed];

  for (const candidate of candidates) {
    if (official.hasLockedSiecle) {
      const siecle = resolveCanonicalSiecleClass(
        candidate,
        official.canonicalByFold,
        official.lockedClasses,
      );
      if (siecle) {
        return siecle === trimmed
          ? { value: siecle, matched: true }
          : { value: siecle, remappedFrom: trimmed, matched: true };
      }
    }
    const fromCatalog = resolveAgainstCatalog(candidate, catalog);
    if (fromCatalog) {
      return fromCatalog === trimmed
        ? { value: fromCatalog, matched: true }
        : { value: fromCatalog, remappedFrom: trimmed, matched: true };
    }
  }

  const fallback = humanized || trimmed;
  return fallback === trimmed
    ? { value: fallback, matched: false }
    : { value: fallback, remappedFrom: trimmed, matched: false };
}

function mapSlotClasses(
  slots: TeacherPlanningSlot[],
  align: (raw: string) => { value: string; remappedFrom?: string; matched: boolean },
  remapped: Map<string, string>,
  unmatched: Set<string>,
  seenOfficial: Set<string>,
): TeacherPlanningSlot[] {
  return slots.map((slot) => {
    const classes = (slot.classes || [])
      .map((c) => {
        const hit = align(c);
        if (hit.remappedFrom && hit.matched) remapped.set(hit.remappedFrom, hit.value);
        if (hit.matched && hit.value) seenOfficial.add(hit.value);
        else if (c.trim()) unmatched.add(hit.value || c.trim());
        return hit.value;
      })
      .filter(Boolean);
    // Déduplique en conservant l’ordre.
    const uniq: string[] = [];
    const seen = new Set<string>();
    for (const c of classes) {
      const k = foldSchoolClass(c);
      if (!k || seen.has(k)) continue;
      seen.add(k);
      uniq.push(c);
    }
    return { ...slot, classes: uniq };
  });
}

function mapReplacementClasses(
  slots: TeacherReplacementSlot[],
  align: (raw: string) => { value: string; remappedFrom?: string; matched: boolean },
  remapped: Map<string, string>,
  unmatched: Set<string>,
  seenOfficial: Set<string>,
): TeacherReplacementSlot[] {
  return slots.map((slot) => {
    const classes = (slot.classes || [])
      .map((c) => {
        const hit = align(c);
        if (hit.remappedFrom && hit.matched) remapped.set(hit.remappedFrom, hit.value);
        if (hit.matched && hit.value) seenOfficial.add(hit.value);
        else if (c.trim()) unmatched.add(hit.value || c.trim());
        return hit.value;
      })
      .filter(Boolean);
    const uniq: string[] = [];
    const seen = new Set<string>();
    for (const c of classes) {
      const k = foldSchoolClass(c);
      if (!k || seen.has(k)) continue;
      seen.add(k);
      uniq.push(c);
    }
    return { ...slot, classes: uniq };
  });
}

/**
 * Aligne les codes classes OCR/Pronote sur le référentiel officiel (Siècle + catalogue).
 * Rend le lien EDT → dossiers élèves plus fluide sans saisie manuelle.
 */
export async function alignTeacherPlanningClasses(
  doc: TeacherPlanningDoc,
  etablissementId?: string | null,
): Promise<PlanningClassAlignResult> {
  const etabId = etablissementId || (await resolveTeacherPlanningEtabId());
  const remapped = new Map<string, string>();
  const unmatched = new Set<string>();
  const seenOfficial = new Set<string>();

  if (!etabId) {
    const classes = collectClasses(doc);
    return { planning: doc, classes, remapped: [], unmatched: [] };
  }

  const [official, appCfg, roster] = await Promise.all([
    loadOfficialSchoolClasses(etabId),
    import("@/app/lib/app-config").then((m) => m.loadAppConfig()),
    import("@/app/lib/school-roster").then((m) => m.loadSchoolRoster()),
  ]);
  const catalogClasses = [
    ...Object.values(appCfg.profRoom.classesByPole || {}).flat(),
    ...roster.classAssignments.map((a) => a.className),
    ...official.lockedClasses,
  ];

  const align = (raw: string) => alignOneClass(raw, official, catalogClasses);

  const planning: TeacherPlanningDoc = {
    ...doc,
    weekA: mapSlotClasses(doc.weekA, align, remapped, unmatched, seenOfficial),
    weekB: mapSlotClasses(doc.weekB, align, remapped, unmatched, seenOfficial),
    replacements: mapReplacementClasses(
      doc.replacements || [],
      align,
      remapped,
      unmatched,
      seenOfficial,
    ),
  };

  for (const to of remapped.values()) unmatched.delete(to);
  for (const c of seenOfficial) unmatched.delete(c);

  // Classes reconnues = référentiel + tout créneau restant (groupes inclus) pour l’accès dossiers.
  const allClasses = collectClasses(planning);

  return {
    planning,
    classes: allClasses,
    remapped: [...remapped.entries()].map(([from, to]) => ({ from, to })),
    unmatched: [...unmatched].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })),
  };
}

function collectClasses(doc: TeacherPlanningDoc): string[] {
  const set = new Set<string>();
  for (const slot of [...doc.weekA, ...doc.weekB, ...(doc.replacements || [])]) {
    for (const c of slot.classes || []) {
      const t = c.trim();
      if (t) set.add(t);
    }
  }
  return [...set].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

export function formatPlanningClassAlignWarnings(result: PlanningClassAlignResult): string[] {
  const out: string[] = [];
  if (result.classes.length > 0) {
    out.push(`Classes reconnues pour les dossiers élèves : ${result.classes.join(", ")}.`);
  }
  if (result.remapped.length > 0) {
    const sample = result.remapped
      .slice(0, 6)
      .map((r) => `${r.from} → ${r.to}`)
      .join(" · ");
    out.push(
      result.remapped.length > 6
        ? `Libellés OCR alignés sur Siècle : ${sample} · +${result.remapped.length - 6}…`
        : `Libellés OCR alignés sur Siècle : ${sample}.`,
    );
  }
  if (result.unmatched.length > 0) {
    out.push(
      `Non rattachés au référentiel (groupes / options ou codes à vérifier) : ${result.unmatched.slice(0, 8).join(", ")}${result.unmatched.length > 8 ? "…" : ""}.`,
    );
  }
  return out;
}
