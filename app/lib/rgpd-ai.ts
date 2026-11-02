import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { getEnabledPlatformDpas } from "@/app/lib/rgpd-platform-dpas";
import type { RgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import type { RgpdProcessingFicheDef } from "@/app/lib/rgpd-processing-fiches";
import type {
  RgpdDataBreachFields,
  RgpdIncidentKind,
  RgpdQuestionnaireAnswers,
  RgpdSecurityIncidentFields,
} from "@/app/lib/rgpd-types";

/** Critères Art. 30 pour analyser une fiche de traitement importée. */
export const RGPD_FICHE_ANALYSIS_CHECKLIST = [
  "Finalité(s) du traitement clairement énoncée(s)",
  "Base légale identifiée",
  "Catégories de personnes concernées",
  "Catégories de données personnelles",
  "Destinataires / sous-traitants",
  "Durée(s) de conservation",
  "Mesures de sécurité techniques et organisationnelles",
  "Responsable du traitement identifié",
] as const;

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";
const MISTRAL_MODELS = ["mistral-small-latest", "mistral-medium"] as const;
const RETRYABLE_STATUS = new Set([429, 500, 502, 503, 504]);

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function mistralUserFacingError(status: number, body: string): Error {
  if (status === 429) {
    return new Error(
      "Service d'analyse temporairement saturé. Réessayez dans quelques instants.",
    );
  }
  if (status === 503 || status === 502 || status === 504 || status === 500) {
    return new Error(
      "Service d'analyse temporairement indisponible (Mistral). Réessayez dans une minute.",
    );
  }
  return new Error(`Analyse IA impossible (${status}) : ${body.slice(0, 160)}`);
}

async function mistralJson<T>(prompt: string): Promise<T> {
  const key = await getMistralApiKey();
  if (!key) throw new Error("Clé Mistral non configurée.");

  let lastStatus = 0;
  let lastBody = "";

  for (const model of MISTRAL_MODELS) {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const res = await fetch(MISTRAL_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${key}`,
        },
        body: JSON.stringify({
          model,
          temperature: 0.1,
          response_format: { type: "json_object" },
          messages: [{ role: "user", content: prompt }],
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const content = data.choices?.[0]?.message?.content;
        if (!content) throw new Error("Réponse Mistral vide.");
        return JSON.parse(content) as T;
      }

      lastStatus = res.status;
      lastBody = await res.text();
      const retryable = RETRYABLE_STATUS.has(res.status);
      if (!retryable) throw mistralUserFacingError(res.status, lastBody);
      if (attempt < 2) await sleep(400 * (attempt + 1));
    }
  }

  throw mistralUserFacingError(lastStatus, lastBody);
}

type RgpdAnalysisResult = {
  documentScore: number;
  presentCriteria: string[];
  missingCriteria: string[];
  improvements: string[];
  suggestedActions: string[];
};

type RgpdAnalyzableDoc = {
  title: string;
  checklist: readonly string[];
  description?: string;
};

export function toRgpdAnalyzableFromCatalog(entry: RgpdCatalogEntry): RgpdAnalyzableDoc {
  return {
    title: entry.title,
    checklist: entry.checklist,
    description: entry.description,
  };
}

export function toRgpdAnalyzableFromFiche(
  fiche: RgpdProcessingFicheDef,
): RgpdAnalyzableDoc {
  return {
    title: fiche.title,
    checklist: RGPD_FICHE_ANALYSIS_CHECKLIST,
    description: fiche.description,
  };
}

export async function analyzeRgpdDocumentWithAi(input: {
  entry: RgpdCatalogEntry | RgpdAnalyzableDoc;
  documentText: string;
  answers: RgpdQuestionnaireAnswers;
}): Promise<RgpdAnalysisResult> {
  const title = input.entry.title;
  const checklistItems =
    "checklist" in input.entry ? input.entry.checklist : [];
  const checklist = checklistItems.join("\n- ");
  const description =
    "description" in input.entry && input.entry.description
      ? `\nContexte du document attendu : ${input.entry.description}`
      : "";
  const prompt = `Tu es expert RGPD pour établissements scolaires français.
Analyse le document fourni par rapport au type attendu : "${title}".${description}
Checklist attendue :
- ${checklist}

Contexte établissement (résumé questionnaire) :
- Types : ${input.answers.establishmentKinds.join(", ") || "non précisé"}
- DPD désigné : ${input.answers.dpdDesignated ? "oui" : "non"}
- Sous-traitants : ENT=${input.answers.subprocessors.ent}, Microsoft=${input.answers.subprocessors.microsoft365}, AWS=${input.answers.subprocessors.aws}, Mistral=${input.answers.subprocessors.mistralAi}
- DPA plateformes : ${getEnabledPlatformDpas(input.answers).map((p) => p.name).join(", ") || "aucun"}

Texte du document (extrait) :
"""
${input.documentText.slice(0, 12_000)}
"""

Réponds en JSON strict :
{
  "documentScore": number entre 0 et 100,
  "presentCriteria": ["critères couverts"],
  "missingCriteria": ["critères manquants ou insuffisants"],
  "improvements": ["suggestions concrètes"],
  "suggestedActions": ["actions prioritaires"]
}`;

  const raw = await mistralJson<RgpdAnalysisResult>(prompt);
  return {
    documentScore: Math.max(0, Math.min(100, Math.round(raw.documentScore ?? 0))),
    presentCriteria: raw.presentCriteria ?? [],
    missingCriteria: raw.missingCriteria ?? [],
    improvements: raw.improvements ?? [],
    suggestedActions: raw.suggestedActions ?? [],
  };
}

type RgpdIncidentChatResult = {
  assistantMessage: string;
  title: string;
  fields: RgpdDataBreachFields | RgpdSecurityIncidentFields;
  complete: boolean;
};

export async function chatRgpdIncidentWithAi(input: {
  kind: RgpdIncidentKind;
  message: string;
  history: Array<{ role: "user" | "assistant"; content: string }>;
  currentFields: RgpdDataBreachFields | RgpdSecurityIncidentFields;
}): Promise<RgpdIncidentChatResult> {
  const isBreach = input.kind === "violation_donnees";
  const fieldSchema = isBreach
    ? `{
  "discoveredAt": "ISO date ou texte",
  "nature": "description nature violation",
  "dataCategories": "catégories données",
  "affectedCount": number ou null,
  "severity": "faible|moyenne|elevee",
  "immediateMeasures": "mesures prises",
  "cnilNotificationRequired": boolean,
  "cnilNotificationPlannedAt": "date si applicable",
  "dataSubjectsNotified": boolean,
  "responsibleName": "nom",
  "timeline": "chronologie",
  "description": "résumé"
}`
    : `{
  "occurredAt": "date/heure",
  "incidentType": "type",
  "impactedSystems": "systèmes",
  "potentialDataImpact": "données potentiellement concernées",
  "containmentMeasures": "mesures",
  "providerContacted": "prestataire",
  "status": "ouvert|en_cours|clos",
  "description": "résumé"
}`;

  const historyText = input.history
    .slice(-8)
    .map((m) => `${m.role}: ${m.content}`)
    .join("\n");

  const prompt = `Tu aides à remplir une fiche d'incident RGPD pour un établissement scolaire français.
Type : ${isBreach ? "VIOLATION DE DONNÉES PERSONNELLES" : "INCIDENT DE SÉCURITÉ INFORMATIQUE"}.

Champs actuels :
${JSON.stringify(input.currentFields, null, 2)}

Historique :
${historyText}

Nouveau message utilisateur :
${input.message}

Extrais ou mets à jour les champs. Pose une question de clarification si une info critique manque.
Réponds en JSON :
{
  "assistantMessage": "réponse en français, professionnelle et rassurante",
  "title": "titre court de l'incident",
  "fields": ${fieldSchema},
  "complete": boolean (true si les champs essentiels sont remplis)
}`;

  const raw = await mistralJson<RgpdIncidentChatResult>(prompt);
  return {
    assistantMessage: raw.assistantMessage || "Merci pour ces précisions.",
    title: raw.title || (isBreach ? "Violation de données" : "Incident de sécurité"),
    fields: { ...input.currentFields, ...(raw.fields ?? {}) },
    complete: Boolean(raw.complete),
  };
}
