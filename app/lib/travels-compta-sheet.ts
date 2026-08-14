import type { TravelsTrip } from "@/app/lib/travels-types";

export type TravelsComptaExpenseLine = {
  label: string;
  amount: number | null;
  source?: string | null;
};

export type TravelsComptaRecetteLine = {
  label: string;
  amount: number | null;
};

export type TravelsComptaIndividualAid = {
  name: string;
  amount: number | null;
};

export type TravelsComptaFacturation = {
  label?: string;
  prixFacture: number | null;
  dateFacturation: string;
  montant: number | null;
};

export function emptyComptaFacturation(): TravelsComptaFacturation {
  return { label: "", prixFacture: null, dateFacturation: "", montant: null };
}

/** Repères indicatifs — convertis en montant € sur le total dépenses (suggestions uniquement). */
export const COMPTA_MARGE_PRESETS = [
  { id: "sans_risque", label: "Sorties sans risques", percent: 0 },
  { id: "pedagogique", label: "Sorties pédagogiques", percent: 5 },
  { id: "france", label: "Séjours et voyages en France", percent: 7 },
  { id: "etranger", label: "Voyages à l'étranger", percent: 10 },
] as const;

export type ComptaMargePresetId = (typeof COMPTA_MARGE_PRESETS)[number]["id"];

/** @deprecated Utiliser COMPTA_MARGE_PRESETS */
export const COMPTA_RISK_FACTOR_OPTIONS = COMPTA_MARGE_PRESETS;
/** @deprecated */
export type ComptaRiskFactorId = ComptaMargePresetId;

export function comptaMargePresetPercent(id: ComptaMargePresetId | null | undefined): number {
  return COMPTA_MARGE_PRESETS.find((o) => o.id === id)?.percent ?? 0;
}

/** @deprecated */
export const comptaRiskFactorPercent = comptaMargePresetPercent;

export function comptaMargePresetLabel(id: ComptaMargePresetId | null | undefined): string {
  return COMPTA_MARGE_PRESETS.find((o) => o.id === id)?.label ?? "—";
}

/** @deprecated */
export const comptaRiskFactorLabel = comptaMargePresetLabel;

export function suggestComptaMargeFromPreset(
  depensesTotal: number | null | undefined,
  percent: number,
): number | null {
  if (depensesTotal == null || depensesTotal <= 0 || percent <= 0) return percent === 0 ? 0 : null;
  return Math.round(depensesTotal * (percent / 100) * 100) / 100;
}

