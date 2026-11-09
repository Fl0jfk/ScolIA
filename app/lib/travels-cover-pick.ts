/**
 * Règles pures du choix d'image de couverture des séjours (testables sans réseau).
 */

export type TravelCoverQueryKind = "place" | "activity";

export type TravelCoverSource =
  | "wikipedia-fr"
  | "wikipedia-en"
  | "commons"
  | "unsplash"
  | "openverse";

/**
 * Ordre des sources selon ce qu'on illustre.
 * - Lieu précis (Tour Eiffel, Chambord) : Wikipedia montre le vrai lieu, les banques
 *   d'images renvoient souvent un lieu qui ressemble.
 * - Activité (surf, ski) : les banques d'images ont de belles photos génériques.
 */
export function travelCoverSourceOrder(
  kind: TravelCoverQueryKind,
  allowWikipedia: boolean,
): TravelCoverSource[] {
  const order: TravelCoverSource[] =
    kind === "place"
      ? ["wikipedia-fr", "wikipedia-en", "commons", "unsplash", "openverse"]
      : ["unsplash", "commons", "openverse", "wikipedia-fr", "wikipedia-en"];
  return allowWikipedia
    ? order
    : order.filter((s) => s !== "wikipedia-fr" && s !== "wikipedia-en");
}

/**
 * Refuse les images trop petites ou extrêmement en hauteur.
 * Les monuments verticaux (Tour Eiffel ≈ 0,54) restent acceptés : la vérification
 * visuelle préfère ensuite une vue en paysage quand il y en a une.
 */
export function isAcceptableTravelCoverSize(
  width?: number | null,
  height?: number | null,
): boolean {
  const w = Number(width) || 0;
  const h = Number(height) || 0;
  if (!w || !h) return true; // taille inconnue : on laisse la vérification visuelle trancher
  if (w < 640) return false;
  return w / h >= 0.5;
}

/** Remet les résultats Commons dans l'ordre de pertinence (l'API les renvoie par id). */
export function sortCommonsPagesByIndex<T extends { index?: number }>(pages: T[]): T[] {
  return [...pages].sort(
    (a, b) => (a.index ?? Number.MAX_SAFE_INTEGER) - (b.index ?? Number.MAX_SAFE_INTEGER),
  );
}

/** Miniature Wikimedia à la largeur voulue, à partir d'une URL originale ou miniature. */
export function wikimediaThumbAtWidth(url: string, width: number): string | null {
  try {
    const u = new URL(url);
    u.search = "";
    u.hash = "";
    const thumb = u.pathname.match(/^(.*\/thumb\/.+\/)(\d+)px-([^/]+)$/);
    if (thumb) {
      u.pathname = `${thumb[1]}${width}px-${thumb[3]}`;
      return u.toString();
    }
    const original = u.pathname.match(
      /^(.*?\/commons\/)([0-9a-f]\/[0-9a-f]{2}\/)([^/]+\.(jpe?g|png|webp|gif))$/i,
    );
    if (!original) return null;
    const fileName = original[3];
    u.pathname = `${original[1]}thumb/${original[2]}${fileName}/${width}px-${fileName}`;
    return u.toString();
  } catch {
    return null;
  }
}

/**
 * Lit la réponse de la vérification visuelle : numéro d'image (1…count) ou « NONE ».
 * Renvoie l'index 0-based, null si aucune ne convient, undefined si illisible.
 */
export function parseTravelCoverVisionPick(
  raw: string,
  count: number,
): number | null | undefined {
  const text = String(raw || "").trim();
  if (!text) return undefined;
  try {
    const start = text.indexOf("{");
    const end = text.lastIndexOf("}");
    if (start >= 0 && end > start) {
      const parsed = JSON.parse(text.slice(start, end + 1)) as { best?: unknown };
      const best = parsed.best;
      if (best === null || best === 0 || /^none$/i.test(String(best ?? ""))) return null;
      const n = Number(best);
      if (Number.isInteger(n) && n >= 1 && n <= count) return n - 1;
      return undefined;
    }
  } catch {
    // on tente la lecture simple ci-dessous
  }
  if (/^none\b/i.test(text)) return null;
  const n = Number(text.match(/^\d+/)?.[0]);
  if (Number.isInteger(n) && n >= 1 && n <= count) return n - 1;
  return undefined;
}
