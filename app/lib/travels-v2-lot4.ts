/**
 * Circuit voyages v2, lot 4 — logique PURE (sans accès aux données, sans Node) :
 * clôture des inscriptions, liste officielle, bon cuisine jour par jour, devis rectifié,
 * récapitulatif comptabilité, désistement et purge du lendemain du retour.
 * Les aides financières sont gérées HORS de l'application : rien ici à ce sujet.
 */

import {
  addDaysIso,
  diffDaysIso,
  isIsoDate,
  mirrorPiqueNiqueDatesToWeekdays,
  REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE,
} from "@/app/lib/travels-v2";
import { caseId, studentState, toCsv, type StudentCase } from "@/app/lib/travels-v2-lot3";
import type { FamilyInvitation, MealType } from "@/app/lib/travels-v2-registration-types";

/* ─── Liste officielle ─── */

export type OfficialStudent = {
  ine: string;
  studentName: string;
  classe?: string;
  /** Invitation portant le « oui » signé (signé en ligne ou coupon papier). */
  inv: FamilyInvitation;
};

export type ClosureLists = {
  /** Engagement « oui » valide : signé ou papier, ni retiré ni désisté, pas en suspens. */
  official: OfficialStudent[];
  /** Désaccord non tranché par la direction. */
  suspended: string[];
  /** Retirés par la direction. */
  removed: OfficialStudent[];
  /** Désistés après la clôture. */
  withdrawn: OfficialStudent[];
};

function groupByIne(invs: FamilyInvitation[]): Map<string, FamilyInvitation[]> {
  const byIne = new Map<string, FamilyInvitation[]>();
  for (const i of invs) byIne.set(i.ine, [...(byIne.get(i.ine) || []), i]);
  return byIne;
}

export function engagementInvitation(list: FamilyInvitation[]): FamilyInvitation | null {
  return list.find((i) => i.signature?.decision === "oui") ?? null;
}

export function buildClosureLists(invs: FamilyInvitation[], cases: StudentCase[]): ClosureLists {
  const caseByIne = new Map(cases.map((c) => [caseId(c.ine), c]));
  const out: ClosureLists = { official: [], suspended: [], removed: [], withdrawn: [] };
  for (const [ine, list] of groupByIne(invs)) {
    const state = studentState(list, caseByIne.get(caseId(ine)));
    const inv = engagementInvitation(list);
    if (!inv) continue;
    const entry: OfficialStudent = { ine, studentName: inv.studentName, classe: inv.classe, inv };
    if (state === "inscrit") out.official.push(entry);
    else if (state === "suspens") out.suspended.push(ine);
    else if (state === "retire") out.removed.push(entry);
    else if (state === "desiste") out.withdrawn.push(entry);
  }
  const byName = (a: OfficialStudent, b: OfficialStudent) =>
    a.studentName.localeCompare(b.studentName, "fr", { sensitivity: "base" });
  out.official.sort(byName);
  out.removed.sort(byName);
  out.withdrawn.sort(byName);
  out.suspended.sort();
  return out;
}

/* ─── Clôture : échéance, déclenchement, blocage ─── */

/**
 * Date de clôture des inscriptions : la date limite générale, repoussée jusqu'à la dernière prolongation
 * accordée à une famille qui n'a pas encore signé, sans jamais dépasser J-14 (délai cuisine / devis).
 */
export function closureDueDate(opts: {
  deadline: string | null | undefined;
  departureIso: string | null | undefined;
  extendedDeadlines?: (string | null | undefined)[];
}): string | null {
  if (!isIsoDate(opts.deadline)) return null;
  let due: string = opts.deadline;
  for (const e of opts.extendedDeadlines || []) if (isIsoDate(e) && e > due) due = e;
  if (isIsoDate(opts.departureIso)) {
    const cap = addDaysIso(opts.departureIso, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE);
    if (due > cap) due = cap;
  }
  return due;
}

