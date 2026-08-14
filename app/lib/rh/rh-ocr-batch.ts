import "server-only";

import { randomBytes } from "crypto";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { after } from "next/server";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { startTextractForS3Key, pollTextractOnce } from "@/app/lib/ocr-textract";
import { moveOneDriveFile } from "@/app/lib/ocr-graph-ops";
import { getRhDriveAccessToken } from "@/app/lib/rh/graph-rh-drive";
import { analyzeDocMatchPersonnelRh } from "@/app/lib/rh/ocr-analyze-personnel-rh";
import { appendRhDocumentToPersonnel } from "@/app/lib/rh/rh-deposit-self";
import { readRhPersonnelIndex } from "@/app/lib/rh/meta-storage";
import { rhTempPath } from "@/app/lib/rh/paths";
import { uploadFileToOneDriveFolder } from "@/app/lib/graph-onedrive-folders";

export type RhOcrJobItem = {
  id: string;
  fileName: string;
  s3Key: string;
  tempPath: string;
  status: "pending" | "processing" | "done" | "failed";
  textractJobId?: string;
  error?: string;
};

export type RhOcrJobResult = {
  fileName: string;
  success: boolean;
  error?: string;
  tempOneDrivePath?: string;
  matchedName?: string;
  destinationPath?: string;
};

export type RhOcrJob = {
  jobId: string;
  userId: string;
  status: "pending" | "processing" | "completed" | "failed";
  startedAt: string;
  updatedAt: string;
  items: RhOcrJobItem[];
  results: RhOcrJobResult[];
  completed: number;
  failed: number;
  label: string;
  percent: number;
  error?: string;
};

const JOB_PREFIX = "rhOCR/jobs/";
const RUN_BUDGET_MS = 50_000;

function newRhOcrJobId() {
  return `rhocr_${Date.now()}_${randomBytes(4).toString("hex")}`;
}

function jobKey(jobId: string) {
  return `${JOB_PREFIX}${jobId}.json`;
}

export async function readRhOcrJob(jobId: string): Promise<RhOcrJob | null> {
  const hit = await getJson<RhOcrJob>(jobKey(jobId));
  return hit?.data ?? null;
}

async function writeRhOcrJob(job: RhOcrJob): Promise<void> {
  await putJson(jobKey(job.jobId), { ...job, updatedAt: new Date().toISOString() });
}

export async function uploadRhBulkToTemp(input: {
  fileName: string;
  bytes: Uint8Array;
  contentType?: string;
}): Promise<{ s3Key: string; tempPath: string }> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) throw new Error(token.error);

  const safeName = input.fileName.replace(/[^a-zA-Z0-9._-]/g, "_") || "document.pdf";
  const s3Key = `rhOCR/uploads/${Date.now()}_${safeName}`;
  const tempPath = rhTempPath(token.basePath, safeName);

  const bucket = await getBucketName();
  const s3 = await getTenantDataS3Client();
  await s3.send(
    new PutObjectCommand({
      Bucket: bucket,
      Key: s3Key,
      Body: input.bytes,
      ContentType: input.contentType || "application/pdf",
    }),
  );

  await uploadFileToOneDriveFolder(
    token.accessToken,
    rhTempPath(token.basePath),
    safeName,
    input.bytes,
    input.contentType || "application/pdf",
  );

  return { s3Key, tempPath };
}

export async function createRhOcrJob(
  userId: string,
  items: Omit<RhOcrJobItem, "id" | "status">[],
): Promise<RhOcrJob> {
  const job: RhOcrJob = {
    jobId: newRhOcrJobId(),
    userId,
    status: "pending",
    startedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    items: items.map((it, i) => ({
      ...it,
      id: `item_${i + 1}`,
      status: "pending" as const,
    })),
    results: [],
    completed: 0,
    failed: 0,
    label: "Initialisation…",
    percent: 0,
  };
  await writeRhOcrJob(job);
  return job;
}

