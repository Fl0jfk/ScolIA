import { NextResponse } from "next/server";
import { requireModule } from "@/app/lib/intranet-auth";

import { loadAppConfig } from "@/app/lib/app-config";
import { fetchTravelsPdfBytes } from "@/app/lib/travels-s3";
import { zeendocDestinationEmail } from "@/app/lib/travels-establishments";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
} from "@/app/lib/tenant-mail";

export async function POST(req: Request) {
  // NJ-11 : module « travels » exigé (et pas seulement une session).
  const gate = await requireModule("travels");
  if (!gate.ok) return gate.response;
  const smtp = await getTenantSmtpConfig();
  if (!smtp) {
    return NextResponse.json({ error: "SMTP non configuré." }, { status: 500 });
  }

  let body: { fileUrl?: string; fileName?: string };
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  const fileUrl = String(body.fileUrl || "").trim();
  const fileName = String(body.fileName || "document").trim();
  if (!fileUrl) {
    return NextResponse.json({ error: "fileUrl requis." }, { status: 400 });
  }

  const config = await loadAppConfig();
  const destination = zeendocDestinationEmail(config);
  if (!destination) {
    return NextResponse.json(
      { error: "Destinataire non configuré (Zeendoc / envoi mail)." },
      { status: 400 },
    );
  }

  try {
    const fileBuffer = await fetchTravelsPdfBytes(fileUrl);
    const contentType = "application/pdf";
    if (!fileBuffer || fileBuffer.length === 0) {
      throw new Error("Pièce jointe vide : envoi annulé.");
    }

    const transporter = await createTenantTransporter();
    if (!transporter) {
      return NextResponse.json({ error: "SMTP non configuré." }, { status: 500 });
    }

    await transporter.sendMail({
      from: `"Travels" <${smtp.user}>`,
      to: destination,
      subject: `Travels - document joint (${fileName})`,
      text: [
        "Envoi automatique Travels.",
        `Document joint: ${fileName}`,
        `Type: ${contentType}`,
        `Taille: ${fileBuffer.length} octets`,
      ].join("\n"),
      attachments: [
        {
          filename: fileName || "document",
          content: fileBuffer,
          contentType,
        },
      ],
    });

    return NextResponse.json({ success: true });
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : "Erreur d'envoi.";
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}
