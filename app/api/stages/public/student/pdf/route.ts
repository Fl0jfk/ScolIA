import { GetObjectCommand } from "@aws-sdk/client-s3";
import { NextResponse } from "next/server";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import {
  buildFreshConventionPdfDownload,
  isScoliaGeneratedConventionPdf,
} from "@/app/lib/stage-pdf-store";
import { resolveConventionByStudentToken } from "@/app/lib/stage-workflow";
import { conventionAllSignaturesValidated } from "@/app/lib/stage-types";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const pdfLimiter = createMemoryRateLimiter({
  windowMs: 10 * 60 * 1000,
  max: 30,
});

/** PDF convention pour l'élève (jeton student) — uniquement une fois signée. */
export async function GET(req: Request) {
  try {
    if (!(await pdfLimiter.allow(clientIpFromRequest(req)))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez dans quelques minutes." },
        { status: 429 },
      );
    }

    const url = new URL(req.url);
    const token = url.searchParams.get("token")?.trim();
    if (!token) return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });

    const convention = await resolveConventionByStudentToken(token);
    if (!convention) return NextResponse.json({ error: "Lien invalide." }, { status: 404 });

    const fullySigned =
      convention.status === "signed" ||
      conventionAllSignaturesValidated(convention.signatures);    if (!fullySigned) {
      return NextResponse.json(
        { error: "La convention n'est pas encore entièrement signée." },
        { status: 403 },
      );
    }

    const download = url.searchParams.get("download") === "1";

    if (isScoliaGeneratedConventionPdf(convention)) {
      const { bytes, fileName } = await buildFreshConventionPdfDownload(convention);
      return new NextResponse(Buffer.from(bytes), {
        status: 200,
        headers: {
          "Content-Type": "application/pdf",
          "Content-Disposition": `${download ? "attachment" : "inline"}; filename="${fileName}"`,
          "Cache-Control": "no-store",
        },
      });
    }

    if (!convention.uploadedPdf?.s3Key) {
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
    return new NextResponse(Buffer.from(bytes), {
      status: 200,
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `${download ? "attachment" : "inline"}; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    console.error("[stages/public/student/pdf]", error);
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