async function processOneItem(
  job: RhOcrJob,
  item: RhOcrJobItem,
  accessToken: string,
  basePath: string,
  indexEntries: Awaited<ReturnType<typeof readRhPersonnelIndex>>,
): Promise<RhOcrJobResult> {
  if (!indexEntries.ok) {
    return { fileName: item.fileName, success: false, error: indexEntries.error, tempOneDrivePath: item.tempPath };
  }

  let textractJobId = item.textractJobId;
  if (!textractJobId) {
    try {
      textractJobId = await startTextractForS3Key(item.s3Key);
      item.textractJobId = textractJobId;
    } catch (e) {
      return {
        fileName: item.fileName,
        success: false,
        error: e instanceof Error ? e.message : "Échec lancement Textract",
        tempOneDrivePath: item.tempPath,
      };
    }
  }

  const polled = await pollTextractOnce(textractJobId);
  if (polled.status === "IN_PROGRESS") {
    return {
      fileName: item.fileName,
      success: false,
      error: "OCR en cours — relance automatique",
      tempOneDrivePath: item.tempPath,
    };
  }
  if (polled.status === "FAILED") {
    return {
      fileName: item.fileName,
      success: false,
      error: "OCR Textract échoué",
      tempOneDrivePath: item.tempPath,
    };
  }

  const text = polled.result.text;
  const analyzed = await analyzeDocMatchPersonnelRh(
    text,
    basePath,
    indexEntries.index.entries,
    item.fileName,
  );

  if (!analyzed.oneDriveFilePath || !analyzed.matchedEntry) {
    return {
      fileName: item.fileName,
      success: false,
      error: "Collaborateur non identifié — fichier laissé dans Temp.",
      tempOneDrivePath: item.tempPath,
      matchedName: analyzed.match.candidates.map((c) => c.displayName).join(", ") || undefined,
    };
  }

  const destParts = analyzed.oneDriveFilePath.split("/");
  const destFileName = destParts.pop() || `${analyzed.fileName}.pdf`;
  const destFolder = destParts.join("/");
  const move = await moveOneDriveFile(accessToken, item.tempPath, destFolder, destFileName);

  if (!move.ok) {
    return {
      fileName: item.fileName,
      success: false,
      error: move.detail || "Déplacement OneDrive impossible",
      tempOneDrivePath: item.tempPath,
    };
  }

  const finalPath = `${destFolder}/${destFileName}`;
  await appendRhDocumentToPersonnel({
    folderName: analyzed.matchedEntry.folderName,
    fileName: destFileName,
    oneDrivePath: finalPath,
    category: analyzed.docCategory,
    uploadedBy: "RH OCR",
  });

  return {
    fileName: item.fileName,
    success: true,
    matchedName: analyzed.matchedEntry.displayName,
    destinationPath: finalPath,
  };
}

export async function runRhOcrJob(jobId: string): Promise<RhOcrJob | null> {
  const job = await readRhOcrJob(jobId);
  if (!job || job.status === "completed" || job.status === "failed") return job;

  const token = await getRhDriveAccessToken();
  if ("error" in token) {
    job.status = "failed";
    job.error = token.error;
    await writeRhOcrJob(job);
    return job;
  }

  const indexEntries = await readRhPersonnelIndex();
  job.status = "processing";
  const started = Date.now();
  let done = 0;
  let failed = 0;

  for (const item of job.items) {
    if (Date.now() - started > RUN_BUDGET_MS) break;
    if (item.status === "done" || item.status === "failed") {
      if (item.status === "done") done++;
      else failed++;
      continue;
    }

    item.status = "processing";
    job.label = `Analyse : ${item.fileName}`;
    await writeRhOcrJob(job);

    const result = await processOneItem(job, item, token.accessToken, token.basePath, indexEntries);

    if (result.success) {
      item.status = "done";
      done++;
    } else if (result.error?.includes("relance")) {
      item.status = "pending";
    } else {
      item.status = "failed";
      item.error = result.error;
      failed++;
    }

    const existing = job.results.findIndex((r) => r.fileName === result.fileName);
    if (existing >= 0) job.results[existing] = result;
    else job.results.push(result);

    job.completed = done;
    job.failed = failed;
    job.percent = Math.round(((done + failed) / job.items.length) * 100);
    await writeRhOcrJob(job);
  }

  const pending = job.items.some((i) => i.status === "pending" || i.status === "processing");
  if (!pending) {
    job.status = failed === job.items.length ? "failed" : "completed";
    job.label = failed ? `${done} classé(s), ${failed} échec(s)` : `${done} document(s) classé(s)`;
    job.percent = 100;
  } else {
    job.label = `${done + failed}/${job.items.length} traités…`;
  }

  await writeRhOcrJob(job);
  return job;
}

export function scheduleRhOcrContinuation(jobId: string, origin: string) {
  after(async () => {
    try {
      await fetch(`${origin}/api/rh/deposit/bulk/process`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jobId }),
      });
    } catch {
      /* ignore */
    }
  });
}
