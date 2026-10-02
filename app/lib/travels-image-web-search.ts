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
  opts?: { allowWikipedia?: boolean },
): Promise<WebImageHit | null> {
  const allowWiki = opts?.allowWikipedia !== false;
  const sources: Array<() => Promise<WebImageHit | null>> = [
    () => searchUnsplash(query, skipUrls),
    () => searchOpenverse(query, skipUrls),
  ];
  if (allowWiki) {
    sources.push(
      () => fetchWikipediaSummary(query, "fr"),
      () => fetchWikipediaSummary(query, "en"),
      () => searchWikimediaCommons(query, skipUrls),
    );
  }

  for (const load of sources) {
    const hit = await load();
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
    return hit;
  }
  return null;
}

function queryAllowsWikipedia(query: string): boolean {
  const parts = query
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  // « drone », « uav », etc. : Wikipedia = pages militaires → Openverse/Unsplash only.
  return !parts.some((p) => isWikiRiskyActivityToken(p));
}

/**
 * Cherche une image web, la copie sur le CDN public, enrichit le catalogue.
 * Enchaîne les requêtes : si download/upload échoue, passe à la suivante.
 */
export async function fetchAndEnrichTravelCoverImage(opts: {
  query?: string;
  title?: string;
  destination?: string;
  excludeImageUrls?: string[];
}): Promise<TravelCatalogImage | null> {
  const title = String(opts.title || "").trim();
  const destination = String(opts.destination || "").trim();
  const placeQueries = buildTravelPlaceOnlyQueries(title, destination);
  const activityQueries = buildTravelActivitySafeQueries(title, destination);
  const queries = [
    ...(opts.query ? [opts.query] : []),
    ...placeQueries,
    ...activityQueries,
  ].filter((q, i, arr) => q && arr.findIndex((x) => x.toLowerCase() === q.toLowerCase()) === i);

  if (queries.length === 0) return null;

  const themeTokens = strongTravelThemeTokens(title, destination);
  const skipUrls = new Set(
    (opts.excludeImageUrls || []).map((u) => String(u || "").trim()).filter(Boolean),
  );

  for (const query of queries) {
    const allowWikipedia = queryAllowsWikipedia(query);
    const hit = await resolveWebHitForQuery(query, themeTokens, skipUrls, {
      allowWikipedia,
    });
    if (!hit) continue;

    skipUrls.add(hit.imageUrl);
    skipUrls.add(cleanImageUrl(hit.imageUrl));

    if (!isSchoolSafeCoverText(hit.title, hit.imageUrl, query)) {
      console.info("[travels-image-web] skip unsafe after resolve", hit.title);
      continue;
    }

    const downloaded = await downloadBestCandidate(hit);
    if (!downloaded) {
      console.warn("[travels-image-web] download failed, try next query", {
        query,
        title: hit.title,
      });
      continue;
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
      console.warn("[travels-image-web] CDN upload failed, try next query", { query });
      continue;
    }

    const label =
      hit.title ||
      (placeQueries.includes(query) ? destination : "") ||
      query;
    const keywords = [query, title, destination, hit.title]
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
    if (saved?.url) return saved;
  }

  console.warn("[travels-image-web] no usable cover after all queries", {
    queries: queries.slice(0, 8),
  });
  return null;
}
