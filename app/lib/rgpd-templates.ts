import "server-only";

import type { RgpdQuestionnaireAnswers, RgpdTemplateSection } from "@/app/lib/rgpd-types";
import {
  formatPlatformDpaBullet,
  getEnabledPlatformDpas,
} from "@/app/lib/rgpd-platform-dpas";
import {
  RGPD_PROCESSING_FICHES,
  buildFullRegisterSections,
  buildFicheSections,
  getRgpdProcessingFiche,
} from "@/app/lib/rgpd-processing-fiches";
import { rgpdEstablishmentDisplayName } from "@/app/lib/rgpd-establishment-helpers";
import { loadAppConfig } from "@/app/lib/app-config";

export type { RgpdTemplateSection } from "@/app/lib/rgpd-types";

type RgpdTemplateContext = {
  establishmentName: string;
  establishmentKinds: string;
  dpdName: string;
  directionReferent: string;
  subprocessorsList: string;
  platformDpaLines: string[];
  audiencesText: string;
  sensitiveText: string;
  date: string;
};

function buildTemplateContext(answers: RgpdQuestionnaireAnswers): RgpdTemplateContext {
  const kinds = answers.establishmentKinds.length
    ? answers.establishmentKinds.join(", ")
    : "établissement scolaire";
  const groupLabel = answers.isGroup ? " — groupe scolaire" : "";

  const sp: string[] = [];
  if (answers.subprocessors.ent) sp.push(`ENT : ${answers.subprocessors.entName || "Pronote / École Directe"}`);
  if (answers.subprocessors.microsoft365) sp.push("Microsoft 365");
  if (answers.subprocessors.scola) sp.push("Scola (intranet)");
  if (answers.subprocessors.aws || answers.subprocessors.hosting) sp.push("AWS (hébergement cloud)");
  if (answers.subprocessors.mistralAi) sp.push("Mistral AI");
  if (answers.subprocessors.otherSaas && answers.subprocessors.otherSaasList) {
    sp.push(answers.subprocessors.otherSaasList);
  }

  const platformDpas = getEnabledPlatformDpas(answers);
  for (const p of platformDpas) {
    if (!sp.some((s) => s.toLowerCase().includes(p.name.toLowerCase().split(" ")[0]))) {
      sp.push(p.name);
    }
  }

  const platformDpaLines = platformDpas.map(formatPlatformDpaBullet);

  const aud: string[] = [];
  if (answers.audiences.students) aud.push("élèves");
  if (answers.audiences.parents) aud.push("parents");
  if (answers.audiences.staff) aud.push("personnel");
  if (answers.audiences.prospects) aud.push("prospects (portes ouvertes)");
  if (answers.audiences.alumni) aud.push("anciens élèves");

  const sens: string[] = [];
  if (answers.sensitiveProcessing.photosVideos) sens.push("photos/vidéos");
  if (answers.sensitiveProcessing.publications) sens.push("publications");
  if (answers.sensitiveProcessing.boarding) sens.push("internat");
  if (answers.sensitiveProcessing.healthData) sens.push("données de santé");
  if (answers.sensitiveProcessing.videoSurveillance) sens.push("vidéosurveillance");
  if (answers.sensitiveProcessing.biometrics) sens.push("biométrie");

  return {
    establishmentName: rgpdEstablishmentDisplayName(answers, kinds + groupLabel),
    establishmentKinds: kinds + groupLabel,
    dpdName: answers.dpdDesignated
      ? (answers.dpdName || "DPD désigné") +
        (answers.dpdEmail ? ` (${answers.dpdEmail})` : "")
      : "Non désigné",
    directionReferent: answers.directionReferent || "Direction de l'établissement",
    subprocessorsList: sp.length ? sp.join(" ; ") : "À compléter",
    platformDpaLines,
    audiencesText: aud.length ? aud.join(", ") : "élèves, parents, personnel",
    sensitiveText: sens.length ? sens.join(", ") : "traitements courants",
    date: new Date().toLocaleDateString("fr-FR"),
  };
}

export async function getRgpdEstablishmentLabel(): Promise<string> {
  const config = await loadAppConfig();
  const names = config.establishments?.map((e) => e.label).filter(Boolean) ?? [];
  return names.length ? names.join(" — ") : "Établissement scolaire";
}

