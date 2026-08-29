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

export type FdPdfSection = {
  title: string;
  lines: string[];
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
      // accents courants via NFKD
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

export function sectionsFromFamilleReponse(
  catalogue: FdCatalogueChoix,
  payload: FdReponsePayload,
): FdPdfSection[] {
  const lines: string[] = [];
  for (const field of catalogue.fields) {
    const raw = payload.values[field.id];
    lines.push(`${field.label} : ${resolveLabel(catalogue, field.id, raw)}`);
  }
  if (payload.forceMalgreAvis) {
    lines.push("La famille maintient ses choix malgré l’avis du conseil.");
  }
  if (payload.comment?.trim()) {
    lines.push(`Commentaire : ${payload.comment.trim()}`);
  }
  return [{ title: "Vœux / choix de la famille", lines }];
}

export function sectionsFromConseil(
  catalogue: FdCatalogueChoix,
  payload: FdConseilDecisionPayload,
): FdPdfSection[] {
  const lines: string[] = [
    `Avis : ${payload.avis}`,
  ];
  if (payload.destinationProposee) {
    const dest =
      catalogue.destinations.find((d) => d.id === payload.destinationProposee)?.label ??
      payload.destinationProposee;
    lines.push(`Destination proposée : ${dest}`);
  }
  if (payload.optionsProposees?.length) {
    const opts = payload.optionsProposees.map(
      (id) => catalogue.options.find((o) => o.id === id)?.label ?? id,
    );
    lines.push(`Options proposées : ${opts.join(", ")}`);
  }
  if (payload.motif?.trim()) lines.push(`Motif : ${payload.motif.trim()}`);
  if (payload.commentaire?.trim()) lines.push(`Commentaire : ${payload.commentaire.trim()}`);
  return [{ title: "Décision du conseil de classe", lines }];
}

export function sectionsFromAcceptation(
  payload: FdAcceptationPayload,
  appel?: FdAppelConfig | null,
): FdPdfSection[] {
  const lines: string[] = [
    payload.accepte
      ? "La famille accepte la décision définitive du conseil de classe."
      : "La famille n’accepte pas la décision définitive du conseil de classe.",
  ];
  if (!payload.accepte && payload.motifRefus?.trim()) {
    lines.push(`Motif du refus : ${payload.motifRefus.trim()}`);
  }
  if (!payload.accepte && appel?.enabled) {
    lines.push("Une procédure d’appel peut être engagée selon les modalités communiquées.");
    if (appel.dateLimite) lines.push(`Date limite d’appel : ${appel.dateLimite}`);
  }
  return [{ title: "Position de la famille", lines }];
}

export async function buildFicheDialoguePdf(input: FdPdfInput): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const fontBold = await doc.embedFont(StandardFonts.HelveticaBold);
  let page = doc.addPage([595.28, 841.89]);
  const margin = 48;
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
    opts?: { bold?: boolean; color?: ReturnType<typeof rgb>; x?: number },
  ) => {
    const maxWidth = width - margin * 2;
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

  drawText("Fiche de dialogue", 20, { bold: true, color: GREEN });
  drawText(input.title, 14, { bold: true });
  if (input.subtitle) drawText(input.subtitle, 10, { color: MUTED });
  y -= 6;
  page.drawLine({
    start: { x: margin, y },
    end: { x: width - margin, y },
    thickness: 1,
    color: LINE,
  });
  y -= 16;

  drawText(`Campagne : ${input.campagneLabel} (${input.anneeLabel})`, 10);
  drawText(`Élève : ${input.elevePrenom} ${input.eleveNom}`, 10, { bold: true });
  drawText(`Classe actuelle : ${input.classeActuelle || "—"}`, 10);
  drawText(`Étape : ${input.etapeLabel}`, 10);
  drawText(`Document édité le ${formatDateFr(new Date())}`, 9, { color: MUTED });
  y -= 10;

  for (const section of input.sections) {
    ensureSpace(40);
    drawText(section.title, 12, { bold: true, color: GREEN });
    y -= 2;
    for (const line of section.lines) {
      drawText(`• ${line}`, 10);
    }
    y -= 8;
  }

  if (input.signatures?.length) {
    ensureSpace(60);
    drawText("Signatures", 12, { bold: true, color: GREEN });
    for (const sig of input.signatures) {
      const when = sig.signedAt ? ` — ${sig.signedAt}` : "";
      drawText(`${sig.role} : ${sig.name}${when}`, 10);
    }
  }

  if (input.footerNote) {
    y -= 12;
    drawText(input.footerNote, 9, { color: MUTED });
  }

  return doc.save();
}
