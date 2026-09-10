import { NextRequest, NextResponse } from "next/server";
import { requireMessagingContext } from "@/app/lib/messaging/access";
import {
  isAllowedMime,
  maxBytesForMime,
  messageTypeFromMime,
} from "@/app/lib/messaging/constants";
import { assertParticipant, MessagingError } from "@/app/lib/messaging/service";
import { putObject, getSignedReadUrl } from "@/app/lib/s3-storage";
import { sanitizeS3FileName } from "@/app/lib/s3-path";

export const maxDuration = 60;

export async function POST(req: NextRequest) {
  const gate = await requireMessagingContext();
  if (!gate.ok) return gate.response;

  try {
    const formData = await req.formData();
    const file = formData.get("file");
    const conversationId = String(formData.get("conversationId") ?? "").trim();

    if (!(file instanceof File)) {
      return NextResponse.json({ error: "Aucun fichier trouvé." }, { status: 400 });
    }
    if (!conversationId) {
      return NextResponse.json({ error: "Conversation requise." }, { status: 400 });
    }

    await assertParticipant(gate.ctx.etablissementId, conversationId, gate.ctx.userId);

    const mime = file.type || "application/octet-stream";
    if (!isAllowedMime(mime)) {
      return NextResponse.json(
        { error: `Format non accepté (${mime}).` },
        { status: 400 },
      );
    }

    const maxBytes = maxBytesForMime(mime);
    if (file.size > maxBytes) {
      return NextResponse.json(
        {
          error: `Fichier trop volumineux (max ${Math.round(maxBytes / (1024 * 1024))} Mo).`,
        },
        { status: 400 },
      );
    }

    const buffer = Buffer.from(await file.arrayBuffer());
    const safeName = sanitizeS3FileName(file.name).replace(/\s+/g, "_");
    const rel = `messaging/${gate.ctx.etablissementId}/${conversationId}/${Date.now()}-${safeName}`;
    const s3Key = await putObject(rel, buffer, mime);
    const url = await getSignedReadUrl(s3Key, 3600);

    return NextResponse.json({
      id: crypto.randomUUID(),
      s3Key,
      mime,
      size: file.size,
      fileName: file.name,
      width: null,
      height: null,
      url,
      suggestedType: messageTypeFromMime(mime),
    });
  } catch (error) {
    if (error instanceof MessagingError) {
      return NextResponse.json(
        { error: error.message, code: error.code },
        { status: error.status },
      );
    }
    console.error("[messaging/upload]", error);
    return NextResponse.json({ error: "Échec de l'upload." }, { status: 500 });
  }
}
