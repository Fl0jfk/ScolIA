import { GetObjectCommand } from "@aws-sdk/client-s3";
import JSZip from "jszip";
import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import {
  canSignAwardAsProf,
  canViewProgram,
} from "@/app/lib/certificates-auth";
import { issueAwardPdf, signAwardAsProf } from "@/app/lib/certificates-workflow";
import { listAwardsForProgram, loadProgram, saveAward } from "@/app/lib/certificates-storage";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { getTenant } from "@/app/lib/tenant-context";
import { isPlatformHostname } from "@/app/lib/platform-hostname";
import { getTenantSecrets, normalizeHostname } from "@/app/lib/tenant-registry";
import { tenantCanonicalOrigin, tenantOrigin } from "@/app/lib/tenant-auth-urls";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getMicrosoftAccessTokenFromRefresh } from "@/app/lib/graph-microsoft-delegated";
import { resolveOneDriveProfileForClerkUserServer } from "@/app/lib/onedrive-user-profiles.server";
import { ensureFolderPath } from "@/app/lib/graph-onedrive-folders";
import { uploadBytesToOneDriveUnique } from "@/app/lib/ocr-graph-ops";
import {
  newBatchJobId,
  registerBatchJobForUser,
  writeBatchJob,
  type OcrBatchJob,
  type OcrBatchJobItem,
} from "@/app/api/agentIAOCR/batch-job/batch-job";
import { kickOcrBatchWorker } from "@/app/lib/ocr-batch-process";

type Params = { params: Promise<{ id: string }> };

