import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { getObjectBytes } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

/** Clé S3 stable — une carte d’assistance par tenant (bucket privé). */
export function travelsAssistanceCardObjectKey(): string {
  return s3Key("settings/travels/assistance-card.pdf");
}

export function sanitizeAssistanceCardFileName(name: string): string {
  const base = name.replace(/[^a-zA-Z0-9._-àâäéèêëïîôùûüçÀÂÄÉÈÊËÏÎÔÙÛÜÇ ]/g, "_").trim();
  return base.slice(0, 120) || "carte-assistance.pdf";
}

export function travelsAssistanceCardDownloadApiPath(): string {
  return "/api/travels/assistance-card?raw=1";
}

export async function resolveTravelsAssistanceCardMeta(): Promise<{
  s3Key: string;
  fileName: string;
} | null> {
  const bundle = await loadAppConfig();
  const configured = bundle.travels.assistanceCardS3Key?.trim();
  if (!configured) return null;
  const key = s3Key(configured.split("?")[0]);
  const bytes = await getObjectBytes(key);
  if (!bytes?.length) return null;
  const fileName =
    bundle.travels.assistanceCardFileName?.trim() || "carte-assistance.pdf";
  return { s3Key: key, fileName: sanitizeAssistanceCardFileName(fileName) };
}

export async function resolveTravelsAssistanceCardBytes(): Promise<{
  bytes: Uint8Array;
  fileName: string;
} | null> {
  const meta = await resolveTravelsAssistanceCardMeta();
  if (!meta) return null;
  const buf = await getObjectBytes(meta.s3Key);
  if (!buf?.length) return null;
  return { bytes: new Uint8Array(buf), fileName: meta.fileName };
}
