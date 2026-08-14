import { NextResponse } from "next/server";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireAuth } from "@/app/lib/intranet-auth";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { canAccessRgpdModule } from "@/app/lib/rgpd-access";
import { getRgpdCatalogEntry } from "@/app/lib/rgpd-catalog";
import { newRgpdId, uploadDocKey } from "@/app/lib/rgpd-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

const ALLOWED = new Set([
  "application/pdf",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
]);

export async function POST(
  req: Request,
  ctx: { params: Promise<{ docId: string }> },
) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessRgpdModule(roles)) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { docId } = await ctx.params;
  if (!getRgpdCatalogEntry(docId)) {
    return NextResponse.json({ error: "Document inconnu." }, { status: 404 });
  }

  try {
    const { fileName, contentType } = await req.json();
    const ct = String(contentType || "application/pdf").trim();
    if (!ALLOWED.has(ct)) {
      return NextResponse.json(
        { error: "Formats acceptés : PDF, DOCX." },
        { status: 400 },
      );
    }
    const ext = ct.includes("word") ? "docx" : "pdf";
    const fileId = newRgpdId("up");
    const relative = uploadDocKey(docId, fileId, ext);
    const fileKey = s3Key(relative);
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: ct,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
    return NextResponse.json({
      uploadUrl,
      key: relative,
      fileId,
      fileName: String(fileName || `document.${ext}`),
    });
  } catch (e) {
    console.error("[rgpd/upload-url]", e);
    return NextResponse.json({ error: "Préparation upload impossible." }, { status: 500 });
  }
}
