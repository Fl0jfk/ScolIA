import { getMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForPdfBytes, runTextractForS3Key } from "@/app/lib/ocr-textract";
import { detectAllSignatureZones, signatureBBoxToPdfLibCoords } from "@/app/lib/pdf-signature-vision";

/** OCR synchrone sur bytes PDF — délègue à Mistral OCR via la façade. */
export async function ocrPdfBytes(pdfBytes: Buffer | Uint8Array): Promise<string> {
  const result = await runTextractForPdfBytes(pdfBytes);
  if (!result.text.trim()) throw new Error("Mistral OCR : texte vide");
  return result.text;
}

/**
 * OCR d'une clé S3 — délègue à Mistral OCR via la façade.
 * Le paramètre `bucket` n'est plus requis par la façade (elle récupère le bucket en interne)
 * mais on le conserve pour ne pas casser les signatures existantes.
 */
export async function ocrS3Key(_bucket: string, key: string): Promise<string> {
  const result = await runTextractForS3Key(key);
  return result.text;
}

type SignatureFieldBBoxNormalized = {
  pageNumber: number;
  left: number;
  top: number;
  width: number;
  height: number;
};

/**
 * Détecte TOUTES les zones de signature client/direction d'un devis (multi-page).
 * Utilise Mistral Vision (pixtral-large) + rasterisation page + filtres anti faux-positifs.
 */
export async function findAllDevisSignatureZones(
  pdfBytes: Buffer | Uint8Array,
): Promise<SignatureFieldBBoxNormalized[]> {
  const apiKey = await getMistralApiKey();
  if (!apiKey) return [];

  try {
    const zones = await detectAllSignatureZones(pdfBytes, apiKey, { mode: "devis" });
    return zones.map((z) => ({
      pageNumber: z.pageNumber,
      left: z.left,
      top: z.top,
      width: z.width,
      height: z.height,
    }));
  } catch (err) {
    console.error("[travel-devis-ocr] findAllDevisSignatureZones:", err);
    return [];
  }
}

/**
 * Compat legacy : une seule BBox (meilleure zone = score le plus haut / dernière page).
 * Préférer findAllDevisSignatureZones pour signer tous les devis d'un PDF.
 */
async function findSignatureFieldBBoxFromTextract(
  pdfBytes: Buffer | Uint8Array,
): Promise<SignatureFieldBBoxNormalized | null> {
  const zones = await findAllDevisSignatureZones(pdfBytes);
  if (!zones.length) return null;
  return zones[zones.length - 1]!;
}

/** Délègue à signatureBBoxToPdfLibCoords — conservé pour rétro-compatibilité des appelants. */
export function textractSignatureBBoxToPdfLibDrawCoords(
  pageWidth: number,
  pageHeight: number,
  bbox: SignatureFieldBBoxNormalized,
  sigDrawWidth: number,
  sigDrawHeight: number,
  gapBelowLabelPt = 6,
): { x: number; y: number } {
  return signatureBBoxToPdfLibCoords(pageWidth, pageHeight, bbox, sigDrawWidth, sigDrawHeight, gapBelowLabelPt);
}

type DevisOcrMetadata = {
  price: string | null;
  company: string | null;
  contactEmail: string | null;
};

function normalizeContactEmail(v: string | null | undefined): string | null {
  if (v == null) return null;
  const s = String(v).trim();
  if (!s || s.toLowerCase() === "null") return null;
  const m = s.match(/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/);
  return m ? m[0].toLowerCase() : null;
}

export type TripCandidateForMatch = {
  id: string;
  title: string;
  destination: string;
  startDate?: string;
  endDate?: string;
  startTime?: string;
  endTime?: string;
  status?: string;
  classes?: string;
  etablissement?: string;
  needsBus?: boolean;
  nbEleves?: string;
  /** Extraits utiles de la demande transport (lieux, freeText…). */
  transportContext?: string;
};

type DevisOcrAndTripMatch = DevisOcrMetadata & {
  matchedTripId: string | null;
  matchConfidence: "high" | "medium" | "low" | null;
  matchMotif: string | null;
  suggestedTripId: string | null;
  matchReviewRequired: boolean;
};