function baseSections(docId: string, ctx: RgpdTemplateContext): RgpdTemplateSection[] {
  switch (docId) {
    case "registre-traitements":
      return [
        {
          heading: "1. Responsable du traitement",
          paragraphs: [
            `${ctx.establishmentName}`,
            `Référent : ${ctx.directionReferent}`,
            `DPD : ${ctx.dpdName}`,
          ],
        },
        {
          heading: "2. Traitements — Scolarité et vie scolaire",
          paragraphs: ["Finalité : gestion administrative et pédagogique des élèves."],
          bullets: [
            `Personnes : ${ctx.audiencesText}`,
            "Données : identité, coordonnées, scolarité, absences, sanctions disciplinaires",
            "Base légale : mission d'intérêt public / obligation légale",
            `Sous-traitants : ${ctx.subprocessorsList}`,
            "Conservation : durée scolarité + archivage légal",
          ],
        },
        {
          heading: "3. Traitements — Personnel",
          paragraphs: ["Finalité : gestion RH et paie."],
          bullets: [
            "Données : identité, coordonnées, contrat, formation",
            "Conservation : durée légale employeur",
          ],
        },
        {
          heading: "4. Mesures de sécurité",
          paragraphs: [
            "Contrôle d'accès, sauvegardes, sensibilisation, gestion des incidents.",
            "Document à compléter et tenir à jour lors de tout nouveau traitement.",
          ],
        },
      ];
    case "mentions-information":
      return [
        {
          heading: "Information des personnes concernées",
          paragraphs: [
            `Établissement : ${ctx.establishmentName}`,
            `Publics : ${ctx.audiencesText}`,
          ],
          bullets: [
            "Finalités : scolarité, communication, sécurité, activités pédagogiques",
            "Destinataires : personnel habilité, autorités compétentes, sous-traitants listés",
            "Durées de conservation : selon politique de conservation de l'établissement",
            "Droits : accès, rectification, effacement, limitation, opposition, portabilité",
            "Réclamation auprès de la CNIL : www.cnil.fr",
            `Contact DPD / référent : ${ctx.dpdName !== "Non désigné" ? ctx.dpdName : ctx.directionReferent}`,
          ],
        },
      ];
    case "procedure-droits":
      return [
        {
          heading: "Réception des demandes",
          paragraphs: [
            "Les demandes peuvent être adressées par courrier ou e-mail à l'adresse de l'établissement.",
            `Référent traitement : ${ctx.directionReferent}`,
          ],
        },
        {
          heading: "Traitement",
          bullets: [
            "Accusé de réception sous 72 h",
            "Réponse sous 1 mois (prolongation possible de 2 mois)",
            "Vérification de l'identité du demandeur",
            "Registre des demandes tenu par le secrétariat / DPD",
          ],
        },
      ];
    case "procedure-violation":
      return [
        {
          heading: "Signalement interne",
          paragraphs: [
            "Tout incident doit être signalé immédiatement au référent et au DPD.",
            "Utiliser la fiche incident du module Conformité RGPD.",
          ],
        },
        {
          heading: "Évaluation et notification",
          bullets: [
            "Évaluer le risque pour les personnes",
            "Notifier la CNIL sous 72 h si risque pour les droits et libertés",
            "Informer les personnes si risque élevé",
            "Consigner dans le registre des violations",
            "Mettre en œuvre des mesures correctives",
          ],
        },
      ];
    case "liste-sous-traitants":
      return [
        {
          heading: "Sous-traitants identifiés",
          paragraphs: [`Liste établie le ${ctx.date} pour ${ctx.establishmentName}.`],
          bullets: ctx.subprocessorsList.split(" ; ").filter(Boolean),
        },
        {
          heading: "Accords de sous-traitance (DPA) — plateformes",
          paragraphs: [
            "Les plateformes ci-dessous disposent de clauses contractuelles RGPD (article 28) intégrées aux conditions générales ou annexes DPA du prestataire.",
          ],
          bullets:
            ctx.platformDpaLines.length > 0
              ? ctx.platformDpaLines
              : ["Aucune plateforme DPA sélectionnée — compléter le questionnaire."],
        },
        {
          heading: "Engagements",
          paragraphs: [
            "Chaque sous-traitant doit disposer d'un contrat ou de clauses RGPD (article 28).",
            "Vérifier la localisation des données et les garanties appropriées.",
            "Conserver une copie ou le lien vers chaque DPA signé ou accepté (licences, console cloud).",
          ],
        },
      ];
    case "politique-conservation":
      return [
        {
          heading: "Durées indicatives",
          bullets: [
            "Dossiers élèves : durée de scolarité + délais légaux d'archivage",
            "Données RH : 5 ans après départ (sauf obligations légales)",
            "Images / publications : 3 ans ou fin d'usage + suppression",
            "Logs techniques : 12 mois maximum",
            "Demandes de droits : 3 ans",
          ],
        },
        {
          heading: "Revue",
          paragraphs: ["Revue annuelle par la direction et le DPD."],
        },
      ];
    case "politique-protection":
      return [
        {
          heading: "Engagements",
          paragraphs: [
            `${ctx.establishmentName} s'engage à protéger les données personnelles conformément au RGPD.`,
            `Référent : ${ctx.directionReferent} — DPD : ${ctx.dpdName}`,
          ],
          bullets: [
            "Minimisation des données",
            "Sécurité et confidentialité",
            "Formation du personnel",
            "Revue annuelle de cette politique",
          ],
        },
      ];
    case "charte-informatique":
      return [
        {
          heading: "Usages autorisés",
          bullets: [
            "Outils institutionnels (ENT, messagerie, intranet)",
            "Respect des identifiants personnels",
            "Interdiction d'installer des logiciels non autorisés",
          ],
        },
        {
          heading: "Sous-traitants",
          paragraphs: [`Outils concernés : ${ctx.subprocessorsList}`],
        },
      ];
    case "charte-photos":
      return [
        {
          heading: "Droit à l'image",
          paragraphs: ["Encadrement des prises de vue et publications pour les activités scolaires."],
          bullets: [
            "Information des familles",
            "Opposition possible",
            `Traitements déclarés : ${ctx.sensitiveText}`,
          ],
        },
      ];
    case "designation-dpd":
      return [
        {
          heading: "Mission du DPD",
          paragraphs: [
            `Désignation de ${ctx.dpdName} en qualité de délégué à la protection des données.`,
          ],
          bullets: [
            "Informer et conseiller le responsable de traitement",
            "Contrôler le respect du RGPD",
            "Point de contact avec la CNIL",
          ],
        },
      ];
    case "aipd":
      return [
        {
          heading: "Analyse d'impact — traitements à risque",
          paragraphs: [
            `Établissement : ${ctx.establishmentName}`,
            `Traitements sensibles identifiés : ${ctx.sensitiveText}`,
          ],
          bullets: [
            "Description et finalités",
            "Évaluation des risques",
            "Mesures d'atténuation",
            "Avis du DPD",
          ],
        },
      ];
    case "mesures-securite":
      return [
        {
          heading: "Mesures techniques et organisationnelles",
          bullets: [
            "Contrôle d'accès et habilitations",
            "Sauvegardes régulières",
            "Antivirus et mises à jour",
            "Procédure de gestion des incidents",
            "Sensibilisation annuelle du personnel",
          ],
        },
      ];
    case "registre-violations":
      return [
        {
          heading: "Registre des violations",
          paragraphs: ["Tableau de suivi des violations de données personnelles."],
          bullets: [
            "Date de la violation",
            "Nature et catégories de données",
            "Notification CNIL (oui/non)",
            "Mesures correctives",
          ],
        },
      ];
    default:
      return [
        {
          heading: "Document",
          paragraphs: [`Modèle ${docId} — à personnaliser pour ${ctx.establishmentName}.`],
        },
      ];
  }
}

