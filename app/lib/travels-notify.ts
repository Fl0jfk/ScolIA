import { loadAppConfig } from "@/app/lib/app-config";
import { createTenantTransporter, getTenantSmtpConfig } from "@/app/lib/tenant-mail";
import { tenantAbsolutePath } from "@/app/lib/tenant-context";

export type TravelsTripForNotify = {
  ownerName?: string;
  ownerEmail?: string;
  type?: string;
  status?: string;
  data?: {
    title?: string;
    destination?: string;
    etablissement?: string;
    startDate?: string;
    endDate?: string;
    date?: string;
    nbEleves?: string | number;
    nbAccompagnateurs?: string | number;
    coutTotal?: string | number;
    piqueNiqueDetails?: { active?: boolean } | null;
  };
  history?: Array<{ action?: string; note?: string; user?: string; date?: string }>;
};

export async function notifyComptaTravelsPhase(params: {
  tripId: string;
  trip: TravelsTripForNotify;
  previousStatus?: string | null;
}) {
  const smtp = await getTenantSmtpConfig();
  if (!smtp) {
    console.warn("[travels-notify] SMTP non configuré — pas d’e-mail compta.");
    return;
  }
  const transporter = await createTenantTransporter();
  if (!transporter) return;

  const d = params.trip.data || {};
  const title = String(d.title || "Sans titre");
  const destination = String(d.destination || "—");
  const etab = String(d.etablissement || "Groupe scolaire");
  const typeLabel = params.trip.type === "COMPLEX" ? "Voyage complexe" : "Sortie simple";
  const dateInfo =
    d.startDate || d.endDate
      ? `du ${d.startDate || d.date || "—"} au ${d.endDate || d.date || "—"}`
      : String(d.date || "—");
  const lastHistory = params.trip.history?.length
    ? params.trip.history[params.trip.history.length - 1]
    : undefined;
  const transitionNote = lastHistory?.note?.trim() || "";
  const transitionBy = lastHistory?.user?.trim() || "";
  const fromStatus = params.previousStatus?.trim() || "—";
  const link = await tenantAbsolutePath(`/travels/${params.tripId}`);
  const bundle = await loadAppConfig();
  const toList =
    bundle.travels.comptaEmails?.length > 0
      ? bundle.travels.comptaEmails
      : bundle.notifications.travelsCompta;
  if (!toList.length) {
    console.warn("[travels-notify] Aucun email compta configuré.");
    return;
  }

  await transporter.sendMail({
    from: `"Plateforme Voyages" <${smtp.user}>`,
    to: toList.join(", "),
    subject: `[Travels] Dossier en attente comptabilité — ${title}`,
    text: [
      `Bonjour,`,
      ``,
      `Un dossier de sortie vient d’être transmis à l’étape Finances (comptabilité) sur Travels.`,
      ``,
      `Titre : ${title}`,
      `Type : ${typeLabel}`,
      `Établissement : ${etab}`,
      `Destination : ${destination}`,
      `Dates : ${dateInfo}`,
      `Créé par : ${params.trip.ownerName || "—"}`,
      `Effectif : ${d.nbEleves ?? "—"} élèves / ${d.nbAccompagnateurs ?? "—"} accompagnateurs`,
      d.coutTotal != null && d.coutTotal !== "" ? `Budget prévisionnel : ${d.coutTotal} €` : "",
      ``,
      `Étape précédente : ${fromStatus}`,
      transitionNote ? `Dernière action : ${transitionNote}` : "",
      transitionBy ? `Par : ${transitionBy}` : "",
      ``,
      `Consulter le dossier : ${link}`,
      ``,
      `Cordialement,`,
      `Plateforme Voyages — ${bundle.identity.shortName || bundle.identity.name}`,
    ]
      .filter(Boolean)
      .join("\n"),
  });
}

/**
 * Après validation finale direction : demande au professeur organisateur
 * de finaliser la liste nominative des élèves (bloque la commande cuisine).
 */
export async function notifyOrganizerListeElevesNeeded(params: {
  tripId: string;
  trip: TravelsTripForNotify;
}): Promise<{ sent: boolean; reason?: string }> {
  const to = String(params.trip.ownerEmail || "").trim();
  if (!to) {
    console.warn("[travels-notify] Pas d’e-mail organisateur — notification liste élèves ignorée.");
    return { sent: false, reason: "Pas d’e-mail organisateur" };
  }

  const smtp = await getTenantSmtpConfig();
  if (!smtp) {
    console.warn("[travels-notify] SMTP non configuré — pas d’e-mail liste élèves.");
    return { sent: false, reason: "SMTP non configuré" };
  }
  const transporter = await createTenantTransporter();
  if (!transporter) return { sent: false, reason: "SMTP non configuré" };

  const d = params.trip.data || {};
  const title = String(d.title || "Sans titre");
  const destination = String(d.destination || "—");
  const etab = String(d.etablissement || "Groupe scolaire");
  const typeLabel = params.trip.type === "COMPLEX" ? "Voyage complexe" : "Sortie simple";
  const dateInfo =
    d.startDate || d.endDate
      ? `du ${d.startDate || d.date || "—"} au ${d.endDate || d.date || "—"}`
      : String(d.date || "—");
  const cuisineActive = Boolean(d.piqueNiqueDetails?.active);
  const link = await tenantAbsolutePath(`/travels/${params.tripId}?tab=eleves`);
  const bundle = await loadAppConfig();
  const orgName = bundle.identity.shortName || bundle.identity.name || "l’établissement";

  const cuisineBlock = cuisineActive
    ? [
        ``,
        `IMPORTANT — Commande de cantine :`,
        `Tant que la liste des élèves n’est pas finalisée (confirmée) ET que les paniers`,
        `n’ont pas été attribués nominativement (« qui mange »),`,
        `le bon de commande cuisine n’est PAS envoyé au chef,`,
        `et la collègue décompte ne reçoit pas la liste des élèves qui mangent.`,
        `Dès confirmation : le dossier passe en « Finalisé », la commande part au chef,`,
        `et la collègue reçoit la commande + la liste « qui mange ».`,
      ]
    : [
        ``,
        `Dès que vous aurez confirmé la liste, le dossier passera en statut « Finalisé ».`,
      ];

  await transporter.sendMail({
    from: `"Plateforme Voyages" <${smtp.user}>`,
    to,
    subject: `[Travels] Direction finalisée — merci de donner la liste des élèves — ${title}`,
    text: [
      `Bonjour${params.trip.ownerName ? ` ${params.trip.ownerName}` : ""},`,
      ``,
      `Attention : la direction a finalisé votre projet de sortie / séjour.`,
      ``,
      `Titre : ${title}`,
      `Type : ${typeLabel}`,
      `Établissement : ${etab}`,
      `Destination : ${destination}`,
      `Dates : ${dateInfo}`,
      ``,
      `Il reste une étape de votre côté : merci de nous donner la liste nominative des élèves`,
      `(onglet « Élèves » du dossier) puis de la confirmer.`,
      ...cuisineBlock,
      ``,
      `Ouvrir l’onglet Élèves : ${link}`,
      ``,
      `Cordialement,`,
      `Plateforme Voyages — ${orgName}`,
    ].join("\n"),
  });

  return { sent: true };
}
