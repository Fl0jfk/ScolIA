import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";

export type AbsencePdfRow = {
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  type: string;
  statut: string;
  justifie: boolean;
  motif: string | null;
};

function formatDateFr(iso: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString("fr-FR");
}

export function renderAbsencesCpePdfBuffer(input: {
  etablissementLabel?: string;
  generatedAt?: Date;
  rows: AbsencePdfRow[];
}): Buffer {
  const doc = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
  const title = "Absences élèves à traiter";
  const etab = input.etablissementLabel?.trim() || "Établissement";
  const when = (input.generatedAt || new Date()).toLocaleString("fr-FR");

  doc.setFontSize(14);
  doc.setFont("helvetica", "bold");
  doc.text(title, 14, 18);
  doc.setFontSize(10);
  doc.setFont("helvetica", "normal");
  doc.text(etab, 14, 26);
  doc.setFontSize(9);
  doc.setTextColor(100);
  doc.text(`Généré le ${when} · ${input.rows.length} ligne(s)`, 14, 32);
  doc.setTextColor(0);

  autoTable(doc, {
    startY: 38,
    head: [["Date", "Élève", "Classe", "Type", "Statut", "Motif"]],
    body: input.rows.map((r) => [
      formatDateFr(r.dateDebut),
      `${r.elevePrenom} ${r.eleveNom}`,
      r.eleveClasse || "—",
      r.type,
      r.justifie ? "Justifiée" : r.statut,
      (r.motif || "—").slice(0, 60),
    ]),
    styles: { fontSize: 8, cellPadding: 2 },
    headStyles: { fillColor: [67, 56, 202], textColor: 255 },
    alternateRowStyles: { fillColor: [248, 250, 252] },
  });

  const arrayBuffer = doc.output("arraybuffer");
  return Buffer.from(arrayBuffer);
}
