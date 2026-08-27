import "server-only";

import { getElevePhotoUrl } from "@/app/lib/eleve-photos";
import { getInternatRooms, getInternatStudents } from "@/app/lib/internat-storage";
import type { InternatStudent } from "@/app/lib/internat-types";
import {
  classOptionLabel,
  resolveSiteIdForClass,
  resolveSiteLabel,
  type EleveDossierClassCatalog,
} from "@/app/lib/eleve-dossier-catalog";
import { buildEleveFolderName } from "@/app/lib/eleves-config";
import {
  MEAL_DAY_ORDER,
  parseEleveGrilleRepas,
  type EleveGrilleRepas,
} from "@/app/lib/eleve-grille-repas";

export type EleveRegimeRestauration = "externe" | "demi_pension" | "interne";

export type EleveMealWeek = {
  regime: EleveRegimeRestauration;
  /** Lun → Ven */
  days: Array<{
    key: "lun" | "mar" | "mer" | "jeu" | "ven";
    label: string;
    midi: boolean;
    soir: boolean;
    etude: boolean;
    garderie: boolean;
    sortSeul: boolean;
  }>;
  repasParSemaine: number | null;
  /** true si le motif de jours est déduit (pas encore saisi jour par jour). */
  inferred: boolean;
  /** Grille explicite persistée (null = déduite). */
  grille: EleveGrilleRepas | null;
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
    /** Valeur affichée en grand (ex. moyenne générale ou « — »). */
    value: string;
    detail: string;
  };
  absences: {
    available: boolean;
    label: string;
    value: string;
    detail: string;
  };
  finances: {
    available: boolean;
    label: string;
    detail: string;
  };
};

const WEEK_DAYS = MEAL_DAY_ORDER;

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
      (s) => s.actif && String(s.eleveRef.folderName || "").trim() === folder,
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
 * Priorité : grille explicite → sinon déduction demi-pension / repasParSemaine / internat.
 */
export function buildEleveMealWeek(opts: {
  demiPension: boolean;
  repasParSemaine: number | null | undefined;
  interne: boolean;
  grilleRepas?: unknown;
}): EleveMealWeek {
  const regime: EleveRegimeRestauration = opts.interne
    ? "interne"
    : opts.demiPension
      ? "demi_pension"
      : "externe";

  const grille = parseEleveGrilleRepas(opts.grilleRepas);
  if (grille) {
    const days = WEEK_DAYS.map((d) => ({
      key: d.key,
      label: d.label,
      midi: grille[d.key].midi,
      soir: grille[d.key].soir,
      etude: grille[d.key].etude,
      garderie: grille[d.key].garderie,
      sortSeul: grille[d.key].sortSeul,
    }));
    return {
      regime,
      days,
      repasParSemaine: days.filter((d) => d.midi).length,
      inferred: false,
      grille,
    };
  }

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
    etude: false,
    garderie: false,
    sortSeul: false,
  }));

  return {
    regime,
    days,
    repasParSemaine: rawCount,
    inferred: regime !== "externe",
    grille: null,
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
    grilleRepas?: unknown;
  } | null;
  catalog: EleveDossierClassCatalog;
  notesMoyennes?: Array<{ matiereLibelle: string; moyenne: string | null; nbNotes: number }>;
  absences?: EleveSyntheseSnapshot["absences"];
  finances?: EleveSyntheseSnapshot["finances"];
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
    grilleRepas: params.scolarite?.grilleRepas,
  });

  const siteId =
    params.eleve.siteId ??
    params.scolarite?.siteId ??
    resolveSiteIdForClass(params.eleve.classe, params.catalog);
  const siteLabel = resolveSiteLabel(siteId, params.catalog);
  const classeLabel = params.eleve.classe
    ? classOptionLabel(params.eleve.classe, siteLabel)
    : null;

  let photoUrl: string | null = null;
  try {
    photoUrl = await getElevePhotoUrl({
      ine: params.eleve.ine ?? "",
      nom: params.eleve.nom,
      prenom: params.eleve.prenom,
      folderName:
        params.eleve.folderName?.trim() ||
        buildEleveFolderName(params.eleve.nom, params.eleve.prenom),
    });
  } catch {
    photoUrl = null;
  }

  const notesLines = (params.notesMoyennes || []).filter((m) => m.moyenne != null);
  let notesValue = "—";
  if (notesLines.length > 0) {
    let sum = 0;
    let n = 0;
    for (const m of notesLines) {
      const v = Number(m.moyenne);
      if (Number.isFinite(v)) {
        sum += v;
        n += 1;
      }
    }
    notesValue = n > 0 ? (sum / n).toFixed(1) : String(notesLines.length);
  }
  const notesDetail =
    notesLines.length > 0
      ? notesLines
          .slice(0, 4)
          .map((m) => `${m.matiereLibelle} : ${m.moyenne}`)
          .join(" · ")
      : "Moyennes et alertes pédagogiques dès le module Notes.";

  return {
    statusLabel: eleveStatusLabel(params.eleve.status),
    classeLabel,
    siteLabel,
    initials: eleveInitials(params.eleve.prenom, params.eleve.nom),
    photoUrl,
    restauration,
    internat: {
      actif: interne,
      roomLabel,
    },
    notesTrimestre: {
      available: notesLines.length > 0,
      label: notesLines.length ? "Moyenne indicative" : "Notes — trimestre en cours",
      value: notesValue,
      detail: notesDetail,
    },
    absences: params.absences ?? {
      available: false,
      label: "Absences & retards",
      value: "—",
      detail: "Résumé vie scolaire non chargé pour ce profil.",
    },
    finances: params.finances ?? {
      available: false,
      label: "Facturation famille",
      detail: "Panneau finances non accessible pour ce profil.",
    },
  };
}