export type ClosureCheck = {
  dueDate: string | null;
  /** La date de clôture est passée (le jour de la date limite reste ouvert). */
  dueNow: boolean;
  /** Chaque élève de la pré-liste a au moins une réponse signée (oui ou non). */
  allAnswered: boolean;
  unansweredCount: number;
  suspendedInes: string[];
};

export function evaluateClosure(opts: {
  deadline: string | null | undefined;
  departureIso: string | null | undefined;
  todayIso: string;
  invs: FamilyInvitation[];
  cases: StudentCase[];
  preListeInes: string[];
}): ClosureCheck {
  const unsignedExt = opts.invs.filter((i) => !i.signature).map((i) => i.extendedDeadline);
  const dueDate = closureDueDate({ deadline: opts.deadline, departureIso: opts.departureIso, extendedDeadlines: unsignedExt });
  const answered = new Set(opts.invs.filter((i) => i.signature).map((i) => i.ine));
  const ines = [...new Set(opts.preListeInes.map((i) => String(i || "").trim()).filter(Boolean))];
  const unansweredCount = ines.filter((i) => !answered.has(i)).length;
  const lists = buildClosureLists(opts.invs, opts.cases);
  return {
    dueDate,
    dueNow: Boolean(dueDate) && opts.todayIso > (dueDate as string),
    allAnswered: ines.length > 0 && unansweredCount === 0,
    unansweredCount,
    suspendedInes: lists.suspended,
  };
}

export type ClosureAction = "wait" | "close_auto" | "blocked_alert" | "propose_early";

/** Ce que doit faire le job pour un voyage dont les inscriptions sont ouvertes. */
export function decideClosureAction(check: ClosureCheck): ClosureAction {
  if (check.suspendedInes.length > 0) return check.dueNow || check.allAnswered ? "blocked_alert" : "wait";
  if (check.dueNow) return "close_auto";
  if (check.allAnswered) return "propose_early";
  return "wait";
}

/** Clôture à la main (organisateur, direction…) : seulement échéance passée ou toutes les familles ont répondu. */
export function canCloseManually(check: ClosureCheck): { ok: true } | { ok: false; error: string } {
  if (check.suspendedInes.length > 0) {
    return {
      ok: false,
      error: `${check.suspendedInes.length} élève(s) en suspens (désaccord entre responsables) : la direction doit trancher avant la clôture.`,
    };
  }
  if (!check.dueNow && !check.allAnswered) {
    return {
      ok: false,
      error: `Clôture impossible : ${check.unansweredCount} famille(s) n'ont pas encore répondu et la date limite n'est pas passée.`,
    };
  }
  return { ok: true };
}

/* ─── Bon cuisine, jour par jour ─── */

export type MealsDay = { total: number; classique: number; sansPorc: number; vegetarien: number };

/**
 * Repas des élèves de la liste officielle, par jour fixé par l'organisateur : « oui » + type.
 * Un jour non renseigné ou « non » (repas fourni par la famille) n'est pas compté.
 */
export function computeMealsPerDay(official: Pick<OfficialStudent, "inv">[], dayKeys: string[]): Record<string, MealsDay> {
  const out: Record<string, MealsDay> = {};
  for (const day of dayKeys) out[day] = { total: 0, classique: 0, sansPorc: 0, vegetarien: 0 };
  for (const { inv } of official) {
    for (const day of dayKeys) {
      const m = inv.answers?.meals?.[day];
      if (m?.wants !== "oui") continue;
      const type: MealType = m.type === "sans_porc" || m.type === "vegetarien" ? m.type : "classique";
      const d = out[day];
      d.total += 1;
      if (type === "sans_porc") d.sansPorc += 1;
      else if (type === "vegetarien") d.vegetarien += 1;
      else d.classique += 1;
    }
  }
  return out;
}

