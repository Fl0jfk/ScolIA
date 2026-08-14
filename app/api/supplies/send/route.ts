import { NextResponse } from "next/server";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";
import { getToolboxConfig } from "@/app/lib/toolbox-config";
import type { FournituresChild } from "@/app/lib/fournitures-types";
import { buildSuppliesListPdf } from "@/app/lib/fournitures-supplies-pdf";
import { clientIpFromRequest, createMemoryRateLimiter } from "@/app/lib/memory-rate-limit";

const suppliesSendLimiter = createMemoryRateLimiter({
  windowMs: 60 * 60 * 1000,
  max: 5,
});

function isValidEmail(value: string) {
  const v = String(value || "").trim();
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v);
}

export async function POST(req: Request) {
  try {
    if (!suppliesSendLimiter.allow(clientIpFromRequest(req))) {
      return NextResponse.json(
        { error: "Trop de tentatives. Réessayez plus tard." },
        { status: 429 },
      );
    }

    const toolbox = await getToolboxConfig();
    if (!toolbox.tools["simulateur-fournitures"].enabled) {
      return NextResponse.json({ error: "Simulateur fournitures désactivé." }, { status: 404 });
    }

    const body = await req.json();
    const email = String(body?.email || "").trim();
    const children = (body?.children || []) as FournituresChild[];

    if (!isValidEmail(email)) {
      return NextResponse.json({ error: "Email invalide." }, { status: 400 });
    }
    if (!Array.isArray(children) || children.length === 0) {
      return NextResponse.json({ error: "Ajoutez au moins un enfant." }, { status: 400 });
    }

    const smtp = await getTenantSmtpConfig();
    if (!smtp) {
      return NextResponse.json({ error: "SMTP non configuré." }, { status: 503 });
    }
    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré." }, { status: 503 });
    }

    const suppliesByChild = (body?.suppliesByChild || {}) as Record<
      string,
      Array<{ title: string; items: string[] }>
    >;

    const { buffer: pdfBuffer, filename: pdfFilename } = await buildSuppliesListPdf({
      children,
      suppliesByChild,
    });

    await transporter.sendMail({
      from: `"Simulateur Fournitures" <${smtp.user}>`,
      to: email,
      subject: "Votre liste de fournitures (PDF)",
      html: `<div style="font-family:Arial,sans-serif;color:#334155;line-height:1.5">
        <p>Bonjour,</p>
        <p>Vous trouverez en pièce jointe la liste des fournitures sélectionnées sur le simulateur.</p>
        <p>Cordialement,</p>
      </div>`,
      attachments: [
        {
          filename: pdfFilename,
          content: pdfBuffer,
          contentType: "application/pdf",
        },
      ],
    });

    return NextResponse.json({ success: true, filename: pdfFilename });
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("supplies/send error:", message);
    return NextResponse.json({ error: "Échec de l'envoi.", details: message }, { status: 500 });
  }
}
