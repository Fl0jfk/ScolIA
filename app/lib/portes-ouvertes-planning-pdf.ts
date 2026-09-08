import "server-only";

import { jsPDF } from "jspdf";
import autoTable from "jspdf-autotable";
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

    const body = slots.map((s) => {
      const regs = input.registrations.filter((r) => r.slotId === s.id);
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
      const visiteurs = regs
        .map((r) => {
          const child = [r.childFirstName, r.childLastName].filter(Boolean).join(" ");
          const visited = r.visitedAt ? " ✓" : "";
          return `${r.firstName} ${r.lastName}${child ? ` (${child})` : ""}${visited}`;
        })
        .join(" · ");
      const places = s.maxPlaces
        ? `${regs.length}/${s.maxPlaces}`
        : String(regs.length);
      return [
        `${s.label}\n${formatWhen(s.startAt)}`,
        places,
        visiteurs || "—",
        ambassadeurs || "—",
        enseignants || "—",
        personnel || "—",
      ];
    });

    autoTable(doc, {
      startY,
      head: [["Créneau", "Places", "Visiteurs", "Ambassadeurs", "Profs", "OGEC"]],
      body,
      styles: { fontSize: 7, cellPadding: 1.5, valign: "top" },
      headStyles: { fillColor: [91, 33, 182], textColor: 255 },
      alternateRowStyles: { fillColor: [248, 250, 252] },
      columnStyles: {
        0: { cellWidth: 36 },
        1: { cellWidth: 16 },
        2: { cellWidth: 70 },
        3: { cellWidth: 45 },
        4: { cellWidth: 45 },
        5: { cellWidth: 45 },
      },
    });

    const finalY =
      (doc as unknown as { lastAutoTable?: { finalY?: number } }).lastAutoTable?.finalY ||
      startY + 20;
    startY = finalY + 10;
  }

  return Buffer.from(doc.output("arraybuffer"));
}
