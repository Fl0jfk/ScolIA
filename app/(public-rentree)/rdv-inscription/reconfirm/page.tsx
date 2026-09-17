import { redirect } from "next/navigation";
import { reconfirmPublicRdvInscription } from "@/app/lib/rdv-inscription-service";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

type PageProps = {
  searchParams: Promise<{ token?: string; action?: string }>;
};

export default async function RdvInscriptionReconfirmPage({ searchParams }: PageProps) {
  const sp = await searchParams;
  const token = String(sp.token || "").trim();
  const actionRaw = String(sp.action || "").trim().toLowerCase();
  const action = actionRaw === "cancel" ? "cancel" : actionRaw === "ok" ? "ok" : null;

  if (!token || !action) {
    redirect("/rdv-inscription/confirme?ok=0&reason=invalid");
  }

  const result = await reconfirmPublicRdvInscription({ token, action });

  const ok = result.ok;
  const cancelled = ok && result.action === "cancel";
  const confirmed = ok && result.action === "ok";

  return (
    <div className="min-h-screen bg-[#f7f8fa]">
      <RentreePublicHeader />
      <main className="mx-auto max-w-lg px-4 py-14 text-center">
        <div
          className={`mx-auto flex h-14 w-14 items-center justify-center rounded-full text-2xl ${
            cancelled
              ? "bg-red-100 text-red-700"
              : confirmed
                ? "bg-emerald-100 text-emerald-700"
                : "bg-amber-100 text-amber-800"
          }`}
        >
          {cancelled ? "✕" : confirmed ? "✓" : "!"}
        </div>
        <h1 className="mt-5 text-2xl font-bold tracking-tight text-slate-900">
          {cancelled
            ? "Rendez-vous annulé"
            : confirmed
              ? "Merci, c’est noté"
              : "Lien invalide"}
        </h1>
        <p className="mt-3 text-sm leading-relaxed text-slate-600">
          {cancelled
            ? "Votre rendez-vous d’inscription a bien été annulé. L’établissement en a été informé."
            : confirmed
              ? "Votre présence est confirmée. Un check apparaît désormais sur l’agenda de la direction."
              : !result.ok
                ? result.message
                : "Impossible de traiter cette demande."}
        </p>
      </main>
    </div>
  );
}
