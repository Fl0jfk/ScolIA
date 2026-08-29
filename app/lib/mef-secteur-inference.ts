import type { Secteur } from "@/app/lib/onedrive-eleves";

/** Infère le secteur scolaire à partir d'un code ou libellé MEF Siècle. */
export function inferSecteurFromMef(code: string, libelle: string): Secteur | null {
  const hay = `${code} ${libelle}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase();

  if (
    /\b(2NDE|1RE|1ERE|TERMINALE|CAP|BP|BAC|PROFESSIONNEL|LYCEE|LYCEEN|SECONDE)\b/.test(hay) ||
    /\b\d{4}(001|002|003|010|011)\b/.test(hay)
  ) {
    return "lycee";
  }
  if (
    /\b(6E|5E|4E|3E|COLLEGE|COLLÈGE|CYCLE 4|CYCLE4|CYCLE 3|CYCLE3)\b/.test(hay) ||
    /\b\d{4}(100|200|300|400)\b/.test(hay)
  ) {
    return "college";
  }
  if (
    /\b(CP|CE1|CE2|CM1|CM2|MATERNELLE|ELEMENTAIRE|ÉLÉMENTAIRE|ECOLE|CYCLE 2|CYCLE2|CYCLE 1)\b/.test(
      hay,
    )
  ) {
    return "ecole";
  }
  return null;
}