export async function extractDevisMetadataWithMistral(ocrText: string): Promise<DevisOcrMetadata> {
  const mistralKey = await getMistralApiKey();
  if (!ocrText || !mistralKey) {
    return { price: null, company: null, contactEmail: null };
  }
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
              "Tu analyses le texte OCR d'un devis de transport. Réponds UNIQUEMENT avec un objet JSON valide, sans markdown, clés: montant_ttc (string ou null, ex: \"1 250,00 €\"), societe_emetrice (string ou null, nom de l'entreprise de transport qui émet le devis), email_contact (string ou null, une seule adresse e-mail professionnelle du service commercial / réservation visible sur le document, pas une URL). Si une info est absente ou incertaine, mets null.",
          },
          { role: "user", content: `Texte:\n${ocrText.slice(0, 4000)}` },
        ],
        temperature: 0.1,
        response_format: { type: "json_object" },
      }),
    });
    const data = (await res.json()) as Record<string, unknown>;
    if (!res.ok) {
      return { price: null, company: null, contactEmail: null };
    }
    const raw = (data.choices as { message?: { content?: string } }[] | undefined)?.[0]?.message?.content?.trim() || "";
    const parsed = JSON.parse(raw) as {
      montant_ttc?: string | null;
      societe_emetrice?: string | null;
      email_contact?: string | null;
    };
    const price = parsed.montant_ttc && String(parsed.montant_ttc).toLowerCase() !== "null" ? String(parsed.montant_ttc).trim() : null;
    const company = parsed.societe_emetrice && String(parsed.societe_emetrice).toLowerCase() !== "null" ? String(parsed.societe_emetrice).trim() : null;
    const contactEmail = normalizeContactEmail(parsed.email_contact);
    return { price: price || null, company: company || null, contactEmail };
  } catch {
    return { price: null, company: null, contactEmail: null };
  }
}

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

async function callMistralDevisTripMatch(
  ocrSlice: string,
  subj: string,
  snip: string,
  tripsJson: string
): Promise<{ ok: boolean; status: number; data: Record<string, unknown>; raw: string }> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) {
    return { ok: false, status: 0, data: { _reason: "missing_mistral_key" }, raw: "" };
  }
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
            "Tu es un assistant pour une école. On te donne le texte OCR d'un devis de transport, l'objet et un extrait d'email, et une liste JSON de voyages scolaires (id, titre, destination, dates, statut, classes). " +
            "Tu dois: (1) extraire montant_ttc et societe_emetrice du devis; (2) choisir UN SEUL id de voyage qui correspond au devis (destination, intitulé, dates, classes dans l'OCR ou l'email). " +
            "Si vraiment aucun voyage ne ressemble au devis, mets trip_id à null. " +
            "confidence: \"high\" = très sûr; \"medium\" = plausible, humain devra vérifier; \"low\" = hypothèse fragile mais tu peux quand même mettre le trip_id le plus plausible si un voyage domine légèrement — sinon null. " +
            "Réponds UNIQUEMENT avec un objet JSON: montant_ttc (string|null), societe_emetrice (string|null), email_contact (string|null, e-mail professionnel commercial/réservation visible sur le devis OCR, une seule adresse), trip_id (string|null, exactement un id de la liste ou null), confidence (\"high\"|\"medium\"|\"low\"), motif (string court en français).",
        },
        {
          role: "user",
          content: `Objet e-mail: ${subj}\nExtrait: ${snip}\n\nTexte OCR du devis:\n${ocrSlice}\n\nVoyages possibles (JSON):\n${tripsJson}`,
        },
      ],
      temperature: 0.15,
      response_format: { type: "json_object" },
    }),
  });
  const data = (await res.json()) as Record<string, unknown>;
  const choices = data.choices as { message?: { content?: string } }[] | undefined;
  const raw = choices?.[0]?.message?.content?.trim() || "";
  return { ok: res.ok, status: res.status, data, raw };
}

