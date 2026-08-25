/** Détection de conflits EDT — contraintes dures explicites (pas un solveur opaque). */

export type EdtCreneauConflictInput = {
  id: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe?: string | null;
  groupeId?: string | null;
  groupeCode?: string | null;
  enseignantNom?: string | null;
  salle?: string | null;
  semaine?: string | null;
};

export type EdtConflictKind = "enseignant" | "classe" | "groupe" | "salle";

export type EdtConflict = {
  kind: EdtConflictKind;
  label: string;
  creneauIds: [string, string];
  detail: string;
};

function toMinutes(hhmm: string): number {
  const m = /^(\d{1,2}):(\d{2})$/.exec(String(hhmm || "").trim());
  if (!m) return -1;
  return Number(m[1]) * 60 + Number(m[2]);
}

function overlaps(aStart: number, aEnd: number, bStart: number, bEnd: number): boolean {
  return aStart >= 0 && bStart >= 0 && aStart < bEnd && bStart < aEnd;
}

function semainesCompatibles(a: string | null | undefined, b: string | null | undefined): boolean {
  const sa = (a || "AB").trim().toUpperCase() || "AB";
  const sb = (b || "AB").trim().toUpperCase() || "AB";
  if (sa === "AB" || sb === "AB") return true;
  return sa === sb;
}

function norm(s: string | null | undefined): string {
  return String(s || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim()
    .toLowerCase();
}

/**
 * Liste les conflits durs entre créneaux (enseignant, classe, groupe, salle).
 * Les souhaits souples / génération IA viendront ensuite — ici on explique chaque conflit.
 */
export function detectEdtConflicts(creneaux: EdtCreneauConflictInput[]): EdtConflict[] {
  const out: EdtConflict[] = [];
  const seen = new Set<string>();

  const push = (c: EdtConflict) => {
    const key = `${c.kind}:${[...c.creneauIds].sort().join("|")}`;
    if (seen.has(key)) return;
    seen.add(key);
    out.push(c);
  };

  for (let i = 0; i < creneaux.length; i += 1) {
    const a = creneaux[i]!;
    const aStart = toMinutes(a.heureDebut);
    const aEnd = toMinutes(a.heureFin);
    if (aStart < 0 || aEnd <= aStart) continue;

    for (let j = i + 1; j < creneaux.length; j += 1) {
      const b = creneaux[j]!;
      if (a.jourSemaine !== b.jourSemaine) continue;
      if (!semainesCompatibles(a.semaine, b.semaine)) continue;
      const bStart = toMinutes(b.heureDebut);
      const bEnd = toMinutes(b.heureFin);
      if (!overlaps(aStart, aEnd, bStart, bEnd)) continue;

      const jour = a.jourSemaine;
      const plage = `${a.heureDebut}–${a.heureFin} / ${b.heureDebut}–${b.heureFin}`;

      const ensA = norm(a.enseignantNom);
      const ensB = norm(b.enseignantNom);
      if (ensA && ensB && ensA === ensB) {
        push({
          kind: "enseignant",
          label: `Enseignant en double : ${a.enseignantNom}`,
          creneauIds: [a.id, b.id],
          detail: `Jour ${jour} · ${plage}`,
        });
      }

      const clA = norm(a.classe);
      const clB = norm(b.classe);
      if (clA && clB && clA === clB) {
        push({
          kind: "classe",
          label: `Classe en double : ${a.classe}`,
          creneauIds: [a.id, b.id],
          detail: `Jour ${jour} · ${plage}`,
        });
      }

      if (a.groupeId && b.groupeId && a.groupeId === b.groupeId) {
        push({
          kind: "groupe",
          label: `Groupe en double : ${a.groupeCode || a.groupeId}`,
          creneauIds: [a.id, b.id],
          detail: `Jour ${jour} · ${plage}`,
        });
      }

      const salleA = norm(a.salle);
      const salleB = norm(b.salle);
      if (salleA && salleB && salleA === salleB) {
        push({
          kind: "salle",
          label: `Salle en double : ${a.salle}`,
          creneauIds: [a.id, b.id],
          detail: `Jour ${jour} · ${plage}`,
        });
      }
    }
  }

  return out;
}
