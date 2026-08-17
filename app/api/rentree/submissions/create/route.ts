import { NextResponse } from "next/server";
import { assertEligibleRequestAttachment } from "@/app/lib/requests";
import { findRentreeSubmissionItem } from "@/app/lib/rentree-pages";
import {
  deletePendingRentreeSubmission,
  generateRentreeSubmissionToken,
  isValidRentreeSenderEmail,
  normalizeRentreeSenderEmail,
  notifyRentreeSubmissionVerification,
  rentreeSubmissionConfirmUrl,
  savePendingRentreeSubmission,
} from "@/app/lib/rentree-submissions";
import { getToolboxConfigResolved } from "@/app/lib/toolbox-config";
import { clientIpFromRequest, createSlidingWindowRateLimiter } from "@/app/lib/memory-rate-limit";
import { getTenantSmtpConfig } from "@/app/lib/tenant-mail";

export const runtime = "nodejs";
export const maxDuration = 60;

const limiter = createSlidingWindowRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 8,
});

export async function POST(req: Request) {
  try {
    if (!limiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez plus tard." },
        { status: 429 },
      );
    }

    const form = await req.formData();
    const honeypot = String(form.get("website") || form.get("company") || "").trim();
    if (honeypot) {
      return NextResponse.json({
        success: true,
        needsEmailVerification: true,
        message:
          "Un e-mail de confirmation vient de vous être envoyé si l’adresse est valide.",
      });
    }

    const establishmentId = String(form.get("establishmentId") || "").trim();
    const itemId = String(form.get("itemId") || "").trim();
    const senderEmail = normalizeRentreeSenderEmail(String(form.get("email") || ""));
    const studentName = String(form.get("studentName") || "").trim().slice(0, 120);
    const file = form.get("file");

    if (!establishmentId || !itemId) {
      return NextResponse.json({ error: "Dépôt introuvable." }, { status: 400 });
    }
    if (!isValidRentreeSenderEmail(senderEmail)) {
      return NextResponse.json({ error: "Indiquez une adresse e-mail valide." }, { status: 400 });
    }
    if (!(file instanceof File) || file.size <= 0) {
      return NextResponse.json({ error: "Ajoutez un fichier." }, { status: 400 });
    }

    const toolbox = await getToolboxConfigResolved();
    if (!toolbox.tools.rentree.enabled) {
      return NextResponse.json({ error: "La rentrée n’est pas ouverte." }, { status: 404 });
    }

    const found = findRentreeSubmissionItem(toolbox.tools.rentree.pages, {
      establishmentId,
      itemId,
    });
    if (!found?.item.submission?.recipientEmails.length) {
      return NextResponse.json({ error: "Ce dépôt n’est pas disponible." }, { status: 404 });
    }

    const check = assertEligibleRequestAttachment(file.name, file.type, file.size);
    if (!check.ok) return NextResponse.json({ error: check.error }, { status: 400 });

    if (!(await getTenantSmtpConfig())) {
      return NextResponse.json(
        {
          error:
            "L’envoi par e-mail n’est pas configuré pour cet établissement. Contactez le secrétariat.",
        },
        { status: 503 },
      );
    }

    const token = generateRentreeSubmissionToken();
    try {
      await savePendingRentreeSubmission(
        token,
        {
          senderEmail,
          studentName: studentName || undefined,
          fileName: file.name,
          contentType: file.type || "application/octet-stream",
          size: file.size,
          itemId: found.item.id || itemId,
          itemTitle: found.item.title,
          establishmentId: found.page.establishmentId,
          establishmentLabel: found.page.label,
          recipientEmails: found.item.submission.recipientEmails,
        },
        {
          buffer: Buffer.from(await file.arrayBuffer()),
          fileName: file.name,
          contentType: file.type || "application/octet-stream",
        },
      );
      const confirmUrl = await rentreeSubmissionConfirmUrl(token);
      await notifyRentreeSubmissionVerification(senderEmail, found.item.title, confirmUrl);
    } catch (e) {
      console.error("[rentree/submissions] create:", e);
      try {
        await deletePendingRentreeSubmission(token);
      } catch {
        /* ignore */
      }
      return NextResponse.json(
        {
          error:
            "Impossible d’envoyer l’e-mail de confirmation. Vérifiez l’adresse ou réessayez plus tard.",
        },
        { status: 500 },
      );
    }

    return NextResponse.json({
      success: true,
      needsEmailVerification: true,
      message:
        "Un e-mail vient de vous être envoyé. Cliquez sur le lien qu’il contient pour transmettre le document.",
    });
  } catch (e) {
    console.error("[rentree/submissions] create:", e);
    return NextResponse.json({ error: "Envoi impossible. Réessayez." }, { status: 500 });
  }
}
