import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import type { BulletinSnapshot } from "@/app/lib/notes-bulletins-db";

export async function renderBulletinPdfBuffer(snapshot: BulletinSnapshot): Promise<Buffer> {
  const letterhead = await getSchoolLetterhead();
  const logo = await loadSchoolLogoForPdf();
  const doc = new jsPDF({ compress: true });
  drawPdfLetterhead(doc, letterhead, logo);

  let y = 48;
  doc.setFontSize(14);
  doc.setFont("helvetica", "bold");
  doc.text("Bulletin scolaire", 14, y);

  y += 8;
  doc.setFontSize(10);
  doc.setFont("helvetica", "normal");
  doc.text(`${snapshot.anneeLabel} — ${snapshot.periode.libelle}`, 14, y);

  y += 10;
  doc.setFont("helvetica", "bold");
  doc.text(`${snapshot.eleve.prenom} ${snapshot.eleve.nom}`, 14, y);
  doc.setFont("helvetica", "normal");
  y += 5;
  if (snapshot.eleve.classe) {
    doc.text(`Classe : ${snapshot.eleve.classe}`, 14, y);
    y += 5;
  }
  if (snapshot.eleve.ine) {
    doc.text(`INE : ${snapshot.eleve.ine}`, 14, y);
    y += 5;
  }

  autoTable(doc, {
    startY: y + 4,
    head: [["Matière", "Coef.", "Moyenne", "Notes", "Enseignant"]],
    body: snapshot.lignes.map((l) => [
      `${l.matiereCode} — ${l.matiereLibelle}`,
      l.coef,
      l.moyenne ?? "—",
      String(l.nbNotes),
      l.enseignantNom || "—",
    ]),
    styles: { fontSize: 8 },
    headStyles: { fillColor: [30, 41, 59] },
    columnStyles: {
      1: { halign: "center", cellWidth: 14 },
      2: { halign: "center", cellWidth: 18 },
      3: { halign: "center", cellWidth: 14 },
    },
  });

  let finalY = (doc as jsPDF & { lastAutoTable?: { finalY: number } }).lastAutoTable?.finalY ?? y + 40;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(11);
  doc.text(
    `Moyenne générale : ${snapshot.moyenneGenerale ?? "—"}`,
    14,
    finalY + 12,
  );

  if (snapshot.competences.length > 0) {
    autoTable(doc, {
      startY: finalY + 22,
      head: [["Domaine", "Compétence", "Maîtrise"]],
      body: snapshot.competences.map((c) => [
        c.domaineLibelle,
        c.itemLibelle,
        `${c.niveau ?? "—"} — ${c.niveauLabel}`,
      ]),
      styles: { fontSize: 8 },
      headStyles: { fillColor: [15, 76, 117] },
    });
    finalY = (doc as jsPDF & { lastAutoTable?: { finalY: number } }).lastAutoTable?.finalY ?? finalY + 40;
  }

  doc.setFont("helvetica", "normal");
  doc.setFontSize(8);
  doc.text(
    `Période ${snapshot.periode.statut === "cloturee" ? "clôturée" : "en cours"} — document généré par ScolIA.`,
    14,
    finalY + 10,
  );

  drawPdfFooter(doc, letterhead);
  return Buffer.from(doc.output("arraybuffer"));
}

export function bulletinPdfFilename(snapshot: BulletinSnapshot): string {
  const slug = `${snapshot.eleve.nom}-${snapshot.eleve.prenom}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase();
  return `bulletin-${snapshot.periode.code}-${slug || snapshot.eleve.id}.pdf`;
}
