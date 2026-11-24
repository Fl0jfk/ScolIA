import "server-only";

import { jsPDF } from "jspdf";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
} from "@/app/lib/pdf-branding";
import { dateFr, type FamilyPagePreviewModel } from "@/app/lib/travels-family-page";
import { FAMILY_PAGE_TEMPLATE } from "@/app/lib/travels-family-page-template";
import {
  MEAL_TYPE_LABELS,
  type FamilyAnswers,
  type MealType,
} from "@/app/lib/travels-v2-registration-types";

const SLATE: [number, number, number] = [30, 41, 59];
const BODY: [number, number, number] = [71, 85, 105];
const MUTED: [number, number, number] = [148, 163, 184];
const INDIGO: [number, number, number] = [67, 56, 202];

function eur(n: number): string {
  return `${n.toFixed(2).replace(".", ",")} €`;
}

export function formatParisDateTime(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "long",
    timeStyle: "short",
  });
}

export type SignedCircularInput = {
  model: FamilyPagePreviewModel;
  studentName: string;
  answers: FamilyAnswers;
  decision: "oui" | "non";
  signerName: string;
  signedAt: string;
  /** Adresse (masquée) ayant reçu le code de vérification. */
  maskedEmail: string;
  /** Pièces déposées : libellé de la pièce -> nom de fichier (ou « dispensé »). */
  documents: { label: string; status: string }[];
  signaturePng?: Buffer;
  reference: string;
};

