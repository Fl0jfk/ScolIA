/**
 * Autorisations de sortie des internes (sorties de classe organisées par un professeur).
 *
 * Règle centrale : un interne ne sort que si le PARENT et la DIRECTION ont dit oui.
 * Chaque élève a son propre statut : le refus d'un parent ne touche pas les autres élèves.
 *
 * Fichier pur (aucune entrée-sortie) : partagé entre le serveur, l'interface et les tests.
 */

export type SortieNiveau = "college" | "lycee" | null;

export type SortieDirection = {
  etablissement: string;
  niveau: SortieNiveau;
  token: string;
  email?: string;
  status: "pending" | "approved" | "refused";
  decidedAt?: string;
  decidedBy?: string;
  note?: string;
  emailSentAt?: string;
};

export type SortieEleve = {
  studentId: string;
  nom: string;
  prenom: string;
  classe: string;
  etablissement: string;
  token: string;
  /** Adresses des représentants légaux au moment de la création. */
  emails: string[];
  parentStatus: "pending" | "authorized" | "refused";
  respondedAt?: string;
  respondedBy?: string;
  /** « lien » = réponse du parent ; « manuel » = saisie tracée (papier, téléphone). */
  source?: "lien" | "manuel";
  note?: string;
  responseIp?: string;
  sentAt?: string;
  /** Dernier rappel, manuel ou automatique. */
  remindedAt?: string;
  /** Rappel automatique de J-2 (un clic manuel plus tôt ne le supprime pas). */
  relanceAutoAt?: string;
};

export type SortieClasse = {
  id: string;
  lieu: string;
  motif?: string;
  date: string;
  heureDepart: string;
  heureRetour: string;
  accompagnateurAller: string;
  accompagnateurRetour: string;
  createdBy: { userId: string; name: string; email: string };
  createdAt: string;
  updatedAt: string;
  cancelledAt?: string;
  cancelledBy?: string;
  directions: SortieDirection[];
  eleves: SortieEleve[];
  /** Horodatages d'envoi unique (idempotence). */
  recapCompletAt?: string;
  pointVeilleAt?: string;
};

export type StatutEleve =
  | "autorise"
  | "refus_parent"
  | "refus_direction"
  | "attente_parent"
  | "attente_direction"
  | "annulee";

export const STATUT_ELEVE_LABELS: Record<StatutEleve, string> = {
  autorise: "Autorisé à sortir",
  refus_parent: "Refusé par le parent",
  refus_direction: "Refusé par la direction",
  attente_parent: "Sans réponse du parent",
  attente_direction: "Parent d'accord — attente direction",
  annulee: "Sortie annulée",
};

export function directionDe(sortie: SortieClasse, eleve: SortieEleve): SortieDirection | undefined {
  return sortie.directions.find((d) => d.etablissement === eleve.etablissement);
}

export function statutEleve(sortie: SortieClasse, eleve: SortieEleve): StatutEleve {
  if (sortie.cancelledAt) return "annulee";
  const direction = directionDe(sortie, eleve);
  if (direction?.status === "refused") return "refus_direction";
  if (eleve.parentStatus === "refused") return "refus_parent";
  if (eleve.parentStatus === "pending") return "attente_parent";
  return direction?.status === "approved" ? "autorise" : "attente_direction";
}

export function compterStatuts(sortie: SortieClasse) {
  const out = { total: sortie.eleves.length, autorises: 0, refuses: 0, attente: 0 };
  for (const e of sortie.eleves) {
    const s = statutEleve(sortie, e);
    if (s === "autorise") out.autorises += 1;
    else if (s === "refus_parent" || s === "refus_direction") out.refuses += 1;
    else if (s !== "annulee") out.attente += 1;
  }
  return out;
}

/** Plus aucune réponse attendue (parents et directions). */
export function reponsesCompletes(sortie: SortieClasse): boolean {
  if (sortie.cancelledAt || !sortie.eleves.length) return false;
  return sortie.eleves.every((e) => {
    const s = statutEleve(sortie, e);
    return s !== "attente_parent" && s !== "attente_direction";
  });
}

/* ─── Dates (heure de Paris) ─── */

