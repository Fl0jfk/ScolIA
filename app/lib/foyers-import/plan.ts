/**
 * Import des foyers — logique pure : rapprochement des élèves, reconstitution des foyers
 * et plan de modification face à l'existant. Aucune écriture, aucune dépendance base.
 *
 * Règle absolue : le plan ne contient JAMAIS d'opération sur la table `eleve`
 * (seulement foyer, foyer_responsable, eleve_foyer_link).
 */

import { createHash } from "node:crypto";
import { autoriteFromLien, normEmail, normName, normPhone } from "./normalize";
import { studentKey, type FileAdult, type FileRow } from "./parse";

export const MAX_RESPONSABLES_PAR_FOYER = 4;

/* ───────── Types d'entrée (base) ───────── */

export type DbEleve = {
  id: string;
  nom: string;
  prenom: string;
  dateNaissance: string | null;
  classe: string | null;
  status: string;
  dateSortie: string | null;
};

export type DbResp = {
  id: string;
  foyerId: string;
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  autoriteParentale: boolean;
  payeur: boolean;
  userId: string | null;
  rang: number;
};

export type ExistingData = {
  foyers: Array<{ id: string; label: string; facturation?: boolean; messagerie?: boolean }>;
  responsables: DbResp[];
  links: Array<{ eleveId: string; foyerId: string }>;
};

/* ───────── Types de sortie ───────── */

export type AnomalyKind =
  | "non_trouve"
  | "ambigu"
  | "doublon"
  | "date_invalide"
  | "sans_responsable"
  | "responsable_incomplet"
  | "sans_contact"
  | "contact_invalide"
  | "lien_inconnu"
  | "foyer_plein"
  | "contact_partage"
  | "sous_ensemble"
  | "non_touche";

export type Anomaly = {
  kind: AnomalyKind;
  lines: number[];
  eleve: string;
  message: string;
};

export type FileFoyerAdult = {
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  lien: string;
  autoriteParentale: boolean;
  /** Clé d'identité interne (e-mail, téléphone ou nom). */
  akey: string;
};

type FileFoyer = {
  key: string;
  adults: FileFoyerAdult[];
  /** Clés d'élèves (fichier). */
  studentKeys: string[];
};

type StudentEntry = {
  key: string;
  nom: string;
  prenom: string;
  dateNaissance: string | null;
  codeClasse: string;
  lines: number[];
  foyerKeys: string[];
  match:
    | { status: "reconnu"; eleveId: string; classe: string | null }
    | { status: "non_trouve" }
    | { status: "ambigu"; candidats: Array<{ id: string; classe: string | null; status: string }> };
};

export type Analysis = {
  students: StudentEntry[];
  foyers: Map<string, FileFoyer>;
  anomalies: Anomaly[];
  matchedEleveIds: string[];
  rowCount: number;
};

export type PlanResp = {
  action: "create" | "update" | "unchanged";
  respId: string | null;
  rang: number;
  nom: string;
  prenom: string;
  email: string | null;
  telephone: string | null;
  autoriteParentale: boolean;
  lien: string;
  before: {
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
  } | null;
  changes: string[];
  /** Différences fichier / base NON appliquées (responsable lié à un compte famille). */
  differences: string[];
  /** Responsable existant lié à un compte : jamais supprimé ni modifié (nom, e-mail, téléphone). */
  hasUser: boolean;
};

export type PlanFoyer = {
  /** Identifiant existant, ou « new:N » pour un foyer à créer. */
  ref: string;
  existingId: string | null;
  label: string;
  action: "create" | "update" | "unchanged";
  responsables: PlanResp[];
  /** Responsables existants absents du fichier (conservés). */
  conserves: Array<{ id: string; nom: string; prenom: string; payeur: boolean; hasUser: boolean }>;
  /** Élèves reconnus du fichier rattachés à ce foyer. */
  eleveIds: string[];
  /** Total d'enfants dans le fichier (reconnus ou non). */
  enfantsFichier: number;
};

