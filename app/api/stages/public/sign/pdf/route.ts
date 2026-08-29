import { GetObjectCommand } from "@aws-sdk/client-s3";
import { NextResponse } from "next/server";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { getSignTokenRef, getStageConvention } from "@/app/lib/stage-storage";

export async function GET(req: Request) {
  try {
    const token = new URL(req.url).searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const ref = await getSignTokenRef(token);
    if (!ref) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    const convention = await getStageConvention(ref.conventionId);
    if (!convention?.uploadedPdf?.s3Key) {
      return NextResponse.json({ error: "PDF introuvable." }, { status: 404 });
    }

    const s3Client = await getTenantDataS3Client();
    const obj = await s3Client.send(
      new GetObjectCommand({
        Bucket: await getBucketName(),
        Key: convention.uploadedPdf.s3Key,
      }),
    );
    const bytes = await obj.Body?.transformToByteArray();
    if (!bytes) return NextResponse.json({ error: "Fichier vide." }, { status: 500 });

    const filename = convention.uploadedPdf.fileName || "convention.pdf";
    const download = new URL(req.url).searchParams.get("download") === "1";
    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `${download ? "attachment" : "inline"}; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
