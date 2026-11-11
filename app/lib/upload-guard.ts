/** Garde minimale des dépôts de fichiers (NJ-38, D-053) : pas de contenu actif, taille bornée. */
export const UPLOAD_MAX_BYTES = 25 * 1024 * 1024;

const RISKY_MIME = /^(text\/html|application\/xhtml\+xml|image\/svg\+xml|text\/xml|application\/xml|text\/javascript|application\/javascript|application\/x-msdownload|application\/x-sh|application\/x-httpd-php)\b/i;
const RISKY_EXT = /\.(html?|xhtml|svg|xml|js|mjs|exe|bat|cmd|sh|php|jar|msi|scr|com|vbs|ps1)$/i;

export function isRiskyUpload(fileName: string, mime: string): boolean {
  return RISKY_MIME.test(String(mime || "")) || RISKY_EXT.test(String(fileName || "").trim());
}