export type PlanLinkAdd = { eleveId: string; foyerRef: string };
export type PlanLinkRemove = { eleveId: string; foyerId: string };

export type Plan = {
  foyers: PlanFoyer[];
  linksAdd: PlanLinkAdd[];
  linksRemove: PlanLinkRemove[];
  /** Liens non retirés car le foyer a facturation / messagerie / compte famille. */
  liensAVerifier: Array<{ eleveId: string; foyerId: string; label: string; raisons: string[] }>;
  /** Foyers ayant perdu un lien (jamais supprimés). */
  orphelins: Array<{ foyerId: string; label: string; elevesRetires: string[] }>;
  students: Array<{
    eleveId: string;
    nom: string;
    prenom: string;
    classe: string | null;
    codeClasse: string;
    foyerRefs: string[];
    liensAjoutes: number;
    liensRetires: number;
  }>;
  anomalies: Anomaly[];
  counts: {
    lignes: number;
    elevesFichier: number;
    elevesReconnus: number;
    elevesNonTrouves: number;
    elevesAmbigus: number;
    foyersCrees: number;
    foyersMisAJour: number;
    foyersInchanges: number;
    responsablesAjoutes: number;
    responsablesModifies: number;
    liensAjoutes: number;
    liensRetires: number;
    liensAVerifier: number;
    foyersOrphelins: number;
  };
};

export type PlanOptions = {
  /** Cocher « Ne pas retirer les anciens liens ». */
  keepOldLinks: boolean;
  /** Retirer aussi les liens des foyers avec facturation / messagerie / compte famille. */
  removeProtectedLinks?: boolean;
};

/* ───────── Analyse du fichier ───────── */

function isActive(e: DbEleve, today: string): boolean {
  if (e.status !== "inscrit" && e.status !== "preinscrit") return false;
  return !e.dateSortie || e.dateSortie > today;
}

const nameId = (a: { nom: string; prenom: string }) => `${normName(a.nom)}|${normName(a.prenom)}`;
const contactOf = (a: FileAdult): string | null =>
  a.email ? `e:${a.email}` : a.telephone ? `t:${a.telephone}` : null;

