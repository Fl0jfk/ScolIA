import type { TravelsHistoryEntry, TravelsTrip, TravelsTripData } from "@/app/lib/travels-types";
import {
  calendarHasDepotAndRecuperation,
  defaultParentCalendarFromTrip,
} from "@/app/lib/travels-parent-calendar";
import { formatCuisineDateFR } from "@/app/lib/travels-cuisine-shared";

/** Motif affiché quand le dossier est en « Modifications demandées » (pas la dernière ligne d'historique). */
export function getModificationRequestNote(trip: {
  status?: string;
  data?: TravelsTripData;
  history?: TravelsHistoryEntry[];
}): string {
  const stored = trip.data?.modificationRequestNote;
  if (typeof stored === "string" && stored.trim()) return stored.trim();
  const history = Array.isArray(trip.history) ? trip.history : [];
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i]?.action === "BESOIN_MODIFICATION" && history[i]?.note?.trim()) {
      return history[i].note!.trim();
    }
  }
  return "";
}

export function complexNeedsBus(trip: { type?: string; data?: { needsBus?: boolean } } | null) {
  return trip?.type === "COMPLEX" && Boolean(trip?.data?.needsBus);
}

export function tripEffectifTotal(data: TravelsTripData | undefined): number {
  return Number(data?.nbEleves) + Number(data?.nbAccompagnateurs || 0);
}

function tripDateLabel(trip: TravelsTrip): string {
  const d = trip.data;
  if (trip.type === "COMPLEX") {
    const start = formatCuisineDateFR(d.startDate);
    const end = formatCuisineDateFR(d.endDate);
    return `${start} → ${end}`;
  }
  return formatCuisineDateFR(d.date || d.startDate);
}

export function tripDateRangeLabel(data: TravelsTripData): string {
  const startDateFR = formatCuisineDateFR(data.startDate ?? data.date);
  const endDateFR = formatCuisineDateFR(data.endDate);
  return endDateFR && endDateFR !== "—" && endDateFR !== startDateFR
    ? `du ${startDateFR} au ${endDateFR}`
    : `le ${startDateFR}`;
}

export function effectifChangedSinceSnapshot(
  data: TravelsTripData,
  snapshot?: { nbEleves: number; nbAccompagnateurs?: number } | null,
): boolean {
  if (!snapshot) return false;
  return (
    Number(snapshot.nbEleves) !== Number(data.nbEleves) ||
    Number(snapshot.nbAccompagnateurs || 0) !== Number(data.nbAccompagnateurs || 0)
  );
}

export function datesChangedSinceSnapshot(
  data: TravelsTripData,
  snapshot?: TravelsTripData["transportDateSnapshot"] | null,
): boolean {
  if (!snapshot) return false;
  return (
    String(snapshot.startDate || "") !== String(data.startDate || data.date || "") ||
    String(snapshot.endDate || "") !== String(data.endDate || "") ||
    String(snapshot.startTime || "") !== String(data.startTime || "") ||
    String(snapshot.endTime || "") !== String(data.endTime || "")
  );
}

export function cuisineEffectifChanged(data: TravelsTripData): boolean {
  const snap = data.cuisineOrderSnapshot as { nbEleves?: number; nbAccompagnateurs?: number } | undefined;
  if (!data.cuisineOrderSentAt || !snap) return false;
  return effectifChangedSinceSnapshot(data, {
    nbEleves: Number(snap.nbEleves) || 0,
    nbAccompagnateurs: Number(snap.nbAccompagnateurs) || 0,
  });
}

/** Date d'envoi cuisine — champ S3 ou repli journal (dossiers validés avant correctif). */
export function resolveCuisineOrderSentAt(trip: {
  data?: TravelsTripData;
  history?: TravelsHistoryEntry[];
  status?: string;
}): string | undefined {
  if (trip.data?.cuisineOrderSentAt) return trip.data.cuisineOrderSentAt;
  const history = Array.isArray(trip.history) ? trip.history : [];
  for (let i = history.length - 1; i >= 0; i--) {
    const h = history[i];
    const action = String(h?.action || "").toLowerCase();
    const note = String(h?.note || "").toLowerCase();
    if (action.includes("cuisine envoyée") || note.includes("commande cuisine envoyée")) {
      return h?.date;
    }
  }
  return undefined;
}

