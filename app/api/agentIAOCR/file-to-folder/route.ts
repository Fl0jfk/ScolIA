import { NextResponse } from "next/server";
import { resolveSession } from "@/app/lib/intranet-session";
import { readBatchJob, writeBatchJob, type OcrBatchResult } from "@/app/api/agentIAOCR/batch-job/batch-job";
import { extractPdfPagesBytes, getPdfPageCountFromS3 } from "@/app/lib/ocr-extract-pages";
import {
  ocrSegmentResultLabel,
  ocrSegmentTempFileName,
  parseOcrSegmentLabel,
} from "@/app/lib/ocr-batch-merge";
import { deleteOneDrivePath, uploadBytesToOneDriveUnique } from "@/app/lib/ocr-graph-ops";
import { ensureFolderPath } from "@/app/lib/graph-onedrive-folders";

export const maxDuration = 60;

function sanitizeOneDriveFileName(raw: string): string {
  const base = String(raw || "")
    .replace(/\.pdf$/i, "")
    .replace(/[<>:"/\\|?*[\]]+/g, "_")
    .replace(/\s+/g, " ")
    .trim();
  return `${base || "Document"}.pdf`;
}

function buildManualFileName(opts: {
  nom?: string;
  prenom?: string;
  suggested?: string;
  pageStart: number;
  pageEnd: number;
}): string {
  const fromPerson = [opts.nom, opts.prenom].filter(Boolean).join("_");
  const fromSuggested = String(opts.suggested || "")
    .replace(/\.pdf$/i, "")
    .replace(/\s*\[p\.\d+-\d+\]\s*/i, "")
    .trim();
  const base = fromPerson || fromSuggested || "Document";
  const withPages =
    opts.pageStart === opts.pageEnd
      ? `${base}_p${opts.pageStart}`
      : `${base}_p${opts.pageStart}-${opts.pageEnd}`;
  return sanitizeOneDriveFileName(withPages);
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
    targetFolderPath?: string;
    newFileName?: string;
    pageStart?: number;
    pageEnd?: number;
    candidate?: { nom?: string; prenom?: string; folderName?: string };
  };

  const accessToken = String(body.accessToken || "").trim();
  const jobId = String(body.jobId || "").trim();
  const fileName = String(body.fileName || "").trim();
  const sourcePath = String(body.sourcePath || "").replace(/^\/+/, "");
  const targetFolderPath = String(body.targetFolderPath || "").replace(/^\/+|\/+$/g, "");

  if (!accessToken) {
    return NextResponse.json({ error: "accessToken manquant" }, { status: 400 });
  }
  if (!fileName) {
    return NextResponse.json({ error: "fileName manquant" }, { status: 400 });
  }
  if (!targetFolderPath) {
    return NextResponse.json({ error: "Dossier cible manquant" }, { status: 400 });
  }

  const job = jobId ? await readBatchJob(jobId) : null;
  if (jobId && (!job || job.userId !== userId)) {
    return NextResponse.json({ error: "Traitement introuvable." }, { status: 404 });
  }

  const parsed = parseOcrSegmentLabel(fileName);
  const item = job
    ? parsed
      ? job.items.find((it) => it.fileName === parsed.fileName)
      : job.items.find((it) => it.fileName === fileName)
    : undefined;

  if (!item?.s3Key) {
    return NextResponse.json(
      {
        error:
          "Impossible de retrouver le PDF source (S3). Rouvrez la session OCR ou rangez le fichier à la main depuis Temp.",
      },
      { status: 404 },
    );
  }

  let docStart = 1;
  let docEnd = item.pdfPageCount || item.pageCount || 1;
  if (parsed) {
    docStart = parsed.pageStart;
    docEnd = parsed.pageEnd;
  } else {
    try {
      docEnd = await getPdfPageCountFromS3(item.s3Key);
    } catch {
      /* keep estimate */
    }
  }

  const pageStart = Math.max(
    docStart,
    Math.min(docEnd, Number(body.pageStart) || docStart),
  );
  const pageEnd = Math.max(
    pageStart,
    Math.min(docEnd, Number(body.pageEnd) || docEnd),
  );

  try {
    await ensureFolderPath(accessToken, targetFolderPath);
  } catch (ensureErr) {
    return NextResponse.json(
      {
        error: "Impossible de créer le dossier cible OneDrive",
        details: ensureErr instanceof Error ? ensureErr.message : String(ensureErr),
      },
      { status: 500 },
    );
  }

  const pdfBytes = await extractPdfPagesBytes(item.s3Key, pageStart, pageEnd);
  const finalName = body.newFileName?.trim()
    ? sanitizeOneDriveFileName(body.newFileName)
    : buildManualFileName({
        nom: body.candidate?.nom,
        prenom: body.candidate?.prenom,
        suggested: (job?.results.find((r) => r.fileName === fileName)?.result as { fileName?: string } | undefined)
          ?.fileName,
        pageStart,
        pageEnd,
      });

  const upload = await uploadBytesToOneDriveUnique(
    accessToken,
    targetFolderPath,
    finalName,
    pdfBytes,
  );
  if (!upload.ok) {
    return NextResponse.json(
      { error: `Dépôt OneDrive impossible : ${upload.detail.slice(0, 220)}` },
      { status: upload.status || 502 },
    );
  }

  const existingResult = job?.results.find((r) => r.fileName === fileName);
  const isPartial = pageStart > docStart || pageEnd < docEnd;
  const filedLabel = isPartial
    ? ocrSegmentResultLabel(item.fileName, pageStart, pageEnd)
    : fileName;

  const successPayload = {
    ...(existingResult?.result && typeof existingResult.result === "object" ? existingResult.result : {}),
    fileName: upload.fileName.replace(/\.pdf$/i, ""),
    oneDriveItemPath: upload.path,
    oneDriveFinalFileName: upload.fileName,
    matchedEleve: body.candidate
      ? {
          nom: body.candidate.nom,
          prenom: body.candidate.prenom,
          folderName: body.candidate.folderName,
        }
      : undefined,
    matchDebug: {
      ...((existingResult?.result as { matchDebug?: Record<string, unknown> } | undefined)?.matchDebug || {}),
      matchedBy: "manual",
      decision: "auto",
      filedPages: { pageStart, pageEnd },
    },
  };

  const successResult: OcrBatchResult = {
    success: true,
    fileName: filedLabel,
    result: successPayload,
  };

  /** Pages restantes du même document → nouveaux dossiers « à traiter ». */
  const remainders: OcrBatchResult[] = [];
  const leftPages: Array<[number, number]> = [];
  if (pageStart > docStart) leftPages.push([docStart, pageStart - 1]);
  if (pageEnd < docEnd) leftPages.push([pageEnd + 1, docEnd]);

  if (leftPages.length > 0 && existingResult && !existingResult.success) {
    for (const [remStart, remEnd] of leftPages) {
      try {
        const remBytes = await extractPdfPagesBytes(item.s3Key, remStart, remEnd);
        const remName = ocrSegmentTempFileName(item.fileName, remStart, remEnd);
        const remUpload = await uploadBytesToOneDriveUnique(accessToken, "Temp", remName, remBytes);
        if (remUpload.ok) {
          remainders.push({
            success: false,
            fileName: ocrSegmentResultLabel(item.fileName, remStart, remEnd),
            error:
              "Pages restantes après rangement manuel — choisissez la personne ou les pages concernées.",
            tempOneDrivePath: remUpload.path,
            // Garde les suggestions (matchCandidates) pour le 2e rangement.
            result: existingResult.result,
          });
        }
      } catch (remErr) {
        console.warn("[file-to-folder] remainder extract failed", remErr);
      }
    }
  }

  if (job) {
    // writeBatchJob fusionne avec l'existant : sans écraser le libellé d'origine
    // (ex. SCAN [p.31-36]), l'échec revient et le document « à traiter » réapparaît.
    const dropNames = new Set<string>([fileName, successResult.fileName]);
    for (const rem of remainders) dropNames.add(rem.fileName);

    let results = job.results.filter((r) => !dropNames.has(r.fileName));
    results.push(successResult, ...remainders);

    // Si on a découpé, le libellé d'origine doit passer en succès (sinon merge le remet en échec).
    // Uniquement si toutes les pages restantes ont bien été déposées dans Temp.
    if (isPartial && filedLabel !== fileName && remainders.length === leftPages.length) {
      results.push({
        success: true,
        fileName,
        result: {
          ...successPayload,
          matchDebug: {
            ...(successPayload.matchDebug || {}),
            matchedBy: "manual",
            decision: "auto",
            splitInto: [filedLabel, ...remainders.map((r) => r.fileName)],
          },
        },
      });
    } else if (isPartial && leftPages.length > 0 && remainders.length !== leftPages.length) {
      return NextResponse.json(
        {
          error:
            "Le fichier a peut-être été déposé, mais les pages restantes n'ont pas pu être préparées dans Temp. Réessayez.",
        },
        { status: 502 },
      );
    }

    await writeBatchJob({ ...job, results });
  }

  // Nettoyer l'extrait Temp dédié s'il n'est plus le scan classe partagé.
  const remPaths = new Set(remainders.map((r) => r.tempOneDrivePath).filter(Boolean));
  if (sourcePath && sourcePath !== item.tempPath && !remPaths.has(sourcePath)) {
    void deleteOneDrivePath(accessToken, sourcePath);
  }

  return NextResponse.json({
    success: true,
    finalFileName: upload.fileName,
    oneDriveItemPath: upload.path,
    filedFileName: successResult.fileName,
    filedPages: { pageStart, pageEnd },
    remainders: remainders.map((r) => ({
      fileName: r.fileName,
      tempOneDrivePath: r.tempOneDrivePath,
      error: r.error,
      result: r.result,
    })),
    remainder: remainders[0]
      ? {
          fileName: remainders[0].fileName,
          tempOneDrivePath: remainders[0].tempOneDrivePath,
          error: remainders[0].error,
          result: remainders[0].result,
        }
      : null,
  });
}
