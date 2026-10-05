import "server-only";
import { getMistralApiKey } from "@/app/lib/tenant-config";
import { isSchoolSafeCoverText } from "@/app/lib/travels-image-catalog-helpers";

export type TravelCoverMistralIntent = {
  /** Requêtes image ordonnées (sens métier, adaptées école). */
  queries: string[];
  subject: "place" | "activity" | "both";
  /** Court résumé pour les logs. */
  rationale?: string;
};

function parseJsonObject(raw: string): Record<string, unknown> | null {
  const trimmed = String(raw || "").trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed) as Record<string, unknown>;
  } catch {
    const start = trimmed.indexOf("{");
    const end = trimmed.lastIndexOf("}");
    if (start >= 0 && end > start) {
      try {
        return JSON.parse(trimmed.slice(start, end + 1)) as Record<string, unknown>;
      } catch {
        return null;
      }
    }
    return null;
  }
}

function sanitizeQuery(raw: unknown): string | null {
  const q = String(raw || "")
    .replace(/[`"']/g, "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 80);
  if (q.length < 3) return null;
  if (!isSchoolSafeCoverText(q)) return null;
  return q;
}

/**
 * Mistral comprend le sens du séjour et propose des requêtes d’image
 * adaptées à une école (avant tout appel Wikimedia / Unsplash / catalogue).
 */
export async function proposeTravelCoverSearchQueries(opts: {
  title: string;
  destination: string;
}): Promise<TravelCoverMistralIntent | null> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey) return null;

  const title = String(opts.title || "").trim() || "Sortie scolaire";
  const destination = String(opts.destination || "").trim();

  try {
    const response = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${mistralKey}`,
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        temperature: 0.2,
        response_format: { type: "json_object" },
        messages: [
          {
            role: "system",
            content:
              `Tu choisis des requêtes de recherche d'IMAGE de couverture pour une SORTIE SCOLAIRE ` +
              `(élèves, établissement en France).\n` +
              `Tu dois d'abord COMPRENDRE le sens du titre + lieu, puis en déduire quoi illustrer.\n\n` +
              `Contexte : illustration positive, claire, adaptée aux familles / élèves.\n` +
              `INTERDIT absolument : guerre, militaire, armes, drones de combat (Shahed, Bayraktar, IAI Heron…), ` +
              `violence, politique, contenu choquant.\n` +
              `INTERDIT aussi : requêtes trop abstraites (« concours », « compétition », « édition 2027 ») ` +
              `qui ne montrent ni le lieu ni l'activité réelle.\n\n` +
              `Exemples de bon sens :\n` +
              `- « Concours de drone 2027 » à Houlgate → « drone loisir élèves », « adolescents drone camera », ` +
              `« quadcopter hobby », « Houlgate plage » (PAS « concours », PAS drone militaire).\n` +
              `- « Surf » à Lacanau → « surf océan », « Lacanau plage ».\n` +
              `- « Visite château » à Chambord → « Château de Chambord ».\n\n` +
              `Propose 3 à 5 requêtes courtes (2 à 6 mots), FR ou EN selon ce qui marche mieux sur Wikimedia/Unsplash.\n` +
              `Ordre : du plus pertinent au plus large.\n` +
              `Réponds UNIQUEMENT en JSON valide :\n` +
              `{"subject":"place"|"activity"|"both","queries":["...","..."],"rationale":"une phrase"}`,
          },
          {
            role: "user",
            content:
              `TITRE DE LA SORTIE : ${title}\n` +
              `LIEU / DESTINATION : ${destination || "(non renseigné)"}\n` +
              `Propose les requêtes d'image adaptées à une école.`,
          },
        ],
      }),
    });

    if (!response.ok) {
      console.error("[travels-cover-mistral] HTTP", response.status);
      return null;
    }

    const resData = (await response.json()) as {
      choices?: Array<{ message?: { content?: string } }>;
    };
    const raw = resData.choices?.[0]?.message?.content || "";
    const parsed = parseJsonObject(raw);
    if (!parsed) {
      console.warn("[travels-cover-mistral] JSON parse failed", raw.slice(0, 200));
      return null;
    }

    const queriesRaw = Array.isArray(parsed.queries) ? parsed.queries : [];
    const queries = queriesRaw
      .map(sanitizeQuery)
      .filter((q): q is string => Boolean(q))
      .filter((q, i, arr) => arr.findIndex((x) => x.toLowerCase() === q.toLowerCase()) === i)
      .slice(0, 6);

    if (queries.length === 0) return null;

    const subjectRaw = String(parsed.subject || "both").toLowerCase();
    const subject: TravelCoverMistralIntent["subject"] =
      subjectRaw === "place" || subjectRaw === "activity" ? subjectRaw : "both";

    const rationale = String(parsed.rationale || "")
      .trim()
      .slice(0, 240);

    console.info("[travels-cover-mistral] intent", {
      title,
      destination,
      subject,
      queries,
      rationale,
    });

    return { queries, subject, rationale: rationale || undefined };
  } catch (err) {
    console.error("[travels-cover-mistral] failed", err);
    return null;
  }
}
