import "server-only";

import { jsPDF } from "jspdf";
import autoTable, { type CellDef, type CellHookData } from "jspdf-autotable";
import type { PortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";
import { PORTES_OUVERTES_CYCLE_LABELS, PORTES_OUVERTES_CYCLES } from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";
import type { PortesOuvertesStaffRow } from "@/app/lib/portes-ouvertes-db";

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "short",
    timeStyle: "short",
  });
}

function visitorPrimaryLabel(r: PortesOuvertesRegistration): string {
  const child = [r.childFirstName, r.childLastName].filter(Boolean).join(" ").trim();
  if (child) return child;
  return `${r.firstName} ${r.lastName}`.trim();
}

/** Texte multi-lignes d’une fiche famille (style planning du jour). */
function visitorCardLines(r: PortesOuvertesRegistration): string {
  const primary = visitorPrimaryLabel(r);
  const visited = r.visitedAt ? "  ✓" : "";
  const lines: string[] = [`${primary}${visited}`];

  const meta = [r.classeSouhaitee?.trim(), r.email?.trim()].filter(Boolean);
  if (meta.length > 0) lines.push(meta.join("  ·  "));

  const child = [r.childFirstName, r.childLastName].filter(Boolean).join(" ").trim();
  if (child) {
    lines.push(`Contact : ${r.firstName} ${r.lastName}`.trim());
  }

  return lines.join("\n");
}

function spanCell(content: string, rowSpan: number): CellDef {
  return {
    content,
    rowSpan,
    styles: { valign: "top" },
  };
}

function visitorCell(content: string): CellDef {
  return {
    content,
    styles: {
      fillColor: [248, 250, 252],
      textColor: [15, 23, 42],
      lineWidth: { top: 0.35, right: 0.2, bottom: 0.35, left: 0.9 },
      lineColor: [203, 213, 225],
      cellPadding: { top: 2.2, right: 2.2, bottom: 2.2, left: 2.8 },
      fontStyle: "normal",
      valign: "top",
      minCellHeight: 11,
    },
  };
}

function emptyVisitorCell(): CellDef {
  return {
    content: "Aucun visiteur",
    styles: {
      textColor: [148, 163, 184],
      fontStyle: "italic",
      valign: "top",
    },
  };
}

function isVisitorCardCell(data: CellHookData): boolean {
  if (data.section !== "body") return false;
  const lw = data.cell.styles.lineWidth;
  // Marqueur posé dans visitorCell (bordure gauche épaisse).
  return typeof lw === "object" && lw !== null && (lw.left ?? 0) >= 0.8;
}

/** Accents violet à gauche des fiches visiteurs (comme les cartes du planning). */
function drawVisitorCardAccent(data: CellHookData): void {
  if (!isVisitorCardCell(data)) return;

  const { doc, cell } = data;
  const x = cell.x + 0.7;
  const y = cell.y + 1.1;
  const h = Math.max(4, cell.height - 2.2);
  doc.setFillColor(91, 33, 182);
  doc.roundedRect(x, y, 1.1, h, 0.4, 0.4, "F");
}

