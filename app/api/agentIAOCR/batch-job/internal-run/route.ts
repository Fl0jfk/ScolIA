import { after, NextResponse } from "next/server";
import { runOcrBatchJob } from "@/app/lib/ocr-batch-process";
import { ocrTrace } from "@/app/lib/ocr-trace";
import { flushOcrJobTraces } from "@/app/lib/ocr-job-trace-store";

/**
 * Relance interne du worker OCR (auto-chaînage serveur, onglet fermé).
 * Appelé uniquement par le worker lui-même, protégé par un secret partagé —
 * pas de session utilisateur requise (route publique côté middleware).
 */
export const maxDuration = 60;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function POST(req: Request) {
  const secret = process.env.OCR_WORKER_SECRET?.trim();
  if (!secret) {
    return NextResponse.json({ error: "Auto-relance désactivée." }, { status: 503 });
  }
  if (req.headers.get("x-ocr-worker-secret") !== secret) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  let body: { jobId?: string; delayMs?: number };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const jobId = typeof body.jobId === "string" ? body.jobId.trim() : "";
  if (!jobId || !/^[a-zA-Z0-9_-]{8,128}$/.test(jobId)) {
    return NextResponse.json({ error: "jobId invalide." }, { status: 400 });
  }
  const delayMs = Math.max(0, Math.min(8_000, Number(body.delayMs) || 0));

  ocrTrace(jobId, "api", "internal-run", "requête internal-run acceptée", { delayMs });

  const execute = async () => {
    if (delayMs > 0) await sleep(delayMs);
    ocrTrace(jobId, "api", "internal-run-exec", "exécution worker internal-run");
    await runOcrBatchJob(jobId);
    await flushOcrJobTraces(jobId);
  };

  // delay=0 : exécution synchrone — after() seul ne démarre souvent pas le worker en serverless.
  if (delayMs === 0) {
    try {
      await execute();
      return NextResponse.json({ ok: true, completed: true }, { status: 200 });
    } catch (err) {
      ocrTrace(jobId, "api", "internal-run-error", "internal-run synchrone en erreur", {
        error: err instanceof Error ? err.message : String(err),
      }, "error");
      return NextResponse.json({ error: "Worker en erreur." }, { status: 500 });
    }
  }

  after(async () => {
    try {
      await execute();
    } catch (err) {
      ocrTrace(jobId, "api", "internal-run-error", "internal-run after() en erreur", {
        error: err instanceof Error ? err.message : String(err),
      }, "error");
    }
  });

  return NextResponse.json({ ok: true, accepted: true, delayMs }, { status: 202 });
}
