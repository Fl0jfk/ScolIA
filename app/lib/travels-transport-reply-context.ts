/** Contexte / récap pour les e-mails « répondre au transporteur ». Safe client & serveur. */

export type TransportReplyQuoteLike = {
  id?: string | null;
  providerName?: string | null;
  extractedCompany?: string | null;
  extractedPrice?: string | null;
  price?: string | null;
  extractedReference?: string | null;
  reference?: string | null;
  originalFilename?: string | null;
  providerEmail?: string | null;
  extractedContactEmail?: string | null;
  email?: string | null;
  fromEmail?: string | null;
  subject?: string | null;
  fileUrl?: string | null;
};

export type TransportReplyTripLike = {
  id: string;
  data?: {
    title?: string | null;
    destination?: string | null;
    date?: string | null;
    startDate?: string | null;
    endDate?: string | null;
    startTime?: string | null;
    endTime?: string | null;
    classes?: string | null;
    transportRequest?: Record<string, unknown> | null;
    selectedBusQuote?: TransportReplyQuoteLike | null;
    transportEmailMessages?: Array<{
      fromEmail?: string | null;
      toEmail?: string | null;
      subject?: string | null;
      summary?: string | null;
      details?: string | null;
    }> | null;
  } | null;
  receivedDevis?: TransportReplyQuoteLike[] | null;
};

export type TransportReplyRecap = {
  tripId: string;
  title: string | null;
  destination: string | null;
  pickupAddress: string | null;
  datesLabel: string | null;
  timesLabel: string | null;
  classes: string | null;
  providerLabel: string | null;
  amountTtc: string | null;
  quoteReference: string | null;
  lines: string[];
  textBlock: string;
  subjectSuffix: string;
};

