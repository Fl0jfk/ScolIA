import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { listRecentBatchJobSummariesForUser } from "../batch-job";

export const maxDuration = 15;

export async function GET() {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });

  // Résumés seulement (revalidés par ETag S3) : plus de téléchargement complet
  // des lots à chaque passage du suivi.
  const jobs = await listRecentBatchJobSummariesForUser(userId, 8);
  const active = jobs.filter(
    (j) => j.status === "pending" || j.status === "processing" || j.status === "needs_token",
  );

  return NextResponse.json({
    jobs: jobs.map((j) => ({
      jobId: j.jobId,
      status: j.status,
      label: j.label,
      percent: j.percent,
      completed: j.completed,
      failed: j.failed,
      totalItems: j.totalItems,
      currentItemIndex: j.currentItemIndex,
      startedAt: j.startedAt,
      updatedAt: j.updatedAt,
      error: j.error,
    })),
    activeCount: active.length,
  });
}
