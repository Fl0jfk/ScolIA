import type { EleveConfig } from "@/app/lib/eleves-config";
import type { TravelsParticipantEleve, TravelsTripData } from "@/app/lib/travels-types";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** Clé stable pour sélection (INE, ou empreinte locale si INE absent). */
export function eleveParticipantKey(e: {
  ine?: string;
  nom: string;
  prenom: string;
  classe?: string;
}): string {
  const ine = String(e.ine || "").trim();
  if (ine) return ine;
  const nom = String(e.nom || "").trim().toUpperCase();
  const prenom = String(e.prenom || "").trim().toUpperCase();
  const classe = String(e.classe || "").trim();
  return `local:${nom}|${prenom}|${classe}`;
}

function collectParticipantParentEmails(eleve: {
  parentEmail?: string;
  parent1Email?: string;
  parent2Email?: string;
}): string[] {
  const set = new Set<string>();
  for (const raw of [eleve.parentEmail, eleve.parent1Email, eleve.parent2Email]) {
    const e = String(raw || "")
      .trim()
      .toLowerCase();
    if (e && EMAIL_RE.test(e)) set.add(e);
  }
  return [...set];
}

function collectParticipantParentPhones(eleve: {
  parentPhone?: string;
  parent1Phone?: string;
  parent2Phone?: string;
}): string[] {
  const set = new Set<string>();
  for (const raw of [eleve.parentPhone, eleve.parent1Phone, eleve.parent2Phone]) {
    const t = String(raw || "").trim();
    if (t.length >= 6) set.add(t);
  }
  return [...set];
}

export function toParticipantEleve(
  e: Pick<EleveConfig, "ine" | "nom" | "prenom" | "classe">,
  droitImageOk = true,
): TravelsParticipantEleve {
  return {
    ine: eleveParticipantKey(e),
    nom: e.nom,
    prenom: e.prenom,
    classe: e.classe,
    droitImageOk,
  };
}

/** Sync nbEleves + libellé classes depuis la liste nominative. */
export function applyParticipantElevesToTripData(
  data: TravelsTripData,
  participants: TravelsParticipantEleve[],
  opts?: { resetConfirmation?: boolean },
): TravelsTripData {
  const classes = [...new Set(participants.map((p) => p.classe).filter(Boolean) as string[])].sort(
    (a, b) => a.localeCompare(b, "fr"),
  );
  const next: TravelsTripData = {
    ...data,
    participantEleves: participants,
    nbEleves: participants.length,
    classes: classes.length > 0 ? classes.join(", ") : data.classes,
  };
  if (opts?.resetConfirmation) {
    next.listeElevesStatus = "draft";
    delete next.listeElevesConfirmedAt;
    delete next.listeElevesConfirmedBy;
    delete next.listeEnvoyeeTransporteurAt;
  }
  return next;
}

function csvCell(value: string): string {
  return `"${String(value).replace(/"/g, '""')}"`;
}

/** CSV interne simple (sans contacts). */
function buildElevesListCsv(participants: TravelsParticipantEleve[]): string {
  const header = "Nom;Prénom;Classe;INE";
  const rows = participants
    .slice()
    .sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    )
    .map((p) =>
      [p.nom, p.prenom, p.classe || "", p.ine.startsWith("local:") ? "" : p.ine]
        .map((c) => csvCell(String(c)))
        .join(";"),
    );
  return [header, ...rows].join("\n");
}

/** CSV pour le transporteur : nom, prénom, classe, e-mail(s) et tél. parent(s). */
export function buildElevesListCsvForTransporter(
  participants: TravelsParticipantEleve[],
  elevesByKey: Map<string, EleveConfig>,
): string {
  const header = "Nom;Prénom;Classe;Email parent;Tél. parent";
  const rows = participants
    .slice()
    .sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    )
    .map((p) => {
      const full = elevesByKey.get(eleveParticipantKey(p)) || elevesByKey.get(p.ine);
      const emails = full ? collectParticipantParentEmails(full) : [];
      const phones = full ? collectParticipantParentPhones(full) : [];
      return [
        p.nom,
        p.prenom,
        p.classe || "",
        emails.join(" / "),
        phones.join(" / "),
      ]
        .map((c) => csvCell(String(c)))
        .join(";");
    });
  return [header, ...rows].join("\n");
}

export function parentEmailCoverage(
  participants: TravelsParticipantEleve[],
  elevesByIne: Map<string, EleveConfig>,
): { withMail: number; withoutMail: number; emails: string[] } {
  const emailSet = new Set<string>();
  let withMail = 0;
  let withoutMail = 0;
  for (const p of participants) {
    const full = elevesByIne.get(p.ine);
    const mails = full ? collectParticipantParentEmails(full) : [];
    if (mails.length > 0) {
      withMail += 1;
      mails.forEach((m) => emailSet.add(m));
    } else {
      withoutMail += 1;
    }
  }
  return { withMail, withoutMail, emails: [...emailSet] };
}

function isListeElevesConfirmed(data: TravelsTripData | undefined): boolean {
  return data?.listeElevesStatus === "confirmed" && (data.participantEleves?.length || 0) > 0;
}
