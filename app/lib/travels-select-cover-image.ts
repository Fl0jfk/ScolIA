import { getMistralApiKey } from "@/app/lib/tenant-config";
import { normalizePublicImageUrl } from "@/app/lib/scola-image";
import {
  buildTravelPlaceSearchQuery,
  findTravelCatalogByNormalizeKey,
  formatTravelImageAttribution,
  listTravelCatalogImages,
  normalizeTravelImageKey,
  rankTravelCatalogCandidates,
  scoreTravelCatalogMatch,
  tokenizeTravelPlaceQuery,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";
import { fetchAndEnrichTravelCoverImage } from "@/app/lib/travels-image-web-search";

export type { TravelCatalogImage };
export { formatTravelImageAttribution };

/** Seuil : l’image doit clairement coller au lieu / thème (pas une approximaton). */
const STRONG_MATCH_SCORE = 6;

function withNormalizedUrl(img: TravelCatalogImage): TravelCatalogImage {
  return { ...img, url: normalizePublicImageUrl(img.url) };
}

function normalizeId(value: string | undefined | null): string {
  return String(value || "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function fallbackImage(
  catalog: TravelCatalogImage[],
  excludeId?: string | null,
): TravelCatalogImage {
  const excluded = normalizeId(excludeId);
  const alternatives = excluded
    ? catalog.filter((img) => normalizeId(img.id) !== excluded)
    : catalog;
  const pool = alternatives.length > 0 ? alternatives : catalog;
  return pool[Math.floor(Math.random() * pool.length)] || catalog[0];
}

function isStrongLexicalMatch(
  img: TravelCatalogImage | undefined,
  title: string,
  destination: string,
): boolean {
  if (!img) return false;
  const tokens = tokenizeTravelPlaceQuery(title, destination);
  return scoreTravelCatalogMatch(img, tokens) >= STRONG_MATCH_SCORE;
}

/**
 * Mistral ne choisit que parmi des candidats déjà proches lexicalement.
 * S’il n’y en a aucun, on ne l’appelle pas : mieux vaut le web qu’un faux positif.
 */
async function pickFromCatalogWithMistral(opts: {
  candidates: TravelCatalogImage[];
  title: string;
  destination: string;
  excludeId?: string | null;
}): Promise<TravelCatalogImage | null> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey || opts.candidates.length === 0) return null;

  const pool = opts.candidates.slice(0, 25);
  const catalogSummary = pool
    .map((i) => `${i.id} — ${i.label}${i.keywords ? ` [${i.keywords}]` : ""}`)
    .join("\n");
  const avoidHint = opts.excludeId
    ? `\nÉvite l'ID "${opts.excludeId}" (image actuelle à remplacer).`
    : "";

  try {
    const response = await fetch("https://api.mistral.ai/v1/chat/completions", {
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
              `Tu sélectionnes l'illustration d'une sortie / d'un séjour scolaire.\n` +
              `Règles STRICTES :\n` +
              `- Choisis un ID UNIQUEMENT si le libellé / mots-clés décrivent clairement le MÊME lieu ou la MÊME activité (ex. surf → surf / océan / plage de surf ; pas un musée, pas un théâtre).\n` +
              `- Si aucune entrée ne correspond clairement, réponds exactement NONE.\n` +
              `- Ne force JAMAIS un choix par approximation, proximité géographique, thème culturel vague ou « meilleure option disponible ».\n` +
              `- NONE est la bonne réponse dès qu'il y a un doute.\n` +
              `Réponds UNIQUEMENT par l'ID exact (une ligne) ou NONE.\n` +
              `Candidats :\n${catalogSummary}` +
              avoidHint,
          },
          {
            role: "user",
            content: `TITRE : "${opts.title}"\nLIEU / THÈME : "${opts.destination}"`,
          },
        ],
        temperature: 0,
      }),
    });

    const resData = (await response.json()) as {
      choices?: Array<{ message?: { content?: string } }>;
    };
    const raw = resData.choices?.[0]?.message?.content?.trim() || "";
    const choice = raw.split(/\s+/)[0]?.replace(/[`"']/g, "") || "";
    if (!choice || /^none$/i.test(choice) || /^img_default$/i.test(choice)) {
      return null;
    }

    const matched = pool.find(
      (img) => normalizeId(img.id) === normalizeId(choice),
    );
    if (!matched) return null;
    if (opts.excludeId && normalizeId(matched.id) === normalizeId(opts.excludeId)) {
      return null;
    }
    // Filet de sécurité : même si Mistral force un ID, on refuse un choix lexicalement faible.
    if (!isStrongLexicalMatch(matched, opts.title, opts.destination)) {
      return null;
    }
    return withNormalizedUrl(matched);
  } catch (err) {
    console.error("[travels-select-cover-image] mistral", err);
    return null;
  }
}

/**
 * Choisit une image de présentation :
 * 1) match fort dans le catalogue (clé exacte / score lexical)
 * 2) sinon Wikimedia / Unsplash → enrichit le catalogue
 * 3) sinon Mistral uniquement parmi des candidats déjà proches (peut répondre NONE)
 * 4) sinon image de repli catalogue (dernier recours)
 */
export async function selectTravelCoverImage(opts: {
  title: string;
  destination: string;
  /** Si fourni, l’IA évite cette image (régénération). */
  excludeId?: string | null;
  /** Désactive l’enrichissement web (tests / offline). */
  allowWebEnrichment?: boolean;
}): Promise<TravelCatalogImage> {
  const title = opts.title || "Titre introuvable";
  const destination = opts.destination || "Destination introuvable";
  const excludeId = opts.excludeId || null;
  const allowWeb = opts.allowWebEnrichment !== false;

  const catalog = await listTravelCatalogImages();
  if (catalog.length === 0) {
    throw new Error("Catalogue d'images voyages vide");
  }

  const placeQuery = buildTravelPlaceSearchQuery(title, destination);
  const exactKey = normalizeTravelImageKey(placeQuery);
  if (exactKey) {
    const exact = await findTravelCatalogByNormalizeKey(exactKey);
    if (
      exact &&
      normalizeId(exact.id) !== normalizeId(excludeId) &&
      isStrongLexicalMatch(exact, title, destination)
    ) {
      return withNormalizedUrl(exact);
    }
  }

  const ranked = rankTravelCatalogCandidates(catalog, title, destination, excludeId);
  if (isStrongLexicalMatch(ranked[0], title, destination)) {
    return withNormalizedUrl(ranked[0]!);
  }

  const tryWebEnrichment = async (): Promise<TravelCatalogImage | null> => {
    if (!allowWeb) return null;
    try {
      const enriched = await fetchAndEnrichTravelCoverImage({
        query: placeQuery,
        title,
        destination,
      });
      if (enriched && normalizeId(enriched.id) !== normalizeId(excludeId)) {
        return withNormalizedUrl(enriched);
      }
    } catch (err) {
      console.error("[travels-select-cover-image] web enrich", err);
    }
    return null;
  };

  // Pas de match catalogue clair → web d’abord (création et régénération).
  const fromWeb = await tryWebEnrichment();
  if (fromWeb) return fromWeb;

  // Mistral seulement sur des candidats déjà proches ; jamais sur tout le catalogue.
  const fromAi = await pickFromCatalogWithMistral({
    candidates: ranked,
    title,
    destination,
    excludeId,
  });
  if (fromAi) return fromAi;

  // Dernier recours : mieux une image générique aléatoire qu’un faux match thématique forcé.
  return withNormalizedUrl(fallbackImage(catalog, excludeId));
}
