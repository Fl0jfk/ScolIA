import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { listDirectoryMembers } from "@/app/lib/directory-members";
import { createTenantTransporter, getTenantSmtpConfig, sendMailWithTimeout } from "@/app/lib/tenant-mail";
import { getTenant, getTenantAppUrl } from "@/app/lib/tenant-context";
import { tenantCanonicalOrigin } from "@/app/lib/tenant-auth-urls";
import { modifierSortie } from "@/app/lib/sorties-classe-store";
import {
  STATUT_ELEVE_LABELS,
  compterStatuts,
  dateLongue,
  directionDe,
  heureLisible,
  pointVeilleDu,
  relanceDue,
  reponsesCompletes,
  reserverRappelDirection,
  sortiePartie,
  statutEleve,
  type SortieClasse,
  type SortieDirection,
  type SortieEleve,
} from "@/app/lib/sorties-classe";

/**
 * E-mails des autorisations de sortie. Aucun envoi ne fait échouer l'action métier : les échecs sont
 * comptés et remontés à l'écran. Chaque représentant légal reçoit son propre message (les adresses des
 * deux parents ne sont jamais mélangées).
 */

type Message = { to: string; subject: string; lines: Array<string | null>; replyTo?: string };

async function ouvrirMessagerie() {
  const smtp = await getTenantSmtpConfig();
  const transporter = smtp ? await createTenantTransporter() : null;
  if (!smtp || !transporter) {
    console.warn("[sorties-classe] SMTP non configuré.");
    return null;
  }
  const bundle = await loadAppConfig();
  const ecole = bundle.identity.shortName || bundle.identity.name || "Établissement";
  return async (message: Message): Promise<boolean> => {
    try {
      await sendMailWithTimeout(transporter, {
        from: `"Internat ${ecole.replace(/["\r\n]/g, "")}" <${smtp.user}>`,
        to: message.to,
        replyTo: message.replyTo || undefined,
        subject: message.subject.replace(/[\r\n]+/g, " "),
        text: [...message.lines.filter((l) => l !== null), "", "Cordialement,", ecole].join("\n"),
      });
      return true;
    } catch (e) {
      console.error("[sorties-classe] envoi en échec", e instanceof Error ? e.message : "erreur");
      return false;
    }
  };
}

type Envoyer = NonNullable<Awaited<ReturnType<typeof ouvrirMessagerie>>>;

/** Envoi à plusieurs destinataires : un message chacun. Vrai si au moins un est parti. */
async function envoyerAChacun(envoyer: Envoyer, destinataires: string[], message: Omit<Message, "to">) {
  const uniques = [...new Set(destinataires.map((d) => d.trim().toLowerCase()).filter(Boolean))];
  let parti = false;
  for (const to of uniques) {
    if (await envoyer({ ...message, to })) parti = true;
  }
  return parti;
}

function lignesSortie(s: SortieClasse): Array<string | null> {
  return [
    `Lieu : ${s.lieu}`,
    s.motif ? `Objet : ${s.motif}` : null,
    `Date : ${dateLongue(s.date)}`,
    `Départ : ${heureLisible(s.heureDepart)} — retour : ${heureLisible(s.heureRetour)}`,
    `Accompagnateur à l'aller : ${s.accompagnateurAller}`,
    `Accompagnateur au retour : ${s.accompagnateurRetour}`,
    `Organisateur : ${s.createdBy.name}`,
  ];
}

function nomEleve(e: SortieEleve) {
  return `${e.prenom} ${e.nom}`.trim();
}

function titre(s: SortieClasse) {
  return `${s.lieu} — ${new Date(`${s.date}T12:00:00Z`).toLocaleDateString("fr-FR", { timeZone: "UTC" })}`;
}

function listeStatuts(s: SortieClasse): string[] {
  const out: string[] = [];
  const groupes: Array<[string, (e: SortieEleve) => boolean]> = [
    ["AUTORISÉS À SORTIR", (e) => statutEleve(s, e) === "autorise"],
    [
      "NE SORTENT PAS (refus)",
      (e) => ["refus_parent", "refus_direction"].includes(statutEleve(s, e)),
    ],
    [
      "NE SORTENT PAS tant que la réponse manque",
      (e) => ["attente_parent", "attente_direction"].includes(statutEleve(s, e)),
    ],
  ];
  for (const [libelle, filtre] of groupes) {
    const eleves = s.eleves.filter(filtre);
    if (!eleves.length) continue;
    out.push("", `${libelle} (${eleves.length}) :`);
    for (const e of eleves) {
      out.push(`• ${nomEleve(e)} (${e.classe}) — ${STATUT_ELEVE_LABELS[statutEleve(s, e)]}`);
    }
  }
  return out;
}

