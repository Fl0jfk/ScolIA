/** Export PDF d'une séance d'enseignement transversal (généré dans le navigateur). */
import {
  ASSOCIATION_LOCKED_IDEA,
  ASSOCIATION_LOCKED_SUBJECT,
  formatSignupSessionDate,
  signupValidationStatus,
  TRANSVERSAL_NIVEAU_LABELS,
} from "@/app/lib/domain-planning-defaults";
import type {
  DomainPlanningSession,
  DomainPlanningSignup,
} from "@/app/lib/domain-planning-types";

const STATUS_LABELS: Record<string, string> = {
  validated: "Validé",
  pending: "En attente",
  changes_requested: "Modifications demandées",
  rejected: "Refusé",
};

function slug(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 60);
}

export async function exportTransversalSessionPdf(input: {
  domainName: string;
  session: DomainPlanningSession;
  classes: string[];
  signups: DomainPlanningSignup[];
}): Promise<void> {
  const { jsPDF } = await import("jspdf");
  const autoTable = (await import("jspdf-autotable")).default;
  const { session, classes, signups, domainName } = input;

  const doc = new jsPDF({ orientation: "landscape", unit: "mm", format: "a4" });
  const niveauLabel = TRANSVERSAL_NIVEAU_LABELS[session.niveau] || session.niveau;
  const isAssociation = session.intervenantConstraint === "fixed_association";

  doc.setFontSize(15);
  doc.setFont("helvetica", "bold");
  doc.text(`${domainName} — ${niveauLabel} — Séance ${session.seanceNumber}`, 14, 16);

  doc.setFontSize(11);
  doc.setFont("helvetica", "normal");
  const themeLines = doc.splitTextToSize(`Thème : ${session.theme || "à définir"}`, 269) as string[];
  doc.text(themeLines, 14, 24);
  let y = 24 + themeLines.length * 5;

  doc.setFontSize(9);
  doc.setTextColor(90);
  doc.text(
    `Intervenant : ${session.intervenantLabel} · Mixte : ${session.mixte ? "oui" : "non"} · Généré le ${new Date().toLocaleString("fr-FR")}`,
    14,
    y + 1,
  );
  doc.setTextColor(0);
  y += 6;

  // Toutes les classes du niveau, plus d'éventuels positionnements sur une classe retirée de la liste.
  const byClass = new Map(signups.map((s) => [s.className, s]));
  const allClasses = [...classes, ...signups.map((s) => s.className).filter((c) => !classes.includes(c))];

  const body = allClasses.map((className) => {
    const s = byClass.get(className);
    if (!s) {
      return isAssociation
        ? [className, "—", session.intervenantLabel, ASSOCIATION_LOCKED_SUBJECT, ASSOCIATION_LOCKED_IDEA, "—"]
        : [className, "—", "Non positionné", "—", "—", "—"];
    }
    const name = [s.firstName, s.lastName].filter(Boolean).join(" ").trim() || s.email || "—";
    const status = signupValidationStatus(s, session) || "pending";
    return [
      className,
      formatSignupSessionDate(s.sessionDate, "short") || "—",
      name,
      s.subject || "—",
      s.sessionIdea || "—",
      STATUS_LABELS[status] || status,
    ];
  });

  autoTable(doc, {
    startY: y,
    head: [["Classe", "Date", "Intervenant", "Matière", "Idée de séance", "Statut"]],
    body: body.length > 0 ? body : [["—", "—", "Aucune classe", "—", "—", "—"]],
    styles: { fontSize: 9, cellPadding: 2, valign: "top" },
    headStyles: { fillColor: [109, 40, 217], textColor: 255 },
    alternateRowStyles: { fillColor: [248, 246, 252] },
    columnStyles: {
      0: { cellWidth: 20, fontStyle: "bold" },
      1: { cellWidth: 24 },
      2: { cellWidth: 45 },
      3: { cellWidth: 35 },
      4: { cellWidth: "auto" },
      5: { cellWidth: 32 },
    },
  });

  const pageCount = doc.getNumberOfPages();
  for (let i = 1; i <= pageCount; i++) {
    doc.setPage(i);
    doc.setFontSize(8);
    doc.setTextColor(120);
    doc.text(`Page ${i} / ${pageCount}`, 283, 203, { align: "right" });
  }

  doc.save(
    `${slug(domainName) || "enseignements-transversaux"}_${slug(niveauLabel)}_seance-${session.seanceNumber}.pdf`,
  );
}
