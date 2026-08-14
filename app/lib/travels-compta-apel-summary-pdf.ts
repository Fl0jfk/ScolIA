import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import {
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import {
  pdfFormatDateFrLong,
  pdfFormatEuroAmount,
} from "@/app/lib/pdf-format-numbers";
import {
  type ComptaApelSummary,
  type ComptaApelTripCommitment,
} from "@/app/lib/travels-compta-apel-summary";

function euroPlain(n: number | null | undefined): string {
  return pdfFormatEuroAmount(n);
}

function pdfLastTableY(doc: jsPDF): number {
  return (doc as jsPDF & { lastAutoTable: { finalY: number } }).lastAutoTable.finalY;
}

function tripRowsBody(trips: ComptaApelTripCommitment[]): string[][] {
  if (trips.length === 0) return [["—", "—", "—", "—", "—"]];
  return trips.map((row) => [
    row.title,
    row.travelDateLabel,
    euroPlain(row.apelCollective),
    euroPlain(row.aidesIndividuelles),
    euroPlain(row.totalApel),
  ]);
}

export async function buildComptaApelSummaryPdfBase64(summary: ComptaApelSummary): Promise<string> {
  const [logo, letterhead] = await Promise.all([loadSchoolLogoForPdf(), getSchoolLetterhead()]);

  const doc = new jsPDF({ compress: true });
  const W = doc.internal.pageSize.getWidth();
  const ML = 14;
  const dateStr = pdfFormatDateFrLong();

  drawPdfLetterhead(doc, letterhead, logo, [16, 185, 129]);
  doc.setFont("helvetica", "bold");
  doc.setFontSize(15);
  doc.setTextColor(30, 41, 59);
  doc.text("RÉCAPITULATIF APEL — SORTIES SCOLAIRES", W / 2, 48, { align: "center" });

  doc.setFontSize(10);
  doc.setFont("helvetica", "normal");
  doc.setTextColor(71, 85, 105);
  doc.text(
    `Année scolaire ${summary.schoolYear.label}, 1er septembre au 15 juillet`,
    W / 2,
    55,
    { align: "center" },
  );

  let y = 64;

  const synthBody = summary.byEtablissement.map((group) => {
    const t = group.totals;
    return [
      group.etablissement,
      euroPlain(t?.apelCollective ?? 0),
      euroPlain(t?.aidesIndividuelles ?? 0),
      euroPlain(t?.totalApel ?? 0),
    ];
  });

  autoTable(doc, {
    startY: y,
    head: [["Niveau scolaire", "APEL collectif", "Aides individ.", "Total à verser"]],
    body: [
      ...synthBody,
      [
        "TOTAL GÉNÉRAL",
        euroPlain(summary.totals.apelCollective),
        euroPlain(summary.totals.aidesIndividuelles),
        euroPlain(summary.totals.totalApel),
      ],
    ],
    theme: "grid",
    headStyles: { fillColor: [16, 185, 129], textColor: 255, fontStyle: "bold", fontSize: 9 },
    columnStyles: {
      0: { cellWidth: 58 },
      1: { halign: "right", cellWidth: 38 },
      2: { halign: "right", cellWidth: 38 },
      3: { halign: "right", cellWidth: 38 },
    },
    bodyStyles: { fontSize: 8.5 },
    styles: { cellPadding: 3 },
    didParseCell: (data) => {
      if (data.section === "body" && data.row.index === synthBody.length) {
        data.cell.styles.fontStyle = "bold";
        data.cell.styles.fillColor = [236, 253, 245];
      }
    },
  });

  y = pdfLastTableY(doc) + 10;

  for (const group of summary.byEtablissement) {
    if (y > 250) {
      doc.addPage();
      y = 20;
    }

    doc.setFont("helvetica", "bold");
    doc.setFontSize(10);
    doc.setTextColor(6, 95, 70);
    doc.text(group.etablissement, ML, y);
    y += 5;

    const body = tripRowsBody(group.trips);
    const subtotalRowIndex = body.length;

    autoTable(doc, {
      startY: y,
      head: [["Voyage", "Date", "APEL collectif", "Aides individ.", "Total APEL"]],
      body: [
        ...body,
        [
          `Sous-total ${group.etablissement}`,
          "",
          euroPlain(group.totals.apelCollective),
          euroPlain(group.totals.aidesIndividuelles),
          euroPlain(group.totals.totalApel),
        ],
      ],
      theme: "grid",
      headStyles: { fillColor: [241, 245, 249], textColor: [51, 65, 85], fontStyle: "bold", fontSize: 8 },
      columnStyles: {
        0: { cellWidth: 52 },
        1: { cellWidth: 34 },
        2: { halign: "right", cellWidth: 28 },
        3: { halign: "right", cellWidth: 28 },
        4: { halign: "right", cellWidth: 28 },
      },
      bodyStyles: { fontSize: 8 },
      styles: { cellPadding: 2.5 },
      didParseCell: (data) => {
        if (data.section === "body" && data.row.index === subtotalRowIndex) {
          data.cell.styles.fontStyle = "bold";
          data.cell.styles.fillColor = [236, 253, 245];
        }
      },
    });

    y = pdfLastTableY(doc) + 8;
  }

  if (summary.byEtablissement.length === 0) {
    doc.setFont("helvetica", "italic");
    doc.setFontSize(9);
    doc.setTextColor(148, 163, 184);
    doc.text("Aucun engagement APEL renseigné sur les voyages de cette année scolaire.", ML, y);
    y += 8;
  }

  doc.setFont("helvetica", "normal");
  doc.setFontSize(8);
  doc.setTextColor(100, 116, 139);
  doc.text(
    "Montants = aide APEL collective (ligne recettes) + aides individuelles. Récapitulatif par niveau pour versement.",
    ML,
    y,
    { maxWidth: W - 28 },
  );

  doc.setFontSize(7);
  doc.setTextColor(180, 190, 200);
  doc.text(`Document généré le ${dateStr}`, W / 2, 292, { align: "center" });

  return doc.output("datauristring");
}

export function comptaApelSummaryPdfFilename(schoolYearLabel: string): string {
  const slug = schoolYearLabel.replace(/\s+/g, "_").replace(/–/g, "-");
  return `Recap_APEL_${slug}.pdf`;
}
