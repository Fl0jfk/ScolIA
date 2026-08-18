/** Upload document séjour via l’API (stockage serveur, plus fiable que le PUT présigné navigateur). */
export async function uploadTravelDocument(
  file: File | Blob,
  fileName: string,
): Promise<{ fileUrl: string; s3Key: string }> {
  const form = new FormData();
  form.append("file", file, fileName);
  const res = await fetch("/api/travels/upload", { method: "POST", body: form });
  const data = (await res.json().catch(() => ({}))) as {
    fileUrl?: string;
    s3Key?: string;
    error?: string;
  };
  if (!res.ok || !data.fileUrl || !data.s3Key) {
    throw new Error(data.error || "Impossible d'enregistrer le fichier sur le serveur.");
  }
  return { fileUrl: data.fileUrl, s3Key: data.s3Key };
}
