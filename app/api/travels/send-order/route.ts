import { NextResponse } from "next/server";
import { GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
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
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getTenantBucketName } from "@/app/lib/tenant-config";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { buildTransportReplyTo } from "@/app/lib/travel-email-routing";

const SIGNED_DEVIS_COPY_TO = "comptabilite@laprovidence-nicolasbarre.fr";

function buildConfirmationPDF(opts: {
  providerName: string;
  tripTitle: string;
  amount?: string;
  reference?: string;
  extractedPrice?: string | null;
  logo: PdfLogo | null;
  letterhead: Awaited<ReturnType<typeof getSchoolLetterhead>>;
  tripData?: any;
}): Buffer {
  const { providerName, tripTitle, amount, reference, extractedPrice, logo, letterhead, tripData } = opts;
  const d = tripData || {};
  const effectifTotal = (Number(d.nbEleves) || 0) + (Number(d.nbAccompagnateurs) || 0);
  const effectifStr = effectifTotal > 0 ? `${effectifTotal} personnes (dont ${d.nbAccompagnateurs || 0} adultes)` : null;
  const datesStr = d.startDate && d.endDate ? `Du ${new Date(d.startDate).toLocaleDateString("fr-FR")} au ${new Date(d.endDate).toLocaleDateString("fr-FR")}` : d.date ? new Date(d.date).toLocaleDateString("fr-FR") : null;
  const agreedPrice = extractedPrice || amount || null;
  const doc = new jsPDF({ compress: true });
  const W = doc.internal.pageSize.getWidth();
  const H = doc.internal.pageSize.getHeight();
  const ML = 15;
  const MR = W - 15;
  const dateStr = new Date().toLocaleDateString("fr-FR", { day: "2-digit", month: "long", year: "numeric" });
  drawPdfLetterhead(doc, letterhead, logo, [37, 99, 235]);
  const colA = ML;
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
  const intro = "Nous avons le plaisir de vous confirmer la sélection de votre offre pour le projet ci-dessous. Veuillez trouver en pièce jointe le devis signé valant bon de commande. Nous vous remercions de votre réactivité et vous souhaitons bonne route !";
  const introLines = doc.splitTextToSize(intro, MR - ML);
  doc.text(introLines, ML, sy);
  sy += introLines.length * 4.5 + 9;
  doc.setFont("helvetica", "bold");
  doc.setFontSize(6.5);
  doc.setTextColor(148, 163, 184);
  doc.text("RÉCAPITULATIF", ML, sy);
  sy += 3.5;
  autoTable(doc, {
    startY: sy,
    body: [
      ["Projet", tripTitle],
      ...(d.classes ? [["Classes concernées", d.classes]] : []),
      ...(d.destination ? [["Destination", d.destination]] : []),
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
      0: { cellWidth: 58, fontStyle: "bold", textColor: [30, 41, 59] as [number, number, number] },
      1: { textColor: [71, 85, 105] as [number, number, number] },
    },
    tableLineColor: [226, 232, 240],
    tableLineWidth: 0.2,
  });
  const closingY = (doc as any).lastAutoTable.finalY + 13;
  doc.setFont("helvetica", "normal");
  doc.setFontSize(8.5);
  doc.setTextColor(71, 85, 105);
  doc.text("Dans l'attente de vous lire, nous vous adressons nos cordiales salutations.", ML, closingY);
  drawPdfFooter(doc, letterhead);
  return Buffer.from(doc.output("arraybuffer"));
}

export async function POST(req: Request) {
  try {
    const { providerEmail, signedQuoteUrl, providerName, tripTitle, tripData, amount, reference } = await req.json();
    let toEmail = typeof providerEmail === "string" ? providerEmail.trim() : "";
    const [logo, letterhead] = await Promise.all([loadSchoolLogoForPdf(), getSchoolLetterhead()]);
    const urlObj = new URL(signedQuoteUrl);
    const fileKey = decodeURIComponent(urlObj.pathname.substring(1));
    let extractedPrice: string | null = null;
    let extractedContactEmail: string | null = null;
    try {
      const bucket = await getTenantBucketName();
      const ocrText = await ocrS3Key(bucket, fileKey);
      if (ocrText) {
        const meta = await extractDevisMetadataWithMistral(ocrText);
        extractedPrice = meta.price;
        extractedContactEmail = meta.contactEmail;
      }
    } catch (ocrErr) {
      console.error("[send-order] Erreur OCR/Mistral:", ocrErr);
    }
    if (!toEmail && extractedContactEmail) {
      toEmail = extractedContactEmail;
    }
    if (!toEmail) {
      return NextResponse.json({ error: "Email du transporteur manquant" }, { status: 400 });
    }
    const bucket = await getTenantBucketName();
    const s3Client = await getTenantDataS3Client();
    const command = new GetObjectCommand({
      Bucket: bucket,
      Key: fileKey,
    });
    const presignedUrl = await getSignedUrl(s3Client, command, { expiresIn: 120 });
    const pdfResponse = await fetch(presignedUrl);
    if (!pdfResponse.ok) { throw new Error(`Impossible d'accéder au PDF sur S3 : ${pdfResponse.statusText}`);}
    const pdfArrayBuffer = await pdfResponse.arrayBuffer();
    const pdfBuffer = Buffer.from(pdfArrayBuffer);
    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }
    const confirmationPdf = buildConfirmationPDF({
      providerName,
      tripTitle,
      tripData: tripData ?? undefined,
      amount: amount ?? undefined,
      reference: reference ?? undefined,
      extractedPrice,
      logo,
      letterhead,
    });
    const replyTo = await buildTransportReplyTo();
    const mailOptions = {
      from: `"Gestion Voyages" <${smtp.user}>`,
      to: toEmail,
      ...(replyTo ? { replyTo } : {}),
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
          filename: `Confirmation_Transport_${tripTitle.replace(/\s+/g, "_")}.pdf`,
          content: confirmationPdf,
          contentType: "application/pdf",
        },
        {
          filename: `Devis_Signe_${tripTitle.replace(/\s+/g, '_')}.pdf`,
          content: pdfBuffer,
          contentType: 'application/pdf'
        }
      ]
    };
    await transporter.sendMail(mailOptions);

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
            filename: `Devis_Signe_${tripTitle.replace(/\s+/g, "_")}.pdf`,
            content: pdfBuffer,
            contentType: "application/pdf",
          },
        ],
      });
    } catch (copyErr) {
      console.error("[send-order] copie devis signé (interne):", copyErr);
    }

    return NextResponse.json({ success: true });
  } catch (error: any) {
    console.error("Erreur Envoi Mail:", error.message);
    return NextResponse.json({ 
      error: "Erreur lors de l'envoi du mail", 
      details: error.message 
    }, { status: 500 });
  }
}