async function membres(roles: string[]): Promise<string[]> {
  try {
    const rows = await listDirectoryMembers();
    return rows.filter((m) => m.roles.some((r) => roles.includes(r))).map((m) => m.email);
  } catch (e) {
    console.error("[sorties-classe] annuaire indisponible", e instanceof Error ? e.name : "error");
    return [];
  }
}

/**
 * Adresse du site pour les liens. Dans la tâche automatique il n'y a pas de requête web :
 * `getTenantAppUrl` lève alors une erreur, on prend l'adresse officielle de l'établissement.
 */
async function adresseSite(): Promise<string> {
  try {
    return await getTenantAppUrl();
  } catch {
    return tenantCanonicalOrigin(await getTenant());
  }
}

/** Adresse du directeur de l'établissement, sinon les comptes « direction » correspondants. */
async function adressesDirection(d: SortieDirection): Promise<string[]> {
  if (d.email) return [d.email];
  const roles = ["direction"];
  if (d.niveau === "college") roles.push("direction_college");
  if (d.niveau === "lycee") roles.push("direction_lycee");
  return membres(roles);
}

/** Équipe internat (rôle « internat »), à défaut les surveillants. */
async function adressesInternat(): Promise<string[]> {
  const internat = await membres(["internat"]);
  return internat.length ? internat : membres(["surveillant"]);
}

async function lienParent(e: SortieEleve) {
  return `${await adresseSite()}/internat/autorisation/classe?token=${encodeURIComponent(e.token)}`;
}

async function messageParent(envoyer: Envoyer, s: SortieClasse, e: SortieEleve, relance: boolean) {
  // Pas de « répondre à » : une réponse citant le message transmettrait le lien personnel.
  return envoyerAChacun(envoyer, e.emails, {
    subject: `${relance ? "Rappel — " : ""}[Internat] Autorisation de sortie pour ${nomEleve(e)}`,
    lines: [
      "Bonjour,",
      "",
      relance
        ? `Nous n'avons pas encore reçu votre réponse pour la sortie proposée à votre enfant ${nomEleve(e)} (${e.classe}).`
        : `Une sortie est proposée à votre enfant ${nomEleve(e)} (${e.classe}), interne dans notre établissement.`,
      "",
      ...lignesSortie(s),
      "",
      "Merci d'indiquer si vous l'autorisez à sortir de l'internat, avant l'heure de départ :",
      await lienParent(e),
      "",
      "Sans réponse de votre part, votre enfant ne pourra pas participer à la sortie.",
      "Ce lien est personnel : ne le transférez pas.",
    ],
  });
}

async function marquer(id: string, mutate: (s: SortieClasse) => void) {
  await modifierSortie(id, (s) => {
    mutate(s);
    return { ok: true };
  });
}

async function parLots<T>(items: T[], taille: number, fn: (item: T) => Promise<void>) {
  for (let i = 0; i < items.length; i += taille) {
    await Promise.all(items.slice(i, i + taille).map(fn));
  }
}

/**
 * Envoi unique : réservé sous verrou (jamais pour une sortie annulée), libéré si l'envoi échoue
 * ou lève une erreur, pour être réessayé au passage suivant.
 */
async function envoiUnique(
  id: string,
  champ: "recapCompletAt" | "pointVeilleAt",
  envoyerCeMessage: (envoyer: Envoyer, sortie: SortieClasse) => Promise<boolean>,
): Promise<boolean> {
  const reservation = await modifierSortie(id, (s) => {
    if (s[champ] || s.cancelledAt) return { ok: false, error: "déjà envoyé ou annulée" };
    s[champ] = new Date().toISOString();
    return { ok: true };
  });
  if (!reservation.ok) return false;
  let parti = false;
  try {
    const envoyer = await ouvrirMessagerie();
    parti = Boolean(envoyer) && (await envoyerCeMessage(envoyer!, reservation.sortie));
  } catch (e) {
    console.error("[sorties-classe] envoi unique en échec", e instanceof Error ? e.name : "error");
  }
  if (!parti) {
    await marquer(id, (cur) => {
      cur[champ] = undefined;
    });
  }
  return parti;
}

