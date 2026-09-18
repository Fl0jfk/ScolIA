/** Upload S3 + ingestion OCR/IA des pièces d’inscription (élève déjà connu). */

export type InscriptionUploadItemResult = {
  fileName: string;
  ok: boolean;
  title?: string;
  kindLabel?: string;
  warning?: string;
  error?: string;
};

export async function uploadInscriptionDocuments(opts: {
  eleveId: string;
  files: File[];
  onProgress?: (done: number, total: number, currentFileName: string) => void;
}): Promise<InscriptionUploadItemResult[]> {
  const { eleveId, files, onProgress } = opts;
  const results: InscriptionUploadItemResult[] = [];
  const total = files.length;

  for (let i = 0; i < files.length; i++) {
    const file = files[i]!;
    onProgress?.(i, total, file.name);
    try {
      const prep = await fetch(`/api/eleves/${encodeURIComponent(eleveId)}/documents/upload`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          fileName: file.name,
          fileType: file.type || "application/octet-stream",
          size: file.size,
        }),
      });
      const prepJ = (await prep.json().catch(() => ({}))) as {
        error?: string;
        uploadUrl?: string;
        s3Key?: string;
        fileUrl?: string;
      };
      if (!prep.ok || !prepJ.uploadUrl || !prepJ.s3Key) {
        throw new Error(prepJ.error || "Préparation upload impossible");
      }

      const put = await fetch(prepJ.uploadUrl, {
        method: "PUT",
        headers: { "Content-Type": file.type || "application/octet-stream" },
        body: file,
      });
      if (!put.ok) throw new Error(`Envoi fichier échoué (${put.status})`);

      const ingest = await fetch(
        `/api/eleves/${encodeURIComponent(eleveId)}/documents/inscription-ingest`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            s3Key: prepJ.s3Key,
            fileName: file.name,
            mimeType: file.type || "application/octet-stream",
            fileUrl: prepJ.fileUrl ?? null,
          }),
        },
      );
      const ingestJ = (await ingest.json().catch(() => ({}))) as {
        error?: string;
        document?: { title?: string };
        classification?: { kindLabel?: string; warning?: string };
      };
      if (!ingest.ok) {
        throw new Error(ingestJ.error || "Analyse / enregistrement impossible");
      }

      results.push({
        fileName: file.name,
        ok: true,
        title: ingestJ.document?.title,
        kindLabel: ingestJ.classification?.kindLabel,
        warning: ingestJ.classification?.warning,
      });
    } catch (e) {
      results.push({
        fileName: file.name,
        ok: false,
        error: e instanceof Error ? e.message : "Erreur upload",
      });
    }
  }

  onProgress?.(total, total, "");
  return results;
}
