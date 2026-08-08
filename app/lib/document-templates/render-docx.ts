import "server-only";

import {
  AlignmentType,
  Document,
  HeadingLevel,
  Packer,
  Paragraph,
  TextRun,
  BorderStyle,
} from "docx";
import { getSchoolLetterhead } from "@/app/lib/pdf-branding";
import type { DocumentTemplateId } from "@/app/lib/document-templates/types";

function str(v: Record<string, string | boolean>, key: string): string {
  const x = v[key];
  if (typeof x === "boolean") return x ? "Oui" : "Non";
  return String(x || "").trim();
}

function formatFrDate(iso: string): string {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) return iso || "—";
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

function p(text: string, opts?: { bold?: boolean; size?: number; center?: boolean; spaceAfter?: number }) {
  return new Paragraph({
    alignment: opts?.center ? AlignmentType.CENTER : AlignmentType.LEFT,
    spacing: { after: opts?.spaceAfter ?? 160 },
    children: [
      new TextRun({
        text,
        bold: opts?.bold,
        size: opts?.size ?? 22, // half-points
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

function sectionTitle(text: string) {
  return new Paragraph({
    spacing: { before: 200, after: 120 },
    children: [new TextRun({ text, bold: true, size: 22, font: "Calibri", color: "1E40AF" })],
  });
}

function bodyParagraphs(text: string): Paragraph[] {
  const blocks = String(text || "")
    .split(/\n+/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (!blocks.length) return [p("—")];
  return blocks.map((line) => p(line, { spaceAfter: 140 }));
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

async function renderCertificat(values: Record<string, string | boolean>): Promise<Buffer> {
  const lh = await getSchoolLetterhead();
  const ville = str(values, "ville") || lh.cityLine || lh.name;
  const doc = new Document({
    sections: [
      {
        properties: {},
        children: [
          ...(await schoolHeader()),
          heading("CERTIFICAT DE SCOLARITÉ"),
          p(
            `Je soussigné(e), ${str(values, "signataire") || str(values, "qualite")}, ${str(values, "qualite")}, certifie que :`,
          ),
          p(`${str(values, "prenom")} ${str(values, "nom")}`.toUpperCase(), { bold: true, size: 26 }),
          p(
            `est régulièrement inscrit(e) dans notre établissement en classe de ${str(values, "classe")} pour l'année scolaire ${str(values, "anneeScolaire")}.`,
          ),
          p("Le présent certificat est délivré pour servir et valoir ce que de droit."),
          p(`Fait à ${ville}, le ${formatFrDate(str(values, "dateDocument"))}.`, {
            spaceAfter: 280,
          }),
          p(str(values, "signataire") || "—", { bold: true }),
          p(str(values, "qualite") || "", { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderFiche(values: Record<string, string | boolean>): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          heading("FICHE D'INSCRIPTION"),
          p(`Année scolaire ${str(values, "anneeScolaire")}`, { size: 20 }),
          sectionTitle("Enfant"),
          p(`Nom : ${str(values, "nom") || "—"}`),
          p(`Prénom : ${str(values, "prenom") || "—"}`),
          p(`Date de naissance : ${formatFrDate(str(values, "dateNaissance"))}`),
          p(`Classe demandée : ${str(values, "classeDemandee") || "—"}`),
          sectionTitle("Responsable 1"),
          p(`Nom : ${str(values, "resp1Nom") || "—"}`),
          p(`E-mail : ${str(values, "resp1Email") || "—"}`),
          p(`Téléphone : ${str(values, "resp1Tel") || "—"}`),
          ...(str(values, "resp2Nom") || str(values, "resp2Email")
            ? [
                sectionTitle("Responsable 2"),
                p(`Nom : ${str(values, "resp2Nom") || "—"}`),
                p(`E-mail : ${str(values, "resp2Email") || "—"}`),
                p(`Téléphone : ${str(values, "resp2Tel") || "—"}`),
              ]
            : []),
          sectionTitle("Coordonnées & infos"),
          p(`Adresse : ${str(values, "adresse") || "—"}`),
          p(`Allergies / précautions : ${str(values, "allergies") || "Néant"}`),
          p(`Droit à l'image : ${str(values, "droitImage")}`),
          ...(str(values, "notes") ? [p(`Notes : ${str(values, "notes")}`)] : []),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderAutorisation(values: Record<string, string | boolean>): Promise<Buffer> {
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          heading("AUTORISATION PARENTALE DE SORTIE"),
          p(
            `Je soussigné(e) ${str(values, "respNom")}, responsable légal(e) de ${str(values, "prenom")} ${str(values, "nom")} (classe ${str(values, "classe")}),`,
          ),
          p(
            `${str(values, "autorise") === "Oui" ? "autorise" : "n'autorise pas"} mon enfant à participer à :`,
          ),
          p(str(values, "sortieTitre") || "—", { bold: true, size: 24 }),
          p(`Lieu : ${str(values, "lieu") || "—"}`),
          p(
            `Du ${formatFrDate(str(values, "dateDebut"))} au ${formatFrDate(str(values, "dateFin"))}` +
              (str(values, "horaireDepart") || str(values, "horaireRetour")
                ? ` — départ ${str(values, "horaireDepart") || "—"} / retour ${str(values, "horaireRetour") || "—"}`
                : ""),
          ),
          p(`Téléphone du responsable : ${str(values, "respTel") || "—"}`),
          ...(str(values, "urgenceTel")
            ? [p(`Téléphone d'urgence : ${str(values, "urgenceTel")}`)]
            : []),
          p(`Soins d'urgence autorisés : ${str(values, "soins")}`),
          ...(str(values, "notes") ? [p(`Informations utiles : ${str(values, "notes")}`)] : []),
          p(`Date : ${formatFrDate(str(values, "dateDocument"))}`, { spaceAfter: 280 }),
          p("Signature du responsable légal", { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

async function renderCourrier(values: Record<string, string | boolean>): Promise<Buffer> {
  const lh = await getSchoolLetterhead();
  const ville = str(values, "ville") || lh.cityLine || lh.name;
  const doc = new Document({
    sections: [
      {
        children: [
          ...(await schoolHeader()),
          p(str(values, "destinataire") || "Aux familles", { spaceAfter: 200 }),
          p(`Objet : ${str(values, "objet") || "—"}`, { bold: true, spaceAfter: 240 }),
          ...bodyParagraphs(str(values, "corps")),
          p(`Fait à ${ville}, le ${formatFrDate(str(values, "dateDocument"))}.`, {
            spaceAfter: 280,
          }),
          p(str(values, "signataire") || "—", { bold: true }),
          p(str(values, "qualite") || "", { size: 18 }),
        ],
      },
    ],
  });
  return Buffer.from(await Packer.toBuffer(doc));
}

export async function renderDocumentTemplateDocx(
  templateId: DocumentTemplateId,
  values: Record<string, string | boolean>,
): Promise<Buffer> {
  if (templateId === "certificat-scolarite") return renderCertificat(values);
  if (templateId === "fiche-inscription") return renderFiche(values);
  if (templateId === "autorisation-sortie") return renderAutorisation(values);
  if (templateId === "courrier-families") return renderCourrier(values);
  throw new Error("Modèle inconnu");
}
