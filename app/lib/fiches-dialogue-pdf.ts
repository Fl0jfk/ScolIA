import { PDFDocument, StandardFonts, rgb } from "pdf-lib";
import type {
  FdAcceptationPayload,
  FdAppelConfig,
  FdCatalogueChoix,
  FdConseilDecisionPayload,
  FdReponsePayload,
} from "@/db/schema-fiches-dialogue";

const GREEN = rgb(47 / 255, 107 / 255, 74 / 255);
const INK = rgb(20 / 255, 35 / 255, 26 / 255);
const MUTED = rgb(75 / 255, 99 / 255, 88 / 255);
const LINE = rgb(0.85, 0.9, 0.87);
const BOX = rgb(0.92, 0.94, 0.93);

export type FdPdfSection = {
  title: string;
  lines: string[];
  /** Cases à cocher affichées comme ☐ / ☑ */
  checks?: Array<{ label: string; checked: boolean }>;
};

export type FdPdfIdentity = {
  dateNaissance?: string | null;
  ine?: string | null;
  mef?: string | null;
  lva?: string | null;
  lvb?: string | null;
  optionsActuelles?: string[];
};

export type FdPdfInput = {
  title: string;
  subtitle?: string;
  campagneLabel: string;
  anneeLabel: string;
  eleveNom: string;
  elevePrenom: string;
  classeActuelle: string;
  etapeLabel: string;
  identity?: FdPdfIdentity;
  sections: FdPdfSection[];
  signatures?: Array<{ role: string; name: string; signedAt?: string }>;
  footerNote?: string;
};

