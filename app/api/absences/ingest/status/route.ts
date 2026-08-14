import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { readIngestJob, canIngestFromUser } from "../ingest-job";

export const maxDuration = 10;

export async function GET(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canIngestFromUser(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

  const jobId = new URL(req.url).searchParams.get("jobId")?.trim();
  if (!jobId || !/^[a-zA-Z0-9_-]{8,128}$/.test(jobId)) {
    return NextResponse.json({ error: "jobId invalide." }, { status: 400 });
  }

  const job = await readIngestJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Import introuvable ou expiré." }, { status: 404 });
  }
  if (job.userId !== userId) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  return NextResponse.json({
    jobId,
    status: job.status,
    phase: job.phase ?? null,
    error: job.error ?? null,
    code: job.code ?? null,
    created: job.created ?? null,
    parsed: job.parsed ?? null,
    startedAt: job.startedAt,
    updatedAt: job.updatedAt,
  });
}
