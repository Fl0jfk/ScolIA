import { NextResponse } from "next/server";
import { GetObjectCommand, HeadObjectCommand } from "@aws-sdk/client-s3";
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
import { accompagnementDownloadFileName } from "@/app/lib/eleve-pap";
import { clientIpFromRequest } from "@/app/lib/memory-rate-limit";

type Ctx = { params: Promise<{ id: string; documentId: string }> };

const SIGNED_DOCUMENT_URL_TTL_SEC = 60;

function documentOpenRequiresBlockingAudit(doc: {
  tiroir: string;
  confidentialite: string;
}): boolean {
  if (doc.tiroir === "psychologue") return true;
  if (doc.confidentialite === "restreint" || doc.confidentialite === "sante") return true;
  if (doc.tiroir === "sante") return true;
  return false;
}

async function persistDocumentOpenAudit(input: {
  etablissementId: string;
  actorUserId: string;
  documentId: string;
  eleveId: string;
  title: string;
  actorIp: string;
  actorUserAgent: string | null;
  actorRoles: string[];
}): Promise<boolean> {
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      await recordEleveAccessAudit({
        etablissementId: input.etablissementId,
        actorUserId: input.actorUserId,
        resourceType: "document",
        resourceId: input.documentId,
        eleveId: input.eleveId,
        action: "open_file",
        metadata: { title: input.title },
        actorIp: input.actorIp,
        actorUserAgent: input.actorUserAgent,
        actorRoles: input.actorRoles,
      });
      return true;
    } catch (err) {
      if (attempt === 0) continue;
      console.error("[eleves/documents/file] audit", err);
      return false;
    }
  }
  return false;
}

function safeFileName(raw: string | null | undefined, fallback: string): string {
  const base = String(raw || fallback)
    .replace(/["\\]/g, "")
    .replace(/[^\w.\- ()àâäéèêëïîôùûüçÀÂÄÉÈÊËÏÎÔÙÛÜÇ]+/g, "_")
    .slice(0, 160);
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
    .select({ id: eleve.id, classe: eleve.classe, nom: eleve.nom, prenom: eleve.prenom })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  const profScoped = isProfesseurScopedDossierViewer({ roles, orgAdmin, platformAdmin });
  let assignedClassesForProf: string[] | undefined;
  if (profScoped) {
    assignedClassesForProf = await listAssignedClassesForTeacher(session.user.businessUserId);
    if (!teacherCanAccessEleveClasse(eleveRow.classe, assignedClassesForProf)) {
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
    eleveClasse: eleveRow.classe,
    assignedClasses: assignedClassesForProf,
  });
  if (!access.ok) {
    return NextResponse.json({ error: access.error }, { status: access.status });
  }

  try {
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const key = s3Key(access.s3Key);
    try {
      await s3Client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));
    } catch (headErr) {
      const name =
        headErr && typeof headErr === "object" && "name" in headErr
          ? String((headErr as { name?: string }).name || "")
          : "";
      const code =
        headErr && typeof headErr === "object" && "Code" in headErr
          ? String((headErr as { Code?: string }).Code || "")
          : "";
      if (name === "NotFound" || name === "NoSuchKey" || code === "NotFound" || code === "NoSuchKey") {
        console.warn("[eleves/documents/file] NoSuchKey", { bucket, key, documentId, eleveId });
        return NextResponse.json(
          {
            error:
              "Fichier introuvable sur le stockage. La pièce est référencée mais le PDF n’est plus accessible.",
            code: "S3_KEY_MISSING",
          },
          { status: 404 },
        );
      }
      throw headErr;
    }

    const accompagnementName = accompagnementDownloadFileName({
      title: access.doc.title,
      prenom: eleveRow.prenom,
      nom: eleveRow.nom,
    });
    const fileName = safeFileName(
      accompagnementName || access.doc.title,
      "document.pdf",
    );
    const contentType = access.doc.mimeType || "application/pdf";
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: key,
      ResponseContentType: contentType,
      ResponseContentDisposition: `inline; filename="${fileName}"`,
    });
    const auditOk = await persistDocumentOpenAudit({
      etablissementId: etabId,
      actorUserId: session.user.id,
      documentId,
      eleveId,
      title: access.doc.title,
      actorIp: clientIpFromRequest(req),
      actorUserAgent: req.headers.get("user-agent"),
      actorRoles: roles,
    });
    if (!auditOk && documentOpenRequiresBlockingAudit(access.doc)) {
      return NextResponse.json(
        {
          error: "Journal d’accès indisponible — ouverture refusée pour ce document.",
          code: "AUDIT_UNAVAILABLE",
        },
        { status: 503 },
      );
    }

    const signedUrl = await getSignedUrl(s3Client, command, {
      expiresIn: SIGNED_DOCUMENT_URL_TTL_SEC,
    });

    const wantJson = new URL(req.url).searchParams.get("format") === "json";
    if (wantJson) {
      return NextResponse.json({
        signedUrl,
        fileName,
        contentType,
        expiresIn: SIGNED_DOCUMENT_URL_TTL_SEC,
      });
    }

    return NextResponse.redirect(signedUrl, 302);
  } catch (error) {
    console.error("[eleves/documents/file]", error);
    return NextResponse.json({ error: "Impossible d'ouvrir le document." }, { status: 500 });
  }
}
