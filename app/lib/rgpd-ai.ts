import "server-only";

import { getMistralApiKey } from "@/app/lib/tenant-config";
import { getEnabledPlatformDpas } from "@/app/lib/rgpd-platform-dpas";
import type { RgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import type {
  RgpdDataBreachFields,
  RgpdIncidentKind,
  RgpdQuestionnaireAnswers,
  RgpdSecurityIncidentFields,
} from "@/app/lib/rgpd-types";

const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

async function mistralJson<T>(prompt: string): Promise<T> {
  const key = await getMistralApiKey();
  if (!key) throw new Error("Clé Mistral non configurée.");

  const res = await fetch(MISTRAL_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${key}`,
    },
    body: JSON.stringify({
      model: "mistral-small-latest",
      temperature: 0.1,
      response_format: { type: "json_object" },
      messages: [{ role: "user", content: prompt }],
    }),
  });

  if (!res.ok) {
    const err = await res.text();
    throw new Error(`Mistral : ${res.status} ${err.slice(0, 200)}`);
  }

  const data = await res.json();
  const content = data.choices?.[0]?.message?.content;
  if (!content) throw new Error("Réponse Mistral vide.");
  return JSON.parse(content) as T;
}

type RgpdAnalysisResult = {
  documentScore: number;
  presentCriteria: string[];
  missingCriteria: string[];
  improvements: string[];
  suggestedActions: string[];
};

export async function analyzeRgpdDocumentWithAi(input: {
  entry: RgpdCatalogEntry;
  documentText: string;
  answers: RgpdQuestionnaireAnswers;
}): Promise<RgpdAnalysisResult> {
  const checklist = input.entry.checklist.join("\n- ");
  const prompt = `Tu es expert RGPD pour établissements scolaires français.
Analyse le document fourni par rapport au type attendu : "${input.entry.title}".
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