/** « AAAA-MM-JJTHH:MM » à l'heure de Paris : comparable par simple ordre alphabétique. */
export function parisStamp(now: Date): string {
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(now);
  const get = (type: string) => parts.find((p) => p.type === type)?.value ?? "00";
  return `${get("year")}-${get("month")}-${get("day")}T${get("hour")}:${get("minute")}`;
}

export function ajouterJours(date: string, jours: number): string {
  const d = new Date(`${date}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() + jours);
  return d.toISOString().slice(0, 10);
}

/** Après l'heure de départ, les liens ne permettent plus de répondre. */
export function sortiePartie(sortie: SortieClasse, now: Date = new Date()): boolean {
  return parisStamp(now) >= `${sortie.date}T${sortie.heureDepart}`;
}

export function sortieTerminee(sortie: SortieClasse, now: Date = new Date()): boolean {
  return parisStamp(now).slice(0, 10) > sortie.date;
}

const TROIS_HEURES_MS = 3 * 60 * 60_000;
const UNE_JOURNEE_MS = 24 * 60 * 60_000;

const UNE_HEURE_MS = 60 * 60_000;

/**
 * Envoi automatique aux parents sans réponse :
 * - invitation jamais partie : nouvel essai (1 h après la création) ;
 * - sinon rappel à partir de J-2, une seule fois, au moins 24 h après le dernier message.
 */
export function relanceDue(sortie: SortieClasse, eleve: SortieEleve, now: Date = new Date()): boolean {
  if (sortie.cancelledAt || sortiePartie(sortie, now)) return false;
  if (eleve.parentStatus !== "pending" || !eleve.emails.length) return false;
  if (directionDe(sortie, eleve)?.status === "refused") return false;
  if (!eleve.sentAt) return now.getTime() - new Date(sortie.createdAt).getTime() >= UNE_HEURE_MS;
  if (eleve.relanceAutoAt) return false;
  const dernier = eleve.remindedAt ?? eleve.sentAt;
  if (now.getTime() - new Date(dernier).getTime() < UNE_JOURNEE_MS) return false;
  return parisStamp(now).slice(0, 10) >= ajouterJours(sortie.date, -2);
}

/**
 * Point final : la veille à 12 h (heure de Paris). Pour une sortie créée plus tard,
 * il part 3 h après la création, afin de laisser le temps de répondre.
 */
export function pointVeilleDu(sortie: SortieClasse, now: Date = new Date()): boolean {
  if (sortie.cancelledAt || sortie.pointVeilleAt || sortiePartie(sortie, now)) return false;
  if (parisStamp(now) < `${ajouterJours(sortie.date, -1)}T12:00`) return false;
  if (now.getTime() - new Date(sortie.createdAt).getTime() >= TROIS_HEURES_MS) return true;
  // Départ dans moins de 3 h : on n'attend plus.
  return parisStamp(new Date(now.getTime() + TROIS_HEURES_MS)) >= `${sortie.date}T${sortie.heureDepart}`;
}

export function dateLongue(date: string): string {
  return new Date(`${date}T12:00:00Z`).toLocaleDateString("fr-FR", {
    timeZone: "UTC",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

export function heureLisible(heure: string): string {
  return heure.replace(":", " h ");
}

/* ─── Saisie ─── */

export function emailValide(raw: unknown): string {
  const e = String(raw || "").trim().toLowerCase();
  return /^[^\s@,;<>]+@[^\s@,;<>]+\.[^\s@,;<>]+$/.test(e) ? e : "";
}

export type SaisieSortie = {
  lieu: string;
  motif?: string;
  date: string;
  heureDepart: string;
  heureRetour: string;
  accompagnateurAller: string;
  accompagnateurRetour: string;
  studentIds: string[];
};

const MAX_ELEVES = 150;

function texte(raw: unknown, max: number): string {
  return String(raw ?? "")
    .replace(/[\u0000-\u001f\u007f]+/g, " ")
    .trim()
    .slice(0, max);
}

export function validerSaisie(
  body: Record<string, unknown>,
  now: Date = new Date(),
): { ok: true; saisie: SaisieSortie } | { ok: false; error: string } {
  const lieu = texte(body.lieu, 200);
  const motif = texte(body.motif, 300);
  const date = texte(body.date, 10);
  const heureDepart = texte(body.heureDepart, 5);
  const heureRetour = texte(body.heureRetour, 5);
  const accompagnateurAller = texte(body.accompagnateurAller, 120);
  const accompagnateurRetour = texte(body.accompagnateurRetour, 120);
  const studentIds = Array.isArray(body.studentIds)
    ? [...new Set(body.studentIds.map((x) => texte(x, 120)).filter(Boolean))]
    : [];

  const fail = (error: string) => ({ ok: false as const, error });
  if (!lieu) return fail("Indiquez le lieu de la sortie.");
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || ajouterJoursSur(date) !== date) return fail("Date de sortie invalide.");
  const heureOk = /^([01]\d|2[0-3]):[0-5]\d$/;
  if (!heureOk.test(heureDepart) || !heureOk.test(heureRetour)) {
    return fail("Indiquez l'heure de départ et l'heure de retour.");
  }
  if (heureRetour <= heureDepart) return fail("L'heure de retour doit être après l'heure de départ.");
  if (parisStamp(now) >= `${date}T${heureDepart}`) return fail("Le départ doit être dans le futur.");
  if (!accompagnateurAller || !accompagnateurRetour) {
    return fail("Indiquez l'accompagnateur à l'aller et au retour.");
  }
  if (!studentIds.length) return fail("Sélectionnez au moins un interne.");
  if (studentIds.length > MAX_ELEVES) return fail(`Pas plus de ${MAX_ELEVES} élèves par sortie.`);

  return {
    ok: true,
    saisie: {
      lieu,
      motif: motif || undefined,
      date,
      heureDepart,
      heureRetour,
      accompagnateurAller,
      accompagnateurRetour,
      studentIds,
    },
  };
}

/** Renvoie la date normalisée, ou une chaîne vide si elle n'existe pas (ex. 31 février). */
function ajouterJoursSur(date: string): string {
  const d = new Date(`${date}T12:00:00Z`);
  return Number.isNaN(d.getTime()) ? "" : d.toISOString().slice(0, 10);
}

/* ─── Droits ─── */

export type Acteur = {
  userId: string;
  email: string;
  roles: string[];
  /** Administrateur général de l'établissement. */
  isAdmin: boolean;
};

const ROLES_DIRECTION = ["direction", "direction_college", "direction_lycee", "direction_ecole"];

export function estDirection(acteur: Acteur): boolean {
  return acteur.roles.some((r) => ROLES_DIRECTION.includes(r));
}

export function peutCreer(acteur: Acteur): boolean {
  return acteur.isAdmin || acteur.roles.includes("professeur");
}

/** Choix des élèves, annulation, saisie manuelle : le créateur et l'administrateur général seulement. */
export function peutGerer(acteur: Acteur, sortie: SortieClasse): boolean {
  return acteur.isAdmin || sortie.createdBy.userId === acteur.userId;
}

export function peutDeciderDirection(acteur: Acteur, direction: SortieDirection): boolean {
  if (acteur.isAdmin || acteur.roles.includes("direction")) return true;
  if (direction.niveau === "college" && acteur.roles.includes("direction_college")) return true;
  if (direction.niveau === "lycee" && acteur.roles.includes("direction_lycee")) return true;
  const email = acteur.email.trim().toLowerCase();
  return estDirection(acteur) && Boolean(email) && direction.email?.trim().toLowerCase() === email;
}

/** Un professeur voit ses sorties ; la direction voit celles qui concernent son périmètre. */
export function peutVoir(acteur: Acteur, sortie: SortieClasse): boolean {
  if (peutGerer(acteur, sortie)) return true;
  return estDirection(acteur) && sortie.directions.some((d) => peutDeciderDirection(acteur, d));
}

/* ─── Réponses ─── */

export type Resultat = { ok: true } | { ok: false; error: string };

export function appliquerReponseParent(
  sortie: SortieClasse,
  studentId: string,
  reponse: {
    decision: "authorized" | "refused";
    source: "lien" | "manuel";
    par: string;
    note?: string;
    ip?: string;
  },
  now: Date = new Date(),
): Resultat {
  if (sortie.cancelledAt) return { ok: false, error: "Cette sortie a été annulée." };
  if (sortiePartie(sortie, now)) {
    return { ok: false, error: "L'heure de départ est passée : la réponse ne peut plus être modifiée." };
  }
  const eleve = sortie.eleves.find((e) => e.studentId === studentId);
  if (!eleve) return { ok: false, error: "Élève introuvable pour cette sortie." };
  if (eleve.parentStatus !== "pending") {
    // Seule correction permise : retirer une autorisation (saisie manuelle). Jamais transformer un refus en accord.
    const retrait =
      reponse.source === "manuel" && eleve.parentStatus === "authorized" && reponse.decision === "refused";
    if (!retrait) return { ok: false, error: "Une réponse est déjà enregistrée pour cet élève." };
  }
  if (reponse.source === "manuel" && !reponse.note?.trim()) {
    return { ok: false, error: "Précisez comment la réponse a été reçue (papier, appel…)." };
  }
  eleve.parentStatus = reponse.decision;
  eleve.source = reponse.source;
  eleve.respondedAt = now.toISOString();
  eleve.respondedBy = reponse.par;
  eleve.note = reponse.note?.trim().slice(0, 500) || undefined;
  eleve.responseIp = reponse.ip;
  sortie.updatedAt = now.toISOString();
  return { ok: true };
}

export function appliquerDecisionDirection(
  sortie: SortieClasse,
  etablissement: string,
  decision: { status: "approved" | "refused"; par: string; note?: string },
  now: Date = new Date(),
): Resultat {
  if (sortie.cancelledAt) return { ok: false, error: "Cette sortie a été annulée." };
  if (sortiePartie(sortie, now)) return { ok: false, error: "L'heure de départ est passée." };
  const direction = sortie.directions.find((d) => d.etablissement === etablissement);
  if (!direction) return { ok: false, error: "Établissement introuvable pour cette sortie." };
  if (direction.status !== "pending") {
    return { ok: false, error: "La direction a déjà répondu pour cet établissement." };
  }
  direction.status = decision.status;
  direction.decidedAt = now.toISOString();
  direction.decidedBy = decision.par;
  direction.note = decision.note?.trim().slice(0, 500) || undefined;
  sortie.updatedAt = now.toISOString();
  return { ok: true };
}

/** Jeton « <id de sortie>.<secret> » : permet de charger directement la bonne sortie. */
export function sortieIdDuJeton(token: string): string | null {
  const match = /^(sc_[a-z0-9]{8,40})\.[A-Za-z0-9_-]{30,80}$/.exec(token);
  return match ? match[1]! : null;
}

/* ─── Vues ─── */

/** Vue envoyée au navigateur d'un utilisateur connecté : sans jetons, adresses ni IP. */
export function vueSortie(sortie: SortieClasse, acteur: Acteur | null) {
  const gerable = acteur ? peutGerer(acteur, sortie) : false;
  return {
    id: sortie.id,
    lieu: sortie.lieu,
    motif: sortie.motif,
    date: sortie.date,
    heureDepart: sortie.heureDepart,
    heureRetour: sortie.heureRetour,
    accompagnateurAller: sortie.accompagnateurAller,
    accompagnateurRetour: sortie.accompagnateurRetour,
    creePar: sortie.createdBy.name,
    creeLe: sortie.createdAt,
    annulee: Boolean(sortie.cancelledAt),
    partie: sortiePartie(sortie),
    terminee: sortieTerminee(sortie),
    gerable,
    compteurs: compterStatuts(sortie),
    directions: sortie.directions.map((d) => ({
      etablissement: d.etablissement,
      status: d.status,
      decidedBy: d.decidedBy,
      note: d.note,
      peutDecider: acteur ? d.status === "pending" && peutDeciderDirection(acteur, d) : false,
      emailEnvoye: Boolean(d.emailSentAt),
    })),
    eleves: sortie.eleves
      .map((e) => ({
        studentId: e.studentId,
        nom: e.nom,
        prenom: e.prenom,
        classe: e.classe,
        statut: statutEleve(sortie, e),
        parentStatus: e.parentStatus,
        respondedAt: e.respondedAt,
        saisieManuelle: e.source === "manuel",
        note: gerable ? e.note : undefined,
        sansEmail: e.emails.length === 0,
        emailEnvoye: Boolean(e.sentAt),
      }))
      .sort((a, b) => a.classe.localeCompare(b.classe, "fr") || a.nom.localeCompare(b.nom, "fr")),
  };
}

export type VueSortie = ReturnType<typeof vueSortie>;
