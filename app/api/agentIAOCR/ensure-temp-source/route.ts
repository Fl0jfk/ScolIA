import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { readBatchJob, writeBatchJob } from "@/app/api/agentIAOCR/batch-job/batch-job";
import { extractPdfPagesBytes, getPdfPageCountFromS3 } from "@/app/lib/ocr-extract-pages";
import { ocrSegmentTempFileName, parseOcrSegmentLabel } from "@/app/lib/ocr-batch-merge";
import { graphDriveRootItemUrl } from "@/app/lib/graph-onedrive-path";
import { uploadBytesToOneDriveUnique } from "@/app/lib/ocr-graph-ops";

export const maxDuration = 30;

async function graphWebUrl(accessToken: string, itemPath: string): Promise<string | null> {
  const res = await fetch(graphDriveRootItemUrl(itemPath, "?$select=webUrl"), {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!res.ok) return null;
  const data = (await res.json()) as { webUrl?: string };
  return data.webUrl?.trim() || null;
}

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return NextResponse.json({ error: "Non autorisé" }, { status: 401 });

  const body = (await req.json()) as {
    accessToken?: string;
    jobId?: string;
    fileName?: string;
    sourcePath?: string;
  };
  const accessToken = String(body.accessToken || "").trim();
  const jobId = String(body.jobId || "").trim();
  const fileName = String(body.fileName || "").trim();
  const sourcePath = String(body.sourcePath || "").replace(/^\/+/, "");

  if (!accessToken) {
    return NextResponse.json({ error: "accessToken manquant" }, { status: 400 });
  }
  if (!fileName) {
    return NextResponse.json({ error: "fileName manquant" }, { status: 400 });
  }

  const parsed = parseOcrSegmentLabel(fileName);
  const job = jobId ? await readBatchJob(jobId) : null;
  if (jobId && (!job || job.userId !== userId)) {
    return NextResponse.json({ error: "Traitement introuvable." }, { status: 404 });
  }

  const item = job
    ? parsed
      ? job.items.find((it) => it.fileName === parsed.fileName)
      : job.items.find((it) => it.fileName === fileName)
    : undefined;
  const sourceIsOriginalScan = Boolean(item?.tempPath && sourcePath && sourcePath === item.tempPath);

  if (sourcePath && !sourceIsOriginalScan) {
    const existing = await graphWebUrl(accessToken, sourcePath);
    if (existing) {
      return NextResponse.json({ path: sourcePath, webUrl: existing });
    }
  }

  if (!job || !item?.s3Key) {
    if (sourcePath) {
      const existing = await graphWebUrl(accessToken, sourcePath);
      if (existing) return NextResponse.json({ path: sourcePath, webUrl: existing });
    }
    return NextResponse.json(
      { error: "Fichier introuvable dans Temp (le scan original a peut-être été rangé)." },
      { status: 404 },
    );
  }

  let pageStart = 1;
  let pageEnd = item.pdfPageCount || item.pageCount || 1;
  let tempFileName = item.fileName.replace(/\.pdf$/i, "") + ".pdf";
  if (parsed) {
    pageStart = parsed.pageStart;
    pageEnd = parsed.pageEnd;
    tempFileName = ocrSegmentTempFileName(parsed.fileName, parsed.pageStart, parsed.pageEnd);
  } else {
    try {
      pageEnd = await getPdfPageCountFromS3(item.s3Key);
    } catch {
      /* pageEnd déjà estimé */
    }
  }

  try {
    const pdfBytes = await extractPdfPagesBytes(item.s3Key, pageStart, pageEnd);
    const upload = await uploadBytesToOneDriveUnique(accessToken, "Temp", tempFileName, pdfBytes);
    if (!upload.ok) {
      return NextResponse.json(
        { error: `Impossible de déposer l'extrait dans Temp : ${upload.detail.slice(0, 200)}` },
        { status: upload.status || 502 },
      );
    }

    const webUrl = await graphWebUrl(accessToken, upload.path);
    const results = job.results.map((r) =>
      r.fileName === fileName ? { ...r, tempOneDrivePath: upload.path } : r,
    );
    await writeBatchJob({ ...job, results });

    return NextResponse.json({ path: upload.path, webUrl: webUrl || undefined });
  } catch (err) {
    console.error("[ensure-temp-source]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Impossible de recréer l'extrait." },
      { status: 500 },
    );
  }
}