export function sanitizeAdultMeals(input: unknown, dayKeys: string[]): Record<string, number> {
  const out: Record<string, number> = {};
  const src = (input && typeof input === "object" ? input : {}) as Record<string, unknown>;
  for (const day of dayKeys) {
    const n = Math.floor(Number(src[day] ?? 0));
    out[day] = Number.isFinite(n) && n > 0 ? Math.min(n, 200) : 0;
  }
  return out;
}

/**
 * Écrit le bon cuisine v1 (`piqueNiqueDetails.orders`) : total = élèves + adultes, dont sans porc et végétarien
 * (les repas adultes sont comptés « classiques »). Seuls les jours déjà choisis par l'organisateur sont touchés.
 */
export function applyMealsToCuisineDetails(
  details: Record<string, unknown>,
  perDay: Record<string, MealsDay>,
  adults: Record<string, number>,
): Record<string, unknown> {
  const orders = { ...((details.orders as Record<string, Record<string, unknown>>) || {}) };
  const dayKeys = Object.keys(perDay);
  for (const day of dayKeys) {
    const d = perDay[day];
    orders[day] = {
      ...(orders[day] || {}),
      picnicTotal: String(d.total + (adults[day] || 0)),
      picnicNoPork: String(d.sansPorc),
      picnicVeg: String(d.vegetarien),
    };
  }
  return mirrorPiqueNiqueDatesToWeekdays({ ...details, orders }, dayKeys);
}

/* ─── Devis rectifié ─── */

export const RECTIFIED_QUOTE_THRESHOLD_PCT = 10;

export type RectifiedQuoteVerdict = {
  proposed: boolean;
  quoteNbEleves: number | null;
  officialCount: number;
  deviationPct: number | null;
  reason: string;
};

/**
 * Devis rectifié : proposé si l'effectif officiel s'écarte de PLUS de 10 % de celui du devis ET s'il reste
 * au moins 14 jours avant le départ. Sinon on garde le devis, rien à faire.
 */
export function evaluateRectifiedQuote(opts: {
  needsBus: boolean;
  quoteNbEleves: number | null | undefined;
  officialCount: number;
  departureIso: string | null | undefined;
  todayIso: string;
}): RectifiedQuoteVerdict {
  const q = Number(opts.quoteNbEleves);
  const o = opts.officialCount;
  const base = { quoteNbEleves: Number.isFinite(q) && q > 0 ? q : null, officialCount: o };
  if (!opts.needsBus) return { ...base, proposed: false, deviationPct: null, reason: "Pas de transport en bus." };
  if (!base.quoteNbEleves) return { ...base, proposed: false, deviationPct: null, reason: "Aucun effectif de devis connu." };
  const deviationPct = Math.round((Math.abs(o - q) / q) * 1000) / 10;
  const over = Math.abs(o - q) * 100 > q * RECTIFIED_QUOTE_THRESHOLD_PCT;
  if (!over) {
    return { ...base, proposed: false, deviationPct, reason: `Écart de ${deviationPct} % (10 % maximum) : on garde le devis.` };
  }
  if (!isIsoDate(opts.departureIso)) return { ...base, proposed: false, deviationPct, reason: "Date de départ inconnue." };
  const left = diffDaysIso(opts.todayIso, opts.departureIso);
  if (left < REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE) {
    return {
      ...base,
      proposed: false,
      deviationPct,
      reason: `Écart de ${deviationPct} % mais moins de ${REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE} jours avant le départ : on garde le devis.`,
    };
  }
  return {
    ...base,
    proposed: true,
    deviationPct,
    reason: `Écart de ${deviationPct} % (devis pour ${q} élèves, ${o} inscrits) : un devis rectifié est proposé.`,
  };
}

/* ─── Comptabilité : récapitulatif des engagements ─── */

export type ComptaStatus = "inscrit" | "desiste" | "retire";

export const COMPTA_STATUS_LABELS: Record<ComptaStatus, string> = {
  inscrit: "Inscrit",
  desiste: "Désisté après la clôture",
  retire: "Retiré par la direction",
};

