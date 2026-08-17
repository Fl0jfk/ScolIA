import Link from "next/link";
import RentreePublicHeader from "@/app/components/RentreePublicHeader";

type Search = Promise<{ ok?: string; erreur?: string }>;

export default async function RentreeDepotConfirmePage({
  searchParams,
}: {
  searchParams: Search;
}) {
  const sp = await searchParams;
  const ok = sp.ok === "1";
  const erreur = String(sp.erreur || "");

  let title = "Lien invalide";
  let body =
    "Ce lien de confirmation n’est pas valide. Déposez à nouveau le document depuis la page rentrée.";
  if (ok) {
    title = "Document transmis";
    body =
      "Votre fichier a bien été envoyé à l’établissement. Un accusé de réception vous a également été adressé.";
  } else if (erreur === "lien_expire") {
    title = "Lien expiré";
    body =
      "Ce lien a expiré (valable 72 heures). Retournez sur la page rentrée pour déposer à nouveau le document.";
  } else if (erreur === "fichier_manquant") {
    title = "Fichier introuvable";
    body = "Le fichier n’est plus disponible. Merci de le déposer à nouveau.";
  } else if (erreur === "serveur") {
    title = "Envoi interrompu";
    body =
      "La confirmation n’a pas pu aboutir. Réessayez le lien dans quelques minutes, ou déposez à nouveau le document.";
  }

  return (
    <div className="min-h-screen bg-white">
      <RentreePublicHeader />
      <main className="mx-auto max-w-lg px-6 py-16 text-center space-y-4">
        <p className="text-[10px] font-black uppercase tracking-widest text-slate-400">Rentrée</p>
        <h1 className="text-3xl font-black text-slate-900">{title}</h1>
        <p className="text-sm leading-relaxed text-slate-600">{body}</p>
        <p>
          <Link href="/rentree" className="text-sm font-black text-indigo-600 hover:underline">
            Retour à la page rentrée →
          </Link>
        </p>
      </main>
    </div>
  );
}
