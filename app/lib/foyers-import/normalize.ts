/**
 * Normalisation pure de l'import Excel des foyers (aucune dépendance serveur).
 */

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** Nettoie un texte (trim + espaces multiples) sans changer la casse. */
export function cleanText(v: unknown): string {
  if (v === null || v === undefined) return "";
  return String(v).replace(/\s+/g, " ").trim();
}

/** Clé de comparaison : sans accents, minuscule, séparateurs (- ' .) = espace. */
export function normName(v: unknown): string {
  return cleanText(v)
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/œ/g, "oe")
    .replace(/æ/g, "ae")
    .replace(/[-'’`.]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** E-mail minuscule valide, sinon null. */
export function normEmail(v: unknown): string | null {
  const e = cleanText(v).toLowerCase();
  return e && EMAIL_RE.test(e) ? e : null;
}

/**
 * Téléphone FR : chiffres seuls, +33 / 0033 → 0, 9 chiffres (zéro perdu par Excel) → 0 + chiffres.
 * Retourne null si inexploitable. Un numéro international non FR est conservé en chiffres.
 */
export function normPhone(v: unknown): string | null {
  let d = cleanText(v).replace(/[^\d+]/g, "");
  if (!d) return null;
  if (d.startsWith("+33")) d = "0" + d.slice(3);
  else if (d.startsWith("0033")) d = "0" + d.slice(4);
  d = d.replace(/\D/g, "");
  if (d.length === 9 && /^[1-9]/.test(d)) d = "0" + d;
  if (/^0\d{9}$/.test(d)) return d;
  if (d.length >= 8 && d.length <= 15) return d;
  return null;
}

function isoFromParts(y: number, m: number, d: number): string | null {
  if (y < 1900 || y > 2100 || m < 1 || m > 12 || d < 1 || d > 31) return null;
  const dt = new Date(Date.UTC(y, m - 1, d));
  if (dt.getUTCFullYear() !== y || dt.getUTCMonth() !== m - 1 || dt.getUTCDate() !== d) return null;
  return `${String(y).padStart(4, "0")}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

/** Numéro de série Excel (système 1900) → ISO. */
export function excelSerialToIso(serial: number): string | null {
  if (!Number.isFinite(serial) || serial < 1 || serial > 80000) return null;
  const ms = Math.round((serial - 25569) * 86400 * 1000);
  const dt = new Date(ms);
  return isoFromParts(dt.getUTCFullYear(), dt.getUTCMonth() + 1, dt.getUTCDate());
}

/** Date de naissance : numéro de série, Date, « JJ/MM/AAAA » ou « AAAA-MM-JJ ». */
export function parseBirthDate(v: unknown): string | null {
  if (v === null || v === undefined || v === "") return null;
  if (v instanceof Date) {
    if (Number.isNaN(v.getTime())) return null;
    return isoFromParts(v.getFullYear(), v.getMonth() + 1, v.getDate());
  }
  if (typeof v === "number") return excelSerialToIso(v);
  const s = cleanText(v);
  if (!s) return null;
  if (/^\d{4,6}(\.\d+)?$/.test(s)) return excelSerialToIso(Number(s));
  let m = /^(\d{1,2})[/.\-](\d{1,2})[/.\-](\d{4})$/.exec(s);
  if (m) return isoFromParts(Number(m[3]), Number(m[2]), Number(m[1]));
  m = /^(\d{4})-(\d{2})-(\d{2})/.exec(s);
  if (m) return isoFromParts(Number(m[1]), Number(m[2]), Number(m[3]));
  return null;
}

/**
 * Autorité parentale déduite du lien de parenté (le fichier ne la donne pas).
 * Père, Mère, Famille d'accueil, Organisme, Oncle → true ; Beau-père, Belle-mère, Autre → false.
 * Lien inconnu → false (signalé à l'aperçu).
 */
export function autoriteFromLien(lien: string): { value: boolean; known: boolean } {
  const n = normName(lien);
  if (!n) return { value: false, known: false };
  if (n === "pere" || n === "mere" || n === "oncle") return { value: true, known: true };
  if (n.startsWith("famille d accueil") || n.startsWith("organisme")) return { value: true, known: true };
  if (n === "beau pere" || n === "belle mere" || n === "autre") return { value: false, known: true };
  return { value: false, known: false };
}
