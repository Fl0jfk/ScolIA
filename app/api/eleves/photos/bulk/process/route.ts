import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import {
  readElevePhotoJob,
  runElevePhotoJob,
  scheduleElevePhotoContinuation,
} from "@/app/lib/eleve-photos-batch";

export const maxDuration = 60;

async function requirePhotosBulkAccess() {
  const admin = await requireAnyModule(["admin-settings"]);
  if (admin.ok) return { ok: true as const };
  return requireInternatManage();
}

/** Continuation worker — auth utilisateur ou secret worker (chaîne serveur). */
export async function POST(req: Request) {
  const workerSecret =
    process.env.PHOTOS_WORKER_SECRET?.trim() || process.env.OCR_WORKER_SECRET?.trim();
  const provided = req.headers.get("x-photos-worker-secret") || req.headers.get("x-ocr-worker-secret");
  const isWorker = Boolean(workerSecret && provided && provided === workerSecret);

  if (!isWorker) {
    const gate = await requirePhotosBulkAccess();
    if (!gate.ok) return gate.response;
  }

  const body = (await req.json().catch(() => ({}))) as { jobId?: string };
  const jobId = String(body.jobId || "").trim();
  if (!jobId) {
    return NextResponse.json({ error: "jobId requis." }, { status: 400 });
  }

  const job = await runElevePhotoJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Job introuvable." }, { status: 404 });
  }

  if (job.status === "processing") {
    scheduleElevePhotoContinuation(jobId);
  }

  return NextResponse.json({
    jobId: job.jobId,
    status: job.status,
    percent: job.percent,
    label: job.label,
    matched: job.matched,
    updated: job.updated,
    unmatched: job.unmatched.length,
  });
}

export async function GET(req: Request) {
  const gate = await requirePhotosBulkAccess();
  if (!gate.ok) return gate.response;

  const jobId = new URL(req.url).searchParams.get("jobId")?.trim();
  if (!jobId) {
    return NextResponse.json({ error: "jobId requis." }, { status: 400 });
  }

  const job = await readElevePhotoJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Job introuvable." }, { status: 404 });
  }

  return NextResponse.json({
    jobId: job.jobId,
    status: job.status,
    percent: job.percent,
    label: job.label,
    total: job.items.length,
    matched: job.matched,
    updated: job.updated,
    unmatched: job.unmatched,
    errors: job.errors,
    error: job.error,
  });
}
