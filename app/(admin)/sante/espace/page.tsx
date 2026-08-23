import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

/** Stub santé — point d’entrée infirmier / psychologue. */
export default function SanteEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Espace santé"
        description="Infirmerie, PAP et suivi santé — accès réservé aux profils infirmerie et psychologue."
        actions={
          <Link href="/sante" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
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
    </ModulePageShell>
  );
}