export function analyzeFile(rows: FileRow[], eleves: DbEleve[], today: string): Analysis {
  const anomalies: Anomaly[] = [];
  const students = new Map<string, StudentEntry>();
  const foyers = new Map<string, FileFoyer>();
  const seenRow = new Set<string>();

  type Valid = { row: FileRow; sKey: string; label: string };
  const valid: Valid[] = [];

  for (const row of rows) {
    const label = `${row.prenom} ${row.nom}`.trim();
    for (const w of row.warnings) {
      anomalies.push({
        kind: w.startsWith("Responsable incomplet") ? "responsable_incomplet" : "contact_invalide",
        lines: [row.line],
        eleve: label,
        message: w,
      });
    }
    if (!row.dateNaissance) {
      anomalies.push({
        kind: "date_invalide",
        lines: [row.line],
        eleve: label,
        message: `Date de naissance illisible (« ${row.dateBrute} ») — ligne ignorée.`,
      });
      continue;
    }
    const sKey = studentKey(row.nom, row.prenom, row.dateNaissance);
    let st = students.get(sKey);
    if (!st) {
      st = {
        key: sKey,
        nom: row.nom,
        prenom: row.prenom,
        dateNaissance: row.dateNaissance,
        codeClasse: row.codeClasse,
        lines: [],
        foyerKeys: [],
        match: { status: "non_trouve" },
      };
      students.set(sKey, st);
    }
    st.lines.push(row.line);
    if (!row.adults.length) {
      anomalies.push({
        kind: "sans_responsable",
        lines: [row.line],
        eleve: label,
        message: "Aucun responsable exploitable sur la ligne — ignorée.",
      });
      continue;
    }
    valid.push({ row, sKey, label });
  }

  // Contacts (e-mail / téléphone) : noms qui se partagent un même contact.
  // Les noms vus ensemble sur une même ligne forment un groupe (couple) ; plusieurs groupes
  // pour un même contact = adultes sans lien apparent → on ne fusionne pas sur ce contact.
  const parents = new Map<string, Map<string, string>>();
  const find = (m: Map<string, string>, x: string): string => {
    let r = x;
    while (m.get(r) !== r) r = m.get(r)!;
    return r;
  };
  for (const { row } of valid) {
    const byContact = new Map<string, string[]>();
    for (const a of row.adults) {
      const c = contactOf(a);
      if (!c) continue;
      const l = byContact.get(c) ?? [];
      l.push(nameId(a));
      byContact.set(c, l);
    }
    for (const [c, names] of byContact) {
      let m = parents.get(c);
      if (!m) {
        m = new Map();
        parents.set(c, m);
      }
      for (const n of names) if (!m.has(n)) m.set(n, n);
      for (const n of names.slice(1)) m.set(find(m, n), find(m, names[0]!));
    }
  }
  const contactMode = new Map<string, "plain" | "named" | "conflict">();
  for (const [c, m] of parents) {
    const roots = new Set([...m.keys()].map((n) => find(m, n)));
    contactMode.set(c, roots.size > 1 ? "conflict" : m.size > 1 ? "named" : "plain");
  }
  const keyOf = (a: FileAdult): string => {
    const c = contactOf(a);
    if (!c) return `n:${nameId(a)}`;
    const mode = contactMode.get(c);
    if (mode === "conflict") return `${c}@${nameId(a)}`;
    if (mode === "named") return `${c}#${nameId(a)}`;
    return c;
  };
  for (const [c, mode] of contactMode) {
    if (mode !== "conflict") continue;
    const names = [...parents.get(c)!.keys()].map((n) => n.replace("|", " "));
    const lines = valid.filter(({ row }) => row.adults.some((a) => contactOf(a) === c)).map(({ row }) => row.line);
    anomalies.push({
      kind: "contact_partage",
      lines,
      eleve: "",
      message: `Contact ${c.slice(2)} partagé par des responsables de noms différents (${names.join(" / ")}) : non fusionnés sur ce contact.`,
    });
  }

  for (const { row, sKey, label } of valid) {
    const st = students.get(sKey)!;
    // Adultes dédoublonnés dans la ligne.
    const seenKeys = new Set<string>();
    const adults: Array<{ a: FileAdult; k: string }> = [];
    for (const a of row.adults) {
      const k = keyOf(a);
      if (seenKeys.has(k)) continue;
      seenKeys.add(k);
      adults.push({ a, k });
    }
    const foyerKey = adults
      .map((x) => x.k)
      .sort()
      .join("||");
    const rowId = `${sKey}§${foyerKey}`;
    if (seenRow.has(rowId)) {
      anomalies.push({
        kind: "doublon",
        lines: [row.line],
        eleve: label,
        message: "Ligne en doublon (même élève, mêmes responsables) — ignorée.",
      });
      continue;
    }
    seenRow.add(rowId);

    let f = foyers.get(foyerKey);
    if (!f) {
      f = { key: foyerKey, adults: [], studentKeys: [] };
      foyers.set(foyerKey, f);
      for (const { a, k } of adults) {
        f.adults.push({
          akey: k,
          nom: a.nom,
          prenom: a.prenom,
          email: a.email,
          telephone: a.telephone,
          lien: a.lien,
          autoriteParentale: autoriteFromLien(a.lien).value,
        });
      }
    } else {
      fillAdults(f, adults);
    }
    if (!f.studentKeys.includes(sKey)) f.studentKeys.push(sKey);
    if (!st.foyerKeys.includes(foyerKey)) st.foyerKeys.push(foyerKey);
  }

  // Sous-ensembles : un foyer dont les adultes sont tous dans un autre (même contact + même nom)
  // est rattaché à ce dernier, sauf si un même enfant appartient aux deux.
  for (const A of [...foyers.values()].sort((x, y) => (x.key < y.key ? -1 : 1))) {
    if (!foyers.has(A.key)) continue;
    const aKeys = A.adults.map((x) => x.akey);
    const supers = [...foyers.values()].filter(
      (B) => B !== A && B.adults.length > A.adults.length && aKeys.every((k) => B.adults.some((x) => x.akey === k)),
    );
    if (!supers.length) continue;
    const kids = A.studentKeys.map((k) => students.get(k)).filter(Boolean) as StudentEntry[];
    const first = kids[0];
    if (supers.length > 1) {
      anomalies.push({
        kind: "sous_ensemble",
        lines: kids.flatMap((k) => k.lines),
        eleve: first ? `${first.prenom} ${first.nom}` : "",
        message: "Responsables inclus dans plusieurs foyers possibles : foyer laissé séparé.",
      });
      continue;
    }
    const B = supers[0]!;
    if (A.studentKeys.some((s) => B.studentKeys.includes(s))) continue;
    fillAdults(
      B,
      A.adults.map((a) => ({ a, k: a.akey })),
    );
    for (const k of kids) {
      k.foyerKeys = k.foyerKeys.map((x) => (x === A.key ? B.key : x));
      if (!B.studentKeys.includes(k.key)) B.studentKeys.push(k.key);
    }
    foyers.delete(A.key);
  }

  // Anomalies par adulte unique (une seule fois par foyer).
  for (const f of foyers.values()) {
    const kids = f.studentKeys.map((k) => students.get(k)).filter(Boolean) as StudentEntry[];
    const first = kids[0];
    for (const a of f.adults) {
      if (!a.email && !a.telephone) {
        anomalies.push({
          kind: "sans_contact",
          lines: kids.flatMap((k) => k.lines),
          eleve: first ? `${first.prenom} ${first.nom}` : "",
          message: `Responsable sans e-mail ni téléphone : ${a.prenom} ${a.nom}.`,
        });
      }
      if (a.lien && !autoriteFromLien(a.lien).known) {
        anomalies.push({
          kind: "lien_inconnu",
          lines: kids.flatMap((k) => k.lines),
          eleve: first ? `${first.prenom} ${first.nom}` : "",
          message: `Lien de parenté inconnu (« ${a.lien} ») pour ${a.prenom} ${a.nom} : autorité parentale non attribuée.`,
        });
      }
    }
  }

  // Rapprochement des élèves.
  const index = new Map<string, DbEleve[]>();
  for (const e of eleves) {
    const k = studentKey(e.nom, e.prenom, e.dateNaissance);
    const list = index.get(k);
    if (list) list.push(e);
    else index.set(k, [e]);
  }
  const matchedEleveIds: string[] = [];
  for (const st of students.values()) {
    const label = `${st.prenom} ${st.nom}`.trim();
    const cands = index.get(st.key) ?? [];
    if (!cands.length) {
      st.match = { status: "non_trouve" };
      anomalies.push({
        kind: "non_trouve",
        lines: st.lines,
        eleve: label,
        message: "Élève introuvable dans l'établissement (nom + prénom + date de naissance).",
      });
      continue;
    }
    let chosen: DbEleve | null = null;
    if (cands.length === 1) chosen = cands[0]!;
    else {
      const actives = cands.filter((c) => isActive(c, today));
      if (actives.length === 1) chosen = actives[0]!;
    }
    if (chosen) {
      st.match = { status: "reconnu", eleveId: chosen.id, classe: chosen.classe };
      matchedEleveIds.push(chosen.id);
    } else {
      st.match = {
        status: "ambigu",
        candidats: cands.map((c) => ({ id: c.id, classe: c.classe, status: c.status })),
      };
      anomalies.push({
        kind: "ambigu",
        lines: st.lines,
        eleve: label,
        message: `${cands.length} élèves correspondent (homonymes) — non traité.`,
      });
    }
  }

  return { students: [...students.values()], foyers, anomalies, matchedEleveIds, rowCount: rows.length };
}

