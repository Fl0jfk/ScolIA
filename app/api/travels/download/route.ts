import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { isAllowedTravelsDownloadKey, resolveTravelsS3ObjectKey } from "@/app/lib/travels-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import { requireAuth } from "@/app/lib/intranet-auth";

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const { fileUrl, s3Key: explicitKey } = await req.json();
    if (!fileUrl && !explicitKey) {
      return NextResponse.json({ error: "URL ou clé S3 manquante." }, { status: 400 });
    }

    const fileUrlStr = String(fileUrl || "");
    const explicitKeyStr = explicitKey ? String(explicitKey) : null;
    const key = await resolveTravelsS3ObjectKey(fileUrlStr, explicitKeyStr);
    if (!key) {
      return NextResponse.json(
        {
          error:
            "Fichier introuvable sur le stockage (clé S3 non résolue). Ré-uploadez le document si le problème persiste.",
        },
        { status: 404 },
      );
    }
    if (!isAllowedTravelsDownloadKey(key)) {
      return NextResponse.json({ error: "Accès refusé à ce fichier." }, { status: 403 });
    }

    const s3Client = await getTenantDataS3Client();
    const command = new GetObjectCommand({ Bucket: await getTenantBucketName(), Key: key });
    const signedUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
    return NextResponse.json({ signedUrl });
  } catch (error) {
    console.error("Erreur signature S3:", error);
    return NextResponse.json({ error: "Impossible de générer le lien." }, { status: 500 });
  }
}
