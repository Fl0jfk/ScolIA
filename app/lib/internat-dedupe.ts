import type {
  InternatParentContact,
  InternatStudent,
  InternatStudentMedical,
} from "@/app/lib/internat-types";

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
  tokens.push(internatNameKey(s.eleveRef.nom, s.eleveRef.prenom));
  const swapped = internatNameKey(s.eleveRef.prenom, s.eleveRef.nom);
  if (swapped !== internatNameKey(s.eleveRef.nom, s.eleveRef.prenom)) {
    tokens.push(swapped);
  }
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

export type InternatDedupeOpts = {
  by?: string;
  at?: string;
  /** studentId → clé S3 photo (si connue) — priorise la fiche qui a la photo. */
  photoS3KeyByStudentId?: ReadonlyMap<string, string>;
};

function studentKeepScore(s: InternatStudent, photoS3KeyByStudentId?: ReadonlyMap<string, string>): number {
  let score = 0;
  if (s.actif) score += 10_000;
  if (photoS3KeyByStudentId?.get(s.id)) score += 5_000;
  if (s.roomId) score += 1_000;
  if (s.eleveRef.ine?.trim()) score += 500;
  if (s.parent1?.email || s.parent1?.telephone) score += 50;
  if (s.parent2?.email || s.parent2?.telephone) score += 25;
  if (s.medical && Object.values(s.medical).some(Boolean)) score += 40;
  if (s.underWatch) score += 10;
  if (s.specialAuthorizations?.length) score += 15 * s.specialAuthorizations.length;
  const updated = Date.parse(s.updatedAt || "") || 0;
  score += Math.min(Math.floor(updated / 1000), 9_999_999);
  return score;
}

function mergeParentContact(
  a?: InternatParentContact,
  b?: InternatParentContact,
): InternatParentContact | undefined {
  if (!a) return b;
  if (!b) return a;
  return {
    nom: a.nom || b.nom,
    email: a.email || b.email,
    telephone: a.telephone || b.telephone,
  };
}

function mergeMedical(
  a?: InternatStudentMedical,
  b?: InternatStudentMedical,
): InternatStudentMedical | undefined {
  if (!a) return b;
  if (!b) return a;
  return {
    allergies: a.allergies || b.allergies,
    pai: a.pai || b.pai,
    treatments: a.treatments || b.treatments,
    notes: a.notes || b.notes,
  };
}

function mergeStudentInto(
  keeper: InternatStudent,
  donor: InternatStudent,
  at: string,
  by: string,
  photoS3KeyByStudentId?: ReadonlyMap<string, string>,
): InternatStudent {
  const keeperPhoto = photoS3KeyByStudentId?.get(keeper.id);
  const donorPhoto = photoS3KeyByStudentId?.get(donor.id);
  // Identité : préférer celle qui porte la photo (sinon garder la fiche principale).
  const preferDonorIdentity = Boolean(donorPhoto && !keeperPhoto);

  const mergedAuths = [...(keeper.specialAuthorizations || [])];
  for (const auth of donor.specialAuthorizations || []) {
    if (!mergedAuths.some((a) => a.id === auth.id || a.label === auth.label)) {
      mergedAuths.push(auth);
    }
  }

  return {
    ...keeper,
    eleveRef: {
      ine: keeper.eleveRef.ine || donor.eleveRef.ine,
      folderName: preferDonorIdentity
        ? donor.eleveRef.folderName || keeper.eleveRef.folderName
        : keeper.eleveRef.folderName || donor.eleveRef.folderName,
      nom: preferDonorIdentity
        ? donor.eleveRef.nom || keeper.eleveRef.nom
        : keeper.eleveRef.nom || donor.eleveRef.nom,
      prenom: preferDonorIdentity
        ? donor.eleveRef.prenom || keeper.eleveRef.prenom
        : keeper.eleveRef.prenom || donor.eleveRef.prenom,
    },
    sexe: keeper.sexe || donor.sexe,
    etablissement: keeper.etablissement || donor.etablissement,
    classe:
      (keeper.classe && keeper.classe !== "—" ? keeper.classe : null) ||
      donor.classe ||
      keeper.classe,
    roomId: keeper.roomId || donor.roomId,
    parent1: mergeParentContact(keeper.parent1, donor.parent1),
    parent2: mergeParentContact(keeper.parent2, donor.parent2),
    medical: mergeMedical(keeper.medical, donor.medical),
    specialAuthorizations: mergedAuths.length ? mergedAuths : undefined,
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
        note: [
          `Fusionné avec ${donor.id} (${donor.eleveRef.nom} ${donor.eleveRef.prenom})`,
          donorPhoto || keeperPhoto ? "photo conservée" : null,
        ]
          .filter(Boolean)
          .join(" — "),
      },
    ],
  };
}

/**
 * Regroupe les fiches internat qui partagent INE, dossier, nom+prénom
 * (accents / apostrophes ignorés, ordre nom/prénom indifférent).
 * Conserve la meilleure fiche (photo, chambre, contacts…), purge les autres.
 */
export function dedupeInternatStudents(
  students: InternatStudent[],
  opts?: InternatDedupeOpts,
): {
  students: InternatStudent[];
  mergedGroups: number;
  removedActifs: number;
  /** Groupes fusionnés : ids conservés + ids absorbés (pour alias photo). */
  mergeTraces: Array<{ keeperId: string; absorbedIds: string[]; members: InternatStudent[] }>;
} {
  const at = opts?.at || new Date().toISOString();
  const by = opts?.by || "systeme:dedupe";
  const photoS3KeyByStudentId = opts?.photoS3KeyByStudentId;
  if (students.length < 2) {
    return { students: [...students], mergedGroups: 0, removedActifs: 0, mergeTraces: [] };
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
  const mergeTraces: Array<{
    keeperId: string;
    absorbedIds: string[];
    members: InternatStudent[];
  }> = [];
  let mergedGroups = 0;
  let removedActifs = 0;

  for (const group of groups.values()) {
    if (group.length === 1) {
      out.push(group[0]!);
      continue;
    }
    mergedGroups += 1;
    const ranked = [...group].sort(
      (a, b) =>
        studentKeepScore(b, photoS3KeyByStudentId) - studentKeepScore(a, photoS3KeyByStudentId),
    );
    let keeper = ranked[0]!;
    const absorbedIds: string[] = [];
    for (const donor of ranked.slice(1)) {
      if (donor.actif) removedActifs += 1;
      absorbedIds.push(donor.id);
      keeper = mergeStudentInto(keeper, donor, at, by, photoS3KeyByStudentId);
    }
    // Propager la photo du donneur vers le keeper pour les étapes suivantes / alias.
    if (photoS3KeyByStudentId instanceof Map) {
      const photo =
        photoS3KeyByStudentId.get(keeper.id) ||
        absorbedIds.map((id) => photoS3KeyByStudentId.get(id)).find(Boolean);
      if (photo) photoS3KeyByStudentId.set(keeper.id, photo);
    }
    mergeTraces.push({ keeperId: keeper.id, absorbedIds, members: group });
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

  return { students: out, mergedGroups, removedActifs, mergeTraces };
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
