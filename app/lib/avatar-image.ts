import "server-only";

import sharp from "sharp";
import { sniffImageType } from "@/app/lib/travels-feed-image";

export const AVATAR_MAX_INPUT_BYTES = 8 * 1024 * 1024;
export const AVATAR_SIDE = 480;
const MAX_INPUT_PIXELS = 60_000_000;

export class AvatarError extends Error {
  readonly status: number;
  constructor(message: string, status = 400) {
    super(message);
    this.name = "AvatarError";
    this.status = status;
  }
}

/**
 * Contrôle des octets, recadrage carré centré sur le sujet, ré-encodage JPEG sans métadonnées
 * (ni position GPS). Le fichier envoyé n'est jamais conservé tel quel.
 */
export async function toAvatarJpeg(input: Buffer): Promise<Buffer> {
  if (input.length > AVATAR_MAX_INPUT_BYTES) throw new AvatarError("Photo trop lourde (8 Mo au maximum).");
  const type = sniffImageType(input);
  if (!type) throw new AvatarError("Format non reconnu : choisissez une photo JPEG, PNG ou WebP.");
  try {
    return await sharp(input, { failOn: "error", limitInputPixels: MAX_INPUT_PIXELS })
      .rotate()
      .resize({ width: AVATAR_SIDE, height: AVATAR_SIDE, fit: "cover", position: sharp.strategy.attention })
      .flatten({ background: "#ffffff" })
      .jpeg({ quality: 82, mozjpeg: true })
      .toBuffer();
  } catch {
    throw new AvatarError(
      type === "heic"
        ? "Photo HEIC non lisible : enregistrez-la en JPEG puis réessayez."
        : "Photo illisible ou abîmée.",
    );
  }
}