function normalizeDestinationText(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

const FOREIGN_DESTINATION_KEYWORDS = [
  "italie",
  "espagne",
  "allemagne",
  "belgique",
  "suisse",
  "luxembourg",
  "royaume-uni",
  "angleterre",
  "ecosse",
  "irlande",
  "pays-bas",
  "hollande",
  "portugal",
  "grece",
  "pologne",
  "autriche",
  "republique tcheque",
  "hongrie",
  "maroc",
  "tunisie",
  "algerie",
  "usa",
  "etats-unis",
  "canada",
  "japon",
  "chine",
  "etranger",
  "international",
];

function tripIsMultiDay(data: TravelsTrip["data"]): boolean {
  const start = String(data.startDate || data.date || "").trim();
  const end = String(data.endDate || "").trim();
  if (start && end && start !== end) return true;
  if (data.endDate && !data.startDate && data.date && data.endDate !== data.date) return true;
  return false;
}

function destinationSuggestsAbroad(destination: string): boolean {
  const t = normalizeDestinationText(destination);
  if (!t) return false;
  if (/\bfrance\b/.test(t) && !FOREIGN_DESTINATION_KEYWORDS.some((k) => t.includes(k))) return false;
  return FOREIGN_DESTINATION_KEYWORDS.some((k) => t.includes(k));
}

/** Proposition automatique selon le type de sortie / destination du dossier. */
export function inferComptaMargePreset(trip: Pick<TravelsTrip, "type" | "data">): ComptaMargePresetId {
  const destination = String(trip.data?.destination || trip.data?.title || "").trim();
  if (destinationSuggestsAbroad(destination)) return "etranger";
  if (trip.type === "COMPLEX" || tripIsMultiDay(trip.data)) return "france";
  return "pedagogique";
}

/** @deprecated */
export const inferComptaRiskFactor = inferComptaMargePreset;

export function inferComptaMargeSecuriteEuro(
  trip: Pick<TravelsTrip, "type" | "data">,
  depensesTotal: number | null | undefined,
): number | null {
  const preset = inferComptaMargePreset(trip);
  return suggestComptaMargeFromPreset(depensesTotal, comptaMargePresetPercent(preset));
}

function resolveMargeSecuriteEuro(
  sheet: Pick<TravelsComptaSheet, "margeSecuriteEuro" | "facteurRisque">,
  depensesTotal: number | null,
  trip?: Pick<TravelsTrip, "type" | "data"> | null,
): number {
  if (sheet.margeSecuriteEuro != null) return sheet.margeSecuriteEuro;
  if (sheet.facteurRisque && depensesTotal != null) {
    return suggestComptaMargeFromPreset(depensesTotal, comptaMargePresetPercent(sheet.facteurRisque)) ?? 0;
  }
  if (trip && depensesTotal != null) {
    return inferComptaMargeSecuriteEuro(trip, depensesTotal) ?? 0;
  }
  return 0;
}

/** Rétrocompat. : ancienne fiche avec `facturation` unique ou lignes multiples → une seule ligne. */
export function resolveFacturationsFromSheet(
  sheet: Partial<Pick<TravelsComptaSheet, "facturations" | "facturation">>,
): TravelsComptaFacturation[] {
  let row = emptyComptaFacturation();
  if (Array.isArray(sheet.facturations) && sheet.facturations.length > 0) {
    row = { ...row, ...sheet.facturations[0] };
  } else if (sheet.facturation) {
    row = { ...row, ...sheet.facturation };
  }
  return [row];
}

export function autresDepensesHorsBus(
  depensesTotal: number | null | undefined,
  busAmount: number | null | undefined,
): number | null {
  if (depensesTotal == null) return null;
  if (busAmount == null || busAmount <= 0) return depensesTotal;
  return Math.max(0, Math.round((depensesTotal - busAmount) * 100) / 100);
}

export function sumFacturationMontants(facturations: TravelsComptaFacturation[]): number | null {
  let has = false;
  let sum = 0;
  for (const f of facturations) {
    if (f.montant != null) {
      has = true;
      sum += f.montant;
    }
  }
  return has ? Math.round(sum * 100) / 100 : null;
}

/** Une seule ligne facturation : total dépenses + marge, date et prix bus. */
export function buildComptaFacturation(
  sheet: Partial<Pick<TravelsComptaSheet, "facturations" | "facturation">>,
  busAmount: number | null,
  montantEleves: number | null,
): TravelsComptaFacturation[] {
  const prev = resolveFacturationsFromSheet(sheet)[0] ?? emptyComptaFacturation();
  return [
    {
      label: "Recettes élèves",
      prixFacture: busAmount ?? prev.prixFacture,
      dateFacturation: prev.dateFacturation ?? "",
      montant: montantEleves,
    },
  ];
}

export type TravelsComptaSheet = {
  compte: string;
  ligne: string;
  classe: string;
  profs: string;
  accompagnateurs: string;
  depenses: TravelsComptaExpenseLine[];
  nbEleves: number | null;
  prixParEleve: number | null;
  /** Lignes de recettes complémentaires (subventions, APEL, etc.). */
  recettesLignes: TravelsComptaRecetteLine[];
  /** @deprecated Migré vers `recettesLignes`. */
  apelAidesCollectives?: number | null;
  /** @deprecated Migré vers `recettesLignes`. */
  autresSubventions?: number | null;
  aidesIndividuelles: TravelsComptaIndividualAid[];
  facturations: TravelsComptaFacturation[];
  /** @deprecated Premier élément de `facturations` — conservé pour rétrocompat. */
  facturation?: TravelsComptaFacturation;
  /** Marge de sécurité (imprévus / annulations) en euros, appliquée au total dépenses. */
  margeSecuriteEuro: number | null;
  /** Marge figée au moment de l'annonce aux parents (null = pas de marge prévue à l'annonce). */
  margeFigeeEuro: number | null;
  /** Prix par élève déjà communiqué aux familles (ne peut plus être augmenté). */
  prixParEleveAnnonce: number | null;
  /** Nombre d'élèves facturés / ayant réglé (peut différer de l'effectif réel). */
  nbElevesFactures: number | null;
  /** Si vrai, les recettes élèves sont figées sur le prix annoncé (déficit possible). */
  recettesElevesFigees: boolean;
  /** @deprecated Ancien facteur % — migré vers margeSecuriteEuro */
  facteurRisque?: ComptaMargePresetId | null;
  /** Champs calculés (recalculés côté client et serveur). */
  depensesTotal: number | null;
  /** Objectif de facturation = total dépenses + marge. */
  montantCibleFacturation: number | null;
  recettesEleves: number | null;
  totalSubventions: number | null;
  totalAidesIndividuelles: number | null;
  totalRecettes: number | null;
  depensesAvecMargeRisque: number | null;
  margeRisqueMontant: number | null;
  prixFactureBus: number | null;
  autresDepensesHorsBus: number | null;
  prixParEleveAvantMargeRisque: number | null;
  prixParEleveAvecSubventions: number | null;
  /** Coût prévisionnel par élève (budget prévisionnel ÷ nb), arrondi à l'entier supérieur. */
  coutPrevisionnelParEleve: number | null;
  excedentOuDeficit: number | null;
  analyzedAt?: string | null;
  analysisNotes?: string | null;
  /** Empreinte des documents du dossier (URLs + libellés) pour sync auto. */
  syncedDocumentsFingerprint?: string | null;
  syncedAt?: string | null;
  /** Cache OCR par document — évite de relire les fichiers inchangés. */
  documentScans?: TravelsComptaDocumentScan[];
  /** Date de validation du budget par la comptabilité. */
  budgetValidatedAt?: string | null;
  /** Dernier journal OCR (affiché sur la fiche compta). */
  ocrDebugLog?: ComptaOcrLogEntry[] | null;
};

export function comptaRecettesFigees(
  sheet: Pick<TravelsComptaSheet, "recettesElevesFigees" | "prixParEleveAnnonce">,
): boolean {
  return Boolean(sheet.recettesElevesFigees || sheet.prixParEleveAnnonce != null);
}

/** Afficher la marge : toujours avant annonce ; après annonce seulement si elle avait été prévue. */
export function comptaAfficheMargeSecurite(
  sheet: Pick<TravelsComptaSheet, "recettesElevesFigees" | "prixParEleveAnnonce" | "margeFigeeEuro">,
): boolean {
  if (!comptaRecettesFigees(sheet)) return true;
  return sheet.margeFigeeEuro != null && sheet.margeFigeeEuro > 0;
}

export type TravelsComptaDocumentRef = {
  key: string;
  fileUrl: string;
  label: string;
  role: "devis_signe" | "devis_activite" | "piece_jointe" | "budget_previsionnel";
  s3Key?: string | null;
  fallbackAmount?: number | null;
};

export type TravelsComptaDocumentScan = {
  key: string;
  fileUrl: string;
  label: string;
  role: TravelsComptaDocumentRef["role"];
  amount: number | null;
  scannedAt: string;
};

export type ComptaOcrLogLevel = "info" | "warn" | "error";

export type ComptaOcrLogEntry = {
  at: string;
  level: ComptaOcrLogLevel;
  message: string;
  detail?: string;
};

export function parseEuroAmount(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) return Math.round(v * 100) / 100;
  if (v == null) return null;
  const raw = String(v).trim();
  if (!raw || raw.toLowerCase() === "null") return null;

  let s = raw
    .replace(/\u00a0/g, " ")
    .replace(/\s/g, "")
    .replace(/€/gi, "")
    .trim();
  if (!s) return null;

  // Format européen : 1.350,00 ou 1 350,00
  if (/^-?\d{1,3}(?:\.\d{3})*,\d{1,2}$/.test(s)) {
    s = s.replace(/\./g, "").replace(",", ".");
  } else if (s.includes(",") && !s.includes(".")) {
    s = s.replace(",", ".");
  } else if (s.includes(",") && s.includes(".")) {
    const lastComma = s.lastIndexOf(",");
    const lastDot = s.lastIndexOf(".");
    if (lastComma > lastDot) {
      s = s.replace(/\./g, "").replace(",", ".");
    } else {
      s = s.replace(/,/g, "");
    }
  }

  const n = parseFloat(s.replace(/[^\d.-]/g, ""));
  return Number.isFinite(n) ? Math.round(n * 100) / 100 : null;
}

