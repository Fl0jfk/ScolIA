import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireAdmin } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName, getSignedReadUrl } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

const ALLOWED_TYPES = new Set([
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/svg+xml",
]);

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "logo";
}

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const { fileName, fileType } = await req.json();
    const type = String(fileType || "").trim().toLowerCase();
    if (!ALLOWED_TYPES.has(type)) {
      return NextResponse.json(
        { error: "Format non supporté. Utilisez PNG, JPEG, WebP ou SVG." },
        { status: 400 },
      );
    }

    const ext = type === "image/svg+xml" ? "svg" : type.split("/")[1] || "png";
    const fileKey = s3Key(`settings/branding/header-logo-${Date.now()}-${safeFileName(String(fileName || `logo.${ext}`))}`);
    const s3Client = await getTenantDataS3Client();

    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: type,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });

    // fileKey = référence stable en JSON (pas d’URL absolue liée à un bucket).
    // previewUrl = URL signée temporaire pour l’aperçu UI.
    return NextResponse.json({
      uploadUrl,
      fileKey,
      fileUrl: fileKey,
      previewUrl: (await getSignedReadUrl(fileKey, 3600)) || null,
    });
  } catch (error) {
    console.error("[settings/upload-logo]", error);
    return NextResponse.json({ error: "Erreur lors de la préparation de l'upload." }, { status: 500 });
  }
}
