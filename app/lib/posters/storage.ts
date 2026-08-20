import "server-only";

import { getJson, putJson, putObject, getObjectBytes, getSignedReadUrl } from "@/app/lib/s3-storage";
import { draftDisplayTitle } from "@/app/lib/posters/catalog";
import type {
  GeneratedPoster,
  GeneratedPosterIndexEntry,
  PosterDraft,
  PosterFormat,
  PosterSavedDraft,
  PosterSavedDraftIndexEntry,
} from "@/app/lib/posters/types";

const INDEX_KEY = "posters/generated/index.json";
const MAX_INDEX = 80;

const DRAFTS_INDEX_KEY = "posters/drafts/index.json";
export const MAX_POSTER_DRAFTS = 5;

function generatedMetaKey(id: string) {
  return `posters/generated/${id}.json`;
}

function draftMetaKey(id: string) {
  return `posters/drafts/${id}.json`;
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

export async function loadPosterDraftsIndex(): Promise<PosterSavedDraftIndexEntry[]> {
  const hit = await getJson<PosterSavedDraftIndexEntry[]>(DRAFTS_INDEX_KEY);
  return Array.isArray(hit?.data) ? hit.data : [];
}

async function savePosterDraftsIndex(entries: PosterSavedDraftIndexEntry[]) {
  await putJson(DRAFTS_INDEX_KEY, entries.slice(0, MAX_POSTER_DRAFTS));
}

export async function loadPosterDraft(id: string): Promise<PosterSavedDraft | null> {
  const safe = String(id || "").replace(/[^a-zA-Z0-9_-]/g, "");
  if (!safe) return null;
  const hit = await getJson<PosterSavedDraft>(draftMetaKey(safe));
  return hit?.data ?? null;
}

export async function savePosterDraft(input: {
  id?: string;
  draft: PosterDraft;
  createdBy?: PosterSavedDraft["createdBy"];
}): Promise<PosterSavedDraft> {
  const now = new Date().toISOString();
  const existingId = input.id?.replace(/[^a-zA-Z0-9_-]/g, "") || "";
  const prev = existingId ? await loadPosterDraft(existingId) : null;
  const id =
    prev?.id ||
    `pdraft_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;

  const title = draftDisplayTitle(input.draft);
  const stored: PosterSavedDraft = {
    id,
    title,
    createdAt: prev?.createdAt || now,
    updatedAt: now,
    format: input.draft.format,
    draft: input.draft,
    createdBy: input.createdBy || prev?.createdBy,
  };

  await putJson(draftMetaKey(id), stored);

  let index = await loadPosterDraftsIndex();
  index = [
    { id, title, updatedAt: now, format: stored.format },
    ...index.filter((e) => e.id !== id),
  ];

  const kept = index.slice(0, MAX_POSTER_DRAFTS);
  await savePosterDraftsIndex(kept);
  return stored;
}

export async function deletePosterDraft(id: string): Promise<boolean> {
  const safe = String(id || "").replace(/[^a-zA-Z0-9_-]/g, "");
  if (!safe) return false;
  const index = await loadPosterDraftsIndex();
  await savePosterDraftsIndex(index.filter((e) => e.id !== safe));
  return true;
}

/** URLs signées pour réafficher un brouillon. */
export async function resolvePosterDraftAssetUrls(draft: PosterDraft): Promise<{
  partnerLogoUrl: string | null;
  backgroundImageUrl: string | null;
  imageUrls: Record<string, string>;
}> {
  const imageUrls: Record<string, string> = {};
  let partnerLogoUrl: string | null = null;
  let backgroundImageUrl: string | null = null;

  if (draft.partnerLogoKey) {
    partnerLogoUrl = (await getSignedReadUrl(draft.partnerLogoKey, 3600)) || null;
  }
  if (draft.backgroundImageKey) {
    backgroundImageUrl = (await getSignedReadUrl(draft.backgroundImageKey, 3600)) || null;
  }
  for (const el of draft.elements) {
    if (el.imageKey && !imageUrls[el.imageKey]) {
      const url = await getSignedReadUrl(el.imageKey, 3600);
      if (url) imageUrls[el.imageKey] = url;
    }
  }
  return { partnerLogoUrl, backgroundImageUrl, imageUrls };
}
