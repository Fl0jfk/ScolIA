import { getMistralApiKey } from "@/app/lib/tenant-config";
import { normalizePublicImageUrl } from "@/app/lib/scola-image";
import {
  findTravelCatalogByNormalizeKey,
  formatTravelImageAttribution,
  listTravelCatalogImages,
  normalizeTravelImageKey,
  rankTravelCatalogCandidates,
  scoreTravelCatalogMatch,
  strongTravelThemeTokens,
  tokenizeTravelPlaceQuery,
  buildTravelWebSearchQueries,
  isSchoolSafeCoverText,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";
import { fetchAndEnrichTravelCoverImage } from "@/app/lib/travels-image-web-search";

export type { TravelCatalogImage };
export { formatTravelImageAttribution };

/** Seuil : l’image doit clairement coller au lieu / thème (pas une approximation). */
const STRONG_MATCH_SCORE = 6;

function withNormalizedUrl(img: TravelCatalogImage): TravelCatalogImage {
  return { ...img, url: normalizePublicImageUrl(img.url) };
}

function normalizeId(value: string | undefined | null): string {
  return String(value || "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function isUsableCatalogCover(
  img: TravelCatalogImage | undefined,
  title: string,
  destination: string,
): img is TravelCatalogImage {
  if (!img) return false;
  if (!isSchoolSafeCoverText(img.label, img.keywords || "", img.id)) return false;
  const tokens = strongTravelThemeTokens(title, destination);
  if (tokens.length === 0) {
    return (
      scoreTravelCatalogMatch(img, tokenizeTravelPlaceQuery(destination)) >=
      STRONG_MATCH_SCORE
    );
  }
  return scoreTravelCatalogMatch(img, tokens) >= STRONG_MATCH_SCORE;
}

/**
 * Mistral uniquement parmi des candidats déjà proches lexicalement.
 * Peut répondre NONE — jamais de catalogue « au hasard ».
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
              `- Choisis un ID UNIQUEMENT si le libellé / mots-clés décrivent clairement le MÊME lieu ou la MÊME activité concrète (ex. drone → drone loisir / Houlgate ; surf → océan).\n` +
              `- Jamais une image générique (« concours », « festival ») si le vrai sujet est une activité ou un lieu.\n` +
              `- Jamais une image militaire, de guerre, d'arme ou de drone de combat.\n` +
              `- Si aucune entrée ne convient clairement et de façon adaptée à une école, réponds exactement NONE.\n` +
              `- Ne force JAMAIS un choix par approximation.\n` +
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
 * 1) match fort catalogue (clé / score lexical)
 * 2) sinon TOUJOURS recherche web (plusieurs requêtes : thème, lieu…)
 * 3) sinon Mistral seulement sur candidats proches
 * 4) jamais d’image catalogue aléatoire hors sujet
 */
export async function selectTravelCoverImage(opts: {
  title: string;
  destination: string;
  /** Si fourni, on évite cette image (régénération). */
  excludeId?: string | null;
  /** URL actuelle à éviter lors d’une régénération web. */
  excludeImageUrl?: string | null;
  /** Désactive l’enrichissement web (tests / offline). */
  allowWebEnrichment?: boolean;
}): Promise<TravelCatalogImage | null> {
  const title = opts.title || "Titre introuvable";
  const destination = opts.destination || "Destination introuvable";
  const excludeId = opts.excludeId || null;
  const allowWeb = opts.allowWebEnrichment !== false;

  const catalog = await listTravelCatalogImages();
  if (catalog.length === 0 && !allowWeb) {
    throw new Error("Catalogue d'images voyages vide");
  }

  // Match catalogue fort uniquement (jamais un « à peu près » Rouen → cimetière).
  if (catalog.length > 0) {
    const queries = buildTravelWebSearchQueries(title, destination);
    for (const q of queries.slice(0, 3)) {
      const exactKey = normalizeTravelImageKey(q);
      if (!exactKey) continue;
      const exact = await findTravelCatalogByNormalizeKey(exactKey);
      if (
        exact &&
        normalizeId(exact.id) !== normalizeId(excludeId) &&
        isUsableCatalogCover(exact, title, destination)
      ) {
        return withNormalizedUrl(exact);
      }
    }

    const ranked = rankTravelCatalogCandidates(catalog, title, destination, excludeId).filter(
      (img) => isUsableCatalogCover(img, title, destination),
    );
    if (ranked[0]) {
      return withNormalizedUrl(ranked[0]);
    }
  }

  const tryWebEnrichment = async (): Promise<TravelCatalogImage | null> => {
    if (!allowWeb) return null;
    try {
      const enriched = await fetchAndEnrichTravelCoverImage({
        title,
        destination,
        excludeImageUrls: opts.excludeImageUrl ? [opts.excludeImageUrl] : [],
      });
      if (!enriched?.url) return null;
      if (!isSchoolSafeCoverText(enriched.label, enriched.keywords || "", enriched.id)) {
        return null;
      }
      // Régénération : on accepte même le même id catalogue si l’URL a changé,
      // et on refuse seulement si c’est strictement la même image exclue.
      if (
        excludeId &&
        normalizeId(enriched.id) === normalizeId(excludeId) &&
        opts.excludeImageUrl &&
        normalizePublicImageUrl(enriched.url) ===
          normalizePublicImageUrl(opts.excludeImageUrl)
      ) {
        return null;
      }
      return withNormalizedUrl(enriched);
    } catch (err) {
      console.error("[travels-select-cover-image] web enrich", err);
      return null;
    }
  };

  const fromWeb = await tryWebEnrichment();
  if (fromWeb) return fromWeb;

  const ranked = rankTravelCatalogCandidates(
    catalog,
    title,
    destination,
    excludeId,
  ).filter((img) => isUsableCatalogCover(img, title, destination));
  const fromAi = await pickFromCatalogWithMistral({
    candidates: ranked,
    title,
    destination,
    excludeId,
  });
  if (fromAi) return fromAi;

  // Pas de fallback aléatoire : mieux aucune image qu’un cimetière pour du surf.
  console.warn("[travels-select-cover-image] no suitable cover", {
    title,
    destination,
    queries: buildTravelWebSearchQueries(title, destination).slice(0, 5),
  });
  return null;
}
