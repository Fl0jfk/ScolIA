import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireModule } from "@/app/lib/intranet-auth";
import { rentreePublicDocumentPathUrl } from "@/app/lib/rentree-public-urls";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

const ALLOWED_TYPES = new Set([
  "application/pdf",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.ms-excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "image/png",
  "image/jpeg",
  "image/webp",
]);

function safeSegment(v: string): string {
  return v.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 48) || "doc";
}

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "document";
}

export async function POST(req: Request) {
  const gate = await requireModule("evenements");
  if (!gate.ok) return gate.response;

  try {
    const { fileName, fileType, establishmentId } = await req.json();
    const type = String(fileType || "").trim().toLowerCase();
    if (!ALLOWED_TYPES.has(type)) {
      return NextResponse.json(
        { error: "Format non supporté. Utilisez PDF, Word, Excel ou image." },
        { status: 400 },
      );
    }

    const estId = safeSegment(String(establishmentId || "general"));
    const ext =
      type === "application/pdf"
        ? "pdf"
        : type.includes("word")
          ? "docx"
          : type.includes("sheet") || type.includes("excel")
            ? "xlsx"
            : type.split("/")[1] || "bin";

    const fileKey = s3Key(
      `toolbox/rentree/${estId}/${Date.now()}-${safeFileName(String(fileName || `document.${ext}`))}`,
    );
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: type,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });

    return NextResponse.json({
      uploadUrl,
      fileUrl: rentreePublicDocumentPathUrl(fileKey),
    });
  } catch (error) {
    console.error("[toolbox/rentree/upload]", error);
    return NextResponse.json({ error: "Erreur lors de la préparation de l'upload." }, { status: 500 });
  }
}
