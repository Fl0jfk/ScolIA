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
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevePhotoIndex, type ElevePhotoIndex } from "@/app/lib/eleve-photos";
import { sanitizeS3FileName } from "@/app/lib/s3-path";
import { valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyPhotoJob, valkeyKeyPhotoJobRoster } from "@/app/lib/valkey-keys";

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

/**
 * Traitement basse priorité (2–3× / an).
 * - Pas d’UPDATE Postgres pendant le matching (index S3 = source de vérité affichage).
 * - Roster élèves chargé 1× puis Valkey / mémoire.
 * - État job préféré Valkey ; S3 seulement en secours / fin de segment.
 */
const RUN_BUDGET_MS = 15_000;
const MAX_ITEMS_PER_SEGMENT = 5;
const PAUSE_BETWEEN_ITEMS_MS = 400;
const PAUSE_BETWEEN_SEGMENTS_MS = 8_000;
const ROSTER_TTL_MS = 60 * 60 * 1000;

/** Verrou process local (évite N workers concurrents sur la même instance). */
const runningJobs = new Set<string>();
/** Roster slim en mémoire par job. */
const rosterMem = new Map<string, { at: number; eleves: EleveConfig[] }>();

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

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
  const fromVk = await valkeyGetJson<ElevePhotoJob>(valkeyKeyPhotoJob(jobId));
  if (fromVk?.jobId) return fromVk;
  const hit = await getJson<ElevePhotoJob>(jobKey(jobId));
  return hit?.data ?? null;
}

async function writeElevePhotoJob(
  job: ElevePhotoJob,
  opts?: { persistS3?: boolean },
): Promise<void> {
  const next = { ...job, updatedAt: new Date().toISOString() };
  await valkeySetJson(valkeyKeyPhotoJob(job.jobId), next, VALKEY_TTL.photoJob);
  if (opts?.persistS3 !== false) {
    // S3 : checkpoint (reprise si Valkey absent / instance recyclée).
    await putJson(jobKey(job.jobId), next);
  }
}

async function loadMatchRoster(jobId: string): Promise<EleveConfig[]> {
  const mem = rosterMem.get(jobId);
  if (mem && Date.now() - mem.at < ROSTER_TTL_MS) return mem.eleves;

  const fromVk = await valkeyGetJson<EleveConfig[]>(valkeyKeyPhotoJobRoster(jobId));
  if (Array.isArray(fromVk) && fromVk.length) {
    rosterMem.set(jobId, { at: Date.now(), eleves: fromVk });
    return fromVk;
  }

  // UNE lecture Postgres pour tout le job — pas à chaque segment.
  const full = await loadElevesRegistry();
  const slim: EleveConfig[] = full.map((e) => ({
    id: e.id,
    ine: e.ine || "",
    nom: e.nom,
    prenom: e.prenom,
    folderName: e.folderName || `${e.nom} ${e.prenom}`,
  }));
  rosterMem.set(jobId, { at: Date.now(), eleves: slim });
  void valkeySetJson(valkeyKeyPhotoJobRoster(jobId), slim, VALKEY_TTL.photoJobRoster);
  return slim;
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
    await sleep(60);
  }

  job.status = "uploading";
  job.label = `${job.items.length} fichier(s) reçu(s) — continuez l’envoi ou lancez le traitement.`;
  job.percent = 0;
  // Pendant l’upload : Valkey + S3 moins souvent (gros JSON).
  await writeElevePhotoJob(job, { persistS3: true });
  return job;
}

