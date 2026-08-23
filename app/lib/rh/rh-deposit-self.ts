import "server-only";

import { uploadFileToOneDriveFolder } from "@/app/lib/graph-onedrive-folders";
import { getRhDriveAccessToken } from "@/app/lib/rh/graph-rh-drive";
import { readMetaRhByFolderName, readRhPersonnelIndex, writeMetaRh } from "@/app/lib/rh/meta-storage";
import { resolveRhIndexEntryForUser } from "@/app/lib/rh/rh-personnel-match";
import { rhDocumentPath, rhDocumentsRoot } from "@/app/lib/rh/paths";
import { rhUid, type RhDocumentRef } from "@/app/lib/rh/types";

function safeFileName(name: string) {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 120) || "document";
}

export async function depositRhSelfDocument(input: {
  externalUserId: string;
  email: string;
  uploadedBy: string;
  fileName: string;
  bytes: Uint8Array;
  contentType?: string;
}): Promise<{ ok: true; path: string; personnelId: string } | { ok: false; error: string }> {
  const token = await getRhDriveAccessToken();
  if ("error" in token) return { ok: false, error: token.error };

  const indexHit = await readRhPersonnelIndex();
  if (!indexHit.ok) return { ok: false, error: indexHit.error };

  const entry = resolveRhIndexEntryForUser(
    indexHit.index.entries,
    input.externalUserId,
    input.email,
  );
  if (!entry) {
    return {
      ok: false,
      error:
        "Aucun dossier RH lié à votre compte. Contactez la direction des ressources humaines.",
    };
  }

  const safeName = safeFileName(input.fileName);
  const folder = `${rhDocumentsRoot(entry.folderName, token.basePath)}/personnels`;
  const upload = await uploadFileToOneDriveFolder(
    token.accessToken,
    folder,
    safeName,
    input.bytes,
    input.contentType || "application/pdf",
  );

  const metaHit = await readMetaRhByFolderName(entry.folderName);
  if (metaHit.ok) {
    const doc: RhDocumentRef = {
      id: rhUid("doc"),
      name: upload.fileName,
      oneDrivePath: upload.fullPath,
      category: "personnel",
      uploadedAt: new Date().toISOString(),
      uploadedBy: input.uploadedBy,
    };
    await writeMetaRh(entry.folderName, {
      ...metaHit.meta,
      documents: [...metaHit.meta.documents, doc],
    });
  }

  return { ok: true, path: upload.fullPath, personnelId: entry.id };
}

export async function appendRhDocumentToPersonnel(input: {
  folderName: string;
  fileName: string;
  oneDrivePath: string;
  category: RhDocumentRef["category"];
  uploadedBy: string;
}): Promise<void> {
  const metaHit = await readMetaRhByFolderName(input.folderName);
  if (!metaHit.ok) return;
  const doc: RhDocumentRef = {
    id: rhUid("doc"),
    name: input.fileName,
    oneDrivePath: input.oneDrivePath,
    category: input.category,
    uploadedAt: new Date().toISOString(),
    uploadedBy: input.uploadedBy,
  };
  await writeMetaRh(input.folderName, {
    ...metaHit.meta,
    documents: [...metaHit.meta.documents, doc],
  });
}

/** Chemin cible pour un dépôt RH identifié (OCR). */
function rhTargetPathForAnalyze(
  folderName: string,
  subfolder: "contrats" | "formations" | "habilitations" | "medecine" | "personnels" | "divers",
  fileName: string,
  basePath: string,
) {
  return rhDocumentPath(folderName, subfolder, fileName, basePath);
}
