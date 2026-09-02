import { NextResponse } from "next/server";
import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
import {
  drawPdfFooter,
  drawPdfLetterhead,
  getSchoolLetterhead,
  loadSchoolLogoForPdf,
  type PdfLogo,
} from "@/app/lib/pdf-branding";
import { extractDevisMetadataWithMistral, ocrS3Key } from "@/app/lib/travel-devis-ocr";
import { requireModule } from "@/app/lib/intranet-auth";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo, mergeTransportMailCc } from "@/app/lib/travel-email-routing";
import {
  fetchTravelsPdfBytes,
  resolveTravelsS3ObjectLocation,
} from "@/app/lib/travels-s3";

const SIGNED_DEVIS_COPY_TO = "comptabilite@laprovidence-nicolasbarre.fr";

function buildConfirmationPDF(opts: {
  providerName: string;
  tripTitle: string;
  amount?: string;
  reference?: string;
  extractedPrice?: string | null;
  logo: PdfLogo | null;
  letterhead: Awaited<ReturnType<typeof getSchoolLetterhead>>;
  tripData?: Record<string, unknown>;
}): Buffer {
  const { providerName, tripTitle, amount, reference, extractedPrice, logo, letterhead, tripData } =
    opts;
  const d = tripData || {};
  const nbEleves = Number(d.nbEleves) || 0;
  const nbAccomp = Number(d.nbAccompagnateurs) || 0;
  const effectifTotal = nbEleves + nbAccomp;
  const effectifStr =
    effectifTotal > 0
      ? `${effectifTotal} personnes (dont ${nbAccomp} adultes)`
      : null;
  const startDate = typeof d.startDate === "string" ? d.startDate : "";
  const endDate = typeof d.endDate === "string" ? d.endDate : "";
  const singleDate = typeof d.date === "string" ? d.date : "";
  const datesStr =
    startDate && endDate
      ? `Du ${new Date(startDate).toLocaleDateString("fr-FR")} au ${new Date(endDate).toLocaleDateString("fr-FR")}`
      : singleDate
        ? new Date(singleDate).toLocaleDateString("fr-FR")
        : null;
  const agreedPrice = extractedPrice || amount || null;
  const doc = new jsPDF({ compress: true });
  const W = doc.internal.pageSize.getWidth();
  const ML = 15;
  const MR = W - 15;
  const dateStr = new Date().toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
  drawPdfLetterhead(doc, letterhead, logo, [37, 99, 235]);
  const colB = W / 2 + 8;
  let yA = 45;
  let yB = 45;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text("DESTINATAIRE", colB, yB);
  yB += 5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(12);
  doc.setTextColor(30, 41, 59);
  doc.text(providerName, colB, yB);
  yB += 9;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text("DATE D'ENVOI", colB, yB);
  yB += 4.5;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8.5);
  doc.setTextColor(71, 85, 105);
  doc.text(dateStr, colB, yB);
  const sepY = Math.max(yA, yB) + 9;
  doc.setDrawColor(226, 232, 240);
  doc.setLineWidth(0.4);
  doc.line(ML, sepY, MR, sepY);
  let sy = sepY + 10;
  doc.setFillColor(22, 163, 74);
  doc.rect(ML, sy - 5, 2.5, 13, "F");
  doc.setFont("helvetica", "normal");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text("OBJET", ML + 7, sy - 0.5);
  sy += 5.5;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(14);
  doc.setTextColor(30, 41, 59);
  const subjectLine = [
    "Confirmation de devis transport",
    reference ? `  Réf. ${reference}` : "",
    amount ? `  —  ${amount} €` : "",
  ].join("");
  doc.text(subjectLine, ML + 7, sy);
  sy += 10;
  doc.setFont("helvetica", "italic");
  doc.setFontSize(8.5);
  doc.setTextColor(71, 85, 105);
  const intro =
    "Nous avons le plaisir de vous confirmer la sélection de votre offre pour le projet ci-dessous. Veuillez trouver en pièce jointe le devis signé valant bon de commande. Nous vous remercions de votre réactivité et vous souhaitons bonne route !";
  const introLines = doc.splitTextToSize(intro, MR - ML);
  doc.text(introLines, ML, sy);
  sy += introLines.length * 4.5 + 9;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text("RÉCAPITULATIF", ML, sy);
  sy += 3.5;
  const classes = typeof d.classes === "string" ? d.classes : "";
  const destination = typeof d.destination === "string" ? d.destination : "";
  autoTable(doc, {
    startY: sy,
    body: [
      ["Projet", tripTitle],
      ...(classes ? [["Classes concernées", classes]] : []),
      ...(destination ? [["Destination", destination]] : []),
      ...(datesStr ? [["Date(s) du voyage", datesStr]] : []),
      ...(effectifStr ? [["Effectif total", effectifStr]] : []),
      ...(reference ? [["Référence devis", reference]] : []),
      ...(agreedPrice ? [["💶 Montant convenu", agreedPrice]] : []),
      ["Date de confirmation", dateStr],
    ],
    theme: "plain",
    styles: {
      fontSize: 8.5,
      cellPadding: { top: 3, bottom: 3, left: 4, right: 4 },
      lineColor: [226, 232, 240],
      lineWidth: 0.2,
    },
    alternateRowStyles: { fillColor: [248, 250, 252] },
    columnStyles: {
      0: {
        cellWidth: 58,
        fontStyle: "bold",
        textColor: [30, 41, 59] as [number, number, number],
      },
      1: { textColor: [71, 85, 105] as [number, number, number] },
    },
    tableLineColor: [226, 232, 240],
    tableLineWidth: 0.2,
  });
  const closingY = (doc as unknown as { lastAutoTable: { finalY: number } }).lastAutoTable
    .finalY + 13;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8.5);
  doc.setTextColor(71, 85, 105);
  doc.text(
    "Dans l'attente de vous lire, nous vous adressons nos cordiales salutations.",
    ML,
    closingY,
  );
  drawPdfFooter(doc, letterhead);
  return Buffer.from(doc.output("arraybuffer"));
}

