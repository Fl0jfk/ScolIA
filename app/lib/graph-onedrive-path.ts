/** Chemins OneDrive pour l’API Graph (`/me/drive/root:/…`). */

export const GRAPH_API_BASE = "https://graph.microsoft.com/v1.0";

function encodeGraphDrivePath(path: string): string {
  return String(path || "")
    .split("/")
    .filter(Boolean)
    .map((seg) => encodeURIComponent(seg))
    .join("/");
}

/** `extra` : `""`, `"/content"`, `"/children?…"`, `"?$select=webUrl"`. */
export function graphDriveRootItemUrl(itemPath: string, extra = ""): string {
  return `${GRAPH_API_BASE}/me/drive/root:/${encodeGraphDrivePath(itemPath)}:${extra}`;
}
