import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  documentTitle,
  generatedFileKey,
  getInscriptionLevelMeta,
  getTemplateMeta,
  inscriptionDocumentTitle,
  isDocumentOutputFormat,
  isDocumentTemplateId,
  isInscriptionLevelId,
  renderDocumentTemplateDocx,
  renderDocumentTemplateFillablePdf,
  renderInscriptionFillablePdf,
  saveGeneratedDocument,
  type DocumentOutputFormat,
  type GeneratedDocument,
  type InscriptionLevelId,
} from "@/app/lib/document-templates";

export const runtime = "nodejs";
export const maxDuration = 60;

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const templateId = String(body.templateId || "");
    if (!isDocumentTemplateId(templateId)) {
      return NextResponse.json({ error: "Modèle inconnu" }, { status: 400 });
    }

    const meta = getTemplateMeta(templateId)!;
    const isInscription = templateId === "fiche-inscription";

    let format: DocumentOutputFormat = "fillable-pdf";
    if (!isInscription) {
      const formatRaw = String(body.format || "fillable-pdf");
      if (!isDocumentOutputFormat(formatRaw)) {
        return NextResponse.json(
          { error: "Format invalide (docx ou fillable-pdf)" },
          { status: 400 },
        );
      }
      format = formatRaw;
      const allowed = meta.formats?.length ? meta.formats : (["fillable-pdf", "docx"] as const);
      if (!allowed.includes(format)) {
        return NextResponse.json(
          {
            error: `Format non disponible pour ce modèle (${allowed.map((f) => f).join(" / ")}).`,
          },
          { status: 400 },
        );
      }
    }

    let levelId: InscriptionLevelId | undefined;
    if (isInscription) {
      const rawLevel = String(body.inscriptionLevelId || body.levelId || "");
      if (!isInscriptionLevelId(rawLevel)) {
        return NextResponse.json(
          { error: "Choisissez un niveau (sixième, seconde…)" },
          { status: 400 },
        );
      }
      levelId = rawLevel;
      format = "fillable-pdf";
    }

    let bytes: Uint8Array | Buffer;
    if (isInscription && levelId) {
      bytes = await renderInscriptionFillablePdf({
        levelId,
        establishmentName:
          typeof body.establishmentName === "string"
            ? body.establishmentName
            : undefined,
        accentColor:
          typeof body.accentColor === "string" ? body.accentColor : undefined,
      });
    } else if (format === "docx") {
      bytes = await renderDocumentTemplateDocx(templateId);
    } else {
      bytes = await renderDocumentTemplateFillablePdf(templateId);
    }

    const user = await safeCurrentUser();
    const id = `doc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
    const now = new Date().toISOString();
    const fileKey = generatedFileKey(id, format);
    const levelMeta = levelId ? getInscriptionLevelMeta(levelId) : undefined;
    const title =
      isInscription && levelId
        ? inscriptionDocumentTitle(levelId)
        : documentTitle(templateId);

    const doc: GeneratedDocument = {
      id,
      templateId,
      templateLabel: levelMeta
        ? `${meta.label} — ${levelMeta.label}`
        : meta.label,
      createdAt: now,
      createdBy: {
        userId: user?.id || gate.ctx.userId,
        name: user?.fullName || undefined,
        email: user?.primaryEmailAddress?.emailAddress || undefined,
      },
      values: {},
      fileKey,
      pdfKey: format === "docx" ? undefined : fileKey,
      format,
      title,
      inscriptionLevelId: levelId,
    };

    await saveGeneratedDocument(doc, bytes);

    return NextResponse.json({
      success: true,
      document: {
        id: doc.id,
        title: doc.title,
        templateId: doc.templateId,
        templateLabel: doc.templateLabel,
        createdAt: doc.createdAt,
        format: doc.format,
        inscriptionLevelId: doc.inscriptionLevelId,
        downloadUrl: `/api/document-templates/generated/${doc.id}/pdf`,
      },
    });
  } catch (e) {
    console.error("[document-templates/generate]", e);
    const msg = e instanceof Error ? e.message : "Génération impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
