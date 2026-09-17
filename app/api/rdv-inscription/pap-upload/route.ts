import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";

const uploadLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 8,
});

const MAX_BYTES = 15 * 1024 * 1024;
const ALLOWED = new Set([
  "application/pdf",
  "image/jpeg",
  "image/png",
  "image/webp",
]);

/** Presign public pour dépôt PAP facultatif (avant confirmation RDV). */
export async function POST(req: Request) {
  try {
    if (!(await uploadLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de téléversements. Réessayez plus tard." },
        { status: 429 },
      );
    }

    const etabId = await resolveCurrentEtablissementId();
    if (!etabId) {
      return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
    }

    const body = (await req.json()) as {
      fileName?: string;
      contentType?: string;
      size?: number;
    };
    const fileName = sanitizeS3FileName(String(body.fileName || "pap.pdf"));
    const contentType = String(body.contentType || "application/pdf").trim().toLowerCase();
    const size = Number(body.size || 0);
    if (!ALLOWED.has(contentType)) {
      return NextResponse.json(
        { error: "Formats acceptés : PDF, JPEG, PNG, WebP." },
        { status: 400 },
      );
    }
    if (!Number.isFinite(size) || size <= 0 || size > MAX_BYTES) {
      return NextResponse.json({ error: "Fichier trop volumineux (max 15 Mo)." }, { status: 400 });
    }

    const fileKey = s3Key(`rdv-inscription/pap/${Date.now()}-${fileName}`);
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const command = new PutObjectCommand({
      Bucket: bucket,
      Key: fileKey,
      ContentType: contentType,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });

    return NextResponse.json({
      uploadUrl,
      s3Key: fileKey,
      fileName,
      contentType,
    });
  } catch (e) {
    return NextResponse.json({ error: String(e) }, { status: 500 });
  }
}