/** Demande de validation à la direction (première fois ou rappel, avec l'état à jour des réponses). */
async function messageDirection(envoyer: Envoyer, s: SortieClasse, d: SortieDirection, rappel: boolean) {
  const base = await adresseSite();
  const eleves = s.eleves.filter((e) => e.etablissement === d.etablissement);
  return envoyerAChacun(envoyer, await adressesDirection(d), {
    subject: `${rappel ? "Rappel — " : ""}[Internat] Sortie à valider — ${titre(s)}`,
    lines: [
      "Bonjour,",
      "",
      rappel
        ? `Votre validation est attendue pour la sortie organisée par ${s.createdBy.name} avec des internes (${d.etablissement}).`
        : `${s.createdBy.name} organise une sortie avec des internes (${d.etablissement}). Les parents sont prévenus en même temps que vous.`,
      "",
      ...lignesSortie(s),
      "",
      "Élèves concernés :",
      ...eleves.map(
        (e) => `• ${nomEleve(e)} (${e.classe})${rappel ? ` — ${STATUT_ELEVE_LABELS[statutEleve(s, e)]}` : ""}`,
      ),
      "",
      "Un élève ne sortira que si son parent ET la direction ont donné leur accord.",
      "Valider ou refuser (lien personnel, ne le transférez pas) :",
      `${base}/internat/autorisation/classe?token=${encodeURIComponent(d.token)}`,
      "",
      `Suivi des réponses : ${base}/gestion-internat?tab=sorties`,
    ],
  });
}

/** Rappel manuel à la direction. Réservé sous verrou, libéré si l'e-mail ne part pas. */
export async function relancerDirection(
  id: string,
  etablissement: string,
): Promise<{ ok: true; sortie: SortieClasse } | { ok: false; error: string; status: number }> {
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return { ok: false, error: "La messagerie n'est pas configurée.", status: 502 };
  let precedent: string | undefined;
  const reservation = await modifierSortie(id, (s) => {
    precedent = s.directions.find((d) => d.etablissement === etablissement)?.remindedAt;
    return reserverRappelDirection(s, etablissement);
  });
  if (!reservation.ok) return reservation;
  const direction = reservation.sortie.directions.find((d) => d.etablissement === etablissement)!;
  let parti = false;
  try {
    parti = await messageDirection(envoyer, reservation.sortie, direction, true);
  } catch (e) {
    console.error("[sorties-classe] rappel direction en échec", e instanceof Error ? e.name : "error");
  }
  await marquer(id, (cur) => {
    const x = cur.directions.find((d) => d.etablissement === etablissement);
    if (!x) return;
    if (parti) x.emailSentAt ??= x.remindedAt;
    else x.remindedAt = precedent;
  });
  if (!parti) {
    return {
      ok: false,
      error: "L'e-mail à la direction n'est pas parti : vérifiez l'adresse du directeur dans la fiche de l'établissement.",
      status: 502,
    };
  }
  return reservation;
}

/** Envoi initial : parents et direction en même temps. Chaque envoi réussi est noté aussitôt. */
export async function envoyerInvitations(s: SortieClasse): Promise<string[]> {
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return ["La messagerie n'est pas configurée : aucun e-mail n'est parti."];
  const avertissements: string[] = [];

  await parLots(s.eleves, 5, async (e) => {
    if (!e.emails.length) {
      avertissements.push(`${nomEleve(e)} : aucun e-mail parent, réponse à recueillir autrement.`);
    } else if (await messageParent(envoyer, s, e, false)) {
      await marquer(s.id, (cur) => {
        const x = cur.eleves.find((y) => y.studentId === e.studentId);
        if (x) x.sentAt = new Date().toISOString();
      });
    } else {
      avertissements.push(`${nomEleve(e)} : l'e-mail aux parents n'est pas parti.`);
    }
  });

  for (const d of s.directions) {
    if (await messageDirection(envoyer, s, d, false)) {
      await marquer(s.id, (cur) => {
        const x = cur.directions.find((y) => y.etablissement === d.etablissement);
        if (x) x.emailSentAt = new Date().toISOString();
      });
    } else {
      avertissements.push(`La direction (${d.etablissement}) n'a pas reçu d'e-mail : prévenez-la.`);
    }
  }
  return avertissements;
}