/** PDF de la circulaire signée : trame fixe remplie + réponses + signature + date/heure + nom. */
export async function buildSignedCircularPdf(input: SignedCircularInput): Promise<Buffer> {
  const [letterhead, logo] = await Promise.all([getSchoolLetterhead(), loadSchoolLogoForPdf()]);
  const doc = new jsPDF({ compress: true });
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();
  const ML = 15;
  const MR = W - 15;
  const TEXT_W = MR - ML;
  const BOTTOM = H - 22;
  const m = input.model;
  const h = FAMILY_PAGE_TEMPLATE.headings;

  drawPdfLetterhead(doc, letterhead, logo, INDIGO);
  let y = 45;

  const ensure = (needed: number) => {
    if (y + needed > BOTTOM) {
      doc.addPage();
      y = 20;
    }
  };
  const heading = (t: string) => {
    ensure(14);
    y += 3;
    doc.setFont("helvetica", "bold");
    doc.setFontSize(11);
    doc.setTextColor(...SLATE);
    doc.text(t, ML, y);
    y += 5.5;
  };
  const para = (t: string, opts?: { bold?: boolean; size?: number; color?: [number, number, number] }) => {
    doc.setFont("helvetica", opts?.bold ? "bold" : "normal");
    doc.setFontSize(opts?.size ?? 9.5);
    doc.setTextColor(...(opts?.color ?? BODY));
    const lines = doc.splitTextToSize(t, TEXT_W) as string[];
    for (const line of lines) {
      ensure(5);
      doc.text(line, ML, y);
      y += 4.6;
    }
    y += 1.2;
  };

  doc.setFont("helvetica", "bold");
  doc.setFontSize(15);
  doc.setTextColor(...SLATE);
  const titleLines = doc.splitTextToSize(m.title, TEXT_W) as string[];
  for (const l of titleLines) {
    doc.text(l, ML, y);
    y += 6.5;
  }
  if (m.destination) para(m.destination, { color: MUTED });
  if (m.dateLine) para(m.dateLine.charAt(0).toUpperCase() + m.dateLine.slice(1), { bold: true, color: INDIGO });
  const meta = [m.classes ? `Classes : ${m.classes}` : "", m.organizer ? `Organisateur : ${m.organizer}` : ""]
    .filter(Boolean)
    .join(" · ");
  if (meta) para(meta, { size: 8.5, color: MUTED });
  para(`Élève : ${input.studentName}`, { bold: true, color: SLATE });

  for (const p of m.intro) para(p);

  heading(h.meeting);
  const meet = (label: string, p: FamilyPagePreviewModel["outbound"]) => {
    if (p && p.place && p.time) para(`${label} : ${p.date ? `${dateFr(p.date)} à ` : "à "}${p.time} — ${p.place}`);
  };
  meet(h.outbound, m.outbound);
  meet(h.inbound, m.inbound);

  if (m.program) {
    heading(h.program);
    para(m.program);
  }
  if (m.equipment.length > 0) {
    heading(h.equipment);
    para(m.equipment.map((e) => `• ${e}`).join("\n"));
  }
  if (m.cash) {
    heading(h.cash);
    para(`${eur(m.cash.amount)} — ${m.cash.reason}`);
    para(FAMILY_PAGE_TEMPLATE.cashNotIncluded, { bold: true });
  }

  heading(h.price);
  if (m.price) {
    para(`Prix par élève : ${eur(m.price.pricePerStudent)}`, { bold: true });
    if (m.price.billingSentence) para(m.price.billingSentence);
    if (m.price.months.length > 0) {
      para(m.price.months.map((x) => `${x.label} : ${eur(x.amount)}`).join("   |   "), { size: 8.5 });
    }
    if (m.price.oneShotSentence) para(m.price.oneShotSentence);
    if (m.price.paymentText) para(m.price.paymentText);
  }
  heading(h.help);
  para(m.financialHelp);
  heading(h.data);
  para(m.dataRetention);

  /* Réponses de la famille */
  heading("Réponse de la famille");
  para(
    input.decision === "oui"
      ? `${input.studentName} PARTICIPE au voyage. Le responsable signataire s'engage à régler la participation financière indiquée ci-dessus.`
      : `${input.studentName} NE PARTICIPE PAS au voyage.`,
    { bold: true, color: SLATE },
  );
  if (input.decision === "oui") {
    if (input.answers.phone) para(`Téléphone joignable pendant le voyage : ${input.answers.phone}`);
    if (m.picnicDays.length > 0) {
      const lines = m.picnicDays.map((d) => {
        const c = input.answers.meals?.[d.key];
        if (!c) return `${d.label} : —`;
        return c.wants === "oui"
          ? `${d.label} : panier fourni par l'établissement (${MEAL_TYPE_LABELS[(c.type ?? "classique") as MealType]})`
          : `${d.label} : repas fourni par la famille`;
      });
      heading(h.meals);
      para(lines.join("\n"));
    }
    if (input.documents.length > 0) {
      heading(h.documents);
      para(input.documents.map((d) => `• ${d.label} — ${d.status}`).join("\n"));
    }
  }

  /* Signature */
  ensure(60);
  heading("Signature");
  para(`Signataire : ${input.signerName}`, { bold: true, color: SLATE });
  para(`Signé le ${formatParisDateTime(input.signedAt)} (heure de Paris).`);
  para(`Identité vérifiée par un code envoyé à l'adresse ${input.maskedEmail}.`, { size: 8.5, color: MUTED });
  if (input.signaturePng) {
    ensure(34);
    try {
      doc.addImage(`data:image/png;base64,${input.signaturePng.toString("base64")}`, "PNG", ML, y, 60, 26);
    } catch {
      para("(tracé de signature non affichable — conservé séparément)", { size: 8, color: MUTED });
    }
    doc.setDrawColor(...MUTED);
    doc.setLineWidth(0.3);
    doc.rect(ML, y, 60, 26);
    y += 30;
  }
  para(`Référence : ${input.reference}`, { size: 8, color: MUTED });

  const pages = doc.getNumberOfPages();
  for (let i = 1; i <= pages; i++) {
    doc.setPage(i);
    drawPdfFooter(doc, letterhead, `Page ${i}/${pages}`);
  }
  return Buffer.from(doc.output("arraybuffer"));
}
