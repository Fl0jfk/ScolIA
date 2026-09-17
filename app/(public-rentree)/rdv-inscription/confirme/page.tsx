import type { Metadata } from "next";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

export const metadata: Metadata = {
  title: "Confirmation rendez-vous d’inscription",
};

type PageProps = {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
};

function one(v: string | string[] | undefined): string {
  if (Array.isArray(v)) return v[0] || "";
  return v || "";
}

function formatSlot(start: string, end: string): string {
  if (!start) return "";
  const s = new Date(start);
  const e = end ? new Date(end) : s;
  const day = s.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const hm = (d: Date) =>
    d.toLocaleTimeString("fr-FR", {
      timeZone: "Europe/Paris",
      hour: "2-digit",
      minute: "2-digit",
    });
  return `${day} · ${hm(s)} – ${hm(e)}`;
}

export default async function RdvInscriptionConfirmePage({ searchParams }: PageProps) {
  const sp = await searchParams;
  const ok = one(sp.ok) === "1";
  const erreur = one(sp.erreur);
  const start = one(sp.start);
  const end = one(sp.end);
  const deja = one(sp.deja) === "1";

  let title = "Lien invalide";
  let body =
    "Ce lien de validation n’est pas valide ou a déjà été utilisé. Reprenez un créneau sur la page de rendez-vous.";

  if (ok) {
    title = deja ? "Créneau déjà validé" : "Réservation validée";
    body = deja
      ? "Ce rendez-vous était déjà confirmé. Un e-mail avec le fichier calendrier vous a été (ou vous sera) envoyé."
      : "Votre créneau est confirmé. Un e-mail de confirmation avec fichier calendrier (.ics) vient de vous être envoyé.";
  } else if (erreur === "expire") {
    title = "Lien expiré";
    body =
      "Le délai pour valider ce créneau est dépassé. Merci de choisir à nouveau un créneau sur la page de rendez-vous.";
  } else if (erreur === "pris") {
    title = "Créneau indisponible";
    body =
      "Ce créneau n’est plus disponible. Merci d’en choisir un autre sur la page de rendez-vous.";
  } else if (erreur === "rate") {
    title = "Trop de tentatives";
    body = "Réessayez dans quelques minutes.";
  } else if (erreur === "erreur") {
    title = "Erreur temporaire";
    body = "Une erreur est survenue. Réessayez dans un instant ou contactez l’établissement.";
  }

  return (
    <div className="min-h-screen bg-[#f7f8fa]">
      <RentreePublicHeader />
      <main className="mx-auto max-w-lg px-4 py-14 text-center">
        <div
          className={`mx-auto flex h-14 w-14 items-center justify-center rounded-full text-2xl ${
            ok ? "bg-emerald-100 text-emerald-700" : "bg-amber-100 text-amber-800"
          }`}
        >
          {ok ? "✓" : "!"}
        </div>
        <h1 className="mt-5 text-2xl font-bold tracking-tight text-slate-900">{title}</h1>
        <p className="mt-3 text-[15px] leading-relaxed text-slate-600">{body}</p>
        {ok && start ? (
          <p className="mt-5 rounded-xl bg-white px-4 py-3 text-sm font-medium text-slate-800 shadow-sm ring-1 ring-slate-200/80">
            {formatSlot(start, end)}
          </p>
        ) : null}
        <p className="mt-8 text-sm text-slate-500">
          Vous pouvez fermer cette page.
        </p>
      </main>
    </div>
  );
}
