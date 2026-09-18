import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { requireAuth } from "@/app/lib/intranet-auth";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { getAppSession } from "@/app/lib/intranet-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { canManageElevePreinscriptions } from "@/app/lib/eleve-dossier-scope";
import { ingestInscriptionDocument } from "@/app/lib/inscription-doc-ingest";
import { isSafeS3RelativeKey, keyHasAllowedPrefix } from "@/app/lib/s3-path";

export const maxDuration = 120;

type Ctx = { params: Promise<{ id: string }> };

/**
 * Après PUT S3 : OCR/IA pour identifier la nature de la pièce, titre =
 * « NOM Prénom — type », enregistrement tiroir inscription.
 * Pas de matching identité dans le PDF (élève déjà connu).
 */
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

  if (
    !canManageElevePreinscriptions({
      roles,
      orgAdmin: Boolean(session.user.orgAdmin),
      platformAdmin: Boolean(session.user.platformAdmin),
    })
  ) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 403 });
  }

  const db = getDb();
  const [row] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etabId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!row) {
    return NextResponse.json({ error: "Élève introuvable." }, { status: 404 });
  }

  const body = (await req.json().catch(() => null)) as {
    s3Key?: string;
    fileName?: string;
    mimeType?: string;
    fileUrl?: string | null;
  } | null;

  const s3Key = String(body?.s3Key || "").trim();
  const fileName = String(body?.fileName || "document").trim() || "document";
  const mimeType = String(body?.mimeType || "application/octet-stream").trim();

  if (!s3Key || !isSafeS3RelativeKey(s3Key)) {
    return NextResponse.json({ error: "Clé fichier invalide." }, { status: 400 });
  }
  if (!keyHasAllowedPrefix(s3Key, [`eleves-dossier/${eleveId}`])) {
    return NextResponse.json({ error: "Clé fichier hors dossier élève." }, { status: 400 });
  }

  try {
    const result = await ingestInscriptionDocument({
      etablissementId: etabId,
      eleveId,
      actorUserId: session.user.id,
      s3Key,
      fileName,
      mimeType,
      fileUrl: body?.fileUrl ?? null,
    });
    return NextResponse.json({
      success: true,
      document: {
        ...result.document,
        createdAt: result.document.createdAt
          ? result.document.createdAt.toISOString()
          : null,
      },
      classification: result.classification,
    });
  } catch (err) {
    console.error("[inscription-ingest]", err);
    return NextResponse.json(
      { error: err instanceof Error ? err.message : "Ingestion impossible." },
      { status: 500 },
    );
  }
}
