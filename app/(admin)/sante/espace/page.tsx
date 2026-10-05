import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function SanteEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Espace santé"
        description="Bloc infirmerie — passages (journée / nuit internat), fiches, PAI, médicaments, accidents, inaptitudes EPS, extraits."
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
            Qui est là (journée ou nuit internat), puis la suite. Le motif reste à
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
        <div className="rounded-3xl border border-teal-200 bg-teal-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-teal-950">PAI</p>
          <p className="mt-1">
            Protocole, traitements autorisés, lien document. Validation par l’infirmerie.
          </p>
          <Link
            href="/sante/pai"
            className="mt-4 inline-flex rounded-xl bg-teal-800 px-4 py-2 text-xs font-bold text-white"
          >
            Gérer les PAI
          </Link>
        </div>
        <div className="rounded-3xl border border-indigo-200 bg-indigo-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-indigo-950">Inaptitudes EPS</p>
          <p className="mt-1">Certificat daté → extrait EPS diffusé automatiquement.</p>
          <Link
            href="/sante/inaptitudes"
            className="mt-4 inline-flex rounded-xl bg-indigo-800 px-4 py-2 text-xs font-bold text-white"
          >
            Gérer les inaptitudes
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
        <div className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          <p className="font-semibold text-slate-800">Documents santé</p>
          <p className="mt-1">PAI, PAP, ordonnances dans le tiroir du dossier élève.</p>
          <Link
            href="/eleves/dossiers"
            className="mt-3 inline-flex rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
          >
            Dossiers élèves
          </Link>
        </div>
      </div>
    </ModulePageShell>
  );
}
