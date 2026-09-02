import type { TripCandidateForMatch } from "@/app/lib/travel-devis-ocr";
import { getMistralApiKey } from "@/app/lib/tenant-config";

function normalizeContactEmail(v: string | null | undefined): string | null {
  if (!v || String(v).toLowerCase() === "null") return null;
  const s = String(v).trim();
  const m = s.match(/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/);
  return m ? m[0].toLowerCase() : null;
}

export type TravelEmailMessageType =
  | "devis_pdf"
  | "confirmation_commande"
  | "info_transport"
  | "reponse_generique"
  | "non_lie";

export type TransportEmailMessage = {
  id: string;
  gmailMessageId: string;
  fromEmail: string;
  subject: string;
  messageType: TravelEmailMessageType;
  summary: string;
  driverName?: string | null;
  driverPhone?: string | null;
  details?: string | null;
  pdfUrl?: string | null;
  s3KeyIncoming?: string | null;
  originalFilename?: string | null;
  receivedAt: string;
  matchConfidence?: string | null;
  matchMotif?: string | null;
  source?: "email";
};

export type TravelEmailAnalysis = {
  messageType: TravelEmailMessageType;
  matchedTripId: string | null;
  matchConfidence: "high" | "medium" | "low" | null;
  matchMotif: string | null;
  summary: string | null;
  driverName: string | null;
  driverPhone: string | null;
  confirmationDetails: string | null;
  price: string | null;
  company: string | null;
  contactEmail: string | null;
  suggestedTripId: string | null;
  matchReviewRequired: boolean;
};

function tripsJsonForMatch(candidates: TripCandidateForMatch[]): string {
  return JSON.stringify(
    candidates.map((c) => ({
      id: c.id,
      titre: c.title,
      destination: c.destination,
      etablissement: c.etablissement || "",
      dates: [c.startDate, c.endDate].filter(Boolean).join(" → "),
      horaires: [c.startTime, c.endTime].filter(Boolean).join(" – "),
      statut: c.status || "",
      classes: c.classes || "",
      besoin_bus: Boolean(c.needsBus),
      effectif_eleves: c.nbEleves || "",
      contexte_transport: c.transportContext || "",
    })),
  );
}

const EMPTY_ANALYSIS: TravelEmailAnalysis = {
  messageType: "non_lie",
  matchedTripId: null,
  matchConfidence: null,
  matchMotif: null,
  summary: null,
  driverName: null,
  driverPhone: null,
  confirmationDetails: null,
  price: null,
  company: null,
  contactEmail: null,
  suggestedTripId: null,
  matchReviewRequired: false,
};

function parseMessageType(raw: string): TravelEmailMessageType {
  const t = raw.toLowerCase();
  if (t === "devis_pdf" || t === "devis") return "devis_pdf";
  if (t === "confirmation_commande" || t === "confirmation") return "confirmation_commande";
  if (t === "info_transport" || t === "info") return "info_transport";
  if (t === "reponse_generique" || t === "reponse") return "reponse_generique";
  return "non_lie";
}

