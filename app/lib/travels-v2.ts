/**
 * Circuit voyages v2 (« engagement des familles ») — fonctions pures, sans effet de bord.
 *
 * Règle n°1 : tout comportement v2 est conditionné à `trip.data.workflowVersion === 2`.
 * Les séjours sans ce champ (v1) et les séries récurrentes gardent le circuit historique.
 */

import { computeMonthlySchedule } from "@/app/lib/travels-billing-schedule";
import type {
  TravelsFamilyDocKind,
  TravelsFamilyDocRequest,
  TravelsFamilyPage,
  TravelsPriceBlock,
  TravelsTripData,
} from "@/app/lib/travels-types";

/* ─── Détection de version ─── */

export function isWorkflowV2(trip: { data?: { workflowVersion?: unknown } | null } | null | undefined): boolean {
  return trip?.data?.workflowVersion === 2;
}

/**
 * Un nouveau séjour est créé en v2 si le drapeau tenant est actif ET que ce n'est pas une série
 * récurrente (les séries restent toujours en v1).
 */
export function shouldCreateTripAsV2(opts: { flagEnabled?: boolean | null; isRecurrenceSeries?: boolean }): boolean {
  return opts.flagEnabled === true && !opts.isRecurrenceSeries;
}

/* ─── Dates (YYYY-MM-DD, calcul en UTC pour éviter les décalages d'heure d'été) ─── */

const ISO_DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

export function isIsoDate(s: unknown): s is string {
  if (typeof s !== "string" || !ISO_DATE_RE.test(s)) return false;
  const d = new Date(`${s}T00:00:00Z`);
  return !Number.isNaN(d.getTime()) && d.toISOString().slice(0, 10) === s;
}

function isoToUtcMs(iso: string): number {
  return new Date(`${iso}T00:00:00Z`).getTime();
}

export function addDaysIso(iso: string, days: number): string {
  return new Date(isoToUtcMs(iso) + days * 86_400_000).toISOString().slice(0, 10);
}

/** Nombre de jours de `a` à `b` (positif si b est après a). */
export function diffDaysIso(a: string, b: string): number {
  return Math.round((isoToUtcMs(b) - isoToUtcMs(a)) / 86_400_000);
}

/** Date du jour au fuseau de Paris, au format YYYY-MM-DD. */
export function todayIsoParis(now: Date = new Date()): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(now);
  return isIsoDate(parts) ? parts : now.toISOString().slice(0, 10);
}

/** Date de départ du séjour (séjour complexe : startDate ; sortie simple : date). */
export function tripDepartureIso(data: Pick<TravelsTripData, "startDate" | "date"> | null | undefined): string | null {
  const s = data?.startDate || data?.date;
  return isIsoDate(s) ? s : null;
}

/** Date de retour du séjour (par défaut la date de départ). */
export function tripReturnIso(
  data: Pick<TravelsTripData, "startDate" | "endDate" | "date"> | null | undefined,
): string | null {
  const start = tripDepartureIso(data);
  const end = data?.endDate;
  if (isIsoDate(end) && start && end >= start) return end;
  return start;
}

/** Liste des jours réels du séjour (plafonnée pour éviter les saisies aberrantes). */
export function listTripDays(startIso: string | null | undefined, endIso: string | null | undefined, max = 31): string[] {
  if (!isIsoDate(startIso)) return [];
  const end = isIsoDate(endIso) && endIso >= startIso ? endIso : startIso;
  const out: string[] = [];
  let cur: string = startIso;
  while (cur <= end && out.length < max) {
    out.push(cur);
    cur = addDaysIso(cur, 1);
  }
  return out;
}

const WEEKDAY_SHORT_FR = ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."];

export function dayShortLabelFr(iso: string): string {
  if (!isIsoDate(iso)) return iso;
  const d = new Date(`${iso}T00:00:00Z`);
  return `${WEEKDAY_SHORT_FR[d.getUTCDay()]} ${iso.slice(8, 10)}/${iso.slice(5, 7)}`;
}

/* ─── Pique-nique sur les vraies dates ─── */

const WEEKDAY_KEYS = ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"] as const;

/**
 * v2 : la sélection des jours de pique-nique est stockée sous les dates ISO (daysSelection / orders).
 * Pour que les écrans existants (structure lundi…vendredi) continuent d'afficher quelque chose,
 * on recopie les dates tombant du lundi au vendredi, sans écraser un jour déjà pris (collision).
 */
