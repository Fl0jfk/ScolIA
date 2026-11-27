import { getTenantAppUrl } from "@/app/lib/tenant-context";
import { after, NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  readRhOcrJob,
  runRhOcrJob,
  scheduleRhOcrContinuation,
} from "@/app/lib/rh/rh-ocr-batch";

export const maxDuration = 60;

/** Chunk worker OCR RH (auto-relance si job incomplet). */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  let jobId = "";
  try {
    const body = await req.json();
    jobId = String(body?.jobId || "").trim();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  if (!jobId) return NextResponse.json({ error: "jobId requis." }, { status: 400 });

  const job = await runRhOcrJob(jobId);
  if (!job) return NextResponse.json({ error: "Job introuvable." }, { status: 404 });

  if (job.status === "processing" || job.items.some((i) => i.status === "pending")) {
    scheduleRhOcrContinuation(jobId, await getTenantAppUrl());
  }

  return NextResponse.json({ job });
}

/** Statut job OCR RH. */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const jobId = new URL(req.url).searchParams.get("jobId")?.trim();
  if (!jobId) return NextResponse.json({ error: "jobId requis." }, { status: 400 });

  const job = await readRhOcrJob(jobId);
  if (!job) return NextResponse.json({ error: "Job introuvable." }, { status: 404 });
  return NextResponse.json({ job });
}
