import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

const ALLOWED = new Set(["application/pdf"]);

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "document.pdf";
}

/** Prépare un upload PDF pour Scolia AI (pièce jointe conversation). */
export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const fileName = String(body.fileName || "document.pdf");
    const contentType = String(body.contentType || "application/pdf").trim();
    if (!ALLOWED.has(contentType)) {
      return NextResponse.json({ error: "Seuls les PDF sont acceptés pour le moment." }, { status: 400 });
    }

    const relative = `brain-ai/uploads/${Date.now()}-${safeFileName(fileName)}`;
    const fileKey = s3Key(relative);
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const uploadUrl = await getSignedUrl(
      s3Client,
      new PutObjectCommand({
        Bucket: bucket,
        Key: fileKey,
        ContentType: contentType,
      }),
      { expiresIn: 3600 },
    );

    return NextResponse.json({
      uploadUrl,
      key: relative,
      fileName: safeFileName(fileName),
      contentType,
    });
  } catch (e) {
    console.error("[chatbot/upload]", e);
    return NextResponse.json({ error: "Préparation de l'upload impossible." }, { status: 500 });
  }
}
