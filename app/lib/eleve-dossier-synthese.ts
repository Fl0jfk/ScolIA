import "server-only";

import { getInternatRooms, getInternatStudents } from "@/app/lib/internat-storage";
import type { InternatStudent } from "@/app/lib/internat-types";
import {
  classOptionLabel,
  resolveSiteIdForClass,
  resolveSiteLabel,
  type EleveDossierClassCatalog,
} from "@/app/lib/eleve-dossier-catalog";

export type EleveRegimeRestauration = "externe" | "demi_pension" | "interne";

export type EleveMealWeek = {
  regime: EleveRegimeRestauration;
  /** Lun → Ven */
  days: Array<{
    key: "lun" | "mar" | "mer" | "jeu" | "ven";
    label: string;
    midi: boolean;
    soir: boolean;
  }>;
  repasParSemaine: number | null;
  /** true si le motif de jours est déduit (pas encore saisi jour par jour). */
  inferred: boolean;
};

export type EleveSyntheseSnapshot = {
  statusLabel: string;
  classeLabel: string | null;
  siteLabel: string | null;
  initials: string;
  photoUrl: string | null;
  restauration: EleveMealWeek;
  internat: {
    actif: boolean;
    roomLabel: string | null;
  };
  notesTrimestre: {
    available: boolean;
    label: string;
    detail: string;
  };
  absences: {
    available: boolean;
    label: string;
    detail: string;
  };
};

const WEEK_DAYS = [
  { key: "lun" as const, label: "Lun" },
  { key: "mar" as const, label: "Mar" },
  { key: "mer" as const, label: "Mer" },
  { key: "jeu" as const, label: "Jeu" },
  { key: "ven" as const, label: "Ven" },
];

export function eleveStatusLabel(status: string | null | undefined): string {
  switch (String(status || "").trim().toLowerCase()) {
    case "inscrit":
      return "En cours";
    case "preinscrit":
      return "Préinscrit";
    case "ancien":
      return "Ancien";
    case "archive":
      return "Archivé";
    default:
      return status?.trim() || "—";
  }
}

export function eleveInitials(prenom: string, nom: string): string {
  const p = prenom.trim().charAt(0);
  const n = nom.trim().charAt(0);
  return `${p}${n}`.toUpperCase() || "?";
}

function foldName(s: string): string {
  return s
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

export function matchInternatStudent(
  students: InternatStudent[],
  eleve: { nom: string; prenom: string; ine?: string | null; folderName?: string | null },
): InternatStudent | null {
  const ine = eleve.ine?.trim().toUpperCase();
  if (ine) {
    const byIne = students.find(
      (s) => s.actif && s.eleveRef.ine?.trim().toUpperCase() === ine,
    );
    if (byIne) return byIne;
  }
  const folder = eleve.folderName?.trim();
  if (folder) {
    const byFolder = students.find(
      (s) => s.actif && s.eleveRef.folderName.trim() === folder,
    );
    if (byFolder) return byFolder;
  }
  const nom = foldName(eleve.nom);
  const prenom = foldName(eleve.prenom);
  return (
    students.find(
      (s) =>
        s.actif &&
        foldName(s.eleveRef.nom) === nom &&
        foldName(s.eleveRef.prenom) === prenom,
    ) ?? null
  );
}

/**
 * Calendrier repas Lun–Ven.
 * Sans saisie jour-par-jour : on déduit depuis demi-pension / repasParSemaine / internat.
 * Midi = premiers N jours ouvrés ; soir (interne) = Lun–Jeu.
 */
export function buildEleveMealWeek(opts: {
  demiPension: boolean;
  repasParSemaine: number | null | undefined;
  interne: boolean;
}): EleveMealWeek {
  const regime: EleveRegimeRestauration = opts.interne
    ? "interne"
    : opts.demiPension
      ? "demi_pension"
      : "externe";

  const rawCount =
    typeof opts.repasParSemaine === "number" && Number.isFinite(opts.repasParSemaine)
      ? Math.max(0, Math.min(5, Math.round(opts.repasParSemaine)))
      : null;

  const midiCount =
    regime === "externe"
      ? 0
      : rawCount ?? (regime === "interne" || regime === "demi_pension" ? 5 : 0);

  const days = WEEK_DAYS.map((d, i) => ({
    key: d.key,
    label: d.label,
    midi: i < midiCount,
    soir: opts.interne && i < 4,
  }));

  return {
    regime,
    days,
    repasParSemaine: rawCount,
    inferred: regime !== "externe",
  };
}

export function regimeLabel(regime: EleveRegimeRestauration): string {
  switch (regime) {
    case "interne":
      return "Interne";
    case "demi_pension":
      return "Demi-pensionnaire";
    default:
      return "Externe";
  }
}

export async function buildEleveSyntheseSnapshot(params: {
  eleve: {
    nom: string;
    prenom: string;
    classe: string | null;
    status: string;
    ine?: string | null;
    folderName?: string | null;
    siteId?: string | null;
  };
  scolarite: {
    demiPension: boolean;
    repasParSemaine: number | null;
    siteId: string | null;
  } | null;
  catalog: EleveDossierClassCatalog;
}): Promise<EleveSyntheseSnapshot> {
  let internatStudent: InternatStudent | null = null;
  let roomLabel: string | null = null;
  try {
    const roster = await getInternatStudents();
    internatStudent = matchInternatStudent(roster, params.eleve);
    if (internatStudent?.roomId) {
      const rooms = await getInternatRooms();
      roomLabel = rooms.find((r) => r.id === internatStudent!.roomId)?.label ?? null;
    }
  } catch {
    internatStudent = null;
  }

  const demiPension = Boolean(params.scolarite?.demiPension);
  const repasParSemaine = params.scolarite?.repasParSemaine ?? null;
  const interne = Boolean(internatStudent);
  const restauration = buildEleveMealWeek({
    demiPension: demiPension || interne,
    repasParSemaine,
    interne,
  });

  const siteId =
    params.eleve.siteId ??
    params.scolarite?.siteId ??
    resolveSiteIdForClass(params.eleve.classe, params.catalog);
  const siteLabel = resolveSiteLabel(siteId, params.catalog);
  const classeLabel = params.eleve.classe
    ? classOptionLabel(params.eleve.classe, siteLabel)
    : null;

  return {
    statusLabel: eleveStatusLabel(params.eleve.status),
    classeLabel,
    siteLabel,
    initials: eleveInitials(params.eleve.prenom, params.eleve.nom),
    photoUrl: null,
    restauration,
    internat: {
      actif: interne,
      roomLabel,
    },
    notesTrimestre: {
      available: false,
      label: "Notes — trimestre en cours",
      detail: "Moyennes et alertes pédagogiques dès le module Notes (P4).",
    },
    absences: {
      available: false,
      label: "Absences & retards",
      detail: "Résumé vie scolaire à brancher — aucun signal pour l’instant.",
    },
  };
}
