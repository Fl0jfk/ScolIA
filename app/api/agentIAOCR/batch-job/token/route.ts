import { after, NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { readBatchJob } from "../batch-job";
import {
  refreshBatchJobAccessToken,
  resumeBatchJobWithToken,
  runOcrBatchJob,
} from "@/app/lib/ocr-batch-process";

export const maxDuration = 60;

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  let body: { jobId?: string; accessToken?: string; refreshOnly?: boolean };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const jobId = typeof body.jobId === "string" ? body.jobId.trim() : "";
  const accessToken = typeof body.accessToken === "string" ? body.accessToken.trim() : "";
  const refreshOnly = body.refreshOnly === true;
  if (!jobId || !accessToken) {
    return NextResponse.json({ error: "jobId et accessToken requis." }, { status: 400 });
  }

  const job = await readBatchJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Traitement introuvable." }, { status: 404 });
  }
  if (job.userId !== userId) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  if (refreshOnly) {
    if (job.status === "completed" || job.status === "failed" || job.status === "cancelled") {
      return NextResponse.json({ ok: true, status: job.status, skipped: true });
    }
    await refreshBatchJobAccessToken(jobId, accessToken);
    return NextResponse.json({ ok: true, status: job.status, refreshed: true });
  }

  if (job.status !== "needs_token") {
    return NextResponse.json({ error: "Ce traitement n'attend pas de reconnexion." }, { status: 409 });
  }

  await resumeBatchJobWithToken(jobId, accessToken);
  after(() => runOcrBatchJob(jobId).catch((err) => console.error("[ocr-batch/token] after():", err)));

  return NextResponse.json({ ok: true, status: "processing" });
}
