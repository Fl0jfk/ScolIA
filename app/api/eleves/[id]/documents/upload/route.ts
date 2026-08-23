import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { eleveDossierSectionsForRoles } from "@/app/lib/eleve-dossier-access";
import {
  isProfesseurScopedDossierViewer,
  listAssignedClassesForTeacher,
  teacherCanAccessEleveClasse,
} from "@/app/lib/eleve-dossier-prof";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { sanitizeS3FileName, s3Key } from "@/app/lib/s3-path";
import { publicS3UrlForKey } from "@/app/lib/travels-s3";

type Ctx = { params: Promise<{ id: string }> };

const MAX_BYTES = 25 * 1024 * 1024;

/** Prépare un PUT S3 pour un document du dossier élève. */
export async function POST(req: Request, ctx: Ctx) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  const { id: eleveId } = await ctx.params;
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
  const sections = eleveDossierSectionsForRoles(roles, {
    orgAdmin: Boolean(session.user.orgAdmin),
    platformAdmin: Boolean(session.user.platformAdmin),
  });
  if (!sections.has("documents")) {
    return NextResponse.json({ error: "Section documents non autorisée." }, { status: 403 });
  }

  const db = getDb();
  const [row] = await db
    .select({ id: eleve.id, classe: eleve.classe })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!row) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  if (isProfesseurScopedDossierViewer({ roles, orgAdmin: session.user.orgAdmin, platformAdmin: session.user.platformAdmin })) {
    const assignedClasses = await listAssignedClassesForTeacher(session.user.businessUserId);
    if (!teacherCanAccessEleveClasse(row.classe, assignedClasses)) {
      return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
    }
  }

  const body = (await req.json()) as {
    fileName?: string;
    fileType?: string;
    size?: number;
  };
  const size = Number(body.size || 0);
  if (size > MAX_BYTES) {
    return NextResponse.json({ error: "Fichier trop volumineux (max 25 Mo)." }, { status: 413 });
  }

  const safeName = sanitizeS3FileName(String(body.fileName || "document")).replace(
    /[^a-zA-Z0-9._-]/g,
    "_",
  );
  const fileKey = s3Key(`eleves-dossier/${eleveId}/${Date.now()}-${safeName}`);
  const contentType = String(body.fileType || "application/octet-stream");

  try {
    const s3Client = await getTenantDataS3Client();
    const command = new PutObjectCommand({
      Bucket: await getBucketName(),
      Key: fileKey,
      ContentType: contentType,
    });
    const uploadUrl = await getSignedUrl(s3Client, command, { expiresIn: 3600 });
    return NextResponse.json({
      uploadUrl,
      s3Key: fileKey,
      fileUrl: await publicS3UrlForKey(fileKey),
    });
  } catch (error) {
    console.error("[eleves/documents/upload]", error);
    return NextResponse.json({ error: "Erreur préparation upload." }, { status: 500 });
  }
}
