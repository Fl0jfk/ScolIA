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
  buildTravelPlaceOnlyQueries,
  buildTravelWebSearchQueries,
  isSchoolSafeCoverText,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";
import { proposeTravelCoverSearchQueries } from "@/app/lib/travels-cover-mistral-intent";
import { fetchAndEnrichTravelCoverImage } from "@/app/lib/travels-image-web-search";

export type { TravelCatalogImage };
export { formatTravelImageAttribution };

const STRONG_MATCH_SCORE = 6;

function withNormalizedUrl(img: TravelCatalogImage): TravelCatalogImage {
  return { ...img, url: normalizePublicImageUrl(img.url) };
}

function normalizeId(value: string | undefined | null): string {
  return String(value || "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function isUsableManualCatalogCover(
  img: TravelCatalogImage | undefined,
  title: string,
  destination: string,
): img is TravelCatalogImage {
  if (!img) return false;
  if (!isSchoolSafeCoverText(img.label, img.keywords || "", img.id, img.url)) {
    return false;
  }
  // Ne jamais resservir les auto-* issus d’anciennes recherches foireuses.
  if (img.source && img.source !== "manual") return false;
  if (String(img.id).startsWith("auto-")) return false;

  const tokens = strongTravelThemeTokens(title, destination);
  if (tokens.length === 0) {
    return (
      scoreTravelCatalogMatch(img, tokenizeTravelPlaceQuery(destination)) >=
      STRONG_MATCH_SCORE
    );
  }
  return scoreTravelCatalogMatch(img, tokens) >= STRONG_MATCH_SCORE;
}

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
              `Tu sélectionnes l'illustration d'une sortie scolaire dans un catalogue.\n` +
              `Comprends le SENS du titre + lieu. Choisis UNIQUEMENT si ça colle clairement ` +
              `(lieu réel ou activité concrète adaptée aux élèves).\n` +
              `Jamais concours générique, jamais militaire / drone de combat.\n` +
              `Sinon NONE.\n` +
              `Candidats :\n${catalogSummary}` +
              avoidHint,
          },
          {
            role: "user",
            content: `TITRE : "${opts.title}"\nLIEU : "${opts.destination}"`,
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
    if (!isUsableManualCatalogCover(matched, opts.title, opts.destination)) {
      return null;
    }
    return withNormalizedUrl(matched);
  } catch (err) {
    console.error("[travels-select-cover-image] mistral catalog", err);
    return null;
  }
}

/**
 * 1) Mistral comprend le sens → propose des requêtes d’image « école »
 * 2) Recherche web (Wikimedia / Unsplash / Openverse) avec ces requêtes
 * 3) Repli heuristique lieu / activité loisir
 * 4) Catalogue manuel fort (Disneyland…) + Mistral catalogue en dernier recours
 */
export async function selectTravelCoverImage(opts: {
  title: string;
  destination: string;
  excludeId?: string | null;
  excludeImageUrl?: string | null;
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

  // 1) Intention Mistral (compréhension du sens) AVANT toute recherche web/catalogue auto.
  const intent = allowWeb
    ? await proposeTravelCoverSearchQueries({ title, destination })
    : null;

  const tryWebEnrichment = async (
    preferredQueries?: string[],
  ): Promise<TravelCatalogImage | null> => {
    if (!allowWeb) return null;
    try {
      const enriched = await fetchAndEnrichTravelCoverImage({
        title,
        destination,
        preferredQueries,
        excludeImageUrls: opts.excludeImageUrl ? [opts.excludeImageUrl] : [],
      });
      if (!enriched?.url) return null;
      if (
        !isSchoolSafeCoverText(
          enriched.label,
          enriched.keywords || "",
          enriched.id,
          enriched.url,
        )
      ) {
        return null;
      }
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

  // 2) Web guidé par Mistral (puis repli heuristique dans fetchAndEnrich).
  const fromWeb = await tryWebEnrichment(intent?.queries);
  if (fromWeb) return fromWeb;

  // 3) Catalogue manuel uniquement (ex. Disneyland déjà en base).
  if (catalog.length > 0) {
    const placeKeys = buildTravelPlaceOnlyQueries(title, destination);
    for (const q of placeKeys.slice(0, 3)) {
      const exactKey = normalizeTravelImageKey(q);
      if (!exactKey) continue;
      const exact = await findTravelCatalogByNormalizeKey(exactKey);
      if (
        exact &&
        normalizeId(exact.id) !== normalizeId(excludeId) &&
        isUsableManualCatalogCover(exact, title, destination)
      ) {
        return withNormalizedUrl(exact);
      }
    }

    const ranked = rankTravelCatalogCandidates(
      catalog,
      title,
      destination,
      excludeId,
    ).filter((img) => isUsableManualCatalogCover(img, title, destination));
    if (ranked[0]) return withNormalizedUrl(ranked[0]);

    const fromAi = await pickFromCatalogWithMistral({
      candidates: ranked,
      title,
      destination,
      excludeId,
    });
    if (fromAi) return fromAi;
  }

  console.warn("[travels-select-cover-image] no suitable cover", {
    title,
    destination,
    mistralQueries: intent?.queries?.slice(0, 5),
    fallbackQueries: buildTravelWebSearchQueries(title, destination).slice(0, 5),
  });
  return null;
}
