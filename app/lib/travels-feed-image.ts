import "server-only";

import sharp from "sharp";

/**
 * Nettoyage des photos du « fil du voyage » : contrôle des OCTETS (jamais l'extension ni le type annoncé),
 * retrait de TOUTES les métadonnées (EXIF dont la position GPS, XMP, IPTC, miniature), orientation appliquée,
 * redimensionnement (~1600 px) et ré-encodage JPEG. Ce que le navigateur a envoyé n'est jamais stocké tel quel.
 */

export const FEED_IMAGE_MAX_INPUT_BYTES = 12 * 1024 * 1024;
export const FEED_IMAGE_MAX_SIDE = 1600;
/** Garde-fou « bombe de décompression ». */
const MAX_INPUT_PIXELS = 60_000_000;

export type SniffedImage = "jpeg" | "png" | "webp" | "heic" | null;

export function sniffImageType(b: Buffer): SniffedImage {
  if (b.length < 12) return null;
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "jpeg";
  if (b.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))) return "png";
  if (b.subarray(0, 4).toString("latin1") === "RIFF" && b.subarray(8, 12).toString("latin1") === "WEBP") return "webp";
  if (b.subarray(4, 8).toString("latin1") === "ftyp") {
    const brand = b.subarray(8, 12).toString("latin1");
    if (["heic", "heix", "hevc", "hevx", "heim", "heis", "mif1", "msf1"].includes(brand)) return "heic";
  }
  return null;
}

export type CleanImage = { bytes: Buffer; width: number; height: number; contentType: "image/jpeg" };

export class FeedImageError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FeedImageError";
  }
}

/** File d'attente simple : au plus 2 traitements sharp en parallèle par instance (mémoire et CPU). */
const MAX_PARALLEL = 2;
let running = 0;
const waiting: (() => void)[] = [];
async function withSlot<T>(fn: () => Promise<T>): Promise<T> {
  while (running >= MAX_PARALLEL) await new Promise<void>((resolve) => waiting.push(resolve));
  running += 1;
  try {
    return await fn();
  } finally {
    running -= 1;
    waiting.shift()?.();
  }
}

export function sanitizeFeedImage(input: Buffer): Promise<CleanImage> {
  return withSlot(() => sanitizeFeedImageNow(input));
}

async function sanitizeFeedImageNow(input: Buffer): Promise<CleanImage> {
  if (input.length > FEED_IMAGE_MAX_INPUT_BYTES) throw new FeedImageError("Photo trop volumineuse.");
  const type = sniffImageType(input);
  if (!type) throw new FeedImageError("Format de photo non reconnu (JPEG, PNG, WebP ou HEIC).");
  try {
    const { data, info } = await sharp(input, { failOn: "error", limitInputPixels: MAX_INPUT_PIXELS })
      .rotate() // applique l'orientation EXIF puis la retire
      .resize({ width: FEED_IMAGE_MAX_SIDE, height: FEED_IMAGE_MAX_SIDE, fit: "inside", withoutEnlargement: true })
      .flatten({ background: "#ffffff" })
      // Aucun `withMetadata` / `keepExif` : sharp ne recopie ni EXIF, ni GPS, ni XMP, ni IPTC, ni miniature.
      .jpeg({ quality: 80, mozjpeg: true })
      .toBuffer({ resolveWithObject: true });
    return { bytes: data, width: info.width, height: info.height, contentType: "image/jpeg" };
  } catch (e) {
    if (type === "heic") {
      throw new FeedImageError("Photo HEIC non lisible sur le serveur : réglez l'appareil sur « Le plus compatible » ou réessayez depuis la galerie.");
    }
    if (e instanceof FeedImageError) throw e;
    throw new FeedImageError("Photo illisible ou corrompue.");
  }
}
