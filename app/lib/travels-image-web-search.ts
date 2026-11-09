import "server-only";
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getPlatformS3Client } from "@/app/lib/s3-clients";
import { SCOLA_IMAGE_BUCKET, scolaImageUrl } from "@/app/lib/scola-image";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import {
  buildTravelActivitySafeQueries,
  buildTravelPlaceOnlyQueries,
  isSchoolSafeCoverText,
  isWikiRiskyActivityToken,
  normalizeTravelImageKey,
  strongTravelThemeTokens,
  upsertTravelCatalogImage,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";
import {
  isAcceptableTravelCoverSize,
  sortCommonsPagesByIndex,
  travelCoverSourceOrder,
  wikimediaThumbAtWidth,
  type TravelCoverQueryKind,
  type TravelCoverSource,
} from "@/app/lib/travels-cover-pick";
import { pickTravelCoverWithVision } from "@/app/lib/travels-cover-vision";

/**
 * Largeur des miniatures pour la vérification visuelle (peu de jetons).
 * Wikimedia n'accepte que des largeurs fixes (330, 500, 960, 1280…) : 512 renvoie 400.
 */
const VISION_THUMB_WIDTH = 500;
/** Nombre maximum de photos comparées par séjour. */
const MAX_VISION_CANDIDATES = 6;

const WIKI_UA =
  "ScolIA/1.0 (school ENT cover images; https://scolia.fr; contact@scolia.fr)";

type WebImageHit = {
  title: string;
  imageUrl: string;
  /** URLs alternatives (miniatures) — préférées pour l’hébergement CDN. */
  alternateUrls?: string[];
  author?: string | null;
  license?: string | null;
  attributionUrl?: string | null;
  sourcePageUrl?: string | null;
  source: "wikimedia" | "unsplash" | "openverse";
  mime?: string | null;
  /** Petite miniature pour la vérification visuelle. */
  thumbUrl?: string | null;
};

/** Uniquement le bucket CDN public — jamais le bucket métier privé (BUCKET_NAME). */
function publicImageBuckets(): string[] {
  const list = [
    process.env.IMAGE_BUCKET?.trim(),
    SCOLA_IMAGE_BUCKET,
  ].filter((b): b is string => Boolean(b));
  return [...new Set(list)];
}

function publicUrlForBucketKey(bucket: string, key: string): string {
  // Toujours via le helper CDN (évite une URL path-style / région privée).
  void bucket;
  return scolaImageUrl(key);
}

function s3ClientForPublicImageBucket(): S3Client {
  const accessKeyId = process.env.ACCESS_KEY_ID?.trim();
  const secretAccessKey = process.env.SECRET_ACCESS_KEY?.trim();
  if (accessKeyId && secretAccessKey) {
    return new S3Client({
      region: "fr-par",
      endpoint: process.env.S3_ENDPOINT?.trim() || "https://s3.fr-par.scw.cloud",
      forcePathStyle: process.env.S3_FORCE_PATH_STYLE !== "false",
      credentials: { accessKeyId, secretAccessKey },
      requestChecksumCalculation: "WHEN_REQUIRED",
      responseChecksumValidation: "WHEN_REQUIRED",
    });
  }
  return getPlatformS3Client();
}

function cleanImageUrl(url: string): string {
  try {
    const parsed = new URL(url);
    // Wikimedia ajoute des utm_* inutiles qui cassent parfois le cache / l’optimizer.
    parsed.search = "";
    parsed.hash = "";
    return parsed.toString();
  } catch {
    return String(url || "").trim();
  }
}

function looksLikeImageBytes(bytes: Buffer): boolean {
  if (bytes.length < 24) return false;
  // JPEG
  if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return true;
  // PNG
  if (
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    return true;
  }
  // GIF
  if (bytes.slice(0, 3).toString("ascii") === "GIF") return true;
  // WEBP (RIFF....WEBP)
  if (
    bytes.slice(0, 4).toString("ascii") === "RIFF" &&
    bytes.slice(8, 12).toString("ascii") === "WEBP"
  ) {
    return true;
  }
  return false;
}

function extFromMimeOrUrl(mime: string | null | undefined, url: string): string {
  const m = String(mime || "").toLowerCase();
  if (m.includes("png")) return "png";
  if (m.includes("webp")) return "webp";
  if (m.includes("gif")) return "gif";
  if (m.includes("avif")) return "avif";
  if (m.includes("jpeg") || m.includes("jpg")) return "jpg";
  const path = url.split("?")[0].toLowerCase();
  const match = path.match(/\.(jpe?g|png|webp|gif|avif)$/);
  return match?.[1]?.replace("jpeg", "jpg") || "jpg";
}

async function downloadBytes(
  url: string,
): Promise<{ bytes: Buffer; contentType: string; sourceUrl: string } | null> {
  const cleaned = cleanImageUrl(url);
  if (!cleaned) return null;
  try {
    const res = await fetch(cleaned, {
      headers: {
        "User-Agent": WIKI_UA,
        Accept: "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
      },
      redirect: "follow",
    });
    if (!res.ok) return null;
    const contentType = res.headers.get("content-type") || "image/jpeg";
    if (!contentType.startsWith("image/")) return null;
    const ab = await res.arrayBuffer();
    // Trop petit = souvent une page d’erreur / stub ; trop gros = risque timeout Scaleway.
    if (!ab.byteLength || ab.byteLength < 4_000 || ab.byteLength > 8_000_000) {
      return null;
    }
    const bytes = Buffer.from(ab);
    if (!looksLikeImageBytes(bytes)) return null;
    return { bytes, contentType, sourceUrl: cleaned };
  } catch (err) {
    console.error("[travels-image-web] download failed", cleaned, err);
    return null;
  }
}

async function downloadBestCandidate(
  hit: WebImageHit,
): Promise<{ bytes: Buffer; contentType: string; sourceUrl: string } | null> {
  const urls = [
    ...(hit.alternateUrls || []),
    hit.imageUrl,
  ]
    .map(cleanImageUrl)
    .filter((u, i, arr) => u && arr.indexOf(u) === i);

  for (const url of urls) {
    const downloaded = await downloadBytes(url);
    if (downloaded) return downloaded;
  }
  return null;
}

async function isPubliclyReadableImage(url: string): Promise<boolean> {
  try {
    const res = await fetch(url, {
      method: "GET",
      headers: {
        "User-Agent": WIKI_UA,
        Range: "bytes=0-64",
        Accept: "image/*,*/*",
      },
      redirect: "follow",
    });
    if (!(res.ok || res.status === 206)) return false;
    const contentType = res.headers.get("content-type") || "";
    if (contentType && !contentType.startsWith("image/") && !contentType.includes("octet-stream")) {
      return false;
    }
    const bytes = Buffer.from(await res.arrayBuffer());
    return looksLikeImageBytes(bytes) || bytes.length > 0;
  } catch {
    return false;
  }
}

async function uploadCoverToPublicBucket(opts: {
  normalizeKey: string;
  bytes: Buffer;
  contentType: string;
  ext: string;
}): Promise<string | null> {
  const fileName = sanitizeS3FileName(
    `${opts.normalizeKey}-${Date.now().toString(36)}.${opts.ext}`,
  );
  const key = s3Key(`travels/auto/${fileName}`);
  const client = s3ClientForPublicImageBucket();

  for (const bucket of publicImageBuckets()) {
    try {
      await client.send(
        new PutObjectCommand({
          Bucket: bucket,
          Key: key,
          Body: opts.bytes,
          ContentType: opts.contentType,
          CacheControl: "public, max-age=31536000, immutable",
          // Obligatoire : sans ça Scaleway crée un objet privé → 403 sur l’URL CDN.
          ACL: "public-read",
        }),
      );
      const url = publicUrlForBucketKey(bucket, key);
      const readableNow = await isPubliclyReadableImage(url);
      const readableRetry =
        readableNow ||
        (await new Promise<boolean>((resolve) => {
          setTimeout(() => {
            void isPubliclyReadableImage(url).then(resolve);
          }, 600);
        }));
      if (!readableRetry) {
        console.error(
          `[travels-image-web] objet uploadé mais toujours en 403 public sur ${bucket}/${key}`,
        );
        continue;
      }
      return url;
    } catch (err) {
      console.warn(
        `[travels-image-web] S3 upload failed on ${bucket}:`,
        err instanceof Error ? err.message : err,
      );
    }
  }
  return null;
}

function normalizeHay(value: string): string {
  return String(value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

/** Refuse une image web hors sujet ou inadaptée à une école. */
function hitLooksRelevant(hit: WebImageHit, query: string, themeTokens: string[]): boolean {
  if (!isSchoolSafeCoverText(hit.title, query, hit.imageUrl)) return false;

  const titleHay = normalizeHay(hit.title);
  const queryHay = normalizeHay(query);
  const distinctive = themeTokens.filter((t) => t.length >= 4);
  if (distinctive.length === 0) return true;

  const themeInQuery = distinctive.filter((t) => queryHay.includes(t));
  if (themeInQuery.length === 0) {
    return true;
  }
  return themeInQuery.some((t) => titleHay.includes(t));
}

function wikimediaMidThumbUrl(originalUrl: string): string | null {
  try {
    const u = new URL(cleanImageUrl(originalUrl));
    // /wikipedia/commons/d/d1/file.jpg
    // → /wikipedia/commons/thumb/d/d1/file.jpg/1280px-file.jpg
    const match = u.pathname.match(
      /^(.*?\/commons\/)([0-9a-f]\/[0-9a-f]{2}\/)([^/]+\.(jpe?g|png|webp|gif))$/i,
    );
    if (!match) return null;
    const fileName = match[3];
    u.pathname = `${match[1]}thumb/${match[2]}${fileName}/1280px-${fileName}`;
    u.search = "";
    return u.toString();
  } catch {
    return null;
  }
}

type WikiSummary = {
  title?: string;
  content_urls?: { desktop?: { page?: string } };
  originalimage?: { source?: string; width?: number; height?: number };
  thumbnail?: { source?: string; width?: number; height?: number };
  type?: string;
};

async function wikiSummaryFromTitle(
  title: string,
  lang: "fr" | "en",
): Promise<WebImageHit | null> {
  const url = `https://${lang}.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(title)}`;
  try {
    const res = await fetch(url, {
      headers: { "User-Agent": WIKI_UA, Accept: "application/json" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as WikiSummary;
    if (data.type === "disambiguation") return null;
    if (
      !isAcceptableTravelCoverSize(
        data.originalimage?.width,
        data.originalimage?.height,
      )
    ) {
      return null;
    }
    const original = data.originalimage?.source
      ? cleanImageUrl(data.originalimage.source)
      : null;
    const thumb = data.thumbnail?.source
      ? cleanImageUrl(data.thumbnail.source)
      : null;
    const mid = original ? wikimediaMidThumbUrl(original) : null;
    const imageUrl = mid || thumb || original;
    if (!imageUrl) return null;
    return {
      title: data.title || title,
      imageUrl,
      alternateUrls: [mid, thumb, original].filter(
        (u): u is string => Boolean(u),
      ),
      author: null,
      license: "Wikipedia / Wikimedia",
      attributionUrl: data.content_urls?.desktop?.page || null,
      sourcePageUrl: data.content_urls?.desktop?.page || null,
      source: "wikimedia",
      mime: null,
      thumbUrl:
        (original && wikimediaThumbAtWidth(original, VISION_THUMB_WIDTH)) || thumb,
    };
  } catch {
    return null;
  }
}

async function fetchWikipediaSummary(query: string, lang: "fr" | "en"): Promise<WebImageHit | null> {
  const titleGuess = query.trim().replace(/\s+/g, "_");
  const directCandidates = [titleGuess, query.trim()];
  for (const candidate of directCandidates) {
    if (!isSchoolSafeCoverText(candidate)) continue;
    const direct = await wikiSummaryFromTitle(candidate, lang);
    if (direct && isSchoolSafeCoverText(direct.title)) return direct;
  }

  // Recherche plein texte — on ignore les pages militaires / hors sujet scolaire.
  try {
    const api = new URL(`https://${lang}.wikipedia.org/w/api.php`);
    api.searchParams.set("action", "query");
    api.searchParams.set("list", "search");
    api.searchParams.set("srsearch", query);
    api.searchParams.set("srlimit", "10");
    api.searchParams.set("srnamespace", "0");
    api.searchParams.set("format", "json");
    api.searchParams.set("origin", "*");
    const res = await fetch(api.toString(), {
      headers: { "User-Agent": WIKI_UA, Accept: "application/json" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as {
      query?: { search?: Array<{ title?: string }> };
    };
    const queryNorm = normalizeHay(query);
    const ranked = (data.query?.search || [])
      .map((row) => String(row.title || "").trim())
      .filter(Boolean)
      .filter((title) => isSchoolSafeCoverText(title))
      .map((title) => {
        const t = normalizeHay(title);
        let score = 0;
        if (t === queryNorm) score += 20;
        if (t.startsWith(queryNorm) || queryNorm.startsWith(t)) score += 10;
        for (const part of queryNorm.split(/\s+/).filter((p) => p.length >= 4)) {
          if (t === part) score += 8;
          else if (t.includes(part)) score += 3;
        }
        // Pénalise les listes / pages techniques hors illustration.
        if (t.startsWith("liste ") || t.startsWith("list of ")) score -= 8;
        return { title, score };
      })
      .filter((x) => x.score > 0)
      .sort((a, b) => b.score - a.score);

    for (const row of ranked) {
      const hit = await wikiSummaryFromTitle(row.title, lang);
      if (hit && isSchoolSafeCoverText(hit.title)) return hit;
    }
  } catch (err) {
    console.error("[travels-image-web] wiki search failed", lang, err);
  }
  return null;
}

type CommonsSearchResponse = {
  query?: {
    pages?: Record<
      string,
      {
        title?: string;
        /** Rang dans la recherche (l'objet `pages` est trié par id, pas par pertinence). */
        index?: number;
        imageinfo?: Array<{
          url?: string;
          thumburl?: string;
          width?: number;
          height?: number;
          mime?: string;
          descriptionurl?: string;
          extmetadata?: Record<string, { value?: string }>;
        }>;
      }
    >;
  };
};

async function searchWikimediaCommons(
  query: string,
  skipUrls: Set<string>,
): Promise<WebImageHit | null> {
  const api = new URL("https://commons.wikimedia.org/w/api.php");
  api.searchParams.set("action", "query");
  api.searchParams.set("format", "json");
  api.searchParams.set("generator", "search");
  api.searchParams.set("gsrsearch", `filetype:bitmap ${query}`);
  api.searchParams.set("gsrnamespace", "6");
  api.searchParams.set("gsrlimit", "12");
  api.searchParams.set("prop", "imageinfo");
  api.searchParams.set("iiprop", "url|mime|extmetadata|size");
  api.searchParams.set("iiurlwidth", "1280");
  api.searchParams.set("origin", "*");

  try {
    const res = await fetch(api.toString(), {
      headers: { "User-Agent": WIKI_UA, Accept: "application/json" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as CommonsSearchResponse;
    const pages = sortCommonsPagesByIndex(Object.values(data.query?.pages || {}));
    for (const page of pages) {
      const info = page.imageinfo?.[0];
      const imageUrl = info?.thumburl || info?.url;
      if (!imageUrl) continue;
      if (!isAcceptableTravelCoverSize(info?.width, info?.height)) continue;
      if (skipUrls.has(cleanImageUrl(imageUrl)) || skipUrls.has(imageUrl)) continue;
      const mime = String(info?.mime || "").toLowerCase();
      if (mime && !mime.startsWith("image/")) continue;
      if (mime.includes("svg")) continue;
      const title = (page.title || query).replace(/^File:/i, "").replace(/\.[^.]+$/, "");
      if (!isSchoolSafeCoverText(title, query)) continue;
      const meta = info?.extmetadata || {};
      const artistRaw = meta.Artist?.value || meta.Credit?.value || "";
      const license = meta.LicenseShortName?.value || meta.License?.value || "CC";
      const author = String(artistRaw)
        .replace(/<[^>]+>/g, " ")
        .replace(/\s+/g, " ")
        .trim()
        .slice(0, 160);
      return {
        title,
        imageUrl: cleanImageUrl(imageUrl),
        alternateUrls: [info?.thumburl, info?.url]
          .filter((u): u is string => Boolean(u))
          .map(cleanImageUrl),
        author: author || null,
        license: String(license).slice(0, 80),
        attributionUrl: info?.descriptionurl || null,
        sourcePageUrl: info?.descriptionurl || null,
        source: "wikimedia",
        mime: info?.mime || null,
        thumbUrl: wikimediaThumbAtWidth(imageUrl, VISION_THUMB_WIDTH),
      };
    }
  } catch (err) {
    console.error("[travels-image-web] commons search failed", err);
  }
  return null;
}

type UnsplashSearchResponse = {
  results?: Array<{
    urls?: { regular?: string; full?: string; small?: string };
    user?: { name?: string; links?: { html?: string } };
    links?: { html?: string };
    alt_description?: string | null;
    description?: string | null;
  }>;
};

async function searchUnsplash(
  query: string,
  skipUrls: Set<string>,
): Promise<WebImageHit | null> {
  const key = process.env.UNSPLASH_ACCESS_KEY?.trim();
  if (!key) return null;
  try {
    const api = new URL("https://api.unsplash.com/search/photos");
    api.searchParams.set("query", query);
    api.searchParams.set("per_page", "8");
    api.searchParams.set("orientation", "landscape");
    api.searchParams.set("content_filter", "high");
    const res = await fetch(api.toString(), {
      headers: {
        Authorization: `Client-ID ${key}`,
        "Accept-Version": "v1",
        "User-Agent": WIKI_UA,
      },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as UnsplashSearchResponse;
    for (const photo of data.results || []) {
      const imageUrl = photo?.urls?.regular || photo?.urls?.full;
      if (!imageUrl || skipUrls.has(imageUrl)) continue;
      const title = photo.alt_description || photo.description || query;
      if (!isSchoolSafeCoverText(title, query)) continue;
      return {
        title,
        imageUrl,
        author: photo.user?.name || "Unsplash photographer",
        license: "Unsplash License",
        attributionUrl: photo.user?.links?.html || photo.links?.html || null,
        sourcePageUrl: photo.links?.html || null,
        source: "unsplash",
        mime: "image/jpeg",
        thumbUrl: photo.urls?.small || null,
      };
    }
  } catch (err) {
    console.error("[travels-image-web] unsplash search failed", err);
  }
  return null;
}

type OpenverseResponse = {
  results?: Array<{
    title?: string;
    url?: string;
    thumbnail?: string;
    width?: number;
    height?: number;
    foreign_landing_url?: string;
    creator?: string | null;
    license?: string | null;
    license_version?: string | null;
  }>;
};

/** Repli sans clé API — images CC via Openverse. */
async function searchOpenverse(
  query: string,
  skipUrls: Set<string>,
): Promise<WebImageHit | null> {
  try {
    const api = new URL("https://api.openverse.org/v1/images/");
    api.searchParams.set("q", query);
    api.searchParams.set("page_size", "8");
    api.searchParams.set("license", "cc0,pdm,by,by-sa");
    api.searchParams.set("format", "json");
    const res = await fetch(api.toString(), {
      headers: { "User-Agent": WIKI_UA, Accept: "application/json" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as OpenverseResponse;
    for (const photo of data.results || []) {
      const imageUrl = photo.url;
      if (!imageUrl || skipUrls.has(imageUrl)) continue;
      if (!isAcceptableTravelCoverSize(photo.width, photo.height)) continue;
      const title = photo.title || query;
      if (!isSchoolSafeCoverText(title, query)) continue;
      const license = [photo.license, photo.license_version].filter(Boolean).join(" ").trim();
      return {
        title,
        imageUrl,
        author: photo.creator || null,
        license: license || "CC",
        attributionUrl: photo.foreign_landing_url || null,
        sourcePageUrl: photo.foreign_landing_url || null,
        source: "openverse",
        mime: null,
        thumbUrl: photo.thumbnail || null,
      };
    }
  } catch (err) {
    console.error("[travels-image-web] openverse search failed", err);
  }
  return null;
}


const SEARCH_BY_SOURCE: Record<
  TravelCoverSource,
  (query: string, skipUrls: Set<string>) => Promise<WebImageHit | null>
> = {
  "wikipedia-fr": (q) => fetchWikipediaSummary(q, "fr"),
  "wikipedia-en": (q) => fetchWikipediaSummary(q, "en"),
  commons: (q, skip) => searchWikimediaCommons(q, skip),
  unsplash: (q, skip) => searchUnsplash(q, skip),
  openverse: (q, skip) => searchOpenverse(q, skip),
};

/** Quelques photos pour une requête, dans l'ordre de sources adapté (lieu ou activité). */
async function resolveWebHitsForQuery(
  query: string,
  kind: TravelCoverQueryKind,
  themeTokens: string[],
  skipUrls: Set<string>,
  opts: { allowWikipedia: boolean; max: number },
): Promise<WebImageHit[]> {
  const hits: WebImageHit[] = [];
  for (const source of travelCoverSourceOrder(kind, opts.allowWikipedia)) {
    if (hits.length >= opts.max) break;
    const hit = await SEARCH_BY_SOURCE[source](query, skipUrls);
    if (!hit?.imageUrl) continue;
    if (skipUrls.has(hit.imageUrl) || skipUrls.has(cleanImageUrl(hit.imageUrl))) continue;
    if (!hitLooksRelevant(hit, query, themeTokens)) {
      console.info("[travels-image-web] skip irrelevant/unsafe hit", {
        query,
        hitTitle: hit.title,
        url: hit.imageUrl.slice(0, 80),
      });
      continue;
    }
    if (!isSchoolSafeCoverText(hit.title, hit.imageUrl, query)) continue;
    hits.push(hit);
  }
  return hits;
}

function queryAllowsWikipedia(query: string): boolean {
  const parts = query
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  // « drone », « uav », etc. : Wikipedia = pages militaires → Openverse/Unsplash only.
  return !parts.some((p) => isWikiRiskyActivityToken(p));
}

type CoverCandidate = { hit: WebImageHit; query: string };

/** Miniature pour la vérification visuelle (petite, pour limiter le coût). */
async function downloadVisionThumb(
  hit: WebImageHit,
): Promise<{ bytes: Buffer; contentType: string } | null> {
  const urls = [hit.thumbUrl, ...(hit.alternateUrls || []), hit.imageUrl].filter(
    (u): u is string => Boolean(u),
  );
  for (const url of urls.slice(0, 2)) {
    const d = await downloadBytes(url);
    // Une image trop lourde coûterait cher en jetons : on la laisse de côté.
    if (d && d.bytes.length <= 1_500_000) {
      return { bytes: d.bytes, contentType: d.contentType };
    }
  }
  return null;
}

/** Télécharge la photo retenue, la copie sur le CDN public et l'ajoute au catalogue. */
async function hostCandidate(
  candidate: CoverCandidate,
  ctx: { title: string; destination: string; placeQueries: Set<string> },
): Promise<TravelCatalogImage | null> {
  const { hit, query } = candidate;
  const downloaded = await downloadBestCandidate(hit);
  if (!downloaded) {
    console.warn("[travels-image-web] download failed", { query, title: hit.title });
    return null;
  }

  const normalizeKey = normalizeTravelImageKey(query);
  const ext = extFromMimeOrUrl(downloaded.contentType || hit.mime, downloaded.sourceUrl);
  const hostedUrl = await uploadCoverToPublicBucket({
    normalizeKey: normalizeKey || "place",
    bytes: downloaded.bytes,
    contentType: downloaded.contentType,
    ext,
  });
  if (!hostedUrl) {
    console.warn("[travels-image-web] CDN upload failed", { query });
    return null;
  }

  const label =
    hit.title ||
    (ctx.placeQueries.has(query.toLowerCase()) ? ctx.destination : "") ||
    query;
  const keywords = [query, ctx.title, ctx.destination, hit.title]
    .join(", ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 400);
  const id = `auto-${normalizeKey || Date.now().toString(36)}`.slice(0, 80);

  const saved = await upsertTravelCatalogImage({
    id,
    label: label.slice(0, 160),
    url: hostedUrl,
    keywords,
    normalizeKey,
    source: hit.source === "openverse" ? "openverse" : hit.source,
    author: hit.author,
    license: hit.license,
    attributionUrl: hit.attributionUrl,
    sourcePageUrl: hit.sourcePageUrl,
  });
  return saved?.url ? saved : null;
}

/**
 * Cherche une image web, la copie sur le CDN public, enrichit le catalogue.
 *
 * 1) Rassemble quelques photos (lieu → Wikipedia d'abord ; activité → banques d'images).
 * 2) Mistral regarde les miniatures et garde celle qui montre vraiment le séjour.
 * 3) Si la vérification visuelle est indisponible : première photo trouvée (ancien comportement).
 */
export async function fetchAndEnrichTravelCoverImage(opts: {
  query?: string;
  title?: string;
  destination?: string;
  /** Requêtes prioritaires (ex. proposées par Mistral après compréhension du sens). */
  preferredQueries?: string[];
  /** Parmi les requêtes prioritaires, celles qui nomment un lieu précis. */
  preferredPlaceQueries?: string[];
  excludeImageUrls?: string[];
  /** Pages source à ne pas reprendre (image actuelle lors d'un « régénérer »). */
  excludeSourcePageUrls?: string[];
}): Promise<TravelCatalogImage | null> {
  const title = String(opts.title || "").trim();
  const destination = String(opts.destination || "").trim();
  const heuristicPlace = buildTravelPlaceOnlyQueries(title, destination);
  const heuristicActivity = buildTravelActivitySafeQueries(title, destination);
  const preferred = (opts.preferredQueries || [])
    .map((q) => String(q || "").trim())
    .filter(Boolean);

  const placeQueries = new Set(
    [...(opts.preferredPlaceQueries || []), ...heuristicPlace, ...(opts.query ? [opts.query] : [])]
      .map((q) => String(q || "").trim().toLowerCase())
      .filter(Boolean),
  );
  const queries = [
    ...preferred,
    ...(opts.query ? [opts.query] : []),
    ...heuristicPlace,
    ...heuristicActivity,
  ].filter((q, i, arr) => q && arr.findIndex((x) => x.toLowerCase() === q.toLowerCase()) === i);

  if (queries.length === 0) return null;

  const themeTokens = strongTravelThemeTokens(title, destination);
  const skipUrls = new Set(
    (opts.excludeImageUrls || []).map((u) => String(u || "").trim()).filter(Boolean),
  );
  const skipPages = new Set(
    (opts.excludeSourcePageUrls || []).map((u) => String(u || "").trim()).filter(Boolean),
  );
  const ctx = { title, destination, placeQueries };

  // Deux vagues de 3 requêtes au plus : la 2e seulement si rien ne convient dans la 1re.
  const seen = new Set<string>();
  for (let start = 0; start < Math.min(queries.length, 6); start += 3) {
    const batch = queries.slice(start, start + 3);
    const perQuery = await Promise.all(
      batch.map((query) =>
        resolveWebHitsForQuery(
          query,
          placeQueries.has(query.toLowerCase()) ? "place" : "activity",
          themeTokens,
          skipUrls,
          { allowWikipedia: queryAllowsWikipedia(query), max: 2 },
        ).then((hits) => hits.map((hit) => ({ hit, query }))),
      ),
    );

    const candidates: CoverCandidate[] = [];
    for (const c of perQuery.flat()) {
      const urlKey = cleanImageUrl(c.hit.imageUrl);
      const pageKey = c.hit.sourcePageUrl || "";
      if (seen.has(urlKey) || (pageKey && seen.has(pageKey))) continue;
      if (pageKey && skipPages.has(pageKey)) continue;
      seen.add(urlKey);
      if (pageKey) seen.add(pageKey);
      candidates.push(c);
      if (candidates.length >= MAX_VISION_CANDIDATES) break;
    }
    if (candidates.length === 0) continue;

    const thumbs = await Promise.all(candidates.map((c) => downloadVisionThumb(c.hit)));
    const withThumb = candidates
      .map((c, i) => ({ c, thumb: thumbs[i] }))
      .filter((x): x is { c: CoverCandidate; thumb: { bytes: Buffer; contentType: string } } =>
        Boolean(x.thumb),
      );

    let order: CoverCandidate[] = candidates;
    if (withThumb.length > 0) {
      const pick = await pickTravelCoverWithVision({
        title,
        destination,
        candidates: withThumb.map((x) => x.thumb),
      });
      if (pick === null) {
        console.info("[travels-image-web] vision: aucune photo ne convient", {
          batch,
          titles: withThumb.map((x) => x.c.hit.title).slice(0, 6),
        });
        continue;
      }
      if (pick !== "unavailable") {
        // Seule la photo validée est hébergée (les autres n'ont pas été jugées bonnes).
        order = [withThumb[pick].c];
      }
    }

    for (const candidate of order) {
      const saved = await hostCandidate(candidate, ctx);
      if (saved) return saved;
    }
  }

  console.warn("[travels-image-web] no usable cover after all queries", {
    queries: queries.slice(0, 8),
  });
  return null;
}
