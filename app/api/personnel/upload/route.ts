import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key, sanitizeS3FileName } from "@/app/lib/s3-path";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";
import { canAccessPersonnelModule } from "@/app/lib/personnel-types";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const user = await safeCurrentUser();
  const roles = rolesFromUserLike(user);
  if (!canAccessPersonnelModule(roles)) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  try {
    const { fileName, fileType, staffId } = await req.json();
    const safeName = sanitizeS3FileName(String(fileName || "document")).replace(/[^a-zA-Z0-9._-]/g, "_");
    const safeStaffId = sanitizeS3FileName(String(staffId || ""));
    const folder = safeStaffId && safeStaffId !== "file" ? `personnel-ogec/${safeStaffId}/documents` : "personnel-ogec/shared";
    const fileKey = s3Key(`${folder}/${Date.now()}-${safeName}`);

    const s3Client = await getTenantDataS3Client();

    const command = new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: fileKey,
      ContentType: String(fileType || "application/octet-stream"),
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });

    return NextResponse.json({
      uploadUrl,
      fileUrl: await publicS3UrlForKey(fileKey),
      s3Key: fileKey,
    });
  } catch (error) {
    console.error("[personnel/upload]", error);
    return NextResponse.json({ error: "Erreur upload." }, { status: 500 });
  }
}
