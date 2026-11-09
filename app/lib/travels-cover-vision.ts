import "server-only";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { MISTRAL_SMALL_MODEL, mistralMessageText } from "@/app/lib/mistral-models";
import { parseTravelCoverVisionPick } from "@/app/lib/travels-cover-pick";

export type TravelCoverVisionCandidate = {
  /** Miniature (≈ 512 px) déjà téléchargée. */
  bytes: Buffer;
  contentType: string;
};

/**
 * Mistral regarde les miniatures trouvées et garde la photo qui montre vraiment le séjour.
 * Seuls le titre et le lieu du séjour sont envoyés (aucune donnée d'élève).
 * Coût : quelques milliers de jetons par séjour (bien moins d'un centime).
 *
 * Renvoie l'index choisi, null si aucune ne convient, "unavailable" si la vérification
 * n'a pas pu se faire (pas de clé, erreur réseau, réponse illisible).
 */
export async function pickTravelCoverWithVision(opts: {
  title: string;
  destination: string;
  candidates: TravelCoverVisionCandidate[];
}): Promise<number | null | "unavailable"> {
  if (opts.candidates.length === 0) return null;
  const apiKey = await getMistralApiKey();
  if (!apiKey) return "unavailable";

  const prompt =
    `Tu choisis la photo de couverture d'une sortie scolaire.\n` +
    `TITRE : "${opts.title}"\nLIEU : "${opts.destination || "non renseigné"}"\n\n` +
    `Voici ${opts.candidates.length} images numérotées dans l'ordre (1 à ${opts.candidates.length}).\n` +
    `Garde la PHOTO RÉELLE qui montre le mieux le lieu précis ou l'activité du séjour :\n` +
    `- si un lieu connu est nommé (monument, château, ville, site), l'image doit montrer CE lieu, ` +
    `pas un lieu qui lui ressemble ;\n` +
    `- refuse : plans, cartes, schémas, logos, affiches, captures d'écran, dessins, texte en gros, ` +
    `photos floues, sombres ou d'archive en noir et blanc, intérieurs sans intérêt, ` +
    `portraits de personnes, tout ce qui est militaire, violent ou choquant ;\n` +
    `- préfère une belle vue d'ensemble, lumineuse, cadrée en paysage.\n` +
    `Si aucune ne convient vraiment, réponds NONE.\n` +
    `Réponds UNIQUEMENT en JSON : {"best": <numéro ou "NONE">, "why": "une phrase"}`;

  const content: Array<
    { type: "text"; text: string } | { type: "image_url"; image_url: { url: string } }
  > = [{ type: "text", text: prompt }];
  for (const c of opts.candidates) {
    const mime = c.contentType.startsWith("image/") ? c.contentType : "image/jpeg";
    content.push({
      type: "image_url",
      image_url: { url: `data:${mime};base64,${c.bytes.toString("base64")}` },
    });
  }

  try {
    const response = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: MISTRAL_SMALL_MODEL,
        temperature: 0,
        response_format: { type: "json_object" },
        messages: [{ role: "user", content }],
      }),
      signal: AbortSignal.timeout(25_000),
    });
    if (!response.ok) {
      console.error("[travels-cover-vision] HTTP", response.status);
      return "unavailable";
    }
    const data = (await response.json()) as {
      choices?: Array<{ message?: { content?: unknown } }>;
    };
    const raw = mistralMessageText(data.choices?.[0]?.message?.content);
    const pick = parseTravelCoverVisionPick(raw, opts.candidates.length);
    console.info("[travels-cover-vision] pick", {
      title: opts.title,
      destination: opts.destination,
      count: opts.candidates.length,
      raw: raw.slice(0, 200),
    });
    return pick === undefined ? "unavailable" : pick;
  } catch (err) {
    console.error("[travels-cover-vision] failed", err);
    return "unavailable";
  }
}
