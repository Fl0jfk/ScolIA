import { GetObjectCommand } from "@aws-sdk/client-s3";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { NextResponse } from "next/server";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { requireAuth } from "@/app/lib/intranet-auth";
import { canViewAllConventions, canViewReferentConventions } from "@/app/lib/stage-access";
import { conventionVisibleToUser } from "@/app/lib/stage-referent";
import {
  buildFreshConventionPdfDownload,
  isScoliaGeneratedConventionPdf,
} from "@/app/lib/stage-pdf-store";
import { getStageConvention } from "@/app/lib/stage-storage";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";

export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  try {
    const gate = await requireAuth();
    if (!gate.ok) return gate.response;

    const user = await safeCurrentUser();
    const roles = intranetRolesFromMetadata(user?.publicMetadata);
    if (!canViewAllConventions(roles) && !canViewReferentConventions(roles)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    const { id } = await ctx.params;
    const convention = await getStageConvention(id);
    if (!convention) {
      return NextResponse.json({ error: "Convention introuvable." }, { status: 404 });
    }

    const userEmail = user?.primaryEmailAddress?.emailAddress?.trim().toLowerCase() || "";
    if (!conventionVisibleToUser(convention, roles, userEmail, gate.ctx.userId)) {
      return NextResponse.json({ error: "Accès réservé." }, { status: 403 });
    }

    // Préconvention ScolIA : toujours régénérer (données à jour + cases de signature).
    if (isScoliaGeneratedConventionPdf(convention)) {
      const { bytes, fileName } = await buildFreshConventionPdfDownload(convention);
      return new NextResponse(Buffer.from(bytes), {
        status: 200,
        headers: {
          "Content-Type": "application/pdf",
          "Content-Disposition": `inline; filename="${fileName}"`,
          "Cache-Control": "no-store",
        },
      });
    }

    if (!convention.uploadedPdf?.s3Key) {
      if (convention.oneDriveFiling) {
        return NextResponse.json(
          {
            error:
              "PDF supprimé de S3 après dépôt OneDrive — consultez le dossier élève sur OneDrive.",
            oneDrive: convention.oneDriveFiling,
          },
          { status: 410 },
        );
      }
      return NextResponse.json({ error: "PDF déposé introuvable." }, { status: 404 });
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
        "Content-Disposition": `inline; filename="${filename}"`,
        "Cache-Control": "no-store",
      },
    });
  } catch (error) {
    return NextResponse.json({ error: String(error) }, { status: 500 });
  }
}
