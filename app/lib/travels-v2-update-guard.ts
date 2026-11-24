/**
 * Circuit voyages v2 — garde serveur de `/api/travels/update` (logique PURE, testable sans serveur).
 *
 * Le client renvoie TOUT le dossier à chaque enregistrement : une fenêtre restée ouverte ou un client malveillant ne doit
 * jamais pouvoir réécrire ce que le serveur pilote (clôture, purge, liste officielle, ouverture de la page…), rouvrir ou
 * rétrograder un voyage, ni modifier ce qui est figé après l'ouverture (pièces, prix, date limite sauf prolongation).
 * Ne s'applique QU'AUX voyages v2 (`workflowVersion === 2`) : le circuit historique (v1) et les séries récurrentes sont intacts.
 */

import {
  addDaysIso,
  evaluatePriceBlockChange,
  isIsoDate,
  REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE,
  tripDepartureIso,
} from "@/app/lib/travels-v2";

/* ─── Rôle d'écriture ─── */

export type V2Role = "writer" | "compta" | "none";

/**
 * - `writer` : organisateur, direction, administratif, admin général (droit d'écriture complet) ;
 * - `compta` : la comptabilité seule — uniquement son bloc prix, sa fiche budget et la validation des finances ;
 * - `none` : aucun droit sur ce voyage.
 */
export function v2RoleFor(a: {
  isOwner: boolean;
  isDirection: boolean;
  isAdmin: boolean;
  isAdministratif: boolean;
  isCompta: boolean;
}): V2Role {
  if (a.isOwner || a.isDirection || a.isAdmin || a.isAdministratif) return "writer";
  return a.isCompta ? "compta" : "none";
}

/** Champs de `data` que la comptabilité (seule) peut enregistrer. */
export const COMPTA_DATA_KEYS = [
  "priceBlock",
  "comptaSheet",
  "finalTotalCost",
  "costPerStudent",
  "coutTotal",
  "previousStatus",
  "modificationRequestNote",
] as const;

/* ─── Transitions de statut ─── */

const CREATION_STATUSES = ["BROUILLON", "EN_ATTENTE_VALIDATION", "EN_ATTENTE_DIR_INITIAL"];
/** Étapes AVANT l'ouverture des inscriptions : circulation libre entre elles (comme en v1). */
const PRE_OPEN_STATUSES = [
  "BROUILLON",
  "EN_ATTENTE_VALIDATION",
  "EN_ATTENTE_DIR_INITIAL",
  "PROF_LOGISTICS",
  "EN_ATTENTE_BUS_SIGNATURE",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
  "BESOIN_MODIFICATION",
];
const TERMINAL_STATUSES = ["ANNULE", "SEANCE_ANNULEE", "REJETE"];
const CANCEL_STATUSES = ["ANNULE", "SEANCE_ANNULEE"];

export type GuardError = { ok: false; status: number; error: string };

/**
 * Liste blanche des changements de statut d'un voyage v2 par `/api/travels/update` :
 * - ouverture des inscriptions : seulement depuis la validation finale, par la direction / un admin ;
 * - VALIDE (clôture) et au-delà : jamais ici (clôture = `/api/travels/v2/closure` ou le job) ;
 * - un voyage ouvert, clôturé ou terminé ne peut être ni rétrogradé ni rouvert (seule l'annulation reste possible).
 */
