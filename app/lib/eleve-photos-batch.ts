import "server-only";

import { randomBytes } from "crypto";
import { after } from "next/server";
import {
  deleteObject,
  getJson,
  getObjectBytes,
  putJson,
  putObject,
} from "@/app/lib/s3-storage";
import {
  elevePhotoLookupKey,
  identityKey,
  matchEleveForPhoto,
  parsePhotoFilename,
  photoRelativePathForEleve,
} from "@/app/lib/eleve-photos-match";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevePhotoIndex, type ElevePhotoIndex } from "@/app/lib/eleve-photos";
import { sanitizeS3FileName } from "@/app/lib/s3-path";

export type ElevePhotoJobItemStatus =
  | "pending"
  | "matched"
  | "unmatched"
  | "failed";

export type ElevePhotoJobItem = {
  id: string;
  fileName: string;
  stagingKey: string;
  contentType: string;
  status: ElevePhotoJobItemStatus;
  error?: string;
};

export type ElevePhotoJobStatus =
  | "uploading"
  | "queued"
  | "processing"
  | "completed"
  | "failed";

export type ElevePhotoJob = {
  jobId: string;
  userId: string;
  status: ElevePhotoJobStatus;
  startedAt: string;
  updatedAt: string;
  items: ElevePhotoJobItem[];
  matched: number;
  updated: number;
  unmatched: string[];
  errors: string[];
  percent: number;
  label: string;
  error?: string;
};

const JOB_PREFIX = "eleves/photo-jobs/";
const INBOX_PREFIX = "eleves/photos/inbox/";
/** Budget d’une invocation worker — on s’arrête avant le timeout Scaleway. */
const RUN_BUDGET_MS = 45_000;

function jobKey(jobId: string): string {
  return `${JOB_PREFIX}${jobId}.json`;
}

export function newElevePhotoJobId(): string {
  return `photos_${Date.now()}_${randomBytes(4).toString("hex")}`;
}

export function isValidElevePhotoJobId(jobId: string): boolean {
  return /^photos_\d{10,16}_[a-zA-Z0-9]{6,32}$/.test(jobId);
}

export async function readElevePhotoJob(jobId: string): Promise<ElevePhotoJob | null> {
  const hit = await getJson<ElevePhotoJob>(jobKey(jobId));
  return hit?.data ?? null;
}

async function writeElevePhotoJob(job: ElevePhotoJob): Promise<void> {
  await putJson(jobKey(job.jobId), {
    ...job,
    updatedAt: new Date().toISOString(),
  });
}

export async function createOrGetElevePhotoJob(
  jobId: string,
  userId: string,
): Promise<ElevePhotoJob> {
  if (!isValidElevePhotoJobId(jobId)) {
    throw new Error("Identifiant de job invalide.");
  }
  const existing = await readElevePhotoJob(jobId);
  if (existing) {
    if (existing.userId !== userId) {
      throw new Error("Job photos réservé à un autre utilisateur.");
    }
    return existing;
  }
  const job: ElevePhotoJob = {
    jobId,
    userId,
    status: "uploading",
    startedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    items: [],
    matched: 0,
    updated: 0,
    unmatched: [],
    errors: [],
    percent: 0,
    label: "Réception des fichiers…",
  };
  await writeElevePhotoJob(job);
  return job;
}

function stagingRelativeKey(jobId: string, fileName: string): string {
  const safe = sanitizeS3FileName(fileName);
  return `${INBOX_PREFIX}${jobId}/${Date.now()}_${safe}`;
}

/** Stocke un lot d’images en boîte d’entrée S3 (écrasement final plus tard). */
export async function stageElevePhotoFiles(
  jobId: string,
  userId: string,
  files: { filename: string; bytes: Uint8Array; contentType: string }[],
): Promise<ElevePhotoJob> {
  const job = await createOrGetElevePhotoJob(jobId, userId);
  if (job.status !== "uploading" && job.status !== "queued") {
    throw new Error("Ce job n’accepte plus de nouveaux fichiers.");
  }

  const seen = new Set(job.items.map((i) => `${i.fileName}:${i.stagingKey}`));
  for (const file of files) {
    const name = file.filename || "photo.jpg";
    if (!/\.(jpe?g|png|webp|gif)$/i.test(name)) continue;
    if (!file.bytes.length) continue;

    const stagingKey = stagingRelativeKey(jobId, name);
    const dedupe = `${name}:${stagingKey}`;
    if (seen.has(dedupe)) continue;
    seen.add(dedupe);

    const ct = file.contentType.startsWith("image/") ? file.contentType : "image/jpeg";
    await putObject(stagingKey, file.bytes, ct);
    job.items.push({
      id: `item_${job.items.length + 1}_${randomBytes(2).toString("hex")}`,
      fileName: name,
      stagingKey,
      contentType: ct,
      status: "pending",
    });
  }

  job.status = "uploading";
  job.label = `${job.items.length} fichier(s) reçu(s) — continuez l’envoi ou lancez le traitement.`;
  job.percent = 0;
  await writeElevePhotoJob(job);
  return job;
}

export async function queueElevePhotoJob(jobId: string, userId: string): Promise<ElevePhotoJob> {
  const job = await readElevePhotoJob(jobId);
  if (!job) throw new Error("Job introuvable.");
  if (job.userId !== userId) throw new Error("Job photos réservé à un autre utilisateur.");
  if (!job.items.length) throw new Error("Aucun fichier à traiter.");
  if (job.status === "processing" || job.status === "completed") return job;

  job.status = "queued";
  job.label = `File d’attente — ${job.items.length} photo(s) à associer…`;
  job.percent = 0;
  await writeElevePhotoJob(job);
  return job;
}

