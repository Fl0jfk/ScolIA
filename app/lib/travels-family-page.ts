/**
 * Page des familles (circuit v2) — fonctions pures : prompt du brouillon IA et modèle d'aperçu
 * partagé entre l'écran (aperçu) et l'export PDF. Aucun prix n'est écrit par l'organisateur.
 */

import type {
  TravelsFamilyDocKind,
  TravelsFamilyDocRequest,
  TravelsFamilyPage,
  TravelsPriceBlock,
  TravelsTrip,
} from "@/app/lib/travels-types";
import { HEALTH_DOC_FORBIDDEN_MESSAGE, tripDepartureIso, tripReturnIso } from "@/app/lib/travels-v2";

export const FINANCIAL_DIFFICULTY_SENTENCE =
  "En cas de difficulté financière, n’hésitez pas à contacter votre secrétariat afin de rencontrer la direction.";

export const FAMILY_PAGE_PROJECT_BANNER = "PROJET — non validé, ne pas diffuser";

/** Retire le balisage Markdown que le modèle pourrait renvoyer malgré la consigne. */
export function stripMarkdown(text: string): string {
  return String(text || "")
    .replace(/\r\n/g, "\n")
    .replace(/^#{1,6}\s*/gm, "")
    .replace(/\*\*(.*?)\*\*/g, "$1")
    .replace(/[*_`]/g, "")
    .replace(/^\s*[-•]\s+/gm, "• ")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

/** Garantit la phrase sur les difficultés financières (ajoutée en fin de texte si absente). */
export function ensureFinancialDifficultySentence(text: string): string {
  const t = String(text || "").trim();
  if (/difficult[ée]s?\s+financi[èe]re/i.test(t)) return t;
  return t ? `${t}\n\n${FINANCIAL_DIFFICULTY_SENTENCE}` : FINANCIAL_DIFFICULTY_SENTENCE;
}

export function buildFamilyPageSystemPrompt(): string {
  return [
    "Tu es un assistant de communication scolaire. Tu rédiges le texte d'une page web destinée aux parents d'élèves, pour présenter un voyage ou une sortie scolaire.",
    "CONSIGNES :",
    "1. N'invente rien : appuie-toi uniquement sur les informations du dossier fournies.",
    "2. Garde ce qui est utile aux parents : objectifs pédagogiques, activités, hébergement ou lieu, horaires et lieu de rendez-vous, équipement à prévoir, repas.",
    "3. INTERDIT : mentionner un prix, un montant, un coût, un tarif, une somme en euros, ou des échéances de paiement. Le prix est ajouté séparément par la comptabilité.",
    "4. INTERDIT : demander un document ou une information liée à la santé (fiche sanitaire, certificat médical, ordonnance, vaccins, allergies, traitement, PAI). Ne parle pas de santé.",
    "5. N'ajoute pas de liste de pièces à fournir : elle est gérée à part.",
    `6. Termine par cette phrase, telle quelle : « ${FINANCIAL_DIFFICULTY_SENTENCE} »`,
    "7. TON : chaleureux, clair, rassurant, professionnel. Vouvoiement.",
    "8. FORMAT : texte brut en paragraphes aérés. Pas de Markdown (ni #, ni *), pas de texte entre crochets.",
  ].join("\n");
}

export function buildFamilyPageUserPrompt(trip: Pick<TravelsTrip, "ownerName" | "data">): string {
  const d = trip.data || {};
  const start = tripDepartureIso(d);
  const end = tripReturnIso(d);
  const dates = start ? (end && end !== start ? `du ${start} au ${end}` : `le ${start}`) : "à préciser";
  const rdv =
    String((d.transportRequest as { pickupPoint?: string } | undefined)?.pickupPoint || "").trim() || "à préciser";
  const cuisine = d.piqueNiqueDetails && (d.piqueNiqueDetails as { active?: boolean }).active
    ? "Repas fournis par la cantine (pique-nique ou self) prévus."
    : "Pas de commande cantine prévue.";
  return [
    "Rédige le texte de la page des familles à partir de ce dossier (le prix est volontairement exclu) :",
    `- Titre : ${d.title || "—"}`,
    `- Établissement : ${d.etablissement || "—"}`,
    `- Classes concernées : ${d.classes || "—"}`,
    `- Objectifs pédagogiques : ${d.objectifs || "—"}`,
    `- Destination : ${d.destination || "—"}`,
    `- Dates : ${dates}`,
    `- Heure de rendez-vous : ${d.startTime || "à préciser"} — heure de retour prévue : ${d.endTime || "à préciser"}`,
    `- Lieu de rendez-vous : ${rdv}`,
    `- Transport : ${d.needsBus ? "autocar" : "non précisé"}`,
    `- Restauration : ${cuisine}`,
    `- Organisateur : ${trip.ownerName || "l'enseignant"}`,
  ].join("\n");
}

/* ─── Modèle d'aperçu (écran + PDF) ─── */

export type FamilyPagePreviewDoc = {
  id: string;
  label: string;
  kind: TravelsFamilyDocKind;
  kindLabel: string;
  dueDate?: string;
  templateName?: string;
};

export type FamilyPagePreviewModel = {
  title: string;
  destination: string;
  imageUrl: string | null;
  dateLine: string;
  meetingLines: string[];
  paragraphs: string[];
  documents: FamilyPagePreviewDoc[];
  deadline: string | null;
  price: {
    pricePerStudent: number;
    installments: { dueDate: string; amount: number }[];
    paymentText: string;
  } | null;
  /** Bandeau « PROJET » : tant que la page n'est pas ouverte aux familles. */
  isProject: boolean;
  projectBanner: string | null;
  openedAt: string | null;
  hasText: boolean;
};

const KIND_LABELS: Record<TravelsFamilyDocKind, string> = {
  avant_signature: "Obligatoire avant de signer",
  facultative: "Facultative",
  avant_depart: "À fournir avant le départ",
};

export function familyDocKindLabel(kind: TravelsFamilyDocKind): string {
  return KIND_LABELS[kind] ?? kind;
}

function dateFr(iso: string | null | undefined): string {
  if (!iso || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return String(iso || "");
  return `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}`;
}

export function buildFamilyPagePreviewModel(
  trip: Pick<TravelsTrip, "data" | "imageUrl">,
  override?: { familyPage?: Partial<TravelsFamilyPage>; priceBlock?: TravelsPriceBlock | null },
): FamilyPagePreviewModel {
  const d = trip.data || {};
  const page = { ...(d.familyPage || {}), ...(override?.familyPage || {}) } as Partial<TravelsFamilyPage>;
  const priceSource = override && "priceBlock" in override ? override.priceBlock : d.priceBlock;
  const start = tripDepartureIso(d);
  const end = tripReturnIso(d);
  const dateLine = start
    ? end && end !== start
      ? `Du ${dateFr(start)} au ${dateFr(end)}`
      : `Le ${dateFr(start)}`
    : "Dates à préciser";

  const meetingLines: string[] = [];
  const cal = d.parentCalendar;
  const points = Array.isArray(cal?.points) ? cal.points : [];
  for (const p of points) {
    if (!p?.date) continue;
    const label = p.label || (p.kind === "depot" ? "Rendez-vous / départ" : p.kind === "recuperation" ? "Retour / récupération" : "Point");
    meetingLines.push(
      `${label} : ${dateFr(p.date)}${p.time ? ` à ${p.time}` : ""}${p.place ? ` — ${p.place}` : ""}`,
    );
  }
  if (meetingLines.length === 0 && start) {
    const pickup = String((d.transportRequest as { pickupPoint?: string } | undefined)?.pickupPoint || "").trim();
    meetingLines.push(
      `Rendez-vous : ${dateFr(start)}${d.startTime ? ` à ${d.startTime}` : ""}${pickup ? ` — ${pickup}` : ""}`,
    );
    if (end) meetingLines.push(`Retour prévu : ${dateFr(end)}${d.endTime ? ` à ${d.endTime}` : ""}`);
  }

  const text = String(page.text || "").trim();
  const paragraphs = text
    ? text
        .replace(/\r\n/g, "\n")
        .split(/\n{2,}/)
        .map((p) => p.trim())
        .filter(Boolean)
    : [];

  const docs: TravelsFamilyDocRequest[] = Array.isArray(page.documents) ? (page.documents as TravelsFamilyDocRequest[]) : [];
  const order: Record<TravelsFamilyDocKind, number> = { avant_signature: 0, avant_depart: 1, facultative: 2 };
  const documents: FamilyPagePreviewDoc[] = [...docs]
    .sort((a, b) => order[a.kind] - order[b.kind])
    .map((x) => ({
      id: x.id,
      label: x.label,
      kind: x.kind,
      kindLabel: familyDocKindLabel(x.kind),
      dueDate: x.kind === "avant_depart" ? x.dueDate : undefined,
      templateName: x.template?.name,
    }));

  const price =
    priceSource && Number.isFinite(Number(priceSource.pricePerStudent))
      ? {
          pricePerStudent: Number(priceSource.pricePerStudent),
          installments: (priceSource.installments || []).map((i) => ({ dueDate: i.dueDate, amount: Number(i.amount) })),
          paymentText: String(priceSource.paymentText || ""),
        }
      : null;

  const openedAt = page.openedAt ? String(page.openedAt) : null;
  return {
    title: String(d.title || "Voyage scolaire"),
    destination: String(d.destination || ""),
    imageUrl: trip.imageUrl || (typeof d.imageUrl === "string" ? d.imageUrl : null) || null,
    dateLine,
    meetingLines,
    paragraphs,
    documents,
    deadline: page.deadline ? String(page.deadline) : null,
    price,
    isProject: !openedAt,
    projectBanner: openedAt ? null : FAMILY_PAGE_PROJECT_BANNER,
    openedAt,
    hasText: paragraphs.length > 0,
  };
}

export const FAMILY_PAGE_HEALTH_REMINDER = HEALTH_DOC_FORBIDDEN_MESSAGE;
