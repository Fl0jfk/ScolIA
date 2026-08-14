import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import { loadAppConfig, getEstablishmentByLabel } from "@/app/lib/app-config";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  fitImageInBox,
  getSchoolLetterhead,
  loadImageForPdfFromRef,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { resolveDirectionSignatureImageUrl } from "@/app/lib/stage-config";

export type HseEtablissement = string;

export type HseAcceptanceRecord = {
  id: string;
  createdAt: string;
  decidedAt?: string;
  createdBy: { name: string; email: string };
  etablissement: HseEtablissement;
  resumeDemande: string;
  motif?: string;
  nombreHeures?: number;
  classe: string;
  details: string;
  directionNote?: string;
  decidedBy?: { name: string };
};

const SLATE: [number, number, number] = [30, 41, 59];
const SLATE_LIGHT: [number, number, number] = [148, 163, 184];
const SLATE_BODY: [number, number, number] = [71, 85, 105];
const EMERALD: [number, number, number] = [5, 150, 105];
const EMERALD_LIGHT: [number, number, number] = [236, 253, 245];
const BORDER: [number, number, number] = [226, 232, 240];

const FOOTER_H = 14;
/** Zone réservée en bas pour la signature (au-dessus du pied de page). */
const SIG_ZONE_H = 42;

function formatNombreHeures(h: number): string {
  const text = Number.isInteger(h) ? String(h) : h.toFixed(2).replace(/\.?0+$/, "").replace(".", ",");
  return `${text} h`;
}

function formatDateLong(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

function hseEtabToLevelHint(etab: HseEtablissement): string {
  return etab || "établissement";
}

function drawSignatureBlock(
  doc: jsPDF,
  opts: {
    MR: number;
    sigTop: number;
    sigImage: Awaited<ReturnType<typeof loadImageForPdfFromRef>>;
    directorName: string;
    etablissement: string;
    decidedBy?: string;
  },
) {
  const { MR, sigTop, sigImage, directorName, etablissement, decidedBy } = opts;
  const sigMaxW = 58;
  const sigMaxH = 22;
  const sigX = MR - sigMaxW;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("POUR LA DIRECTION", sigX + sigMaxW, sigTop, { align: "right" });

  const imgTop = sigTop + 4;
  if (sigImage) {
    const fitted = fitImageInBox(
      sigImage.width || sigMaxW,
      sigImage.height || sigMaxH,
      sigMaxW,
      sigMaxH,
    );
    const imgX = sigX + sigMaxW - fitted.width;
    doc.addImage(sigImage.dataUri, sigImage.format, imgX, imgTop, fitted.width, fitted.height);
  } else {
    doc.setDrawColor(...BORDER);
    doc.setLineWidth(0.35);
    doc.rect(sigX, imgTop, sigMaxW, sigMaxH);
    doc.setFont("helvetica", "italic");
    doc.setFontSize(7);
    doc.setTextColor(...SLATE_LIGHT);
    doc.text("Signature", sigX + sigMaxW / 2, imgTop + 12, { align: "center" });
  }

  let nameY = imgTop + sigMaxH + 5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(9.5);
  doc.setTextColor(...SLATE);
  doc.text(directorName, MR, nameY, { align: "right" });
  nameY += 5;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8);
  doc.setTextColor(...SLATE_BODY);
  doc.text(`Directrice / Directeur — ${etablissement}`, MR, nameY, { align: "right" });
  if (decidedBy && decidedBy !== directorName) {
    nameY += 4;
    doc.setFontSize(7);
    doc.setTextColor(...SLATE_LIGHT);
    doc.text(`Décision enregistrée par ${decidedBy}`, MR, nameY, { align: "right" });
  }
}

