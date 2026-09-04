import type { Secteur } from "@/app/lib/onedrive-eleves-types";
import type {
  PilotageDropSignal,
  PilotageEleveDossier,
  PilotageEleveSummary,
  PilotageFlashPoint,
  PilotageMood,
  PilotageNiveauMoyenne,
  PilotagePieceKind,
  PilotageSignal,
  PilotageSignalId,
} from "@/app/lib/pilotage-eleves-types";

/** Ordre scolaire pour « année en cours / année précédente » (3e avant 2nde, pas 4e). */
export const NIVEAU_PATH = [
  "PS",
  "MS",
  "GS",
  "CP",
  "CE1",
  "CE2",
  "CM1",
  "CM2",
  "6e",
  "5e",
  "4e",
  "3e",
  "2nde",
  "1re",
  "Tle",
] as const;

export const PILOTAGE_SIGNAL_META: Record<
  PilotageSignalId,
  { emoji: string; label: string; hint: string; tone: "plus" | "watch" | "alert" | "info" }
> = {
  sun: {
    emoji: "☀️",
    label: "Tout va bien",
    hint: "Rien de préoccupant sur l’année en cours / précédente.",
    tone: "plus",
  },
  participate: {
    emoji: "✋",
    label: "Participe",
    hint: "Implication / participation active mentionnée au bulletin.",
    tone: "plus",
  },
  rise: {
    emoji: "📈",
    label: "Progression",
    hint: "Moyenne en hausse sur l’année récente.",
    tone: "plus",
  },
  chat: {
    emoji: "💬",
    label: "Bavardage",
    hint: "Tendance à bavarder mentionnée au bulletin.",
    tone: "watch",
  },
  late: {
    emoji: "⏰",
    label: "Retards",
    hint: "Retards mentionnés au bulletin.",
    tone: "watch",
  },
  absent: {
    emoji: "🚪",
    label: "Absences",
    hint: "Absences / assiduité mentionnées au bulletin.",
    tone: "watch",
  },
  work: {
    emoji: "📓",
    label: "Travail",
    hint: "Travail / devoirs à suivre.",
    tone: "watch",
  },
  drop: {
    emoji: "📉",
    label: "Chute",
    hint: "Baisse nette sur l’année récente.",
    tone: "alert",
  },
  pap: {
    emoji: "📎",
    label: "PAP/PAI",
    hint: "Pièce d’aménagement au dossier.",
    tone: "info",
  },
};

export function slugPilotageKey(ine: string | undefined, folderName: string): string {
  const ineKey = String(ine ?? "")
    .trim()
    .replace(/[^a-zA-Z0-9]/g, "");
  if (ineKey.length >= 6) return ineKey.toUpperCase();
  return String(folderName || "eleve")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80) || "eleve";
}

export function dossierS3Key(secteur: Secteur, key: string): string {
  return `pilotage/eleves/${secteur}/${key}.json`;
}

export function indexS3Key(secteur: Secteur): string {
  return `pilotage/index/${secteur}.json`;
}

export const PIECE_KIND_LABEL: Record<PilotagePieceKind, string> = {
  bulletin: "Bulletin",
  pap: "PAP",
  pai: "PAI",
  pps: "PPS",
  gevasco: "GEVASCO",
  tap: "TAP",
  certificat: "Certificat",
  convention: "Convention",
  autre: "Document",
};

export function classifyPieceKind(fileName: string): PilotagePieceKind {
  const n = fileName
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (/\bgevasco\b/.test(n) || /\bgevas[\s._-]?co\b/.test(n)) {
    return "gevasco";
  }
  if (/\b(pap)\b/.test(n) || n.includes("plan d'accompagnement") || n.includes("plan d accompagnement")) {
    return "pap";
  }
  if (/\b(pai)\b/.test(n) || n.includes("projet d'accueil") || n.includes("projet d accueil")) {
    return "pai";
  }
  if (/\b(pps)\b/.test(n) || n.includes("projet personnalise de scolarisation")) {
    return "pps";
  }
  if (/\b(tap)\b/.test(n)) return "tap";
  if (n.includes("bulletin") || n.includes("releve de notes") || n.includes("livret")) return "bulletin";
  if (n.includes("convention") || n.includes("pfmp")) return "convention";
  if (n.includes("certificat") || n.includes("attestation") || n.includes("diplome")) return "certificat";
  return "autre";
}