function safeFileNamePart(value: string): string {
  return value
    .replace(/[<>:"/\\|?*\u0000-\u001F]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function requestOrigin(req: Request): string {
  const reqUrl = new URL(req.url);
  const host = req.headers.get("x-forwarded-host") || req.headers.get("host") || reqUrl.host;
  const proto = req.headers.get("x-forwarded-proto") || (host.includes("localhost") ? "http" : "https");
  return `${proto}://${host}`.replace(/\/+$/, "");
}

function canManageOcrFromRoles(roles: string[]): boolean {
  return hasGlobalAdminRole(roles) || hasRole(roles, "administratif");
}

export async function GET(req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const { id } = await params;
  const program = await loadProgram(id);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canViewProgram(program, gate.ctx.userId)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const awards = await listAwardsForProgram(id);
  const issued = awards.filter((a) => a.status === "issued" && a.pdfS3Key);
  if (!issued.length) {
    return NextResponse.json({ error: "Aucun certificat PDF disponible pour ce parcours." }, { status: 400 });
  }

  const s3 = await getTenantDataS3Client();
  const bucket = await getBucketName();
  const zip = new JSZip();

  for (const award of issued) {
    const key = award.pdfS3Key;
    if (!key) continue;
    try {
      const obj = await s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
      const bytes = await obj.Body?.transformToByteArray();
      if (!bytes?.length) continue;
      const student = safeFileNamePart(`${award.student.nom} ${award.student.prenom}`);
      const classe = safeFileNamePart(award.student.classe || "Sans classe");
      zip.file(`${classe}/${student}.pdf`, bytes);
    } catch {
      // On ignore les PDFs introuvables et on compresse le reste.
    }
  }

  const buffer = await zip.generateAsync({ type: "nodebuffer" });
  if (!buffer.length) {
    return NextResponse.json({ error: "Impossible de créer l'archive (PDF indisponibles)." }, { status: 400 });
  }

  const filename = safeFileNamePart(program.title || "parcours");
  return new NextResponse(new Uint8Array(buffer), {
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="certificats-${filename}.zip"`,
      "Cache-Control": "no-store, no-cache, must-revalidate, proxy-revalidate",
      Pragma: "no-cache",
      Expires: "0",
    },
  });
}

export async function POST(req: Request, { params }: Params) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const { id } = await params;
  const program = await loadProgram(id);
  if (!program) return NextResponse.json({ error: "Parcours introuvable." }, { status: 404 });
  if (!canViewProgram(program, gate.ctx.userId)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const body = await req.json().catch(() => ({}));
  const action = String(body?.action || "").trim();
  const awards = await listAwardsForProgram(id);

  if (action === "sign-prof-all") {
    const signable = awards.filter((a) => canSignAwardAsProf(a, gate.ctx.userId));
    let signed = 0;
    const errors: string[] = [];

    for (const award of signable) {
      const result = await signAwardAsProf(award, gate.ctx.userId);
      if ("error" in result) {
        errors.push(`${award.student.prenom} ${award.student.nom}: ${result.error}`);
        continue;
      }
      await saveAward(result);
      signed += 1;
    }

    return NextResponse.json({
      ok: true,
      action,
      totalInProgram: awards.length,
      eligible: signable.length,
      done: signed,
      errors,
    });
  }

  if (action === "generate-pdf-all") {
    const tenant = await getTenant();
    const canonicalOrigin = tenantCanonicalOrigin(tenant);
    const canonicalHost = normalizeHostname(new URL(canonicalOrigin).host);
    const requestHost = normalizeHostname(new URL(requestOrigin(req)).host);
    const requestIsTenantHost = tenant.hostnames.some((h) => normalizeHostname(h) === requestHost);
    const canonicalLooksPlatform = isPlatformHostname(canonicalHost);
    const origin =
      canonicalLooksPlatform && requestIsTenantHost
        ? tenantOrigin(tenant, requestHost)
        : canonicalOrigin;

    const candidates = awards.filter(
      (a) => a.directionSignature && (a.status === "direction_signed" || a.status === "issued"),
    );
    let generated = 0;
    const errors: string[] = [];

    for (const award of candidates) {
      const result = await issueAwardPdf(award, origin);
      if ("error" in result) {
        errors.push(`${award.student.prenom} ${award.student.nom}: ${result.error}`);
        continue;
      }
      await saveAward(result);
      generated += 1;
    }

    return NextResponse.json({
      ok: true,
      action,
      totalInProgram: awards.length,
      eligible: candidates.length,
      done: generated,
      errors,
    });
  }

  if (action === "send-ocr-all") {
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canManageOcrFromRoles(roles)) {
      return NextResponse.json({ error: "Action réservée à l'administratif." }, { status: 403 });
    }

    const issued = awards.filter((a) => a.status === "issued" && a.pdfS3Key);
    if (!issued.length) {
      return NextResponse.json({ error: "Aucun certificat PDF émis à envoyer en OCR." }, { status: 400 });
    }

    const odProfile = await resolveOneDriveProfileForClerkUserServer({
      lastName: user?.lastName,
      emailAddresses: user?.emailAddresses?.map((e) => ({ emailAddress: e.emailAddress })),
      primaryEmailAddress: user?.primaryEmailAddress
        ? { emailAddress: user.primaryEmailAddress.emailAddress }
        : null,
    });
    if (!odProfile) {
      return NextResponse.json({ error: "Profil OneDrive non configuré pour ce compte." }, { status: 400 });
    }

    const tenant = await getTenant();
    const secrets = await getTenantSecrets(tenant.slug);
    const refreshToken = secrets?.microsoft?.oneDriveBySecteur?.[odProfile.secteur]?.refreshToken?.trim() || "";
    if (!refreshToken) {
      return NextResponse.json(
        { error: "Refresh token OneDrive manquant pour ce secteur (Paramètres tenant)." },
        { status: 400 },
      );
    }
    const tokenResult = await getMicrosoftAccessTokenFromRefresh(refreshToken);
    if ("error" in tokenResult || !tokenResult.accessToken) {
      return NextResponse.json({ error: "Impossible d'obtenir un token OneDrive serveur." }, { status: 500 });
    }

    const accessToken = tokenResult.accessToken;
    const tempFolder = `${odProfile.basePath.replace(/\/+$/, "")}/Temp`;
    await ensureFolderPath(accessToken, tempFolder);

    const s3 = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const items: OcrBatchJobItem[] = [];

    for (const award of issued) {
      const key = award.pdfS3Key;
      if (!key) continue;
      try {
        const obj = await s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
        const bytes = await obj.Body?.transformToByteArray();
        if (!bytes?.length) continue;

        const fileName = safeFileNamePart(
          `${award.student.nom}_${award.student.prenom}_${award.student.classe || "SansClasse"}.pdf`,
        );
        const uploaded = await uploadBytesToOneDriveUnique(accessToken, tempFolder, fileName, bytes);
        if (!uploaded.ok) continue;

        items.push({
          id: `item_${items.length + 1}`,
          fileName: uploaded.fileName,
          mode: "standard",
          s3Key: key,
          tempPath: uploaded.path,
          status: "pending",
        });
      } catch {
        // on continue avec les autres certificats
      }
    }

    if (!items.length) {
      return NextResponse.json({ error: "Aucun fichier n'a pu être déposé dans OneDrive Temp." }, { status: 400 });
    }

    const jobId = newBatchJobId();
    const now = new Date().toISOString();
    const job: OcrBatchJob = {
      jobId,
      userId: gate.ctx.userId,
      status: "pending",
      startedAt: now,
      updatedAt: now,
      accessToken,
      refreshToken,
      originUrl: requestOrigin(req),
      items,
      currentItemIndex: 0,
      results: [],
      label: `Certificats OCR — ${items.length} PDF`,
      percent: 0,
      completed: 0,
      failed: 0,
    };

    await writeBatchJob(job);
    await registerBatchJobForUser(gate.ctx.userId, jobId);
    await kickOcrBatchWorker(jobId, job.originUrl);

    return NextResponse.json({
      ok: true,
      action,
      jobId,
      totalInProgram: awards.length,
      eligible: issued.length,
      done: items.length,
      errors: issued.length - items.length,
    });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}