export function cuisineOrderWasSent(trip: {
  data?: TravelsTripData;
  history?: TravelsHistoryEntry[];
  status?: string;
}): boolean {
  return Boolean(resolveCuisineOrderSentAt(trip));
}

export function busLogisticsActive(trip: TravelsTrip): boolean {
  return (
    complexNeedsBus(trip) &&
    Boolean(
      trip.data.transportQuoteSnapshot ||
        trip.data.selectedBusQuote ||
        trip.data.signedQuoteUrl ||
        (Array.isArray(trip.receivedDevis) && trip.receivedDevis.length > 0),
    )
  );
}

export function isValidEmailLoose(s: string) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(s.trim());
}

function daysUntilTrip(data: TravelsTripData): number | null {
  const raw = data.startDate || data.date;
  if (!raw) return null;
  const tripDate = new Date(raw);
  if (Number.isNaN(tripDate.getTime())) return null;
  const now = new Date();
  now.setHours(0, 0, 0, 0);
  tripDate.setHours(0, 0, 0, 0);
  return Math.round((tripDate.getTime() - now.getTime()) / 86400000);
}

function todayStartMs(): number {
  const t = new Date();
  t.setHours(0, 0, 0, 0);
  return t.getTime();
}

/** Timestamp (début de journée) du séjour pour tri / comparaison. */
function tripTravelStartMs(trip: { data?: TravelsTripData }): number | null {
  const raw = trip.data?.startDate || trip.data?.date;
  if (!raw) return null;
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) return null;
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** Dernier jour du séjour (fin pour les voyages, date unique pour les sorties). */
function tripTravelEndMs(trip: {
  type?: string;
  data?: TravelsTripData;
}): number | null {
  const d = trip.data;
  if (!d) return null;
  const raw =
    trip.type === "COMPLEX"
      ? d.endDate || d.startDate || d.date
      : d.date || d.startDate;
  if (!raw) return null;
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) return null;
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** Séjour terminé (dernier jour strictement avant aujourd'hui). */
export function isTripTravelDatePast(trip: {
  type?: string;
  data?: TravelsTripData;
}): boolean {
  const endMs = tripTravelEndMs(trip);
  if (endMs == null) return false;
  return endMs < todayStartMs();
}

const TRIP_PURGE_AFTER_MS = 365 * 24 * 60 * 60 * 1000;

/** À retirer de l'index : séjour terminé depuis plus d'un an. */
export function isTripEligibleForPurge(trip: {
  type?: string;
  data?: TravelsTripData;
}): boolean {
  const endMs = tripTravelEndMs(trip);
  if (endMs == null) return false;
  return endMs + TRIP_PURGE_AFTER_MS < todayStartMs();
}

/** Séjours les plus lointains en premier ; dossiers sans date en fin de liste. */
export function compareTripsByTravelDate(
  a: { data?: TravelsTripData; createdAt?: string },
  b: { data?: TravelsTripData; createdAt?: string },
): number {
  const ta = tripTravelStartMs(a);
  const tb = tripTravelStartMs(b);
  if (ta != null && tb != null && ta !== tb) return tb - ta;
  if (ta != null && tb == null) return -1;
  if (ta == null && tb != null) return 1;
  return new Date(b.createdAt || 0).getTime() - new Date(a.createdAt || 0).getTime();
}

export type TripReminder = {
  id: string;
  tripId: string;
  type:
    | "cuisine_j15"
    | "effectif_j7"
    | "blocked_status"
    | "transport_pending"
    | "bus_liste_j3"
    | "com_parents_j0"
    | "parent_meeting";
  label: string;
  severity: "info" | "warning" | "urgent";
  daysUntil?: number;
  href?: string;
};

export function computeTripReminders(trip: TravelsTrip): TripReminder[] {
  const out: TripReminder[] = [];
  const days = daysUntilTrip(trip.data);
  const status = trip.status;
  const sent = trip.data.remindersSent || {};

  if (status === "ANNULE" || status === "REJETE" || status === "SEANCE_ANNULEE") return out;

  if (trip.data.piqueNiqueDetails?.active && trip.data.cuisineOrderSentAt && days != null && days <= 15 && days >= 0) {
    out.push({
      id: `${trip.id}_cuisine_j15`,
      tripId: trip.id,
      type: "cuisine_j15",
      label: `J-${days} : affiner la liste élèves/adultes pour la cuisine (24h avant = facturation)`,
      severity: days <= 3 ? "urgent" : days <= 7 ? "warning" : "info",
      daysUntil: days,
    });
  }

  if (days != null && days <= 14 && days >= 0 && status === "VALIDE") {
    const cal = trip.data.parentCalendar || defaultParentCalendarFromTrip(trip.data);
    if (!calendarHasDepotAndRecuperation(cal)) {
      out.push({
        id: `${trip.id}_parent_meeting`,
        tripId: trip.id,
        type: "parent_meeting",
        label: `J-${days} : vérifier dépôt / récupération parents (calendrier .ics)`,
        severity: days <= 3 ? "urgent" : "warning",
        daysUntil: days,
        href: `/travels/${trip.id}?tab=eleves`,
      });
    }
  }

  if (days != null && days <= 7 && days >= 0 && status === "VALIDE") {
    out.push({
      id: `${trip.id}_effectif_j7`,
      tripId: trip.id,
      type: "effectif_j7",
      label: `J-${days} : vérifier l'effectif définitif et relancer transport/cuisine si besoin`,
      severity: days <= 2 ? "urgent" : "warning",
      daysUntil: days,
    });
  }

  if (
    complexNeedsBus(trip) &&
    trip.data.pendingAmendedQuote &&
    !["VALIDE", "REJETE", "ANNULE"].includes(status)
  ) {
    out.push({
      id: `${trip.id}_transport_pending`,
      tripId: trip.id,
      type: "transport_pending",
      label: "Avenant transport envoyé — en attente du nouveau devis",
      severity: "warning",
    });
  }

  const blockedStatuses = ["BESOIN_MODIFICATION", "EN_ATTENTE_DIR_INITIAL", "PROF_LOGISTICS"];
  if (blockedStatuses.includes(status) && days != null && days <= 14 && days >= 0) {
    out.push({
      id: `${trip.id}_blocked`,
      tripId: trip.id,
      type: "blocked_status",
      label: `Dossier encore en « ${status} » — sortie dans ${days} jour(s)`,
      severity: days <= 5 ? "urgent" : "warning",
      daysUntil: days,
    });
  }

  const listeConfirmed =
    trip.data.listeElevesStatus === "confirmed" && (trip.data.participantEleves?.length || 0) > 0;

  if (
    !listeConfirmed &&
    days != null &&
    days <= 4 &&
    days >= 0 &&
    (status === "VALIDE" || complexNeedsBus(trip)) &&
    !sent.bus_liste_j3
  ) {
    const busBit = complexNeedsBus(trip) ? " (envoi transporteur)" : "";
    out.push({
      id: `${trip.id}_bus_liste_j3`,
      tripId: trip.id,
      type: "bus_liste_j3",
      label: `J-${days} : confirmez la liste des élèves et les horaires dépôt / reprise${busBit}`,
      severity: days <= 1 ? "urgent" : "warning",
      daysUntil: days,
      href: `/travels/${trip.id}?tab=eleves`,
    });
  }

  if (
    listeConfirmed &&
    days != null &&
    days === 0 &&
    !sent.com_parents_j0
  ) {
    out.push({
      id: `${trip.id}_com_parents_j0`,
      tripId: trip.id,
      type: "com_parents_j0",
      label: "Jour J : vous pouvez communiquer aux parents (messages + photos)",
      severity: "info",
      daysUntil: 0,
      href: `/travels/${trip.id}?tab=communication`,
    });
  }

  return out;
}

/** Normalise une requête / un champ pour recherche (casse + accents). */
export function normalizeTravelsSearchText(raw: string): string {
  return String(raw || "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

/**
 * Texte indexé pour la liste des dossiers : titre, lieu, prof responsable,
 * accompagnateurs (prénom/nom).
 */
export function travelsTripSearchHaystack(trip: TravelsTrip): string {
  const data = trip.data;
  const accompagnateurNames = (data?.accompagnateurs || [])
    .map((a) => String(a?.name || "").trim())
    .filter(Boolean);
  return normalizeTravelsSearchText(
    [
      data?.title,
      data?.destination,
      trip.ownerName,
      data?.nomsAccompagnateurs,
      ...accompagnateurNames,
      data?.classes,
    ]
      .filter(Boolean)
      .join(" "),
  );
}

/** Chaque mot de la requête doit apparaître quelque part (titre, lieu, prof…). */
export function travelsTripMatchesSearch(trip: TravelsTrip, query: string): boolean {
  const needle = normalizeTravelsSearchText(query);
  if (!needle) return true;
  const haystack = travelsTripSearchHaystack(trip);
  const tokens = needle.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return true;
  return tokens.every((token) => haystack.includes(token));
}

function positiveIntOrNull(raw: unknown): number | null {
  if (raw == null || raw === "") return null;
  const n = typeof raw === "number" ? raw : Number(String(raw).replace(/\s/g, "").replace(",", "."));
  if (!Number.isFinite(n) || n < 0) return null;
  return Math.round(n);
}

function positiveEuroOrNull(raw: unknown): number | null {
  if (raw == null || raw === "") return null;
  if (typeof raw === "number" && Number.isFinite(raw)) return Math.round(raw * 100) / 100;
  const s = String(raw)
    .trim()
    .replace(/\u00a0/g, " ")
    .replace(/\s/g, "")
    .replace(/€/gi, "")
    .replace(",", ".");
  if (!s) return null;
  const n = Number(s);
  if (!Number.isFinite(n) || n < 0) return null;
  return Math.round(n * 100) / 100;
}

/** Effectif élèves pour la liste d'accueil (dossier + fiche compta + liste nominative). */
export function travelsListNbEleves(trip: TravelsTrip): number | null {
  const fromData = positiveIntOrNull(trip.data?.nbEleves);
  if (fromData != null && fromData > 0) return fromData;

  const sheet = trip.data?.comptaSheet;
  const fromCompta = positiveIntOrNull(sheet?.nbEleves);
  if (fromCompta != null && fromCompta > 0) return fromCompta;

  const fromFactures = positiveIntOrNull(sheet?.nbElevesFactures);
  if (fromFactures != null && fromFactures > 0) return fromFactures;

  const participants = trip.data?.participantEleves;
  if (Array.isArray(participants) && participants.length > 0) return participants.length;

  return fromData;
}

export type TravelsListBudget = {
  amount: number | null;
  kind: "previsionnel" | "valide";
};

/**
 * Budget carte liste : prévisionnel (`coutTotal`) tant que la compta n'a pas validé ;
 * total dépenses compta / `finalTotalCost` après validation.
 */
export function travelsListBudget(trip: TravelsTrip): TravelsListBudget {
  const sheet = trip.data?.comptaSheet;
  const validated = Boolean(
    sheet?.budgetValidatedAt ||
      (trip.data?.finalTotalCost != null && String(trip.data.finalTotalCost).trim() !== ""),
  );

  if (validated) {
    const amount =
      positiveEuroOrNull(trip.data?.finalTotalCost) ??
      positiveEuroOrNull(sheet?.depensesTotal) ??
      null;
    return { amount, kind: "valide" };
  }

  return {
    amount: positiveEuroOrNull(trip.data?.coutTotal),
    kind: "previsionnel",
  };
}
