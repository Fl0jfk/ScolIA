import "server-only";
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getPlatformS3Client } from "@/app/lib/s3-clients";
import { SCOLA_IMAGE_BUCKET, scolaImageUrl } from "@/app/lib/scola-image";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import {
  normalizeTravelImageKey,
  upsertTravelCatalogImage,
  type TravelCatalogImage,
} from "@/app/lib/travels-image-catalog-db";

const WIKI_UA =
  "ScolIA/1.0 (school ENT cover images; https://scolia.fr; contact@scolia.fr)";

type WebImageHit = {
  title: string;
  imageUrl: string;
  author?: string | null;
  license?: string | null;
  attributionUrl?: string | null;
  sourcePageUrl?: string | null;
  source: "wikimedia" | "unsplash";
  mime?: string | null;
};

function preferredImageBuckets(): string[] {
  const list = [
    process.env.IMAGE_BUCKET?.trim(),
    SCOLA_IMAGE_BUCKET,
    process.env.BUCKET_NAME?.trim(),
  ].filter((b): b is string => Boolean(b));
  return [...new Set(list)];
}

function publicUrlForBucketKey(bucket: string, key: string): string {
  const encoded = key
    .split("/")
    .map((s) => encodeURIComponent(s))
    .join("/");
  if (bucket === SCOLA_IMAGE_BUCKET || bucket === process.env.IMAGE_BUCKET?.trim()) {
    return scolaImageUrl(key);
  }
  const region = process.env.REGION?.trim() || "fr-par";
  const endpoint = process.env.S3_ENDPOINT?.trim();
  if (endpoint) {
    return `${endpoint.replace(/\/$/, "")}/${bucket}/${encoded}`;
  }
  if (region.startsWith("fr-") || region.includes("par")) {
    return `https://${bucket}.s3.${region}.scw.cloud/${encoded}`;
  }
  return `https://${bucket}.s3.${region}.amazonaws.com/${encoded}`;
}

function s3ClientForBucket(bucket: string): S3Client {
  // Bucket CDN Scaleway : forcer fr-par même si REGION locale = eu-west-3.
  if (bucket === SCOLA_IMAGE_BUCKET || bucket === (process.env.IMAGE_BUCKET?.trim() || "")) {
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
  }
  return getPlatformS3Client();
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
): Promise<{ bytes: Buffer; contentType: string } | null> {
  try {
    const res = await fetch(url, {
      headers: {
        "User-Agent": WIKI_UA,
        Accept: "image/*,*/*",
      },
      redirect: "follow",
    });
    if (!res.ok) return null;
    const contentType = res.headers.get("content-type") || "image/jpeg";
    if (!contentType.startsWith("image/")) return null;
    const ab = await res.arrayBuffer();
    if (!ab.byteLength || ab.byteLength > 12_000_000) return null;
    return { bytes: Buffer.from(ab), contentType };
  } catch (err) {
    console.error("[travels-image-web] download failed", url, err);
    return null;
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

  for (const bucket of preferredImageBuckets()) {
    try {
      const client = s3ClientForBucket(bucket);
      await client.send(
        new PutObjectCommand({
          Bucket: bucket,
          Key: key,
          Body: opts.bytes,
          ContentType: opts.contentType,
          CacheControl: "public, max-age=31536000, immutable",
        }),
      );
      return publicUrlForBucketKey(bucket, key);
    } catch (err) {
      console.warn(
        `[travels-image-web] S3 upload failed on ${bucket}:`,
        err instanceof Error ? err.message : err,
      );
    }
  }
  return null;
}

type WikiSummary = {
  title?: string;
  content_urls?: { desktop?: { page?: string } };
  originalimage?: { source?: string };
  thumbnail?: { source?: string };
  type?: string;
};

async function fetchWikipediaSummary(query: string, lang: "fr" | "en"): Promise<WebImageHit | null> {
  const titleGuess = query.trim().replace(/\s+/g, "_");
  const urls = [
    `https://${lang}.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(titleGuess)}`,
    `https://${lang}.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(query.trim())}`,
  ];
  for (const url of urls) {
    try {
      const res = await fetch(url, {
        headers: { "User-Agent": WIKI_UA, Accept: "application/json" },
      });
      if (!res.ok) continue;
      const data = (await res.json()) as WikiSummary;
      if (data.type === "disambiguation") continue;
      const imageUrl = data.originalimage?.source || data.thumbnail?.source;
      if (!imageUrl) continue;
      return {
        title: data.title || query,
        imageUrl,
        author: null,
        license: "Wikipedia / Wikimedia",
        attributionUrl: data.content_urls?.desktop?.page || null,
        sourcePageUrl: data.content_urls?.desktop?.page || null,
        source: "wikimedia",
        mime: null,
      };
    } catch {
      /* try next */
    }
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
          mime?: string;
          descriptionurl?: string;
          extmetadata?: Record<string, { value?: string }>;
        }>;
      }
    >;
  };
};

