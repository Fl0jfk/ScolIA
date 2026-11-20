import "server-only";

import { jsPDF } from "jspdf";
import autoTable, { type CellDef, type CellHookData } from "jspdf-autotable";
import type { PortesOuvertesCycle } from "@/app/lib/portes-ouvertes-types";
import { PORTES_OUVERTES_CYCLE_LABELS, PORTES_OUVERTES_CYCLES } from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesRegistration } from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";
import type { PortesOuvertesStaffRow } from "@/app/lib/portes-ouvertes-db";
import { parisDateKey } from "@/app/lib/paris-time";

type Rgb = [number, number, number];

const PAGE_W = 297;
const PAGE_H = 210;
const MARGIN_X = 12;
const CONTENT_W = PAGE_W - MARGIN_X * 2; // 273
const TOP = 24; // sous le bandeau d’en-tête
const BOTTOM = 196; // au-dessus du pied de page

const VIOLET: Rgb = [91, 33, 182];
const INK: Rgb = [15, 23, 42];
const MUTED: Rgb = [100, 116, 139];
const LINE: Rgb = [226, 232, 240];
const SOFT: Rgb = [248, 250, 252];

/** Couleur d’accent par établissement (lisible avec du texte blanc). */
const CYCLE_COLORS: Record<PortesOuvertesCycle, Rgb> = {
  ecole: [161, 98, 7],
  college: [3, 105, 161],
  lycee: [190, 24, 93],
};

function paris(iso: string, opts: Intl.DateTimeFormatOptions): string {
  return new Date(iso).toLocaleString("fr-FR", { timeZone: "Europe/Paris", ...opts });
}

function timeOf(iso: string): string {
  return paris(iso, { hour: "2-digit", minute: "2-digit" });
}

