import { NextResponse } from "next/server";
import { PDFDocument } from "pdf-lib";
import { requireAppUser } from "@/app/lib/app-session";
import { canSignTravelsDirectionFromAppUserByEstablishmentId } from "@/app/lib/establishments";
import { resolveDirectionSignatureBytes } from "@/app/lib/direction-signature";
import {
  findAllDevisSignatureZones,
  textractSignatureBBoxToPdfLibDrawCoords,
} from "@/app/lib/travel-devis-ocr";
import { fetchTravelsPdfBytes } from "@/app/lib/travels-s3";

const SIG_W = 150;
const SIG_H = 75;

export async function POST(req: Request) {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return NextResponse.json({ error: "Non autorisé." }, { status: 401 });
  }

  try {
    const { quoteUrl, signatureType } = await req.json();
    const estId = String(signatureType || "").trim();
    if (!estId) {
      return NextResponse.json(
        { error: "Établissement de signature manquant (signatureType)." },
        { status: 400 },
      );
    }

    const allowed = await canSignTravelsDirectionFromAppUserByEstablishmentId(
      appUser.user,
      estId,
    );
    if (!allowed) {
      return NextResponse.json(
        {
          error:
            "Seule la direction de l’établissement concerné peut signer ce devis.",
        },
        { status: 403 },
      );
    }

    const sigBytes = await resolveDirectionSignatureBytes(estId);
    if (!sigBytes?.length) {
      return NextResponse.json(
        {
          error:
            "Signature direction non configurée. Paramètres → Établissements → ajouter la signature.",
        },
        { status: 400 },
      );
    }

    const pdfBuffer = await fetchTravelsPdfBytes(quoteUrl);
    const pdfDoc = await PDFDocument.load(pdfBuffer);

    const isJpg = sigBytes[0] === 0xff && sigBytes[1] === 0xd8;
    const sigImage = isJpg
      ? await pdfDoc.embedJpg(sigBytes)
      : await pdfDoc.embedPng(sigBytes);

    const pages = pdfDoc.getPages();

    const zones = await findAllDevisSignatureZones(pdfBuffer);

    let stampedCount = 0;

    if (zones.length > 0) {
      for (const bbox of zones) {
        const pageIndex = Math.min(Math.max(1, bbox.pageNumber), pages.length) - 1;
        const page = pages[pageIndex]!;
        const { width: pw, height: ph } = page.getSize();
        const { x, y } = textractSignatureBBoxToPdfLibDrawCoords(pw, ph, bbox, SIG_W, SIG_H);
        page.drawImage(sigImage, { x, y, width: SIG_W, height: SIG_H });
        stampedCount += 1;
      }
      console.log(
        `[sign-pdf] ${stampedCount} signature(s) apposée(s) sur ${zones.length} zone(s) détectée(s)`,
      );
    } else {
      const lastPage = pages[pages.length - 1]!;
      const { width } = lastPage.getSize();
      lastPage.drawImage(sigImage, {
        x: width - 210,
        y: 60,
        width: SIG_W,
        height: SIG_H,
      });
      stampedCount = 1;
      console.warn("[sign-pdf] Aucune zone Vision — fallback bas-droite dernière page");
    }

    const pdfBase64 = await pdfDoc.saveAsBase64({ dataUri: true });
    return NextResponse.json({
      success: true,
      signedPdfData: pdfBase64,
      fileName: `devis_signe_${estId}.pdf`,
      stampedCount,
      zonesDetected: zones.length,
    });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : "Erreur technique signature";
    console.error("Erreur signature API:", message);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
