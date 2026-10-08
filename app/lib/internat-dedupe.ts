import type { InternatStudent } from "@/app/lib/internat-types";

/** Normalise une clé personne (accents / casse / apostrophes / ponctuation). */
export function normalizeInternatPersonPart(raw: string | undefined | null): string {
  return String(raw ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    // Apostrophes / quotes typographiques : N'SONI → nsoni (pas « n soni »)
    .replace(/[\u0027\u2018\u2019\u0060\u00b4\u02bc]/g, "")
    .replace(/[—–−]/g, "-")
    .replace(/[^a-z0-9]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Tokens triés issus de nom + prénom (ordre indifférent). */
export function internatNameTokens(nom: string, prenom: string): string[] {
  const parts = `${normalizeInternatPersonPart(nom)} ${normalizeInternatPersonPart(prenom)}`
    .split(" ")
    .map((t) => t.trim())
    .filter(Boolean);
  return [...new Set(parts)].sort();
}

export function internatNameBagKey(nom: string, prenom: string): string {
  const tokens = internatNameTokens(nom, prenom);
  return tokens.length ? `bag:${tokens.join("|")}` : "";
}

export function internatNameKey(nom: string, prenom: string): string {
  return `name:${normalizeInternatPersonPart(nom)}|${normalizeInternatPersonPart(prenom)}`;
}

export function internatStudentIdentityTokens(s: {
  eleveRef: { ine?: string; folderName?: string; nom: string; prenom: string };
}): string[] {
  const tokens: string[] = [];
  const ine = String(s.eleveRef.ine || "")
    .trim()
    .toUpperCase();
  if (ine) tokens.push(`ine:${ine}`);
  const folder = normalizeInternatPersonPart(s.eleveRef.folderName);
  if (folder) tokens.push(`folder:${folder}`);
  // Ordre saisi
  tokens.push(internatNameKey(s.eleveRef.nom, s.eleveRef.prenom));
  // Ordre inversé (import Excel parfois « Prénom Nom » dans les mauvaises colonnes)
  const swapped = internatNameKey(s.eleveRef.prenom, s.eleveRef.nom);
  if (swapped !== internatNameKey(s.eleveRef.nom, s.eleveRef.prenom)) {
    tokens.push(swapped);
  }
  // Sac de mots : « Dane Junior » + « N'SONI » ≡ « NSONI » + « Dane Junior »
  const bag = internatNameBagKey(s.eleveRef.nom, s.eleveRef.prenom);
  if (bag) tokens.push(bag);
  return tokens;
}

export function internatRosterIdentityTokens(entry: {
  ine?: string;
  folderName?: string;
  nom: string;
  prenom: string;
}): string[] {
  return internatStudentIdentityTokens({
    eleveRef: {
      ine: entry.ine,
      folderName: entry.folderName,
      nom: entry.nom,
      prenom: entry.prenom,
    },
  });
}

export function internatStudentsShareIdentity(
  a: InternatStudent,
  b: InternatStudent,
): boolean {
  const aTokens = new Set(internatStudentIdentityTokens(a));
  return internatStudentIdentityTokens(b).some((t) => aTokens.has(t));
}

export function internatStudentMatchesRoster(
  s: InternatStudent,
  entry: { ine?: string; folderName?: string; nom: string; prenom: string },
): boolean {
  const sTokens = new Set(internatStudentIdentityTokens(s));
  return internatRosterIdentityTokens(entry).some((t) => sTokens.has(t));
}

function studentKeepScore(s: InternatStudent): number {
  let score = 0;
  if (s.actif) score += 10_000;
  if (s.roomId) score += 1_000;
  if (s.eleveRef.ine?.trim()) score += 500;
  if (s.parent1?.email || s.parent1?.telephone) score += 50;
  if (s.parent2?.email || s.parent2?.telephone) score += 25;
  if (s.medical && Object.values(s.medical).some(Boolean)) score += 40;
  if (s.underWatch) score += 10;
  const updated = Date.parse(s.updatedAt || "") || 0;
  score += Math.min(Math.floor(updated / 1000), 9_999_999);
  return score;
}

function mergeStudentInto(keeper: InternatStudent, donor: InternatStudent, at: string, by: string): InternatStudent {
  return {
    ...keeper,
    eleveRef: {
      ine: keeper.eleveRef.ine || donor.eleveRef.ine,
      folderName: keeper.eleveRef.folderName || donor.eleveRef.folderName,
      nom: keeper.eleveRef.nom || donor.eleveRef.nom,
      prenom: keeper.eleveRef.prenom || donor.eleveRef.prenom,
    },
    roomId: keeper.roomId || donor.roomId,
    parent1: keeper.parent1 || donor.parent1,
    parent2: keeper.parent2 || donor.parent2,
    medical: keeper.medical || donor.medical,
    specialAuthorizations:
      (keeper.specialAuthorizations?.length || 0) >= (donor.specialAuthorizations?.length || 0)
        ? keeper.specialAuthorizations
        : donor.specialAuthorizations,
    underWatch: keeper.underWatch || donor.underWatch,
    underWatchNote: keeper.underWatchNote || donor.underWatchNote,
    updatedAt: at,
    history: [
      ...(keeper.history || []),
      ...(donor.history || []),
      {
        at,
        by,
        action: "FUSION_DOUBLON",
        note: `Fusionné avec ${donor.id} (${donor.eleveRef.nom} ${donor.eleveRef.prenom})`,
      },
    ],
  };
}

/**
 * Regroupe les fiches internat qui partagent INE, dossier, nom+prénom
 * (accents / apostrophes ignorés, ordre nom/prénom indifférent).
 * Conserve la meilleure fiche, purge les autres.
 */
export function dedupeInternatStudents(
  students: InternatStudent[],
  opts?: { by?: string; at?: string },
): { students: InternatStudent[]; mergedGroups: number; removedActifs: number } {
  const at = opts?.at || new Date().toISOString();
  const by = opts?.by || "systeme:dedupe";
  if (students.length < 2) {
    return { students: [...students], mergedGroups: 0, removedActifs: 0 };
  }

  const parent = new Map<string, string>();
  const find = (id: string): string => {
    const p = parent.get(id) || id;
    if (p !== id) {
      const root = find(p);
      parent.set(id, root);
      return root;
    }
    return id;
  };
  const union = (a: string, b: string) => {
    const ra = find(a);
    const rb = find(b);
    if (ra !== rb) parent.set(rb, ra);
  };

  for (const s of students) parent.set(s.id, s.id);

  const tokenOwner = new Map<string, string>();
  for (const s of students) {
    for (const token of internatStudentIdentityTokens(s)) {
      const existing = tokenOwner.get(token);
      if (existing) union(existing, s.id);
      else tokenOwner.set(token, s.id);
    }
  }

  const groups = new Map<string, InternatStudent[]>();
  for (const s of students) {
    const root = find(s.id);
    const list = groups.get(root) || [];
    list.push(s);
    groups.set(root, list);
  }

  const out: InternatStudent[] = [];
  let mergedGroups = 0;
  let removedActifs = 0;

  for (const group of groups.values()) {
    if (group.length === 1) {
      out.push(group[0]!);
      continue;
    }
    mergedGroups += 1;
    const ranked = [...group].sort((a, b) => studentKeepScore(b) - studentKeepScore(a));
    let keeper = ranked[0]!;
    for (const donor of ranked.slice(1)) {
      if (donor.actif) removedActifs += 1;
      keeper = mergeStudentInto(keeper, donor, at, by);
    }
    out.push(keeper);
  }

  out.sort((a, b) => {
    const an = `${a.eleveRef.nom} ${a.eleveRef.prenom}`.localeCompare(
      `${b.eleveRef.nom} ${b.eleveRef.prenom}`,
      "fr",
    );
    if (an !== 0) return an;
    return Number(b.actif) - Number(a.actif);
  });

  return { students: out, mergedGroups, removedActifs };
}

/** Cherche le premier interne qui correspond à un élève référentiel. */
export function findInternatStudentForEleve(
  students: InternatStudent[],
  eleve: { ine?: string | null; folderName?: string | null; nom: string; prenom: string },
): InternatStudent | undefined {
  return students.find((s) =>
    internatStudentMatchesRoster(s, {
      ine: eleve.ine || undefined,
      folderName: eleve.folderName || undefined,
      nom: eleve.nom,
      prenom: eleve.prenom,
    }),
  );
}
