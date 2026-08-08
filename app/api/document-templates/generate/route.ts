import { NextResponse } from "next/server";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { requireAdmin } from "@/app/lib/intranet-auth";
import {
  documentTitle,
  generatedFileKey,
  getTemplateMeta,
  isDocumentOutputFormat,
  isDocumentTemplateId,
  mergeTemplateValues,
  renderDocumentTemplateDocx,
  renderDocumentTemplateFillablePdf,
  renderDocumentTemplatePdf,
  saveGeneratedDocument,
  type DocumentOutputFormat,
  type GeneratedDocument,
} from "@/app/lib/document-templates";
import { findEleveByIne } from "@/app/lib/eleves-registry";

export const runtime = "nodejs";

export async function POST(req: Request) {
  const gate = await requireAdmin();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const templateId = String(body.templateId || "");
    if (!isDocumentTemplateId(templateId)) {
      return NextResponse.json({ error: "Modèle inconnu" }, { status: 400 });
    }
    const formatRaw = String(body.format || "pdf");
    const format: DocumentOutputFormat = isDocumentOutputFormat(formatRaw) ? formatRaw : "pdf";

    const meta = getTemplateMeta(templateId)!;
    const eleveIne = body.eleveIne ? String(body.eleveIne).trim() : "";
    const eleve = eleveIne ? await findEleveByIne(eleveIne) : null;
    if (eleveIne && !eleve) {
      return NextResponse.json({ error: "Élève introuvable" }, { status: 404 });
    }

    const values = mergeTemplateValues(
      templateId,
      (body.values && typeof body.values === "object" ? body.values : {}) as Record<string, unknown>,
      eleve,
      // PDF à trous : champs peuvent rester vides pour dépôt ED / préinscription
      { skipRequired: format === "fillable-pdf" },
    );

    let bytes: Uint8Array | Buffer;
    if (format === "docx") {
      bytes = await renderDocumentTemplateDocx(templateId, values);
    } else if (format === "fillable-pdf") {
      bytes = await renderDocumentTemplateFillablePdf(templateId, values);
    } else {
      bytes = await renderDocumentTemplatePdf(templateId, values);
    }

    const user = await safeCurrentUser();
    const id = `doc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
    const now = new Date().toISOString();
    const fileKey = generatedFileKey(id, format);
    const doc: GeneratedDocument = {
      id,
      templateId,
      templateLabel: meta.label,
      createdAt: now,
      createdBy: {
        userId: user?.id || gate.ctx.userId,
        name: user?.fullName || undefined,
        email: user?.primaryEmailAddress?.emailAddress || undefined,
      },
      values,
      eleveIne: eleve?.ine || eleveIne || undefined,
      fileKey,
      pdfKey: format === "docx" ? undefined : fileKey,
      format,
      title: documentTitle(templateId, values),
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
        downloadUrl: `/api/document-templates/generated/${doc.id}/pdf`,
      },
    });
  } catch (e) {
    console.error("[document-templates/generate]", e);
    const msg = e instanceof Error ? e.message : "Génération impossible";
    return NextResponse.json({ error: msg }, { status: 400 });
  }
}
