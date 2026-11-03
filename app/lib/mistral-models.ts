/**
 * Modèles Mistral utilisés par l'appli.
 *
 * - Small 4 : appels simples (classement, routage, reformulation, rapprochement).
 * - Large 4 : extraction difficile et vision (pièces d'inscription, RH, conventions,
 *   signatures, RGPD).
 *
 * Large 4 réfléchit toujours avant de répondre : `message.content` arrive alors sous
 * forme de liste (bloc « thinking » + bloc « text »). Toujours lire la réponse avec
 * `mistralMessageText`. Il refuse aussi `temperature: 0` sans `top_p: 1`.
 */
export const MISTRAL_SMALL_MODEL = "mistral-small-2603";
export const MISTRAL_LARGE_MODEL = "mistral-large-4";

/** Options à ajouter à chaque appel Large 4 (top_p requis à température 0, réflexion courte). */
export const MISTRAL_LARGE_OPTIONS = { top_p: 1, reasoning_effort: "low" } as const;

/** Options propres au modèle (vide pour Small 4). */
export function mistralModelOptions(model: string): Record<string, unknown> {
  return model === MISTRAL_LARGE_MODEL ? { ...MISTRAL_LARGE_OPTIONS } : {};
}

/** Texte final d'une réponse chat Mistral, sans les blocs de réflexion. */
export function mistralMessageText(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((part) => {
      if (!part || typeof part !== "object") return "";
      const p = part as { type?: unknown; text?: unknown };
      return p.type === "text" && typeof p.text === "string" ? p.text : "";
    })
    .join("");
}
