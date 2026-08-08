import type { EleveConfig } from "@/app/lib/eleves-config";
import { getTemplateMeta } from "@/app/lib/document-templates/catalog";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

export function valuesFromEleve(
  templateId: DocumentTemplateId,
  eleve: EleveConfig,
): Record<string, string | boolean> {
  const meta = getTemplateMeta(templateId);
  if (!meta) return {};
  const out: Record<string, string | boolean> = {};
  for (const f of meta.fields) {
    if (!f.fromEleve) continue;
    if (f.fromEleve === "nom") out[f.key] = eleve.nom || "";
    if (f.fromEleve === "prenom") out[f.key] = eleve.prenom || "";
    if (f.fromEleve === "classe") out[f.key] = eleve.classe || "";
    if (f.fromEleve === "nomComplet") out[f.key] = `${eleve.prenom || ""} ${eleve.nom || ""}`.trim();
  }
  return out;
}

export function mergeTemplateValues(
  templateId: DocumentTemplateId,
  raw: Record<string, unknown>,
  eleve?: EleveConfig | null,
  opts?: { skipRequired?: boolean },
): Record<string, string | boolean> {
  const meta = getTemplateMeta(templateId);
  if (!meta) throw new Error("Modèle inconnu");
  const fromEleve = eleve ? valuesFromEleve(templateId, eleve) : {};
  const out: Record<string, string | boolean> = { ...fromEleve };

  for (const f of meta.fields) {
    if (!(f.key in raw) && f.key in out) continue;
    const v = raw[f.key];
    if (f.type === "checkbox") {
      out[f.key] = v === true || v === "true" || v === "on" || v === 1;
      continue;
    }
    if (v !== undefined && v !== null) out[f.key] = String(v).trim();
    else if (!(f.key in out)) out[f.key] = "";
  }

  if (!opts?.skipRequired) {
    for (const f of meta.fields) {
      if (!f.required) continue;
      if (f.type === "checkbox") continue;
      if (!String(out[f.key] || "").trim()) {
        throw new Error(`Champ requis manquant : ${f.label}`);
      }
    }
  }
  return out;
}

export function documentTitle(
  templateId: DocumentTemplateId,
  values: Record<string, string | boolean>,
): string {
  const nom = String(values.nom || "").trim();
  const prenom = String(values.prenom || "").trim();
  const who = [prenom, nom].filter(Boolean).join(" ") || "document";
  if (templateId === "certificat-scolarite") return `Certificat — ${who}`;
  if (templateId === "fiche-inscription") return `Inscription — ${who}`;
  if (templateId === "autorisation-sortie") {
    const titre = String(values.sortieTitre || "").trim();
    return titre ? `Autorisation — ${titre}` : `Autorisation — ${who}`;
  }
  if (templateId === "courrier-families") {
    const objet = String(values.objet || "").trim();
    return objet ? `Courrier — ${objet}` : "Courrier familles";
  }
  return who;
}
