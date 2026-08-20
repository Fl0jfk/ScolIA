import "server-only";

import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import { MARKETING } from "@/app/lib/marketing-site";
import { formatEur } from "@/app/lib/pricing";
import type { BillingMode } from "@/app/lib/pricing";

export type BillingInvoiceLine = {
  label: string;
  detail?: string;
  amountEur: number;
};

export type BillingInvoiceInput = {
  invoiceNumber: string;
  issuedAt: Date;
  customerName: string;
  customerEmail: string;
  customerAddress?: {
    street?: string;
    zip?: string;
    city?: string;
  };
  billingMode: BillingMode;
  periodLabel: string;
  lineItems: BillingInvoiceLine[];
  totalEur: number;
  transactionId?: string;
  tenantSlug?: string;
};

const GREEN = rgb(47 / 255, 107 / 255, 74 / 255);
const INK = rgb(20 / 255, 35 / 255, 26 / 255);
const MUTED = rgb(75 / 255, 99 / 255, 88 / 255);
const LINE = rgb(0.85, 0.9, 0.87);

function formatDateFr(d: Date): string {
  return d.toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

export function buildInvoiceNumber(opts: {
  slug: string;
  paidAt: Date;
  tid?: string;
}): string {
  const yyyymm = `${opts.paidAt.getUTCFullYear()}${String(opts.paidAt.getUTCMonth() + 1).padStart(2, "0")}`;
  const slugPart = opts.slug
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 12);
  const tidPart = (opts.tid || "XXXX").replace(/[^a-zA-Z0-9]/g, "").slice(-6).toUpperCase();
  return `FAC-${slugPart || "SCOLA"}-${yyyymm}-${tidPart}`;
}

export function billingPeriodLabel(mode: BillingMode, paidAt: Date): string {
  if (mode === "annual_upfront") {
    const y = paidAt.getFullYear();
    return `Année scolaire ${y}–${y + 1}`;
  }
  return paidAt.toLocaleDateString("fr-FR", { month: "long", year: "numeric" });
}

/** Génère une facture PDF Scola (abonnement). */
export async function renderBillingInvoicePdf(input: BillingInvoiceInput): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const page = doc.addPage([595.28, 841.89]); // A4
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const fontBold = await doc.embedFont(StandardFonts.HelveticaBold);

  const margin = 48;
  const width = page.getWidth();
  let y = page.getHeight() - margin;

  const drawText = (
    text: string,
    x: number,
    yy: number,
    size: number,
    opts?: { bold?: boolean; color?: ReturnType<typeof rgb>; maxWidth?: number },
  ) => {
    page.drawText(text, {
      x,
      y: yy,
      size,
      font: opts?.bold ? fontBold : font,
      color: opts?.color ?? INK,
      maxWidth: opts?.maxWidth,
    });
  };

  // Header
  drawText(MARKETING.productName, margin, y, 22, { bold: true, color: GREEN });
  y -= 16;
  drawText(MARKETING.tagline, margin, y, 9, { color: MUTED });
  y -= 28;

  drawText("FACTURE", width - margin - 90, page.getHeight() - margin, 16, {
    bold: true,
    color: GREEN,
  });
  drawText(`N° ${input.invoiceNumber}`, width - margin - 160, page.getHeight() - margin - 18, 10, {
    color: MUTED,
  });
  drawText(`Date : ${formatDateFr(input.issuedAt)}`, width - margin - 160, page.getHeight() - margin - 32, 10, {
    color: MUTED,
  });

  // Émetteur
  const legal = MARKETING.legal;
  drawText("Émetteur", margin, y, 10, { bold: true, color: GREEN });
  y -= 14;
  drawText(legal.companyName, margin, y, 10, { bold: true });
  y -= 12;
  drawText(`${legal.legalForm} — capital ${legal.shareCapital}`, margin, y, 9, { color: MUTED });
  y -= 12;
  drawText(legal.address, margin, y, 9, { color: MUTED, maxWidth: 240 });
  y -= 12;
  drawText(`SIRET ${legal.siret} · ${legal.rcs}`, margin, y, 8, { color: MUTED });
  y -= 12;
  drawText(`TVA ${legal.vat}`, margin, y, 8, { color: MUTED });
  y -= 28;

  // Client
  drawText("Facturé à", margin, y, 10, { bold: true, color: GREEN });
  y -= 14;
  drawText(input.customerName, margin, y, 11, { bold: true });
  y -= 13;
  drawText(input.customerEmail, margin, y, 9, { color: MUTED });
  y -= 12;
  if (input.customerAddress?.street) {
    drawText(input.customerAddress.street, margin, y, 9, { color: MUTED });
    y -= 12;
  }
  const cityLine = [input.customerAddress?.zip, input.customerAddress?.city].filter(Boolean).join(" ");
  if (cityLine) {
    drawText(cityLine, margin, y, 9, { color: MUTED });
    y -= 12;
  }
  if (input.tenantSlug) {
    drawText(`Réf. espace : ${input.tenantSlug}`, margin, y, 8, { color: MUTED });
    y -= 12;
  }
  y -= 16;

  // Période
  const modeLabel = input.billingMode === "monthly" ? "Abonnement mensuel" : "Abonnement annuel";
  drawText(`${modeLabel} — ${input.periodLabel}`, margin, y, 10, { bold: true });
  y -= 22;

  // Table header
  page.drawRectangle({
    x: margin,
    y: y - 6,
    width: width - margin * 2,
    height: 22,
    color: rgb(0.93, 0.97, 0.94),
  });
  drawText("Désignation", margin + 8, y, 9, { bold: true, color: GREEN });
  drawText("Montant", width - margin - 70, y, 9, { bold: true, color: GREEN });
  y -= 28;

  for (const item of input.lineItems) {
    drawText(item.label, margin + 8, y, 10, { maxWidth: width - margin * 2 - 100 });
    drawText(formatEur(item.amountEur, { decimals: 2 }), width - margin - 70, y, 10);
    y -= 12;
    if (item.detail) {
      drawText(item.detail, margin + 8, y, 8, { color: MUTED, maxWidth: width - margin * 2 - 100 });
      y -= 12;
    }
    page.drawLine({
      start: { x: margin, y: y + 4 },
      end: { x: width - margin, y: y + 4 },
      thickness: 0.5,
      color: LINE,
    });
    y -= 14;
  }

  y -= 8;
  page.drawRectangle({
    x: width - margin - 180,
    y: y - 10,
    width: 180,
    height: 36,
    color: rgb(0.93, 0.97, 0.94),
  });
  drawText("Total TTC", width - margin - 168, y + 8, 10, { bold: true, color: MUTED });
  drawText(formatEur(input.totalEur, { decimals: 2 }), width - margin - 168, y - 6, 14, {
    bold: true,
    color: GREEN,
  });
  y -= 50;

  if (input.transactionId) {
    drawText(`Référence de paiement : ${input.transactionId}`, margin, y, 8, { color: MUTED });
    y -= 12;
  }
  drawText("Paiement reçu — facture acquittée.", margin, y, 9, { bold: true, color: GREEN });
  y -= 24;

  drawText(
    "TVA non applicable, art. 293 B du CGI (à confirmer selon votre régime fiscal).",
    margin,
    y,
    8,
    { color: MUTED, maxWidth: width - margin * 2 },
  );
  y -= 28;
  drawText(`Contact : ${MARKETING.contactEmail}`, margin, y, 8, { color: MUTED });
  y -= 12;
  drawText(`Document généré automatiquement par ${MARKETING.productName}.`, margin, y, 8, {
    color: MUTED,
  });

  return doc.save();
}
