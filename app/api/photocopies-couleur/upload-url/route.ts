import { safeCurrentUser } from "@/app/lib/intranet-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { canCreatePhotocopiesDemand } from "@/app/lib/photocopies-couleur-access";

const ALLOWED_TYPES = new Set(["application/pdf"]);

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "document.pdf";
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const { fileName, contentType } = await req.json();
    const ct = String(contentType || "application/pdf").trim();
    if (!ALLOWED_TYPES.has(ct)) {
      return NextResponse.json({ error: "Seuls les fichiers PDF sont acceptés." }, { status: 400 });
    }

    const user = await safeCurrentUser();
    const roles = rolesFromUserLike(user);
    if (!canCreatePhotocopiesDemand(roles)) {
      return NextResponse.json({ error: "Accès réservé aux demandeurs." }, { status: 403 });
    }

    const relative = `photocopies-couleur/uploads/${Date.now()}-${safeFileName(String(fileName || "document.pdf"))}`;
    const fileKey = s3Key(relative);
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: ct,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
    return NextResponse.json({ uploadUrl, key: relative });
  } catch (e) {
    console.error("[photocopies-couleur/upload-url]", e);
    return NextResponse.json({ error: "Préparation de l'upload impossible." }, { status: 500 });
  }
}
