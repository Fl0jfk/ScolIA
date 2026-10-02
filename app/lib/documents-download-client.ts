/**
 * Téléchargement cloud (navigateur) — remplace l’édition Collabora en ligne.
 */

export type CloudDownloadTarget = {
  scope: string;
  path: string;
  shareId?: string | null;
  fileShareId?: string | null;
  /** Nom suggéré pour l’attribut download. */
  fileName?: string;
};

function basenameFromPath(path: string): string {
  const cleaned = path.replace(/\/+$/, "");
  const parts = cleaned.split("/");
  return parts[parts.length - 1] || "document";
}

/** Déclenche le téléchargement d’un fichier cloud via URL présignée. */
export async function downloadCloudDocument(
  target: CloudDownloadTarget,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const params = new URLSearchParams({
    scope: target.scope,
    path: target.path,
    download: "1",
  });
  if (target.shareId) params.set("shareId", target.shareId);
  if (target.fileShareId) {
    params.set("scope", "fileshare");
    params.set("fileShareId", target.fileShareId);
    // get-url résout aussi via parseFileShareIdFromRel sur le path virtuel.
  }

  const res = await fetch(`/api/documents/get-url?${params.toString()}`);
  const data = (await res.json().catch(() => ({}))) as { url?: string; error?: string };
  if (!res.ok || !data.url) {
    return { ok: false, error: data.error || "Téléchargement impossible." };
  }

  const a = document.createElement("a");
  a.href = data.url;
  a.download = target.fileName || basenameFromPath(target.path);
  a.rel = "noopener";
  a.target = "_blank";
  document.body.appendChild(a);
  a.click();
  a.remove();
  return { ok: true };
}
