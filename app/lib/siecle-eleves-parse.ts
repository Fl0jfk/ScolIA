import { buildEleveFolderName, type EleveConfig } from "@/app/lib/eleves-config";
import { isRegimeInterne } from "@/app/lib/eleve-regime";

function normalizeSiecleDate(raw: string): string {
  const s = raw.trim();
  const fr = s.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (fr) return `${fr[3]}-${fr[2]}-${fr[1]}`;
  const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (iso) return `${iso[1]}-${iso[2]}-${iso[3]}`;
  return "";
}

/** Parse ElevesSansAdresses.xml (BEE_ELEVES) → EleveConfig[] (regex, sans DOM). */
export function parseSiecleElevesXmlServer(xmlText: string): {
  eleves: EleveConfig[];
  internesCount: number;
  total: number;
} {
  // Next.js server : utiliser regex robuste sans DOM (évite dépendance linkedom)
  const eleves: EleveConfig[] = [];
  let internesCount = 0;
  const blocks = xmlText.split(/<ELEVE\b/i).slice(1);

  for (const block of blocks) {
    const chunk = block.includes("</ELEVE>") ? block.slice(0, block.indexOf("</ELEVE>")) : block;
    const tag = (name: string) => {
      const m = chunk.match(new RegExp(`<${name}[^>]*>([\\s\\S]*?)</${name}>`, "i"));
      return (m?.[1] || "").replace(/<!\[CDATA\[|\]\]>/g, "").trim();
    };
    const nom = tag("NOM_DE_FAMILLE");
    const prenom = tag("PRENOM");
    if (!nom || !prenom) continue;

    const ine = tag("ID_NATIONAL");
    const codeRegime = tag("CODE_REGIME");
    const codeSexe = tag("CODE_SEXE");
    const dateNaiss = tag("DATE_NAISS");
    const email = tag("MEL");
    const codeStructure = tag("CODE_STRUCTURE");
    const codeMef = tag("CODE_MEF");
    const sexe: "M" | "F" | undefined =
      codeSexe === "2" ? "F" : codeSexe === "1" ? "M" : undefined;
    const dateNaissance = normalizeSiecleDate(dateNaiss);
    const folderName = buildEleveFolderName(nom, prenom);

    if (isRegimeInterne(codeRegime)) internesCount += 1;

    eleves.push({
      ine,
      nom,
      prenom,
      folderName,
      ...(codeStructure ? { classe: codeStructure } : {}),
      ...(codeMef ? { mef: codeMef } : {}),
      ...(email ? { email } : {}),
      ...(dateNaissance ? { dateNaissance } : {}),
      ...(codeRegime ? { regime: codeRegime } : {}),
      ...(sexe ? { sexe } : {}),
    });
  }

  return { eleves, internesCount, total: eleves.length };
}
