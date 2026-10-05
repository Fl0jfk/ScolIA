import "server-only";

import {
  closeAppel,
  getAppelWithLignes,
  getOrCreateAppel,
  saveAppelLignes,
  type CloseAppelResult,
  type VsAppelLigneInput,
} from "@/app/lib/vs-absences-db";
import {
  assertProfesseurCanAccessAppel,
  filterCreneauxForProfesseur,
  isProfesseurScopedAppelActor,
  professeurOwnsCreneau,
} from "@/app/lib/vs-appels-access";
import { jourSemaineFromIsoDate, listEdtCreneauxForJour } from "@/app/lib/vs-calendrier-db";

export type VsAppelActor = {
  userId: string;
  displayName: string;
  roles: string[];
  isOrgAdmin: boolean;
};

export async function openAppelForCreneau(
  etablissementId: string,
  input: {
    dateAppel: string;
    creneauId?: string | null;
    classe?: string;
    heureDebut?: string | null;
    heureFin?: string | null;
    matiereLibelle?: string | null;
  },
  actor: VsAppelActor,
) {
  const dateAppel = input.dateAppel.trim();
  if (!dateAppel) throw new Error("dateAppel requis.");

  let classe = String(input.classe || "").trim();
  let heureDebut = input.heureDebut || null;
  let heureFin = input.heureFin || null;
  let matiereLibelle = input.matiereLibelle || null;
  const creneauId = input.creneauId || null;
  let groupeId: string | null = null;

  if (creneauId) {
    const jour = jourSemaineFromIsoDate(dateAppel);
    const creneaux = await listEdtCreneauxForJour(etablissementId, jour);
    const creneau = creneaux.find((c) => c.id === creneauId);
    if (!creneau) throw new Error("Créneau introuvable.");
    if (
      isProfesseurScopedAppelActor(actor) &&
      !professeurOwnsCreneau(creneau, { userId: actor.userId, displayName: actor.displayName })
    ) {
      throw new Error("Ce créneau n’est pas le vôtre.");
    }
    heureDebut = heureDebut || creneau.heureDebut;
    heureFin = heureFin || creneau.heureFin;
    matiereLibelle = matiereLibelle || creneau.matiereLibelle || creneau.enseignantNom || null;
    groupeId = creneau.groupeId ?? null;
    if (!classe) classe = creneau.groupeCode || creneau.classe || "";
  }

  if (!classe && !groupeId) {
    throw new Error("dateAppel et classe (ou créneau) requis.");
  }
  if (!classe && groupeId) classe = "groupe";

  const appel = await getOrCreateAppel(etablissementId, {
    dateAppel,
    classe,
    creneauId,
    heureDebut,
    heureFin,
    matiereLibelle,
    enseignantUserId: actor.userId,
    enseignantNom: actor.displayName,
  });
  return { appel, groupeId, classe };
}

export async function saveAppelLignesAction(
  etablissementId: string,
  appelId: string,
  lignes: VsAppelLigneInput[],
  actor: VsAppelActor,
) {
  await assertProfesseurCanAccessAppel(etablissementId, appelId, actor);
  const result = await saveAppelLignes(etablissementId, appelId, lignes);
  const data = await getAppelWithLignes(etablissementId, appelId);
  return { ...result, ...data };
}

export async function closeAppelAction(
  etablissementId: string,
  appelId: string,
  actor: VsAppelActor,
): Promise<CloseAppelResult> {
  await assertProfesseurCanAccessAppel(etablissementId, appelId, actor);
  const closed = await closeAppel(etablissementId, appelId, { actorUserId: actor.userId });
  if (!closed) throw new Error("Appel introuvable.");
  return closed;
}

export async function listCreneauxAppelForActor(
  etablissementId: string,
  dateAppel: string,
  actor: VsAppelActor,
  opts?: { classe?: string },
) {
  const jour = jourSemaineFromIsoDate(dateAppel);
  const creneaux = await listEdtCreneauxForJour(etablissementId, jour, { classe: opts?.classe });
  return filterCreneauxForProfesseur(creneaux, actor);
}