export function renderPortesOuvertesPlanningPdf(input: {
  title: string;
  address?: string;
  generatedAt?: Date;
  cycleLabels?: Partial<Record<PortesOuvertesCycle, string>>;
  /** Cycles à afficher (défaut : tous). */
  cycles?: PortesOuvertesCycle[];
  slots: PortesOuvertesSlot[];
  registrations: PortesOuvertesRegistration[];
  staff: PortesOuvertesStaffRow[];
}): Buffer {
  const doc = new jsPDF({ orientation: "landscape", unit: "mm", format: "a4" });
  const when = (input.generatedAt || new Date()).toLocaleString("fr-FR");
  const labels = { ...PORTES_OUVERTES_CYCLE_LABELS, ...input.cycleLabels };
  const cycles = input.cycles?.length ? input.cycles : [...PORTES_OUVERTES_CYCLES];

  doc.setFontSize(14);
  doc.setFont("helvetica", "bold");
  doc.text(input.title || "Portes ouvertes — Planning", 14, 16);
  doc.setFontSize(9);
  doc.setFont("helvetica", "normal");
  doc.setTextColor(80);
  doc.text(
    `${input.address || ""} · Généré le ${when} · ${input.registrations.length} inscrit(s)`.trim(),
    14,
    22,
  );
  doc.setTextColor(0);

  let startY = 28;
  for (const cycle of cycles) {
    const slots = input.slots
      .filter((s) => s.cycle === cycle || (!s.cycle && cycle === "college"))
      .sort((a, b) => a.startAt.localeCompare(b.startAt));
    if (slots.length === 0) continue;

    if (startY > 180) {
      doc.addPage();
      startY = 16;
    }

    doc.setFontSize(11);
    doc.setFont("helvetica", "bold");
    doc.text(labels[cycle] || cycle, 14, startY);
    startY += 4;

    const body: CellDef[][] = [];

    for (const s of slots) {
      const regs = input.registrations
        .filter((r) => r.slotId === s.id)
        .sort((a, b) =>
          visitorPrimaryLabel(a).localeCompare(visitorPrimaryLabel(b), "fr", {
            sensitivity: "base",
          }),
        );
      const staff = input.staff.filter((x) => x.slotId === s.id);
      const ambassadeurs = staff
        .filter((x) => x.role === "ambassadeur")
        .map((x) => x.displayName)
        .join(", ");
      const enseignants = staff
        .filter((x) => x.role === "enseignant")
        .map((x) => x.displayName)
        .join(", ");
      const personnel = staff
        .filter((x) => x.role === "personnel")
        .map((x) => x.displayName)
        .join(", ");
      const places = s.maxPlaces ? `${regs.length}/${s.maxPlaces}` : String(regs.length);
      const creneau = `${s.label}\n${formatWhen(s.startAt)}`;
      const rowSpan = Math.max(1, regs.length);

      if (regs.length === 0) {
        body.push([
          spanCell(creneau, 1),
          spanCell(places, 1),
          emptyVisitorCell(),
          spanCell(ambassadeurs || "—", 1),
          spanCell(enseignants || "—", 1),
          spanCell(personnel || "—", 1),
        ]);
        continue;
      }

      regs.forEach((r, index) => {
        if (index === 0) {
          body.push([
            spanCell(creneau, rowSpan),
            spanCell(places, rowSpan),
            visitorCell(visitorCardLines(r)),
            spanCell(ambassadeurs || "—", rowSpan),
            spanCell(enseignants || "—", rowSpan),
            spanCell(personnel || "—", rowSpan),
          ]);
        } else {
          // Une ligne = une réservation famille ; créneau / staff déjà en rowSpan.
          body.push([visitorCell(visitorCardLines(r))]);
        }
      });
    }

    autoTable(doc, {
      startY,
      head: [["Créneau", "Places", "Visiteurs (familles)", "Ambassadeurs", "Profs", "OGEC"]],
      body,
      styles: { fontSize: 7, cellPadding: 1.8, valign: "top", overflow: "linebreak" },
      headStyles: { fillColor: [91, 33, 182], textColor: 255, fontStyle: "bold" },
      alternateRowStyles: { fillColor: [255, 255, 255] },
      margin: { left: 12, right: 12 },
      tableWidth: 273,
      columnStyles: {
        0: { cellWidth: 32 },
        1: { cellWidth: 14 },
        2: { cellWidth: 85 },
        3: { cellWidth: 47 },
        4: { cellWidth: 47 },
        5: { cellWidth: 48 },
      },
      didDrawCell: (data) => {
        drawVisitorCardAccent(data);
      },
    });

    const finalY =
      (doc as unknown as { lastAutoTable?: { finalY?: number } }).lastAutoTable?.finalY ||
      startY + 20;
    startY = finalY + 10;
  }

  return Buffer.from(doc.output("arraybuffer"));
}
