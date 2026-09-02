import type { TripCandidateForMatch } from "@/app/lib/travel-devis-ocr";
import type { TravelsTrip } from "@/app/lib/travels-types";

/** Séjour déjà commandé / signé : inutile d’y rattacher un nouveau devis. */
export function tripAlreadyHasSignedBusQuote(trip: {
  status?: string | null;
  data?: Record<string, unknown> | null;
}): boolean {
  const d = trip.data || {};
  if (typeof d.signedQuoteUrl === "string" && d.signedQuoteUrl.trim()) return true;
  if (typeof d.signedQuoteS3Key === "string" && d.signedQuoteS3Key.trim()) return true;
  const status = String(trip.status || "");
  if (
    status === "EN_ATTENTE_COMPTA" ||
    status === "EN_ATTENTE_DIR_FINAL" ||
    status === "VALIDE"
  ) {
    if (d.selectedBusQuote && typeof d.selectedBusQuote === "object") return true;
  }
  return false;
}

const FR_MONTHS: Record<string, string> = {
  janvier: "01",
  fevrier: "02",
  février: "02",
  mars: "03",
  avril: "04",
  mai: "05",
  juin: "06",
  juillet: "07",
  aout: "08",
  août: "08",
  septembre: "09",
  octobre: "10",
  novembre: "11",
  decembre: "12",
  décembre: "12",
};

function pad2(n: string): string {
  return n.padStart(2, "0");
}

/** Extrait des dates ISO (YYYY-MM-DD) depuis objet / corps / OCR. */
export function extractIsoDatesFromText(text: string): string[] {
  const raw = text || "";
  const found = new Set<string>();

  for (const m of raw.matchAll(/\b(20\d{2})-(\d{2})-(\d{2})\b/g)) {
    found.add(`${m[1]}-${m[2]}-${m[3]}`);
  }
  for (const m of raw.matchAll(/\b(\d{1,2})[./](\d{1,2})[./](20\d{2})\b/g)) {
    found.add(`${m[3]}-${pad2(m[2]!)}-${pad2(m[1]!)}`);
  }
  for (const m of raw.matchAll(
    /\b(\d{1,2})\s+(janvier|f[eé]vrier|mars|avril|mai|juin|juillet|ao[uû]t|septembre|octobre|novembre|d[eé]cembre)\s+(20\d{2})\b/gi,
  )) {
    const monthKey = m[2]!.toLowerCase();
    const mm = FR_MONTHS[monthKey];
    if (mm) found.add(`${m[3]}-${mm}-${pad2(m[1]!)}`);
  }

  return [...found];
}

function tripDateSet(c: TripCandidateForMatch): Set<string> {
  const out = new Set<string>();
  const start = (c.startDate || "").slice(0, 10);
  const end = (c.endDate || start).slice(0, 10);
  if (/^\d{4}-\d{2}-\d{2}$/.test(start)) out.add(start);
  if (/^\d{4}-\d{2}-\d{2}$/.test(end)) out.add(end);
  return out;
}

function normalizeTripIdToken(raw: string): string {
  const digits = raw.replace(/\D/g, "");
  return digits;
}

/** Réf. explicite type « trip-… » / « ref trip … » dans le mail. */
export function extractExplicitTripIdHints(text: string): string[] {
  const hints: string[] = [];
  for (const m of text.matchAll(/\btrip[-_\s]?(\d{10,20})\b/gi)) {
    hints.push(`trip-${m[1]}`);
  }
  for (const m of text.matchAll(/\bref\s+trip\s+(\d{10,20})\b/gi)) {
    hints.push(`trip-${m[1]}`);
  }
  return [...new Set(hints)];
}

function digitDistance(a: string, b: string): number {
  if (a === b) return 0;
  const max = Math.max(a.length, b.length);
  if (Math.abs(a.length - b.length) > 2) return max;
  const dp: number[][] = Array.from({ length: a.length + 1 }, () =>
    Array.from({ length: b.length + 1 }, () => 0),
  );
  for (let i = 0; i <= a.length; i++) dp[i]![0] = i;
  for (let j = 0; j <= b.length; j++) dp[0]![j] = j;
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      dp[i]![j] = Math.min(
        dp[i - 1]![j]! + 1,
        dp[i]![j - 1]! + 1,
        dp[i - 1]![j - 1]! + cost,
      );
    }
  }
  return dp[a.length]![b.length]!;
}

function resolveExplicitTripId(
  hints: string[],
  candidates: TripCandidateForMatch[],
): string | null {
  const byId = new Map(candidates.map((c) => [c.id, c]));
  for (const h of hints) {
    if (byId.has(h)) return h;
  }
  for (const h of hints) {
    const hd = normalizeTripIdToken(h);
    let best: { id: string; dist: number } | null = null;
    for (const c of candidates) {
      const cd = normalizeTripIdToken(c.id);
      const dist = digitDistance(hd, cd);
      if (dist <= 2 && (!best || dist < best.dist)) best = { id: c.id, dist };
    }
    if (best) return best.id;
  }
  return null;
}

