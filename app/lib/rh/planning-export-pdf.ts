import {
  PLANNING_WEEKDAY_LABELS,
  type PlanningWeekday,
  type StaffFixedSlot,
  type StaffMissionSlot,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";
import { planningSlotColor } from "@/app/lib/rh/planning-slot-colors";

type ExportGridSlot = {
  day: PlanningWeekday;
  start: string;
  end: string;
  title: string;
  subtitle?: string;
  colorKey: string;
};

function slugify(name: string): string {
  return name
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/^_|_$/g, "")
    .slice(0, 48);
}

function toMin(hhmm: string): number {
  const [h, m] = hhmm.split(":").map(Number);
  return (h || 0) * 60 + (m || 0);
}

function teacherSlotsToGrid(slots: TeacherPlanningSlot[]): ExportGridSlot[] {
  return slots.map((s) => ({
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.subject || "Cours",
    subtitle: [(s.classes || []).join(", "), s.room].filter(Boolean).join(" · "),
    colorKey: s.subject || "cours",
  }));
}

function fixedSlotsToGrid(slots: StaffFixedSlot[]): ExportGridSlot[] {
  return slots.map((s) => ({
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.label || "Poste",
    colorKey: s.label || "poste",
  }));
}

function missionSlotsToGrid(slots: StaffMissionSlot[]): ExportGridSlot[] {
  return slots.map((s) => ({
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.mission || "Mission",
    subtitle: s.location || undefined,
    colorKey: s.mission || "mission",
  }));
}

async function renderPlanningGridPdf(input: {
  title: string;
  subtitle: string;
  slots: ExportGridSlot[];
  fileName: string;
}): Promise<void> {
  const { jsPDF } = await import("jspdf");
  const doc = new jsPDF({ orientation: "landscape", unit: "mm", format: "a4" });
  const pageW = doc.internal.pageSize.getWidth();
  const pageH = doc.internal.pageSize.getHeight();
  const margin = 10;
  const gridTop = 28;
  const gridLeft = margin + 12;
  const gridW = pageW - gridLeft - margin;
  const gridH = pageH - gridTop - margin;
  const dayW = gridW / 5;
  const dayStartMin = 7 * 60;
  const dayEndMin = 19 * 60;
  const minH = gridH / (dayEndMin - dayStartMin);

  doc.setFont("helvetica", "bold");
  doc.setFontSize(14);
  doc.text(input.title, margin, 14);
  doc.setFont("helvetica", "normal");
  doc.setFontSize(9);
  doc.setTextColor(80, 80, 90);
  doc.text(input.subtitle, margin, 20);
  doc.setTextColor(0, 0, 0);

  for (let d = 1; d <= 5; d += 1) {
    const x = gridLeft + (d - 1) * dayW;
    doc.setFillColor(248, 250, 252);
    doc.rect(x, gridTop, dayW, gridH, "F");
    doc.setDrawColor(226, 232, 240);
    doc.rect(x, gridTop, dayW, gridH, "S");
    doc.setFont("helvetica", "bold");
    doc.setFontSize(8);
    doc.text(PLANNING_WEEKDAY_LABELS[d as PlanningWeekday], x + dayW / 2, gridTop - 2, {
      align: "center",
    });
  }

  for (let hour = 7; hour <= 19; hour += 1) {
    const y = gridTop + (hour * 60 - dayStartMin) * minH;
    doc.setDrawColor(241, 245, 249);
    doc.line(gridLeft, y, gridLeft + gridW, y);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(7);
    doc.setTextColor(148, 163, 184);
    doc.text(`${String(hour).padStart(2, "0")}:00`, margin, y + 1.5);
    doc.setTextColor(0, 0, 0);
  }

  for (const slot of input.slots) {
    const color = planningSlotColor(slot.colorKey);
    const x = gridLeft + (slot.day - 1) * dayW + 0.6;
    const w = dayW - 1.2;
    const top = gridTop + (Math.max(dayStartMin, toMin(slot.start)) - dayStartMin) * minH;
    const bottom = gridTop + (Math.min(dayEndMin, toMin(slot.end)) - dayStartMin) * minH;
    const h = Math.max(6, bottom - top);
    doc.setFillColor(...color.pdfFill);
    doc.setDrawColor(...color.pdfStroke);
    doc.roundedRect(x, top, w, h, 1.2, 1.2, "FD");
    doc.setFont("helvetica", "bold");
    doc.setFontSize(6.5);
    doc.text(`${slot.start}–${slot.end}`, x + 1.2, top + 3.2);
    doc.setFontSize(7);
    const titleLines = doc.splitTextToSize(slot.title, w - 2.4);
    doc.text(titleLines.slice(0, 2), x + 1.2, top + 6.5);
    if (slot.subtitle) {
      doc.setFont("helvetica", "normal");
      doc.setFontSize(6);
      const subLines = doc.splitTextToSize(slot.subtitle, w - 2.4);
      doc.text(subLines.slice(0, 2), x + 1.2, top + 6.5 + titleLines.slice(0, 2).length * 2.8);
    }
  }

  doc.save(input.fileName);
}

export async function downloadTeacherPlanningPdf(input: {
  displayName: string;
  week: "A" | "B";
  slots: TeacherPlanningSlot[];
}): Promise<void> {
  const name = input.displayName.trim() || "Planning";
  await renderPlanningGridPdf({
    title: `${name} — Semaine type ${input.week}`,
    subtitle: `Exporté le ${new Date().toLocaleDateString("fr-FR")} · ${input.slots.length} créneau(x)`,
    slots: teacherSlotsToGrid(input.slots),
    fileName: `planning_${slugify(name)}_semaine_${input.week}.pdf`,
  });
}

export async function downloadStaffFixedPlanningPdf(input: {
  displayName: string;
  slots: StaffFixedSlot[];
}): Promise<void> {
  const name = input.displayName.trim() || "Planning";
  await renderPlanningGridPdf({
    title: `${name} — Semaine type`,
    subtitle: `Exporté le ${new Date().toLocaleDateString("fr-FR")} · ${input.slots.length} créneau(x)`,
    slots: fixedSlotsToGrid(input.slots),
    fileName: `planning_${slugify(name)}_fixe.pdf`,
  });
}

export async function downloadStaffMissionPlanningPdf(input: {
  displayName: string;
  rotationLabel: string;
  slots: StaffMissionSlot[];
}): Promise<void> {
  const name = input.displayName.trim() || "Planning";
  await renderPlanningGridPdf({
    title: `${name} — ${input.rotationLabel || "Missions"}`,
    subtitle: `Exporté le ${new Date().toLocaleDateString("fr-FR")} · ${input.slots.length} créneau(x)`,
    slots: missionSlotsToGrid(input.slots),
    fileName: `planning_${slugify(name)}_missions.pdf`,
  });
}