async function searchWikimediaCommons(query: string): Promise<WebImageHit | null> {
  const api = new URL("https://commons.wikimedia.org/w/api.php");
  api.searchParams.set("action", "query");
  api.searchParams.set("format", "json");
  api.searchParams.set("generator", "search");
  api.searchParams.set("gsrsearch", `filetype:bitmap ${query}`);
  api.searchParams.set("gsrnamespace", "6");
  api.searchParams.set("gsrlimit", "8");
  api.searchParams.set("prop", "imageinfo");
  api.searchParams.set("iiprop", "url|mime|extmetadata|size");
  api.searchParams.set("iiurlwidth", "1600");
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
      if (!info?.url) continue;
      const mime = String(info.mime || "").toLowerCase();
      if (mime && !mime.startsWith("image/")) continue;
      if (mime.includes("svg")) continue;
      const meta = info.extmetadata || {};
      const artistRaw = meta.Artist?.value || meta.Credit?.value || "";
      const license = meta.LicenseShortName?.value || meta.License?.value || "CC";
      const author = String(artistRaw)
        .replace(/<[^>]+>/g, " ")
        .replace(/\s+/g, " ")
        .trim()
        .slice(0, 160);
      return {
        title: (page.title || query).replace(/^File:/i, "").replace(/\.[^.]+$/, ""),
        imageUrl: info.url,
        author: author || null,
        license: String(license).slice(0, 80),
        attributionUrl: info.descriptionurl || null,
        sourcePageUrl: info.descriptionurl || null,
        source: "wikimedia",
        mime: info.mime || null,
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

async function searchUnsplash(query: string): Promise<WebImageHit | null> {
  const key = process.env.UNSPLASH_ACCESS_KEY?.trim();
  if (!key) return null;
  try {
    const api = new URL("https://api.unsplash.com/search/photos");
    api.searchParams.set("query", query);
    api.searchParams.set("per_page", "5");
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
    const photo = data.results?.[0];
    const imageUrl = photo?.urls?.regular || photo?.urls?.full;
    if (!imageUrl || !photo) return null;
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
  } catch (err) {
    console.error("[travels-image-web] unsplash search failed", err);
    return null;
  }
}

async function resolveWebHit(query: string): Promise<WebImageHit | null> {
  const wikiFr = await fetchWikipediaSummary(query, "fr");
  if (wikiFr) return wikiFr;
  const wikiEn = await fetchWikipediaSummary(query, "en");
  if (wikiEn) return wikiEn;
  const commons = await searchWikimediaCommons(query);
  if (commons) return commons;
  return searchUnsplash(query);
}

/**
 * Cherche une image web (Wikimedia puis Unsplash), la copie sur le CDN public,
 * et l'enregistre dans le catalogue partagé pour réutilisation.
 * Si l'upload S3 échoue (env local sans bucket images), conserve l'URL source + attribution.
 */
export async function fetchAndEnrichTravelCoverImage(opts: {
  query: string;
  title?: string;
  destination?: string;
}): Promise<TravelCatalogImage | null> {
  const query = String(opts.query || "").trim();
  if (!query) return null;

  const hit = await resolveWebHit(query);
  if (!hit) return null;

  const downloaded = await downloadBytes(hit.imageUrl);
  let hostedUrl: string | null = null;
  if (downloaded) {
    const normalizeKey = normalizeTravelImageKey(query);
    const ext = extFromMimeOrUrl(downloaded.contentType || hit.mime, hit.imageUrl);
    hostedUrl = await uploadCoverToPublicBucket({
      normalizeKey: normalizeKey || "place",
      bytes: downloaded.bytes,
      contentType: downloaded.contentType,
      ext,
    });
  }

  // Repli : URL Wikimedia/Unsplash (attribution conservée) si copie S3 impossible.
  const finalUrl = hostedUrl || hit.imageUrl;
  if (!finalUrl) return null;

  const normalizeKey = normalizeTravelImageKey(query);
  const label =
    String(opts.destination || "").trim() ||
    hit.title ||
    query;
  const keywords = [
    query,
    opts.title || "",
    opts.destination || "",
    hit.title,
  ]
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
    source: hit.source,
    author: hit.author,
    license: hit.license,
    attributionUrl: hit.attributionUrl,
    sourcePageUrl: hit.sourcePageUrl,
  });
}
