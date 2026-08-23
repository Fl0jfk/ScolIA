import { getMistralApiKey } from "@/app/lib/tenant-config";
import { normalizePublicImageUrl } from "@/app/lib/scola-image";
import IMAGE_CATALOG from "@/app/api/travels/update/image-catalog.json";

type TravelCatalogImage = {
  id: string;
  label: string;
  url: string;
  keywords?: string;
};

const catalog = (IMAGE_CATALOG as TravelCatalogImage[]).map((img) => ({
  ...img,
  url: normalizePublicImageUrl(img.url),
}));

function withNormalizedUrl(img: TravelCatalogImage): TravelCatalogImage {
  return { ...img, url: normalizePublicImageUrl(img.url) };
}

function normalizeId(value: string | undefined | null): string {
  return String(value || "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function fallbackImage(excludeId?: string | null): TravelCatalogImage {
  const excluded = normalizeId(excludeId);
  const alternatives = excluded
    ? catalog.filter((img) => normalizeId(img.id) !== excluded)
    : catalog;
  const pool = alternatives.length > 0 ? alternatives : catalog;
  return pool[Math.floor(Math.random() * pool.length)] || catalog[0];
}

/**
 * Choisit une image de présentation dans le catalogue via Mistral
 * (même logique qu’à la création d’un dossier travels).
 */
export async function selectTravelCoverImage(opts: {
  title: string;
  destination: string;
  /** Si fourni, l’IA évite cette image (régénération). */
  excludeId?: string | null;
}): Promise<TravelCatalogImage> {
  const title = opts.title || "Titre introuvable";
  const destination = opts.destination || "Destination introuvable";
  const excludeId = opts.excludeId || null;

  try {
    const mistralKey = await getMistralApiKey();
    if (!mistralKey) return withNormalizedUrl(fallbackImage(excludeId));

    const catalogSummary = catalog.map((i) => `${i.id} (${i.label})`).join(", ");
    const avoidHint = excludeId
      ? ` Évite l'ID "${excludeId}" si une autre image convient.`
      : "";

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
            content: `Choisis l'ID exact parmis : ${catalogSummary}. Réponds uniquement par l'ID. Sinon "img_default".${avoidHint}`,
          },
          {
            role: "user",
            content: `DONNÉES À ANALYSER : - TITRE : "${title}" - LIEU : "${destination}"`,
          },
        ],
        temperature: excludeId ? 0.4 : 0,
      }),
    });

    const resData = (await response.json()) as {
      choices?: Array<{ message?: { content?: string } }>;
    };
    const mistralChoice = resData.choices?.[0]?.message?.content?.trim();
    const matched =
      catalog.find(
        (img) => normalizeId(img.id) === normalizeId(mistralChoice),
      ) ||
      catalog.find((img) => img.id === "img_default") ||
      catalog[0];

    if (excludeId && normalizeId(matched.id) === normalizeId(excludeId)) {
      return withNormalizedUrl(fallbackImage(excludeId));
    }
    return withNormalizedUrl(matched);
  } catch (err) {
    console.error("[travels-select-cover-image]", err);
    return withNormalizedUrl(fallbackImage(excludeId));
  }
}