export function mirrorPiqueNiqueDatesToWeekdays(
  details: Record<string, unknown>,
  /** Jours réels du séjour : les dates hors séjour (anciennes sélections) sont retirées. */
  allowedDays?: string[],
): Record<string, unknown> {
  const sel = { ...((details.daysSelection as Record<string, boolean>) || {}) };
  const orders = { ...((details.orders as Record<string, unknown>) || {}) };
  if (allowedDays) {
    for (const k of Object.keys(sel)) if (isIsoDate(k) && !allowedDays.includes(k)) delete sel[k];
    for (const k of Object.keys(orders)) if (isIsoDate(k) && !allowedDays.includes(k)) delete orders[k];
  }
  const used = new Set<string>();
  const isoKeys = Object.keys(sel)
    .filter((k) => isIsoDate(k))
    .sort();
  for (const iso of isoKeys) {
    if (!sel[iso]) continue;
    const wd = WEEKDAY_KEYS[new Date(`${iso}T00:00:00Z`).getUTCDay()];
    if (wd === "samedi" || wd === "dimanche" || used.has(wd)) continue;
    used.add(wd);
    sel[wd] = true;
    if (orders[iso] !== undefined) orders[wd] = orders[iso];
  }
  return { ...details, daysSelection: sel, orders, datesMode: true };
}

/* ─── Documents de santé interdits ─── */

export const HEALTH_DOC_FORBIDDEN_MESSAGE =
  "Vous ne pouvez pas demander de document lié à la santé (fiche sanitaire, certificat médical, ordonnance…). " +
  "La fiche sanitaire est déjà remplie à la réinscription via Charlemagne et École Directe.";