export async function buildHseAcceptancePdf(record: HseAcceptanceRecord): Promise<Uint8Array> {
  const [letterhead, logo, bundle] = await Promise.all([
    getSchoolLetterhead(),
    loadSchoolLogoForPdf(),
    loadAppConfig(),
  ]);

  const est = getEstablishmentByLabel(bundle, record.etablissement);
  const directorName = est?.directorName?.trim() || record.decidedBy?.name || "La direction";
  const decidedAt = record.decidedAt || new Date().toISOString();
  const createdAt = record.createdAt;
  const refShort = record.id.slice(-8).toUpperCase();

  const sigUrl = await resolveDirectionSignatureImageUrl(hseEtabToLevelHint(record.etablissement));
  const sigImage = sigUrl ? await loadImageForPdfFromRef(sigUrl) : null;

  const doc = new jsPDF({ compress: true });
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();
  const ML = 15;
  const MR = W - 15;
  const colB = W / 2 + 8;
  const sigTop = H - FOOTER_H - SIG_ZONE_H;

  drawPdfLetterhead(doc, letterhead, logo, EMERALD);

  let yA = 45;
  let yB = 45;

  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("DEMANDEUR", ML, yA);
  yA += 4.5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(10);
  doc.setTextColor(...SLATE);
  doc.text(record.createdBy.name, ML, yA);
  yA += 5;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8);
  doc.setTextColor(...SLATE_BODY);
  doc.text(record.createdBy.email, ML, yA);

  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("ÉTABLISSEMENT", colB, yB);
  yB += 4.5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(11);
  doc.setTextColor(...SLATE);
  doc.text(record.etablissement, colB, yB);
  yB += 7;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("DATE DE DÉCISION", colB, yB);
  yB += 4.5;
  doc.setFontSize(8.5);
  doc.setTextColor(...SLATE_BODY);
  doc.text(formatDateLong(decidedAt), colB, yB);
  yB += 6;
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("DÉPÔT INITIAL", colB, yB);
  yB += 4.5;
  doc.setFontSize(8);
  doc.setTextColor(...SLATE_BODY);
  doc.text(formatDateLong(createdAt), colB, yB);

  const sepY = Math.max(yA, yB) + 9;
  doc.setDrawColor(...BORDER);
  doc.setLineWidth(0.4);
  doc.line(ML, sepY, MR, sepY);

  let sy = sepY + 10;
  doc.setFillColor(...EMERALD);
  doc.rect(ML, sy - 5, 2.5, 13, "F");
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("OBJET", ML + 7, sy - 0.5);
  sy += 5.5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(14);
  doc.setTextColor(...SLATE);
  doc.text("Attestation d'acceptation — HSE", ML + 7, sy);
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text(`Réf. HSE-${refShort}`, MR, sy, { align: "right" });
  sy += 9;

  const badgeW = 52;
  doc.setFillColor(...EMERALD_LIGHT);
  doc.setDrawColor(...EMERALD);
  doc.setLineWidth(0.35);
  doc.roundedRect(ML, sy - 5.5, badgeW, 8, 1.5, 1.5, "FD");
  doc.setFont("helvetica", "bold");
  doc.setFontSize(7.5);
  doc.setTextColor(...EMERALD);
  doc.text("ACCEPTÉE PAR LA DIRECTION", ML + badgeW / 2, sy, { align: "center" });
  sy += 12;

  doc.setFont("helvetica", "italic");
  doc.setFontSize(8.5);
  doc.setTextColor(...SLATE_BODY);
  const intro =
    "La direction de l'établissement atteste avoir examiné la demande d'heures supplémentaires exceptionnelles (HSE) " +
    "présentée ci-dessous et l'accepte dans les conditions détaillées. Le présent document vaut attestation officielle " +
    "pour les services administratifs et le suivi RH.";
  const introLines = doc.splitTextToSize(intro, MR - ML);
  doc.text(introLines, ML, sy);
  sy += introLines.length * 4.5 + 9;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(6.5);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("DÉTAIL DE LA DEMANDE", ML, sy);
  sy += 3.5;

  const tableBody: string[][] = [
    ["Enseignant demandeur", record.createdBy.name],
    ["Adresse e-mail", record.createdBy.email],
    ["Établissement", record.etablissement],
    ["Classe / contexte pédagogique", record.classe || "—"],
  ];
  if (record.nombreHeures != null) {
    tableBody.push(["Nombre d'heures accordées", formatNombreHeures(record.nombreHeures)]);
  }
  tableBody.push(["Objet de la demande", record.resumeDemande || "—"]);
  if (record.motif?.trim() && record.motif !== record.resumeDemande) {
    tableBody.push(["Motif", record.motif]);
  }
  if (record.details?.trim()) {
    tableBody.push(["Précisions du demandeur", record.details]);
  }
  tableBody.push(["Date de dépôt", formatDateLong(createdAt)]);
  tableBody.push(["Date de décision", formatDateLong(decidedAt)]);

  autoTable(doc, {
    startY: sy,
    body: tableBody,
    theme: "plain",
    styles: {
      fontSize: 8.5,
      cellPadding: { top: 3.5, bottom: 3.5, left: 4, right: 4 },
      lineColor: BORDER,
      lineWidth: 0.2,
      overflow: "linebreak",
      valign: "top",
    },
    alternateRowStyles: { fillColor: [248, 250, 252] },
    columnStyles: {
      0: { cellWidth: 58, fontStyle: "bold", textColor: SLATE },
      1: { textColor: SLATE_BODY },
    },
    tableLineColor: BORDER,
    tableLineWidth: 0.2,
    margin: { left: ML, right: ML },
  });

  let afterY = (doc as jsPDF & { lastAutoTable: { finalY: number } }).lastAutoTable.finalY + 8;

  if (record.directionNote?.trim()) {
    const noteLines = doc.splitTextToSize(record.directionNote.trim(), MR - ML - 10);
    const boxH = noteLines.length * 4.2 + 14;
    if (afterY + boxH > sigTop - 4) {
      doc.addPage();
      drawPdfLetterhead(doc, letterhead, logo, EMERALD);
      afterY = 45;
    }
    doc.setFillColor(248, 250, 252);
    doc.setDrawColor(...BORDER);
    doc.setLineWidth(0.3);
    doc.roundedRect(ML, afterY, MR - ML, boxH, 2, 2, "FD");
    doc.setFont("helvetica", "bold");
    doc.setFontSize(6.5);
    doc.setTextColor(...SLATE_LIGHT);
    doc.text("MESSAGE DE LA DIRECTION", ML + 5, afterY + 7);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(8.5);
    doc.setTextColor(...SLATE_BODY);
    doc.text(noteLines, ML + 5, afterY + 13);
    afterY += boxH + 8;
  }

  const sigBlockH = 48;
  if (afterY + sigBlockH > H - FOOTER_H - 4) {
    doc.addPage();
    drawPdfLetterhead(doc, letterhead, logo, EMERALD);
    afterY = 45;
  }

  drawSignatureBlock(doc, {
    MR,
    sigTop: afterY,
    sigImage,
    directorName,
    etablissement: record.etablissement,
    decidedBy: record.decidedBy?.name,
  });

  const pageCount = doc.getNumberOfPages();
  for (let p = 1; p <= pageCount; p++) {
    doc.setPage(p);
    drawPdfFooter(doc, letterhead, `Réf. HSE-${refShort}`);
  }

  return new Uint8Array(doc.output("arraybuffer"));
}

export function hseAcceptancePdfFilename(record: HseAcceptanceRecord): string {
  const slug = record.createdBy.name
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/^_|_$/g, "")
    .slice(0, 40);
  const date = (record.decidedAt || record.createdAt).slice(0, 10);
  return `attestation_hse_${slug || "demande"}_${date}_${record.id.slice(-8)}.pdf`;
}
