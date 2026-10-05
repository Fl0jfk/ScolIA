import "server-only";

import {
  isAllowedMime,
  maxBytesForMime,
} from "@/app/lib/messaging/constants";
import { putObject } from "@/app/lib/s3-storage";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import type { FamilleAttachmentInput } from "@/app/lib/famille-messaging-db";

const INLINE_MAX_BYTES = 900_000;

export async function storeFamilleAttachmentFile(opts: {
  etablissementId: string;
  threadOrTempId: string;
  file: File;
}): Promise<FamilleAttachmentInput> {
  const mime = opts.file.type || "application/octet-stream";
  if (!isAllowedMime(mime)) {
    throw new Error(`Format non accepté (${mime}).`);
  }
  const maxBytes = Math.min(maxBytesForMime(mime), 10 * 1024 * 1024);
  if (opts.file.size > maxBytes) {
    throw new Error(`Fichier trop volumineux (max ${Math.round(maxBytes / (1024 * 1024))} Mo).`);
  }
  const buffer = Buffer.from(await opts.file.arrayBuffer());
  const safeName = sanitizeS3FileName(opts.file.name).replace(/\s+/g, "_");
  const rel = `famille-messaging/${opts.etablissementId}/${opts.threadOrTempId}/${Date.now()}-${safeName}`;

  try {
    const s3Key = await putObject(rel, buffer, mime);
    return {
      fileName: opts.file.name,
      mime,
      size: opts.file.size,
      s3Key,
      contentBase64: null,
    };
  } catch (err) {
    // Repli labo local si S3 indisponible.
    if (buffer.length > INLINE_MAX_BYTES) {
      throw err instanceof Error
        ? err
        : new Error("Upload S3 impossible et fichier trop grand pour le repli local.");
    }
    return {
      fileName: opts.file.name,
      mime,
      size: opts.file.size,
      s3Key: null,
      contentBase64: buffer.toString("base64"),
    };
  }
}
