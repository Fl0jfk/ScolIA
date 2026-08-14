/** Chemins S3 à la racine du bucket (une instance = un bucket dédié). */

export function s3Key(relativePath: string): string {
  return relativePath.replace(/^\/+/, "");
}

/**
 * Clé relative sûre : pas de segments `.` / `..`, pas vide.
 * Ne change pas le comportement de `s3Key` (aucun throw).
 */
export function isSafeS3RelativeKey(key: string): boolean {
  const n = s3Key(key);
  if (!n || n.length > 2048) return false;
  if (n.includes("\0")) return false;
  if (n.split("/").some((seg) => seg === ".." || seg === ".")) return false;
  return true;
}

export function keyHasAllowedPrefix(key: string, prefixes: readonly string[]): boolean {
  const n = s3Key(key);
  if (!isSafeS3RelativeKey(n)) return false;
  return prefixes.some((p) => {
    const exact = p.replace(/\/+$/, "");
    const prefix = exact ? `${exact}/` : "";
    return n === exact || (prefix ? n.startsWith(prefix) : false);
  });
}

/** Nom de fichier pour une clé S3 : pas de séparateurs de chemin. */
export function sanitizeS3FileName(name: string): string {
  const base =
    String(name || "file")
      .replace(/\\/g, "/")
      .split("/")
      .filter((seg) => seg && seg !== "." && seg !== "..")
      .pop()
      ?.replace(/\0/g, "")
      .slice(0, 180) || "file";
  return base;
}
