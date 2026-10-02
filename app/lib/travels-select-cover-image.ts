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
  ranked: TravelCatalogImage[],
  title: string,
  destination: string,
): boolean {
  const top = ranked[0];
  if (!top) return false;
  const tokens = tokenizeTravelPlaceQuery(title, destination);
  return scoreTravelCatalogMatch(top, tokens) >= 6;
}

async function pickFromCatalogWithMistral(opts: {
  catalog: TravelCatalogImage[];
  title: string;
  destination: string;
  excludeId?: string | null;
}): Promise<TravelCatalogImage | null> {
  const mistralKey = await getMistralApiKey();
  if (!mistralKey || opts.catalog.length === 0) return null;

  const candidates = rankTravelCatalogCandidates(
    opts.catalog,
    opts.title,
    opts.destination,
    opts.excludeId,
  ).slice(0, 25);
  const pool = candidates.length > 0 ? candidates : opts.catalog.slice(0, 40);
  const catalogSummary = pool.map((i) => `${i.id} (${i.label})`).join(", ");
  const avoidHint = opts.excludeId
    ? ` Évite l'ID "${opts.excludeId}" si une autre image convient.`
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
              `Tu choisis l'illustration d'une sortie scolaire. ` +
              `Réponds UNIQUEMENT par l'ID exact parmi : ${catalogSummary}. ` +
              `Si AUCUNE image ne correspond clairement au lieu / thème, réponds exactement NONE.` +
              avoidHint,
          },
          {
            role: "user",
            content: `TITRE : "${opts.title}" — LIEU : "${opts.destination}"`,
          },
        ],
        temperature: opts.excludeId ? 0.4 : 0,
      }),
    });

    const resData = (await response.json()) as {
      choices?: Array<{ message?: { content?: string } }>;
    };
    const choice = resData.choices?.[0]?.message?.content?.trim() || "";
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
    return withNormalizedUrl(matched);
  } catch (err) {
    console.error("[travels-select-cover-image] mistral", err);
    return null;
  }
}

/**
 * Choisit une image de présentation :
 * 1) match fort / Mistral dans le catalogue partagé (JSON seed + BDD enrichie)
 * 2) sinon recherche Wikimedia / Unsplash → copie S3 → enrichit le catalogue
 * 3) sinon image aléatoire du catalogue
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
    if (exact && normalizeId(exact.id) !== normalizeId(excludeId)) {
      return withNormalizedUrl(exact);
    }
  }

  const ranked = rankTravelCatalogCandidates(catalog, title, destination, excludeId);
  const strongCatalogMatch =
    ranked[0] && isStrongLexicalMatch(ranked, title, destination);
  if (strongCatalogMatch) {
    return withNormalizedUrl(ranked[0]);
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

  // Régénération : sans match catalogue fort, même repli web qu’à la création
  // (sinon Mistral choisit souvent une autre image générique du seed JSON).
  if (excludeId) {
    const fromWeb = await tryWebEnrichment();
    if (fromWeb) return fromWeb;
  }

  const fromAi = await pickFromCatalogWithMistral({
    catalog,
    title,
    destination,
    excludeId,
  });
  if (fromAi) {
    if (excludeId) {
      const tokens = tokenizeTravelPlaceQuery(title, destination);
      const aiScore = scoreTravelCatalogMatch(fromAi, tokens);
      if (aiScore < 6) {
        const fromWeb = await tryWebEnrichment();
        if (fromWeb) return fromWeb;
      }
    }
    return fromAi;
  }

  const fromWeb = await tryWebEnrichment();
  if (fromWeb) return fromWeb;

  if (ranked[0]) return withNormalizedUrl(ranked[0]);
  return withNormalizedUrl(fallbackImage(catalog, excludeId));
}
