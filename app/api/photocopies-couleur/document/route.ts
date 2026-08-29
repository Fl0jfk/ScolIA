import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
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
import { resolvePhotocopiesOpsEmails } from "@/app/lib/photocopies-couleur-ops";
import { isPhotocopiesOpsHandlerResolved } from "@/app/lib/photocopies-couleur-ops-server";
import { loadModuleAccess } from "@/app/lib/module-access-store";
import type { PhotoCopieRecord } from "@/app/lib/photocopies-couleur-types";

const INDEX_KEY = "photocopies-couleur/index.json";

function isValidDocumentKey(key: string): boolean {
  return key.startsWith("photocopies-couleur/uploads/") && !key.includes("..");
}

/**
 * Lien signé vers le PDF joint d'une demande (direction, ops, demandeur).
 * GET ?id=<demandeId>
 */
export async function GET(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const { userId } = gate.ctx;

  const id = new URL(req.url).searchParams.get("id")?.trim() || "";
  if (!id) {
    return NextResponse.json({ error: "Identifiant manquant." }, { status: 400 });
  }

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  const email = user?.primaryEmailAddress?.emailAddress?.trim() || "";
  const bundle = await loadAppConfig();
  const opsEmails = resolvePhotocopiesOpsEmails(bundle.notifications);
  const moduleAccess = await loadModuleAccess().catch(() => null);
  const isOps = isPhotocopiesOpsHandlerResolved({
    email,
    opsEmails,
    moduleAccess,
    lookup: { userId, businessUserId: userId },
  });

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
    if (!canViewPhotocopiesDemand(record, userId, roles, bundle.establishments, { isOpsHandler: isOps })) {
      return NextResponse.json({ error: "Accès refusé à cette demande." }, { status: 403 });
    }
    if (!record.documentKey || !isValidDocumentKey(record.documentKey)) {
      return NextResponse.json({ error: "Aucun document joint à cette demande." }, { status: 404 });
    }

    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: s3Key(record.documentKey),
      ResponseContentType: record.documentContentType || "application/pdf",
      ResponseContentDisposition: `inline; filename="${(record.documentFileName || "document.pdf").replace(/"/g, "")}"`,
    });
    const signedUrl = await getSignedUrl(s3Client, command, { expiresIn: 900 });

    return NextResponse.json({
      signedUrl,
      fileName: record.documentFileName || "document.pdf",
      contentType: record.documentContentType || "application/pdf",
    });
  } catch (e) {
    console.error("[photocopies-couleur/document]", e);
    return NextResponse.json({ error: "Impossible d'ouvrir le document." }, { status: 500 });
  }
}
