import { DeleteObjectCommand, GetObjectCommand } from "@aws-sdk/client-s3";
import { loadAppConfig } from "@/app/lib/app-config";
import { getMicrosoftAccessTokenFromRefresh } from "@/app/lib/graph-microsoft-delegated";
import { uploadFileToOneDriveFolder } from "@/app/lib/graph-onedrive-folders";
import type { Secteur } from "@/app/lib/onedrive-eleves";
import type { OneDriveUserProfile } from "@/app/lib/onedrive-user-profiles";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getTenant } from "@/app/lib/tenant-context";
import { getTenantSecrets } from "@/app/lib/tenant-registry";
import {
  conventionOneDriveFileName,
  matchEleveForConvention,
  resolveConventionSecteur,
  resolveOneDriveProfileForConvention,
} from "@/app/lib/stage-eleve-match";
import { resolveEleveFolderName } from "@/app/lib/eleves-config";
import { buildStageConventionPdf } from "@/app/lib/stage-pdf";
import { getConventionsIndex, getStageConvention, saveStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention } from "@/app/lib/stage-types";

async function verifyOneDriveAccessToken(accessToken: string): Promise<boolean> {
  const res = await fetch("https://graph.microsoft.com/v1.0/me/drive?$select=id", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  return res.ok;
}

async function isOneDriveIntegrationEnabled(): Promise<boolean> {
  const config = await loadAppConfig();
  return config.integrations.microsoftOneDrive?.enabled === true;
}

async function loadConventionPdfBytes(convention: StageConvention): Promise<Buffer> {
  const key = convention.uploadedPdf?.s3Key;
  if (key) {
    try {
      const s3Client = await getTenantDataS3Client();
      const obj = await s3Client.send(
        new GetObjectCommand({ Bucket: await getBucketName(), Key: key }),
      );
      const bytes = await obj.Body?.transformToByteArray();
      if (bytes?.length) return Buffer.from(bytes);
    } catch {
      /* repli PDF généré */
    }
  }
  return Buffer.from(await buildStageConventionPdf(convention));
}

/** Supprime le PDF de transition sur S3 après dépôt OneDrive réussi. */
async function purgeConventionUploadedPdfFromS3(
  convention: StageConvention,
): Promise<{ purged: boolean; s3Key?: string; error?: string }> {
  const key = convention.uploadedPdf?.s3Key?.trim();
  if (!key) return { purged: true };

  try {
    const s3Client = await getTenantDataS3Client();
    await s3Client.send(
      new DeleteObjectCommand({ Bucket: await getBucketName(), Key: key }),
    );
    return { purged: true, s3Key: key };
  } catch (e) {
    return {
      purged: false,
      s3Key: key,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

async function fileSignedConventionCore(params: {
  convention: StageConvention;
  accessToken: string;
  odProfile: OneDriveUserProfile;
  filedBy: string;
}): Promise<
  | {
      ok: true;
      convention: StageConvention;
      folderPath: string;
      fileName: string;
      fullPath: string;
      matchedFolderName: string;
    }
  | { ok: false; error: string; debug?: Record<string, unknown> }
> {
  const { matchedEleve, folderPath, debug } = await matchEleveForConvention(
    params.convention,
    params.odProfile,
  );
  if (!matchedEleve || !folderPath) {
    return {
      ok: false,
      error:
        "Élève introuvable dans eleves.json — vérifiez nom/prénom ou rattachez l'INE côté admin.",
      debug,
    };
  }

  const pdfBytes = await loadConventionPdfBytes(params.convention);
  const fileName = conventionOneDriveFileName(params.convention);

  const uploaded = await uploadFileToOneDriveFolder(
    params.accessToken,
    folderPath,
    fileName,
    pdfBytes,
  );

  const purge = await purgeConventionUploadedPdfFromS3(params.convention);
  const now = new Date().toISOString();
  const historyEntries = [
    ...params.convention.history,
    {
      at: now,
      by: params.filedBy,
      action: "ONEDRIVE_DEPOSE",
      note: purge.purged
        ? `${uploaded.fullPath} — PDF transition S3 supprimé`
        : `${uploaded.fullPath} — PDF S3 non supprimé : ${purge.error ?? "erreur"}`,
    },
  ];
  if (purge.purged && purge.s3Key) {
    historyEntries.push({
      at: now,
      by: params.filedBy,
      action: "PDF_S3_SUPPRIME",
      note: purge.s3Key,
    });
  }

  const next: StageConvention = {
    ...params.convention,
    updatedAt: now,
    uploadedPdf: purge.purged ? undefined : params.convention.uploadedPdf,
    oneDriveFiling: {
      filedAt: now,
      filedBy: params.filedBy,
      folderPath: uploaded.folderPath,
      fileName: uploaded.fileName,
      matchedFolderName: resolveEleveFolderName(matchedEleve),
    },
    oneDriveFilingPending: false,
    oneDriveFilingError: purge.purged ? undefined : `PDF S3 non supprimé : ${purge.error ?? "erreur"}`,
    history: historyEntries,
  };
  await saveStageConvention(next);

  return {
    ok: true,
    convention: next,
    folderPath: uploaded.folderPath,
    fileName: uploaded.fileName,
    fullPath: uploaded.fullPath,
    matchedFolderName: resolveEleveFolderName(matchedEleve),
  };
}

export async function fileSignedConventionToOneDrive(params: {
  conventionId: string;
  accessToken: string;
  odProfile: OneDriveUserProfile | null;
  filedBy: string;
}): Promise<
  | {
      ok: true;
      convention: StageConvention;
      folderPath: string;
      fileName: string;
      fullPath: string;
      matchedFolderName: string;
    }
  | { ok: false; error: string; debug?: Record<string, unknown> }
> {
  const enabled = await isOneDriveIntegrationEnabled();
  if (!enabled) {
    return { ok: false, error: "OneDrive n'est pas activé pour cet établissement (Paramètres → Intégrations)." };
  }

  const tokenOk = await verifyOneDriveAccessToken(params.accessToken);
  if (!tokenOk) {
    return {
      ok: false,
      error: "Session OneDrive invalide ou expirée. Reconnectez-vous à Microsoft.",
    };
  }

  const convention = await getStageConvention(params.conventionId);
  if (!convention) return { ok: false, error: "Convention introuvable." };
  if (convention.status !== "signed") {
    return { ok: false, error: "La convention doit être signée par toutes les parties avant envoi." };
  }
  if (convention.oneDriveFiling) {
    return { ok: false, error: "Convention déjà déposée sur OneDrive." };
  }

  const odProfile = await resolveOneDriveProfileForConvention(convention, params.odProfile);
  if (!odProfile) {
    return {
      ok: false,
      error:
        "Impossible de déterminer le dossier OneDrive (secteur élève) — vérifiez la classe ou rattachez l'élève.",
    };
  }

  return fileSignedConventionCore({
    convention,
    accessToken: params.accessToken,
    odProfile,
    filedBy: params.filedBy,
  });
}

async function markOneDriveFilingPending(
  convention: StageConvention,
  reason: string,
): Promise<void> {
  if (convention.oneDriveFiling) return;
  const now = new Date().toISOString();
  const next: StageConvention = {
    ...convention,
    updatedAt: now,
    oneDriveFilingPending: true,
    oneDriveFilingError: reason,
    history: [
      ...convention.history,
      { at: now, by: "Système", action: "ONEDRIVE_EN_ATTENTE", note: reason },
    ],
  };
  await saveStageConvention(next);
}

/** Dépôt automatique serveur après signature complète (refresh token par secteur). */
export async function tryAutoFileConventionToOneDrive(
  convention: StageConvention,
): Promise<{ filed: boolean; reason?: string }> {
  if (convention.oneDriveFiling) return { filed: true };
  if (convention.status !== "signed") return { filed: false, reason: "not_signed" };
  if (!(await isOneDriveIntegrationEnabled())) return { filed: false, reason: "onedrive_disabled" };

  const secteur = await resolveConventionSecteur(convention);
  if (!secteur) {
    await markOneDriveFilingPending(
      convention,
      "Secteur élève non déterminé — rattachez l'élève ou précisez la classe.",
    );
    return { filed: false, reason: "no_secteur" };
  }

  const tenant = await getTenant();
  const secrets = await getTenantSecrets(tenant.slug);
  const refreshToken = secrets?.microsoft?.oneDriveBySecteur?.[secteur as Secteur]?.refreshToken;
  if (!refreshToken?.trim()) {
    await markOneDriveFilingPending(
      convention,
      `Dépôt auto non configuré pour le ${secteur} — utilisez le bouton manuel ou ajoutez un refresh token.`,
    );
    return { filed: false, reason: "no_refresh_token" };
  }

  const tokenResult = await getMicrosoftAccessTokenFromRefresh(refreshToken);
  if ("error" in tokenResult) {
    await markOneDriveFilingPending(convention, tokenResult.error);
    return { filed: false, reason: "token_error" };
  }

  const odProfile = await resolveOneDriveProfileForConvention(convention, null);
  if (!odProfile) {
    await markOneDriveFilingPending(convention, "Profil OneDrive secteur introuvable.");
    return { filed: false, reason: "no_profile" };
  }

  const result = await fileSignedConventionCore({
    convention,
    accessToken: tokenResult.accessToken,
    odProfile,
    filedBy: "Automatique",
  });

  if (!result.ok) {
    await markOneDriveFilingPending(convention, result.error);
    return { filed: false, reason: result.error };
  }

  return { filed: true };
}

async function listSignedConventionsPendingOneDrive(): Promise<StageConvention[]> {
  const index = await getConventionsIndex();
  const all = await Promise.all(index.map((e) => getStageConvention(e.id)));
  return all.filter(
    (c): c is StageConvention =>
      Boolean(c && c.status === "signed" && !c.oneDriveFiling),
  );
}

type BatchOneDriveFilingResult = {
  total: number;
  filed: number;
  skippedOtherSecteur: number;
  failed: Array<{ id: string; studentName: string; error: string }>;
  filedItems: Array<{ id: string; studentName: string; fullPath: string }>;
};

function studentName(c: StageConvention): string {
  return `${c.student.firstName} ${c.student.lastName}`.trim() || c.id;
}

/** Envoi groupé — une session Microsoft ; le PDF de transition S3 est supprimé après dépôt. */
export async function batchFileSignedConventionsToOneDrive(params: {
  accessToken: string;
  odProfile: OneDriveUserProfile | null;
  filedBy: string;
}): Promise<BatchOneDriveFilingResult | { error: string }> {
  const enabled = await isOneDriveIntegrationEnabled();
  if (!enabled) {
    return { error: "OneDrive n'est pas activé pour cet établissement (Paramètres → Intégrations)." };
  }

  const tokenOk = await verifyOneDriveAccessToken(params.accessToken);
  if (!tokenOk) {
    return { error: "Session OneDrive invalide ou expirée. Reconnectez-vous à Microsoft." };
  }

  const pending = await listSignedConventionsPendingOneDrive();
  const result: BatchOneDriveFilingResult = {
    total: pending.length,
    filed: 0,
    skippedOtherSecteur: 0,
    failed: [],
    filedItems: [],
  };

  for (const convention of pending) {
    const targetProfile = await resolveOneDriveProfileForConvention(
      convention,
      params.odProfile,
    );
    if (!targetProfile) {
      result.failed.push({
        id: convention.id,
        studentName: studentName(convention),
        error: "Secteur ou dossier OneDrive introuvable.",
      });
      continue;
    }

    if (params.odProfile && targetProfile.secteur !== params.odProfile.secteur) {
      result.skippedOtherSecteur += 1;
      continue;
    }

    const filed = await fileSignedConventionCore({
      convention,
      accessToken: params.accessToken,
      odProfile: targetProfile,
      filedBy: params.filedBy,
    });

    if (!filed.ok) {
      result.failed.push({
        id: convention.id,
        studentName: studentName(convention),
        error: filed.error,
      });
      continue;
    }

    result.filed += 1;
    result.filedItems.push({
      id: convention.id,
      studentName: studentName(convention),
      fullPath: filed.fullPath,
    });
  }

  return result;
}

export async function previewBatchOneDriveFiling(odProfile: OneDriveUserProfile | null): Promise<{
  totalPending: number;
  forMySecteur: number;
  bySecteur: Partial<Record<Secteur, number>>;
}> {
  const pending = await listSignedConventionsPendingOneDrive();
  const bySecteur: Partial<Record<Secteur, number>> = {};
  let forMySecteur = 0;

  for (const c of pending) {
    const secteur = await resolveConventionSecteur(c);
    if (secteur) {
      bySecteur[secteur] = (bySecteur[secteur] ?? 0) + 1;
      if (odProfile?.secteur === secteur) forMySecteur += 1;
    } else if (!odProfile) {
      forMySecteur += 1;
    }
  }

  if (!odProfile) {
    forMySecteur = pending.length;
  }

  return { totalPending: pending.length, forMySecteur, bySecteur };
}