function fold(s: string | undefined | null): string {
  return String(s ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/['’]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function normClasse(classe: string | undefined): string {
  return fold(classe);
}

/**
 * Fusionne les libellés Pronote hétérogènes : « 1ère A », « 1ere A », « 1A », « 1reA » → « 1re A ».
 * TA/TD… au lycée → Tle A / Tle D.
 */
export function canonicalClasseLabel(raw: string | undefined): string {
  const original = String(raw ?? "").trim();
  if (!original) return "Sans classe";
  let c = fold(original)
    .replace(/\bpremiere\b/g, "1re")
    .replace(/\b1ere\b/g, "1re")
    .replace(/\bseconde\b/g, "2nde")
    .replace(/\b2de\b/g, "2nde")
    .replace(/\bterminale\b/g, "tle")
    .replace(/\btale\b/g, "tle")
    .replace(/\b(\d)eme\b/g, "$1e")
    .replace(/\b(\d)ème\b/g, "$1e");

  const compact = c.replace(/[\s._-]+/g, "");

  const tleLetter = compact.match(/^t(?:le)?([a-e])$/);
  if (tleLetter) return `Tle ${tleLetter[1]!.toUpperCase()}`;

  const premiere = compact.match(/^1(?:re)?([a-e])$/);
  if (premiere) return `1re ${premiere[1]!.toUpperCase()}`;

  const secondeLetter = compact.match(/^2(?:nde)?([a-e])$/);
  if (secondeLetter) return `2nde ${secondeLetter[1]!.toUpperCase()}`;

  const secondeNum = compact.match(/^2(?:nde)?(\d{1,2})$/);
  if (secondeNum) return `2nde ${secondeNum[1]}`;

  const college = compact.match(/^([3-6])e(\d{1,2}|[a-e])$/);
  if (college) {
    const div = college[2]!;
    const pretty = /^[a-e]$/.test(div) ? div.toUpperCase() : div;
    return `${college[1]}e ${pretty}`;
  }

  c = c.replace(/\b1re\b/, "1re").replace(/\b2nde\b/, "2nde").replace(/\btle\b/, "Tle");
  c = c.replace(/^([1-6](?:re|nde|e)?|tle)\s*([a-e0-9]{1,2})$/i, (_, niv: string, div: string) => {
    const n = niv.toLowerCase() === "tle" ? "Tle" : niv;
    return `${n} ${div.toUpperCase()}`;
  });

  if (!c) return original;
  return c.replace(/\b\w/g, (ch) => ch.toUpperCase()).replace(/^Tle\b/, "Tle").replace(/^1re\b/, "1re").replace(/^2nde\b/, "2nde");
}

export function inferSecteurFromClasseLabel(classe: string | undefined): Secteur | null {
  const c = fold(classe);
  const compact = c.replace(/[\s._-]+/g, "");
  if (/^(ps|ms|gs|cp|ce1|ce2|cm1|cm2|tps)\b/.test(c)) return "ecole";
  if (/^[3-6]e/.test(compact) || /\b(6e|5e|4e|3e)\b/.test(c)) return "college";
  if (
    /^(2nde|2de|1re|1ere|tle|terminale)/.test(compact) ||
    /^t[a-e]$/.test(compact) ||
    /^[12][a-e]$/.test(compact) ||
    /^2\d{1,2}$/.test(compact)
  ) {
    return "lycee";
  }
  return null;
}

/** Rang croissant dans la scolarité (plus grand = niveau plus élevé). */
export function classProgressRank(classe: string | undefined): number | null {
  const c = normClasse(classe);
  const compact = c.replace(/[\s._-]+/g, "");
  if (/^(ps|tps)\b/.test(c)) return 1;
  if (/^ms\b/.test(c)) return 2;
  if (/^gs\b/.test(c)) return 3;
  if (/^cp\b/.test(c)) return 4;
  if (/^ce1\b/.test(c)) return 5;
  if (/^ce2\b/.test(c)) return 6;
  if (/^cm1\b/.test(c)) return 7;
  if (/^cm2\b/.test(c)) return 8;
  if (/^(6e|6eme)/.test(compact)) return 16;
  if (/^(5e|5eme)/.test(compact)) return 17;
  if (/^(4e|4eme)/.test(compact)) return 18;
  if (/^(3e|3eme)/.test(compact)) return 19;
  if (/^(2nde|2de|seconde)/.test(compact) || /^2[a-e0-9]/.test(compact)) return 26;
  if (/^(1re|1ere|premiere)/.test(compact) || /^1[a-e]$/.test(compact)) return 27;
  if (/^(tle|tale|terminale)/.test(compact) || /^t[a-e]$/.test(compact)) return 28;
  return null;
}

function cycleOfRank(rank: number): "ecole" | "college" | "lycee" {
  if (rank < 16) return "ecole";
  if (rank < 26) return "college";
  return "lycee";
}

export function inferBulletinMetaFromFileName(fileName: string): {
  periode?: string;
  classe?: string;
  anneeScolaire?: string;
} {
  const n = fold(fileName);
  let periode: string | undefined;
  if (/\b(t1|trimestre\s*1|1er trimestre|premier trimestre)\b/.test(n)) periode = "T1";
  else if (/\b(t2|trimestre\s*2|2e trimestre|2eme trimestre|deuxieme trimestre)\b/.test(n)) periode = "T2";
  else if (/\b(t3|trimestre\s*3|3e trimestre|3eme trimestre|troisieme trimestre)\b/.test(n)) periode = "T3";
  else if (/\b(s1|semestre\s*1|1er semestre)\b/.test(n)) periode = "S1";
  else if (/\b(s2|semestre\s*2|2e semestre|2eme semestre)\b/.test(n)) periode = "S2";

  const year = n.match(/\b(20\d{2})\s*[-/]\s*(20\d{2})\b/);
  const anneeScolaire = year ? `${year[1]}-${year[2]}` : undefined;

  const classHit = n.match(/\b(6e|5e|4e|3e|2nde|2de|1re|1ere|tle|terminale)\s*([a-e0-9]{0,2})\b/);
  const classe = classHit
    ? canonicalClasseLabel(`${classHit[1]} ${classHit[2] || ""}`.trim())
    : undefined;

  return { periode, classe, anneeScolaire };
}

function periodRank(periode: string | undefined): number {
  const p = fold(periode);
  if (/t1|s1|trimestre 1/.test(p)) return 1;
  if (/t2|trimestre 2/.test(p)) return 2;
  if (/t3|s2|trimestre 3/.test(p)) return 3;
  return 0;
}

export function sortBulletinsChrono<
  T extends { classe?: string; anneeScolaire?: string; periode?: string },
>(bulletins: T[]): T[] {
  return [...bulletins].sort((a, b) => {
    const ra = classProgressRank(a.classe) ?? 0;
    const rb = classProgressRank(b.classe) ?? 0;
    if (ra !== rb) return ra - rb;
    const ya = a.anneeScolaire ?? "";
    const yb = b.anneeScolaire ?? "";
    if (ya !== yb) return ya.localeCompare(yb);
    return periodRank(a.periode) - periodRank(b.periode);
  });
}

export function computeDropSignal(
  bulletins: Array<{ moyenneGenerale?: number | null; classe?: string; periode?: string; anneeScolaire?: string }>,
): PilotageDropSignal {
  const withAvg = sortBulletinsChrono(
    bulletins.filter((b) => typeof b.moyenneGenerale === "number" && Number.isFinite(b.moyenneGenerale)),
  ).map((b) => ({
    moyenne: b.moyenneGenerale as number,
    classe: b.classe,
    periode: b.periode,
    annee: b.anneeScolaire,
    rank: classProgressRank(b.classe),
  }));

  if (withAvg.length === 0) {
    return { kind: "none", detail: "Pas de moyenne lisible dans les bulletins classés." };
  }
  if (withAvg.length === 1) {
    return {
      kind: "start",
      detail: "Un seul bulletin chiffré — pas de tendance, point de départ.",
      to: withAvg[0]!.moyenne,
    };
  }

  const last = withAvg[withAvg.length - 1]!;
  const samePeriodPrev =
    last.periode
      ? [...withAvg]
          .reverse()
          .find((b) => b !== last && fold(b.periode) === fold(last.periode) && b.rank !== last.rank)
      : undefined;
  const prev = samePeriodPrev ?? withAvg[withAvg.length - 2]!;
  const delta = last.moyenne - prev.moyenne;
  const cycleChange =
    last.rank != null && prev.rank != null && cycleOfRank(last.rank) !== cycleOfRank(prev.rank);

  const fromLabel = [canonicalClasseLabel(prev.classe), prev.periode].filter(Boolean).join(" ");
  const toLabel = [canonicalClasseLabel(last.classe), last.periode].filter(Boolean).join(" ");

  if (delta >= -0.4) {
    return {
      kind: "none",
      detail: `Pas de chute nette (${fromLabel || "précédent"} → ${toLabel || "dernier"}).`,
      from: prev.moyenne,
      to: last.moyenne,
    };
  }

  if (cycleChange && delta >= -1.4) {
    return {
      kind: "expected_cycle",
      detail: `Recul d’environ ${Math.abs(delta).toFixed(1)} pt au changement de cycle (${fromLabel} → ${toLabel}) — souvent observé.`,
      from: prev.moyenne,
      to: last.moyenne,
    };
  }

  return {
    kind: "drop",
    detail: `Baisse de ${Math.abs(delta).toFixed(1)} pt (${fromLabel} ${prev.moyenne.toFixed(1)} → ${toLabel} ${last.moyenne.toFixed(1)}).`,
    from: prev.moyenne,
    to: last.moyenne,
  };
}

/** 2nde 4 / 2nde A → « 2nde » pour agréger S1+S2 (moyenne d’année). */
export function niveauScolaireLabel(classe: string | undefined): string | null {
  const c = fold(classe);
  const compact = c.replace(/[\s._-]+/g, "");
  if (/^(ps|tps)\b/.test(c)) return "PS";
  if (/^ms\b/.test(c)) return "MS";
  if (/^gs\b/.test(c)) return "GS";
  if (/^cp\b/.test(c)) return "CP";
  if (/^ce1\b/.test(c)) return "CE1";
  if (/^ce2\b/.test(c)) return "CE2";
  if (/^cm1\b/.test(c)) return "CM1";
  if (/^cm2\b/.test(c)) return "CM2";
  if (/^(6e|6eme)/.test(compact)) return "6e";
  if (/^(5e|5eme)/.test(compact)) return "5e";
  if (/^(4e|4eme)/.test(compact)) return "4e";
  if (/^(3e|3eme)/.test(compact)) return "3e";
  if (/^(2nde|2de|seconde)/.test(compact) || /^2[a-e0-9]/.test(compact)) return "2nde";
  if (/^(1re|1ere|premiere)/.test(compact) || /^1[a-e]$/.test(compact)) return "1re";
  if (/^(tle|tale|terminale)/.test(compact) || /^t[a-e]$/.test(compact)) return "Tle";
  return null;
}

export function computeNiveauAverages(
  bulletins: Array<{
    moyenneGenerale?: number | null;
    classe?: string;
    periode?: string;
    anneeScolaire?: string;
  }>,
): PilotageNiveauMoyenne[] {
  const buckets = new Map<
    string,
    { rank: number; sum: number; n: number; periodes: string[]; annee?: string }
  >();
  for (const b of bulletins) {
    if (typeof b.moyenneGenerale !== "number" || !Number.isFinite(b.moyenneGenerale)) continue;
    const niveau = niveauScolaireLabel(b.classe);
    if (!niveau) continue;
    const rank = classProgressRank(b.classe) ?? 0;
    const cur = buckets.get(niveau) ?? { rank, sum: 0, n: 0, periodes: [], annee: b.anneeScolaire };
    cur.sum += b.moyenneGenerale;
    cur.n += 1;
    if (b.periode && !cur.periodes.includes(b.periode)) cur.periodes.push(b.periode);
    if (b.anneeScolaire) cur.annee = b.anneeScolaire;
    buckets.set(niveau, cur);
  }
  return [...buckets.entries()]
    .sort((a, b) => a[1].rank - b[1].rank)
    .map(([niveau, v]) => ({
      niveau,
      moyenne: Math.round((v.sum / v.n) * 10) / 10,
      periodes: v.periodes,
      anneeScolaire: v.annee,
    }));
}

export function previousNiveauLabel(niveau: string): string | null {
  const i = (NIVEAU_PATH as readonly string[]).indexOf(niveau);
  return i > 0 ? NIVEAU_PATH[i - 1]! : null;
}

/**
 * Année en cours (classe actuelle) + année précédente uniquement.
 * 2nde sans bulletin → 3e, pas 4e.
 */
export function recentNiveauxForEleve(
  classeActuelle: string | undefined,
  niveaux: PilotageNiveauMoyenne[],
): PilotageNiveauMoyenne[] {
  if (!niveaux.length) return [];
  const byLabel = new Map(niveaux.map((n) => [n.niveau, n]));
  const current = niveauScolaireLabel(classeActuelle);
  const pick: PilotageNiveauMoyenne[] = [];

  if (current) {
    const here = byLabel.get(current);
    if (here) pick.push(here);
    const prev = previousNiveauLabel(current);
    if (prev) {
      const prevHit = byLabel.get(prev);
      if (prevHit) pick.push(prevHit);
    }
    if (pick.length === 0) {
      const currentRank = classProgressRank(current) ?? 99;
      const below = [...niveaux]
        .filter((n) => (classProgressRank(n.niveau) ?? 0) <= currentRank)
        .sort((a, b) => (classProgressRank(a.niveau) ?? 0) - (classProgressRank(b.niveau) ?? 0));
      if (below.length) pick.push(below[below.length - 1]!);
    }
  } else {
    const sorted = [...niveaux].sort(
      (a, b) => (classProgressRank(a.niveau) ?? 0) - (classProgressRank(b.niveau) ?? 0),
    );
    pick.push(...sorted.slice(-2));
  }

  const seen = new Set<string>();
  return pick
    .filter((n) => {
      if (seen.has(n.niveau)) return false;
      seen.add(n.niveau);
      return true;
    })
    .sort((a, b) => (classProgressRank(a.niveau) ?? 0) - (classProgressRank(b.niveau) ?? 0));
}

export function recentNiveauLabels(
  classeActuelle: string | undefined,
  niveaux: PilotageNiveauMoyenne[],
): Set<string> {
  const recent = recentNiveauxForEleve(classeActuelle, niveaux);
  const labels = new Set(recent.map((n) => n.niveau));
  const current = niveauScolaireLabel(classeActuelle);
  if (current) {
    labels.add(current);
    const prev = previousNiveauLabel(current);
    if (prev) labels.add(prev);
  }
  return labels;
}

export function filterRecentBulletins<
  T extends { classe?: string },
>(
  bulletins: T[],
  classeActuelle: string | undefined,
  niveaux?: PilotageNiveauMoyenne[],
): T[] {
  const allNiv = niveaux ?? computeNiveauAverages(
    bulletins as Array<{ moyenneGenerale?: number | null; classe?: string; periode?: string; anneeScolaire?: string }>,
  );
  const allowed = recentNiveauLabels(classeActuelle, allNiv);
  return bulletins.filter((b) => {
    const n = niveauScolaireLabel(b.classe);
    return n != null && allowed.has(n);
  });
}

function bulletinHaystack(b: {
  appreciation?: string;
  travail?: string;
  comportement?: string;
  absencesMention?: string;
}): string {
  return fold([b.appreciation, b.travail, b.comportement, b.absencesMention].filter(Boolean).join(" \n "));
}

export function detectBehaviorSignals(
  bulletins: Array<{
    appreciation?: string;
    travail?: string;
    comportement?: string;
    absencesMention?: string;
  }>,
): PilotageSignalId[] {
  const h = bulletinHaystack({
    appreciation: bulletins.map((b) => b.appreciation).filter(Boolean).join(" "),
    travail: bulletins.map((b) => b.travail).filter(Boolean).join(" "),
    comportement: bulletins.map((b) => b.comportement).filter(Boolean).join(" "),
    absencesMention: bulletins.map((b) => b.absencesMention).filter(Boolean).join(" "),
  });
  const ids: PilotageSignalId[] = [];
  if (/absent|assiduit|manque d['’ ]?assid|nombreuses absences/.test(h)) ids.push("absent");
  if (/\bretard/.test(h)) ids.push("late");
  if (/bavard|parle trop|trop parle|bavardage|volubil|trop de discussion/.test(h)) ids.push("chat");
  if (/devoirs? (non|pas )|travail insuffisant|manque de travail|ne travaille pas|investit peu/.test(h)) {
    ids.push("work");
  }
  if (
    /particip|s['’]?implique|implication|volontaire|dynamique|actif(ve)? en classe|bon investissement/.test(h)
  ) {
    ids.push("participate");
  }
  return ids;
}

const SIGNAL_PRIORITY: PilotageSignalId[] = [
  "drop",
  "absent",
  "late",
  "chat",
  "work",
  "pap",
  "rise",
  "participate",
  "sun",
];

export function signalFromId(id: PilotageSignalId): PilotageSignal {
  return { id, label: PILOTAGE_SIGNAL_META[id].label };
}

export function moodFromSignals(signals: PilotageSignal[]): PilotageMood {
  const ids = new Set(signals.map((s) => s.id));
  if (ids.has("drop")) return "alert";
  if (ids.has("absent") || ids.has("late") || ids.has("chat") || ids.has("work")) return "watch";
  if (ids.has("sun") || ids.has("participate") || ids.has("rise")) return "plus";
  return "neutral";
}

export function buildEleveSignals(d: {
  flags: PilotageEleveDossier["flags"];
  drop: PilotageDropSignal;
  bulletins: PilotageEleveDossier["bulletins"];
  niveaux: PilotageNiveauMoyenne[];
  extraIds?: PilotageSignalId[];
}): PilotageSignal[] {
  const ids = new Set<PilotageSignalId>();
  if (d.flags.emptyDossier) return [];

  if (d.drop.kind === "drop") ids.add("drop");
  if (d.flags.hasPap || d.flags.hasPai || d.flags.hasPps || d.flags.hasGevasco) ids.add("pap");
  for (const id of detectBehaviorSignals(d.bulletins)) ids.add(id);
  for (const id of d.extraIds ?? []) ids.add(id);

  if (d.niveaux.length >= 2) {
    const last = d.niveaux[d.niveaux.length - 1]!;
    const prev = d.niveaux[d.niveaux.length - 2]!;
    const delta = last.moyenne - prev.moyenne;
    if (delta >= 0.8) ids.add("rise");
    if (delta <= -1) ids.add("drop");
  }

  const negative = ids.has("drop") || ids.has("absent") || ids.has("late") || ids.has("chat") || ids.has("work");
  if (!negative && d.bulletins.length > 0) ids.add("sun");
  if (ids.has("sun") && negative) ids.delete("sun");

  return SIGNAL_PRIORITY.filter((id) => ids.has(id))
    .slice(0, 4)
    .map(signalFromId);
}

export function focusDossierOnRecentYears(d: {
  classe?: string;
  flags: PilotageEleveDossier["flags"];
  bulletins: PilotageEleveDossier["bulletins"];
  moyennesParNiveau?: PilotageNiveauMoyenne[];
  extraSignalIds?: PilotageSignalId[];
}): {
  allNiveaux: PilotageNiveauMoyenne[];
  niveaux: PilotageNiveauMoyenne[];
  bulletins: PilotageEleveDossier["bulletins"];
  drop: PilotageDropSignal;
  signals: PilotageSignal[];
  mood: PilotageMood;
} {
  const allNiveaux = d.moyennesParNiveau?.length
    ? d.moyennesParNiveau
    : computeNiveauAverages(d.bulletins);
  const niveaux = recentNiveauxForEleve(d.classe, allNiveaux);
  const bulletins = filterRecentBulletins(d.bulletins, d.classe, allNiveaux);
  const drop = computeDropSignal(bulletins);
  const signals = buildEleveSignals({
    flags: d.flags,
    drop,
    bulletins,
    niveaux,
    extraIds: d.extraSignalIds,
  });
  return { allNiveaux, niveaux, bulletins, drop, signals, mood: moodFromSignals(signals) };
}

export function buildDeterministicFlashPoints(d: {
  flags: PilotageEleveDossier["flags"];
  drop: PilotageDropSignal;
  bulletins: PilotageEleveDossier["bulletins"];
  moyennesParNiveau?: PilotageNiveauMoyenne[];
  classe?: string;
}): PilotageFlashPoint[] {
  const focus = focusDossierOnRecentYears({
    classe: d.classe,
    flags: d.flags,
    bulletins: d.bulletins,
    moyennesParNiveau: d.moyennesParNiveau,
  });
  const points: PilotageFlashPoint[] = [];
  const niveaux = focus.niveaux;

  if (d.flags.emptyDossier) {
    points.push({
      tone: "info",
      titre: "Dossier encore vide",
      detail: "Pas de pièce indexée — rien à conclure pour le conseil.",
    });
    return points;
  }

  if (niveaux.length >= 2) {
    const last = niveaux[niveaux.length - 1]!;
    const prev = niveaux[niveaux.length - 2]!;
    const delta = last.moyenne - prev.moyenne;
    if (delta <= -1) {
      points.push({
        tone: "alert",
        titre: `Chute ${prev.niveau} → ${last.niveau}`,
        detail: `${prev.moyenne.toFixed(1)} puis ${last.moyenne.toFixed(1)} (moyennes d’année).`,
      });
    } else if (delta >= 0.8) {
      points.push({
        tone: "plus",
        titre: `Progression ${prev.niveau} → ${last.niveau}`,
        detail: `${prev.moyenne.toFixed(1)} puis ${last.moyenne.toFixed(1)}.`,
      });
    }
  } else if (niveaux.length === 1) {
    const n = niveaux[0]!;
    points.push({
      tone: "info",
      titre: `${n.niveau} : ${n.moyenne.toFixed(1)}`,
      detail:
        n.periodes.length > 1
          ? `Moyenne d’année (${n.periodes.join(" + ")}).`
          : "Une seule période chiffrée.",
    });
  }

  if (focus.drop.kind === "drop") {
    points.push({ tone: "alert", titre: "Chute récente", detail: focus.drop.detail });
  }

  return points.slice(0, 3);
}

export function summaryFromDossier(d: PilotageEleveDossier): PilotageEleveSummary {
  const focus = focusDossierOnRecentYears({
    classe: d.classe,
    flags: d.flags,
    bulletins: d.bulletins,
    moyennesParNiveau: d.moyennesParNiveau,
    extraSignalIds: undefined,
  });
  const currentLabel = niveauScolaireLabel(d.classe);
  const display =
    (currentLabel ? focus.niveaux.find((n) => n.niveau === currentLabel) : undefined) ??
    focus.niveaux[focus.niveaux.length - 1];
  const last = [...sortBulletinsChrono(focus.bulletins)]
    .reverse()
    .find((b) => typeof b.moyenneGenerale === "number");
  return {
    key: d.key,
    nom: d.nom,
    prenom: d.prenom,
    classe: canonicalClasseLabel(d.classe),
    folderName: d.folderName,
    emptyDossier: d.flags.emptyDossier || d.pieces.length === 0,
    hasBulletin: d.bulletins.length > 0 || d.pieces.some((p) => p.kind === "bulletin"),
    hasPapPaiPps: d.flags.hasPap || d.flags.hasPai || d.flags.hasPps || d.flags.hasGevasco,
    dropSignal: focus.drop.kind === "drop",
    lastMoyenne: display?.moyenne ?? last?.moyenneGenerale ?? null,
    lastPeriode: display
      ? [display.niveau, display.periodes.join("+") || display.anneeScolaire].filter(Boolean).join(" ")
      : undefined,
    moyenneRecente: display?.moyenne ?? last?.moyenneGenerale ?? null,
    moyenneNiveau: display?.niveau ?? niveauScolaireLabel(last?.classe) ?? undefined,
    mood: focus.mood,
    signals: focus.signals,
  };
}

export function inferSecteurFromOneDrivePath(path: string): Secteur | null {
  const n = path
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (n.includes("lycee") || n.includes("/lycee")) return "lycee";
  if (n.includes("college") || n.includes("/college")) return "college";
  if (n.includes("ecole") || n.includes("/ecole") || n.includes("primaire")) return "ecole";
  return null;
}

export function folderNameFromOneDrivePath(folderPath: string): string {
  const parts = folderPath.replace(/\/+$/, "").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? "";
}

export const PILOTAGE_NOTES_ROOT = "ScolIA - Notes direction";

export function pilotageNotesFilePath(params: {
  secteur: Secteur;
  classe: string;
  folderName: string;
}): string {
  const classe = (params.classe || "sans-classe").replace(/[\\/:*?"<>|]+/g, "-").trim();
  const folder = params.folderName.replace(/[\\/:*?"<>|]+/g, "_").trim();
  return `${PILOTAGE_NOTES_ROOT}/${params.secteur}/${classe}/${folder}.md`;
}

export function compareClassesForSort(a: string, b: string): number {
  const ra = classProgressRank(a) ?? 99;
  const rb = classProgressRank(b) ?? 99;
  if (ra !== rb) return ra - rb;
  return canonicalClasseLabel(a).localeCompare(canonicalClasseLabel(b), "fr", { sensitivity: "base" });
}
