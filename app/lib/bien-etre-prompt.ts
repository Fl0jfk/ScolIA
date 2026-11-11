import type { BienEtreSessionAnalysis } from "@/app/lib/bien-etre-types";

export const BIEN_ETRE_OFF_TOPIC_REPLY =
  "Je suis là pour t'écouter sur ce qui te pèse au quotidien ou à l'école, pas pour les devoirs ou les cours. Si tu as besoin d'aide scolaire, parle à un professeur ou à tes parents. Tu veux me dire ce qui te tracasse vraiment ?";

const OFF_TOPIC_PATTERN =
  /\b(devoirs?|dm\b|examen|contrôle|bac\b|brevet|maths?|français|physique|chimie|histoire|géo|dissertation|rédaction|exercice|corrigé|triche|chatgpt|copier)\b/i;

export function bienEtreSystemPrompt(): string {
  return [
    "Tu es un assistant bien-être pour des élèves de collège et lycée en France.",
    "Ton rôle : écoute bienveillante, validation des émotions, questions ouvertes courtes.",
    "Tu peux parler de la vie courante, des relations, du stress, de la solitude, de la famille.",
    "Tu ne fais PAS : devoirs, cours, triche, diagnostics médicaux, conseils de traitement.",
    "Si harcèlement, violence, mal-être grave ou idées noires : reste calme, propose le signalement au psychologue de l'établissement (bouton dédié), rappelle le 119 (Allô enfance en danger, 24h/24), le 3114 (souffrance psychique, risque suicidaire) et le 3018 (harcèlement).",
    "Ne force jamais un signalement. Réponses courtes (2-4 phrases), tutoiement respectueux.",
    "Ne prétends pas être humain. Ne stocke rien — rappelle que la conversation disparaît si on quitte la page.",
  ].join("\n");
}

export function isOffTopicUserMessage(message: string): boolean {
  return OFF_TOPIC_PATTERN.test(message);
}

export function analysisFromClassifier(raw: unknown): BienEtreSessionAnalysis {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const distress = Math.max(0, Math.min(10, Number(o.distressLevel) || 0));
  const categories = Array.isArray(o.categories)
    ? o.categories.map(String).filter(Boolean).slice(0, 6)
    : [];
  return {
    offTopic: o.offTopic === true,
    distressLevel: distress,
    suggestSignalement: o.suggestSignalement === true || distress >= 7,
    categories,
  };
}

export function classifierPrompt(userMessage: string, assistantReply: string): string {
  return [
    "Analyse ce tour de conversation élève / bot bien-être.",
    'Réponds UNIQUEMENT en JSON : {"offTopic":bool,"distressLevel":0-10,"suggestSignalement":bool,"categories":["..."]}',
    "categories possibles : detresse, harcelement, isolement, famille, stress_scolaire, idees_noires, vie_courante, autre",
    "suggestSignalement = true si harcèlement, danger, détresse forte ou besoin d'adulte référent.",
    `Message élève:\n${userMessage}`,
    `Réponse bot:\n${assistantReply}`,
  ].join("\n\n");
}

export function severityFromDistress(distress: number): "low" | "medium" | "high" {
  if (distress >= 8) return "high";
  if (distress >= 5) return "medium";
  return "low";
}