export async function queueElevePhotoJob(jobId: string, userId: string): Promise<ElevePhotoJob> {
  const job = await readElevePhotoJob(jobId);
  if (!job) throw new Error("Job introuvable.");
  if (job.userId !== userId) throw new Error("Job photos réservé à un autre utilisateur.");
  if (!job.items.length) throw new Error("Aucun fichier à traiter.");
  if (job.status === "processing" || job.status === "completed") return job;

  job.status = "queued";
  job.label = `File d’attente — ${job.items.length} photo(s) à associer (basse priorité, sans charge BDD)…`;
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
 * Traite un segment du job. Aucune écriture Postgres : l’index S3 suffit à l’affichage.
 */
export async function runElevePhotoJob(jobId: string): Promise<ElevePhotoJob | null> {
  if (runningJobs.has(jobId)) {
    return readElevePhotoJob(jobId);
  }
  runningJobs.add(jobId);

  try {
    const job = await readElevePhotoJob(jobId);
    if (!job) return null;
    if (job.status === "completed" || job.status === "failed") return job;
    if (job.status === "uploading") {
      job.error = "L’upload n’est pas terminé.";
      await writeElevePhotoJob(job);
      return job;
    }

    job.status = "processing";
    job.label = "Association lente (S3 + cache) — BDD non sollicitée…";
    await writeElevePhotoJob(job, { persistS3: false });

    const eleves = await loadMatchRoster(jobId);
    if (!eleves.length) {
      job.status = "failed";
      job.error =
        "Aucun élève pour associer les photos — importez d’abord le référentiel élèves.";
      job.label = "Échec — référentiel élèves vide.";
      await writeElevePhotoJob(job);
      return job;
    }

    const index = await loadElevePhotoIndex();
    const started = Date.now();
    let processedThisSegment = 0;

    for (let i = 0; i < job.items.length; i++) {
      if (Date.now() - started > RUN_BUDGET_MS) break;
      if (processedThisSegment >= MAX_ITEMS_PER_SEGMENT) break;
      const current = job.items[i]!;
      if (current.status !== "pending") continue;

      try {
        const result = await processOneItem(current, eleves, index);
        job.items[i] = result.item;

        if (result.item.status === "matched" && result.photoKey) {
          job.matched += 1;
          job.updated += 1;
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

      processedThisSegment += 1;
      const done = job.items.filter((it) => it.status !== "pending").length;
      job.percent = Math.round((done / job.items.length) * 100);
      job.label = `${done}/${job.items.length} photo(s) — basse priorité (sans BDD)…`;

      // Progression : Valkey seulement (évite de réécrire un gros JSON S3 à chaque photo).
      if (processedThisSegment === 1 || processedThisSegment % MAX_ITEMS_PER_SEGMENT === 0) {
        await writeElevePhotoJob(job, { persistS3: false });
      }

      if (PAUSE_BETWEEN_ITEMS_MS > 0) {
        await sleep(PAUSE_BETWEEN_ITEMS_MS);
      }
    }

    if (processedThisSegment > 0) {
      await putJson("eleves/photo-index.json", index);
      try {
        const { invalidateElevePhotoIndexCache } = await import("@/app/lib/eleve-photos");
        invalidateElevePhotoIndexCache();
      } catch {
        /* optional */
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
      rosterMem.delete(jobId);
    } else {
      job.status = "processing";
      job.label = `${job.items.filter((it) => it.status !== "pending").length}/${job.items.length} traitées — pause ${PAUSE_BETWEEN_SEGMENTS_MS / 1000}s…`;
    }

    await writeElevePhotoJob(job, { persistS3: true });
    return job;
  } finally {
    runningJobs.delete(jobId);
  }
}

/**
 * Enchaîne le traitement sans dépendre du navigateur.
 * Un seul worker à la fois ; pause longue entre segments.
 */
export function scheduleElevePhotoContinuation(jobId: string, _origin?: string): void {
  after(async () => {
    try {
      if (PAUSE_BETWEEN_SEGMENTS_MS > 0) {
        await sleep(PAUSE_BETWEEN_SEGMENTS_MS);
      }
      if (runningJobs.has(jobId)) return;
      const updated = await runElevePhotoJob(jobId);
      if (updated && updated.status === "processing") {
        scheduleElevePhotoContinuation(jobId);
      }
    } catch (e) {
      console.error("[eleve-photos-batch] continuation", jobId, e);
    }
  });
}