const REF_PATTERNS: RegExp[] = [
  /(?:devis|offre|facture|proforma)\s*(?:n[°ºo.]|num[eé]ro|ref(?:[eé]rence)?|réf\.?)?\s*[:\s#]*([A-Z0-9][A-Z0-9._/\-]{2,})/i,
  /(?:n[°ºo.]|num[eé]ro|ref(?:[eé]rence)?|réf\.?)\s*(?:devis|offre)?\s*[:\s#]*([A-Z0-9][A-Z0-9._/\-]{2,})/i,
];

function cleanStr(v: unknown): string | null {
  if (v == null) return null;
  const s = String(v).trim();
  return s.length > 0 ? s : null;
}

function formatFrDate(iso: string | null | undefined): string | null {
  const raw = cleanStr(iso);
  if (!raw) return null;
  const d = new Date(raw.length <= 10 ? `${raw}T12:00:00` : raw);
  if (Number.isNaN(d.getTime())) return raw;
  return d.toLocaleDateString("fr-FR", {
    weekday: "short",
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

/** Normalise une référence devis OCR / saisie. */
export function normalizeQuoteReference(raw: unknown): string | null {
  const s = cleanStr(raw);
  if (!s || s.toLowerCase() === "null") return null;
  return s.replace(/^[\s#:.-]+/, "").slice(0, 80);
}

/** Cherche un n° de devis dans un texte libre (objet mail, OCR, résumé IA…). */
export function guessQuoteReferenceFromText(...chunks: Array<string | null | undefined>): string | null {
  const blob = chunks.map((c) => cleanStr(c) || "").filter(Boolean).join("\n");
  if (!blob) return null;
  for (const re of REF_PATTERNS) {
    const m = blob.match(re);
    if (m?.[1]) {
      const ref = normalizeQuoteReference(m[1]);
      if (ref && !/^(ttc|ht|eur|euro|euros)$/i.test(ref)) return ref;
    }
  }
  return null;
}

export function findTransportQuoteForReply(
  trip: TransportReplyTripLike,
  quoteId?: string | null,
): TransportReplyQuoteLike | null {
  const id = cleanStr(quoteId);
  const list = Array.isArray(trip.receivedDevis) ? trip.receivedDevis : [];
  if (id) {
    const hit = list.find((q) => cleanStr(q?.id) === id);
    if (hit) return hit;
    const selected = trip.data?.selectedBusQuote;
    if (selected && cleanStr(selected.id) === id) return selected;
  }
  const selected = trip.data?.selectedBusQuote;
  if (selected && (selected.providerName || selected.extractedPrice || selected.fileUrl)) {
    return selected;
  }
  return null;
}

function resolveQuoteReference(
  trip: TransportReplyTripLike,
  quote: TransportReplyQuoteLike | null,
): string | null {
  if (!quote) return null;
  const direct =
    normalizeQuoteReference(quote.extractedReference) ||
    normalizeQuoteReference(quote.reference);
  if (direct) return direct;

  const fromQuoteText = guessQuoteReferenceFromText(
    quote.originalFilename,
    quote.subject,
    quote.providerName,
    quote.extractedCompany,
  );
  if (fromQuoteText) return fromQuoteText;

  const emailHints = (Array.isArray(trip.data?.transportEmailMessages)
    ? trip.data!.transportEmailMessages!
    : []
  ).filter((m) => {
    const from = cleanStr(m.fromEmail)?.toLowerCase() || "";
    const to = cleanStr(m.toEmail)?.toLowerCase() || "";
    const providers = [
      quote.providerEmail,
      quote.extractedContactEmail,
      quote.email,
      quote.fromEmail,
    ]
      .map((e) => cleanStr(e)?.toLowerCase())
      .filter(Boolean) as string[];
    if (providers.length === 0) return false;
    return providers.some((p) => from === p || to === p || from.includes(p) || p.includes(from));
  });

  for (const m of emailHints) {
    const guessed = guessQuoteReferenceFromText(m.subject, m.summary, m.details);
    if (guessed) return guessed;
  }
  return null;
}

function pickupFromTrip(trip: TransportReplyTripLike): string | null {
  const tr = trip.data?.transportRequest;
  if (!tr || typeof tr !== "object") return null;
  return cleanStr(tr.pickupPoint) || cleanStr(tr.address) || cleanStr(tr.lieu);
}

function datesLabelFromTrip(trip: TransportReplyTripLike): string | null {
  const start = formatFrDate(trip.data?.startDate || trip.data?.date);
  const end = formatFrDate(trip.data?.endDate);
  if (start && end && trip.data?.endDate && trip.data.endDate !== (trip.data.startDate || trip.data.date)) {
    return `Du ${start} au ${end}`;
  }
  return start;
}

function timesLabelFromTrip(trip: TransportReplyTripLike): string | null {
  const a = cleanStr(trip.data?.startTime)?.slice(0, 5) || null;
  const b = cleanStr(trip.data?.endTime)?.slice(0, 5) || null;
  if (a && b) return `${a} → ${b}`;
  return a || b;
}

/**
 * Construit le récap joint automatiquement aux e-mails transporteur.
 * Inclut réf. dossier, dates, adresse, montant TTC et n° devis quand disponibles.
 */
export function buildTransportReplyRecap(
  trip: TransportReplyTripLike,
  quoteId?: string | null,
): TransportReplyRecap {
  const quote = findTransportQuoteForReply(trip, quoteId);
  const tripId = String(trip.id);
  const title = cleanStr(trip.data?.title);
  const destination = cleanStr(trip.data?.destination);
  const pickupAddress = pickupFromTrip(trip);
  const datesLabel = datesLabelFromTrip(trip);
  const timesLabel = timesLabelFromTrip(trip);
  const classes = cleanStr(trip.data?.classes);
  const providerLabel =
    cleanStr(quote?.extractedCompany) || cleanStr(quote?.providerName) || null;
  const amountTtc =
    cleanStr(quote?.extractedPrice) || cleanStr(quote?.price) || null;
  const quoteReference = resolveQuoteReference(trip, quote);

  const lines: string[] = [];
  lines.push(`Réf. dossier ScolIA : ${tripId}`);
  if (title) lines.push(`Projet : ${title}`);
  if (destination) lines.push(`Destination : ${destination}`);
  if (pickupAddress) lines.push(`Lieu de RDV / départ : ${pickupAddress}`);
  if (datesLabel) lines.push(`Date(s) : ${datesLabel}`);
  if (timesLabel) lines.push(`Horaires (dossier) : ${timesLabel}`);
  if (classes) lines.push(`Classes : ${classes}`);
  if (providerLabel) lines.push(`Transporteur / devis : ${providerLabel}`);
  if (amountTtc) lines.push(`Montant proposé (TTC) : ${amountTtc}`);
  if (quoteReference) lines.push(`N° / réf. devis : ${quoteReference}`);

  const textBlock = [
    "——— Contexte sortie (rappel automatique) ———",
    ...lines,
    "———————————————————————————————",
  ].join("\n");

  const subjectParts = [destination || title || "sortie"].filter(Boolean);
  if (quoteReference) subjectParts.push(`devis ${quoteReference}`);
  else subjectParts.push(`réf. ${tripId}`);

  return {
    tripId,
    title,
    destination,
    pickupAddress,
    datesLabel,
    timesLabel,
    classes,
    providerLabel,
    amountTtc,
    quoteReference,
    lines,
    textBlock,
    subjectSuffix: subjectParts.join(" — "),
  };
}

export function composeTransportReplyBody(userMessage: string, recap: TransportReplyRecap): string {
  const msg = String(userMessage || "").trim();
  if (!msg) return recap.textBlock;
  return `${msg}\n\n${recap.textBlock}`;
}