export function checkV2StatusTransition(opts: {
  previous: string | null;
  next: string;
  role: V2Role;
  actor: { isDirectionOfEtab: boolean; isGlobalAdmin: boolean };
}): { ok: true } | GuardError {
  const { previous, next, role, actor } = opts;
  if (previous === null) {
    return CREATION_STATUSES.includes(next)
      ? { ok: true }
      : { ok: false, status: 400, error: "Statut initial invalide pour un nouveau voyage." };
  }
  if (next === previous) return { ok: true };
  if (TERMINAL_STATUSES.includes(previous)) {
    return { ok: false, status: 409, error: "Ce voyage est clos (annulé ou refusé) : son statut ne peut plus changer." };
  }
  if (role === "compta") {
    // Validation des finances : seul mouvement permis à la comptabilité seule.
    const ok = previous === "EN_ATTENTE_COMPTA" && (next === "EN_ATTENTE_DIR_FINAL" || next === "BESOIN_MODIFICATION");
    return ok ? { ok: true } : { ok: false, status: 403, error: "La comptabilité ne peut changer le statut qu'à la validation des finances." };
  }
  if (previous === "INSCRIPTIONS_OUVERTES" || previous === "VALIDE" || previous === "FINALISE_DIR_ATTENTE_ELEVES") {
    if (CANCEL_STATUSES.includes(next)) return { ok: true };
    return {
      ok: false,
      status: 409,
      error:
        "Les inscriptions sont ouvertes ou clôturées : le statut est piloté par le serveur (clôture, purge). " +
        "Rechargez la page : votre fenêtre est peut-être périmée.",
    };
  }
  if (!PRE_OPEN_STATUSES.includes(previous)) {
    return { ok: false, status: 409, error: "Changement de statut non autorisé." };
  }
  if (next === "INSCRIPTIONS_OUVERTES") {
    if (previous !== "EN_ATTENTE_DIR_FINAL") {
      return { ok: false, status: 400, error: "Les inscriptions ne peuvent être ouvertes qu'à l'issue de la validation finale." };
    }
    if (!(actor.isDirectionOfEtab || actor.isGlobalAdmin)) {
      return {
        ok: false,
        status: 403,
        error: "Seule la direction de l'établissement concerné (ou un administrateur général) peut ouvrir les inscriptions des familles.",
      };
    }
    return { ok: true };
  }
  if (next === "VALIDE" || next === "FINALISE_DIR_ATTENTE_ELEVES") {
    return { ok: false, status: 409, error: "Un voyage v2 est finalisé uniquement par la clôture des inscriptions." };
  }
  if (PRE_OPEN_STATUSES.includes(next) || next === "REJETE" || CANCEL_STATUSES.includes(next)) return { ok: true };
  return { ok: false, status: 400, error: "Statut inconnu." };
}

/* ─── Pièces demandées : comparaison canonique ─── */

function canonicalDocs(docs: unknown): string {
  if (!Array.isArray(docs)) return "[]";
  return JSON.stringify(
    [...docs]
      .map((d: Record<string, unknown>) => ({
        id: String(d?.id ?? ""),
        label: String(d?.label ?? "").trim(),
        kind: String(d?.kind ?? ""),
        dueDate: String(d?.dueDate ?? ""),
        tpl: String((d?.template as Record<string, unknown> | undefined)?.name ?? ""),
      }))
      .sort((a, b) => a.id.localeCompare(b.id)),
  );
}

/* ─── Garde principal ─── */

export type GuardResult = { ok: true; data: Record<string, unknown> } | GuardError;

const SERVER_PILOTED_KEYS = ["transportRequestSentAt", "listeElevesStatus", "listeElevesConfirmedAt", "listeElevesConfirmedBy", "listeEnvoyeeTransporteurAt"];

function isServerPrefixed(k: string): boolean {
  return /^v2[A-Z_]/.test(k);
}

/**
 * Retourne le `data` à enregistrer : celui du client, après
 *  1. refus d'une série récurrente en v2 ;
 *  2. contrôle du droit d'écriture et de la transition de statut ;
 *  3. réinjection, depuis l'existant, de TOUS les champs pilotés par le serveur ;
 *  4. gel après ouverture : pièces et prix figés, date limite seulement reculée (prolongation, plafonnée à J-14).
 * `existingData` = null à la création.
 */