/** Minuscules, sans accents, ponctuation → espaces. */
export function normalizeForHealthCheck(input: string): string {
  return String(input || "")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[’'`´]/g, " ")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

const HEALTH_PATTERNS: RegExp[] = [
  /\bsanitaire/,
  /\bsante\b/,
  /\bmedic/, // médical, médicament, médicale…
  /\bmedecin/,
  /\bordonnance/,
  /\bvaccin/,
  /\bpai\b/,
  /\bp a i\b/,
  /\bprojet d accueil individualise/,
  /\ballerg/,
  /\btraitement/,
  /\bmaladie/,
  /\bpathologi/,
  /\binfirm/,
  /\bhandicap/,
  /\bcontre indication/,
  /\bcarnet de sante/,
  /\bdiagnostic/,
  /\bmdph\b/,
  /\bpps\b/,
];

/** Vrai si l'intitulé d'une pièce évoque la santé de l'élève (interdit à l'organisateur). */
export function isHealthRelatedLabel(label: string): boolean {
  const n = normalizeForHealthCheck(label);
  if (!n) return false;
  return HEALTH_PATTERNS.some((re) => re.test(n));
}

export type ValidationResult = { ok: true } | { ok: false; error: string };

const DOC_KINDS: readonly TravelsFamilyDocKind[] = ["avant_signature", "facultative", "avant_depart"];

/** Contrôle la liste des pièces demandées (santé interdite, types valides, échéance des pièces « avant départ »). */
export function validateFamilyDocuments(
  docs: unknown,
  opts: { departureIso?: string | null } = {},
): ValidationResult {
  if (docs === undefined || docs === null) return { ok: true };
  if (!Array.isArray(docs)) return { ok: false, error: "Liste de pièces invalide." };
  if (docs.length > 30) return { ok: false, error: "Trop de pièces demandées (30 maximum)." };
  const seen = new Set<string>();
  for (const raw of docs as Partial<TravelsFamilyDocRequest>[]) {
    const label = String(raw?.label ?? "").trim();
    if (!label) return { ok: false, error: "Chaque pièce demandée doit avoir un intitulé." };
    if (label.length > 160) return { ok: false, error: "Intitulé de pièce trop long (160 caractères maximum)." };
    if (isHealthRelatedLabel(label)) return { ok: false, error: HEALTH_DOC_FORBIDDEN_MESSAGE };
    const tplName = String(raw?.template?.name ?? "").trim();
    if (tplName && isHealthRelatedLabel(tplName)) return { ok: false, error: HEALTH_DOC_FORBIDDEN_MESSAGE };
    if (!raw?.kind || !DOC_KINDS.includes(raw.kind)) {
      return { ok: false, error: `Type de pièce invalide pour « ${label} ».` };
    }
    if (raw.kind === "avant_depart") {
      if (!isIsoDate(raw.dueDate)) {
        return { ok: false, error: `Indiquez la date limite de dépôt de la pièce « ${label} » (à fournir avant le départ).` };
      }
      if (opts.departureIso && raw.dueDate > opts.departureIso) {
        return { ok: false, error: `La pièce « ${label} » doit être fournie au plus tard le jour du départ.` };
      }
    }
    const id = String(raw?.id ?? "");
    if (id) {
      if (seen.has(id)) return { ok: false, error: "Deux pièces ont le même identifiant." };
      seen.add(id);
    }
  }
  return { ok: true };
}

/* ─── Rétroplanning depuis la date de départ ─── */

/** Délai cuisine et devis rectifié : clôture des inscriptions au plus tard J-14. */
export const REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE = 14;
/** Les inscriptions restent ouvertes au moins 7 jours. */
export const REGISTRATION_MIN_OPEN_DAYS = 7;
/** Validation finale au plus tard J-21 (= J-14 + 7 jours d'ouverture). */
export const FINAL_VALIDATION_DAYS_BEFORE_DEPARTURE =
  REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE + REGISTRATION_MIN_OPEN_DAYS;

export type Retroplanning = {
  departureIso: string;
  /** Date limite de réponse des familles au plus tard (J-14). */
  latestDeadlineIso: string;
  /** Validation finale (ouverture des inscriptions) au plus tard (J-21). */
  latestFinalValidationIso: string;
  daysUntilDeparture: number;
  /** Le départ est trop proche pour respecter le rétroplanning (J-21 déjà dépassé). */
  tooClose: boolean;
  /** Jours disponibles pour les inscriptions si on ouvrait aujourd'hui (peut être < 7). */
  openDaysIfOpenedToday: number;
  warning: string | null;
};

export function computeRetroplanning(departureIso: string | null | undefined, todayIso: string): Retroplanning | null {
  if (!isIsoDate(departureIso) || !isIsoDate(todayIso)) return null;
  const latestDeadlineIso = addDaysIso(departureIso, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE);
  const latestFinalValidationIso = addDaysIso(departureIso, -FINAL_VALIDATION_DAYS_BEFORE_DEPARTURE);
  const daysUntilDeparture = diffDaysIso(todayIso, departureIso);
  const openDaysIfOpenedToday = diffDaysIso(todayIso, latestDeadlineIso);
  const tooClose = openDaysIfOpenedToday < REGISTRATION_MIN_OPEN_DAYS;
  let warning: string | null = null;
  if (daysUntilDeparture < 0) {
    warning = "La date de départ est dépassée.";
  } else if (tooClose) {
    warning =
      `Départ trop proche : il reste ${daysUntilDeparture} jour${daysUntilDeparture > 1 ? "s" : ""} avant le départ. ` +
      `Les inscriptions doivent se clôturer au plus tard à J-${REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE} ` +
      `et rester ouvertes au moins ${REGISTRATION_MIN_OPEN_DAYS} jours (validation finale au plus tard à J-${FINAL_VALIDATION_DAYS_BEFORE_DEPARTURE}, ` +
      `soit le ${latestFinalValidationIso}). Le circuit ne pourra pas être respecté tel quel : prévenez la direction.`;
  }
  return {
    departureIso,
    latestDeadlineIso,
    latestFinalValidationIso,
    daysUntilDeparture,
    tooClose,
    openDaysIfOpenedToday,
    warning,
  };
}

/* ─── Garde-fous de la validation finale (date limite) ─── */

export function validateDeadlineForOpening(opts: {
  deadline: string | null | undefined;
  departureIso: string | null | undefined;
  todayIso: string;
}): ValidationResult {
  const { deadline, departureIso, todayIso } = opts;
  if (!deadline || !isIsoDate(deadline)) {
    return { ok: false, error: "Renseignez la date limite de réponse des familles avant d'ouvrir les inscriptions." };
  }
  const minDeadline = addDaysIso(todayIso, REGISTRATION_MIN_OPEN_DAYS);
  if (deadline < minDeadline) {
    return {
      ok: false,
      error:
        `La date limite (${deadline}) laisse moins de ${REGISTRATION_MIN_OPEN_DAYS} jours aux familles : ` +
        `repoussez la date limite au ${minDeadline} ou plus tard.`,
    };
  }
  if (isIsoDate(departureIso)) {
    const maxDeadline = addDaysIso(departureIso, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE);
    if (deadline > maxDeadline) {
      return {
        ok: false,
        error:
          `La date limite (${deadline}) est trop proche du départ : la clôture des inscriptions doit avoir lieu ` +
          `au plus tard ${REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE} jours avant (le ${maxDeadline}). Avancez la date limite.` +
          (maxDeadline < minDeadline
            ? ` Le départ est trop proche pour laisser ${REGISTRATION_MIN_OPEN_DAYS} jours aux familles : demandez une dérogation à la direction.`
            : ""),
      };
    }
  }
  return { ok: true };
}

/* ─── Bloc prix (compta) ─── */

export function validatePriceBlock(pb: Partial<TravelsPriceBlock> | null | undefined): ValidationResult {
  if (!pb || typeof pb !== "object") {
    return { ok: false, error: "Le bloc prix de la page des familles doit être renseigné par la comptabilité." };
  }
  const price = Number(pb.pricePerStudent);
  if (pb.pricePerStudent === undefined || pb.pricePerStudent === null || (pb.pricePerStudent as unknown) === "" || !Number.isFinite(price) || price < 0) {
    return { ok: false, error: "Bloc prix : indiquez le prix par élève (0 € accepté)." };
  }
  // Prix > 0 : lissage mensuel sur les factures (premier et dernier mois obligatoires, période cohérente).
  const schedule = computeMonthlySchedule(price, pb.firstMonth, pb.lastMonth);
  if (!schedule.ok) return { ok: false, error: "Bloc prix : " + schedule.error };
  return { ok: true };
}

/* ─── Transport après la pédagogie ─── */

/** Vrai si la validation pédagogique doit déclencher (une seule fois) l'envoi des demandes de devis. */
export function shouldSendTransportOnPedagogyValidation(opts: {
  isV2: boolean;
  previousStatus: string | null;
  newStatus: string;
  tripType?: string;
  needsBus?: boolean;
  transportRequestSentAt?: string | null;
}): boolean {
  return (
    opts.isV2 &&
    opts.previousStatus === "EN_ATTENTE_DIR_INITIAL" &&
    opts.newStatus === "PROF_LOGISTICS" &&
    opts.tripType === "COMPLEX" &&
    opts.needsBus === true &&
    !opts.transportRequestSentAt
  );
}

/** Statut suivant la validation pédagogique (v2) : bus → logistique, sinon directement finances. */
export function statusAfterPedagogyV2(opts: { tripType?: string; needsBus?: boolean }): "PROF_LOGISTICS" | "EN_ATTENTE_COMPTA" {
  return opts.tripType === "COMPLEX" && opts.needsBus ? "PROF_LOGISTICS" : "EN_ATTENTE_COMPTA";
}

/* ─── Page des familles : accès et verrouillage ─── */

/** La rédaction de la page est ouverte dès la validation pédagogique (en parallèle du transport). */
export function isFamilyPageEditableStatus(status: string | null | undefined): boolean {
  return (
    status === "PROF_LOGISTICS" ||
    status === "EN_ATTENTE_BUS_SIGNATURE" ||
    status === "EN_ATTENTE_COMPTA" ||
    status === "EN_ATTENTE_DIR_FINAL"
  );
}

export function isFamilyPageOpened(page: Pick<TravelsFamilyPage, "openedAt"> | null | undefined): boolean {
  return Boolean(page?.openedAt);
}

/* ─── Champs structurés de la page (trame fixe) ─── */

export const FAMILY_PROGRAM_MAX_CHARS = 600;

export type MeetingPoint = { place: string; date: string; time: string };

/** Rendez-vous aller / retour = points « depot » et « recuperation » du calendrier parents existant. */
export function getFamilyMeetingPoints(
  data: Pick<TravelsTripData, "parentCalendar"> | null | undefined,
): { outbound: MeetingPoint | null; inbound: MeetingPoint | null } {
  const points = Array.isArray(data?.parentCalendar?.points) ? data!.parentCalendar!.points : [];
  const toPoint = (kind: "depot" | "recuperation"): MeetingPoint | null => {
    const p = points.find((x) => x?.kind === kind);
    return p ? { place: String(p.place || "").trim(), date: String(p.date || ""), time: String(p.time || "") } : null;
  };
  return { outbound: toPoint("depot"), inbound: toPoint("recuperation") };
}

/** Empreinte des champs structurés (équipement, liquide, programme, rendez-vous) pour détecter une modification. */
export function familyStructuredSnapshot(data: Partial<TravelsTripData> | null | undefined): string {
  const page = data?.familyPage;
  const { outbound, inbound } = getFamilyMeetingPoints(data);
  return JSON.stringify({
    e: (page?.equipment || []).map((x) => String(x).trim()),
    c: page?.cash ? [Number(page.cash.amount), String(page.cash.reason || "").trim()] : null,
    p: String(page?.program ?? "").trim(),
    o: outbound,
    i: inbound,
  });
}

/**
 * Somme en liquide FACULTATIVE : case décochée = aucune ligne (`cash` retiré, rien sur la page) ;
 * case cochée = montant et motif obligatoires (la validation `validateFamilyStructured` refuse une ligne vide).
 */
export function resolveCashDraft(opts: {
  enabled: boolean;
  amountText: string;
  reason: string;
}): TravelsFamilyPage["cash"] | undefined {
  if (!opts.enabled) return undefined;
  const amount = Number(String(opts.amountText ?? "").replace(",", "."));
  return { amount: Number.isFinite(amount) ? amount : 0, reason: String(opts.reason ?? "").trim() };
}

export function validateFamilyStructured(page: Partial<TravelsFamilyPage> | null | undefined): ValidationResult {
  if (!page) return { ok: true };
  if (page.equipment !== undefined) {
    if (!Array.isArray(page.equipment) || page.equipment.length > 40) {
      return { ok: false, error: "Liste d'équipement invalide (40 éléments maximum)." };
    }
    if (page.equipment.some((x) => typeof x !== "string" || !x.trim() || x.length > 80)) {
      return { ok: false, error: "Chaque élément d'équipement doit être renseigné (80 caractères maximum)." };
    }
  }
  if (page.cash !== undefined && page.cash !== null) {
    const amount = Number(page.cash.amount);
    if (!Number.isFinite(amount) || amount <= 0) return { ok: false, error: "Liquide à prévoir : indiquez un montant positif ou retirez la ligne." };
    if (!String(page.cash.reason ?? "").trim()) return { ok: false, error: "Liquide à prévoir : indiquez le motif (ex. sandwich sur la route du retour)." };
  }
  if (page.program !== undefined && String(page.program).length > FAMILY_PROGRAM_MAX_CHARS) {
    return { ok: false, error: "Programme trop long (" + FAMILY_PROGRAM_MAX_CHARS + " caractères maximum)." };
  }
  return { ok: true };
}

/** Rendez-vous aller et retour obligatoires (lieu + heure) pour ouvrir les inscriptions. */
export function validateFamilyMeetings(data: Pick<TravelsTripData, "parentCalendar"> | null | undefined): ValidationResult {
  const { outbound, inbound } = getFamilyMeetingPoints(data);
  if (!outbound || !outbound.place || !outbound.time) {
    return { ok: false, error: "Rendez-vous aller : indiquez le lieu et l'heure de dépôt des élèves." };
  }
  if (!inbound || !inbound.place || !inbound.time) {
    return { ok: false, error: "Rendez-vous retour : indiquez le lieu et l'heure de récupération des élèves." };
  }
  return { ok: true };
}

/* ─── Contrôles serveur de /api/travels/update (v2) ─── */

export type V2Actor = {
  isCompta: boolean;
  /** Direction de l'établissement du séjour (canSignTravelsDirectionForEtab). */
  isDirectionOfEtab: boolean;
  isGlobalAdmin: boolean;
};

export type PriceBlockChange = "ok" | "restore" | "forbidden";

function canonicalPriceBlock(pb: Partial<TravelsPriceBlock> | null | undefined): string {
  if (!pb) return "";
  return JSON.stringify({
    p: Number(pb.pricePerStudent),
    f: pb.firstMonth ?? "",
    l: pb.lastMonth ?? "",
    t: String(pb.paymentText ?? "").trim(),
  });
}

/**
 * Seule la compta modifie le bloc prix.
 * - contenu identique → ok ;
 * - non-compta qui renvoie un dossier sans bloc prix (copie périmée) → "restore" (on garde celui du serveur) ;
 * - non-compta qui change ou supprime un bloc existant, ou en crée un → "forbidden".
 */
export function evaluatePriceBlockChange(opts: {
  existing: Partial<TravelsPriceBlock> | null | undefined;
  incoming: Partial<TravelsPriceBlock> | null | undefined;
  isCompta: boolean;
}): PriceBlockChange {
  const { existing, incoming, isCompta } = opts;
  if (isCompta) return "ok";
  if (canonicalPriceBlock(existing) === canonicalPriceBlock(incoming)) return "ok";
  if (existing && !incoming) return "restore";
  return "forbidden";
}

export function canOpenRegistrations(actor: Pick<V2Actor, "isDirectionOfEtab" | "isGlobalAdmin">): boolean {
  return actor.isDirectionOfEtab || actor.isGlobalAdmin;
}

/**
 * Champs structurés de la page : verrouillés après ouverture ; jamais modifiables par la compta seule
 * (sa copie est alors silencieusement remplacée par celle du serveur : "restore").
 */
export function evaluateFamilyStructuredChange(opts: {
  existing: Partial<TravelsTripData> | null | undefined;
  incoming: Partial<TravelsTripData> | null | undefined;
  actor: V2Actor;
}): "ok" | "restore" | "forbidden" {
  const { existing, incoming, actor } = opts;
  if (familyStructuredSnapshot(existing) === familyStructuredSnapshot(incoming)) return "ok";
  if (actor.isCompta && !actor.isDirectionOfEtab && !actor.isGlobalAdmin) return "restore";
  if (existing?.familyPage?.openedAt) return "forbidden";
  return "ok";
}

export type V2UpdateCheck =
  | { ok: true; restorePriceBlock: boolean; restoreFamilyStructured: boolean; stampOpenedAt: boolean }
  | { ok: false; status: number; error: string };

/**
 * Contrôles serveur v2 appliqués à `/api/travels/update`.
 * `existingData` = `data` du dossier déjà stocké (null à la création) ; `incomingData` = `data` reçu du client.
 * Ne s'applique que si le dossier est (ou devient, à la création) un dossier v2.
 */
export function checkWorkflowV2Update(opts: {
  existingData: Record<string, unknown> | null;
  incomingData: Record<string, unknown>;
  previousStatus: string | null;
  newStatus: string;
  actor: V2Actor;
  todayIso: string;
}): V2UpdateCheck {
  const { existingData, incomingData, previousStatus, newStatus, actor, todayIso } = opts;
  // La version ne change jamais après la création.
  if (existingData && existingData.workflowVersion !== 2 && incomingData.workflowVersion === 2) {
    return { ok: false, status: 400, error: "Un dossier existant ne peut pas passer au nouveau circuit." };
  }
  const isV2 = (existingData ?? incomingData)?.workflowVersion === 2;
  if (!isV2) return { ok: true, restorePriceBlock: false, restoreFamilyStructured: false, stampOpenedAt: false };
  if (existingData && incomingData.workflowVersion !== 2) {
    return { ok: false, status: 400, error: "La version du circuit ne peut pas être modifiée." };
  }

  const incomingPage = (incomingData.familyPage as Partial<TravelsFamilyPage> | undefined) ?? undefined;
  const existingPage = (existingData?.familyPage as Partial<TravelsFamilyPage> | undefined) ?? undefined;
  const departureIso = tripDepartureIso(incomingData as Pick<TravelsTripData, "startDate" | "date">);

  // Pièces demandées : jamais de document de santé.
  if (incomingPage?.documents !== undefined) {
    const docs = validateFamilyDocuments(incomingPage.documents, { departureIso });
    if (!docs.ok) return { ok: false, status: 400, error: docs.error };
  }

  // Bloc prix : compta uniquement.
  const priceChange = evaluatePriceBlockChange({
    existing: existingData?.priceBlock as Partial<TravelsPriceBlock> | undefined,
    incoming: incomingData.priceBlock as Partial<TravelsPriceBlock> | undefined,
    isCompta: actor.isCompta,
  });
  if (priceChange === "forbidden") {
    return { ok: false, status: 403, error: "Seule la comptabilité peut saisir ou modifier le bloc prix de la page des familles." };
  }

  // Champs structurés : verrouillés après ouverture, non modifiables par la compta.
  const structuredChange = evaluateFamilyStructuredChange({
    existing: existingData as Partial<TravelsTripData> | null,
    incoming: incomingData as Partial<TravelsTripData>,
    actor,
  });
  if (structuredChange === "forbidden") {
    return { ok: false, status: 403, error: "La page des familles est ouverte : ses informations ne peuvent plus être modifiées." };
  }
  if (structuredChange !== "restore") {
    const structured = validateFamilyStructured(incomingPage);
    if (!structured.ok) return { ok: false, status: 400, error: structured.error };
  }

  // Validation finances : le bloc prix doit être rempli.
  if (previousStatus === "EN_ATTENTE_COMPTA" && newStatus === "EN_ATTENTE_DIR_FINAL") {
    const pb = priceChange === "restore" ? (existingData?.priceBlock as Partial<TravelsPriceBlock>) : (incomingData.priceBlock as Partial<TravelsPriceBlock> | undefined);
    const res = validatePriceBlock(pb);
    if (!res.ok) return { ok: false, status: 400, error: res.error };
  }

  // Ouverture des inscriptions : direction de l'établissement ou admin général, avec garde-fous.
  let stampOpenedAt = false;
  if (newStatus === "INSCRIPTIONS_OUVERTES" && previousStatus !== "INSCRIPTIONS_OUVERTES") {
    if (!canOpenRegistrations(actor)) {
      return { ok: false, status: 403, error: "Seule la direction de l'établissement concerné (ou un administrateur général) peut ouvrir les inscriptions des familles." };
    }
    if (previousStatus !== "EN_ATTENTE_DIR_FINAL") {
      return { ok: false, status: 400, error: "Les inscriptions ne peuvent être ouvertes qu'à l'issue de la validation finale." };
    }
    const meetings = validateFamilyMeetings(structuredChange === "restore" ? (existingData as Partial<TravelsTripData>) : (incomingData as Partial<TravelsTripData>));
    if (!meetings.ok) return { ok: false, status: 400, error: meetings.error };
    const pb = priceChange === "restore" ? (existingData?.priceBlock as Partial<TravelsPriceBlock>) : (incomingData.priceBlock as Partial<TravelsPriceBlock> | undefined);
    const priceRes = validatePriceBlock(pb);
    if (!priceRes.ok) return { ok: false, status: 400, error: priceRes.error };
    const deadlineRes = validateDeadlineForOpening({ deadline: incomingPage?.deadline, departureIso, todayIso });
    if (!deadlineRes.ok) return { ok: false, status: 400, error: deadlineRes.error };
    stampOpenedAt = true;
  }

  // On ne peut pas poser `openedAt` à la main sans passer par l'ouverture.
  if (incomingPage?.openedAt && !existingPage?.openedAt && newStatus !== "INSCRIPTIONS_OUVERTES") {
    return { ok: false, status: 403, error: "L'ouverture de la page passe uniquement par la validation finale." };
  }

  return { ok: true, restorePriceBlock: priceChange === "restore", restoreFamilyStructured: structuredChange === "restore", stampOpenedAt };
}

/* ─── Aperçu direction : points de contrôle ─── */

export function familyPageReadiness(data: TravelsTripData | null | undefined, todayIso: string): { ready: boolean; problems: string[] } {
  const problems: string[] = [];
  const page = data?.familyPage;
  const meetings = validateFamilyMeetings(data);
  if (!meetings.ok) problems.push(meetings.error);
  const pb = validatePriceBlock(data?.priceBlock);
  if (!pb.ok) problems.push(pb.error);
  const dl = validateDeadlineForOpening({ deadline: page?.deadline, departureIso: tripDepartureIso(data), todayIso });
  if (!dl.ok) problems.push(dl.error);
  const docs = validateFamilyDocuments(page?.documents, { departureIso: tripDepartureIso(data) });
  if (!docs.ok) problems.push(docs.error);
  return { ready: problems.length === 0, problems };
}
