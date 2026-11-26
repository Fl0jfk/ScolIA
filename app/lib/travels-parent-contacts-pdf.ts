import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import type { EleveContacts } from "@/app/lib/travels-parent-contacts";

export type ContactsSheetInput = {
  tripTitle: string;
  destination?: string;
  dateLabel?: string;
  editedAt: Date;
  editedBy: string;
  eleves: EleveContacts[];
};

const frDate = (d: Date) =>
  d.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short", timeZone: "Europe/Paris" });

/** Fiche contacts à emporter (usage hors réseau). Générée à la demande, jamais stockée. */
export function buildParentContactsPdf(input: ContactsSheetInput): Uint8Array {
  const doc = new jsPDF({ unit: "mm", format: "a4" });
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();

  doc.setFont("helvetica", "bold");
  doc.setFontSize(10);
  doc.setTextColor(180, 20, 20);
  doc.text("CONFIDENTIEL - usage pendant le voyage uniquement", W / 2, 10, { align: "center" });
  doc.setTextColor(20, 20, 20);
  doc.setFontSize(15);
  doc.text("Contacts des parents", 14, 20);
  doc.setFontSize(11);
  doc.text(input.tripTitle || "Voyage", 14, 27);
  doc.setFont("helvetica", "normal");
  doc.setFontSize(9);
  const sub = [input.destination, input.dateLabel].filter(Boolean).join(" - ");
  if (sub) doc.text(sub, 14, 32);
  doc.text(`Édité le ${frDate(input.editedAt)} par ${input.editedBy}`, 14, sub ? 37 : 32);

  const body: string[][] = [];
  for (const e of input.eleves) {
    const name = `${e.prenom} ${e.nom.toUpperCase()}${e.classe ? ` (${e.classe})` : ""}`;
    const lines: string[] = [];
    for (const g of e.groups) {
      if (e.groups.length > 1 || g.source === "fiche") lines.push(`[${g.titre}]`);
      for (const r of g.responsables) {
        const who = `${r.prenom} ${r.nom.toUpperCase()}`.trim();
        const tags = [r.lien, r.autoriteParentale ? "autorité parentale" : null].filter(Boolean).join(", ");
        lines.push(`${who}${tags ? ` (${tags})` : ""} : ${r.telephone ?? "-"}${r.email ? `  ${r.email}` : ""}`);
      }
    }
    body.push([name, lines.length ? lines.join("\n") : "Aucun contact enregistré"]);
  }

  autoTable(doc, {
    startY: sub ? 42 : 37,
    head: [["Élève", "Responsables et téléphones"]],
    body,
    styles: { fontSize: 9, cellPadding: 2, overflow: "linebreak" },
    headStyles: { fillColor: [30, 41, 59] },
    columnStyles: { 0: { cellWidth: 50, fontStyle: "bold" } },
    margin: { left: 14, right: 14, bottom: 16 },
  });

  const pages = doc.getNumberOfPages();
  for (let p = 1; p <= pages; p++) {
    doc.setPage(p);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(8);
    doc.setTextColor(180, 20, 20);
    doc.text("CONFIDENTIEL - Document à détruire au retour", W / 2, H - 8, { align: "center" });
    doc.setTextColor(110, 110, 110);
    doc.setFont("helvetica", "normal");
    doc.text(`${p}/${pages}`, W - 14, H - 8, { align: "right" });
  }
  return new Uint8Array(doc.output("arraybuffer"));
}