export async function POST(req: Request) {
  const gate = await requireModule("travels");
  if (!gate.ok) return gate.response;

  try {
    const body = (await req.json()) as {
      providerEmail?: string;
      signedQuoteUrl?: string;
      signedQuoteS3Key?: string;
      providerName?: string;
      tripTitle?: string;
      tripData?: Record<string, unknown>;
      amount?: string;
      reference?: string;
    };
    let toEmail = typeof body.providerEmail === "string" ? body.providerEmail.trim() : "";
    const signedQuoteUrl = String(body.signedQuoteUrl || "").trim();
    const signedQuoteS3Key = String(body.signedQuoteS3Key || "").trim() || null;
    const providerName = String(body.providerName || "Transporteur").trim();
    const tripTitle = String(body.tripTitle || "Sortie scolaire").trim();
    if (!signedQuoteUrl && !signedQuoteS3Key) {
      return NextResponse.json(
        { error: "PDF du devis signé manquant (URL ou clé S3)." },
        { status: 400 },
      );
    }

    const [logo, letterhead] = await Promise.all([
      loadSchoolLogoForPdf(),
      getSchoolLetterhead(),
    ]);

    const loc = await resolveTravelsS3ObjectLocation(
      signedQuoteUrl || signedQuoteS3Key || "",
      signedQuoteS3Key,
    );

    let extractedPrice: string | null = null;
    let extractedContactEmail: string | null = null;
    if (loc) {
      try {
        const ocrText = await ocrS3Key(loc.bucket, loc.key);
        if (ocrText) {
          const meta = await extractDevisMetadataWithMistral(ocrText);
          extractedPrice = meta.price;
          extractedContactEmail = meta.contactEmail;
        }
      } catch (ocrErr) {
        console.error("[send-order] Erreur OCR/Mistral:", ocrErr);
      }
    }

    if (!toEmail && extractedContactEmail) {
      toEmail = extractedContactEmail;
    }
    if (!toEmail) {
      return NextResponse.json({ error: "Email du transporteur manquant" }, { status: 400 });
    }

    let pdfBuffer: Buffer;
    try {
      pdfBuffer = await fetchTravelsPdfBytes(
        signedQuoteUrl || signedQuoteS3Key || "",
        signedQuoteS3Key,
      );
    } catch (pdfErr) {
      console.error("[send-order] PDF devis signé introuvable:", pdfErr);
      return NextResponse.json(
        {
          error:
            "Impossible de retrouver le devis signé sur le stockage. Réessayez la signature.",
          details: pdfErr instanceof Error ? pdfErr.message : String(pdfErr),
        },
        { status: 500 },
      );
    }

    const smtp = await getTenantSmtpConfig();
    const transporter = await createTenantTransporter();
    if (!smtp || !transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const confirmationPdf = buildConfirmationPDF({
      providerName,
      tripTitle,
      tripData: body.tripData ?? undefined,
      amount: body.amount ?? undefined,
      reference: body.reference ?? undefined,
      extractedPrice,
      logo,
      letterhead,
    });
    const replyTo = await buildTransportReplyTo();
    const safeTitle = tripTitle.replace(/\s+/g, "_");

    try {
      await transporter.sendMail({
        from: `"Gestion Voyages" <${smtp.user}>`,
        to: toEmail,
        ...(replyTo ? { replyTo } : {}),
        cc: mergeTransportMailCc(),
        subject: `Confirmation de commande : ${tripTitle}`,
        html: `
        <div style="font-family: sans-serif; line-height: 1.5; color: #333;">
          <p>Bonjour <strong>${providerName}</strong>,</p>
          <p>Nous avons le plaisir de vous confirmer la sélection de votre offre pour le projet : <strong>${tripTitle}</strong>.</p>
          <p>Veuillez trouver en pièce jointe la lettre de confirmation ainsi que le <strong>devis signé</strong> valant bon de commande.</p>
          <br />
          <p>Cordialement,</p>
          <p><em>L'administration de l'établissement</em></p>
        </div>
      `,
        attachments: [
          {
            filename: `Confirmation_Transport_${safeTitle}.pdf`,
            content: confirmationPdf,
            contentType: "application/pdf",
          },
          {
            filename: `Devis_Signe_${safeTitle}.pdf`,
            content: pdfBuffer,
            contentType: "application/pdf",
          },
        ],
      });
    } catch (mailErr) {
      console.error("[send-order] envoi transporteur:", mailErr);
      return NextResponse.json(
        {
          error: "Erreur lors de l'envoi du mail au transporteur (SMTP).",
          details: mailErr instanceof Error ? mailErr.message : String(mailErr),
        },
        { status: 500 },
      );
    }

    try {
      await transporter.sendMail({
        from: `"Gestion Voyages" <${smtp.user}>`,
        to: SIGNED_DEVIS_COPY_TO,
        subject: `[Copie] Devis signé — ${tripTitle}`,
        text: [
          `Bonjour,`,
          ``,
          `Ci-joint uniquement le devis signé pour le projet « ${tripTitle} ».`,
          `Le transporteur (${providerName}) a reçu ce devis avec la lettre de confirmation de commande ; ce message ne contient pas cette lettre.`,
          ``,
          `Cordialement,`,
          `Plateforme Voyages — La Providence Nicolas Barré`,
        ].join("\n"),
        attachments: [
          {
            filename: `Devis_Signe_${safeTitle}.pdf`,
            content: pdfBuffer,
            contentType: "application/pdf",
          },
        ],
      });
    } catch (copyErr) {
      console.error("[send-order] copie devis signé (interne):", copyErr);
    }

    return NextResponse.json({ success: true });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("Erreur Envoi Mail:", message);
    return NextResponse.json(
      {
        error: "Erreur lors de l'envoi du mail",
        details: message,
      },
      { status: 500 },
    );
  }
}