function formatDateFr(d: Date): string {
  return d.toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

function formatMaybeDate(value: string | null | undefined): string {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return String(value);
  return d.toLocaleDateString("fr-FR");
}

/** Helvetica WinAnsi : retirer caractères hors plage (exposants, etc.). */
function pdfSafe(text: string): string {
  return Array.from(String(text || ""))
    .map((ch) => {
      const code = ch.codePointAt(0) ?? 0;
      if (code === 0x1d49 || ch === "ᵉ") return "e";
      if (code === 0x2b3 || ch === "ʳ") return "r";
      if (code === 0x1d48 || ch === "ᵈ") return "d";
      if (code >= 0x20 && code <= 0x7e) return ch;
      if (code >= 0xa0 && code <= 0xff) return ch;
      const stripped = ch.normalize("NFKD").replace(/[\u0300-\u036f]/g, "");
      if (stripped.length === 1) {
        const c2 = stripped.codePointAt(0) ?? 0;
        if ((c2 >= 0x20 && c2 <= 0x7e) || (c2 >= 0xa0 && c2 <= 0xff)) return stripped;
      }
      return "?";
    })
    .join("");
}

function resolveLabel(
  catalogue: FdCatalogueChoix,
  fieldId: string,
  value: string | string[] | boolean | null | undefined,
): string {
  if (value == null || value === "") return "—";
  if (typeof value === "boolean") return value ? "Oui" : "Non";
  const field = catalogue.fields.find((f) => f.id === fieldId);
  const mapLabel = (id: string): string => {
    if (field?.optionsFrom === "destinations") {
      return catalogue.destinations.find((d) => d.id === id)?.label ?? id;
    }
    if (field?.optionsFrom === "options") {
      return catalogue.options.find((o) => o.id === id)?.label ?? id;
    }
    const inline = field?.inlineOptions?.find((o) => o.id === id);
    return inline?.label ?? id;
  };
  if (Array.isArray(value)) {
    if (!value.length) return "—";
    return value.map(mapLabel).join(", ");
  }
  return mapLabel(String(value));
}

function checksFromCatalogueValues(
  catalogue: FdCatalogueChoix,
  values: Record<string, string | string[] | boolean | null>,
): Array<{ label: string; checked: boolean }> {
  const checks: Array<{ label: string; checked: boolean }> = [];
  const destVal = values.destination;
  for (const d of catalogue.destinations) {
    const checked =
      destVal === d.id || (Array.isArray(destVal) && destVal.includes(d.id));
    checks.push({ label: d.label, checked: Boolean(checked) });
  }
  const optVal = values.options ?? values.specialites;
  const selected = new Set(
    Array.isArray(optVal) ? optVal.map(String) : optVal ? [String(optVal)] : [],
  );
  for (const o of catalogue.options) {
    checks.push({ label: o.label, checked: selected.has(o.id) });
  }
  return checks;
}

export function sectionsFromFamilleReponse(
  catalogue: FdCatalogueChoix,
  payload: FdReponsePayload,
): FdPdfSection[] {
  const checks = checksFromCatalogueValues(catalogue, payload.values);
  const lines: string[] = [];
  if (payload.forceMalgreAvis) {
    lines.push("La famille maintient ses choix malgré l’avis du conseil.");
  }
  if (payload.comment?.trim()) {
    lines.push(`Commentaire : ${payload.comment.trim()}`);
  }
  if (payload.etablissementsVoeux?.length) {
    lines.push(
      "Établissements : " +
        payload.etablissementsVoeux
          .map((v) => `${v.rang}. ${v.label}${v.chezNous ? " (chez nous)" : ""}`)
          .join(" ; "),
    );
  }
  return [{ title: "Réponse / vœux de la famille", lines, checks }];
}

export function sectionsFromConseil(
  catalogue: FdCatalogueChoix,
  payload: FdConseilDecisionPayload,
): FdPdfSection[] {
  const values: Record<string, string | string[] | boolean | null> = {
    destination: payload.destinationProposee ?? null,
    options: payload.optionsProposees ?? [],
  };
  const checks = checksFromCatalogueValues(catalogue, values);
  const lines: string[] = [`Avis : ${payload.avis}`];
  if (payload.motif?.trim()) lines.push(`Motif : ${payload.motif.trim()}`);
  if (payload.commentaire?.trim()) lines.push(`Commentaire : ${payload.commentaire.trim()}`);
  return [{ title: "Proposition / avis du conseil de classe", lines, checks }];
}

export function sectionsFromAcceptation(
  payload: FdAcceptationPayload,
  appel?: FdAppelConfig | null,
): FdPdfSection[] {
  const checks = [
    {
      label: "Nous acceptons le choix du conseil de classe",
      checked: Boolean(payload.accepte),
    },
    {
      label: "Nous refusons la proposition du conseil de classe",
      checked: !payload.accepte,
    },
  ];
  const lines: string[] = [];
  if (!payload.accepte && payload.motifRefus?.trim()) {
    lines.push(`Motif du refus : ${payload.motifRefus.trim()}`);
  }
  if (!payload.accepte && appel?.enabled) {
    lines.push("Une procédure d’appel peut être engagée selon les modalités communiquées.");
    if (appel.dateLimite) lines.push(`Date limite d’appel : ${appel.dateLimite}`);
  }
  return [{ title: "Position de la famille", lines, checks }];
}

export function identityFromFiche(fiche: {
  eleveDateNaissance?: string | Date | null;
  eleveIne?: string | null;
  eleveMef?: string | null;
  optionsActuelles?: string[] | null;
}): FdPdfIdentity {
  const opts = fiche.optionsActuelles ?? [];
  const lva = opts.find((o) => /^LV1\b/i.test(o)) ?? null;
  const lvb = opts.find((o) => /^LV2\b/i.test(o)) ?? null;
  const dateNaissance =
    fiche.eleveDateNaissance instanceof Date
      ? fiche.eleveDateNaissance.toISOString().slice(0, 10)
      : fiche.eleveDateNaissance ?? null;
  return {
    dateNaissance,
    ine: fiche.eleveIne ?? null,
    mef: fiche.eleveMef ?? null,
    lva,
    lvb,
    optionsActuelles: opts,
  };
}

export async function buildFicheDialoguePdf(input: FdPdfInput): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const fontBold = await doc.embedFont(StandardFonts.HelveticaBold);
  let page = doc.addPage([595.28, 841.89]);
  const margin = 40;
  const width = page.getWidth();
  let y = page.getHeight() - margin;

  const ensureSpace = (needed: number) => {
    if (y - needed < margin) {
      page = doc.addPage([595.28, 841.89]);
      y = page.getHeight() - margin;
    }
  };

  const drawText = (
    text: string,
    size: number,
    opts?: { bold?: boolean; color?: ReturnType<typeof rgb>; x?: number; maxWidth?: number },
  ) => {
    const maxWidth = opts?.maxWidth ?? width - margin * 2;
    const words = pdfSafe(text).split(/\s+/);
    let line = "";
    for (const word of words) {
      const candidate = line ? `${line} ${word}` : word;
      const f = opts?.bold ? fontBold : font;
      if (f.widthOfTextAtSize(candidate, size) > maxWidth && line) {
        ensureSpace(size + 4);
        page.drawText(line, {
          x: opts?.x ?? margin,
          y,
          size,
          font: f,
          color: opts?.color ?? INK,
        });
        y -= size + 4;
        line = word;
      } else {
        line = candidate;
      }
    }
    if (line) {
      ensureSpace(size + 4);
      page.drawText(line, {
        x: opts?.x ?? margin,
        y,
        size,
        font: opts?.bold ? fontBold : font,
        color: opts?.color ?? INK,
      });
      y -= size + 4;
    }
  };

  drawText("FICHE DE DIALOGUE", 16, { bold: true, color: GREEN });
  drawText(input.title, 12, { bold: true });
  if (input.subtitle) drawText(input.subtitle, 10, { color: MUTED });
  y -= 4;
  page.drawLine({
    start: { x: margin, y },
    end: { x: width - margin, y },
    thickness: 1,
    color: LINE,
  });
  y -= 14;

  // En-tête identité type feuille papier
  ensureSpace(90);
  const boxTop = y + 10;
  const boxHeight = 78;
  page.drawRectangle({
    x: margin,
    y: boxTop - boxHeight,
    width: width - margin * 2,
    height: boxHeight,
    color: BOX,
    borderColor: LINE,
    borderWidth: 0.8,
  });
  // Emplacement photo
  page.drawRectangle({
    x: width - margin - 56,
    y: boxTop - 66,
    width: 48,
    height: 56,
    borderColor: MUTED,
    borderWidth: 0.8,
  });
  page.drawText(pdfSafe("Photo"), {
    x: width - margin - 48,
    y: boxTop - 38,
    size: 8,
    font,
    color: MUTED,
  });

  const id = input.identity;
  const leftX = margin + 8;
  let idY = boxTop - 14;
  const idLine = (label: string, value: string) => {
    page.drawText(pdfSafe(`${label} ${value}`), {
      x: leftX,
      y: idY,
      size: 9,
      font,
      color: INK,
    });
    idY -= 12;
  };
  idLine("NOM :", input.eleveNom);
  idLine("Prenom :", input.elevePrenom);
  idLine("Date de naissance :", formatMaybeDate(id?.dateNaissance));
  idLine("INE :", id?.ine || "—");
  idLine("MEF :", id?.mef || "—");
  idLine("Classe :", input.classeActuelle || "—");
  const lvParts = [
    id?.lva ? `LVA : ${id.lva}` : null,
    id?.lvb ? `LVB : ${id.lvb}` : null,
  ].filter(Boolean);
  if (lvParts.length) {
    page.drawText(pdfSafe(lvParts.join("   ")), {
      x: leftX,
      y: idY,
      size: 9,
      font,
      color: INK,
    });
  }
  y = boxTop - boxHeight - 14;

  drawText(`Campagne : ${input.campagneLabel} (${input.anneeLabel})`, 9, { color: MUTED });
  drawText(`Etape : ${input.etapeLabel}`, 9, { color: MUTED });
  drawText(`Document edite le ${formatDateFr(new Date())}`, 8, { color: MUTED });
  y -= 8;

  for (const section of input.sections) {
    ensureSpace(36);
    drawText(section.title, 11, { bold: true, color: GREEN });
    y -= 2;
    if (section.checks?.length) {
      for (const c of section.checks) {
        ensureSpace(14);
        const mark = c.checked ? "[X]" : "[ ]";
        drawText(`${mark}  ${c.label}`, 10);
      }
    }
    for (const line of section.lines) {
      drawText(`• ${line}`, 10);
    }
    y -= 8;
  }

  if (input.signatures?.length) {
    ensureSpace(70);
    drawText("Signatures", 11, { bold: true, color: GREEN });
    const colW = (width - margin * 2) / Math.min(input.signatures.length, 2);
    let col = 0;
    const sigY = y;
    for (const sig of input.signatures) {
      const x = margin + col * colW;
      page.drawText(pdfSafe(sig.role), { x, y: sigY, size: 9, font: fontBold, color: INK });
      page.drawText(pdfSafe(sig.name), { x, y: sigY - 14, size: 10, font, color: INK });
      if (sig.signedAt) {
        page.drawText(pdfSafe(sig.signedAt), { x, y: sigY - 26, size: 8, font, color: MUTED });
      }
      col += 1;
      if (col >= 2) {
        col = 0;
        y = sigY - 40;
      }
    }
    if (col !== 0) y = sigY - 40;
  }

  if (input.footerNote) {
    y -= 10;
    drawText(input.footerNote, 9, { color: MUTED });
  }

  return doc.save();
}