/** Montant exploitable (0 = non renseigné pour l'auto-remplissage OCR). */
export function isUsableComptaAmount(amount: number | null | undefined): amount is number {
  return amount != null && amount > 0;
}

export function formatEuroDisplay(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return "";
  return new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(n);
}

/** Montant par élève arrondi à l'entier supérieur (tarification familles). */
export function perStudentEuroCeil(total: number | null | undefined, nb: number): number | null {
  if (nb <= 0 || total == null) return null;
  return Math.ceil(Math.max(0, total) / nb);
}

export function perStudentEuroCeilAdjusted(total: number | null | undefined, nb: number): boolean {
  if (nb <= 0 || total == null) return false;
  const raw = Math.max(0, total) / nb;
  return raw !== Math.ceil(raw);
}

export function formatEuroWhole(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return "";
  return new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 0 }).format(n);
}

export const COMPTA_APEL_RECETTE_LABEL = "APEL";

export function isBlankRecetteLine(line: TravelsComptaRecetteLine): boolean {
  return !line.label.trim() && line.amount == null;
}

export function isApelRecetteLineIndex(index: number): boolean {
  return index === 0;
}

export function normalizeComptaRecettesLignes(
  sheet: Pick<TravelsComptaSheet, "recettesLignes" | "apelAidesCollectives" | "autresSubventions">,
): TravelsComptaRecetteLine[] {
  const raw = Array.isArray(sheet.recettesLignes) ? [...sheet.recettesLignes] : [];
  let apelAmount: number | null = null;
  const additional: TravelsComptaRecetteLine[] = [];

  if (raw.length === 0) {
    if (isUsableComptaAmount(sheet.apelAidesCollectives)) {
      apelAmount = sheet.apelAidesCollectives!;
    }
    if (isUsableComptaAmount(sheet.autresSubventions)) {
      additional.push({ label: "Autres subventions", amount: sheet.autresSubventions! });
    }
  } else {
    for (const line of raw) {
      if (/^apel\b/i.test(line.label.trim()) && apelAmount == null) {
        apelAmount = line.amount;
      } else {
        additional.push(line);
      }
    }
  }

  return [{ label: COMPTA_APEL_RECETTE_LABEL, amount: apelAmount }, ...additional];
}

export function withoutBlankRecettesLignes(lines: TravelsComptaRecetteLine[]): TravelsComptaRecetteLine[] {
  const normalized = normalizeComptaRecettesLignes({ recettesLignes: lines });
  const [apel, ...rest] = normalized;
  return [apel, ...rest.filter((line) => !isBlankRecetteLine(line))];
}

/** @deprecated Utiliser normalizeComptaRecettesLignes */
export function resolveComptaRecettesLignes(
  sheet: Pick<TravelsComptaSheet, "recettesLignes" | "apelAidesCollectives" | "autresSubventions">,
): TravelsComptaRecetteLine[] {
  return normalizeComptaRecettesLignes(sheet);
}

export function emptyComptaSheet(): TravelsComptaSheet {
  return {
    compte: "",
    ligne: "",
    classe: "",
    profs: "",
    accompagnateurs: "",
    depenses: [
      { label: "", amount: null },
      { label: "", amount: null },
    ],
    nbEleves: null,
    prixParEleve: null,
    recettesLignes: [{ label: COMPTA_APEL_RECETTE_LABEL, amount: null }],
    aidesIndividuelles: [{ name: "", amount: null }],
    facturations: [emptyComptaFacturation()],
    margeSecuriteEuro: null,
    margeFigeeEuro: null,
    prixParEleveAnnonce: null,
    nbElevesFactures: null,
    recettesElevesFigees: false,
    depensesTotal: null,
    montantCibleFacturation: null,
    recettesEleves: null,
    totalSubventions: null,
    totalAidesIndividuelles: null,
    totalRecettes: null,
    depensesAvecMargeRisque: null,
    margeRisqueMontant: null,
    prixFactureBus: null,
    autresDepensesHorsBus: null,
    prixParEleveAvantMargeRisque: null,
    prixParEleveAvecSubventions: null,
    coutPrevisionnelParEleve: null,
    excedentOuDeficit: null,
  };
}

/** Extrait le plus grand montant TTC plausible depuis un texte OCR (repli si l'IA échoue). */
export function extractLargestEuroAmountFromText(text: string): number | null {
  const raw = String(text || "");
  if (!raw.trim()) return null;

  const candidates: number[] = [];
  const labeled = [
    /total\s*ttc[^\d]{0,20}([\d\s.,]+)/gi,
    /montant\s*ttc[^\d]{0,20}([\d\s.,]+)/gi,
    /net\s*à\s*payer[^\d]{0,20}([\d\s.,]+)/gi,
    /total\s*général[^\d]{0,20}([\d\s.,]+)/gi,
    /total[^\d]{0,12}([\d\s.,]{4,})\s*€/gi,
  ];
  for (const re of labeled) {
    for (const m of raw.matchAll(re)) {
      const n = parseEuroAmount(m[1]);
      if (n != null && n >= 50) candidates.push(n);
    }
  }
  for (const m of raw.matchAll(/([\d]{1,3}(?:[.\s]\d{3})*,\d{2}|\d+[.,]\d{2})\s*€/g)) {
    const n = parseEuroAmount(m[1]);
    if (n != null && n >= 50) candidates.push(n);
  }
  for (const m of raw.matchAll(/€\s*([\d]{1,3}(?:[.\s]\d{3})*,\d{2}|\d+[.,]\d{2})/g)) {
    const n = parseEuroAmount(m[1]);
    if (n != null && n >= 50) candidates.push(n);
  }

  if (!candidates.length) return null;
  return Math.max(...candidates);
}

export function resolveBusDepenseAmount(sheet: Pick<TravelsComptaSheet, "depenses">): number | null {
  return sheet.depenses.find((d) => d.source === "devis_signe")?.amount ?? null;
}

