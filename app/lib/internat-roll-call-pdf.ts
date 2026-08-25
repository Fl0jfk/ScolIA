import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";

export type InternatRollCallPdfRow = {
  nom: string;
  prenom: string;
  classe: string;
  sexe: string;
  statut: string;
};

export function renderInternatRollCallPdfBuffer(input: {
  title: string;
  etablissementLabel: string;
  schoolName: string;
  dateLabel: string;
  periodLabel: string;
  validatedBy: string;
  generatedAt?: Date;
  rows: InternatRollCallPdfRow[];
  counts: {
    present: number;
    absent: number;
    excuse: number;
    activite: number;
  };
}): Buffer {
  const doc = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
  const when = (input.generatedAt || new Date()).toLocaleString("fr-FR");

  doc.setFontSize(14);
  doc.setFont("helvetica", "bold");
  doc.text(input.title, 14, 16);

  doc.setFontSize(10);
  doc.setFont("helvetica", "normal");
  doc.text(input.schoolName, 14, 23);
  doc.text(`${input.etablissementLabel} · ${input.periodLabel} du ${input.dateLabel}`, 14, 29);

  doc.setFontSize(9);
  doc.setTextColor(80);
  doc.text(`Validé par ${input.validatedBy} · Généré le ${when}`, 14, 35);
  doc.text(
    `Présents : ${input.counts.present} · Absents : ${input.counts.absent} · Excusés : ${input.counts.excuse} · Activité : ${input.counts.activite} · Total : ${input.rows.length}`,
    14,
    41,
  );
  doc.setTextColor(0);

  autoTable(doc, {
    startY: 46,
    head: [["Nom", "Prénom", "Classe", "Sexe", "Statut"]],
    body: input.rows.map((r) => [r.nom, r.prenom, r.classe || "—", r.sexe, r.statut]),
    styles: { fontSize: 8, cellPadding: 2 },
    headStyles: { fillColor: [30, 64, 175], textColor: 255 },
    alternateRowStyles: { fillColor: [248, 250, 252] },
    columnStyles: {
      0: { cellWidth: 42 },
      1: { cellWidth: 42 },
      2: { cellWidth: 28 },
      3: { cellWidth: 18 },
      4: { cellWidth: 40 },
    },
  });

  const pageCount = doc.getNumberOfPages();
  for (let i = 1; i <= pageCount; i++) {
    doc.setPage(i);
    doc.setFontSize(8);
    doc.setTextColor(120);
    doc.text(`Page ${i} / ${pageCount}`, 14, 287);
    doc.text("Appel internat — document confidentiel", 105, 287, { align: "center" });
  }

  return Buffer.from(doc.output("arraybuffer"));
}
