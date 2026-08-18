import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { s3Key, sanitizeS3FileName } from "@/app/lib/s3-path";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName, putObject } from "@/app/lib/s3-storage";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";
import { requireAuth } from "@/app/lib/intranet-auth";

export const maxDuration = 60;

function buildAttachmentKey(fileName: string): string {
  return s3Key(`attachments/${Date.now()}-${sanitizeS3FileName(fileName)}`);
}

/** Upload direct multipart (recommandé — évite les échecs silencieux du PUT présigné navigateur). */
async function uploadFromFormData(req: Request) {
  const formData = await req.formData();
  const file = formData.get("file");
  if (!(file instanceof File) && !(file instanceof Blob)) {
    return NextResponse.json({ error: "Fichier manquant." }, { status: 400 });
  }
  const rawName = file instanceof File ? file.name : "document";
  const fileKey = buildAttachmentKey(rawName);
  const buffer = Buffer.from(await file.arrayBuffer());
  await putObject(fileKey, buffer, file.type || "application/octet-stream");
  return NextResponse.json({
    fileUrl: await publicS3UrlForKey(fileKey),
    s3Key: fileKey,
  });
}

/** Flux legacy : URL présignée pour PUT navigateur (conservé pour compatibilité). */
async function uploadPresignedFromJson(req: Request) {
  const { fileName, fileType } = await req.json();
  const fileKey = buildAttachmentKey(String(fileName || "document"));
  const s3Client = await getTenantDataS3Client();
  const bucket = await getBucketName();
  const command = new PutObjectCommand({
    Bucket: bucket,
    Key: fileKey,
    ContentType: fileType,
  });
  const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
  return NextResponse.json({
    uploadUrl,
    fileUrl: await publicS3UrlForKey(fileKey),
    s3Key: fileKey,
  });
}

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const contentType = req.headers.get("content-type") || "";
    if (contentType.includes("multipart/form-data")) {
      return uploadFromFormData(req);
    }
    return uploadPresignedFromJson(req);
  } catch (error) {
    console.error("[travels/upload]", error);
    return NextResponse.json({ error: "Erreur upload" }, { status: 500 });
  }
}
