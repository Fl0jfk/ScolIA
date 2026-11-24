/**
 * PDF d'engagement : texte représentable avec la police intégrée de jsPDF (Helvetica, codage WinAnsi = Windows-1252).
 * Aucune police Unicode (TTF) n'est embarquée dans le dépôt et rien n'est téléchargé d'internet : on RESTREINT donc
 * proprement. Les lettres latines accentuées usuelles passent telles quelles ; les lettres latines étendues absentes de
 * WinAnsi sont ramenées à leur lettre de base (« ł » → « l ») ; tout le reste (cyrillique, arabe, chinois…) devient « ? »
 * et le texte est signalé comme « lossy » pour que l'appelant refuse la saisie avec un message clair.
 * Pure : aucune dépendance.
 */

/** Points de code de la plage 0x80-0x9F de Windows-1252 (hors contrôles). */
const WIN1252_EXTRA = new Set<number>([
  0x20ac, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039, 0x0152, 0x017d, 0x2018, 0x2019, 0x201c, 0x201d,
  0x2022, 0x2013, 0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x017e, 0x0178,
]);

const BASE_LETTER: Record<string, string> = {
  ł: "l",
  Ł: "L",
  đ: "d",
  Đ: "D",
  ı: "i",
  ħ: "h",
  Ħ: "H",
  ŧ: "t",
  Ŧ: "T",
};

export function isWinAnsiChar(ch: string): boolean {
  const cp = ch.codePointAt(0) ?? 0;
  if (cp === 0x09 || cp === 0x0a || cp === 0x0d) return true;
  if (cp >= 0x20 && cp <= 0x7e) return true;
  if (cp >= 0xa0 && cp <= 0xff) return true;
  return WIN1252_EXTRA.has(cp);
}

export type PdfSafeText = { text: string; lossy: boolean };

export function toPdfSafe(input: string): PdfSafeText {
  let lossy = false;
  let out = "";
  for (const ch of String(input ?? "").normalize("NFC")) {
    if (isWinAnsiChar(ch)) {
      out += ch;
      continue;
    }
    if (BASE_LETTER[ch]) {
      out += BASE_LETTER[ch];
      continue;
    }
    const base = ch.normalize("NFD").replace(/[̀-ͯ]/g, "");
    if (base && base !== ch && [...base].every((c) => isWinAnsiChar(c))) {
      out += base;
      continue;
    }
    // Espaces et marques invisibles : ignorés sans signaler de perte.
    if (/[​-‏⁠﻿]/.test(ch)) continue;
    out += "?";
    lossy = true;
  }
  return { text: out, lossy };
}

/** Vrai si le texte s'écrit sans perte dans le PDF (alphabet latin, accents compris). */
export function canRenderInPdf(input: string): boolean {
  return !toPdfSafe(input).lossy;
}

export const PDF_NAME_RESTRICTION_MESSAGE =
  "Votre nom contient des caractères que le PDF d'engagement ne sait pas encore écrire (alphabet latin, accents compris, uniquement). " +
  "Saisissez-le en lettres latines, ou demandez un coupon papier à l'organisateur du voyage.";

/** Applique `toPdfSafe` à toutes les chaînes d'une structure (objets et tableaux simples ; les Buffer sont laissés tels quels). */
export function pdfSafeDeep<T>(value: T): T {
  if (typeof value === "string") return toPdfSafe(value).text as unknown as T;
  if (Array.isArray(value)) return value.map((v) => pdfSafeDeep(v)) as unknown as T;
  if (value && typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) out[k] = pdfSafeDeep(v);
    return out as T;
  }
  return value;
}
