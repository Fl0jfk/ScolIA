import { resolveSession } from "@/app/lib/intranet-session";
import { NextResponse } from 'next/server';
import { PutObjectCommand } from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { getTenantDataS3Client } from '@/app/lib/s3-clients';
import { getBucketName } from "@/app/lib/s3-storage";

export async function POST(req: Request) {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const session = await resolveSession();
    const userId = session?.userId;
    if (!userId) return NextResponse.json({ error: 'Non autorisé' }, { status: 401 });
    const body = await req.json();
    if (!body.filename || !body.contentType) {
      return NextResponse.json({ error: 'filename et contentType requis' }, { status: 400 });
    }
    const key = `uploads-temp/${Date.now()}_${body.filename}`;
    const s3 = await getTenantDataS3Client();
    const command = new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: key,
      ContentType: body.contentType,
    });
    const url = await getSignedUrl(s3, command, { expiresIn: 3600 });
    return NextResponse.json({ url, key });
  } catch (error) {
    console.error('Erreur génération URL signée S3:', error);
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
