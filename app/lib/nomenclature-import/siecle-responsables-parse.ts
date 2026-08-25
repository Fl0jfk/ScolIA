function extractBlocks(xml: string, tag: string): string[] {
  const re = new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)</${tag}>`, "gi");
  const out: string[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(xml))) {
    out.push(m[1] ?? "");
  }
  return out;
}

function tagValue(block: string, tag: string): string {
  const m = block.match(new RegExp(`<${tag}[^>]*>([^<]*)</${tag}>`, "i"));
  return (m?.[1] ?? "").trim();
}

function boolTag(block: string, tag: string): boolean {
  const v = tagValue(block, tag).toLowerCase();
  return v === "1" || v === "true" || v === "o" || v === "oui";
}

export type SiecleAdresseRow = {
  adresseId: string;
  ligne1: string;
  codePostal: string;
  ville: string;
};

export type SieclePersonneRow = {
  personneId: string;
  nom: string;
  prenom: string;
  email: string;
  telephone: string;
  adresseId: string;
};

export type SiecleResponsableEleveRow = {
  eleveId: string;
  personneId: string;
  codeParente: string;
  payeur: boolean;
  contactPrioritaire: boolean;
  heberge: boolean;
};

export type SiecleResponsablesParsed = {
  adresses: SiecleAdresseRow[];
  personnes: SieclePersonneRow[];
  liens: SiecleResponsableEleveRow[];
};

/** Parse ResponsablesAvecAdresses.xml (BEE_RESPONSABLES). */
export function parseSiecleResponsablesXml(xml: string): SiecleResponsablesParsed {
  const adresses: SiecleAdresseRow[] = [];
  for (const block of extractBlocks(xml, "ADRESSE")) {
    const adresseId = tagValue(block, "ADRESSE_ID");
    if (!adresseId) continue;
    adresses.push({
      adresseId,
      ligne1:
        tagValue(block, "ADRESSE_1") ||
        tagValue(block, "ADRESSE1") ||
        tagValue(block, "LIGNE_1") ||
        "",
      codePostal: tagValue(block, "CODE_POSTAL") || tagValue(block, "CP") || "",
      ville: tagValue(block, "VILLE") || tagValue(block, "LIBELLE_VILLE") || "",
    });
  }

  const personnes: SieclePersonneRow[] = [];
  for (const block of extractBlocks(xml, "PERSONNE")) {
    const personneId = tagValue(block, "PERSONNE_ID");
    const nom = tagValue(block, "NOM") || tagValue(block, "NOM_DE_FAMILLE");
    const prenom = tagValue(block, "PRENOM") || tagValue(block, "PRENOM_1");
    if (!personneId || !nom || !prenom) continue;
    personnes.push({
      personneId,
      nom,
      prenom,
      email: tagValue(block, "MEL") || tagValue(block, "EMAIL") || "",
      telephone:
        tagValue(block, "TEL") ||
        tagValue(block, "TELEPHONE") ||
        tagValue(block, "TEL_PORTABLE") ||
        tagValue(block, "TEL_DOMICILE") ||
        "",
      adresseId: tagValue(block, "ADRESSE_ID") || "",
    });
  }

  const liens: SiecleResponsableEleveRow[] = [];
  for (const block of extractBlocks(xml, "RESPONSABLE_ELEVE")) {
    const eleveId = tagValue(block, "ELEVE_ID");
    const personneId = tagValue(block, "PERSONNE_ID");
    if (!eleveId || !personneId) continue;
    liens.push({
      eleveId,
      personneId,
      codeParente: tagValue(block, "CODE_PARENTE") || "",
      payeur: boolTag(block, "PAIE_FRAIS_SCOLAIRES"),
      contactPrioritaire: boolTag(block, "A_CONTACTER_EN_PRIORITE"),
      heberge: boolTag(block, "HEBERGE_ELEVE"),
    });
  }

  return { adresses, personnes, liens };
}
