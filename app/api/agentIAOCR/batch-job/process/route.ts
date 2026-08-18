import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { readBatchJob } from "../batch-job";
import { runOcrBatchJob } from "@/app/lib/ocr-batch-process";
import { isOcrBatchJobFullyCovered } from "@/app/lib/ocr-batch-merge";
import { ocrTrace, summarizeBatchJob } from "@/app/lib/ocr-trace";
import { flushOcrJobTraces } from "@/app/lib/ocr-job-trace-store";

/**
 * Moteur d'avancement piloté par le client (page ouverte).
 * Exécute UN chunk du worker de façon SYNCHRONE pendant la requête (selfChain=false) :
 * c'est fiable sur Scaleway car cela ne dépend ni de after() ni d'un secret d'auto-relance.
 * Le verrou S3 garantit qu'on ne double-traite pas si une chaîne serveur tourne déjà en arrière-plan.
 * Le client rappelle cette route à chaque sondage pour enchaîner les chunks jusqu'à la fin.
 */
export const maxDuration = 60;

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  let body: { jobId?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const jobId = typeof body.jobId === "string" ? body.jobId.trim() : "";
  if (!jobId || !/^[a-zA-Z0-9_-]{8,128}$/.test(jobId)) {
    return NextResponse.json({ error: "jobId invalide." }, { status: 400 });
  }

  const job = await readBatchJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Traitement introuvable." }, { status: 404 });
  }
  if (job.userId !== userId) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  if (job.status === "completed" && isOcrBatchJobFullyCovered(job)) {
    return NextResponse.json({ ok: true, status: "completed" }, { status: 200 });
  }
  if (job.status === "failed" && isOcrBatchJobFullyCovered(job)) {
    return NextResponse.json(
      { ok: false, status: "failed", error: job.error ?? null },
      { status: 200 },
    );
  }
  if (job.status === "cancelled") {
    return NextResponse.json(
      { ok: false, status: job.status, error: job.error ?? null },
      { status: 200 },
    );
  }
  if (job.status === "needs_token") {
    return NextResponse.json(
      { ok: false, status: "needs_token", error: job.error ?? null },
      { status: 200 },
    );
  }

  ocrTrace(jobId, "api", "process", "process déclenché par client (exécution synchrone d'un chunk)", {
    userId,
    jobStatus: job.status,
    ...summarizeBatchJob(job),
  });

  try {
    // selfChain=false : un seul chunk, pas d'auto-relance serveur — le client rappellera /process.
    await runOcrBatchJob(jobId, { selfChain: false });
    await flushOcrJobTraces(jobId);
  } catch (err) {
    ocrTrace(jobId, "api", "process-error", "chunk worker synchrone en erreur", {
      error: err instanceof Error ? err.message : String(err),
    }, "error");
    // On ne propage pas : le statut reste la source de vérité côté client, qui relancera.
  }

  const fresh = (await readBatchJob(jobId)) ?? job;
  return NextResponse.json(
    { ok: true, status: fresh.status, detail: "Chunk de traitement OCR exécuté." },
    { status: 200 },
  );
}