export async function analyzeTravelEmailWithMistral(input: {
  subject: string;
  snippet: string;
  bodyPlain?: string;
  ocrText?: string;
  fromEmail?: string;
  hasPdfAttachment?: boolean;
  candidates: TripCandidateForMatch[];
}): Promise<TravelEmailAnalysis> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) return { ...EMPTY_ANALYSIS, matchMotif: "missing_mistral_key" };

  const { subject, snippet, bodyPlain, ocrText, fromEmail, hasPdfAttachment, candidates } = input;
  if (!candidates.length) {
    return { ...EMPTY_ANALYSIS, matchMotif: "aucun_voyage_en_liste" };
  }

  const emailBody = (bodyPlain || snippet || "").slice(0, 6000);
  const ocrSlice = (ocrText || "").slice(0, 5000);
  const tripsJson = tripsJsonForMatch(candidates);

  try {
    const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${mistralKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        messages: [
          {
            role: "system",
            content:
              "Tu analyses des e-mails reçus par une école concernant des sorties / séjours scolaires (souvent transport bus). " +
              "On te donne l'objet, le corps du mail, éventuellement le texte OCR d'une pièce jointe PDF, et une liste JSON de séjours en cours " +
              "(id, titre, destination/lieux, dates, horaires, établissement, classes, besoin bus, effectif, contexte transport).\n" +
              "PRIORITÉ DE MATCHING (ordre strict):\n" +
              "1) DATE(S) du devis/mail vs dates du séjour — critère le plus fort (rare d'avoir 2 sorties le même jour).\n" +
              "2) Destination / lieux / titre (Caen, plages, musée, etc.).\n" +
              "3) Autres indices (classes, effectif, contexte transport).\n" +
              "Si une date du mail/OCR ne correspond à AUCUN séjour de la liste → trip_id null (ne force PAS un mauvais séjour).\n" +
              "Si une date correspond à un seul séjour → trip_id de ce séjour (high).\n" +
              "N'exige AUCUNE référence dossier / numéro interne. Si présente (ex. « ref trip … »), utilise-la si elle matche un id de la liste.\n" +
              "IMPORTANT: une pièce jointe PDF n'est PAS automatiquement un devis. Lis le contenu OCR pour décider.\n" +
              "Types possibles:\n" +
              "- devis_pdf: proposition commerciale / devis / offre de prix / facture proforma transport.\n" +
              "- confirmation_commande: confirmation de réservation ou de commande après accord.\n" +
              "- info_transport: chauffeur, téléphone, horaires définitifs, consignes.\n" +
              "- reponse_generique: question, demande de détails, réponse utile sans catégorie claire — à rattacher quand même au séjour.\n" +
              "- non_lie: spam ou hors sujet (trip_id null).\n" +
              "Tu dois:\n" +
              "1) Associer UN séjour (trip_id) de la liste, ou null si impossible sans inventer.\n" +
              "2) Classer le message (message_type) selon le CONTENU.\n" +
              "3) Résumer en 1-2 phrases (resume).\n" +
              "4) Extraire si présent: nom_chauffeur, telephone_chauffeur, details_confirmation, montant_ttc, societe_emetrice, email_contact.\n" +
              "confidence: high/medium/low. motif: courte explication en français (dates/lieux/titre qui ont convaincu).\n" +
              `Pièce jointe PDF détectée: ${hasPdfAttachment ? "oui — analyser l'OCR" : "non — s'appuyer sur objet et corps"}.\n` +
              "Réponds UNIQUEMENT en JSON: message_type, trip_id, confidence, motif, resume, nom_chauffeur, telephone_chauffeur, details_confirmation, montant_ttc, societe_emetrice, email_contact.",
          },
          {
            role: "user",
            content: `Expéditeur: ${fromEmail || "—"}\nObjet: ${subject.slice(0, 500)}\n\nCorps du mail:\n${emailBody}\n\n${
              ocrSlice ? `Texte OCR pièce jointe:\n${ocrSlice}\n\n` : ""
            }Séjours possibles:\n${tripsJson}`,
          },
        ],
        temperature: 0.15,
        response_format: { type: "json_object" },
      }),
    });

    if (!res.ok) {
      return { ...EMPTY_ANALYSIS, matchMotif: "erreur_http_mistral" };
    }

    const data = (await res.json()) as Record<string, unknown>;
    const raw =
      (data.choices as { message?: { content?: string } }[] | undefined)?.[0]?.message?.content?.trim() ||
      "";
    if (!raw) return { ...EMPTY_ANALYSIS, matchMotif: "reponse_mistral_vide" };

    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const allowedIds = new Set(candidates.map((c) => String(c.id)));
    const tripRaw = parsed.trip_id != null ? String(parsed.trip_id).trim() : "";
    const tripOk = Boolean(tripRaw && allowedIds.has(tripRaw));
    const confRaw = String(parsed.confidence || "").toLowerCase();
    const matchConfidence: "high" | "medium" | "low" | null =
      confRaw === "high" || confRaw === "medium" || confRaw === "low" ? confRaw : null;

    const messageType = parseMessageType(String(parsed.message_type || ""));

    return {
      messageType,
      matchedTripId: tripOk ? tripRaw : null,
      matchConfidence,
      matchMotif: parsed.motif ? String(parsed.motif).slice(0, 500) : null,
      summary: parsed.resume ? String(parsed.resume).slice(0, 1000) : null,
      driverName:
        parsed.nom_chauffeur && String(parsed.nom_chauffeur).toLowerCase() !== "null"
          ? String(parsed.nom_chauffeur).trim()
          : null,
      driverPhone:
        parsed.telephone_chauffeur && String(parsed.telephone_chauffeur).toLowerCase() !== "null"
          ? String(parsed.telephone_chauffeur).trim()
          : null,
      confirmationDetails:
        parsed.details_confirmation && String(parsed.details_confirmation).toLowerCase() !== "null"
          ? String(parsed.details_confirmation).slice(0, 2000)
          : null,
      price:
        parsed.montant_ttc && String(parsed.montant_ttc).toLowerCase() !== "null"
          ? String(parsed.montant_ttc).trim()
          : null,
      company:
        parsed.societe_emetrice && String(parsed.societe_emetrice).toLowerCase() !== "null"
          ? String(parsed.societe_emetrice).trim()
          : null,
      contactEmail: normalizeContactEmail(
        parsed.email_contact != null ? String(parsed.email_contact) : null,
      ),
      suggestedTripId: !tripOk && tripRaw ? tripRaw : null,
      matchReviewRequired: tripOk && matchConfidence !== "high",
    };
  } catch {
    return { ...EMPTY_ANALYSIS, matchMotif: "erreur_mistral_analyse" };
  }
}
