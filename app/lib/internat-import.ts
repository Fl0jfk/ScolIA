import type { EleveSecteurInput } from "@/app/lib/onedrive-eleves";
import { loadMefSecteurMap, normMefCode } from "@/app/lib/mef-secteurs";
import { resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import { normalizeParentContact } from "@/app/lib/internat-outing";
import type { InternatEtablissement, InternatStudent } from "@/app/lib/internat-types";
import { internatEtablissementFromRaw, newId } from "@/app/lib/internat-types";
import { loadAppConfig } from "@/app/lib/app-config";
import { buildEleveFolderName, type EleveConfig } from "@/app/lib/eleves-config";
import { isRegimeInterne } from "@/app/lib/eleve-regime";

export const INTERNAT_ROSTER_KEY = "internat/roster.json";

export type InternatRosterEntry = {
  ine?: string;
  nom: string;
  prenom: string;
  folderName: string;
  mef?: string;
  formation?: string;
  secteur?: string;
  classe?: string;
  sexe?: "M" | "F";
  etablissement?: InternatEtablissement;
  parent1?: { nom?: string; email?: string; telephone?: string };
  parent2?: { nom?: string; email?: string; telephone?: string };
};

export type InternatRosterMeta = {
  updatedAt: string;
  updatedBy: string;
  count: number;
  lastAppliedAt?: string;
  lastAppliedBy?: string;
  lastApplySummary?: {
    added: number;
    updated: number;
    skipped: number;
    sorties?: number;
    reactivated?: number;
  };
};

export type InternatRosterFile = {
  meta: InternatRosterMeta;
  entries: InternatRosterEntry[];
};

function rosterKey(e: InternatRosterEntry) {
  const ine = String(e.ine || "").trim().toUpperCase();
  if (ine) return `ine:${ine}`;
  const folder = String(e.folderName || "").trim();
  if (folder) return `folder:${folder}`;
  return `name:${String(e.nom).trim().toUpperCase()}|${String(e.prenom).trim().toUpperCase()}`;
}

export function validateInternatRoster(
  data: unknown,
): { ok: true; entries: InternatRosterEntry[] } | { ok: false; error: string } {
  if (!Array.isArray(data)) {
    return { ok: false, error: "Le fichier doit être un tableau JSON." };
  }
  if (data.length === 0) {
    return { ok: false, error: "La liste ne peut pas être vide." };
  }

  const entries: InternatRosterEntry[] = [];
  const keys = new Set<string>();

  for (let i = 0; i < data.length; i++) {
    const row = data[i];
    if (!row || typeof row !== "object") {
      return { ok: false, error: `Ligne ${i + 1} : entrée invalide.` };
    }
    const o = row as Record<string, unknown>;
    const nom = String(o.nom ?? "").trim();
    const prenom = String(o.prenom ?? "").trim();
    if (!nom || !prenom) {
      return { ok: false, error: `Ligne ${i + 1} : nom et prénom obligatoires.` };
    }

    const entry: InternatRosterEntry = {
      nom,
      prenom,
      ine: String(o.ine ?? "").trim(),
      folderName: String(o.folderName ?? "").trim() || `${nom} — ${prenom}`,
      mef: String(o.mef ?? o.formation ?? "").trim() || undefined,
      formation: String(o.formation ?? "").trim() || undefined,
      secteur: String(o.secteur ?? "").trim() || undefined,
      classe: String(o.classe ?? "").trim() || undefined,
    };

    if (o.sexe === "F" || o.sexe === "M") entry.sexe = o.sexe;
    if (typeof o.etablissement === "string" && o.etablissement.trim()) {
      entry.etablissement = o.etablissement.trim();
    }
    const p1 = normalizeParentContact(o.parent1);
    const p2 = normalizeParentContact(o.parent2);
    if (p1) entry.parent1 = p1;
    if (p2) entry.parent2 = p2;

    const key = rosterKey(entry);
    if (keys.has(key)) {
      return { ok: false, error: `Ligne ${i + 1} : doublon (${nom} ${prenom}).` };
    }
    keys.add(key);
    entries.push(entry);
  }

  return { ok: true, entries };
}

async function resolveInternatEtablissement(
  entry: InternatRosterEntry,
): Promise<InternatEtablissement> {
  const preset = entry.etablissement?.trim();
  // Scripts CLI : pas de contexte Next/headers — utiliser le preset directement.
  if (preset && process.env.ENT_IMPORT_SCRIPT === "1") {
    return preset;
  }
  if (preset) {
    try {
      const bundle = await loadAppConfig();
      return internatEtablissementFromRaw(preset, bundle.establishments) || preset;
    } catch {
      return preset;
    }
  }

  try {
    const bundle = await loadAppConfig();
    const mefMap = await loadMefSecteurMap();
    const secteur = resolveEleveSecteur(entry as EleveSecteurInput, mefMap);
    const fromSecteur = internatEtablissementFromRaw(
      secteur || entry.mef || entry.formation || entry.folderName,
      bundle.establishments,
    );
    return (
      fromSecteur ||
      internatEtablissementFromRaw(entry.folderName, bundle.establishments) ||
      "Lycée"
    );
  } catch {
    return "Lycée";
  }
}

function inferClasse(entry: InternatRosterEntry): string {
  if (entry.classe?.trim()) return entry.classe.trim();
  const parts = entry.folderName.split("—").map((p) => p.trim()).filter(Boolean);
  if (parts.length >= 3) return parts[parts.length - 1]!;
  if (parts.length === 2) return parts[1]!;
  return "—";
}

function studentMatchesRoster(s: InternatStudent, entry: InternatRosterEntry) {
  if (entry.ine && s.eleveRef.ine && s.eleveRef.ine.toUpperCase() === entry.ine.toUpperCase()) return true;
  if (entry.folderName && s.eleveRef.folderName === entry.folderName) return true;
  return (
    s.eleveRef.nom.trim().toUpperCase() === entry.nom.trim().toUpperCase() &&
    s.eleveRef.prenom.trim().toUpperCase() === entry.prenom.trim().toUpperCase()
  );
}

export async function applyInternatRoster(params: {
  entries: InternatRosterEntry[];
  students: InternatStudent[];
  appliedBy: string;
}): Promise<{
  students: InternatStudent[];
  added: number;
  updated: number;
  skipped: number;
  sorties: number;
  reactivated: number;
}> {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;
  let skipped = 0;
  let sorties = 0;
  let reactivated = 0;
  const list = [...params.students];
  const matchedIds = new Set<string>();

  for (const entry of params.entries) {
    const etablissement = await resolveInternatEtablissement(entry);
    const classe = inferClasse(entry);
    const idx = list.findIndex((s) => studentMatchesRoster(s, entry));

    if (idx < 0) {
      const student: InternatStudent = {
        id: newId("stu"),
        eleveRef: {
          ine: entry.ine || undefined,
          folderName: entry.folderName,
          nom: entry.nom,
          prenom: entry.prenom,
        },
        sexe: entry.sexe === "F" ? "F" : "M",
        etablissement,
        classe,
        parent1: entry.parent1,
        parent2: entry.parent2,
        actif: true,
        createdAt: now,
        updatedAt: now,
        history: [{ at: now, by: params.appliedBy, action: "IMPORT_ROSTER", note: entry.folderName }],
      };
      list.push(student);
      matchedIds.add(student.id);
      added += 1;
      continue;
    }

    const prev = list[idx]!;
    matchedIds.add(prev.id);
    const wasSortie = !prev.actif;
    const next: InternatStudent = {
      ...prev,
      etablissement,
      classe,
      sexe: entry.sexe === "F" || entry.sexe === "M" ? entry.sexe : prev.sexe,
      parent1: entry.parent1 ?? prev.parent1,
      parent2: entry.parent2 ?? prev.parent2,
      actif: true,
      sortieAt: undefined,
      sortieMotif: undefined,
      updatedAt: now,
      history: [
        ...(prev.history || []),
        {
          at: now,
          by: params.appliedBy,
          action: wasSortie ? "REACTIVATION_REGIME" : "SYNC_ROSTER",
          note: entry.folderName,
        },
      ],
    };

    const unchanged =
      prev.etablissement === next.etablissement &&
      prev.classe === next.classe &&
      prev.sexe === next.sexe &&
      prev.actif === next.actif &&
      !wasSortie;
    if (wasSortie) {
      reactivated += 1;
    } else if (unchanged) {
      skipped += 1;
    } else {
      updated += 1;
    }
    list[idx] = next;
  }

  // Ex-internes absents du roster : sortie (pas de suppression).
  for (let i = 0; i < list.length; i++) {
    const prev = list[i]!;
    if (matchedIds.has(prev.id) || !prev.actif) continue;
    list[i] = {
      ...prev,
      actif: false,
      sortieAt: now,
      sortieMotif: "Changement de régime — n’est plus interne",
      updatedAt: now,
      history: [
        ...(prev.history || []),
        {
          at: now,
          by: params.appliedBy,
          action: "SORTIE_REGIME",
          note: "Absent du roster internes (Excel / référentiel)",
        },
      ],
    };
    sorties += 1;
  }

  return { students: list, added, updated, skipped, sorties, reactivated };
}

/** Aperçu collège/lycée pour l'UI avant import. */
export async function previewRosterEntry(entry: InternatRosterEntry) {
  const mefMap = await loadMefSecteurMap();
  const mef = normMefCode(String(entry.mef ?? entry.formation ?? ""));
  const secteur = resolveEleveSecteur(entry as EleveSecteurInput, mefMap);
  const etablissement = await resolveInternatEtablissement(entry);
  return {
    etablissement,
    classe: inferClasse(entry),
    mefResolved: mef && mefMap.has(mef),
    secteurHint: secteur,
  };
}

/** Convertit une liste (déjà filtrée ou Excel « internes ») en roster, sans re-filtrer le régime. */
export function elevesAsInternatRosterEntries(eleves: EleveConfig[]): InternatRosterEntry[] {
  const entries: InternatRosterEntry[] = [];
  const keys = new Set<string>();

  for (const e of eleves) {
    const nom = e.nom.trim();
    const prenom = e.prenom.trim();
    if (!nom || !prenom) continue;
    const folderName = e.folderName?.trim() || buildEleveFolderName(nom, prenom);
    const entry: InternatRosterEntry = {
      nom,
      prenom,
      folderName,
      ine: e.ine?.trim() || undefined,
      mef: e.mef || e.formation,
      formation: e.formation,
      secteur: e.secteur,
      classe: e.classe,
      sexe: e.sexe,
    };
    const p1 = normalizeParentContact({
      email: e.parent1Email || e.parentEmail,
      telephone: e.parent1Phone || e.parentPhone,
    });
    const p2 = normalizeParentContact({
      email: e.parent2Email,
      telephone: e.parent2Phone,
    });
    if (p1) entry.parent1 = p1;
    if (p2) entry.parent2 = p2;
    const key = rosterKey(entry);
    if (keys.has(key)) continue;
    keys.add(key);
    entries.push(entry);
  }
  return entries;
}

/** Convertit des élèves du référentiel (filtrés internes) en entrées roster. */
export function elevesToInternatRosterEntries(eleves: EleveConfig[]): InternatRosterEntry[] {
  return elevesAsInternatRosterEntries(eleves.filter((e) => isRegimeInterne(e.regime)));
}

/** Alignement proactif internat ↔ régimes du référentiel (ajouts + sorties). */
export async function syncInternatFromElevesRegime(
  eleves: EleveConfig[],
  appliedBy: string,
): Promise<{
  added: number;
  updated: number;
  skipped: number;
  sorties: number;
  reactivated: number;
  rosterCount: number;
} | null> {
  const hasRegime = eleves.some((e) => e.regime?.trim());
  if (!hasRegime) return null;

  const { getInternatStudents, saveInternatStudents, saveInternatRoster } =
    await import("@/app/lib/internat-storage");

  const entries = elevesToInternatRosterEntries(eleves);
  const students = await getInternatStudents();
  if (!entries.length && !students.some((s) => s.actif)) {
    return null;
  }

  const result = await applyInternatRoster({ entries, students, appliedBy });
  await saveInternatStudents(result.students);

  const now = new Date().toISOString();
  await saveInternatRoster({
    meta: {
      updatedAt: now,
      updatedBy: appliedBy,
      count: entries.length,
      lastAppliedAt: now,
      lastAppliedBy: appliedBy,
      lastApplySummary: {
        added: result.added,
        updated: result.updated,
        skipped: result.skipped,
        sorties: result.sorties,
        reactivated: result.reactivated,
      },
    },
    entries,
  });

  return {
    added: result.added,
    updated: result.updated,
    skipped: result.skipped,
    sorties: result.sorties,
    reactivated: result.reactivated,
    rosterCount: entries.length,
  };
}
