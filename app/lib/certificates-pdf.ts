import "server-only";

import { jsPDF } from "jspdf";
import QRCode from "qrcode";
import { loadAppConfig } from "@/app/lib/app-config";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  fitImageInBox,
  getSchoolLetterhead,
  loadImageForPdfFromRef,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { loadCertificateProfSignatureBytes } from "@/app/lib/certificates-signature-store";
import { shortContentHash } from "@/app/lib/certificates-verify";
import {
  CERTIFICATE_SECTEUR_LABELS,
  type StudentAward,
} from "@/app/lib/certificates-types";
import { resolveDirectionSignatureImageUrl } from "@/app/lib/stage-config";

const SLATE: [number, number, number] = [30, 41, 59];
const SLATE_BODY: [number, number, number] = [71, 85, 105];
const SLATE_LIGHT: [number, number, number] = [148, 163, 184];
const BORDER: [number, number, number] = [226, 232, 240];
const ACCENT: [number, number, number] = [79, 70, 229];
const FOOTER_H = 14;
function formatDateLong(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

async function loadProfSigDataUri(externalUserId: string): Promise<string | null> {
  const bytes = await loadCertificateProfSignatureBytes(externalUserId);
  if (!bytes?.length) return null;
  return `data:image/png;base64,${Buffer.from(bytes).toString("base64")}`;
}

export async function generateCertificatePdf(
  award: StudentAward,
  verifyUrl: string,
): Promise<Uint8Array> {
  const [letterhead, logo] = await Promise.all([getSchoolLetterhead(), loadSchoolLogoForPdf()]);
  const doc = new jsPDF({ unit: "mm", format: "a4", compress: true });
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();
  const ML = 22;
  const MR = W - 22;
  const contentW = MR - ML;

  drawPdfLetterhead(doc, letterhead, logo, ACCENT);

  let y = 48;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(9);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("CERTIFICAT OFFICIEL", W / 2, y, { align: "center" });
  y += 6;

  doc.setDrawColor(...BORDER);
  doc.setLineWidth(0.3);
  doc.line(ML + 20, y, W / 2 - 22, y);
  doc.line(W / 2 + 22, y, MR - 20, y);
  y += 8;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(20);
  doc.setTextColor(...ACCENT);
  const titleLines = doc.splitTextToSize(award.programTitle, contentW);
  doc.text(titleLines, W / 2, y, { align: "center" });
  y += titleLines.length * 8 + 4;

  doc.setFont("helvetica", "normal");
  doc.setFontSize(11);
  doc.setTextColor(...SLATE);
  doc.text("Ce certificat atteste officiellement la participation et les réalisations de", W / 2, y, {
    align: "center",
  });
  y += 7;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(14);
  doc.setTextColor(...SLATE);
  doc.text(`${award.student.prenom} ${award.student.nom.toUpperCase()}`, W / 2, y, { align: "center" });
  y += 6;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(10);
  doc.setTextColor(...SLATE_BODY);
  doc.text(`${award.student.classe || "—"} · Année scolaire ${award.schoolYear}`, W / 2, y, { align: "center" });
  y += 12;

  doc.setDrawColor(...BORDER);
  doc.setLineWidth(0.3);
  doc.line(ML, y, MR, y);
  y += 10;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(10);
  doc.setTextColor(...SLATE);
  doc.text("Parcours et réalisations", ML, y);
  y += 8;

  doc.setFont("helvetica", "normal");
  doc.setFontSize(10);
  doc.setTextColor(...SLATE_BODY);
  for (let i = 0; i < award.lines.length; i++) {
    const line = award.lines[i];
    const periodSuffix = line.period ? ` (${line.period})` : "";
    const heading = `${i + 1}. ${line.title}${periodSuffix}`;
    doc.setFont("helvetica", "bold");
    doc.setFontSize(10);
    doc.setTextColor(...SLATE);
    const headingWrapped = doc.splitTextToSize(heading, contentW - 6);
    if (y + headingWrapped.length * 5 + 12 > H - 80) {
      doc.addPage();
      y = 24;
    }
    doc.text(headingWrapped, ML + 4, y);
    y += headingWrapped.length * 5 + 2;

    doc.setFont("helvetica", "normal");
    doc.setFontSize(10);
    doc.setTextColor(...SLATE_BODY);
    const descWrapped = doc.splitTextToSize(line.description, contentW - 10);
    doc.text(descWrapped, ML + 8, y);
    y += descWrapped.length * 5 + 5;
  }

  if (!award.lines.length) {
    doc.setFont("helvetica", "italic");
    doc.text("—", ML + 4, y);
    y += 8;
  }

  y += 6;
  if (y > H - 75) {
    doc.addPage();
    y = 24;
  }

  doc.setFont("helvetica", "bold");
  doc.setFontSize(10);
  doc.setTextColor(...SLATE);
  doc.text("Signatures", ML, y);
  y += 7;

  const signatureCards: Array<{
    kind: "prof" | "direction";
    role: string;
    name: string;
    date?: string;
    imageDataUri?: string;
    imageFormat?: "PNG" | "JPEG";
    imageWidth?: number;
    imageHeight?: number;
  }> = [];

  const signedProfs = award.designatedSignatories.filter((s) => s.status === "signed");
  for (const prof of signedProfs) {
    const sigData = await loadProfSigDataUri(prof.externalUserId);
    signatureCards.push({
      kind: "prof",
      role: "Enseignant",
      name: prof.name,
      date: prof.signedAt ? formatDateLong(prof.signedAt) : undefined,
      imageDataUri: sigData || undefined,
      imageFormat: "PNG",
      imageWidth: 40,
      imageHeight: 12,
    });
  }

  if (award.directionSignature) {
    const dirUrl = await resolveDirectionSignatureImageUrl(award.directionSignature.level);
    const dirImg = dirUrl ? await loadImageForPdfFromRef(dirUrl) : null;
    signatureCards.push({
      kind: "direction",
      role: `Direction (${CERTIFICATE_SECTEUR_LABELS[award.directionSignature.level]})`,
      name: award.directionSignature.signedByName,
      date: formatDateLong(award.directionSignature.signedAt),
      imageDataUri: dirImg?.dataUri,
      imageFormat: dirImg?.format,
      imageWidth: dirImg?.width,
      imageHeight: dirImg?.height,
    });
  }

  const cardW = 54;
  const cardH = 36;
  const gapX = 5;
  const gapY = 5;
  const cols = Math.max(1, Math.floor((contentW + gapX) / (cardW + gapX)));

  for (let i = 0; i < signatureCards.length; i++) {
    const col = i % cols;
    if (i > 0 && col === 0) y += cardH + gapY;
    if (y + cardH > H - 55) {
      doc.addPage();
      y = 24;
    }
    const x = ML + col * (cardW + gapX);
    const card = signatureCards[i];

    doc.setFont("helvetica", "bold");
    doc.setFontSize(7);
    doc.setTextColor(...SLATE_LIGHT);
    doc.text(card.role, x + 2, y + 4.5);

    doc.setFont("helvetica", "bold");
    doc.setFontSize(8);
    doc.setTextColor(...SLATE);
    const nameLines = doc.splitTextToSize(card.name, cardW - 4).slice(0, 2);
    doc.text(nameLines, x + 2, y + 9.5);

    doc.setFont("helvetica", "normal");
    doc.setFontSize(7);
    doc.setTextColor(...SLATE_LIGHT);
    if (card.date) doc.text(card.date, x + 2, y + 17);

    const sigBoxX = x + 2;
    const sigBoxY = y + 19;
    const sigBoxW = cardW - 4;
    const sigBoxH = 14;
    if (card.imageDataUri && card.imageFormat) {
      const fittedBase = fitImageInBox(
        card.imageWidth || sigBoxW,
        card.imageHeight || sigBoxH,
        sigBoxW,
        sigBoxH,
      );
      const boost = card.kind === "direction" ? 1.15 : 1;
      const boostedW = Math.min(sigBoxW, fittedBase.width * boost);
      const boostedH = Math.min(sigBoxH, fittedBase.height * boost);
      const imgX = sigBoxX + (sigBoxW - boostedW) / 2;
      const imgY = sigBoxY + (sigBoxH - boostedH) / 2;
      doc.addImage(card.imageDataUri, card.imageFormat, imgX, imgY, boostedW, boostedH);
    } else {
      doc.setDrawColor(...BORDER);
      doc.rect(sigBoxX, sigBoxY, sigBoxW, sigBoxH);
    }
  }

  const qrSize = 28;
  const qrY = H - FOOTER_H - qrSize - 6;
  const qrX = MR - qrSize;
  try {
    const qrDataUrl = await QRCode.toDataURL(verifyUrl, { margin: 1, width: 200 });
    doc.addImage(qrDataUrl, "PNG", qrX, qrY, qrSize, qrSize);
  } catch {
    /* ignore */
  }

  doc.setFont("helvetica", "normal");
  doc.setFontSize(7);
  doc.setTextColor(...SLATE_LIGHT);
  doc.text("Vérifier l'authenticité", qrX + qrSize / 2, qrY + qrSize + 4, { align: "center" });
  doc.text(`Réf. ${shortContentHash(award.contentHash)}`, ML, H - FOOTER_H - 4);

  drawPdfFooter(doc, letterhead);

  return new Uint8Array(doc.output("arraybuffer"));
}
