import "server-only";

import { jsPDF } from "jspdf";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";

export type QuittancePdfInput = {
  numeroQuittance: string;
  dateEncaissement: string;
  mode: string;
  montant: string;
  reference: string | null;
  foyerLabel: string;
  factures: Array<{ numero: string; montant: string }>;
};

/** Quittance = preuve d’encaissement (pas de table dédiée). Générée à la volée. */
export async function renderQuittancePdfBuffer(input: QuittancePdfInput): Promise<Buffer> {
  const letterhead = await getSchoolLetterhead();
  const logo = await loadSchoolLogoForPdf();
  const doc = new jsPDF({ compress: true });
  drawPdfLetterhead(doc, letterhead, logo);

  let y = 48;
  doc.setFontSize(14);
  doc.setFont("helvetica", "bold");
  doc.text("Quittance de paiement", 14, y);
  y += 10;
  doc.setFontSize(10);
  doc.setFont("helvetica", "normal");
  doc.text(`N° ${input.numeroQuittance}`, 14, y);
  y += 6;
  doc.text(`Date : ${input.dateEncaissement}`, 14, y);
  y += 6;
  doc.text(`Mode : ${input.mode}`, 14, y);
  y += 10;
  doc.setFont("helvetica", "bold");
  doc.text("Reçu de", 14, y);
  doc.setFont("helvetica", "normal");
  y += 6;
  doc.text(input.foyerLabel, 14, y);
  y += 12;
  doc.setFont("helvetica", "bold");
  doc.text(`Montant : ${input.montant} €`, 14, y);
  y += 10;
  doc.setFont("helvetica", "normal");
  doc.text("Au titre de :", 14, y);
  y += 6;
  for (const f of input.factures) {
    doc.text(`• Facture ${f.numero} — ${f.montant} €`, 18, y);
    y += 5;
  }
  if (input.reference) {
    y += 4;
    doc.setFontSize(9);
    doc.setTextColor(100);
    doc.text(`Réf. : ${input.reference}`, 14, y);
    doc.setTextColor(0);
  }

  drawPdfFooter(doc, letterhead);
  return Buffer.from(doc.output("arraybuffer"));
}
