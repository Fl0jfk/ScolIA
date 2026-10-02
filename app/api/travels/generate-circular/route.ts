import { NextResponse } from 'next/server';
import { resolveSession } from "@/app/lib/intranet-session";

import { jsPDF } from 'jspdf';
import { drawPdfLetterhead, getSchoolLetterhead, loadSchoolLogoForPdf } from "@/app/lib/pdf-branding";
import { normalizeTravelImageUrl } from "@/app/lib/travels-image-url";
import { parseTravelsS3KeyFromUrl } from "@/app/lib/travels-s3";
import { requireMistralApiKey } from "@/app/lib/tenant-config";
import { runTextractForS3Key } from "@/app/lib/ocr-textract";

export async function POST(req: Request) {
  const session = await resolveSession();
  const userId = session?.userId;
  if (!userId) return new NextResponse("Non autorisé", { status: 401 });
  try {
    const { tripData } = await req.json();
    const d = tripData.data;
    const professorName = tripData.ownerName || "l'enseignant";
    const costPerStudent = d.costPerStudent || (d.coutTotal && d.nbEleves ? (d.coutTotal / d.nbEleves).toFixed(2) : 'À préciser');
    let ocrCombinedText = "";
    const documents = tripData.data?.attachments; 
    if (documents && Array.isArray(documents)) {      
      for (const doc of documents) {
        try {
          const docKey = doc.key || (doc.url ? await parseTravelsS3KeyFromUrl(doc.url) : null);
          if (!docKey) {
            console.warn(`[OCR] Clé S3 introuvable pour ${doc.name || "document"}`);
            continue;
          }
          const ocrResult = await runTextractForS3Key(docKey);
          ocrCombinedText += `\n--- Contenu du document [${doc.name || docKey}] ---\n${ocrResult.text}\n`;
        } catch (ocrErr) { console.error(`Erreur OCR sur le document ${doc.name}:`, ocrErr)}
      }
    }
    const superContext = `
      DONNÉES DU FORMULAIRE :
      - Titre : ${d.title}
      - Classes concernées : ${d.classes || "—"}
      - Objectifs pédagogiques : ${d.objectifs || "—"}
      - Destination : ${d.destination}
      - Organisateur : ${professorName}
      - Dates : du ${d.startDate || d.date} au ${d.endDate || d.date}
      - Effectif : ${d.nbEleves || "—"} élèves / ${d.nbAccompagnateurs || "—"} accompagnateurs
      - Coût par élève : ${costPerStudent} €
      - Transport : ${d.needsBus ? "Autocar" : "Non coché (voir documents)"}
      - Lieu de RDV : ${d.transportRequest?.pickupPoint || "—"}
      - Bus sur place pour les visites : ${d.transportRequest?.stayOnSite ? "Oui" : "Non"}
      - Infos complémentaires transport : ${d.transportRequest?.freeText || "—"}
      - Pique-nique : ${(d.piqueNiqueDetails?.active || d.piqueNique) ? "Oui" : "Non"}
      - Pique-nique (détails) : ${d.piqueNiqueDetails?.active ? `Livraison à ${d.piqueNiqueDetails.deliveryPlace} (${d.piqueNiqueDetails.deliveryTime || "heure à préciser"})` : "—"}
      CONTENU DES PIÈCES JOINTES (ANALYSE OCR) : ${ocrCombinedText || "Aucun document joint n'a pu être analysé."}`;
    const mistralKey = await requireMistralApiKey();
    const mistralResponse = await fetch("https://api.mistral.ai/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${mistralKey}`
      },
      body: JSON.stringify({
        model: "mistral-small-latest",
        messages: [
          { 
            role: "system", 
            content: `Tu es un assistant de communication scolaire. Ton but est de rédiger une circulaire qui donne envie aux parents et les rassure.
            SOURCES : On te donne un formulaire et des extraits OCR de documents.
            CONSIGNES : N'invente rien, ne te trompe pas de données.
            1. NE GARDE QUE ce qui est utile aux parents : Horaires, lieu de rendez-vous, équipement à prévoir si il y en a, activités phares.
            2. IGNORE TOUT LE RESTE : Détails d'assurance, clauses juridiques, contrats internes, montants financiers HT/TTC globaux, ou documents sans rapport (pranks). Les parents s'en foutent des détails de l'assurance voyage.
            3. TON : Enthousiaste, clair et professionnel.
            4. FORMAT : Pas de Markdown (pas de # ni de *), pas de texte entre crochets. Juste du texte brut aéré.` 
          },
          { 
            role: "user", 
            content: `Rédige la circulaire pour les parents à partir de ce dossier : ${superContext}`
          }
        ],
        temperature: 0.5
      })
    });
    if (!mistralResponse.ok) {
      const errBody = await mistralResponse.text().catch(() => "");
      console.error("Mistral API error:", mistralResponse.status, errBody);
      return NextResponse.json({ error: "Échec génération du texte (service IA)." }, { status: 502 });
    }
    const resData = await mistralResponse.json();
    let generatedText = resData.choices?.[0]?.message?.content || "Détails du voyage à venir...";
    generatedText = generatedText.replace(/[*#]/g, '').trim();
    const [logo, letterhead] = await Promise.all([loadSchoolLogoForPdf(), getSchoolLetterhead()]);
    const docPdf = new jsPDF({ compress: true });
    const W  = docPdf.internal.pageSize.getWidth();  
    const H  = docPdf.internal.pageSize.getHeight();
    const ML = 15;
    const MR = W - 15;
    const CW = W - 30; 
    const HEADER_H     = 38;
    const BANNER_H     = 58; 
    const FOOTER_RESV  = 88;
    const nowStr = new Date().toLocaleDateString("fr-FR", { day: "2-digit", month: "long", year: "numeric" });
    const startDate = d.startDate || d.date || "";
    const endDate   = d.endDate   || d.date || "";
    const dateLine  = (d.endDate && d.endDate !== d.startDate) ? `du ${startDate} au ${endDate}` : `le ${startDate}`;
    const transportLine = d.needsBus ? `Autocar — RDV : ${d.transportRequest?.pickupPoint || "à préciser"}`: "Non précisé";
    const renderLetterhead = () => {
      drawPdfLetterhead(docPdf, letterhead, logo, [37, 99, 235]);
    };
    renderLetterhead();
    let currentY = HEADER_H + 2;
    if (tripData.imageUrl) {
      try {
        const { loadTravelCoverImageBytes } = await import("@/app/lib/travels-cover-cdn");
        const loaded = await loadTravelCoverImageBytes(tripData.imageUrl);
        if (loaded) {
          const mime = loaded.contentType.includes("png")
            ? "PNG"
            : loaded.contentType.includes("webp")
              ? "WEBP"
              : "JPEG";
          const imgData = `data:${loaded.contentType};base64,${loaded.bytes.toString("base64")}`;
          docPdf.addImage(imgData, mime, 0, currentY, W, BANNER_H);
          currentY += BANNER_H;
        }
      } catch (e) {console.error("Erreur image circulaire:", e)}
    }
    docPdf.setFillColor(30, 41, 59);
    docPdf.rect(0, currentY, W, 13, "F");
    docPdf.setFillColor(37, 99, 235);
    docPdf.rect(0, currentY + 11, W, 2, "F");
    docPdf.setFont("helvetica", "bold");
    docPdf.setFontSize(12);
    docPdf.setTextColor(255, 255, 255);
    const titleStr = String(d.title || "Voyage").toUpperCase();
    docPdf.text(titleStr, ML, currentY + 8.5, { maxWidth: CW - 30 });
    docPdf.setFont("helvetica", "normal");
    docPdf.setFontSize(6.5);
    docPdf.setTextColor(147, 197, 253);
    docPdf.text("CIRCULAIRE PARENTS", MR, currentY + 8.5, { align: "right" });
    currentY += 17;
    const infoBoxH = 36;
    docPdf.setFillColor(248, 250, 252);
    docPdf.setDrawColor(226, 232, 240);
    docPdf.rect(ML - 2, currentY, CW + 4, infoBoxH, "FD");
    const halfW = CW / 2;
    const infoRows = [
      [["Organisateur", professorName],         ["Destination",     d.destination || "—"]],
      [["Date(s)",      dateLine],               ["Effectif",        `${d.nbEleves || "—"} élèves, ${d.nbAccompagnateurs || "—"} accomp.`]],
      [["Transport",    transportLine],           ["Participation",   `${costPerStudent} €`]],
    ];
    let infoY = currentY + 7;
    infoRows.forEach(([left, right]) => {
      [[left, ML], [right, ML + halfW]].forEach(([item, colX]) => {
        const [label, value] = item as [string, string];
        const x = colX as number;
        docPdf.setFont("helvetica", "bold");
        docPdf.setFontSize(6.5);
        docPdf.setTextColor(100, 116, 139);
        docPdf.text(label.toUpperCase(), x, infoY);
        docPdf.setFont("helvetica", "normal");
        docPdf.setFontSize(8);
        docPdf.setTextColor(30, 41, 59);
        const trimmed = docPdf.splitTextToSize(value, halfW - 3)[0] || "";
        docPdf.text(trimmed, x, infoY + 4);
      });
      infoY += 11;
    });
    currentY += infoBoxH + 7;
    docPdf.setFont("helvetica", "bold");
    docPdf.setFontSize(10);
    docPdf.setTextColor(37, 99, 235);
    docPdf.text("Programme et informations pour votre enfant", ML, currentY);
    docPdf.setFillColor(37, 99, 235);
    docPdf.rect(ML, currentY + 1.5, CW, 0.4, "F");
    currentY += 8;
    docPdf.setTextColor(55, 65, 81);
    docPdf.setFont("helvetica", "normal");
    docPdf.setFontSize(9.5);
    const paragraphs = generatedText.replace(/\r\n/g, "\n").split(/\n{2,}/g).map((p: string) => p.trim()).filter(Boolean);
    const mainEndY = H - FOOTER_RESV;
    const lineH    = 5;
    const renderParagraph = (p: string) => {
      const lines = docPdf.splitTextToSize(p, CW);
      for (const line of lines) {
        if (currentY + lineH > mainEndY) {
          docPdf.addPage();
          renderLetterhead();
          currentY = HEADER_H + 10;
        }
        docPdf.text(String(line), ML, currentY);
        currentY += lineH;
      }
    };
    for (const p of paragraphs) {
      renderParagraph(p);
      if (currentY + lineH < mainEndY) currentY += 2;
    }
    const couponY = H - 82;
    docPdf.setDrawColor(180, 180, 180);
    docPdf.setLineDashPattern([2, 2], 0);
    docPdf.line(ML - 2, couponY - 5, MR + 2, couponY - 5);
    docPdf.setLineDashPattern([], 0);
    docPdf.setFont("helvetica", "normal");
    docPdf.setFontSize(7);
    docPdf.setTextColor(160, 160, 160);
    docPdf.text("✂", ML - 2, couponY - 3);
    docPdf.setFillColor(255, 251, 235);
    docPdf.setDrawColor(245, 158, 11);
    docPdf.rect(ML - 2, couponY, CW + 4, 77, "FD");
    docPdf.setFont("helvetica", "bold");
    docPdf.setFontSize(9);
    docPdf.setTextColor(180, 83, 9);
    docPdf.text("COUPON-RÉPONSE", ML + 3, couponY + 8);
    docPdf.setFont("helvetica", "normal");
    docPdf.setFontSize(8.5);
    docPdf.setTextColor(40, 40, 40);
    const couponLines: string[] = [
      `Nom et prénom de l'élève : .....................................................   Classe : ...............`,
      `Responsable légal (nom, prénom) : .....................................................................................................`,
      ``,
      `autorise mon enfant à participer au voyage "${d.title}" organisé par ${professorName}`,
      `et m'engage à régler la somme de  ${costPerStudent} €  correspondant aux frais de participation.`,
      ``,
      `Fait à : ...................................   Le : ....../ ....../ 20......   Signature : ............................................`,
    ];
    let couponCY = couponY + 17;
    for (const line of couponLines) {
      if (line === "") { couponCY += 2; continue; }
      const wrapped = docPdf.splitTextToSize(line, CW - 4);
      for (const wl of wrapped) {
        docPdf.text(String(wl), ML + 2, couponCY);
        couponCY += 5.5;
      }
    }
    docPdf.setFont("helvetica", "normal");
    docPdf.setFontSize(7);
    docPdf.setTextColor(180, 190, 200);
    docPdf.text(`Document généré le ${nowStr} — ${letterhead.name}`, W / 2, H - 2, { align: "center" });
    const pdfBase64 = docPdf.output('datauristring');
    return NextResponse.json({ pdf: pdfBase64 });
  } catch (error) {
    console.error("Erreur générale dans la route :", error);
    return NextResponse.json({ error: "Échec génération" }, { status: 500 });
  }
}