export function computeComptaSheetDerived(
  sheet: TravelsComptaSheet,
  trip?: Pick<TravelsTrip, "type" | "data"> | null,
): TravelsComptaSheet {
  const depenses = sheet.depenses ?? [];
  const recettesLignes = normalizeComptaRecettesLignes(sheet);
  const depensesTotal = depenses.reduce((sum, line) => sum + (line?.amount ?? 0), 0);
  const nb = sheet.nbEleves ?? 0;
  const depensesTotalRounded =
    depensesTotal > 0 ? depensesTotal : depensesTotal === 0 ? 0 : null;

  const prixParEleve =
    nb > 0 && depensesTotalRounded != null
      ? Math.round((depensesTotalRounded / nb) * 100) / 100
      : null;

  const busAmount = resolveBusDepenseAmount({ depenses });
  const prixFactureBus = busAmount;
  const autresDepensesTotal = autresDepensesHorsBus(depensesTotalRounded, busAmount);
  const recettesFigees = comptaRecettesFigees(sheet);
  const rawMarge = resolveMargeSecuriteEuro(sheet, depensesTotalRounded, trip);

  let margeFigeeEuro = sheet.margeFigeeEuro;
  if (recettesFigees && margeFigeeEuro == null && rawMarge > 0) {
    margeFigeeEuro = rawMarge;
  }

  const margeRisqueMontant = recettesFigees ? (margeFigeeEuro ?? 0) : rawMarge;
  const depensesAvecMargeRisque =
    depensesTotalRounded != null
      ? Math.round((depensesTotalRounded + margeRisqueMontant) * 100) / 100
      : null;

  const montantCibleFacturation = depensesAvecMargeRisque;
  const coutPrevisionnelParEleve = perStudentEuroCeil(montantCibleFacturation, nb);
  const nbFactures = sheet.nbElevesFactures ?? (nb > 0 ? nb : null);

  let recettesEleves: number | null;
  if (recettesFigees && sheet.prixParEleveAnnonce != null && nbFactures != null && nbFactures > 0) {
    recettesEleves = Math.round(sheet.prixParEleveAnnonce * nbFactures * 100) / 100;
  } else {
    recettesEleves = montantCibleFacturation;
  }

  const facturations = buildComptaFacturation(sheet, busAmount, montantCibleFacturation);

  const totalAidesIndividuelles = sheet.aidesIndividuelles.reduce((sum, a) => sum + (a.amount ?? 0), 0);
  const totalSubventions = recettesLignes.reduce((sum, line) => sum + (line?.amount ?? 0), 0);

  const recettesSum = (recettesEleves ?? 0) + totalSubventions;
  const totalRecettes =
    recettesEleves != null || totalSubventions > 0 || depensesTotalRounded != null
      ? Math.round(recettesSum * 100) / 100
      : null;

  const prixParEleveAvantMargeRisque =
    nb > 0 && depensesTotalRounded != null
      ? Math.round((Math.max(0, depensesTotalRounded - totalSubventions) / nb) * 100) / 100
      : null;

  const prixParEleveAvecSubventions =
    nb > 0 && depensesAvecMargeRisque != null
      ? perStudentEuroCeil(Math.max(0, depensesAvecMargeRisque - totalSubventions), nb)
      : prixParEleveAvantMargeRisque != null
        ? Math.ceil(prixParEleveAvantMargeRisque)
        : null;

  const excedentOuDeficit =
    depensesTotalRounded != null
      ? recettesFigees && montantCibleFacturation != null && totalRecettes != null
        ? Math.round((totalRecettes - montantCibleFacturation) * 100) / 100
        : Math.round((recettesSum - depensesTotalRounded) * 100) / 100
      : null;

  return {
    ...sheet,
    depenses,
    recettesLignes,
    apelAidesCollectives: null,
    autresSubventions: null,
    margeFigeeEuro: recettesFigees ? margeFigeeEuro : sheet.margeFigeeEuro,
    recettesElevesFigees: recettesFigees,
    prixParEleve,
    facturations,
    facturation: facturations[0],
    depensesTotal: depensesTotalRounded,
    montantCibleFacturation,
    recettesEleves,
    totalSubventions: totalSubventions > 0 ? totalSubventions : totalSubventions === 0 ? 0 : null,
    totalAidesIndividuelles:
      totalAidesIndividuelles > 0
        ? totalAidesIndividuelles
        : totalAidesIndividuelles === 0
          ? 0
          : null,
    totalRecettes,
    depensesAvecMargeRisque,
    margeRisqueMontant: depensesTotalRounded != null ? margeRisqueMontant : null,
    prixFactureBus,
    autresDepensesHorsBus: autresDepensesTotal,
    prixParEleveAvantMargeRisque,
    prixParEleveAvecSubventions,
    coutPrevisionnelParEleve,
    excedentOuDeficit,
  };
}

function tripNbEleves(trip: TravelsTrip): number | null {
  const n = parseEuroAmount(trip.data?.nbEleves);
  return n != null ? Math.round(n) : null;
}

function tripNbAccompagnateurs(trip: TravelsTrip): number | null {
  const n = parseEuroAmount(trip.data?.nbAccompagnateurs);
  return n != null ? Math.round(n) : null;
}

function isTransportExpenseLabel(label: string): boolean {
  const t = label.toLowerCase();
  return (
    t.includes("transport") ||
    t.includes("bus") ||
    t.includes("autocar") ||
    t.includes("car ") ||
    t.startsWith("car ") ||
    t.includes("autocariste")
  );
}

function normalizeDocName(name: string): string {
  return name
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/[_-]+/g, " ");
}

/** Documents administratifs / pédagogiques sans montant — exclus de la compta. */
const NON_FINANCIAL_ATTACHMENT_KEYWORDS = [
  "circulaire",
  "programme pedagogique",
  "programme pedago",
  "programme de voyage",
  "programme voyage",
  "prog pedagog",
  "pedagogique",
  "pedago",
  "convocation",
  "liste eleve",
  "liste des eleves",
  "feuille de presence",
  "emargement",
  "autorisation parentale",
  "planning",
  "fiche informative",
  "fiche info",
  "carnet de voyage",
  "ordre du jour",
  "synthese",
  "compte rendu",
  "affiche",
  "diaporama",
];

/** Pièces jointes susceptibles de contenir un tarif (devis, facture…). */
const FINANCIAL_ATTACHMENT_KEYWORDS = [
  "devis",
  "facture",
  "tarif",
  "prix",
  "bon de commande",
  "bon_de_commande",
  "offre",
  "proposition",
  "prestation",
  "montant",
  "€",
  "euro",
  "ttc",
  "ht",
];

/** Une pièce jointe du dossier doit-elle apparaître dans les dépenses compta ? */
export function isComptaExpenseAttachmentName(name: string): boolean {
  const t = normalizeDocName(name.trim());
  if (!t) return false;
  if (isNonFinancialDepenseLabel(t)) return false;
  return FINANCIAL_ATTACHMENT_KEYWORDS.some((k) => t.includes(k));
}

