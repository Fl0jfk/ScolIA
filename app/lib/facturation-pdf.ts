import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";

export type FacturePdfInput = {
  facture: {
    numero: string;
    statut: string;
    dateEmission: string | null;
    dateEcheance: string | null;
    totalHt: string;
    totalTtc: string;
  };
  lignes: Array<{
    libelle: string;
    quantite: string;
    prixUnitaire: string;
    remise: string;
    totalTtc: string;
    periode: string | null;
  }>;
  foyer: {
    label: string;
    adresse: string | null;
    codePostal: string | null;
    ville: string | null;
    payeurNom?: string | null;
  };
};

export async function renderFacturePdfBuffer(input: FacturePdfInput): Promise<Buffer> {
  const letterhead = await getSchoolLetterhead();
  const logo = await loadSchoolLogoForPdf();
  const doc = new jsPDF({ compress: true });
  drawPdfLetterhead(doc, letterhead, logo);

  let y = 48;
  doc.setFontSize(11);
  doc.setFont("helvetica", "bold");
  doc.text(`Facture n° ${input.facture.numero}`, 14, y);
  y += 6;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(9);
  doc.text(`Statut : ${input.facture.statut}`, 14, y);
  if (input.facture.dateEmission) {
    y += 5;
    doc.text(`Date d'émission : ${input.facture.dateEmission}`, 14, y);
  }
  if (input.facture.dateEcheance) {
    y += 5;
    doc.text(`Échéance : ${input.facture.dateEcheance}`, 14, y);
  }

  y += 10;
  doc.setFont("helvetica", "bold");
  doc.text("Facturé à", 14, y);
  doc.setFont("helvetica", "normal");
  y += 5;
  doc.text(input.foyer.label, 14, y);
  if (input.foyer.payeurNom) {
    y += 5;
    doc.text(input.foyer.payeurNom, 14, y);
  }
  const addr = [input.foyer.adresse, [input.foyer.codePostal, input.foyer.ville].filter(Boolean).join(" ")]
    .filter(Boolean)
    .join(", ");
  if (addr) {
    y += 5;
    doc.text(addr, 14, y);
  }

  autoTable(doc, {
    startY: y + 8,
    head: [["Libellé", "Période", "Qté", "P.U.", "Remise", "Total TTC"]],
    body: input.lignes.map((l) => [
      l.libelle,
      l.periode || "—",
      String(l.quantite),
      `${l.prixUnitaire} €`,
      `${l.remise} €`,
      `${l.totalTtc} €`,
    ]),
    styles: { fontSize: 8 },
    headStyles: { fillColor: [30, 41, 59] },
  });

  const finalY = (doc as jsPDF & { lastAutoTable?: { finalY: number } }).lastAutoTable?.finalY ?? y + 40;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(10);
  doc.text(`Total HT : ${input.facture.totalHt} €`, 140, finalY + 10);
  doc.text(`Total TTC : ${input.facture.totalTtc} €`, 140, finalY + 16);

  drawPdfFooter(doc, letterhead);
  return Buffer.from(doc.output("arraybuffer"));
}
