/** Caractères mal rendus par Helvetica dans jsPDF (espaces fines, flèches, tirets Unicode). */
function pdfSanitizeText(text: string): string {
  return text
    .replace(/\u00a0/g, " ")
    .replace(/\u202f/g, " ")
    .replace(/\u2009/g, " ")
    .replace(/\u2212/g, "-")
    .replace(/[\u2013\u2014]/g, "-")
    .replace(/\u2192/g, " au ")
    .replace(/\u2019/g, "'");
}

export function pdfFormatDateFr(value: string | Date): string {
  const d = typeof value === "string" ? new Date(value) : value;
  if (Number.isNaN(d.getTime())) return "—";
  const dd = String(d.getDate()).padStart(2, "0");
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const yyyy = d.getFullYear();
  return `${dd}/${mm}/${yyyy}`;
}

export function pdfFormatDateFrLong(now = new Date()): string {
  const months = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
  ];
  return `${now.getDate()} ${months[now.getMonth()]} ${now.getFullYear()}`;
}

/** Format nombre pour jsPDF (évite les espaces insécables / fines qui s'affichent en « / »). */
export function pdfFormatEuroAmount(n: number | null | undefined, fractionDigits = 2): string {
  if (n == null || !Number.isFinite(n)) return "—";
  const fixed = n.toFixed(fractionDigits);
  const [intPart, decPart] = fixed.split(".");
  const grouped = intPart.replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  const body = decPart != null && fractionDigits > 0 ? `${grouped},${decPart}` : grouped;
  return `${body} €`;
}

export function pdfFormatWholeEuro(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return "—";
  const intPart = String(Math.round(n));
  return `${intPart.replace(/\B(?=(\d{3})+(?!\d))/g, " ")} €`;
}