export function isNonFinancialDepenseLabel(label: string): boolean {
  const t = normalizeDocName(label.trim());
  if (!t) return false;
  return NON_FINANCIAL_ATTACHMENT_KEYWORDS.some((k) => t.includes(k));
}

function attachmentMetaForUrl(
  trip: TravelsTrip,
  fileUrl: string,
): { s3Key?: string | null; name?: string } {
  const att = (trip.data?.attachments || []).find((a) => String(a.url || "") === fileUrl);
  return {
    s3Key: att?.s3Key ? String(att.s3Key) : null,
    name: att?.name ? String(att.name) : undefined,
  };
}

function quoteRecordAmount(quote: Record<string, unknown> | undefined): number | null {
  if (!quote) return null;
  return (
    parseEuroAmount(quote.extractedPrice) ??
    parseEuroAmount(quote.price) ??
    parseEuroAmount(quote.montant_ttc) ??
    parseEuroAmount(quote.montantTtc) ??
    parseEuroAmount(quote.amount) ??
    parseEuroAmount(quote.montant) ??
    parseEuroAmount(quote.total) ??
    parseEuroAmount(quote.cout) ??
    parseEuroAmount(quote.agreedPrice) ??
    null
  );
}

function findMatchingReceivedBusQuote(
  trip: TravelsTrip,
  selected?: Record<string, unknown>,
): Record<string, unknown> | undefined {
  const selectedUrl = String(selected?.fileUrl || "");
  const selectedId = String(selected?.id || "");
  const provider = String(selected?.providerName || selected?.extractedCompany || "").toLowerCase();

  for (const q of trip.receivedDevis || []) {
    const quote = q as Record<string, unknown>;
    const fileUrl = String(quote.fileUrl || "");
    if (selectedUrl && fileUrl && fileUrl === selectedUrl) return quote;
    if (selectedId && String(quote.id || "") === selectedId) return quote;
  }

  if (provider) {
    for (const q of trip.receivedDevis || []) {
      const quote = q as Record<string, unknown>;
      const quoteProvider = String(quote.providerName || quote.extractedCompany || "").toLowerCase();
      if (quoteProvider && quoteProvider === provider) return quote;
    }
  }
  return undefined;
}

/** Montant bus depuis le dossier voyage uniquement (sans la fiche compta). */
export function resolveBusQuoteAmountFromTrip(trip: TravelsTrip): number | null {
  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const fromSelected = quoteRecordAmount(selected);
  if (fromSelected != null) return fromSelected;

  const matched = findMatchingReceivedBusQuote(trip, selected);
  const fromReceived = quoteRecordAmount(matched);
  if (fromReceived != null) return fromReceived;

  const signedUrl = String(trip.data?.signedQuoteUrl || "");
  if (signedUrl) {
    for (const q of trip.receivedDevis || []) {
      const quote = q as Record<string, unknown>;
      if (String(quote.fileUrl || "") === signedUrl) {
        const amount = quoteRecordAmount(quote);
        if (amount != null) return amount;
      }
    }
  }

  return providerBusQuoteFromReceivedDevis(trip);
}

function providerBusQuoteFromReceivedDevis(trip: TravelsTrip): number | null {
  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const provider = String(selected?.providerName || selected?.extractedCompany || "").toLowerCase();
  if (!provider) return null;
  for (const q of trip.receivedDevis || []) {
    const quote = q as Record<string, unknown>;
    const quoteProvider = String(quote.providerName || quote.extractedCompany || "").toLowerCase();
    if (!quoteProvider || quoteProvider !== provider) continue;
    const amount = quoteRecordAmount(quote);
    if (amount != null) return amount;
  }
  return null;
}

/** Montant connu du devis bus (sélectionné, reçu par mail ou facturation compta). */
export function resolveSignedBusQuoteAmount(
  trip: TravelsTrip,
  sheet?: Partial<Pick<TravelsComptaSheet, "facturations" | "facturation" | "depenses">> | null,
): number | null {
  const savedBus = sheet?.depenses?.find((d) => d.source === "devis_signe")?.amount;
  if (isUsableComptaAmount(savedBus)) return savedBus;

  const fromFacturation = parseEuroAmount(resolveFacturationsFromSheet(sheet ?? {})[0]?.prixFacture);
  if (fromFacturation != null) return fromFacturation;

  return resolveBusQuoteAmountFromTrip(trip);
}

export function applyBusQuoteAmountFallback(
  trip: TravelsTrip,
  depenses: TravelsComptaExpenseLine[],
  sheet?: Partial<Pick<TravelsComptaSheet, "facturations" | "facturation" | "depenses">> | null,
): TravelsComptaExpenseLine[] {
  const manualBus = sheet?.depenses?.find((d) => d.source === "devis_signe")?.amount;
  const busAmount =
    isUsableComptaAmount(manualBus)
      ? manualBus
      : resolveBusQuoteAmountFromTrip(trip) ??
        parseEuroAmount(resolveFacturationsFromSheet(sheet ?? {})[0]?.prixFacture);
  if (!isUsableComptaAmount(busAmount)) return depenses;
  return depenses.map((line) =>
    line.source === "devis_signe"
      ? { ...line, amount: isUsableComptaAmount(line.amount) ? line.amount : busAmount }
      : line,
  );
}

function resolveDepenseLineAmount(
  ref: TravelsComptaDocumentRef,
  saved: TravelsComptaExpenseLine | undefined,
  scan: TravelsComptaDocumentScan | undefined,
  trip: TravelsTrip,
): number | null {
  if (isUsableComptaAmount(saved?.amount)) return saved!.amount!;

  if (ref.role === "devis_signe") {
    if (isUsableComptaAmount(scan?.amount)) return scan!.amount!;
    const fromTrip = resolveBusQuoteAmountFromTrip(trip);
    if (isUsableComptaAmount(fromTrip)) return fromTrip;
  } else if (isUsableComptaAmount(scan?.amount)) {
    return scan!.amount!;
  }

  return isUsableComptaAmount(ref.fallbackAmount) ? ref.fallbackAmount! : null;
}

