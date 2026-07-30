import type { EleveConfig } from "@/app/lib/eleves-config";
import type { TravelsParticipantEleve, TravelsTripData } from "@/app/lib/travels-types";

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function collectParticipantParentEmails(eleve: {
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

export function toParticipantEleve(
  e: Pick<EleveConfig, "ine" | "nom" | "prenom" | "classe">,
  droitImageOk = true,
): TravelsParticipantEleve {
  return {
    ine: e.ine,
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

export function buildElevesListCsv(participants: TravelsParticipantEleve[]): string {
  const header = "Nom;Prénom;Classe;INE";
  const rows = participants
    .slice()
    .sort((a, b) =>
      `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", { sensitivity: "base" }),
    )
    .map((p) =>
      [p.nom, p.prenom, p.classe || "", p.ine]
        .map((c) => `"${String(c).replace(/"/g, '""')}"`)
        .join(";"),
    );
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

export function isListeElevesConfirmed(data: TravelsTripData | undefined): boolean {
  return data?.listeElevesStatus === "confirmed" && (data.participantEleves?.length || 0) > 0;
}
