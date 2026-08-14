import { formatInstallationSlotFr } from "@/app/lib/internat-installation-slots";

type Search = Promise<{ ok?: string; erreur?: string; slot?: string }>;

export default async function InternatInstallationConfirmePage({
  searchParams,
}: {
  searchParams: Search;
}) {
  const sp = await searchParams;
  const ok = sp.ok === "1";
  const erreur = String(sp.erreur || "");
  const slotLabel = sp.slot ? formatInstallationSlotFr(sp.slot) : "";

  let title = "Lien invalide";
  let body = "Ce lien de confirmation n’est pas valide. Demandez un nouveau rendez-vous.";
  if (ok) {
    title = "Rendez-vous confirmé";
    body = slotLabel
      ? `Créneau : ${slotLabel}. Un e-mail avec un fichier calendrier (.ics) vous a été envoyé.`
      : "Un e-mail avec un fichier calendrier (.ics) vous a été envoyé.";
  } else if (erreur === "lien_expire") {
    title = "Lien expiré";
    body = "Ce lien a expiré (valable 2 heures). Reprenez un créneau sur la page d’inscription.";
  } else if (erreur === "creneau_complet") {
    title = "Créneau complet";
    body = "Ce créneau vient d’être pris. Choisissez un autre horaire sur la page d’inscription.";
  }

  return (
    <main className="min-h-[100dvh] bg-[radial-gradient(ellipse_at_top,_#eef2ff_0%,_#f8fafc_50%,_#f1f5f9_100%)]">
      <div className="mx-auto max-w-lg px-4 py-16 text-center space-y-3">
        <h1 className="text-2xl font-black text-slate-900">{title}</h1>
        <p className="text-slate-600 text-sm">{body}</p>
        <p>
          <a
            href="/internat/installation"
            className="text-sm font-semibold text-indigo-700 hover:underline"
          >
            Retour aux créneaux
          </a>
        </p>
      </div>
    </main>
  );
}
