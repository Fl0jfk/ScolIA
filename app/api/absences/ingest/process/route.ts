import { resolveSession, safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";

import { readIngestJob, canIngestFromUser } from "../ingest-job";
import { runAbsenceIngestJob, tryClaimIngestJob } from "@/app/lib/absence-ingest-process";

export const maxDuration = 300;

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canIngestFromUser(roles)) {
    return NextResponse.json({ error: "Action non autorisée." }, { status: 403 });
  }

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

  const job = await readIngestJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Import introuvable ou expiré." }, { status: 404 });
  }
  if (job.userId !== userId) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  if (job.status === "completed") {
    return NextResponse.json({ ok: true, status: "completed", created: job.created ?? null }, { status: 200 });
  }
  if (job.status === "failed") {
    return NextResponse.json(
      { ok: false, status: "failed", error: job.error ?? null, code: job.code ?? null },
      { status: 200 },
    );
  }

  const claimed = await tryClaimIngestJob(jobId);
  if (claimed) {
    try {
      await runAbsenceIngestJob(jobId, job.documentKey, job.sourceFileName);
    } catch (err) {
      console.error("[absences/ingest/process]", err);
    }
    const final = await readIngestJob(jobId);
    if (final?.status === "completed") {
      return NextResponse.json(
        { ok: true, status: "completed", created: final.created ?? null },
        { status: 200 },
      );
    }
    if (final?.status === "failed") {
      return NextResponse.json(
        { ok: false, status: "failed", error: final.error ?? null, code: final.code ?? null },
        { status: 200 },
      );
    }
  }

  return NextResponse.json(
    {
      ok: true,
      accepted: true,
      claimed,
      detail: claimed
        ? "Traitement OCR + IA en cours."
        : "Traitement déjà en cours ou file d'attente ; le suivi continue.",
    },
    { status: 202 },
  );
}