/** Complète les coordonnées / lien manquants d'un foyer avec les adultes d'une autre ligne. */
function fillAdults(
  f: FileFoyer,
  adults: Array<{ a: { email: string | null; telephone: string | null; lien: string }; k: string }>,
) {
  for (const { a, k } of adults) {
    const target = f.adults.find((x) => x.akey === k);
    if (!target) continue;
    if (!target.email && a.email) target.email = a.email;
    if (!target.telephone && a.telephone) target.telephone = a.telephone;
    if (!target.lien && a.lien) {
      target.lien = a.lien;
      target.autoriteParentale = autoriteFromLien(a.lien).value;
    }
  }
}

/* ───────── Plan face à l'existant ───────── */

function foyerLabel(adults: FileFoyerAdult[]): string {
  const n1 = adults[0]?.nom ?? "";
  const n2 = adults[1]?.nom ?? "";
  if (n2 && normName(n1) !== normName(n2)) return `Foyer ${n1} / ${n2}`.slice(0, 120);
  return `Foyer ${n1}`.slice(0, 120);
}

function phoneEq(a: string | null, b: string | null): boolean {
  if (!a || !b) return false;
  return (normPhone(a) ?? a.replace(/\D/g, "")) === (normPhone(b) ?? b.replace(/\D/g, ""));
}

