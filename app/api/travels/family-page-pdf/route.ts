import { NextResponse } from "next/server";
import { jsPDF } from "jspdf";
import { getJson } from "@/app/lib/s3-storage";
import { drawPdfLetterhead, getSchoolLetterhead, loadSchoolLogoForPdf } from "@/app/lib/pdf-branding";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import { buildFamilyPagePreviewModel, FAMILY_PAGE_PROJECT_BANNER } from "@/app/lib/travels-family-page";
import { isWorkflowV2 } from "@/app/lib/travels-v2";
import type { TravelsFamilyPage, TravelsTrip } from "@/app/lib/travels-types";

/**
 * Export PDF de la page des familles (circuit v2). Tant que la page n'est pas ouverte,
 * chaque page du PDF porte le filigrane « PROJET — non validé, ne pas diffuser ».
 * Le contenu vient du dossier stocké ; le client peut seulement fournir le texte / les pièces / la date limite
 * en cours de rédaction (rendu uniquement, rien n'est enregistré).
 */
export async function POST(req: Request) {
  try {
    const body = (await req.json().catch(() => ({}))) as {
      tripId?: string;
      familyPage?: Partial<TravelsFamilyPage>;
    };
    if (!body.tripId || typeof body.tripId !== "string") {
      return NextResponse.json({ error: "Identifiant du séjour manquant." }, { status: 400 });
    }
    const hit = await getJson<TravelsTrip>(`travels/${body.tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Séjour introuvable." }, { status: 404 });
    if (!isWorkflowV2(trip)) {
      return NextResponse.json({ error: "Ce séjour n'utilise pas le circuit des familles." }, { status: 400 });
    }
    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    // Une page ouverte n'accepte plus de texte venant du client : on rend la version stockée.
    const stored = trip.data.familyPage;
    const override = stored?.openedAt
      ? undefined
      : body.familyPage
        ? {
            text: typeof body.familyPage.text === "string" ? body.familyPage.text : undefined,
            deadline: typeof body.familyPage.deadline === "string" ? body.familyPage.deadline : undefined,
            documents: Array.isArray(body.familyPage.documents) ? body.familyPage.documents : undefined,
          }
        : undefined;
    const cleanOverride = override
      ? Object.fromEntries(Object.entries(override).filter(([, v]) => v !== undefined))
      : undefined;
    const model = buildFamilyPagePreviewModel(trip, cleanOverride ? { familyPage: cleanOverride } : undefined);

    const [logo, letterhead] = await Promise.all([loadSchoolLogoForPdf(), getSchoolLetterhead()]);
    const doc = new jsPDF({ compress: true });
    const W = doc.internal.pageSize.getWidth();
    const H = doc.internal.pageSize.getHeight();
    const ML = 15;
    const CW = W - 30;
    const HEADER_H = 38;
    const BOTTOM = H - 18;
    let y = HEADER_H + 2;
    drawPdfLetterhead(doc, letterhead, logo, [37, 99, 235]);

    const ensureSpace = (needed: number) => {
      if (y + needed > BOTTOM) {
        doc.addPage();
        drawPdfLetterhead(doc, letterhead, logo, [37, 99, 235]);
        y = HEADER_H + 10;
      }
    };
    const writeLines = (text: string, size: number, opts?: { bold?: boolean; color?: [number, number, number]; gap?: number }) => {
      doc.setFont("helvetica", opts?.bold ? "bold" : "normal");
      doc.setFontSize(size);
      const c = opts?.color ?? [55, 65, 81];
      doc.setTextColor(c[0], c[1], c[2]);
      const lh = size * 0.5;
      for (const line of doc.splitTextToSize(text, CW) as string[]) {
        ensureSpace(lh + 1);
        doc.text(String(line), ML, y);
        y += lh;
      }
      y += opts?.gap ?? 2;
    };

    // Image du voyage
    if (model.imageUrl) {
      try {
        const { loadTravelCoverImageBytes } = await import("@/app/lib/travels-cover-cdn");
        const loaded = await loadTravelCoverImageBytes(model.imageUrl);
        if (loaded) {
          const mime = loaded.contentType.includes("png") ? "PNG" : loaded.contentType.includes("webp") ? "WEBP" : "JPEG";
          doc.addImage(`data:${loaded.contentType};base64,${loaded.bytes.toString("base64")}`, mime, 0, y, W, 55);
          y += 59;
        }
      } catch (e) {
        console.error("[family-page-pdf] image", e);
      }
    }

    writeLines(model.title.toUpperCase(), 15, { bold: true, color: [30, 41, 59], gap: 1 });
    if (model.destination) writeLines(model.destination, 10, { color: [100, 116, 139], gap: 3 });
    writeLines(model.dateLine, 11, { bold: true, color: [37, 99, 235], gap: 2 });
    for (const m of model.meetingLines) writeLines(m, 9.5, { gap: 1 });
    y += 3;

    if (model.paragraphs.length > 0) {
      for (const p of model.paragraphs) writeLines(p, 10, { gap: 3 });
    } else {
      writeLines("(Le texte de la page n'est pas encore rédigé.)", 10, { color: [148, 163, 184] });
    }

    if (model.documents.length > 0) {
      y += 2;
      writeLines("Pièces à fournir", 11, { bold: true, color: [37, 99, 235], gap: 1 });
      for (const d of model.documents) {
        writeLines(
          `• ${d.label} — ${d.kindLabel}${d.dueDate ? ` (au plus tard le ${d.dueDate.slice(8, 10)}/${d.dueDate.slice(5, 7)}/${d.dueDate.slice(0, 4)})` : ""}`,
          9.5,
          { gap: 1 },
        );
      }
    }

    if (model.price) {
      y += 3;
      writeLines("Participation financière", 11, { bold: true, color: [37, 99, 235], gap: 1 });
      writeLines(`Prix par élève : ${model.price.pricePerStudent.toFixed(2).replace(".", ",")} €`, 10, { bold: true, gap: 1 });
      for (const i of model.price.installments) {
        writeLines(
          `• ${i.dueDate.slice(8, 10)}/${i.dueDate.slice(5, 7)}/${i.dueDate.slice(0, 4)} : ${i.amount.toFixed(2).replace(".", ",")} €`,
          9.5,
          { gap: 1 },
        );
      }
      if (model.price.paymentText) writeLines(model.price.paymentText, 9.5);
    } else if (model.isProject) {
      y += 3;
      writeLines("Participation financière : en attente de la saisie par la comptabilité.", 9.5, { color: [148, 163, 184] });
    }

    if (model.deadline) {
      y += 3;
      writeLines(
        `Date limite de réponse : ${model.deadline.slice(8, 10)}/${model.deadline.slice(5, 7)}/${model.deadline.slice(0, 4)}`,
        10.5,
        { bold: true, color: [180, 83, 9] },
      );
    }

    // Filigrane + bandeau « PROJET » sur chaque page tant que la page n'est pas ouverte.
    if (model.isProject) {
      const pages = doc.getNumberOfPages();
      for (let i = 1; i <= pages; i++) {
        doc.setPage(i);
        doc.saveGraphicsState();
        doc.setGState(new (doc as unknown as { GState: new (o: { opacity: number }) => unknown }).GState({ opacity: 0.16 }) as never);
        doc.setFont("helvetica", "bold");
        doc.setFontSize(48);
        doc.setTextColor(220, 38, 38);
        doc.text("PROJET", W / 2, H / 2 - 12, { align: "center", angle: 40 });
        doc.setFontSize(20);
        doc.text("NON VALIDÉ — NE PAS DIFFUSER", W / 2, H / 2 + 14, { align: "center", angle: 40 });
        doc.restoreGraphicsState();
        doc.setFillColor(220, 38, 38);
        doc.rect(0, 0, W, 7, "F");
        doc.setFont("helvetica", "bold");
        doc.setFontSize(8);
        doc.setTextColor(255, 255, 255);
        doc.text(FAMILY_PAGE_PROJECT_BANNER.toUpperCase(), W / 2, 4.8, { align: "center" });
      }
    }

    const pdf = doc.output("datauristring");
    return NextResponse.json({ pdf, filename: `Page_familles_${model.title.replace(/[^A-Za-z0-9]+/g, "_")}${model.isProject ? "_PROJET" : ""}.pdf` });
  } catch (error) {
    console.error("[family-page-pdf]", error);
    return NextResponse.json({ error: "Échec de l'export PDF." }, { status: 500 });
  }
}