/** Prix par élève définitif transmis à la direction après validation compta. */
export function comptaDefinitiveCostPerStudent(sheet: TravelsComptaSheet): number | null {
  const derived = computeComptaSheetDerived(sheet);
  if (derived.recettesElevesFigees && sheet.prixParEleveAnnonce != null) {
    return sheet.prixParEleveAnnonce;
  }
  return derived.prixParEleveAvecSubventions ?? derived.prixParEleve ?? null;
}

function isManualComptaDepenseLine(line: TravelsComptaExpenseLine, trip: TravelsTrip): boolean {
  if (!line.label.trim() && line.amount == null) return false;
  const src = String(line.source || "").trim();
  if (src === "devis_signe" || src === "budget_previsionnel") return false;
  if (src.startsWith("http")) return false;
  if (isNonFinancialDepenseLabel(line.label)) return false;
  if (isUnsignedBusDevisLabel(line.label, trip)) return false;
  if (isTransportExpenseLabel(line.label)) return false;
  return true;
}

const ACTIVITY_QUOTE_KEYWORDS = [
  "musée",
  "musee",
  "parc",
  "aquarium",
  "zoo",
  "château",
  "chateau",
  "activité",
  "activite",
  "entrée",
  "entree",
  "restaurant",
  "cinéma",
  "cinema",
  "spectacle",
  "guide",
  "atelier",
  "visite",
];

function quoteText(quote: Record<string, unknown>): string {
  return `${quote.providerName || ""} ${quote.extractedCompany || ""} ${quote.originalFilename || ""}`.toLowerCase();
}

/** Devis reçu par mail classé transport mais qui ressemble à une activité / entrée. */
export function isLikelyActivityQuote(quote: Record<string, unknown>): boolean {
  const text = quoteText(quote);
  if (!text.trim()) return false;
  if (ACTIVITY_QUOTE_KEYWORDS.some((k) => text.includes(k))) return true;
  return !isTransportExpenseLabel(text);
}

