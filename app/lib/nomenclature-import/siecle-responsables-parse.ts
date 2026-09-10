/**
 * Parse ResponsablesAvecAdresses.xml (BEE_RESPONSABLES).
 * Siècle met souvent PERSONNE_ID / ADRESSE_ID en **attribut** de la balise ouvrante
 * (pas en balise enfant) — cf. exports BEE réels.
 */
import {
  attrValue,
  extractSiecleElements,
  firstNonEmpty,
  tagValue,
} from "@/app/lib/nomenclature-import/siecle-xml-parse-utils";

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

function boolFromRaw(raw: string): boolean {
  const v = raw.trim().toLowerCase();
  return v === "1" || v === "true" || v === "o" || v === "oui";
}

function boolTag(block: string, tag: string): boolean {
  return boolFromRaw(tagValue(block, tag));
}

function idFromElement(
  attrs: string,
  inner: string,
  attrName: string,
  tagName: string,
): string {
  return firstNonEmpty(attrValue(attrs, attrName), tagValue(inner, tagName));
}

/** Parse ResponsablesAvecAdresses.xml (BEE_RESPONSABLES). */
export function parseSiecleResponsablesXml(xml: string): SiecleResponsablesParsed {
  const adresses: SiecleAdresseRow[] = [];
  for (const el of extractSiecleElements(xml, "ADRESSE")) {
    const adresseId = idFromElement(el.attrs, el.inner, "ADRESSE_ID", "ADRESSE_ID");
    if (!adresseId) continue;
    adresses.push({
      adresseId,
      ligne1: firstNonEmpty(
        tagValue(el.inner, "ADRESSE_1"),
        tagValue(el.inner, "ADRESSE1"),
        tagValue(el.inner, "LIGNE_1"),
        tagValue(el.inner, "LIGNE1_ADRESSE"),
        tagValue(el.inner, "LIGNE1"),
      ),
      codePostal: firstNonEmpty(tagValue(el.inner, "CODE_POSTAL"), tagValue(el.inner, "CP")),
      ville: firstNonEmpty(
        tagValue(el.inner, "VILLE"),
        tagValue(el.inner, "LIBELLE_VILLE"),
        tagValue(el.inner, "LIBELLE_POSTAL"),
      ),
    });
  }

  const personnes: SieclePersonneRow[] = [];
  for (const el of extractSiecleElements(xml, "PERSONNE")) {
    const personneId = idFromElement(el.attrs, el.inner, "PERSONNE_ID", "PERSONNE_ID");
    const nom = firstNonEmpty(tagValue(el.inner, "NOM"), tagValue(el.inner, "NOM_DE_FAMILLE"));
    const prenom = firstNonEmpty(tagValue(el.inner, "PRENOM"), tagValue(el.inner, "PRENOM_1"));
    if (!personneId || !nom || !prenom) continue;
    personnes.push({
      personneId,
      nom,
      prenom,
      email: firstNonEmpty(tagValue(el.inner, "MEL"), tagValue(el.inner, "EMAIL")),
      telephone: firstNonEmpty(
        tagValue(el.inner, "TEL"),
        tagValue(el.inner, "TELEPHONE"),
        tagValue(el.inner, "TEL_PORTABLE"),
        tagValue(el.inner, "TEL_PERSONNEL"),
        tagValue(el.inner, "TEL_DOMICILE"),
      ),
      adresseId: idFromElement(el.attrs, el.inner, "ADRESSE_ID", "ADRESSE_ID"),
    });
  }

  const liens: SiecleResponsableEleveRow[] = [];
  for (const el of extractSiecleElements(xml, "RESPONSABLE_ELEVE")) {
    const eleveId = firstNonEmpty(
      tagValue(el.inner, "ELEVE_ID"),
      attrValue(el.attrs, "ELEVE_ID"),
    );
    const personneId = firstNonEmpty(
      tagValue(el.inner, "PERSONNE_ID"),
      attrValue(el.attrs, "PERSONNE_ID"),
    );
    if (!eleveId || !personneId) continue;

    const niveau = firstNonEmpty(
      tagValue(el.inner, "NIVEAU_RESPONSABILITE"),
      attrValue(el.attrs, "NIVEAU_RESPONSABILITE"),
    );
    const contactPrioritaire =
      boolTag(el.inner, "A_CONTACTER_EN_PRIORITE") || niveau === "1";

    liens.push({
      eleveId,
      personneId,
      codeParente: firstNonEmpty(
        tagValue(el.inner, "CODE_PARENTE"),
        attrValue(el.attrs, "CODE_PARENTE"),
      ),
      payeur:
        boolTag(el.inner, "PAIE_FRAIS_SCOLAIRES") || boolTag(el.inner, "RESP_FINANCIER"),
      contactPrioritaire,
      heberge: boolTag(el.inner, "HEBERGE_ELEVE"),
    });
  }

  return { adresses, personnes, liens };
}
