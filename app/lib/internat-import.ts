import type { EleveSecteurInput } from "@/app/lib/onedrive-eleves";
import { loadMefSecteurMap, normMefCode } from "@/app/lib/mef-secteurs";
import { resolveEleveSecteur } from "@/app/lib/onedrive-eleves";
import { normalizeParentContact } from "@/app/lib/internat-outing";
import type { InternatEtablissement, InternatStudent } from "@/app/lib/internat-types";
import { etablissementFromSecteur, newId } from "@/app/lib/internat-types";

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
  lastApplySummary?: { added: number; updated: number; skipped: number };
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
    if (o.etablissement === "Collège" || o.etablissement === "Lycée") {
      entry.etablissement = o.etablissement;
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
  if (entry.etablissement === "Collège" || entry.etablissement === "Lycée") {
    return entry.etablissement;
  }

  const mefMap = await loadMefSecteurMap();
  const secteur = resolveEleveSecteur(entry as EleveSecteurInput, mefMap);
  if (secteur === "college") return "Collège";
  if (secteur === "lycee") return "Lycée";

  const mefOrSecteur = entry.mef || entry.formation || entry.secteur || "";
  if (mefOrSecteur) {
    return etablissementFromSecteur(mefOrSecteur);
  }

  return etablissementFromSecteur(entry.folderName);
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
}): Promise<{ students: InternatStudent[]; added: number; updated: number; skipped: number }> {
  const now = new Date().toISOString();
  let added = 0;
  let updated = 0;
  let skipped = 0;
  const list = [...params.students];

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
      added += 1;
      continue;
    }

    const prev = list[idx]!;
    const next: InternatStudent = {
      ...prev,
      etablissement,
      classe,
      sexe: entry.sexe === "F" || entry.sexe === "M" ? entry.sexe : prev.sexe,
      parent1: entry.parent1 ?? prev.parent1,
      parent2: entry.parent2 ?? prev.parent2,
      actif: true,
      updatedAt: now,
      history: [
        ...(prev.history || []),
        { at: now, by: params.appliedBy, action: "SYNC_ROSTER", note: entry.folderName },
      ],
    };

    const unchanged =
      prev.etablissement === next.etablissement &&
      prev.classe === next.classe &&
      prev.sexe === next.sexe &&
      prev.actif === next.actif;
    if (unchanged) {
      skipped += 1;
    } else {
      updated += 1;
    }
    list[idx] = next;
  }

  return { students: list, added, updated, skipped };
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
