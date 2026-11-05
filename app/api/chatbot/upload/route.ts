import { NextResponse } from "next/server";
import { PutObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import {
  CHAT_MAX_FILE_BYTES,
  CHAT_MAX_IMAGE_BYTES,
  chatAttachmentKind,
  chatUploadPrefix,
} from "@/app/lib/brain-ai/chat-attachments";
import { safeCurrentUser } from "@/app/lib/intranet-session";
import { getTenantDataS3Client } from "@/app/lib/s3-clients";
import { getBucketName } from "@/app/lib/s3-storage";
import { s3Key } from "@/app/lib/s3-path";

function safeFileName(name: string): string {
  return name.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 80) || "fichier";
}

/**
 * Prépare l'envoi d'une pièce jointe ScolIA (PDF, photo, texte).
 * Le fichier est rangé dans le dossier de l'utilisateur : le chat refuse de lire
 * un fichier hors de ce dossier.
 */
export async function POST(req: Request) {
  // Même identifiant que /api/chatbot, qui vérifie le dossier du fichier.
  const user = await safeCurrentUser();
  if (!user) {
    return NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 });
  }

  try {
    const body = await req.json();
    const fileName = String(body.fileName || "fichier");
    const contentType = String(body.contentType || "").trim().toLowerCase();
    const kind = chatAttachmentKind(contentType);
    if (!kind) {
      return NextResponse.json(
        { error: "Type de fichier non pris en charge (PDF, photo ou fichier texte)." },
        { status: 400 },
      );
    }
    const size = Number(body.size);
    const maxBytes = kind === "image" ? CHAT_MAX_IMAGE_BYTES : CHAT_MAX_FILE_BYTES;
    if (!Number.isFinite(size) || size <= 0 || size > maxBytes) {
      return NextResponse.json(
        { error: `Fichier trop lourd (max ${Math.round(maxBytes / 1024 / 1024)} Mo).` },
        { status: 400 },
      );
    }

    const random = Math.random().toString(36).slice(2, 8);
    const relative = `${chatUploadPrefix(user.id)}/${Date.now()}-${random}-${safeFileName(fileName)}`;
    const s3Client = await getTenantDataS3Client();
    const bucket = await getBucketName();
    const uploadUrl = await getSignedUrl(
      s3Client,
      new PutObjectCommand({
        Bucket: bucket,
        Key: s3Key(relative),
        ContentType: contentType,
        // Taille signée : le stockage refuse un fichier d'une autre taille.
        ContentLength: size,
      }),
      { expiresIn: 600 },
    );

    return NextResponse.json({
      uploadUrl,
      key: relative,
      fileName: safeFileName(fileName),
      contentType,
    });
  } catch (e) {
    console.error("[chatbot/upload]", e);
    return NextResponse.json({ error: "Préparation de l'upload impossible." }, { status: 500 });
  }
}
