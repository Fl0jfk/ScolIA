import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { requireAuth } from "@/app/lib/intranet-auth";
import { getAppSession } from "@/app/lib/intranet-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  teacherCanAccessEleveClasse,
} from "@/app/lib/eleve-dossier-prof";
import { assertCanOpenEleveDocument } from "@/app/lib/eleve-document-file";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { and, eq } from "drizzle-orm";
import { recordEleveAccessAudit } from "@/app/lib/eleve-dossier-access";

type Ctx = { params: Promise<{ id: string; documentId: string }> };

function safeFileName(raw: string | null | undefined, fallback: string): string {
  const base = String(raw || fallback)
    .replace(/["\\]/g, "")
    .replace(/[^\w.\- ()àâäéèêëïîôùûüçÀÂÄÉÈÊËÏÎÔÙÛÜÇ]+/g, "_")
    .slice(0, 120);
  return base || fallback;
}

/**
 * Ouvre un document dossier élève via URL S3 pré-signée.
 * GET → redirect (liens / img) ; `?format=json` → `{ signedUrl }`.
 */
export async function GET(req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { id: eleveId, documentId } = await ctx.params;
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) {
    return NextResponse.json({ error: "Établissement introuvable." }, { status: 400 });
  }

  const session = await getAppSession();
  if (!session?.user) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }
  const roles =
    session.user.roles.length > 0
      ? session.user.roles
      : await listUserRolesFromDb(session.user.id, etabId);
  const orgAdmin = Boolean(session.user.orgAdmin);
  const platformAdmin = Boolean(session.user.platformAdmin);

  const db = getDb();
  const [eleveRow] = await db
    .select({ id: eleve.id, classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  if (isProfesseurScopedDossierViewer({ roles, orgAdmin, platformAdmin })) {
    const assignedClasses = await listAssignedClassesForTeacher(session.user.businessUserId);
    if (!teacherCanAccessEleveClasse(eleveRow.classe, assignedClasses)) {
      return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    }
  }

  const access = await assertCanOpenEleveDocument({
    etablissementId: etabId,
    eleveId,
    documentId,
    userId: session.user.id,
    roles,
    orgAdmin,
    platformAdmin,
  });
  if (!access.ok) {
    return NextResponse.json({ error: access.error }, { status: access.status });
  }

  try {
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const fileName = safeFileName(access.doc.title, "document.pdf");
    const contentType = access.doc.mimeType || "application/pdf";
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: s3Key(access.s3Key),
      ResponseContentType: contentType,
      ResponseContentDisposition: `inline; filename="${fileName}"`,
    });
    const signedUrl = await getSignedUrl(s3Client, command, { expiresIn: 900 });

    void recordEleveAccessAudit({
      etablissementId: etabId,
      actorUserId: session.user.id,
      resourceType: "document",
      resourceId: documentId,
      eleveId,
      action: "open_file",
      metadata: { title: access.doc.title },
    }).catch((err) => console.error("[eleves/documents/file] audit", err));

    const wantJson = new URL(req.url).searchParams.get("format") === "json";
    if (wantJson) {
      return NextResponse.json({
        signedUrl,
        fileName,
        contentType,
        expiresIn: 900,
      });
    }

    return NextResponse.redirect(signedUrl, 302);
  } catch (error) {
    console.error("[eleves/documents/file]", error);
    return NextResponse.json({ error: "Impossible d'ouvrir le document." }, { status: 500 });
  }
}
