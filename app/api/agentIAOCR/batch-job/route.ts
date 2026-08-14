import { after, NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  newBatchJobId,
  registerBatchJobForUser,
  writeBatchJob,
  type OcrBatchJob,
  type OcrBatchJobItem,
} from "./batch-job";
import { runOcrBatchJob, resolveWorkerOrigin, kickOcrBatchWorker } from "@/app/lib/ocr-batch-process";
import { ocrTrace } from "@/app/lib/ocr-trace";
import { flushOcrJobTraces } from "@/app/lib/ocr-job-trace-store";

export const maxDuration = 60;

/** Origine HTTP réelle (derrière le reverse-proxy) pour l'auto-relance serveur. */
function resolveRequestOrigin(req: Request): string | undefined {
  const host = req.headers.get("x-forwarded-host") || req.headers.get("host");
  if (host) {
    const proto =
      req.headers.get("x-forwarded-proto") || (host.startsWith("localhost") ? "http" : "https");
    return `${proto}://${host}`.replace(/\/+$/, "");
  }
  return resolveWorkerOrigin(null);
}

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  let body: {
    accessToken?: string;
    items?: Array<{
      fileName?: string;
      mode?: string;
      s3Key?: string;
      tempPath?: string;
    }>;
  };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const accessToken = typeof body.accessToken === "string" ? body.accessToken.trim() : "";
  if (!accessToken) {
    return NextResponse.json({ error: "accessToken requis" }, { status: 400 });
  }

  const rawItems = Array.isArray(body.items) ? body.items : [];
  const items: OcrBatchJobItem[] = rawItems
    .filter((it) => it?.s3Key && it?.fileName && it?.tempPath)
    .map((it, idx) => ({
      id: `item_${idx + 1}`,
      fileName: String(it.fileName),
      mode: it.mode === "class" ? "class" : "standard",
      s3Key: String(it.s3Key),
      tempPath: String(it.tempPath),
      status: "pending" as const,
    }));

  if (items.length === 0) {
    return NextResponse.json({ error: "Aucun fichier valide dans la file." }, { status: 400 });
  }

  const jobId = newBatchJobId();
  const now = new Date().toISOString();
  const job: OcrBatchJob = {
    jobId,
    userId,
    status: "pending",
    startedAt: now,
    updatedAt: now,
    accessToken,
    originUrl: resolveRequestOrigin(req),
    items,
    currentItemIndex: 0,
    results: [],
    label: `File d'attente — ${items.length} PDF`,
    percent: 0,
    completed: 0,
    failed: 0,
  };

  await writeBatchJob(job);
  await registerBatchJobForUser(userId, jobId);

  const origin = job.originUrl;
  const serverSelfRelays = Boolean(process.env.OCR_WORKER_SECRET?.trim() && origin);

  ocrTrace(jobId, "api", "create", "lot créé", {
    totalItems: items.length,
    modes: items.map((i) => i.mode),
    originUrl: origin ?? null,
    serverSelfRelays,
    items: items.map((i) => ({ fileName: i.fileName, mode: i.mode, s3Key: i.s3Key, tempPath: i.tempPath })),
  });

  // Double démarrage : after() local + chaîne HTTP (after() seul est peu fiable en serverless).
  after(async () => {
    try {
      ocrTrace(jobId, "api", "create-after", "worker via after() à la création");
      await runOcrBatchJob(jobId);
      await flushOcrJobTraces(jobId);
    } catch (err) {
      ocrTrace(jobId, "api", "create-after-error", "after() création en erreur", {
        error: err instanceof Error ? err.message : String(err),
      }, "error");
    }
  });

  // Kick non bloquant : on renvoie le jobId immédiatement. Le worker démarre via after()
  // (autonomie onglet fermé si secret configuré) ET via le pilotage client (/process synchrone).
  void kickOcrBatchWorker(jobId, origin).catch((err) =>
    ocrTrace(jobId, "api", "kick-fail", "échec kick initial", {
      error: err instanceof Error ? err.message : String(err),
    }, "error"),
  );
  await flushOcrJobTraces(jobId);

  return NextResponse.json(
    { ok: true, jobId, total: items.length, serverSelfRelays },
    { status: 201 },
  );
}