async function interpretMistralDevisMatchResponse(
  raw: string,
  candidates: TripCandidateForMatch[],
  ocrText: string
): Promise<DevisOcrAndTripMatch> {
  if (!raw) {
    const meta = await extractDevisMetadataWithMistral(ocrText);
    return {
      ...meta,
      matchedTripId: null,
      matchConfidence: null,
      matchMotif: "reponse_mistral_vide",
      suggestedTripId: null,
      matchReviewRequired: false,
    };
  }
  let parsed: {
    montant_ttc?: string | null;
    societe_emetrice?: string | null;
    email_contact?: string | null;
    trip_id?: string | null;
    confidence?: string;
    motif?: string | null;
  };
  try {
    parsed = JSON.parse(raw) as typeof parsed;
  } catch {
    const meta = await extractDevisMetadataWithMistral(ocrText);
    return {
      ...meta,
      matchedTripId: null,
      matchConfidence: null,
      matchMotif: "erreur_mistral_match",
      suggestedTripId: null,
      matchReviewRequired: false,
    };
  }

  const price =
    parsed.montant_ttc && String(parsed.montant_ttc).toLowerCase() !== "null"
      ? String(parsed.montant_ttc).trim()
      : null;
  const company =
    parsed.societe_emetrice && String(parsed.societe_emetrice).toLowerCase() !== "null"
      ? String(parsed.societe_emetrice).trim()
      : null;
  const contactEmail = normalizeContactEmail(parsed.email_contact);

  const allowedIds = new Set(candidates.map((c) => String(c.id)));
  const confRaw = String(parsed.confidence || "").toLowerCase();
  const tripRaw = parsed.trip_id != null ? String(parsed.trip_id).trim() : "";
  const tripOk = Boolean(tripRaw && allowedIds.has(tripRaw));
  const matchConfidence: "high" | "medium" | "low" | null =
    confRaw === "high" || confRaw === "medium" || confRaw === "low" ? confRaw : null;

  const matchedTripId: string | null = tripOk ? tripRaw : null;
  const matchReviewRequired = tripOk && matchConfidence !== "high";
  const suggestedTripId: string | null = !tripOk && tripRaw ? tripRaw : null;

  return {
    price: price || null,
    company: company || null,
    contactEmail,
    matchedTripId,
    matchConfidence,
    matchMotif: parsed.motif ? String(parsed.motif).slice(0, 500) : null,
    suggestedTripId,
    matchReviewRequired,
  };
}

async function extractDevisAndMatchTripWithMistral(
  ocrText: string,
  emailContext: { subject: string; snippet: string },
  candidates: TripCandidateForMatch[]
): Promise<DevisOcrAndTripMatch> {
  const empty: DevisOcrAndTripMatch = {
    price: null,
    company: null,
    contactEmail: null,
    matchedTripId: null,
    matchConfidence: null,
    matchMotif: null,
    suggestedTripId: null,
    matchReviewRequired: false,
  };
  if (!(await getMistralApiKey())) {
    return empty;
  }

  const ocrSlice = (ocrText || "").slice(0, 3500);
  const subj = (emailContext.subject || "").slice(0, 500);
  const snip = (emailContext.snippet || "").slice(0, 500);

  if (!candidates.length) {
    const meta = await extractDevisMetadataWithMistral(ocrText);
    return { ...empty, ...meta, matchMotif: "aucun_voyage_en_liste" };
  }

  const tripsJson = tripsJsonForMatch(candidates);

  try {
    const { ok, raw } = await callMistralDevisTripMatch(ocrSlice, subj, snip, tripsJson);
    if (!ok) {
      const meta = await extractDevisMetadataWithMistral(ocrText);
      return {
        ...meta,
        matchedTripId: null,
        matchConfidence: null,
        matchMotif: "erreur_http_mistral",
        suggestedTripId: null,
        matchReviewRequired: false,
      };
    }
    if (!raw) {
      const meta = await extractDevisMetadataWithMistral(ocrText);
      return {
        ...meta,
        matchedTripId: null,
        matchConfidence: null,
        matchMotif: "reponse_mistral_vide",
        suggestedTripId: null,
        matchReviewRequired: false,
      };
    }
    return await interpretMistralDevisMatchResponse(raw, candidates, ocrText);
  } catch {
    const meta = await extractDevisMetadataWithMistral(ocrText);
    return {
      ...meta,
      matchedTripId: null,
      matchConfidence: null,
      matchMotif: "erreur_mistral_match",
      suggestedTripId: null,
      matchReviewRequired: false,
    };
  }
}