export type ComptaRow = {
  ine: string;
  studentName: string;
  classe: string;
  signerName: string;
  mode: "signé en ligne" | "coupon papier";
  signedAt: string;
  amount: number;
  firstMonth: string;
  lastMonth: string;
  status: ComptaStatus;
  /** Clé objet du fichier (PDF signé ou scan papier). */
  fileKey: string;
  /** Chemin du fichier dans le ZIP. */
  zipPath: string;
};

function extOfKey(key: string): string {
  const m = /\.([A-Za-z0-9]{2,5})$/.exec(key);
  return (m?.[1] || "pdf").toLowerCase();
}

export function safeFileSegment(s: string): string {
  return (
    String(s || "")
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^A-Za-z0-9._-]+/g, "_")
      .replace(/^_+|_+$/g, "")
      .slice(0, 60) || "eleve"
  );
}

/** Tous les engagements « oui » du voyage (y compris désistés / retirés, signalés dans le statut). */
export function buildComptaRows(invs: FamilyInvitation[], cases: StudentCase[]): ComptaRow[] {
  const caseByIne = new Map(cases.map((c) => [caseId(c.ine), c]));
  const rows: ComptaRow[] = [];
  const usedPaths = new Set<string>();
  for (const [ine, list] of groupByIne(invs)) {
    const inv = engagementInvitation(list);
    const sig = inv?.signature;
    const fileKey = sig?.pdfKey || inv?.engagement?.pdfKey || sig?.paper?.scanKey;
    if (!inv || !sig || !fileKey) continue;
    const state = studentState(list, caseByIne.get(caseId(ine)));
    const status: ComptaStatus =
      state === "desiste"
        ? inv.withdrawal?.kind === "retire_direction"
          ? "retire"
          : "desiste"
        : state === "retire"
          ? "retire"
          : "inscrit";
    let zipPath = `engagements/${safeFileSegment(inv.studentName)}-${safeFileSegment(ine)}.${extOfKey(fileKey)}`;
    if (usedPaths.has(zipPath)) zipPath = zipPath.replace(/(\.[a-z0-9]+)$/, `-${inv.id.slice(0, 6)}$1`);
    usedPaths.add(zipPath);
    rows.push({
      ine,
      studentName: inv.studentName,
      classe: inv.classe || "",
      signerName: sig.signerName,
      mode: sig.paper ? "coupon papier" : "signé en ligne",
      signedAt: sig.signedAt,
      amount: inv.engagement?.amount ?? 0,
      firstMonth: inv.engagement?.firstMonth || "",
      lastMonth: inv.engagement?.lastMonth || "",
      status,
      fileKey,
      zipPath,
    });
  }
  return rows.sort((a, b) => a.studentName.localeCompare(b.studentName, "fr", { sensitivity: "base" }));
}

export const COMPTA_HEADER = [
  "Élève",
  "Classe",
  "INE",
  "Parent signataire",
  "Mode",
  "Signé le",
  "Montant (€)",
  "Premier mois",
  "Dernier mois",
  "Statut",
  "Fichier",
];

export function comptaTable(rows: ComptaRow[]): { header: string[]; body: unknown[][] } {
  return {
    header: COMPTA_HEADER,
    body: rows.map((r) => [
      r.studentName,
      r.classe,
      r.ine,
      r.signerName,
      r.mode,
      r.signedAt ? r.signedAt.slice(0, 10) : "",
      r.amount,
      r.firstMonth,
      r.lastMonth,
      COMPTA_STATUS_LABELS[r.status],
      r.zipPath,
    ]),
  };
}

export function comptaCsv(rows: ComptaRow[]): string {
  const t = comptaTable(rows);
  return toCsv(t.header, t.body);
}

/* ─── Purge du lendemain du retour ─── */

