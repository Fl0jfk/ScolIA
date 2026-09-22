import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function SanteEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Espace santé"
        description="Infirmerie, PAP et suivi santé — accès réservé aux profils concernés."
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
            Qui est là maintenant. Le motif court reste à l’infirmerie ; la vie scolaire voit le
            signal de présence.
          </p>
          <Link
            href="/sante/passages"
            className="mt-4 inline-flex rounded-xl bg-rose-800 px-4 py-2 text-xs font-bold text-white"
          >
            Ouvrir les passages
          </Link>
        </div>
        <div className="space-y-3 rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          <p>
            Les documents santé du dossier élève (tiroir « sante », confidentialité restreinte) sont
            visibles depuis la fiche élève.
          </p>
          <Link
            href="/eleves/dossiers"
            className="inline-flex rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
          >
            Ouvrir les dossiers élèves
          </Link>
        </div>
      </div>
    </ModulePageShell>
  );
}
