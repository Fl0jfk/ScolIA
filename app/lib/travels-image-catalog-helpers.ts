export type TravelImageCatalogSource = "manual" | "wikimedia" | "unsplash";

export type TravelCatalogImage = {
  id: string;
  label: string;
  url: string;
  keywords?: string;
  source?: TravelImageCatalogSource;
  author?: string | null;
  license?: string | null;
  attributionUrl?: string | null;
  sourcePageUrl?: string | null;
  normalizeKey?: string;
};

/** Normalise un libellé lieu pour dédup / matching (sans accents, alphanum). */
export function normalizeTravelImageKey(value: string): string {
  return String(value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "")
    .slice(0, 120);
}

const STOP_WORDS = new Set([
  "sortie",
  "sorties",
  "voyage",
  "voyages",
  "sejour",
  "séjour",
  "visite",
  "visites",
  "classe",
  "classes",
  "scolaire",
  "scolaires",
  "journee",
  "journée",
  "au",
  "aux",
  "de",
  "des",
  "du",
  "la",
  "le",
  "les",
  "un",
  "une",
  "et",
  "en",
  "a",
  "à",
  "pour",
  "avec",
  "sur",
  "dans",
  "par",
  "the",
  "of",
  "to",
]);

export function tokenizeTravelPlaceQuery(...parts: string[]): string[] {
  const raw = parts.join(" ");
  const tokens = raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((t) => t.length >= 3 && !STOP_WORDS.has(t));
  return [...new Set(tokens)];
}

/** Score lexical simple sur label + keywords. */
export function scoreTravelCatalogMatch(
  img: TravelCatalogImage,
  tokens: string[],
): number {
  if (tokens.length === 0) return 0;
  const hay = `${img.label} ${img.keywords || ""} ${img.id}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  let score = 0;
  for (const token of tokens) {
    if (hay.includes(token)) score += token.length >= 6 ? 3 : 2;
  }
  const key = img.normalizeKey || normalizeTravelImageKey(img.label);
  const queryKey = normalizeTravelImageKey(tokens.join(""));
  if (key && queryKey && (key.includes(queryKey) || queryKey.includes(key))) {
    score += 8;
  }
  return score;
}

export function rankTravelCatalogCandidates(
  catalog: TravelCatalogImage[],
  title: string,
  destination: string,
  excludeId?: string | null,
): TravelCatalogImage[] {
  const tokens = tokenizeTravelPlaceQuery(title, destination);
  const excluded = String(excludeId || "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
  return catalog
    .filter((img) => {
      if (!excluded) return true;
      const idNorm = String(img.id || "")
        .toLowerCase()
        .replace(/[^a-z0-9]/g, "");
      return idNorm !== excluded;
    })
    .map((img) => ({ img, score: scoreTravelCatalogMatch(img, tokens) }))
    .filter((x) => x.score >= 4)
    .sort((a, b) => b.score - a.score)
    .map((x) => x.img);
}

/** Libellé de lieu exploitable pour une recherche web. */
export function buildTravelPlaceSearchQuery(title: string, destination: string): string {
  const dest = String(destination || "").trim();
  const tit = String(title || "").trim();
  const preferred =
    dest &&
    !/^destination\s*introuvable$/i.test(dest) &&
    dest.toLowerCase() !== "n/a"
      ? dest
      : tit;
  const cleaned = preferred
    .replace(/\b(sortie|voyage|séjour|sejour|visite|journée|journee)\b/gi, " ")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || preferred || "école france";
}

/** Crédit d'attribution affichable. */
export function formatTravelImageAttribution(img: TravelCatalogImage): string | null {
  if (!img.author && !img.license) return null;
  const parts: string[] = [];
  if (img.author) parts.push(img.author);
  if (img.license) parts.push(img.license);
  if (img.source === "wikimedia") parts.push("Wikimedia Commons");
  if (img.source === "unsplash") parts.push("Unsplash");
  return parts.join(" · ");
}
