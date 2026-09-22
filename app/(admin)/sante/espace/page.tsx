import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function SanteEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Espace santé"
        description="Bloc infirmerie — passages, extraits, documents. Le bloc n’est pas fini tant que PAI, médicaments et accidents ne sont pas tenus."
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
          <p className="font-semibold text-slate-800">Encore ouverts dans le bloc</p>
          <ul className="list-disc space-y-1 pl-5">
            <li>Fiche infirmerie (table prête)</li>
            <li>Médicaments / journal des prises (table prête)</li>
            <li>Registre des accidents (table prête)</li>
            <li>PAI validé + lien document</li>
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
