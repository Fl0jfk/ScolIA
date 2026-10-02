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

/**
 * Mots trop génériques pour illustrer une sortie (jamais en requête web seuls).
 * Ex. « concours de drone à Houlgate » → drone / Houlgate, pas « concours ».
 */
const WEAK_THEME_WORDS = new Set([
  "concours",
  "competition",
  "compétition",
  "competitions",
  "championnat",
  "championnats",
  "tournoi",
  "tournois",
  "finale",
  "finales",
  "festival",
  "festivals",
  "salon",
  "salons",
  "foire",
  "foires",
  "meeting",
  "meetings",
  "rencontre",
  "rencontres",
  "edition",
  "editions",
  "édition",
  "éditions",
  "annee",
  "année",
  "saison",
  "projet",
  "projets",
  "atelier",
  "ateliers",
  "animation",
  "animations",
  "decouverte",
  "découverte",
  "initiation",
  "challenge",
  "challenges",
  "open",
  "cup",
  "trophy",
  "trophee",
  "trophée",
]);

function isYearToken(token: string): boolean {
  return /^(19|20)\d{2}$/.test(token);
}

function isWeakThemeToken(token: string): boolean {
  return WEAK_THEME_WORDS.has(token) || isYearToken(token);
}

export function tokenizeTravelPlaceQuery(...parts: string[]): string[] {
  const raw = parts.join(" ");
  const tokens = raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((t) => t.length >= 3 && !STOP_WORDS.has(t) && !isYearToken(t));
  return [...new Set(tokens)];
}

