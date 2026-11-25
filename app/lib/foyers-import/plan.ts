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
  foyers: Array<{ id: string; label: string }>;
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
  | "foyer_plein";

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
  /** Responsable existant lié à un compte : jamais supprimé. */
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
    foyersOrphelins: number;
  };
};

export type PlanOptions = {
  /** Cocher « Ne pas retirer les anciens liens ». */
  keepOldLinks: boolean;
};

/* ───────── Analyse du fichier ───────── */

function adultKeys(adults: FileAdult[]): string[] {
  const keys = adults.map((a) =>
    a.email ? `e:${a.email}` : a.telephone ? `t:${a.telephone}` : `n:${normName(a.nom)}|${normName(a.prenom)}`,
  );
  // E-mail / téléphone commun au couple : on distingue par le prénom.
  const count = new Map<string, number>();
  for (const k of keys) count.set(k, (count.get(k) ?? 0) + 1);
  return keys.map((k, i) => ((count.get(k) ?? 0) > 1 ? `${k}#${normName(adults[i]!.prenom)}` : k));
}

function isActive(e: DbEleve, today: string): boolean {
  if (e.status !== "inscrit" && e.status !== "preinscrit") return false;
  return !e.dateSortie || e.dateSortie > today;
}

export function analyzeFile(rows: FileRow[], eleves: DbEleve[], today: string): Analysis {
  const anomalies: Anomaly[] = [];
  const students = new Map<string, StudentEntry>();
  const foyers = new Map<string, FileFoyer>();
  const seenRow = new Set<string>();

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

    const keys = adultKeys(row.adults);
    const foyerKey = [...keys].sort().join("||");
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
      row.adults.forEach((a, i) => {
        f!.adults.push({
          akey: keys[i]!,
          nom: a.nom,
          prenom: a.prenom,
          email: a.email,
          telephone: a.telephone,
          lien: a.lien,
          autoriteParentale: autoriteFromLien(a.lien).value,
        });
      });
    } else {
      // Complète les coordonnées manquantes avec les autres lignes de la fratrie.
      row.adults.forEach((a, i) => {
        const k = keys[i]!;
        const target = f!.adults.find((x) => x.akey === k);
        if (!target) return;
        if (!target.email && a.email) target.email = a.email;
        if (!target.telephone && a.telephone) target.telephone = a.telephone;
        if (!target.lien && a.lien) {
          target.lien = a.lien;
          target.autoriteParentale = autoriteFromLien(a.lien).value;
        }
      });
    }
    if (!f.studentKeys.includes(sKey)) f.studentKeys.push(sKey);
    if (!st.foyerKeys.includes(foyerKey)) st.foyerKeys.push(foyerKey);
  }

  // Anomalies par adulte unique (une seule fois par foyer).
  for (const f of foyers.values()) {
    const first = students.get(f.studentKeys[0]!);
    const kids = f.studentKeys.map((k) => students.get(k)).filter(Boolean) as StudentEntry[];
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

function anyMatch(a: FileFoyerAdult, r: DbResp): boolean {
  return nameEq(a, r) || emailEq(a.email, r.email) || phoneEq(a.telephone, r.telephone);
}

/** Associe chaque adulte du fichier à au plus un responsable existant (nom, puis e-mail, puis téléphone). */
function matchAdults(adults: FileFoyerAdult[], resps: DbResp[]): Array<DbResp | null> {
  const used = new Set<string>();
  const out: Array<DbResp | null> = adults.map(() => null);
  const passes: Array<(a: FileFoyerAdult, r: DbResp) => boolean> = [
    (a, r) => nameEq(a, r),
    (a, r) => emailEq(a.email, r.email),
    (a, r) => phoneEq(a.telephone, r.telephone),
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
        const changes: string[] = [];
        if (normName(a.nom) !== normName(m.nom)) changes.push("nom");
        if (normName(a.prenom) !== normName(m.prenom)) changes.push("prénom");
        if (a.email && !emailEq(a.email, m.email)) changes.push("e-mail");
        if (a.telephone && !phoneEq(a.telephone, m.telephone)) changes.push("téléphone");
        if (a.autoriteParentale && !m.autoriteParentale) changes.push("autorité parentale");
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
  const planStudents: Plan["students"] = [];
  const removedByFoyer = new Map<string, string[]>();
  for (const st of analysis.students) {
    if (st.match.status !== "reconnu") continue;
    const eleveId = st.match.eleveId;
    const refs = st.foyerKeys.map((k) => refByFileKey.get(k)).filter((r): r is string => !!r);
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
    foyersOrphelins: orphelins.length,
  };

  return { foyers: planFoyers, linksAdd, linksRemove, orphelins, students: planStudents, anomalies, counts };
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
    .update(JSON.stringify({ fileHash, keepOldLinks: options.keepOldLinks, ops }))
    .digest("hex");
}
