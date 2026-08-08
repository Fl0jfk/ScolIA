import "server-only";

import {
  AlignmentType,
  BorderStyle,
  Document,
  HeadingLevel,
  Packer,
  Paragraph,
  TextRun,
} from "docx";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";
import { getTemplateMeta, placeholderToken } from "@/app/lib/document-templates/catalog";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

function p(
  text: string,
  opts?: { bold?: boolean; size?: number; center?: boolean; spaceAfter?: number },
) {
  return new Paragraph({
    alignment: opts?.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: opts?.spaceAfter ?? 160 },
    children: [
      new TextRun({
        text,
        bold: opts?.bold,
        size: opts?.size ?? 22,
        font: "Calibri",
      }),
    ],
  });
}

function heading(text: string) {
  return new Paragraph({
    heading: HeadingLevel.HEADING_1,
    spacing: { after: 240 },
    border: {
      bottom: { style: BorderStyle.SINGLE, size: 12, color: "1E293B", space: 8 },
    },
    children: [new TextRun({ text, bold: true, size: 28, font: "Calibri", color: "0F172A" })],
  });
}

function ph(key: string) {
  return placeholderToken(key);
}

async function schoolHeader(): Promise<Paragraph[]> {
  const lh = await getSchoolLetterhead();
  return [
    p(lh.name, { bold: true, size: 26 }),
    ...(lh.subtitle ? [p(lh.subtitle, { size: 18 })] : []),
    ...(lh.addressLine ? [p(lh.addressLine, { size: 18 })] : []),
    new Paragraph({
      spacing: { after: 280 },
      border: {
        bottom: { style: BorderStyle.SINGLE, size: 18, color: "2563EB", space: 1 },
      },
      children: [],
    }),
  ];
}

function helpNote(): Paragraph {
  return p(
    "Modèle vierge — remplacez les marqueurs $$…$$ par les champs de votre logiciel (Charlemagne, Pronote, Word).",
    { size: 16, spaceAfter: 240 },
  );
}

async function renderCertificat(): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          heading("CERTIFICAT DE SCOLARITÉ"),
          helpNote(),
          p(
            `Je soussigné(e), ${ph("signataire")}, ${ph("qualite")}, certifie que :`,
          ),
          p(`${ph("prenom")} ${ph("nom")}`.toUpperCase(), { bold: true, size: 26 }),
          p(
            `est régulièrement inscrit(e) dans notre établissement en classe de ${ph("classe")} pour l'année scolaire ${ph("annee")}.`,
          ),
          p("Le présent certificat est délivré pour servir et valoir ce que de droit."),
          p(`Fait à ${ph("ville")}, le ${ph("date")}.`, { spaceAfter: 280 }),
          p(ph("signataire"), { bold: true }),
          p(ph("qualite"), { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderFiche(): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          heading("FICHE D'INSCRIPTION"),
          helpNote(),
          p(`Année scolaire ${ph("annee")}`, { size: 20 }),
          p("Enfant", { bold: true }),
          p(`Nom : ${ph("nom")}`),
          p(`Prénom : ${ph("prenom")}`),
          p(`Date de naissance : ${ph("dateNaissance")}`),
          p(`Classe demandée : ${ph("classe")}`),
          p("Responsable 1", { bold: true }),
          p(`Nom : ${ph("responsable")}`),
          p(`E-mail : ${ph("resp1Email")}`),
          p(`Téléphone : ${ph("resp1Tel")}`),
          p("Coordonnées & infos", { bold: true }),
          p(`Adresse : ${ph("adresse")}`),
          p(`Allergies / précautions : ${ph("allergies")}`),
          p(`Droit à l'image : ${ph("droitImage")}`),
          p(`Notes : ${ph("notes")}`),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderAutorisation(): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          heading("AUTORISATION PARENTALE DE SORTIE"),
          helpNote(),
          p(
            `Je soussigné(e) ${ph("responsable")}, responsable légal(e) de ${ph("prenom")} ${ph("nom")} (classe ${ph("classe")}),`,
          ),
          p(`autorise mon enfant à participer à :`),
          p(ph("sortie"), { bold: true, size: 24 }),
          p(`Lieu : ${ph("lieu")}`),
          p(
            `Du ${ph("dateDebut")} au ${ph("dateFin")} — départ ${ph("horaireDepart")} / retour ${ph("horaireRetour")}`,
          ),
          p(`Téléphone du responsable : ${ph("respTel")}`),
          p(`Téléphone d'urgence : ${ph("urgenceTel")}`),
          p(`Participation autorisée : ${ph("autorise")}`),
          p(`Soins d'urgence autorisés : ${ph("soins")}`),
          p(`Informations utiles : ${ph("notes")}`),
          p(`Date : ${ph("date")}`, { spaceAfter: 280 }),
          p("Signature du responsable légal", { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderCourrier(): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          helpNote(),
          p(ph("destinataire"), { spaceAfter: 200 }),
          p(`Objet : ${ph("objet")}`, { bold: true, spaceAfter: 240 }),
          p(ph("corps"), { spaceAfter: 200 }),
          p(`Fait à ${ph("ville")}, le ${ph("date")}.`, { spaceAfter: 280 }),
          p(ph("signataire"), { bold: true }),
          p(ph("qualite"), { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

/**
 * DOCX brandé avec placeholders `$$key$$` (pas de MERGEFIELD OOXML).
 * Le secrétariat remplace dans Charlemagne / Pronote / Word.
 */
export async function renderDocumentTemplateDocx(templateId: DocumentTemplateId): Promise<Buffer> {
  if (!getTemplateMeta(templateId)) throw new Error("Modèle inconnu");
  if (templateId === "certificat-scolarite") return renderCertificat();
  if (templateId === "fiche-inscription") return renderFiche();
  if (templateId === "autorisation-sortie") return renderAutorisation();
  if (templateId === "courrier-families") return renderCourrier();
  throw new Error("Modèle inconnu");
}
