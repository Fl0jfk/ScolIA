/** Textes des e-mails du Secret Santa — purs. Aucun ne contient le nom tiré. */

import { isRevealDue, type SantaRecord } from "@/app/lib/secret-santa";

export type SantaMailKind = "invitation" | "update" | "changed";

const dateParis = (date: Date) =>
  new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);

/** Informations pratiques : une ligne n'apparaît que si le champ est rempli. */
function details(santa: SantaRecord): string[] {
  const meeting = [santa.eventAt ? dateParis(santa.eventAt) : "", santa.eventPlace].filter(Boolean).join(", ");
  return [
    meeting ? `Rendez-vous : ${meeting}` : "",
    santa.drawnAt ? "" : `Tirage au sort : ${dateParis(santa.revealAt)} (heure de Paris)`,
    santa.budget ? `Budget : ${santa.budget}` : "",
    santa.message ? `\n${santa.message}` : "",
  ].filter(Boolean);
}

export function santaMail(
  kind: SantaMailKind,
  santa: SantaRecord,
  link: string,
  now: Date,
): { subject: string; lines: string[] } {
  const title = santa.title.replace(/[\r\n]+/g, " ");
  const access = `Votre lien (connexion à ScolIA demandée) : ${link}`;
  if (kind === "changed") {
    return {
      subject: `Secret Santa : rouvrez votre cadeau (${title})`,
      lines: [
        `Il y a eu un changement dans le Secret Santa « ${title} ».`,
        "Rouvrez votre cadeau avant de faire votre achat.",
        "",
        access,
      ],
    };
  }
  const drawn = santa.drawnAt !== null || isRevealDue(santa.revealAt, now);
  if (kind === "update") {
    return {
      subject: `Secret Santa : informations mises à jour (${title})`,
      lines: [
        `Les informations du Secret Santa « ${title} » ont été mises à jour.`,
        "",
        ...details(santa),
        // La même phrase pour tous : elle ne désigne personne.
        ...(santa.drawnAt ? ["Avant tout achat, rouvrez la page pour vérifier le nom tiré."] : []),
        "",
        access,
      ],
    };
  }
  return {
    subject: `Invitation au Secret Santa : ${title}`,
    lines: [
      `Vous avez été invité(e) par ${santa.organizerName} au Secret Santa « ${title} ».`,
      "",
      ...details(santa),
      "",
      ...(drawn
        ? [
            "Le tirage a eu lieu : ouvrez votre cadeau pour découvrir la personne à qui vous devez offrir un cadeau.",
            "Gardez ce nom secret.",
          ]
        : [
            "La participation est libre. Répondez avant le tirage : « Je participe » ou « Non merci ».",
            "À l'heure du tirage, les participants découvrent en même temps à qui offrir un cadeau.",
            "",
            "Le principe : chacun découvre le nom de la personne à qui il doit offrir un cadeau (et non celle qui lui en offrira un). Ce nom doit rester secret.",
          ]),
      "",
      access,
    ],
  };
}
