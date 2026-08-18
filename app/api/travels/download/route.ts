import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { isAllowedTravelsDownloadKey, resolveTravelsS3ObjectLocation } from "@/app/lib/travels-s3";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
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
    const loc = await resolveTravelsS3ObjectLocation(fileUrlStr, explicitKeyStr);
    if (!loc) {
      return NextResponse.json(
        {
          error:
            "Fichier introuvable sur le stockage. Vérifiez le bucket Scaleway ou ré-uploadez le document.",
        },
        { status: 404 },
      );
    }
    if (!isAllowedTravelsDownloadKey(loc.key)) {
      return NextResponse.json({ error: "Accès refusé à ce fichier." }, { status: 403 });
    }

    const s3Client = await getTenantDataS3Client();
    const command = new GetObjectCommand({ Bucket: loc.bucket, Key: loc.key });
    const signedUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
    return NextResponse.json({ signedUrl });
  } catch (error) {
    console.error("Erreur signature S3:", error);
    return NextResponse.json({ error: "Impossible de générer le lien." }, { status: 500 });
  }
}
