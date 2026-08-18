import { NextResponse } from 'next/server';
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { s3Key, sanitizeS3FileName } from "@/app/lib/s3-path";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";
import { requireAuth } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const { fileName, fileType } = await req.json();
    const fileKey = s3Key( `attachments/${Date.now()}-${sanitizeS3FileName(fileName)}`);
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
  } catch (error) {
    console.error(error);
    return NextResponse.json({ error: "Erreur upload" }, { status: 500 });
  }
}