function emailEq(a: string | null, b: string | null): boolean {
  if (!a || !b) return false;
  return (normEmail(a) ?? a.trim().toLowerCase()) === (normEmail(b) ?? b.trim().toLowerCase());
}

function nameEq(a: { nom: string; prenom: string }, b: { nom: string; prenom: string }): boolean {
  return normName(a.nom) === normName(b.nom) && normName(a.prenom) === normName(b.prenom);
}

function lev(a: string, b: string): number {
  const d = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)] as number[]);
  for (let j = 1; j <= b.length; j++) d[0]![j] = j;
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      d[i]![j] = Math.min(
        d[i - 1]![j]! + 1,
        d[i]![j - 1]! + 1,
        d[i - 1]![j - 1]! + (a[i - 1] === b[j - 1] ? 0 : 1),
      );
    }
  }
  return d[a.length]![b.length]!;
}

/** Prénoms identiques ou proches (initiale, préfixe, une faute de frappe). */
function prenomClose(a: string, b: string): boolean {
  const x = normName(a);
  const y = normName(b);
  if (!x || !y) return false;
  if (x === y || x.startsWith(y) || y.startsWith(x)) return true;
  return lev(x, y) <= (Math.max(x.length, y.length) >= 5 ? 2 : 1);
}

/**
 * Associe chaque adulte du fichier à au plus un responsable existant :
 * nom + prénom, puis e-mail, puis téléphone — l'e-mail ou le téléphone ne comptent que si
 * le prénom est le même (ou proche) : un nouveau conjoint ayant le portable de la mère reste un nouveau responsable.
 */
function matchAdults(adults: FileFoyerAdult[], resps: DbResp[]): Array<DbResp | null> {
  const used = new Set<string>();
  const out: Array<DbResp | null> = adults.map(() => null);
  const passes: Array<(a: FileFoyerAdult, r: DbResp) => boolean> = [
    (a, r) => nameEq(a, r),
    (a, r) => emailEq(a.email, r.email) && prenomClose(a.prenom, r.prenom),
    (a, r) => phoneEq(a.telephone, r.telephone) && prenomClose(a.prenom, r.prenom),
  ];
  for (const pass of passes) {
    adults.forEach((a, i) => {
      if (out[i]) return;
      const hit = resps.find((r) => !used.has(r.id) && pass(a, r));
      if (hit) {
        used.add(hit.id);
        out[i] = hit;
      }
    });
  }
  return out;
}

