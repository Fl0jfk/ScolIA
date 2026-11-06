/**
 * Recherche web de ScolIA via l'outil intégré `web_search` de Mistral (API Conversations,
 * bêta). Exposée au modèle comme un outil « savoir » : il formule une requête générale,
 * le serveur interroge Mistral, renvoie un résumé et les sources.
 *
 * Données personnelles : une requête part vers un moteur de recherche externe. Elle est
 * refusée si elle contient un e-mail, un téléphone, une étiquette de pseudonyme ([Élève 1])
 * ou le nom d'une personne connue de l'établissement (contrôle fait par l'appelant).
 * Conversation non conservée chez Mistral (`store: false`).
 */

export const WEB_SEARCH_TOOL_SCHEMA = {
  type: "function" as const,
  function: {
    name: "web_search",
    description:
      "SAVOIR — recherche sur le web (actualité, textes officiels, Éducation nationale, académie, informations pratiques ou générales). " +
      "La requête doit être GÉNÉRALE : jamais de nom d'élève, de parent ou de collègue, ni d'information personnelle. " +
      "Cite ensuite les sources renvoyées.",
    parameters: {
      type: "object" as const,
      properties: {
        query: { type: "string", description: "Requête générale, en français de préférence." },
      },
      required: ["query"],
      additionalProperties: false,
    },
  },
};

export type WebSource = { title: string; url: string };
export type WebSearchResult =
  | { ok: true; text: string; sources: WebSource[] }
  | { ok: false; error: string; code: string };

const MAX_QUERY = 300;

/** Refus des requêtes contenant des données personnelles évidentes. */
export function webQueryProblem(query: string): string | null {
  const q = String(query || "").trim();
  if (q.length < 3) return "Requête trop courte.";
  if (q.length > MAX_QUERY) return "Requête trop longue : reformule en quelques mots-clés.";
  if (/[\w.+-]+@[\w-]+\.[\w.]+/.test(q)) return "Requête refusée : elle contient une adresse e-mail.";
  if (/(\+33|0)\s?[1-9](?:[\s.-]?\d{2}){4}/.test(q)) return "Requête refusée : elle contient un numéro de téléphone.";
  if (/\[\s*(élève|eleve|parent|personnel|famille)\b/i.test(q)) {
    return "Requête refusée : elle désigne une personne. Reformule sans nom ni personne.";
  }
  return null;
}

/** Lecture tolérante de la réponse Conversations (format documenté via les SDK seulement). */
export function parseConversationOutput(data: unknown): { text: string; sources: WebSource[] } {
  const outputs = Array.isArray((data as { outputs?: unknown })?.outputs)
    ? ((data as { outputs: unknown[] }).outputs)
    : [];
  let text = "";
  const sources: WebSource[] = [];
  const seen = new Set<string>();
  for (const entry of outputs) {
    const e = entry as { type?: string; content?: unknown };
    if (e?.type !== "message.output") continue;
    if (typeof e.content === "string") {
      text += e.content;
      continue;
    }
    if (!Array.isArray(e.content)) continue;
    for (const chunk of e.content) {
      const c = chunk as { type?: string; text?: string; url?: string; title?: string };
      if (c?.type === "text" && typeof c.text === "string") text += c.text;
      if (c?.type === "tool_reference" && typeof c.url === "string" && /^https:\/\//.test(c.url)) {
        if (seen.has(c.url)) continue;
        seen.add(c.url);
        sources.push({ title: String(c.title || c.url).slice(0, 140), url: c.url });
      }
    }
  }
  return { text: text.trim(), sources: sources.slice(0, 8) };
}

export async function runMistralWebSearch(
  query: string,
  apiKey: string,
  fetchImpl: typeof fetch = fetch,
): Promise<WebSearchResult> {
  const problem = webQueryProblem(query);
  if (problem) return { ok: false, error: problem, code: "INVALID_QUERY" };
  try {
    const res = await fetchImpl("https://api.mistral.ai/v1/conversations", {
      method: "POST",
      signal: AbortSignal.timeout(45_000),
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({
        model: "mistral-small-latest",
        inputs:
          `Recherche sur le web et réponds en français, de façon factuelle et sourcée, ` +
          `en 10 lignes maximum : ${query.trim()}`,
        tools: [{ type: "web_search" }],
        store: false,
      }),
    });
    if (!res.ok) {
      console.warn("[brain-ai/web_search] HTTP", res.status);
      return { ok: false, error: "Recherche web indisponible pour le moment.", code: "WEB_UNAVAILABLE" };
    }
    const parsed = parseConversationOutput(await res.json());
    if (!parsed.text) {
      return { ok: false, error: "La recherche web n'a rien donné d'exploitable.", code: "WEB_EMPTY" };
    }
    return { ok: true, ...parsed };
  } catch (err) {
    console.warn("[brain-ai/web_search] échec", (err as Error)?.name);
    return { ok: false, error: "Recherche web indisponible pour le moment.", code: "WEB_UNAVAILABLE" };
  }
}

/**
 * Recherche avec contrôle des noms connus de l'établissement : si la requête en contient,
 * elle n'est pas envoyée.
 */
export async function guardedWebSearch(
  query: string,
  apiKey: string,
  etablissementId: string | null | undefined,
): Promise<WebSearchResult> {
  if (etablissementId) {
    const { pseudonymizeForLlm } = await import("@/app/lib/brain-ai/pseudonymize");
    const p = await pseudonymizeForLlm(etablissementId, [query]);
    if (p.maskedCount > 0) {
      return {
        ok: false,
        error: "Requête refusée : elle contient le nom d'une personne de l'établissement. Reformule de façon générale.",
        code: "PERSONAL_DATA",
      };
    }
  }
  return runMistralWebSearch(query, apiKey);
}
