export type TravelImageCatalogSource = "manual" | "wikimedia" | "unsplash" | "openverse";

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
  "pedagogique",
  "pedagogiques",
  "educative",
  "educatives",
  "culturelle",
  "culturelles",
  "joueur",
  "joueurs",
  "joueuse",
  "joueuses",
  "eleve",
  "eleves",
  "enfant",
  "enfants",
  "groupe",
  "groupes",
  "club",
  "team",
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

/** Score lexical simple sur label + keywords (mots entiers, pas de sous-chaîne hasardeuse). */
export function scoreTravelCatalogMatch(
  img: TravelCatalogImage,
  tokens: string[],
): number {
  if (tokens.length === 0) return 0;
  const hay = `${img.label} ${img.keywords || ""} ${img.id}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  const hayTokens = new Set(
    hay
      .split(/[^a-z0-9]+/)
      .filter((t) => t.length >= 3),
  );
  let score = 0;
  for (const token of tokens) {
    if (hayTokens.has(token)) {
      score += token.length >= 6 ? 3 : 2;
      continue;
    }
    // Compose (ex. beaux-arts) : accepter seulement si le token est long (≥ 5).
    if (token.length >= 5 && hay.includes(token)) {
      score += token.length >= 6 ? 3 : 2;
    }
  }
  const key = img.normalizeKey || normalizeTravelImageKey(img.label);
  const queryKey = normalizeTravelImageKey(tokens.join(""));
  // Bonus clé exacte / quasi-exacte uniquement (évite un lieu court inclus dans un autre).
  if (
    key &&
    queryKey &&
    queryKey.length >= 5 &&
    (key === queryKey ||
      (queryKey.length >= 8 && (key.includes(queryKey) || queryKey.includes(key))))
  ) {
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

/** Libellé de lieu exploitable pour une recherche web (requête principale). */
export function buildTravelPlaceSearchQuery(title: string, destination: string): string {
  const queries = buildTravelWebSearchQueries(title, destination);
  return queries[0] || "école france";
}

/**
 * Plusieurs requêtes web, du plus thématique au plus large.
 * Ex. « Joueur surf » + « Rouen » → ["joueur surf", "surf", "joueur surf Rouen", "Rouen", …]
 * (évite de ne tenter que « joueur surf Rouen » qui rate Wikipedia).
 */
export function buildTravelWebSearchQueries(title: string, destination: string): string[] {
  const dest = String(destination || "").trim();
  const tit = String(title || "").trim();
  const destOk =
    dest &&
    !/^destination\s*introuvable$/i.test(dest) &&
    dest.toLowerCase() !== "n/a";
  const titleOk = tit && !/^titre\s*introuvable$/i.test(tit);

  const titleTokens = titleOk ? tokenizeTravelPlaceQuery(tit) : [];
  const destTokens = destOk ? tokenizeTravelPlaceQuery(dest) : [];
  const queries: string[] = [];
  const add = (raw: string) => {
    const q = String(raw || "")
      .replace(/\b(sortie|voyage|séjour|sejour|visite|journée|journee)\b/gi, " ")
      .replace(/\s+/g, " ")
      .trim();
    if (!q) return;
    if (queries.some((x) => x.toLowerCase() === q.toLowerCase())) return;
    queries.push(q);
  };

  // 1) Thème du titre seul (surf, accrobranche…) — le plus utile pour Wiki/Unsplash.
  if (titleTokens.length > 0) add(titleTokens.join(" "));
  for (const token of titleTokens) {
    if (token.length >= 4) add(token);
  }

  // 2) Thème + lieu
  if (titleTokens.length > 0 && destOk) {
    add(`${titleTokens.join(" ")} ${dest}`);
  }

  // 3) Lieu seul
  if (destOk) add(dest);
  for (const token of destTokens) {
    if (token.length >= 4) add(token);
  }

  // 4) Titre brut nettoyé
  if (titleOk) add(tit);

  if (queries.length === 0) add("école france");
  return queries.slice(0, 10);
}

/** Crédit d'attribution affichable. */
export function formatTravelImageAttribution(img: TravelCatalogImage): string | null {
  if (!img.author && !img.license) return null;
  const parts: string[] = [];
  if (img.author) parts.push(img.author);
  if (img.license) parts.push(img.license);
  if (img.source === "wikimedia") parts.push("Wikimedia Commons");
  if (img.source === "unsplash") parts.push("Unsplash");
  if (img.source === "openverse") parts.push("Openverse");
  return parts.join(" · ");
}
