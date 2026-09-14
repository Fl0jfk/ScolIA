import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireAuth } from "@/app/lib/intranet-auth";
import { loadAppConfig } from "@/app/lib/app-config";
import { getJson, getBucketName } from "@/app/lib/s3-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { s3Key } from "@/app/lib/s3-path";
import {
  canCreatePhotocopiesDemand,
  canViewPhotocopiesDemand,
  getPhotocopiesRoleFlags,
} from "@/app/lib/photocopies-couleur-access";
import { resolvePhotocopiesOpsViewer } from "@/app/lib/photocopies-couleur-ops-server";
import {
  getPhotocopieDocuments,
  type PhotoCopieRecord,
} from "@/app/lib/photocopies-couleur-types";

const INDEX_KEY = "photocopies-couleur/index.json";

function isValidDocumentKey(key: string): boolean {
  return key.startsWith("photocopies-couleur/uploads/") && !key.includes("..");
}

/**
 * Lien signé vers un PDF joint d'une demande (direction, ops, demandeur).
 * GET ?id=<demandeId>&index=<0-based, défaut 0>
 */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const url = new URL(req.url);
  const id = url.searchParams.get("id")?.trim() || "";
  const indexRaw = url.searchParams.get("index");
  const index = indexRaw == null || indexRaw === "" ? 0 : Number(indexRaw);
  if (!id) {
    return NextResponse.json({ error: "Identifiant manquant." }, { status: 400 });
  }
  if (!Number.isInteger(index) || index < 0) {
    return NextResponse.json({ error: "Index de document invalide." }, { status: 400 });
  }

  const viewer = await resolvePhotocopiesOpsViewer();
  const userId = viewer.businessUserId || gate.ctx.userId;
  const roles = viewer.roles;
  const isOps = viewer.isOps;
  const bundle = await loadAppConfig();

  if (!canCreatePhotocopiesDemand(roles) && !isOps) {
    const f = getPhotocopiesRoleFlags(roles);
    if (!f.isDirection) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }
  }

  try {
    const hit = await getJson<PhotoCopieRecord[]>(INDEX_KEY);
    const all = Array.isArray(hit?.data) ? hit.data : [];
    const record = all.find((r) => r.id === id);
    if (!record) {
      return NextResponse.json({ error: "Demande introuvable." }, { status: 404 });
    }
    if (
      !canViewPhotocopiesDemand(record, userId, roles, bundle.establishments, {
        isOpsHandler: isOps,
        altUserIds: [viewer.authUserId],
      })
    ) {
      return NextResponse.json({ error: "Accès refusé à cette demande." }, { status: 403 });
    }

    const docs = getPhotocopieDocuments(record).filter((d) => isValidDocumentKey(d.key));
    if (docs.length === 0) {
      return NextResponse.json({ error: "Aucun document joint à cette demande." }, { status: 404 });
    }
    const doc = docs[index];
    if (!doc) {
      return NextResponse.json(
        { error: `Document introuvable (index ${index}, ${docs.length} PDF joint(s)).` },
        { status: 404 },
      );
    }

    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: s3Key(doc.key),
      ResponseContentType: doc.contentType || "application/pdf",
      ResponseContentDisposition: `inline; filename="${(doc.fileName || "document.pdf").replace(/"/g, "")}"`,
    });
    const signedUrl = await getSignedUrl(s3Client, command, { expiresIn: 900 });

    return NextResponse.json({
      signedUrl,
      fileName: doc.fileName || "document.pdf",
      contentType: doc.contentType || "application/pdf",
      index,
      total: docs.length,
    });
  } catch (e) {
    console.error("[photocopies-couleur/document]", e);
    return NextResponse.json({ error: "Impossible d'ouvrir le document." }, { status: 500 });
  }
}