export function buildPlan(analysis: Analysis, existing: ExistingData, options: PlanOptions): Plan {
  const anomalies = [...analysis.anomalies];
  const studentByKey = new Map(analysis.students.map((s) => [s.key, s]));
  const respsByFoyer = new Map<string, DbResp[]>();
  for (const r of existing.responsables) {
    const l = respsByFoyer.get(r.foyerId);
    if (l) l.push(r);
    else respsByFoyer.set(r.foyerId, [r]);
  }
  for (const l of respsByFoyer.values()) l.sort((a, b) => a.rang - b.rang);
  const foyerLabelById = new Map(existing.foyers.map((f) => [f.id, f.label]));
  const foyerMeta = new Map(existing.foyers.map((f) => [f.id, f]));
  /** Raisons de protéger un foyer : facturation, messagerie, compte famille. */
  const protectionOf = (fid: string): string[] => {
    const raisons: string[] = [];
    const m = foyerMeta.get(fid);
    if (m?.facturation) raisons.push("facturation");
    if (m?.messagerie) raisons.push("messagerie");
    if ((respsByFoyer.get(fid) ?? []).some((r) => r.userId)) raisons.push("compte famille");
    return raisons;
  };
  const linksByEleve = new Map<string, Set<string>>();
  for (const l of existing.links) {
    const s = linksByEleve.get(l.eleveId);
    if (s) s.add(l.foyerId);
    else linksByEleve.set(l.eleveId, new Set([l.foyerId]));
  }

  // Foyers du fichier qui ont au moins un enfant reconnu, dans un ordre déterministe.
  const fileFoyers = [...analysis.foyers.values()]
    .map((f) => ({
      f,
      eleveIds: f.studentKeys
        .map((k) => studentByKey.get(k)?.match)
        .filter((m): m is { status: "reconnu"; eleveId: string; classe: string | null } => m?.status === "reconnu")
        .map((m) => m.eleveId),
    }))
    .filter((x) => x.eleveIds.length > 0)
    .sort((a, b) => (a.f.key < b.f.key ? -1 : a.f.key > b.f.key ? 1 : 0));

  // Rattachement à un foyer existant (glouton par score décroissant).
  type Pair = { fi: number; foyerId: string; score: number; overlap: number };
  const pairs: Pair[] = [];
  fileFoyers.forEach(({ f, eleveIds }, fi) => {
    const cands = new Map<string, number>();
    for (const e of eleveIds) for (const fid of linksByEleve.get(e) ?? []) cands.set(fid, (cands.get(fid) ?? 0) + 1);
    for (const [foyerId, overlap] of cands) {
      const resps = respsByFoyer.get(foyerId) ?? [];
      const score = matchAdults(f.adults, resps).filter(Boolean).length;
      if (score > 0) pairs.push({ fi, foyerId, score, overlap });
    }
  });
  pairs.sort((a, b) => b.score - a.score || b.overlap - a.overlap || (a.foyerId < b.foyerId ? -1 : 1));
  const assigned = new Map<number, string>();
  const claimed = new Set<string>();
  for (const p of pairs) {
    if (assigned.has(p.fi) || claimed.has(p.foyerId)) continue;
    assigned.set(p.fi, p.foyerId);
    claimed.add(p.foyerId);
  }

  const planFoyers: PlanFoyer[] = [];
  const refByFileKey = new Map<string, string>();
  let newCounter = 0;
  let respAdded = 0;
  let respModified = 0;

  fileFoyers.forEach(({ f, eleveIds }, fi) => {
    const existingId = assigned.get(fi) ?? null;
    const dbResps = existingId ? (respsByFoyer.get(existingId) ?? []) : [];
    const matches = existingId ? matchAdults(f.adults, dbResps) : f.adults.map(() => null);
    const usedRangs = new Set(dbResps.map((r) => r.rang));
    const resps: PlanResp[] = [];
    let full = false;
    f.adults.forEach((a, i) => {
      const m = matches[i] ?? null;
      if (m) {
        const all: string[] = [];
        if (normName(a.nom) !== normName(m.nom)) all.push("nom");
        if (normName(a.prenom) !== normName(m.prenom)) all.push("prénom");
        if (a.email && !emailEq(a.email, m.email)) all.push("e-mail");
        if (a.telephone && !phoneEq(a.telephone, m.telephone)) all.push("téléphone");
        if (a.autoriteParentale && !m.autoriteParentale) all.push("autorité parentale");
        // Responsable avec compte famille : on ne touche ni au nom, ni à l'e-mail, ni au téléphone.
        const changes = m.userId ? all.filter((c) => c === "autorité parentale") : all;
        const differences = m.userId ? all.filter((c) => c !== "autorité parentale") : [];
        const nameChanged = changes.includes("nom") || changes.includes("prénom");
        resps.push({
          action: changes.length ? "update" : "unchanged",
          respId: m.id,
          rang: m.rang,
          nom: nameChanged ? a.nom : m.nom,
          prenom: nameChanged ? a.prenom : m.prenom,
          email: a.email && changes.includes("e-mail") ? a.email : m.email,
          telephone: a.telephone && changes.includes("téléphone") ? a.telephone : m.telephone,
          autoriteParentale: m.autoriteParentale || a.autoriteParentale,
          lien: a.lien,
          before: {
            nom: m.nom,
            prenom: m.prenom,
            email: m.email,
            telephone: m.telephone,
            autoriteParentale: m.autoriteParentale,
          },
          changes,
          differences,
          hasUser: !!m.userId,
        });
        if (changes.length) respModified++;
      } else {
        let rang = 1;
        while (usedRangs.has(rang) && rang <= MAX_RESPONSABLES_PAR_FOYER) rang++;
        if (rang > MAX_RESPONSABLES_PAR_FOYER) {
          full = true;
          anomalies.push({
            kind: "foyer_plein",
            lines: f.studentKeys.flatMap((k) => studentByKey.get(k)?.lines ?? []),
            eleve: `${a.prenom} ${a.nom}`,
            message: `Foyer existant déjà à ${MAX_RESPONSABLES_PAR_FOYER} responsables : ${a.prenom} ${a.nom} non ajouté.`,
          });
          return;
        }
        usedRangs.add(rang);
        resps.push({
          action: "create",
          respId: null,
          rang,
          nom: a.nom,
          prenom: a.prenom,
          email: a.email,
          telephone: a.telephone,
          autoriteParentale: a.autoriteParentale,
          lien: a.lien,
          before: null,
          changes: [],
          differences: [],
          hasUser: false,
        });
        respAdded++;
      }
    });
    void full;
    const matchedIds = new Set(matches.filter(Boolean).map((m) => m!.id));
    const conserves = dbResps
      .filter((r) => !matchedIds.has(r.id))
      .map((r) => ({ id: r.id, nom: r.nom, prenom: r.prenom, payeur: r.payeur, hasUser: !!r.userId }));

    const ref = existingId ?? `new:${newCounter++}`;
    refByFileKey.set(f.key, ref);
    const changed = resps.some((r) => r.action !== "unchanged");
    planFoyers.push({
      ref,
      existingId,
      label: existingId ? (foyerLabelById.get(existingId) ?? foyerLabel(f.adults)) : foyerLabel(f.adults),
      action: !existingId ? "create" : changed ? "update" : "unchanged",
      responsables: resps,
      conserves,
      eleveIds,
      enfantsFichier: f.studentKeys.length,
    });
  });

  // Liens élève ↔ foyer.
  const linksAdd: PlanLinkAdd[] = [];
  const linksRemove: PlanLinkRemove[] = [];
  const liensAVerifier: Plan["liensAVerifier"] = [];
  const planStudents: Plan["students"] = [];
  const removedByFoyer = new Map<string, string[]>();
  for (const st of analysis.students) {
    if (st.match.status !== "reconnu") continue;
    const eleveId = st.match.eleveId;
    const refs = st.foyerKeys.map((k) => refByFileKey.get(k)).filter((r): r is string => !!r);
    if (!refs.length) {
      // Aucun foyer valide dans le fichier pour cet élève : on n'y touche pas (aucun lien retiré).
      anomalies.push({
        kind: "non_touche",
        lines: st.lines,
        eleve: `${st.prenom} ${st.nom}`.trim(),
        message: "Aucun responsable exploitable dans le fichier : élève non touché, liens existants conservés.",
      });
      continue;
    }
    const current = linksByEleve.get(eleveId) ?? new Set<string>();
    let added = 0;
    let removed = 0;
    for (const ref of refs) {
      if (ref.startsWith("new:") || !current.has(ref)) {
        linksAdd.push({ eleveId, foyerRef: ref });
        added++;
      }
    }
    if (!options.keepOldLinks) {
      for (const fid of current) {
        if (refs.includes(fid)) continue;
        const raisons = protectionOf(fid);
        if (raisons.length && !options.removeProtectedLinks) {
          liensAVerifier.push({ eleveId, foyerId: fid, label: foyerLabelById.get(fid) ?? "Foyer", raisons });
          continue;
        }
        linksRemove.push({ eleveId, foyerId: fid });
        removed++;
        const l = removedByFoyer.get(fid);
        if (l) l.push(eleveId);
        else removedByFoyer.set(fid, [eleveId]);
      }
    }
    planStudents.push({
      eleveId,
      nom: st.nom,
      prenom: st.prenom,
      classe: st.match.classe,
      codeClasse: st.codeClasse,
      foyerRefs: refs,
      liensAjoutes: added,
      liensRetires: removed,
    });
  }
  const orphelins = [...removedByFoyer.entries()]
    .map(([foyerId, elevesRetires]) => ({
      foyerId,
      label: foyerLabelById.get(foyerId) ?? "Foyer",
      elevesRetires,
    }))
    .sort((a, b) => (a.foyerId < b.foyerId ? -1 : 1));

  const countStatus = (s: string) => analysis.students.filter((x) => x.match.status === s).length;
  const counts: Plan["counts"] = {
    lignes: analysis.rowCount,
    elevesFichier: analysis.students.length,
    elevesReconnus: countStatus("reconnu"),
    elevesNonTrouves: countStatus("non_trouve"),
    elevesAmbigus: countStatus("ambigu"),
    foyersCrees: planFoyers.filter((f) => f.action === "create").length,
    foyersMisAJour: planFoyers.filter((f) => f.action === "update").length,
    foyersInchanges: planFoyers.filter((f) => f.action === "unchanged").length,
    responsablesAjoutes: respAdded,
    responsablesModifies: respModified,
    liensAjoutes: linksAdd.length,
    liensRetires: linksRemove.length,
    liensAVerifier: liensAVerifier.length,
    foyersOrphelins: orphelins.length,
  };

  return { foyers: planFoyers, linksAdd, linksRemove, liensAVerifier, orphelins, students: planStudents, anomalies, counts };
}

