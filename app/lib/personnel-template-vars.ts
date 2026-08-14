import { PERSONNEL_CATEGORY_LABELS, type PersonnelRecord } from "@/app/lib/personnel-types";
import type { PersonnelProfile } from "@/app/lib/personnel-profile";
import { defaultPersonnelProfile } from "@/app/lib/personnel-profile";

type PersonnelTemplateVarDef = {
  key: string;
  label: string;
  group: string;
};

/** Variables disponibles pour modèles de documents (contrat, Sécu, prévoyance…). */
export const PERSONNEL_TEMPLATE_VAR_DEFS: PersonnelTemplateVarDef[] = [
  { key: "prenom", label: "Prénom", group: "Identité" },
  { key: "nom", label: "Nom", group: "Identité" },
  { key: "nom_complet", label: "Nom complet", group: "Identité" },
  { key: "nom_naissance", label: "Nom de naissance", group: "Identité" },
  { key: "email", label: "Email professionnel", group: "Identité" },
  { key: "poste", label: "Poste / fonction", group: "Emploi" },
  { key: "categorie", label: "Catégorie RH", group: "Emploi" },
  { key: "date_entree", label: "Date d'entrée", group: "Emploi" },
  { key: "etablissement", label: "Établissement", group: "Emploi" },
  { key: "type_contrat", label: "Type de contrat", group: "Contrat" },
  { key: "date_fin_contrat", label: "Date fin de contrat", group: "Contrat" },
  { key: "temps_travail", label: "Temps de travail (%)", group: "Contrat" },
  { key: "classification", label: "Classification", group: "Contrat" },
  { key: "coefficient", label: "Coefficient", group: "Contrat" },
  { key: "salaire_brut", label: "Salaire brut mensuel", group: "Contrat" },
  { key: "matricule", label: "Matricule interne", group: "Administratif" },
  { key: "date_naissance", label: "Date de naissance", group: "État civil" },
  { key: "lieu_naissance", label: "Lieu de naissance", group: "État civil" },
  { key: "nationalite", label: "Nationalité", group: "État civil" },
  { key: "sexe", label: "Sexe", group: "État civil" },
  { key: "situation_familiale", label: "Situation familiale", group: "État civil" },
  { key: "nombre_enfants", label: "Nombre d'enfants", group: "État civil" },
  { key: "numero_securite_sociale", label: "N° sécurité sociale (NIR)", group: "Sécu / prévoyance" },
  { key: "adresse_ligne1", label: "Adresse ligne 1", group: "Coordonnées" },
  { key: "adresse_ligne2", label: "Adresse ligne 2", group: "Coordonnées" },
  { key: "code_postal", label: "Code postal", group: "Coordonnées" },
  { key: "ville", label: "Ville", group: "Coordonnées" },
  { key: "pays", label: "Pays", group: "Coordonnées" },
  { key: "adresse_complete", label: "Adresse complète", group: "Coordonnées" },
  { key: "telephone", label: "Téléphone", group: "Coordonnées" },
  { key: "telephone_mobile", label: "Mobile", group: "Coordonnées" },
  { key: "iban", label: "IBAN", group: "Banque" },
  { key: "bic", label: "BIC", group: "Banque" },
  { key: "contact_urgence_nom", label: "Contact urgence — nom", group: "Urgence" },
  { key: "contact_urgence_tel", label: "Contact urgence — téléphone", group: "Urgence" },
];

function fmtDate(iso?: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : d.toLocaleDateString("fr-FR");
}

function contractLabel(t?: PersonnelProfile["contractType"]): string {
  const map: Record<string, string> = {
    cdi: "CDI",
    cdd: "CDD",
    cddu: "CDDU",
    interim: "Intérim",
    stage: "Stage",
    autre: "Autre",
  };
  return t ? map[t] || t : "";
}

function fullAddress(p: PersonnelProfile): string {
  return [p.addressLine1, p.addressLine2, [p.postalCode, p.city].filter(Boolean).join(" "), p.country]
    .filter(Boolean)
    .join(", ");
}

/** Construit le dictionnaire de variables pour fusion de documents. */
function buildPersonnelTemplateVars(record: PersonnelRecord): Record<string, string> {
  const p = record.profile || defaultPersonnelProfile();

  const vars: Record<string, string> = {
    prenom: record.firstName,
    nom: record.lastName,
    nom_complet: record.displayName,
    nom_naissance: p.birthName || "",
    email: record.email,
    poste: record.jobTitle || "",
    categorie: PERSONNEL_CATEGORY_LABELS[record.category],
    date_entree: fmtDate(record.hireDate),
    etablissement: p.establishment || "",
    type_contrat: contractLabel(p.contractType),
    date_fin_contrat: fmtDate(p.contractEndDate),
    temps_travail: p.workTimePercent != null ? String(p.workTimePercent) : "",
    classification: p.classification || "",
    coefficient: p.coefficient || "",
    salaire_brut: p.grossMonthlySalary || "",
    matricule: p.internalId || "",
    date_naissance: fmtDate(p.birthDate),
    lieu_naissance: p.birthPlace || "",
    nationalite: p.nationality || "",
    sexe: p.gender || "",
    situation_familiale: p.maritalStatus || "",
    nombre_enfants: p.childrenCount != null ? String(p.childrenCount) : "",
    numero_securite_sociale: p.socialSecurityNumber || "",
    adresse_ligne1: p.addressLine1 || "",
    adresse_ligne2: p.addressLine2 || "",
    code_postal: p.postalCode || "",
    ville: p.city || "",
    pays: p.country || "",
    adresse_complete: fullAddress(p),
    telephone: p.phone || "",
    telephone_mobile: p.phoneMobile || "",
    iban: p.iban || "",
    bic: p.bic || "",
    contact_urgence_nom: p.emergencyContactName || "",
    contact_urgence_tel: p.emergencyContactPhone || "",
  };

  for (const [k, v] of Object.entries(vars)) {
    vars[`collaborateur.${k}`] = v;
  }

  return vars;
}
