import "server-only";

import { getJson, putJson, putObject, getObjectBytes } from "@/app/lib/s3-storage";
import type {
  GeneratedPoster,
  GeneratedPosterIndexEntry,
  PosterFormat,
} from "@/app/lib/posters/types";

const INDEX_KEY = "posters/generated/index.json";
const MAX_INDEX = 80;

function generatedMetaKey(id: string) {
  return `posters/generated/${id}.json`;
}

export function posterGeneratedFileKey(id: string) {
  return `posters/generated/${id}.pdf`;
}

export function posterAssetKey(
  kind: "partner-logo" | "background" | "image",
  fileName: string,
) {
  const safe = fileName.replace(/[^a-zA-Z0-9._-]+/g, "_").slice(0, 80);
  return `posters/assets/${kind}/${Date.now()}-${safe}`;
}

export async function loadPosterGeneratedIndex(): Promise<GeneratedPosterIndexEntry[]> {
  const hit = await getJson<GeneratedPosterIndexEntry[]>(INDEX_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function savePosterGeneratedIndex(entries: GeneratedPosterIndexEntry[]) {
  await putJson(INDEX_KEY, entries.slice(0, MAX_INDEX));
}

export async function saveGeneratedPoster(
  doc: GeneratedPoster,
  bytes: Uint8Array | Buffer,
): Promise<GeneratedPoster> {
  const fileKey = doc.fileKey || posterGeneratedFileKey(doc.id);
  await putObject(fileKey, Buffer.from(bytes), "application/pdf");
  const stored: GeneratedPoster = { ...doc, fileKey };
  await putJson(generatedMetaKey(doc.id), stored);
  const index = await loadPosterGeneratedIndex();
  const entry: GeneratedPosterIndexEntry = {
    id: stored.id,
    templateId: stored.templateId,
    templateLabel: stored.templateLabel,
    title: stored.title,
    createdAt: stored.createdAt,
    format: stored.format,
  };
  await savePosterGeneratedIndex([entry, ...index.filter((e) => e.id !== doc.id)]);
  return stored;
}

export async function loadGeneratedPoster(id: string): Promise<GeneratedPoster | null> {
  const hit = await getJson<GeneratedPoster>(generatedMetaKey(id));
  return hit?.data ?? null;
}

export async function loadGeneratedPosterBytes(id: string): Promise<Buffer | null> {
  const meta = await loadGeneratedPoster(id);
  if (!meta?.fileKey) return null;
  return getObjectBytes(meta.fileKey);
}

export async function savePosterAsset(
  kind: "partner-logo" | "background" | "image",
  fileName: string,
  bytes: Buffer,
  contentType: string,
): Promise<{ key: string }> {
  const key = posterAssetKey(kind, fileName);
  await putObject(key, bytes, contentType || "application/octet-stream");
  return { key };
}

export function formatLabel(format: PosterFormat): string {
  if (format === "a3-landscape") return "A3 paysage";
  if (format === "a5-portrait") return "A5 (×4 sur A4)";
  return "A4 portrait";
}
