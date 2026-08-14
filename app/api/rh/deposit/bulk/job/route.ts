import { after, NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canManagePersonnel } from "@/app/lib/personnel-types";
import {
  createRhOcrJob,
  runRhOcrJob,
  scheduleRhOcrContinuation,
} from "@/app/lib/rh/rh-ocr-batch";

export const maxDuration = 60;


function requestOrigin(req: Request): string {
  const host = req.headers.get("x-forwarded-host") || req.headers.get("host");
  const proto = req.headers.get("x-forwarded-proto") || "https";
  if (host) return `${proto}://${host}`.replace(/\/+$/, "");
  return new URL(req.url).origin;
}

/** Crée un job OCR RH et lance le traitement. */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  if (!user || !canManagePersonnel(rolesFromUserLike(user))) {
    return NextResponse.json({ error: "Réservé à la RH." }, { status: 403 });
  }

  try {
    const body = await req.json();
    const items = Array.isArray(body?.items) ? body.items : [];
    const valid = items.filter(
      (it: { s3Key?: string; tempPath?: string; fileName?: string }) =>
        it?.s3Key && it?.tempPath && it?.fileName,
    );
    if (valid.length === 0) {
      return NextResponse.json({ error: "Aucun fichier valide." }, { status: 400 });
    }

    const job = await createRhOcrJob(
      user.id,
      valid.map((it: { s3Key: string; tempPath: string; fileName: string }) => ({
        s3Key: it.s3Key,
        tempPath: it.tempPath,
        fileName: it.fileName,
      })),
    );

    const origin = requestOrigin(req);
    after(async () => {
      const updated = await runRhOcrJob(job.jobId);
      if (
        updated &&
        (updated.status === "processing" ||
          updated.items.some((i) => i.status === "pending" || i.status === "processing"))
      ) {
        scheduleRhOcrContinuation(job.jobId, origin);
      }
    });

    return NextResponse.json({ jobId: job.jobId, job });
  } catch (e) {
    return NextResponse.json(
      { error: e instanceof Error ? e.message : "Job impossible." },
      { status: 500 },
    );
  }
}