export function guardV2Update(opts: {
  existingData: Record<string, unknown> | null;
  incomingData: Record<string, unknown>;
  previousStatus: string | null;
  newStatus: string;
  role: V2Role;
  actor: { isDirectionOfEtab: boolean; isGlobalAdmin: boolean };
}): GuardResult {
  const { existingData, incomingData, previousStatus, newStatus, role, actor } = opts;

  // 1. Les séries récurrentes restent en v1.
  if (incomingData.recurrenceSeriesId || existingData?.recurrenceSeriesId) {
    return { ok: false, status: 400, error: "Une série récurrente ne peut pas utiliser le nouveau circuit des voyages." };
  }

  // 2. Droit d'écriture et statut.
  if (existingData && role === "none") {
    return { ok: false, status: 403, error: "Vous n'avez pas le droit de modifier ce voyage." };
  }
  const transition = checkV2StatusTransition({ previous: existingData ? previousStatus : null, next: newStatus, role, actor });
  if (!transition.ok) return transition;

  let out: Record<string, unknown> = { ...incomingData };

  // La comptabilité seule : uniquement ses champs, tout le reste vient de l'existant.
  if (existingData && role === "compta") {
    out = { ...existingData };
    for (const k of COMPTA_DATA_KEYS) {
      if (k in incomingData) out[k] = incomingData[k];
      else delete out[k];
    }
  }

  if (!existingData) {
    // Création : aucun champ piloté par le serveur ne vient du client.
    for (const k of Object.keys(out)) if (isServerPrefixed(k)) delete out[k];
    for (const k of SERVER_PILOTED_KEYS) delete out[k];
    if (out.familyPage && typeof out.familyPage === "object") {
      const page = { ...(out.familyPage as Record<string, unknown>) };
      delete page.openedAt;
      delete page.openedBy;
      out.familyPage = page;
    }
    return { ok: true, data: out };
  }

  // 3. Réinjection des champs pilotés par le serveur.
  for (const k of Object.keys(out)) if (isServerPrefixed(k) && !(k in existingData)) delete out[k];
  for (const k of Object.keys(existingData)) if (isServerPrefixed(k)) out[k] = existingData[k];
  for (const k of SERVER_PILOTED_KEYS) {
    if (k in existingData) out[k] = existingData[k];
    else delete out[k];
  }
  if (existingData.v2Closure) {
    // Liste officielle et effectifs posés par la clôture.
    for (const k of ["participantEleves", "nbEleves", "classes"]) {
      if (k in existingData) out[k] = existingData[k];
      else delete out[k];
    }
  }

  const existingPage = (existingData.familyPage as Record<string, unknown> | undefined) ?? undefined;
  const opened = Boolean(existingPage?.openedAt);
  let page = (out.familyPage as Record<string, unknown> | undefined) ? { ...(out.familyPage as Record<string, unknown>) } : undefined;

  if (opened) {
    // Page ouverte : on ne la perd pas si la fenêtre n'en avait pas, et l'horodatage d'ouverture reste celui du serveur.
    if (!page) page = { ...existingPage! };
    page.openedAt = existingPage!.openedAt;
    if (existingPage!.openedBy !== undefined) page.openedBy = existingPage!.openedBy;
    else delete page.openedBy;

    // 4a. Pièces demandées : figées.
    if (page.documents === undefined) page.documents = existingPage!.documents;
    else if (canonicalDocs(page.documents) !== canonicalDocs(existingPage!.documents)) {
      return { ok: false, status: 403, error: "La page des familles est ouverte : les pièces demandées sont figées." };
    } else page.documents = existingPage!.documents;

    // 4b. Date limite : seulement prolongée, jamais au-delà de J-14.
    const oldDeadline = existingPage!.deadline as string | undefined;
    const newDeadline = page.deadline as string | undefined;
    if (newDeadline === undefined || newDeadline === oldDeadline) {
      if (oldDeadline !== undefined) page.deadline = oldDeadline;
    } else {
      if (!isIsoDate(newDeadline)) return { ok: false, status: 400, error: "Date limite invalide." };
      if (isIsoDate(oldDeadline) && newDeadline < oldDeadline) {
        return {
          ok: false,
          status: 400,
          error: "La page est ouverte : la date limite ne peut que être reculée (prolongation), jamais avancée.",
        };
      }
      const departure = tripDepartureIso(out as Parameters<typeof tripDepartureIso>[0]);
      if (departure) {
        const cap = addDaysIso(departure, -REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE);
        if (newDeadline > cap) {
          return {
            ok: false,
            status: 400,
            error: `La date limite ne peut pas dépasser J-${REGISTRATION_CLOSE_DAYS_BEFORE_DEPARTURE} avant le départ (le ${cap}).`,
          };
        }
      }
    }
    out.familyPage = page;

    // 4c. Prix : figé pour tout le monde (compta comprise) une fois la page ouverte.
    const priceChange = evaluatePriceBlockChange({
      existing: existingData.priceBlock as Record<string, unknown> | undefined,
      incoming: out.priceBlock as Record<string, unknown> | undefined,
      isCompta: false,
    });
    if (priceChange === "forbidden") {
      return { ok: false, status: 403, error: "La page des familles est ouverte : le prix et les mois facturés sont figés." };
    }
    if (existingData.priceBlock !== undefined) out.priceBlock = existingData.priceBlock;
    else delete out.priceBlock;
  } else if (page) {
    // Pas encore ouverte : le client ne pose jamais l'horodatage d'ouverture (posé par le serveur à la validation finale).
    delete page.openedAt;
    delete page.openedBy;
    out.familyPage = page;
  }

  return { ok: true, data: out };
}
