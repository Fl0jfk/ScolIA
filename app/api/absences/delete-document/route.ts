import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import { DeleteObjectCommand } from "@aws-sdk/client-s3";

import { getAbsenceDocumentKeys, isDocumentKeyReferenced } from "@/app/lib/absences-documents";
import { isDocumentKeyReferencedInLegacy } from "@/app/lib/absences-legacy-convocations";
import { canManageAbsenceAttachment } from "@/app/lib/absences-types";
import { getAbsenceIndex, getAbsenceRecord, saveAbsenceIndex, saveAbsenceRecord } from "@/app/lib/absences-storage";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);

  try {
    const body = await req.json().catch(() => ({}));
    const id = String(body.id || "").trim();
    const target = String(body.target || "").trim();
    const docIndex = Number(body.index ?? 0);
    if (!id) return NextResponse.json({ error: "Identifiant du créneau manquant." }, { status: 400 });

    const record = await getAbsenceRecord(id);
    if (!record) return NextResponse.json({ error: "Créneau introuvable." }, { status: 404 });
    if (!canManageAbsenceAttachment(record, roles)) {
      return NextResponse.json({ error: "Suppression non autorisée." }, { status: 403 });
    }

    const documentKeys = [...getAbsenceDocumentKeys(record)];
    const justificationUrl = record.justification?.fileUrl?.trim();
    let justificationKey: string | null = null;
    if (justificationUrl) {
      justificationKey = (await resolveTravelsS3ObjectKey(justificationUrl)) || null;
    }

    let keyToDelete: string | null = null;
    let isJustificationDoc = false;

    if (target === "justification") {
      if (!justificationKey) {
        return NextResponse.json({ error: "Aucun justificatif sur ce créneau." }, { status: 404 });
      }
      keyToDelete = justificationKey;
      isJustificationDoc = true;
    } else {
      if (!Number.isFinite(docIndex)) {
        return NextResponse.json({ error: "Index de document invalide." }, { status: 400 });
      }
      const allKeys = [...documentKeys];
      if (justificationKey && !allKeys.includes(justificationKey)) allKeys.push(justificationKey);
      if (allKeys.length === 0) {
        return NextResponse.json({ error: "Aucun document sur ce créneau." }, { status: 404 });
      }
      const index = Math.max(0, Math.min(allKeys.length - 1, Math.floor(docIndex)));
      keyToDelete = allKeys[index]!;
      isJustificationDoc = !!justificationKey && keyToDelete === justificationKey && index >= documentKeys.length;
    }

    if (isJustificationDoc) {
      record.justification = null;
      if (record.workflowStatus === "JUSTIFICATIF_DEPOSE") {
        record.workflowStatus = "OUVERTE";
      }
    } else if (keyToDelete) {
      const keyIndex = documentKeys.indexOf(keyToDelete);
      if (keyIndex < 0) {
        return NextResponse.json({ error: "Document introuvable." }, { status: 404 });
      }
      documentKeys.splice(keyIndex, 1);
      record.data.documentKeys = documentKeys;
    } else {
      return NextResponse.json({ error: "Document introuvable." }, { status: 404 });
    }

    const now = new Date().toISOString();
    record.updatedAt = now;
    record.history = [
      ...(record.history || []),
      {
        at: now,
        by: user?.fullName || "Utilisateur",
        action: "DOCUMENT_SUPPRIME",
        note: isJustificationDoc ? "Justificatif supprimé" : "Document supprimé",
      },
    ];

    await saveAbsenceRecord(record);
    const indexList = await getAbsenceIndex();
    await saveAbsenceIndex(indexList.map((r) => (r.id === id ? record : r)));

    const bucket = await getBucketName();
    const s3Client = await getTenantDataS3Client();
    const stillUsed =
      isDocumentKeyReferenced(
        indexList.map((r) => (r.id === id ? record : r)),
        keyToDelete,
      ) || (await isDocumentKeyReferencedInLegacy(keyToDelete));
    if (!stillUsed) {
      await s3Client.send(new DeleteObjectCommand({ Bucket: bucket, Key: s3Key(keyToDelete) }));
    }

    const remaining =
      getAbsenceDocumentKeys(record).length + (record.justification?.fileUrl ? 1 : 0);

    return NextResponse.json({ success: true, documentCount: remaining });
  } catch (error) {
    console.error("Absences delete-document error:", error);
    return NextResponse.json({ error: "Erreur lors de la suppression du document." }, { status: 500 });
  }
}
