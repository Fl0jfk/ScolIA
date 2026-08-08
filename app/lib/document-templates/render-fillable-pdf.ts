import "server-only";

import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import { getSchoolLetterhead, loadSchoolLogoForPdf, fitImageInBox } from "@/app/lib/pdf-branding";
import { getTemplateMeta } from "@/app/lib/document-templates/catalog";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

function sanitize(input: string): string {
  return String(input || "")
    .replace(/[\u2018\u2019]/g, "'")
    .replace(/[\u201C\u201D]/g, '"')
    .replace(/\u2026/g, "...")
    .replace(/[^\x00-\xFF]/g, "?");
}

/**
 * PDF AcroForm vierge (champs vides) brandé établissement — dépôt ED / Adobe / impression.
 */
export async function renderDocumentTemplateFillablePdf(
  templateId: DocumentTemplateId,
): Promise<Uint8Array> {
  const meta = getTemplateMeta(templateId);
  if (!meta) throw new Error("Modèle inconnu");

  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  let page = doc.addPage([595.28, 841.89]);
  const form = doc.getForm();
  const margin = 48;
  const width = 595.28;
  let y = 800;

  const ensure = (needed: number) => {
    if (y - needed < 56) {
      page = doc.addPage([595.28, 841.89]);
      y = 800;
    }
  };

  const letterhead = await getSchoolLetterhead();
  const logo = await loadSchoolLogoForPdf();
  if (logo) {
    const b64 = logo.dataUri.split(",")[1];
    if (b64) {
      const bytes = Buffer.from(b64, "base64");
      const img =
        logo.format === "JPEG" ? await doc.embedJpg(bytes) : await doc.embedPng(bytes);
      const fitted = fitImageInBox(logo.width || 120, logo.height || 80, 90, 48);
      page.drawImage(img, {
        x: margin,
        y: y - fitted.height + 8,
        width: fitted.width,
        height: fitted.height,
      });
    }
  }
  const name = sanitize(letterhead.name);
  page.drawText(name, {
    x: width - margin - bold.widthOfTextAtSize(name, 12),
    y,
    size: 12,
    font: bold,
    color: rgb(0.12, 0.16, 0.22),
  });
  y -= 56;
  page.drawRectangle({
    x: 0,
    y,
    width,
    height: 2.5,
    color: rgb(0.12, 0.16, 0.22),
  });
  y -= 28;

  ensure(36);
  page.drawText(sanitize(meta.label.toUpperCase()), {
    x: margin,
    y,
    size: 14,
    font: bold,
    color: rgb(0.1, 0.12, 0.16),
  });
  y -= 14;
  page.drawText(sanitize("Formulaire vierge remplissable — a completer a l'ecran ou imprimer."), {
    x: margin,
    y,
    size: 8,
    font,
    color: rgb(0.4, 0.45, 0.52),
  });
  y -= 22;

  for (const field of meta.fields) {
    const label = sanitize(field.label);

    if (field.type === "checkbox") {
      ensure(28);
      page.drawText(label, {
        x: margin + 22,
        y: y + 2,
        size: 10,
        font,
        color: rgb(0.15, 0.18, 0.22),
      });
      const cb = form.createCheckBox(`f_${field.key}`);
      cb.addToPage(page, {
        x: margin,
        y: y - 2,
        width: 14,
        height: 14,
      });
      y -= 26;
      continue;
    }

    const isArea = field.type === "textarea";
    const boxH = isArea ? 54 : 22;
    ensure(18 + boxH + 10);
    page.drawText(label, {
      x: margin,
      y,
      size: 9,
      font: bold,
      color: rgb(0.25, 0.3, 0.38),
    });
    y -= 14;
    const tf = form.createTextField(`f_${field.key}`);
    tf.setFontSize(10);
    if (isArea) tf.enableMultiline();
    tf.addToPage(page, {
      x: margin,
      y: y - boxH + 8,
      width: width - margin * 2,
      height: boxH,
      borderWidth: 1,
      borderColor: rgb(0.75, 0.78, 0.82),
      backgroundColor: rgb(0.99, 0.99, 1),
    });
    y -= boxH + 12;
  }

  ensure(40);
  y -= 8;
  page.drawText(sanitize("Signature du responsable / famille"), {
    x: margin,
    y,
    size: 9,
    font: bold,
    color: rgb(0.25, 0.3, 0.38),
  });
  y -= 14;
  const sig = form.createTextField("f_signature");
  sig.setFontSize(10);
  sig.addToPage(page, {
    x: margin,
    y: y - 36,
    width: 220,
    height: 40,
    borderWidth: 1,
    borderColor: rgb(0.75, 0.78, 0.82),
  });

  page.drawText(sanitize(letterhead.footerLeft), {
    x: margin,
    y: 28,
    size: 7,
    font,
    color: rgb(0.55, 0.6, 0.65),
  });

  form.updateFieldAppearances(font);
  return doc.save({ updateFieldAppearances: true });
}
