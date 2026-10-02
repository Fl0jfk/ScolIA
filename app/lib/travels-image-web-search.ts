import "server-only";
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getPlatformS3Client } from "@/app/lib/s3-clients";
import { SCOLA_IMAGE_BUCKET, scolaImageUrl } from "@/app/lib/scola-image";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import {
  buildTravelWebSearchQueries,
  normalizeTravelImageKey,
  tokenizeTravelPlaceQuery,
  upsertTravelCatalogImage,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";

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

/** Refuse une image web hors sujet (ex. cimetière pour une requête surf). */
function hitLooksRelevant(hit: WebImageHit, query: string, themeTokens: string[]): boolean {
  const titleHay = normalizeHay(hit.title);
  const queryHay = normalizeHay(query);
  const distinctive = themeTokens.filter((t) => t.length >= 4);
  if (distinctive.length === 0) return true;

  // Tokens du thème présents dans CETTE requête (ex. « surf » dans « joueur surf »).
  const themeInQuery = distinctive.filter((t) => queryHay.includes(t));
  if (themeInQuery.length === 0) {
    // Requête = lieu seul (ex. « Rouen ») : on accepte le résultat lieu.
    return true;
  }
  // Le titre de l’image doit coller au thème (pas seulement à la ville).
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
    };
  } catch {
    return null;
  }
}

