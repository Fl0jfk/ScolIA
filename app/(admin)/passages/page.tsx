import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

export default function PassagesEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Établissement"
        title="Passages & cantine"
        description="Portail (entrée / sortie) et self (repas pris). Le régime = le droit ; le passage self = le pris."
        actions={
          <Link href="/dashboard" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Dashboard
          </Link>
        }
      />
      <div className="space-y-4">
        <div className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-700">
          <p className="font-bold text-slate-950">Portail</p>
          <p className="mt-1">
            Entrée et sortie de l’établissement. « Où est X » lit la dernière sortie du jour.
          </p>
          <Link
            href="/passages/portail"
            className="mt-4 inline-flex rounded-xl bg-slate-900 px-4 py-2 text-xs font-bold text-white"
          >
            Saisir au portail
          </Link>
        </div>
        <div className="rounded-3xl border border-amber-200 bg-amber-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-amber-950">Self — repas pris</p>
          <p className="mt-1">
            Chaque passage au self. Les extraits cantine (allergies) s’affichent à la saisie.
          </p>
          <Link
            href="/passages/self"
            className="mt-4 inline-flex rounded-xl bg-amber-800 px-4 py-2 text-xs font-bold text-white"
          >
            Saisir au self
          </Link>
        </div>
        <div className="rounded-3xl border border-sky-200 bg-sky-50/50 p-6 text-sm text-slate-700">
          <p className="font-bold text-sky-950">Prévision des repas</p>
          <p className="mt-1">
            Droit (grille Lun–Ven, sinon régime) contre pris (self). Manquants et imprévus du jour.
          </p>
          <Link
            href="/passages/prevision"
            className="mt-4 inline-flex rounded-xl bg-sky-800 px-4 py-2 text-xs font-bold text-white"
          >
            Voir la prévision
          </Link>
        </div>
        <div className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          <p className="font-semibold text-slate-800">Encore ouvert dans le bloc</p>
          <ul className="mt-2 list-disc space-y-1 pl-5">
            <li>Facturation au forfait ou au réel (réglage)</li>
          </ul>
        </div>
      </div>
    </ModulePageShell>
  );
}