function tokenizePlace(s: string): string[] {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/[^a-z0-9\s]/g, " ")
    .split(/\s+/)
    .filter((w) => w.length >= 4);
}

function destinationScore(text: string, c: TripCandidateForMatch): number {
  const hay = tokenizePlace(`${c.title} ${c.destination} ${c.transportContext || ""}`);
  if (!hay.length) return 0;
  const needle = new Set(tokenizePlace(text));
  let hit = 0;
  for (const w of hay) if (needle.has(w)) hit++;
  return hit;
}

export type DeterministicMatchResult = {
  tripId: string | null;
  confidence: "high" | "medium" | "low" | null;
  motif: string | null;
  /** Si non null, restreindre l’IA à ces ids (dates correspondantes). */
  dateFilteredCandidateIds: string[] | null;
};

/**
 * Matching déterministe avant / après IA.
 * Priorité : id explicite → date unique → date + destination.
 */
export function resolveTripMatchDeterministic(input: {
  subject: string;
  bodyPlain?: string;
  snippet?: string;
  ocrText?: string;
  candidates: TripCandidateForMatch[];
}): DeterministicMatchResult {
  const { candidates } = input;
  if (!candidates.length) {
    return { tripId: null, confidence: null, motif: null, dateFilteredCandidateIds: null };
  }

  const blob = [input.subject, input.bodyPlain || "", input.snippet || "", input.ocrText || ""].join(
    "\n",
  );

  const explicit = resolveExplicitTripId(extractExplicitTripIdHints(blob), candidates);
  if (explicit) {
    return {
      tripId: explicit,
      confidence: "high",
      motif: `Référence séjour détectée dans le mail (${explicit}).`,
      dateFilteredCandidateIds: null,
    };
  }

  const mailDates = extractIsoDatesFromText(blob);
  if (!mailDates.length) {
    return { tripId: null, confidence: null, motif: null, dateFilteredCandidateIds: null };
  }

  const byDate = candidates.filter((c) => {
    const ds = tripDateSet(c);
    return mailDates.some((d) => ds.has(d));
  });

  if (byDate.length === 1) {
    return {
      tripId: byDate[0]!.id,
      confidence: "high",
      motif: `Date du devis (${mailDates.join(", ")}) unique parmi les séjours actifs.`,
      dateFilteredCandidateIds: [byDate[0]!.id],
    };
  }

  if (byDate.length > 1) {
    const ranked = [...byDate]
      .map((c) => ({ c, score: destinationScore(blob, c) }))
      .sort((a, b) => b.score - a.score);
    const top = ranked[0]!;
    const second = ranked[1];
    if (top.score >= 2 && (!second || top.score > second.score)) {
      return {
        tripId: top.c.id,
        confidence: "high",
        motif: `Date (${mailDates.join(", ")}) + destination/titre (score ${top.score}).`,
        dateFilteredCandidateIds: byDate.map((c) => c.id),
      };
    }
    return {
      tripId: null,
      confidence: null,
      motif: null,
      dateFilteredCandidateIds: byDate.map((c) => c.id),
    };
  }

  return { tripId: null, confidence: null, motif: null, dateFilteredCandidateIds: null };
}

export function candidateFromTrip(t: TravelsTrip): TripCandidateForMatch {
  const d = t.data || {};
  const classes = Array.isArray(d.classes) ? d.classes.join(", ") : String(d.classes || "");
  const tr =
    d.transportRequest && typeof d.transportRequest === "object"
      ? (d.transportRequest as Record<string, unknown>)
      : undefined;
  const parts: string[] = [];
  if (tr) {
    for (const key of [
      "departure",
      "arrival",
      "aller",
      "retour",
      "from",
      "to",
      "lieuDepart",
      "lieuArrivee",
      "freeText",
    ]) {
      const v = tr[key];
      if (typeof v === "string" && v.trim()) parts.push(v.trim().slice(0, 200));
    }
  }
  return {
    id: String(t.id),
    title: String(d.title || ""),
    destination: String(d.destination || ""),
    startDate: d.startDate || undefined,
    endDate: d.endDate || undefined,
    startTime: d.startTime || undefined,
    endTime: d.endTime || undefined,
    status: t.status || "",
    classes,
    etablissement: d.etablissement ? String(d.etablissement) : undefined,
    needsBus: Boolean(d.needsBus || d.transportRequest),
    nbEleves: d.nbEleves != null ? String(d.nbEleves) : undefined,
    transportContext: parts.length ? parts.join(" · ").slice(0, 400) : undefined,
  };
}