/** Récapitulatif au professeur, avec la direction et l'équipe internat. */
async function envoyerRecap(envoyer: Envoyer, s: SortieClasse, moment: "complet" | "veille") {
  const c = compterStatuts(s);
  const lines = [
    "Bonjour,",
    "",
    moment === "complet"
      ? "Toutes les réponses sont arrivées pour cette sortie."
      : "Voici le point sur les autorisations avant la sortie.",
    "",
    ...lignesSortie(s),
    ...listeStatuts(s),
    "",
    `Suivi en direct : ${await adresseSite()}/gestion-internat?tab=sorties`,
  ];
  const subject = `[Sortie] ${moment === "complet" ? "Réponses complètes" : "Point avant sortie"} : ${c.autorises}/${c.total} autorisés — ${titre(s)}`;
  const directions = (await Promise.all(s.directions.map(adressesDirection))).flat();
  return envoyerAChacun(envoyer, [s.createdBy.email, ...directions, ...(await adressesInternat())], {
    subject,
    lines,
  });
}

export async function envoyerRecapSiComplet(s: SortieClasse): Promise<void> {
  if (s.recapCompletAt || !reponsesCompletes(s)) return;
  await envoiUnique(s.id, "recapCompletAt", (envoyer, cur) => envoyerRecap(envoyer, cur, "complet"));
}

/** Changement survenu après un récapitulatif déjà envoyé : l'internat doit le savoir. */
async function signalerChangementTardif(envoyer: Envoyer, s: SortieClasse, phrase: string) {
  if (!s.recapCompletAt && !s.pointVeilleAt) return;
  // Le récapitulatif complet part juste après : pas de doublon.
  if (!s.recapCompletAt && reponsesCompletes(s)) return;
  await envoyerAChacun(envoyer, await adressesInternat(), {
    subject: `[Sortie] Mise à jour — ${titre(s)}`,
    lines: ["Bonjour,", "", phrase, "", ...lignesSortie(s), ...listeStatuts(s)],
  });
}

export async function apresReponseParent(s: SortieClasse, studentId: string): Promise<void> {
  const eleve = s.eleves.find((e) => e.studentId === studentId);
  if (!eleve) return;
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return;
  const refus = eleve.parentStatus === "refused";
  if (refus) {
    await envoyerAChacun(envoyer, [s.createdBy.email], {
      subject: `[Sortie] Refus : ${nomEleve(eleve)} — ${titre(s)}`,
      lines: [
        "Bonjour,",
        "",
        `${nomEleve(eleve)} (${eleve.classe}) n'est pas autorisé à sortir : refus du représentant légal.`,
        eleve.note ? `Commentaire : ${eleve.note}` : null,
        "",
        ...lignesSortie(s),
      ],
    });
  }
  await signalerChangementTardif(
    envoyer,
    s,
    `${nomEleve(eleve)} (${eleve.classe}) : ${STATUT_ELEVE_LABELS[statutEleve(s, eleve)]}.`,
  );
  await envoyerRecapSiComplet(s);
}

export async function apresDecisionDirection(s: SortieClasse, etablissement: string): Promise<void> {
  const d = s.directions.find((x) => x.etablissement === etablissement);
  if (!d) return;
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return;
  const accord = d.status === "approved";
  await envoyerAChacun(envoyer, [s.createdBy.email], {
    subject: `[Sortie] Direction : ${accord ? "accord" : "refus"} — ${titre(s)}`,
    lines: [
      "Bonjour,",
      "",
      `La direction (${d.etablissement}) a ${accord ? "validé" : "refusé"} la sortie.`,
      d.note ? `Commentaire : ${d.note}` : null,
      "",
      ...lignesSortie(s),
      ...listeStatuts(s),
    ],
  });
  if (!accord) {
    for (const e of s.eleves.filter((x) => directionDe(s, x) === d)) {
      await envoyerAChacun(envoyer, e.emails, {
        replyTo: s.createdBy.email,
        subject: `[Internat] Sortie non maintenue pour ${nomEleve(e)}`,
        lines: [
          "Bonjour,",
          "",
          `La direction n'a pas validé la sortie : ${nomEleve(e)} ne sortira pas de l'internat à cette occasion. Vous n'avez rien à faire.`,
          "",
          ...lignesSortie(s),
        ],
      });
    }
  }
  await signalerChangementTardif(
    envoyer,
    s,
    `La direction (${d.etablissement}) a ${accord ? "validé" : "refusé"} la sortie.`,
  );
  await envoyerRecapSiComplet(s);
}