/** La purge se déclenche à partir du lendemain de la date de retour (Europe/Paris, `todayIso` fourni par l'appelant). */
export function isPurgeDue(returnIso: string | null | undefined, todayIso: string): boolean {
  if (!isIsoDate(returnIso)) return false;
  return todayIso >= addDaysIso(returnIso, 1);
}

/** Clés objet qu'une invitation doit CONSERVER (engagement signé ou scan papier « oui »). */
export function engagementKeys(inv: FamilyInvitation): string[] {
  if (inv.signature?.decision !== "oui") return [];
  return [inv.signature.pdfKey, inv.engagement?.pdfKey, inv.signature.paper?.scanKey].filter((k): k is string => Boolean(k));
}

/**
 * Enregistrement d'engagement minimal : identité de l'élève, signataire, montants, fichier d'engagement.
 * Plus aucun e-mail, jeton, code, réponse pratique, pièce, tracé, ni motif de désaccord.
 */
export function archiveInvitation(inv: FamilyInvitation, caseRec?: StudentCase | null, nowIso?: string): FamilyInvitation {
  const sig = inv.signature;
  const retired = caseRec?.decision?.kind === "retirer";
  return {
    id: inv.id,
    tripId: inv.tripId,
    ine: inv.ine,
    studentName: inv.studentName,
    classe: inv.classe,
    email: "",
    createdAt: inv.createdAt,
    tokenHashes: [],
    sendCount: 0,
    answers: {},
    documents: {},
    exemptions: {},
    codeSends: [],
    ...(sig
      ? {
          signature: {
            decision: sig.decision,
            signedAt: sig.signedAt,
            signerName: sig.signerName,
            ...(sig.pdfKey ? { pdfKey: sig.pdfKey } : {}),
            ...(sig.paper ? { paper: { ...sig.paper } } : {}),
          },
        }
      : {}),
    ...(inv.engagement ? { engagement: inv.engagement } : {}),
    ...(inv.withdrawal
      ? { withdrawal: inv.withdrawal }
      : retired && caseRec?.decision
        ? { withdrawal: { at: caseRec.decision.at, by: caseRec.decision.by, kind: "retire_direction" as const } }
        : {}),
    purgedAt: inv.purgedAt || nowIso || new Date().toISOString(),
  };
}

export type PurgePlan = {
  /** Objets à supprimer (jamais une clé d'engagement). */
  deleteKeys: string[];
  /** Invitations sans engagement « oui » : supprimées en entier. */
  deleteInvitationIds: string[];
  /** Invitations avec engagement : réécrites sous forme minimale. */
  archive: FamilyInvitation[];
  keptKeys: string[];
};

export function planPurge(invs: FamilyInvitation[], cases: StudentCase[] = [], nowIso?: string): PurgePlan {
  const caseByIne = new Map(cases.map((c) => [caseId(c.ine), c]));
  const kept = new Set<string>();
  const del = new Set<string>();
  const deleteInvitationIds: string[] = [];
  const archive: FamilyInvitation[] = [];
  for (const inv of invs) {
    const isEngagement = inv.signature?.decision === "oui";
    const mine = new Set(engagementKeys(inv));
    for (const k of mine) kept.add(k);
    for (const d of Object.values(inv.documents || {})) if (d?.key) del.add(d.key);
    if (inv.signature?.signatureKey) del.add(inv.signature.signatureKey);
    if (!isEngagement) {
      if (inv.signature?.paper?.scanKey) del.add(inv.signature.paper.scanKey);
      deleteInvitationIds.push(inv.id);
    } else {
      archive.push(archiveInvitation(inv, caseByIne.get(caseId(inv.ine)), nowIso));
    }
  }
  // Garde-fou : un engagement n'est JAMAIS supprimé, quoi qu'il arrive.
  const deleteKeys = [...del].filter((k) => !kept.has(k));
  return { deleteKeys, deleteInvitationIds, archive, keptKeys: [...kept] };
}
