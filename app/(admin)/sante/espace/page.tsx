import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function SanteEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Espace santé"
        description="Bloc infirmerie — passages, fiches, médicaments, accidents, extraits. Le PAI validé manque encore."
        actions={
          <Link href="/sante" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <div className="space-y-4">
        <div className="rounded-3xl border border-rose-200 bg-rose-50/60 p-6 text-sm text-slate-700">
          <p className="font-bold text-rose-950">Passages à l’infirmerie</p>
          <p className="mt-1">
            Qui est là, puis la suite (retour cours, renvoi famille…). Le motif reste à
            l’infirmerie ; la vie scolaire voit le signal.
          </p>
          <Link
            href="/sante/passages"
            className="mt-4 inline-flex rounded-xl bg-rose-800 px-4 py-2 text-xs font-bold text-white"
          >
            Ouvrir les passages
          </Link>
        </div>
        <div className="rounded-3xl border border-violet-200 bg-violet-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-violet-950">Fiches infirmerie</p>
          <p className="mt-1">
            Antécédents utiles à l’établissement et personnes à prévenir.
          </p>
          <Link
            href="/sante/fiches"
            className="mt-4 inline-flex rounded-xl bg-violet-800 px-4 py-2 text-xs font-bold text-white"
          >
            Ouvrir les fiches
          </Link>
        </div>
        <div className="rounded-3xl border border-sky-200 bg-sky-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-sky-950">Médicaments</p>
          <p className="mt-1">Journal des prises (qui, quand, quoi). Ordonnance = document.</p>
          <Link
            href="/sante/medicaments"
            className="mt-4 inline-flex rounded-xl bg-sky-800 px-4 py-2 text-xs font-bold text-white"
          >
            Journal des prises
          </Link>
        </div>
        <div className="rounded-3xl border border-orange-200 bg-orange-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-orange-950">Accidents</p>
          <p className="mt-1">Registre : circonstances, soins, suite. Conservation longue.</p>
          <Link
            href="/sante/accidents"
            className="mt-4 inline-flex rounded-xl bg-orange-800 px-4 py-2 text-xs font-bold text-white"
          >
            Registre des accidents
          </Link>
        </div>
        <div className="rounded-3xl border border-amber-200 bg-amber-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-amber-950">Extraits diffusés</p>
          <p className="mt-1">
            Allergie cantine, inaptitude EPS, alerte voyage — ce que les autres services ont le
            droit de lire.
          </p>
          <Link
            href="/sante/extraits"
            className="mt-4 inline-flex rounded-xl bg-amber-800 px-4 py-2 text-xs font-bold text-white"
          >
            Gérer les extraits
          </Link>
        </div>
        <div className="space-y-3 rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          <p className="font-semibold text-slate-800">Encore ouvert dans le bloc</p>
          <ul className="list-disc space-y-1 pl-5">
            <li>PAI validé + lien document</li>
            <li>Inaptitude EPS → extrait</li>
            <li>Nuit internat</li>
          </ul>
          <Link
            href="/eleves/dossiers"
            className="mt-2 inline-flex rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
          >
            Dossiers élèves (documents santé)
          </Link>
        </div>
      </div>
    </ModulePageShell>
  );
}