const RGPD_PRIORITY_DOC_IDS = [
  "registre-traitements",
  "mentions-information",
  "procedure-droits",
  "procedure-violation",
  "liste-sous-traitants",
  "politique-conservation",
] as const;

export function getTemplateSections(
  docId: string,
  answers: RgpdQuestionnaireAnswers,
  establishmentLabel?: string,
): { title: string; sections: RgpdTemplateSection[] } {
  const ctx = buildTemplateContext(answers);
  const estLabel = establishmentLabel || ctx.establishmentName;
  if (establishmentLabel) ctx.establishmentName = establishmentLabel;

  const fiche = getRgpdProcessingFiche(docId);
  if (fiche) {
    return {
      title: fiche.getTitle?.(answers) ?? fiche.title,
      sections: fiche.buildSections(answers, estLabel),
    };
  }

  if (docId === "registre-traitements") {
    return {
      title: "Registre des activités de traitement (complet)",
      sections: buildFullRegisterSections(answers, estLabel),
    };
  }

  if (docId === "politique-conservation") {
    return {
      title: "Politique de conservation des données",
      sections: [
        ...baseSections("politique-conservation", ctx),
        { heading: "—".repeat(40) },
        ...buildFicheSections("fiche-synthese-conservation", answers, estLabel)!,
      ],
    };
  }

  const titles: Record<string, string> = {
    "mentions-information": "Mentions d'information",
    "procedure-droits": "Procédure d'exercice des droits",
    "procedure-violation": "Procédure de notification de violation",
    "liste-sous-traitants": "Liste des sous-traitants",
    "politique-protection": "Politique de protection des données",
    "charte-informatique": "Charte informatique",
    "charte-photos": "Charte photos et droit à l'image",
    "designation-dpd": "Mission du DPD",
    aipd: "Analyse d'impact (AIPD)",
    "mesures-securite": "Mesures de sécurité",
    "registre-violations": "Registre des violations",
  };

  return {
    title: titles[docId] || docId,
    sections: baseSections(docId, ctx),
  };
}