function unsignedBusProviderNames(trip: TravelsTrip): string[] {
  const signedProvider = String(
    (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.providerName || "",
  ).toLowerCase();
  return (trip.receivedDevis || [])
    .map((q) => String((q as Record<string, unknown>).providerName || "").toLowerCase())
    .filter((p) => p && p !== signedProvider);
}

function isUnsignedBusDevisLabel(label: string, trip: TravelsTrip): boolean {
  const l = label.trim().toLowerCase();
  if (!l) return false;
  if (l.includes("signé") || l.includes("signe")) return false;
  if (isTransportExpenseLabel(l)) return true;
  if (/^devis\s*[-—]/.test(l)) {
    for (const provider of unsignedBusProviderNames(trip)) {
      if (l.includes(provider)) return true;
    }
  }
  return false;
}

function normalizeDepenseKey(line: TravelsComptaExpenseLine): string {
  const src = String(line.source || "").trim();
  if (src && src !== "piece_jointe" && src !== "devis_signe" && src !== "budget_previsionnel") {
    return src;
  }
  return line.label.trim().toLowerCase();
}

/** Ligne transport : uniquement le devis bus signé. Autres lignes : pièces jointes et devis activités. */
export function buildDefaultComptaDepenses(trip: TravelsTrip): TravelsComptaExpenseLine[] {
  const depenses: TravelsComptaExpenseLine[] = [];
  const signedUrl = trip.data?.signedQuoteUrl ? String(trip.data.signedQuoteUrl) : "";
  const selectedUrl = String(
    (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.fileUrl || "",
  );
  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const coutPrev = parseEuroAmount(trip.data?.coutTotal);
  const seenSources = new Set<string>();

  if (signedUrl) {
    depenses.push({
      label: selected?.providerName
        ? `Transport (devis bus signé) — ${String(selected.providerName)}`
        : "Transport (devis bus signé)",
      amount: resolveBusQuoteAmountFromTrip(trip),
      source: "devis_signe",
    });
    seenSources.add(signedUrl);
  } else if (tripTransportBypassed(trip)) {
    depenses.push({
      label: "Transport (saisie manuelle)",
      amount: resolveBusQuoteAmountFromTrip(trip),
      source: "devis_signe",
    });
  }

  for (const att of trip.data?.attachments || []) {
    const url = String(att.url || "");
    const name = String(att.name || "").trim();
    if (!url || url === signedUrl || seenSources.has(url)) continue;
    if (!isComptaExpenseAttachmentName(name)) continue;
    seenSources.add(url);
    depenses.push({
      label: name || "Devis / document (hors transport)",
      amount: null,
      source: url,
    });
  }

  for (const q of trip.receivedDevis || []) {
    const quote = q as Record<string, unknown>;
    const fileUrl = String(quote.fileUrl || "");
    if (!fileUrl || fileUrl === signedUrl || fileUrl === selectedUrl || seenSources.has(fileUrl)) continue;
    if (!isLikelyActivityQuote(quote)) continue;
    seenSources.add(fileUrl);
    depenses.push({
      label: quote.providerName ? `Devis — ${String(quote.providerName)}` : "Devis activité / entrée",
      amount: parseEuroAmount(quote.extractedPrice) ?? parseEuroAmount(quote.price) ?? null,
      source: fileUrl,
    });
  }

  if (coutPrev != null && depenses.filter((d) => d.source !== "devis_signe").length === 0) {
    depenses.push({
      label: "Budget prévisionnel (activités / entrées)",
      amount: coutPrev,
      source: "budget_previsionnel",
    });
  }

  while (depenses.length < 2) depenses.push({ label: "", amount: null });
  return depenses;
}

/** Documents du dossier à suivre pour la fiche compta (hors devis bus non signés). */
export function listComptaDocumentRefs(trip: TravelsTrip): TravelsComptaDocumentRef[] {
  const refs: TravelsComptaDocumentRef[] = [];
  const signedUrl = trip.data?.signedQuoteUrl ? String(trip.data.signedQuoteUrl) : "";
  const selectedUrl = String(
    (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.fileUrl || "",
  );
  const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
  const coutPrev = parseEuroAmount(trip.data?.coutTotal);
  const seen = new Set<string>();

  if (signedUrl) {
    const signedMeta = attachmentMetaForUrl(trip, signedUrl);
    refs.push({
      key: "devis_signe",
      fileUrl: signedUrl,
      label: selected?.providerName
        ? `Transport (devis bus signé) — ${String(selected.providerName)}`
        : "Transport (devis bus signé)",
      role: "devis_signe",
      s3Key:
        signedMeta.s3Key ||
        (selected?.s3KeyIncoming ? String(selected.s3KeyIncoming) : null) ||
        (selected?.fileUrl ? attachmentMetaForUrl(trip, String(selected.fileUrl)).s3Key : null),
      fallbackAmount: resolveBusQuoteAmountFromTrip(trip),
    });
    seen.add(signedUrl);
  } else if (tripTransportBypassed(trip)) {
    refs.push({
      key: "devis_signe",
      fileUrl: "",
      label: "Transport (saisie manuelle — sans devis signé)",
      role: "devis_signe",
      fallbackAmount: resolveBusQuoteAmountFromTrip(trip),
    });
  }

  for (const att of trip.data?.attachments || []) {
    const fileUrl = String(att.url || "");
    const name = String(att.name || "").trim();
    if (!fileUrl || fileUrl === signedUrl || seen.has(fileUrl)) continue;
    if (!isComptaExpenseAttachmentName(name)) continue;
    seen.add(fileUrl);
    refs.push({
      key: fileUrl,
      fileUrl,
      label: name || "Devis / document (hors transport)",
      role: "piece_jointe",
      s3Key: att.s3Key ? String(att.s3Key) : null,
    });
  }

  for (const q of trip.receivedDevis || []) {
    const quote = q as Record<string, unknown>;
    const fileUrl = String(quote.fileUrl || "");
    if (!fileUrl || fileUrl === signedUrl || fileUrl === selectedUrl || seen.has(fileUrl)) continue;
    if (!isLikelyActivityQuote(quote)) continue;
    seen.add(fileUrl);
    refs.push({
      key: fileUrl,
      fileUrl,
      label: quote.providerName ? `Devis — ${String(quote.providerName)}` : "Devis activité / entrée",
      role: "devis_activite",
      s3Key: quote.s3KeyIncoming ? String(quote.s3KeyIncoming) : null,
      fallbackAmount: parseEuroAmount(quote.extractedPrice) ?? parseEuroAmount(quote.price) ?? null,
    });
  }

  if (coutPrev != null && refs.filter((r) => r.role !== "devis_signe").length === 0) {
    refs.push({
      key: "budget_previsionnel",
      fileUrl: "",
      label: "Budget prévisionnel (activités / entrées)",
      role: "budget_previsionnel",
      fallbackAmount: coutPrev,
    });
  }

  return refs;
}

export function comptaDocumentsFingerprint(trip: TravelsTrip): string {
  return listComptaDocumentRefs(trip)
    .map((r) => `${r.key}\t${r.label}\t${r.role}`)
    .join("\n");
}

/** Marque la fiche comme synchronisée avec les documents du dossier (évite un re-OCR qui efface la saisie). */
export function finalizeComptaSheetForPersist(
  trip: TravelsTrip,
  sheet: TravelsComptaSheet,
): TravelsComptaSheet {
  const fingerprint = comptaDocumentsFingerprint(trip);
  const now = new Date().toISOString();
  return computeComptaSheetDerived(
    patchDocumentScansFromDepenses({
      ...sheet,
      syncedDocumentsFingerprint: fingerprint,
      syncedAt: now,
      analyzedAt: sheet.analyzedAt || now,
    }),
    trip,
  );
}

function tripTransportBypassed(trip: TravelsTrip): boolean {
  return Boolean(trip.data?.transportPhaseBypassedAt);
}

function depenseSourceKey(line: TravelsComptaExpenseLine): string {
  const src = String(line.source || "").trim();
  if (src === "devis_signe") return "devis_signe";
  if (src === "budget_previsionnel") return "budget_previsionnel";
  if (src) return src;
  return line.label.trim().toLowerCase();
}

/** Applique le cache OCR + saisie existante sur la structure canonique du dossier. */
export function depensesFromDocumentSync(
  trip: TravelsTrip,
  scans: TravelsComptaDocumentScan[],
  savedDepenses?: TravelsComptaExpenseLine[] | null,
): TravelsComptaExpenseLine[] {
  const scanByKey = new Map(scans.map((s) => [s.key, s]));
  const savedByKey = new Map(
    (savedDepenses || []).map((d) => [depenseSourceKey(d), d]),
  );
  const refs = listComptaDocumentRefs(trip);
  const lines: TravelsComptaExpenseLine[] = refs.map((ref) => {
    const scan = scanByKey.get(ref.key);
    const saved = savedByKey.get(ref.key);
    const source = ref.role === "devis_signe" ? "devis_signe" : ref.role === "budget_previsionnel" ? "budget_previsionnel" : ref.fileUrl;
    return {
      label: saved?.label?.trim() ? saved.label : ref.label,
      amount: resolveDepenseLineAmount(ref, saved, scan, trip),
      source,
    };
  });

  for (const saved of savedDepenses || []) {
    if (!isManualComptaDepenseLine(saved, trip)) continue;
    const key = depenseSourceKey(saved);
    if (lines.some((l) => depenseSourceKey(l) === key)) continue;
    lines.push(saved);
  }

  while (lines.length < 2) lines.push({ label: "", amount: null });
  return filterComptaDepenses(
    applyBusQuoteAmountFallback(trip, lines, savedDepenses ? { depenses: savedDepenses } : null),
    trip,
  );
}

export function documentsNeedComptaSync(trip: TravelsTrip, sheet: TravelsComptaSheet | null): boolean {
  const fp = comptaDocumentsFingerprint(trip);
  if (!sheet?.analyzedAt) return true;
  if (sheet.syncedDocumentsFingerprint !== fp) return true;
  const refKeys = new Set(listComptaDocumentRefs(trip).map((r) => r.key));
  const scanKeys = new Set((sheet.documentScans || []).map((s) => s.key));
  for (const k of refKeys) {
    if (k === "budget_previsionnel") continue;
    if (!scanKeys.has(k)) return true;
  }
  for (const k of scanKeys) {
    if (!refKeys.has(k)) return true;
  }
  const validSources = new Set(
    listComptaDocumentRefs(trip).flatMap((r) =>
      r.role === "devis_signe" ? ["devis_signe", r.fileUrl] : r.fileUrl ? [r.fileUrl, r.key] : [r.key],
    ),
  );
  for (const line of sheet.depenses || []) {
    if (isNonFinancialDepenseLabel(line.label)) return true;
    const src = String(line.source || "").trim();
    if (src.startsWith("http") && !validSources.has(src)) return true;
  }
  return false;
}

/** Retire les devis transport non signés ; une seule ligne transport (devis signé). */
export function filterComptaDepenses(
  depenses: TravelsComptaExpenseLine[],
  trip: TravelsTrip,
): TravelsComptaExpenseLine[] {
  const signedUrl = trip.data?.signedQuoteUrl ? String(trip.data.signedQuoteUrl) : "";
  const selectedUrl = String(
    (trip.data?.selectedBusQuote as Record<string, unknown> | undefined)?.fileUrl || "",
  );
  const receivedBusUrls = new Set(
    (trip.receivedDevis || [])
      .map((q) => String((q as Record<string, unknown>).fileUrl || ""))
      .filter(Boolean),
  );

  const nonTransport: TravelsComptaExpenseLine[] = [];
  let transportLine: TravelsComptaExpenseLine | null = null;

  for (const line of depenses) {
    const src = String(line.source || "");
    const label = line.label.trim();
    if (!label && line.amount == null) {
      nonTransport.push(line);
      continue;
    }

    if (isNonFinancialDepenseLabel(label)) continue;

    // Ligne canonique du devis bus signé — ne jamais la filtrer comme « devis non signé »
    if (src === "devis_signe") {
      transportLine = { ...line, source: "devis_signe" };
      continue;
    }

    const isUnsignedBus =
      src === "devis_recu" ||
      src === "devis_choisi" ||
      (src !== "" && receivedBusUrls.has(src) && src !== signedUrl) ||
      (selectedUrl !== "" && src === selectedUrl && src !== signedUrl) ||
      isUnsignedBusDevisLabel(label, trip);

    if (isUnsignedBus) continue;

    const transportLike = isTransportExpenseLabel(label) && src !== "budget_previsionnel";
    if (transportLike) {
      continue;
    }

    nonTransport.push(line);
  }

  if (signedUrl && !transportLine) {
    const selected = trip.data?.selectedBusQuote as Record<string, unknown> | undefined;
    transportLine = {
      label: selected?.providerName
        ? `Transport (devis bus signé) — ${String(selected.providerName)}`
        : "Transport (devis bus signé)",
      amount: resolveBusQuoteAmountFromTrip(trip),
      source: "devis_signe",
    };
  }

  const out = [...(transportLine ? [transportLine] : []), ...nonTransport];
  while (out.length < 2) out.push({ label: "", amount: null });
  return out;
}

/** Structure canonique du dossier + montants issus de la sauvegarde ou de l'IA. */
export function resolveComptaDepenses(
  trip: TravelsTrip,
  incoming?: TravelsComptaExpenseLine[] | null,
): TravelsComptaExpenseLine[] {
  const canonical = buildDefaultComptaDepenses(trip);
  if (!incoming?.some((d) => d.label.trim() || d.amount != null)) {
    return filterComptaDepenses(canonical, trip);
  }

  const filtered = filterComptaDepenses(incoming, trip);
  const merged: TravelsComptaExpenseLine[] = canonical.map((line) => {
    const key = normalizeDepenseKey(line);
    const hit =
      filtered.find((i) => normalizeDepenseKey(i) === key) ||
      filtered.find((i) => i.label.trim().toLowerCase() === line.label.trim().toLowerCase()) ||
      (line.source === "devis_signe"
        ? filtered.find((i) => i.source === "devis_signe")
        : undefined);
    if (!hit) return line;
    return {
      ...line,
      label: hit.label.trim() ? hit.label : line.label,
      amount: hit.amount ?? line.amount,
    };
  });

  for (const line of filtered) {
    if (!isManualComptaDepenseLine(line, trip)) continue;
    const key = normalizeDepenseKey(line);
    if (merged.some((m) => normalizeDepenseKey(m) === key)) continue;
    merged.push(line);
  }

  return filterComptaDepenses(merged, trip);
}

export function comptaSheetFromTrip(trip: TravelsTrip, existing?: TravelsComptaSheet | null): TravelsComptaSheet {
  const base = existing ? { ...existing } : emptyComptaSheet();
  const nbEleves = tripNbEleves(trip);
  const nbAcc = tripNbAccompagnateurs(trip);
  const depenses = applyBusQuoteAmountFallback(
    trip,
    resolveComptaDepenses(trip, base.depenses),
    base,
  );
  const depensesTotal = depenses.reduce((sum, line) => sum + (line?.amount ?? 0), 0);

  const sheet: TravelsComptaSheet = {
    ...base,
    classe: base.classe || String(trip.data?.classes || ""),
    accompagnateurs:
      base.accompagnateurs ||
      [trip.data?.nomsAccompagnateurs, nbAcc != null ? `${nbAcc} accompagnateur(s)` : ""]
        .filter(Boolean)
        .join(" — "),
    profs: base.profs || String(trip.data?.ownerName || ""),
    nbEleves: base.nbEleves ?? nbEleves,
    depenses,
    margeSecuriteEuro:
      base.margeSecuriteEuro ??
      (base.facteurRisque
        ? suggestComptaMargeFromPreset(depensesTotal, comptaMargePresetPercent(base.facteurRisque))
        : inferComptaMargeSecuriteEuro(trip, depensesTotal)),
  };

  return computeComptaSheetDerived(sheet, trip);
}

export function readComptaSheetFromTrip(trip: TravelsTrip): TravelsComptaSheet | null {
  const raw = trip.data?.comptaSheet;
  if (!raw || typeof raw !== "object") return null;
  const rawSheet = raw as TravelsComptaSheet;
  const sheet = computeComptaSheetDerived({ ...emptyComptaSheet(), ...rawSheet }, trip);
  const depenses =
    sheet.documentScans && sheet.documentScans.length > 0
      ? depensesFromDocumentSync(trip, sheet.documentScans, sheet.depenses)
      : resolveComptaDepenses(trip, sheet.depenses);
  return computeComptaSheetDerived({ ...sheet, depenses }, trip);
}

/** Met à jour le cache documentaire quand la compta modifie un montant à la main. */
export function patchDocumentScansFromDepenses(sheet: TravelsComptaSheet): TravelsComptaSheet {
  if (!sheet.documentScans?.length) return sheet;
  const byKey = new Map(sheet.depenses.map((d) => [depenseSourceKey(d), d]));
  const documentScans = sheet.documentScans.map((scan) => {
    const dep = byKey.get(scan.key);
    if (dep?.amount != null) return { ...scan, amount: dep.amount };
    return scan;
  });
  return { ...sheet, documentScans };
}