async function processOneItem(
  item: ElevePhotoJobItem,
  eleves: EleveConfig[],
  index: ElevePhotoIndex,
): Promise<{
  item: ElevePhotoJobItem;
  photoKey?: string;
  eleveKey?: string;
}> {
  const bytes = await getObjectBytes(item.stagingKey);
  if (!bytes?.length) {
    return {
      item: {
        ...item,
        status: "failed",
        error: "Fichier introuvable sur le serveur.",
      },
    };
  }

  const parsed = parsePhotoFilename(item.fileName);
  if (!parsed) {
    return { item: { ...item, status: "unmatched", error: "Nom de fichier illisible." } };
  }

  const eleve = matchEleveForPhoto(eleves, parsed.nom, parsed.prenom);
  if (!eleve) {
    return { item: { ...item, status: "unmatched" } };
  }

  const relative = photoRelativePathForEleve(eleve);
  const ct = item.contentType.startsWith("image/") ? item.contentType : "image/jpeg";
  // putObject écrase la clé finale (photos de l’année précédente remplacées).
  const s3Key = await putObject(relative, bytes, ct);
  const lookup = elevePhotoLookupKey(eleve);
  index[lookup] = s3Key;
  index[`name:${identityKey(eleve.nom, eleve.prenom)}`] = s3Key;

  try {
    await deleteObject(item.stagingKey);
  } catch {
    /* best-effort cleanup */
  }

  return {
    item: { ...item, status: "matched" },
    photoKey: s3Key,
    eleveKey: identityKey(eleve.nom, eleve.prenom),
  };
}

/**
 * Traite un segment du job (budget temps). Relancer tant qu’il reste des pending.
 * Remplace les photos existantes (même clé S3 élève).
 */
export async function runElevePhotoJob(jobId: string): Promise<ElevePhotoJob | null> {
  const job = await readElevePhotoJob(jobId);
  if (!job) return null;
  if (job.status === "completed" || job.status === "failed") return job;
  if (job.status === "uploading") {
    job.error = "L’upload n’est pas terminé.";
    await writeElevePhotoJob(job);
    return job;
  }

  job.status = "processing";
  job.label = "Association en cours sur le serveur…";
  await writeElevePhotoJob(job);

  const eleves = await loadElevesRegistry();
  if (!eleves.length) {
    job.status = "failed";
    job.error =
      "Aucun élève pour associer les photos — importez d’abord le référentiel élèves.";
    job.label = "Échec — référentiel élèves vide.";
    await writeElevePhotoJob(job);
    return job;
  }

  const index = await loadElevePhotoIndex();
  const nextEleves = [...eleves];
  const started = Date.now();
  let touchedRegistry = false;

  for (let i = 0; i < job.items.length; i++) {
    if (Date.now() - started > RUN_BUDGET_MS) break;
    const current = job.items[i]!;
    if (current.status !== "pending") continue;

    try {
      const result = await processOneItem(current, nextEleves, index);
      job.items[i] = result.item;

      if (result.item.status === "matched" && result.photoKey && result.eleveKey) {
        job.matched += 1;
        const idx = nextEleves.findIndex(
          (e) => identityKey(e.nom, e.prenom) === result.eleveKey,
        );
        if (idx >= 0) {
          nextEleves[idx] = { ...nextEleves[idx]!, photoKey: result.photoKey };
          job.updated += 1;
          touchedRegistry = true;
        }
      } else if (result.item.status === "unmatched") {
        if (!job.unmatched.includes(result.item.fileName)) {
          job.unmatched.push(result.item.fileName);
        }
      } else if (result.item.status === "failed") {
        const msg = result.item.error || result.item.fileName;
        if (!job.errors.includes(msg)) job.errors.push(msg);
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Erreur traitement";
      job.items[i] = { ...current, status: "failed", error: msg };
      if (!job.errors.includes(msg)) job.errors.push(msg);
    }

    const done = job.items.filter((it) => it.status !== "pending").length;
    job.percent = Math.round((done / job.items.length) * 100);
    job.label = `${done}/${job.items.length} photo(s) traitées…`;
    await writeElevePhotoJob(job);
  }

  await putJson("eleves/photo-index.json", index);
  try {
    const { invalidateElevePhotoIndexCache } = await import("@/app/lib/eleve-photos");
    invalidateElevePhotoIndexCache();
  } catch {
    /* optional */
  }
  if (touchedRegistry) {
    try {
      await saveElevesRegistry(nextEleves);
    } catch (e) {
      console.warn("[eleve-photos-batch] saveElevesRegistry", e);
    }
  }

  const pending = job.items.some((it) => it.status === "pending");
  if (!pending) {
    const failedAll =
      job.items.length > 0 && job.items.every((it) => it.status === "failed");
    job.status = failedAll ? "failed" : "completed";
    job.percent = 100;
    job.label = failedAll
      ? "Échec du traitement."
      : `${job.updated} photo(s) enregistrée(s)${
          job.unmatched.length ? ` · ${job.unmatched.length} non reconnue(s)` : ""
        }.`;
  } else {
    job.status = "processing";
    job.label = `${job.items.filter((it) => it.status !== "pending").length}/${job.items.length} traitées — poursuite…`;
  }

  await writeElevePhotoJob(job);
  return job;
}

/**
 * Enchaîne le traitement sans dépendre du navigateur ni d’un cookie session.
 * Chaque segment s’exécute dans un `after()` (durée max de la route parente).
 */
export function scheduleElevePhotoContinuation(jobId: string, _origin?: string): void {
  after(async () => {
    try {
      const updated = await runElevePhotoJob(jobId);
      if (updated && updated.status === "processing") {
        scheduleElevePhotoContinuation(jobId);
      }
    } catch (e) {
      console.error("[eleve-photos-batch] continuation", jobId, e);
    }
  });
}
