import { chatUploadPrefix } from "@/app/lib/brain-ai/chat-attachments";
import {
  FILE_MIME,
  FileRenderError,
  normalizeSpec,
  renderFile,
  safeBaseName,
} from "@/app/lib/brain-ai/file-render";
import type { BrainToolCtx, BrainToolResult } from "@/app/lib/brain-ai/types";

const DOWNLOAD_TTL_SECONDS = 15 * 60;

/**
 * Génère un fichier Word / PDF / Excel à partir d'un contenu structuré, le dépose
 * dans le dossier S3 de l'utilisateur et renvoie un bouton de téléchargement
 * (URL pré-signée courte). Ne modifie rien dans l'ENT. Aucun contenu n'est journalisé.
 */
export async function handleCreateFile(
  ctx: BrainToolCtx,
  args: Record<string, unknown>,
): Promise<BrainToolResult> {
  let spec;
  try {
    spec = normalizeSpec({ format: args.format, title: args.title, blocks: args.blocks });
  } catch (e) {
    return { ok: false, error: e instanceof FileRenderError ? e.message : "Contenu du fichier invalide." };
  }

  let buf: Buffer;
  try {
    buf = await renderFile(spec);
  } catch (e) {
    if (e instanceof FileRenderError) return { ok: false, error: e.message };
    console.error("[brain-ai/create_file] rendu impossible", { format: spec.format, blocks: spec.blocks.length });
    return { ok: false, error: "Génération du fichier impossible." };
  }

  if (!ctx.userId) return { ok: false, error: "Connexion requise.", code: "AUTH_REQUIRED" };
  const baseName = safeBaseName(args.fileName || spec.title);
  const fileName = `${baseName}.${spec.format}`;
  const key = `${chatUploadPrefix(ctx.userId)}/generated/${Date.now()}-${baseName}.${spec.format}`;

  try {
    const [{ PutObjectCommand, GetObjectCommand }, { getSignedUrl }, { getTenantDataS3Client }, { getBucketName }, { s3Key }] =
      await Promise.all([
        import("@aws-sdk/client-s3"),
        import("@aws-sdk/s3-request-presigner"),
        import("@/app/lib/s3-clients"),
        import("@/app/lib/s3-storage"),
        import("@/app/lib/s3-path"),
      ]);
    const s3 = await getTenantDataS3Client();
    const bucket = await getBucketName();
    await s3.send(
      new PutObjectCommand({
        Bucket: bucket,
        Key: s3Key(key),
        Body: buf,
        ContentType: FILE_MIME[spec.format],
        ContentLength: buf.length,
      }),
    );
    const href = await getSignedUrl(
      s3,
      new GetObjectCommand({
        Bucket: bucket,
        Key: s3Key(key),
        ResponseContentType: FILE_MIME[spec.format],
        ResponseContentDisposition: `attachment; filename="${fileName}"; filename*=UTF-8''${encodeURIComponent(fileName)}`,
      }),
      { expiresIn: DOWNLOAD_TTL_SECONDS },
    );
    console.info("[brain-ai/create_file]", { format: spec.format, blocks: spec.blocks.length, bytes: buf.length });
    return {
      ok: true,
      data: {
        fileName,
        format: spec.format,
        sizeBytes: buf.length,
        ctas: [{ label: `Télécharger ${fileName}`, href }],
        note: "Fichier prêt : le bouton de téléchargement est affiché sous la réponse (lien valable 15 minutes).",
      },
      summaryFr: `Fichier ${fileName} créé.`,
    };
  } catch {
    console.error("[brain-ai/create_file] stockage impossible", { format: spec.format, bytes: buf.length });
    return { ok: false, error: "Enregistrement du fichier impossible pour le moment." };
  }
}