/** Tokens utiles pour une illustration (activité / lieu), hors « concours », années, etc. */
export function strongTravelThemeTokens(...parts: string[]): string[] {
  return tokenizeTravelPlaceQuery(...parts).filter((t) => !isWeakThemeToken(t));
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
  // Scoring sur l’activité / lieu concrets — pas « concours », années, etc.
  const tokens =
    strongTravelThemeTokens(title, destination).length > 0
      ? strongTravelThemeTokens(title, destination)
      : tokenizeTravelPlaceQuery(destination);
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
 * Contenu inadapté à une illustration scolaire (guerre, armes, etc.).
 * Inclut les noms de fichiers Wikimedia (ex. IAI_Heron sur la page « Drone »).
 */
const SCHOOL_UNSAFE_COVER_PATTERNS: RegExp[] = [
  /\bshahed\b/i,
  /\bgeran\b/i,
  /\bbayraktar\b/i,
  /\bbaykar\b/i,
  /\bswitchblade\b/i,
  /\bdrone\s+de\s+combat\b/i,
  /\bcombat\s+drone\b/i,
  /\bmunition\s+r[oô]deuse\b/i,
  /\bloitering\s+munition\b/i,
  /\bmissile\b/i,
  /\bbombe\b/i,
  /\bbombard/i,
  /\bguerre\b/i,
  /\bwar\b/i,
  /\bmilitair/i,
  /\bmilitary\b/i,
  /\barm[eé]e\b/i,
  /\bweapon\b/i,
  /\bartiller/i,
  /\bchar\s+de\s+combat\b/i,
  /\btank\b/i,
  /\bgaza\b/i,
  /\bukraine\b/i,
  /\bukrainian\b/i,
  /\brussian\s+invasion\b/i,
  /\bnucl[eé]air/i,
  /\bexplod/i,
  /\bsuicide\s+drone\b/i,
  /\bkamikaze\b/i,
  /\biai[_\-\s]?heron\b/i,
  /iai[_-]?heron/i,
  /heron[_-]?(tp|1|2)/i,
  /mq[_-]?\d+/i,
  /rq[_-]?\d+/i,
  /\bpredator\b/i,
  /\breaper\b/i,
  /global[\s_-]?hawk/i,
  /\bwatchkeeper\b/i,
  /\barmed\s+drone\b/i,
  /\bucav\b/i,
];

export function isSchoolSafeCoverText(...parts: string[]): boolean {
  const hay = parts
    .join(" ")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
  if (!hay.trim()) return true;
  return !SCHOOL_UNSAFE_COVER_PATTERNS.some((re) => re.test(hay));
}

/** Thèmes où Wikipedia renvoie surtout du militaire / technique inadapté. */
const WIKI_RISKY_ACTIVITY_TOKENS = new Set([
  "drone",
  "drones",
  "uav",
  "robot",
  "robots",
  "fusil",
  "arme",
  "tank",
]);

export function isWikiRiskyActivityToken(token: string): boolean {
  return WIKI_RISKY_ACTIVITY_TOKENS.has(
    String(token || "")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase(),
  );
}

/** Reformulations « scolaires / loisirs » pour des thèmes piégés sur Wikipedia. */
const SCHOOL_SAFE_ACTIVITY_QUERIES: Record<string, string[]> = {
  drone: [
    "quadcopter hobby",
    "drone camera hobby",
    "mini drone recreational",
    "drone loisir",
    "quadrirotor",
  ],
  drones: ["quadcopter hobby", "drone camera hobby", "drone loisir"],
  uav: ["quadcopter hobby", "drone camera hobby"],
  robot: ["robot educatif", "educational robot kids"],
  robots: ["robot educatif", "educational robot kids"],
  fusil: [],
  arme: [],
  tank: [],
};

function cleanSearchQuery(raw: string): string {
  return String(raw || "")
    .replace(
      /\b(sortie|voyage|séjour|sejour|visite|journée|journee|concours|competition|compétition)\b/gi,
      " ",
    )
    .replace(/\b(19|20)\d{2}\b/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Requêtes « lieu uniquement » (prioritaires pour une sortie scolaire). */
export function buildTravelPlaceOnlyQueries(
  title: string,
  destination: string,
): string[] {
  const dest = String(destination || "").trim();
  const destOk =
    dest &&
    !/^destination\s*introuvable$/i.test(dest) &&
    dest.toLowerCase() !== "n/a";
  if (!destOk) return [];

  const queries: string[] = [];
  const add = (raw: string) => {
    const q = cleanSearchQuery(raw);
    if (!q) return;
    if (queries.some((x) => x.toLowerCase() === q.toLowerCase())) return;
    if (!isSchoolSafeCoverText(q)) return;
    queries.push(q);
  };

  add(dest);
  add(`${dest} France`);
  add(`${dest} plage`);
  add(`${dest} ville`);
  for (const token of strongTravelThemeTokens(dest)) {
    if (token.length >= 4) add(token);
  }
  return queries.slice(0, 6);
}

/** Requêtes activité « sûres » (jamais le mot nu « drone » → Wikipedia militaire). */
export function buildTravelActivitySafeQueries(
  title: string,
  destination: string,
): string[] {
  const dest = String(destination || "").trim();
  const tit = String(title || "").trim();
  const destOk =
    dest &&
    !/^destination\s*introuvable$/i.test(dest) &&
    dest.toLowerCase() !== "n/a";
  const titleOk = tit && !/^titre\s*introuvable$/i.test(tit);
  if (!titleOk) return [];

  const strongTitle = strongTravelThemeTokens(tit);
  const queries: string[] = [];
  const add = (raw: string) => {
    const q = cleanSearchQuery(raw);
    if (!q) return;
    if (queries.some((x) => x.toLowerCase() === q.toLowerCase())) return;
    if (!isSchoolSafeCoverText(q)) return;
    const parts = tokenizeTravelPlaceQuery(q);
    if (parts.length === 0) return;
    if (parts.every((p) => isWeakThemeToken(p))) return;
    // Interdit une requête réduite à un seul token Wikipedia-risqué (drone, uav…).
    if (parts.length === 1 && isWikiRiskyActivityToken(parts[0]!)) return;
    queries.push(q);
  };

  for (const token of strongTitle) {
    const expansions = SCHOOL_SAFE_ACTIVITY_QUERIES[token];
    if (expansions && expansions.length > 0) {
      for (const alt of expansions) add(alt);
      if (destOk) {
        for (const alt of expansions.slice(0, 2)) add(`${alt} ${dest}`);
      }
      continue;
    }
    if (!isWikiRiskyActivityToken(token) && token.length >= 4) {
      add(token);
      if (destOk) add(`${token} ${dest}`);
    }
  }

  return queries.slice(0, 8);
}

/**
 * Lieu d’abord, puis activité reformulée « loisir ».
 * Jamais « concours » ni « drone » nu (page Wikipedia = drones militaires).
 */
export function buildTravelWebSearchQueries(title: string, destination: string): string[] {
  const queries = [
    ...buildTravelPlaceOnlyQueries(title, destination),
    ...buildTravelActivitySafeQueries(title, destination),
  ];
  if (queries.length === 0) return ["école france"];
  return queries.slice(0, 12);
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