function dayLabel(iso: string): string {
  const s = paris(iso, { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function visitorPrimaryLabel(r: PortesOuvertesRegistration): string {
  const child = [r.childFirstName, r.childLastName].filter(Boolean).join(" ").trim();
  if (child) return child;
  return `${r.firstName} ${r.lastName}`.trim();
}

function hasChild(r: PortesOuvertesRegistration): boolean {
  return Boolean([r.childFirstName, r.childLastName].filter(Boolean).join(" ").trim());
}

function staffNames(staff: PortesOuvertesStaffRow[], role: string): string {
  return staff
    .filter((x) => x.role === role)
    .map((x) => x.displayName)
    .join(", ");
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n > 1 ? many : one}`;
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
  const generatedAt = input.generatedAt || new Date();
  const when = generatedAt.toLocaleString("fr-FR", {
    timeZone: "Europe/Paris",
    dateStyle: "long",
    timeStyle: "short",
  });
  const labels = { ...PORTES_OUVERTES_CYCLE_LABELS, ...input.cycleLabels };
  const cycles = input.cycles?.length ? input.cycles : [...PORTES_OUVERTES_CYCLES];
  const title = input.title || "Portes ouvertes — Planning";

  const lastY = () =>
    (doc as unknown as { lastAutoTable?: { finalY?: number } }).lastAutoTable?.finalY ?? TOP;

  // --- Page 1 : synthèse -------------------------------------------------
  let y = TOP + 2;
  const totalSlots = input.slots.length;
  const totalRegs = input.registrations.length;
  const totalVisited = input.registrations.filter((r) => r.visitedAt).length;

  const tiles: Array<{ value: string; label: string }> = [
    { value: String(totalSlots), label: totalSlots > 1 ? "créneaux" : "créneau" },
    { value: String(totalRegs), label: totalRegs > 1 ? "inscrits" : "inscrit" },
    { value: String(totalVisited), label: totalVisited > 1 ? "visites effectuées" : "visite effectuée" },
  ];
  const tileW = 56;
  tiles.forEach((t, i) => {
    const x = MARGIN_X + i * (tileW + 6);
    doc.setFillColor(...SOFT);
    doc.setDrawColor(...LINE);
    doc.roundedRect(x, y, tileW, 16, 2, 2, "FD");
    doc.setTextColor(...VIOLET);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(16);
    doc.text(t.value, x + 5, y + 10.5);
    doc.setTextColor(...MUTED);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(8.5);
    doc.text(t.label, x + 5 + doc.getTextWidth(t.value) * 1.0 + 3, y + 10.2);
  });
  y += 22;

  let firstCycle = true;
  let printedAny = false;

  for (const cycle of cycles) {
    const slots = input.slots
      .filter((s) => s.cycle === cycle || (!s.cycle && cycle === "college"))
      .sort((a, b) => a.startAt.localeCompare(b.startAt));
    if (slots.length === 0) continue;
    printedAny = true;

    // Un établissement par page : lecture et impression bien séparées.
    if (!firstCycle) {
      doc.addPage();
      y = TOP;
    }
    firstCycle = false;

    const color = CYCLE_COLORS[cycle];
    const slotIds = new Set(slots.map((s) => s.id));
    const cycleRegs = input.registrations.filter((r) => slotIds.has(r.slotId)).length;

    // Bandeau d’établissement
    doc.setFillColor(...color);
    doc.roundedRect(MARGIN_X, y, CONTENT_W, 10, 2, 2, "F");
    doc.setTextColor(255, 255, 255);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(12);
    doc.text(labels[cycle] || cycle, MARGIN_X + 5, y + 6.9);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(9);
    doc.text(
      `${plural(slots.length, "créneau", "créneaux")}  ·  ${plural(cycleRegs, "inscrit", "inscrits")}`,
      MARGIN_X + CONTENT_W - 5,
      y + 6.6,
      { align: "right" },
    );
    y += 15;

    let currentDay = "";

    for (const s of slots) {
      const dKey = parisDateKey(s.startAt);

      const regs = input.registrations
        .filter((r) => r.slotId === s.id)
        .sort((a, b) =>
          visitorPrimaryLabel(a).localeCompare(visitorPrimaryLabel(b), "fr", { sensitivity: "base" }),
        );
      const staff = input.staff.filter((x) => x.slotId === s.id);
      const ambassadeurs = staffNames(staff, "ambassadeur");
      const enseignants = staffNames(staff, "enseignant");
      const personnel = staffNames(staff, "personnel");
      const staffParts = [
        ambassadeurs ? `Ambassadeurs : ${ambassadeurs}` : "",
        enseignants ? `Professeurs : ${enseignants}` : "",
        personnel ? `OGEC : ${personnel}` : "",
      ].filter(Boolean);

      // Place nécessaire pour ne pas laisser un titre orphelin en bas de page.
      const dayChanged = dKey !== currentDay;
      const needed = (dayChanged ? 12 : 0) + 8 + (staffParts.length ? 6 : 0) + 6 + 2 * 11;
      if (y + needed > BOTTOM) {
        doc.addPage();
        y = TOP;
      }

      if (dayChanged) {
        currentDay = dKey;
        doc.setTextColor(...INK);
        doc.setFont("helvetica", "bold");
        doc.setFontSize(11);
        doc.text(dayLabel(s.startAt), MARGIN_X, y + 4);
        doc.setDrawColor(...color);
        doc.setLineWidth(0.5);
        doc.line(MARGIN_X, y + 6.2, MARGIN_X + CONTENT_W, y + 6.2);
        doc.setLineWidth(0.2);
        y += 10;
      }

      const places = s.maxPlaces ? `${regs.length} / ${s.maxPlaces} places` : plural(regs.length, "inscrit", "inscrits");
      const full = s.maxPlaces ? regs.length >= s.maxPlaces : false;

      const head: CellDef[][] = [
        [
          {
            content: `${timeOf(s.startAt)} – ${timeOf(s.endAt)}   ${s.label}`,
            colSpan: 4,
            styles: {
              fillColor: VIOLET,
              textColor: 255,
              fontStyle: "bold",
              fontSize: 9.5,
              halign: "left",
              cellPadding: { top: 2.4, bottom: 2.4, left: 3, right: 3 },
            },
          },
          {
            content: places,
            styles: {
              fillColor: VIOLET,
              textColor: full ? [254, 202, 202] : 255,
              fontStyle: "bold",
              fontSize: 8.5,
              halign: "right",
              cellPadding: { top: 2.4, bottom: 2.4, left: 3, right: 3 },
            },
          },
        ],
      ];
      if (staffParts.length) {
        head.push([
          {
            content: staffParts.join("      "),
            colSpan: 5,
            styles: {
              fillColor: [237, 233, 254],
              textColor: [76, 29, 149],
              fontStyle: "normal",
              fontSize: 7.5,
              halign: "left",
              cellPadding: { top: 1.8, bottom: 1.8, left: 3, right: 3 },
            },
          },
        ]);
      }
      head.push(
        ["Élève / visiteur", "Classe visée", "Contact", "Coordonnées", "Présence"].map(
          (c, i): CellDef => ({
            content: c,
            styles: {
              fillColor: [241, 245, 249],
              textColor: MUTED,
              fontStyle: "bold",
              fontSize: 6.8,
              halign: i === 4 ? "center" : "left",
              cellPadding: { top: 1.3, bottom: 1.3, left: 3, right: 3 },
            },
          }),
        ),
      );

      const body: CellDef[][] =
        regs.length === 0
          ? [
              [
                {
                  content: "Aucun visiteur inscrit pour le moment",
                  colSpan: 5,
                  styles: {
                    textColor: [148, 163, 184],
                    fontStyle: "italic",
                    halign: "center",
                    cellPadding: 3,
                  },
                },
              ],
            ]
          : regs.map((r): CellDef[] => {
              const contact = hasChild(r) ? `${r.firstName} ${r.lastName}`.trim() : "—";
              const coords = [r.email?.trim(), r.phone?.trim()].filter(Boolean).join("\n") || "—";
              return [
                { content: visitorPrimaryLabel(r), styles: { fontStyle: "bold", textColor: INK } },
                { content: r.classeSouhaitee?.trim() || "—" },
                { content: contact },
                { content: coords },
                { content: "", styles: { halign: "center" } },
              ];
            });

      autoTable(doc, {
        startY: y,
        head,
        body,
        showHead: "everyPage",
        theme: "plain",
        margin: { top: TOP, bottom: PAGE_H - BOTTOM, left: MARGIN_X, right: MARGIN_X },
        tableWidth: CONTENT_W,
        styles: {
          font: "helvetica",
          fontSize: 8,
          textColor: INK,
          cellPadding: { top: 2.2, bottom: 2.2, left: 3, right: 3 },
          valign: "middle",
          overflow: "linebreak",
          lineColor: LINE,
          lineWidth: 0,
        },
        columnStyles: {
          0: { cellWidth: 66 },
          1: { cellWidth: 32 },
          2: { cellWidth: 58 },
          3: { cellWidth: 85 },
          4: { cellWidth: 32 },
        },
        alternateRowStyles: { fillColor: SOFT },
        rowPageBreak: "avoid",
        didDrawCell: (data: CellHookData) => {
          if (data.section !== "body") return;
          const { doc: d, cell } = data;
          // Filet fin sous chaque ligne.
          d.setDrawColor(...LINE);
          d.setLineWidth(0.2);
          d.line(cell.x, cell.y + cell.height, cell.x + cell.width, cell.y + cell.height);

          // Colonne « Présence » : case à cocher, ou « Vu » si déjà validé.
          if (data.column.index === 4 && regs.length > 0) {
            const reg = regs[data.row.index];
            if (!reg) return;
            const cx = cell.x + cell.width / 2;
            const cy = cell.y + cell.height / 2;
            if (reg.visitedAt) {
              d.setFont("helvetica", "bold");
              d.setFontSize(8);
              d.setTextColor(21, 128, 61);
              d.text("Vu", cx, cy + 1, { align: "center" });
              d.setTextColor(...INK);
            } else {
              d.setDrawColor(148, 163, 184);
              d.setLineWidth(0.35);
              d.roundedRect(cx - 2.2, cy - 2.2, 4.4, 4.4, 0.6, 0.6, "S");
              d.setLineWidth(0.2);
            }
          }
        },
      });

      y = lastY() + 7;
    }
  }

  if (!printedAny) {
    doc.setTextColor(...MUTED);
    doc.setFont("helvetica", "italic");
    doc.setFontSize(11);
    doc.text("Aucun créneau à afficher pour cette sélection.", MARGIN_X, y + 6);
  }

  // --- En-tête et pied de page sur chaque page ---------------------------
  const pages = doc.getNumberOfPages();
  for (let p = 1; p <= pages; p++) {
    doc.setPage(p);

    doc.setFillColor(...VIOLET);
    doc.rect(0, 0, PAGE_W, 17, "F");
    doc.setFillColor(167, 139, 250);
    doc.rect(0, 17, PAGE_W, 1.2, "F");

    doc.setTextColor(255, 255, 255);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(14);
    doc.text(title, MARGIN_X, 10.2);
    if (input.address) {
      doc.setFont("helvetica", "normal");
      doc.setFontSize(8);
      doc.setTextColor(221, 214, 254);
      doc.text(input.address, MARGIN_X, 14.4);
    }
    doc.setFont("helvetica", "normal");
    doc.setFontSize(8);
    doc.setTextColor(221, 214, 254);
    doc.text("Planning des portes ouvertes", PAGE_W - MARGIN_X, 10.2, { align: "right" });

    doc.setDrawColor(...LINE);
    doc.setLineWidth(0.2);
    doc.line(MARGIN_X, 201, PAGE_W - MARGIN_X, 201);
    doc.setTextColor(...MUTED);
    doc.setFontSize(7.5);
    doc.text(`ScolIA  ·  Édité le ${when}`, MARGIN_X, 205.5);
    doc.text(`Page ${p} / ${pages}`, PAGE_W - MARGIN_X, 205.5, { align: "right" });
  }

  return Buffer.from(doc.output("arraybuffer"));
}