async function fetchWikipediaSummary(query: string, lang: "fr" | "en"): Promise<WebImageHit | null> {
  const titleGuess = query.trim().replace(/\s+/g, "_");
  const direct =
    (await wikiSummaryFromTitle(titleGuess, lang)) ||
    (await wikiSummaryFromTitle(query.trim(), lang));
  if (direct) return direct;

  // Recherche plein texte : « joueur surf » → page « Surf »
  try {
    const api = new URL(`https://${lang}.wikipedia.org/w/api.php`);
    api.searchParams.set("action", "query");
    api.searchParams.set("list", "search");
    api.searchParams.set("srsearch", query);
    api.searchParams.set("srlimit", "5");
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
    for (const row of data.query?.search || []) {
      const title = String(row.title || "").trim();
      if (!title) continue;
      const hit = await wikiSummaryFromTitle(title, lang);
      if (hit) return hit;
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
        imageinfo?: Array<{
          url?: string;
          thumburl?: string;
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
    const pages = Object.values(data.query?.pages || {});
    for (const page of pages) {
      const info = page.imageinfo?.[0];
      const imageUrl = info?.thumburl || info?.url;
      if (!imageUrl) continue;
      if (skipUrls.has(cleanImageUrl(imageUrl)) || skipUrls.has(imageUrl)) continue;
      const mime = String(info?.mime || "").toLowerCase();
      if (mime && !mime.startsWith("image/")) continue;
      if (mime.includes("svg")) continue;
      const meta = info?.extmetadata || {};
      const artistRaw = meta.Artist?.value || meta.Credit?.value || "";
      const license = meta.LicenseShortName?.value || meta.License?.value || "CC";
      const author = String(artistRaw)
        .replace(/<[^>]+>/g, " ")
        .replace(/\s+/g, " ")
        .trim()
        .slice(0, 160);
      return {
        title: (page.title || query).replace(/^File:/i, "").replace(/\.[^.]+$/, ""),
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
      };
    }
  } catch (err) {
    console.error("[travels-image-web] commons search failed", err);
  }
  return null;
}

type UnsplashSearchResponse = {
  results?: Array<{
    urls?: { regular?: string; full?: string };
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
      return {
        title: photo.alt_description || photo.description || query,
        imageUrl,
        author: photo.user?.name || "Unsplash photographer",
        license: "Unsplash License",
        attributionUrl: photo.user?.links?.html || photo.links?.html || null,
        sourcePageUrl: photo.links?.html || null,
        source: "unsplash",
        mime: "image/jpeg",
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
      const license = [photo.license, photo.license_version].filter(Boolean).join(" ").trim();
      return {
        title: photo.title || query,
        imageUrl,
        author: photo.creator || null,
        license: license || "CC",
        attributionUrl: photo.foreign_landing_url || null,
        sourcePageUrl: photo.foreign_landing_url || null,
        source: "openverse",
        mime: null,
      };
    }
  } catch (err) {
    console.error("[travels-image-web] openverse search failed", err);
  }
  return null;
}

async function resolveWebHitForQuery(
  query: string,
  themeTokens: string[],
  skipUrls: Set<string>,
): Promise<WebImageHit | null> {
  const candidates: Array<WebImageHit | null> = [
    await fetchWikipediaSummary(query, "fr"),
    await fetchWikipediaSummary(query, "en"),
    await searchWikimediaCommons(query, skipUrls),
    await searchUnsplash(query, skipUrls),
    await searchOpenverse(query, skipUrls),
  ];

  for (const hit of candidates) {
    if (!hit?.imageUrl) continue;
    if (skipUrls.has(hit.imageUrl)) continue;
    if (!hitLooksRelevant(hit, query, themeTokens)) {
      console.info(
        "[travels-image-web] skip irrelevant hit",
        { query, hitTitle: hit.title },
      );
      continue;
    }
    return hit;
  }
  return null;
}

async function resolveWebHit(
  queries: string[],
  themeTokens: string[],
  skipUrls: Set<string>,
): Promise<{ hit: WebImageHit; query: string } | null> {
  for (const query of queries) {
    const hit = await resolveWebHitForQuery(query, themeTokens, skipUrls);
    if (hit) return { hit, query };
  }
  return null;
}

/**
 * Cherche une image web (Wikimedia / Unsplash / Openverse), la copie sur le CDN public,
 * et l'enregistre dans le catalogue partagé pour réutilisation.
 */
export async function fetchAndEnrichTravelCoverImage(opts: {
  query?: string;
  title?: string;
  destination?: string;
  /** URLs à éviter (régénération). */
  excludeImageUrls?: string[];
}): Promise<TravelCatalogImage | null> {
  const title = String(opts.title || "").trim();
  const destination = String(opts.destination || "").trim();
  const queries = [
    ...(opts.query ? [opts.query] : []),
    ...buildTravelWebSearchQueries(title, destination),
  ].filter((q, i, arr) => q && arr.findIndex((x) => x.toLowerCase() === q.toLowerCase()) === i);

  if (queries.length === 0) return null;

  const themeTokens = tokenizeTravelPlaceQuery(title, destination);
  const skipUrls = new Set(
    (opts.excludeImageUrls || []).map((u) => String(u || "").trim()).filter(Boolean),
  );

  const resolved = await resolveWebHit(queries, themeTokens, skipUrls);
  if (!resolved) {
    console.warn("[travels-image-web] no relevant hit", { queries: queries.slice(0, 5) });
    return null;
  }

  const { hit, query } = resolved;
  const downloaded = await downloadBestCandidate(hit);
  if (!downloaded) {
    console.warn("[travels-image-web] could not download image bytes", {
      query,
      title: hit.title,
      url: hit.imageUrl,
    });
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

  // Obligatoire : URL sur le CDN public scolia-images (pas de bucket privé, pas d’URL wiki brute
  // que next/image n’arrive pas à optimiser côté serveur).
  const finalUrl = hostedUrl;
  if (!finalUrl) {
    console.error(
      "[travels-image-web] public CDN upload failed — refuse d’enregistrer une URL cassée",
      { query, source: hit.source },
    );
    return null;
  }

  const label = hit.title || String(opts.destination || "").trim() || query;
  const keywords = [query, title, destination, hit.title]
    .join(", ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 400);

  const id = `auto-${normalizeKey || Date.now().toString(36)}`.slice(0, 80);

  return upsertTravelCatalogImage({
    id,
    label: label.slice(0, 160),
    url: finalUrl,
    keywords,
    normalizeKey,
    source: hit.source === "openverse" ? "openverse" : hit.source,
    author: hit.author,
    license: hit.license,
    attributionUrl: hit.attributionUrl,
    sourcePageUrl: hit.sourcePageUrl,
  });
}
