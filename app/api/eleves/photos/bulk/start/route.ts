import { after, NextResponse } from "next/server";
import { requireInternatManage } from "@/app/api/internat/_auth";
import { requireAnyModule } from "@/app/lib/intranet-auth";
import { getAppSession } from "@/app/lib/intranet-session";
import {
  queueElevePhotoJob,
  runElevePhotoJob,
  scheduleElevePhotoContinuation,
} from "@/app/lib/eleve-photos-batch";

export const maxDuration = 60;

async function requirePhotosBulkAccess() {
  const admin = await requireAnyModule(["admin-settings"]);
  if (admin.ok) return { ok: true as const };
  return requireInternatManage();
}

/**
 * Clôture l’upload et lance le traitement serveur (batch).
 * La réponse part immédiatement : on peut quitter la page.
 */
export async function POST(req: Request) {
  const gate = await requirePhotosBulkAccess();
  if (!gate.ok) return gate.response;

  const session = await getAppSession();
  const userId = session?.user?.id?.trim();
  if (!userId) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const body = (await req.json().catch(() => ({}))) as { jobId?: string };
  const jobId = String(body.jobId || "").trim();
  if (!jobId) {
    return NextResponse.json({ error: "jobId requis." }, { status: 400 });
  }

  try {
    const job = await queueElevePhotoJob(jobId, userId);

    after(async () => {
      const updated = await runElevePhotoJob(job.jobId);
      if (updated && updated.status === "processing") {
        scheduleElevePhotoContinuation(job.jobId);
      }
    });

    return NextResponse.json({
      jobId: job.jobId,
      status: job.status,
      total: job.items.length,
      label: job.label,
      message:
        "Upload terminé. Le serveur associe les photos en arrière-plan (écrasement des anciennes). Vous pouvez quitter cette page.",
    });
  } catch (e: unknown) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Démarrage impossible." },
      { status: 400 },
    );
  }
}
