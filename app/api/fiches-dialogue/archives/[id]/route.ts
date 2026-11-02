import { GetObjectCommand } from "@aws-sdk/client-s3";
import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { fdPdfArchive } from "@/db/schema";
import { requireAppUser } from "@/app/lib/app-session";
import { canConseilFichesDialogue } from "@/app/lib/fiches-dialogue-access";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { requireTenantId } from "@/app/lib/tenant-scope";

type Ctx = { params: Promise<{ id: string }> };

/**
 * Téléchargement d’une archive PDF de fiche de dialogue.
 * Sert le repli base64 quand S3 n’était pas disponible à l’archivage
 * (aucune perte silencieuse du document).
 */
export async function GET(_req: Request, ctx: Ctx) {
  const scope = await requireTenantId();
  if (!scope.ok) return scope.response;
  const appUser = await requireAppUser();
  if (!appUser.ok) return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  if (!canConseilFichesDialogue(appUser.user.roles, { orgAdmin: appUser.user.orgAdmin })) {
    return NextResponse.json({ error: "Accès refusé." }, { status: 403 });
  }

  const { id } = await ctx.params;
  const db = getDb();
  const [archive] = await db
    .select()
    .from(fdPdfArchive)
    .where(
      and(
        eq(fdPdfArchive.etablissementId, scope.ctx.etablissementId),
        eq(fdPdfArchive.id, id),
      ),
    )
    .limit(1);
  if (!archive) return NextResponse.json({ error: "Document introuvable." }, { status: 404 });

  let bytes: Uint8Array | null = null;
  if (archive.pdfBase64) {
    bytes = new Uint8Array(Buffer.from(archive.pdfBase64, "base64"));
  } else if (archive.s3Key) {
    try {
      const s3 = await getTenantDataS3Client();
      const obj = await s3.send(
        new GetObjectCommand({ Bucket: await getBucketName(), Key: archive.s3Key }),
      );
      bytes = (await obj.Body?.transformToByteArray()) ?? null;
    } catch {
      bytes = null;
    }
  }
  if (!bytes?.length) {
    return NextResponse.json({ error: "PDF introuvable." }, { status: 404 });
  }

  const filename = `fiche-dialogue-${archive.kind}.pdf`.replace(/\s+/g, "-");
  return new NextResponse(Buffer.from(bytes), {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="${filename}"`,
      "Cache-Control": "no-store, no-cache, must-revalidate, proxy-revalidate",
      Pragma: "no-cache",
      Expires: "0",
    },
  });
}
