import { NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import { getAppSession } from "@/app/lib/intranet-session";
import { readElevePhotoJob } from "@/app/lib/eleve-photos-batch";

export const maxDuration = 30;

async function requirePhotosBulkAccess() {
  const admin = await requireAnyModule(["admin-settings"]);
  if (admin.ok) return { ok: true as const };
  return requireInternatManage();
}

/** Suivi optionnel du job (si l’utilisateur reste sur la page). */
export async function GET(req: Request) {
  const gate = await requirePhotosBulkAccess();
  if (!gate.ok) return gate.response;

  const session = await getAppSession();
  const userId = session?.user?.id?.trim();
  if (!userId) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const jobId = new URL(req.url).searchParams.get("jobId")?.trim();
  if (!jobId) {
    return NextResponse.json({ error: "jobId requis." }, { status: 400 });
  }

  const job = await readElevePhotoJob(jobId);
  if (!job) {
    return NextResponse.json({ error: "Job introuvable." }, { status: 404 });
  }
  if (job.userId !== userId) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
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
