import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import { collectEleveParentEmails } from "@/app/lib/eleves-parent-emails";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { assertTravelsTripAccess } from "@/app/lib/travels-rbac-server";
import type { TravelsParentComLog, TravelsTrip } from "@/app/lib/travels-types";
import {
  createTenantTransporter,
  getTenantSmtpConfig,
  sendMailWithTimeout,
} from "@/app/lib/tenant-mail";

const MAX_PHOTOS = 12;
const MAX_PHOTO_BYTES = 900_000;
const BATCH_SIZE = 40;

type PhotoPayload = {
  filename: string;
  contentType: string;
  /** base64 sans préfixe data: */
  contentBase64: string;
};

export async function POST(req: Request) {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;

  try {
    const body = await req.json();
    const tripId = String(body.tripId || "");
    const subject = String(body.subject || "").trim();
    const message = String(body.body || body.message || "").trim();
    const photos: PhotoPayload[] = Array.isArray(body.photos) ? body.photos : [];

    if (!tripId) return NextResponse.json({ error: "tripId requis" }, { status: 400 });
    if (!subject) return NextResponse.json({ error: "Sujet requis" }, { status: 400 });
    if (!message) return NextResponse.json({ error: "Message requis" }, { status: 400 });
    if (photos.length > MAX_PHOTOS) {
      return NextResponse.json({ error: `Maximum ${MAX_PHOTOS} photos par envoi.` }, { status: 400 });
    }

    const hit = await getJson<TravelsTrip>(`travels/${tripId}.json`);
    const trip = hit?.data;
    if (!trip?.data) return NextResponse.json({ error: "Dossier introuvable" }, { status: 404 });

    const access = await assertTravelsTripAccess(trip, { requireOwnerOrDirection: true });
    if (!access.ok) return NextResponse.json({ error: access.error }, { status: access.status });

    const participants = trip.data.participantEleves || [];
    if (participants.length === 0) {
      return NextResponse.json({ error: "Aucun élève sur la liste — composez d'abord la liste." }, { status: 400 });
    }

    const eleves = await loadElevesRegistry();
    const byIne = new Map(eleves.map((e) => [e.ine, e]));
    const emailSet = new Set<string>();
    for (const p of participants) {
      const full = byIne.get(p.ine);
      if (!full) continue;
      for (const mail of collectEleveParentEmails(full)) emailSet.add(mail);
    }
    const recipients = [...emailSet];
    if (recipients.length === 0) {
      return NextResponse.json(
        { error: "Aucun e-mail parent trouvé pour les élèves de la liste." },
        { status: 400 },
      );
    }

    const attachments: Array<{ filename: string; content: Buffer; contentType: string }> = [];
    for (let i = 0; i < photos.length; i++) {
      const ph = photos[i];
      const raw = String(ph.contentBase64 || "").replace(/\s/g, "");
      if (!raw) continue;
      const buf = Buffer.from(raw, "base64");
      if (buf.length > MAX_PHOTO_BYTES) {
        return NextResponse.json(
          { error: `Photo « ${ph.filename || i + 1} » trop volumineuse (max ~900 Ko compressée).` },
          { status: 400 },
        );
      }
      const ct = String(ph.contentType || "image/jpeg").toLowerCase();
      if (!ct.startsWith("image/")) {
        return NextResponse.json({ error: "Seules les images sont acceptées." }, { status: 400 });
      }
      attachments.push({
        filename: String(ph.filename || `photo_${i + 1}.jpg`).replace(/[^\w.\-]+/g, "_"),
        content: buf,
        contentType: ct,
      });
    }

    const smtp = await getTenantSmtpConfig();
    const transporter = smtp ? await createTenantTransporter() : null;
    if (!smtp || !transporter) {
      return NextResponse.json({ error: "SMTP non configuré" }, { status: 503 });
    }

    const userName = access.user.fullName || "Équipe pédagogique";
    const tripLabel = trip.data.title || trip.data.destination || tripId;
    const html = `
      <div style="font-family: sans-serif; line-height: 1.55; color: #334155; max-width: 560px;">
        <p>Bonjour,</p>
        <p>Message concernant la sortie <strong>${escapeHtml(String(tripLabel))}</strong>.</p>
        <div style="white-space: pre-wrap; margin: 16px 0; padding: 12px 14px; background: #f8fafc; border-radius: 8px; border: 1px solid #e2e8f0;">${escapeHtml(message)}</div>
        ${attachments.length ? `<p>${attachments.length} photo(s) en pièce jointe.</p>` : ""}
        <p style="font-size: 12px; color: #64748b;">Cet e-mail est envoyé par l'établissement. Merci de ne pas y répondre sur ce canal (communication unidirectionnelle).</p>
        <p>Cordialement,<br/>${escapeHtml(userName)}</p>
      </div>
    `;

    for (let i = 0; i < recipients.length; i += BATCH_SIZE) {
      const batch = recipients.slice(i, i + BATCH_SIZE);
      await sendMailWithTimeout(
        transporter,
        {
          from: `"Sorties scolaires" <${smtp.user}>`,
          bcc: batch,
          subject,
          text: [
            "Bonjour,",
            "",
            `Message concernant la sortie « ${tripLabel} ».`,
            "",
            message,
            "",
            attachments.length ? `${attachments.length} photo(s) en pièce jointe.` : "",
            "",
            "Cet e-mail est envoyé par l'établissement (communication unidirectionnelle).",
            "",
            `Cordialement,`,
            userName,
          ]
            .filter(Boolean)
            .join("\n"),
          html,
          attachments: attachments.length ? attachments : undefined,
        },
        120_000,
      );
    }

    const now = new Date().toISOString();
    const log: TravelsParentComLog = {
      id: `pc_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`,
      sentAt: now,
      sentBy: { userId: access.user.id, name: userName },
      subject,
      body: message,
      photoCount: attachments.length,
      recipientCount: recipients.length,
    };

    const updatedTrip: TravelsTrip = {
      ...trip,
      updatedAt: now,
      data: {
        ...trip.data,
        parentComLogs: [...(trip.data.parentComLogs || []), log],
      },
      history: [
        ...(Array.isArray(trip.history) ? trip.history : []),
        {
          date: now,
          user: userName,
          action: `Communication parents envoyée (${recipients.length} destinataire(s), ${attachments.length} photo(s))`,
          note: subject,
        },
      ],
    };

    await putJson(`travels/${tripId}.json`, updatedTrip);
    const indexHit = await getJson<TravelsTrip[]>("travels/index.json");
    const index = Array.isArray(indexHit?.data) ? indexHit.data : [];
    await putJson(
      "travels/index.json",
      index.map((t) => (t.id === tripId ? { ...t, updatedAt: now } : t)),
    );

    return NextResponse.json({
      success: true,
      trip: updatedTrip,
      recipientCount: recipients.length,
      photoCount: attachments.length,
    });
  } catch (e) {
    console.error("[send-parents]", e);
    const msg = e instanceof Error ? e.message : "Envoi impossible";
    return NextResponse.json({ error: msg }, { status: 500 });
  }
}

function escapeHtml(s: string) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
