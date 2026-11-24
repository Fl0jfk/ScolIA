/**
 * Page des familles (circuit v2) — modèle d'aperçu construit depuis la trame fixe
 * (travels-family-page-template.ts) et les champs du dossier. Aucun texte libre, aucun appel IA.
 * Ce modèle servira aussi au PDF d'engagement du lot 2.
 */

import type {
  TravelsFamilyDocKind,
  TravelsFamilyDocRequest,
  TravelsFamilyPage,
  TravelsParentCalendar,
  TravelsPriceBlock,
  TravelsTrip,
} from "@/app/lib/travels-types";
import { FAMILY_PAGE_TEMPLATE, fillFamilyTemplate } from "@/app/lib/travels-family-page-template";
import { getFamilyMeetingPoints, tripDepartureIso, tripReturnIso, type MeetingPoint } from "@/app/lib/travels-v2";

/** Petite étiquette neutre de l'aperçu (pas de bandeau « projet » : le professeur ne diffuse rien). */
export const FAMILY_PAGE_PREVIEW_LABEL = "Aperçu";

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
  classes: string;
  organizer: string;
  imageUrl: string | null;
  dateLine: string;
  intro: string[];
  outbound: MeetingPoint | null;
  inbound: MeetingPoint | null;
  program: string;
  equipment: string[];
  cash: { amount: number; reason: string } | null;
  mealsRule: string[];
  documents: FamilyPagePreviewDoc[];
  deadline: string | null;
  price: {
    pricePerStudent: number;
    installments: { dueDate: string; amount: number }[];
    paymentText: string;
  } | null;
  financialHelp: string;
  dataRetention: string;
  opened: boolean;
  openedAt: string | null;
  previewLabel: string | null;
};

const KIND_LABELS: Record<TravelsFamilyDocKind, string> = {
  avant_signature: "Obligatoire avant de signer",
  facultative: "Facultative",
  avant_depart: "À fournir avant le départ",
};

export function familyDocKindLabel(kind: TravelsFamilyDocKind): string {
  return KIND_LABELS[kind] ?? kind;
}

export function dateFr(iso: string | null | undefined): string {
  if (!iso || !/^\d{4}-\d{2}-\d{2}$/.test(iso)) return String(iso || "");
  return `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}`;
}

export function buildFamilyPagePreviewModel(
  trip: Pick<TravelsTrip, "data" | "imageUrl" | "ownerName">,
  override?: {
    familyPage?: Partial<TravelsFamilyPage>;
    priceBlock?: TravelsPriceBlock | null;
    parentCalendar?: TravelsParentCalendar;
  },
): FamilyPagePreviewModel {
  const d = trip.data || {};
  const page = { ...(d.familyPage || {}), ...(override?.familyPage || {}) } as Partial<TravelsFamilyPage>;
  const priceSource = override && "priceBlock" in override ? override.priceBlock : d.priceBlock;
  const start = tripDepartureIso(d);
  const end = tripReturnIso(d);
  const dateLine = start
    ? end && end !== start
      ? `du ${dateFr(start)} au ${dateFr(end)}`
      : `le ${dateFr(start)}`
    : "";
  const { outbound, inbound } = getFamilyMeetingPoints({ parentCalendar: override?.parentCalendar ?? d.parentCalendar });

  const title = String(d.title || "Voyage scolaire");
  const destination = String(d.destination || "");
  const classes = String(d.classes || "");
  const organizer = String(trip.ownerName || "");
  const deadline = page.deadline ? String(page.deadline) : null;
  const vars = {
    titre: title,
    destination,
    dates: dateLine,
    classes,
    organisateur: organizer,
    etablissement: String(d.etablissement || ""),
    dateLimite: deadline ? dateFr(deadline) : "",
  };
  const intro = FAMILY_PAGE_TEMPLATE.intro.map((p) => fillFamilyTemplate(p, vars));

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

  const cash =
    page.cash && Number(page.cash.amount) > 0 && String(page.cash.reason || "").trim()
      ? { amount: Number(page.cash.amount), reason: String(page.cash.reason).trim() }
      : null;

  const openedAt = page.openedAt ? String(page.openedAt) : null;
  return {
    title,
    destination,
    classes,
    organizer,
    imageUrl: trip.imageUrl || (typeof d.imageUrl === "string" ? d.imageUrl : null) || null,
    dateLine,
    intro,
    outbound,
    inbound,
    program: String(page.program || "").trim(),
    equipment: (page.equipment || []).map((x) => String(x).trim()).filter(Boolean),
    cash,
    mealsRule: [...FAMILY_PAGE_TEMPLATE.meals],
    documents,
    deadline,
    price,
    financialHelp: FAMILY_PAGE_TEMPLATE.financialHelp,
    dataRetention: FAMILY_PAGE_TEMPLATE.dataRetention,
    opened: Boolean(openedAt),
    openedAt,
    previewLabel: openedAt ? null : FAMILY_PAGE_PREVIEW_LABEL,
  };
}