/** Plan « inchangé » : plus rien à écrire. */
export function planIsNoop(plan: Plan): boolean {
  return (
    plan.foyers.every((f) => f.action === "unchanged") &&
    plan.linksAdd.length === 0 &&
    plan.linksRemove.length === 0
  );
}

/* ───────── Empreintes ───────── */

export function fileFingerprint(buf: Buffer): string {
  return createHash("sha256").update(buf).digest("hex");
}

/** Empreinte du plan : fichier + options + opérations d'écriture (pas les libellés d'aperçu). */
export function planFingerprint(plan: Plan, fileHash: string, options: PlanOptions): string {
  const ops = {
    f: plan.foyers
      .filter((f) => f.action !== "unchanged")
      .map((f) => [
        f.ref,
        f.action,
        f.label,
        f.responsables
          .filter((r) => r.action !== "unchanged")
          .map((r) => [r.action, r.respId, r.rang, r.nom, r.prenom, r.email, r.telephone, r.autoriteParentale]),
        f.eleveIds,
      ]),
    a: plan.linksAdd.map((l) => [l.eleveId, l.foyerRef]),
    r: plan.linksRemove.map((l) => [l.eleveId, l.foyerId]),
  };
  return createHash("sha256")
    .update(JSON.stringify({
        fileHash,
        keepOldLinks: options.keepOldLinks,
        removeProtectedLinks: !!options.removeProtectedLinks,
        ops,
      }))
    .digest("hex");
}