/** Renvoie des avertissements si des personnes n'ont pas pu être prévenues. */
export async function apresAnnulation(s: SortieClasse): Promise<string[]> {
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return ["Aucun e-mail d'annulation n'est parti : prévenez les familles et l'internat."];
  const lines = [
    "Bonjour,",
    "",
    "La sortie suivante est ANNULÉE. Les autorisations données ne sont plus valables.",
    "",
    ...lignesSortie(s),
  ];
  const subject = `[Internat] Sortie annulée — ${titre(s)}`;
  const avertissements: string[] = [];
  // Toutes les familles, même si l'envoi initial n'a pas été noté.
  await parLots(s.eleves, 5, async (e) => {
    if (e.emails.length && !(await envoyerAChacun(envoyer, e.emails, { subject, lines }))) {
      avertissements.push(`Les parents de ${nomEleve(e)} n'ont pas reçu l'annulation.`);
    }
  });
  const internat = await adressesInternat();
  if (!(await envoyerAChacun(envoyer, internat, { subject, lines }))) {
    avertissements.push("L'équipe internat n'a pas reçu l'annulation.");
  }
  const directions = (await Promise.all(s.directions.map(adressesDirection))).flat();
  await envoyerAChacun(envoyer, [s.createdBy.email, ...directions], { subject, lines });
  return avertissements;
}

/**
 * Rappel (manuel ou automatique) aux parents sans réponse. Chaque envoi est réservé sous verrou,
 * sur l'état à jour : une réponse, une annulation ou un autre envoi entre-temps l'empêche.
 */
export async function relancerParents(
  s: SortieClasse,
  eleves: SortieEleve[],
  origine: "manuelle" | "automatique",
): Promise<number> {
  const envoyer = await ouvrirMessagerie();
  if (!envoyer) return 0;
  let envoyes = 0;
  for (const vu of eleves) {
    const at = new Date().toISOString();
    const reservation = await modifierSortie(s.id, (cur) => {
      const x = cur.eleves.find((y) => y.studentId === vu.studentId);
      if (
        !x ||
        cur.cancelledAt ||
        sortiePartie(cur) ||
        x.parentStatus !== "pending" ||
        !x.emails.length ||
        directionDe(cur, x)?.status === "refused" ||
        x.remindedAt !== vu.remindedAt ||
        x.sentAt !== vu.sentAt
      ) {
        return { ok: false, error: "plus à relancer" };
      }
      x.remindedAt = at;
      if (origine === "automatique" && x.sentAt) x.relanceAutoAt = at;
      return { ok: true };
    });
    if (!reservation.ok) continue;
    const frais = reservation.sortie.eleves.find((y) => y.studentId === vu.studentId)!;
    let parti = false;
    try {
      parti = await messageParent(envoyer, reservation.sortie, frais, Boolean(vu.sentAt));
    } catch (e) {
      console.error("[sorties-classe] rappel en échec", e instanceof Error ? e.name : "error");
    }
    if (parti) envoyes += 1;
    await marquer(s.id, (cur) => {
      const x = cur.eleves.find((y) => y.studentId === vu.studentId);
      if (!x) return;
      if (parti) x.sentAt ??= at;
      else {
        x.remindedAt = vu.remindedAt;
        x.relanceAutoAt = vu.relanceAutoAt;
      }
    });
  }
  return envoyes;
}

/** Passage automatique : rappels, récapitulatif complet resté en échec, point final de la veille. */
export async function traiterEcheances(s: SortieClasse, now: Date = new Date()) {
  const aRelancer = s.eleves.filter((e) => relanceDue(s, e, now));
  const relances = aRelancer.length ? await relancerParents(s, aRelancer, "automatique") : 0;
  await envoyerRecapSiComplet(s);
  const points =
    pointVeilleDu(s, now) &&
    (await envoiUnique(s.id, "pointVeilleAt", (envoyer, cur) => envoyerRecap(envoyer, cur, "veille")))
      ? 1
      : 0;
  return { relances, points };
}
