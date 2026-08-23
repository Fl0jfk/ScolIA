import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

/** Stub P4 — notes / bulletins. */
export default function NotesEspacePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Administratif"
        title="Notes & bulletins"
        description="Le module notes (P4) arrive bientôt — les bulletins générés seront classés automatiquement dans le dossier élève."
        actions={
          <Link href="/administratif" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Administratif
          </Link>
        }
      />
      <p className="rounded-3xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
        En attendant, ouvrez un{" "}
        <Link href="/eleves/dossiers" className="font-semibold text-indigo-600 hover:underline">
          dossier élève
        </Link>{" "}
        : l’onglet Notes y est déjà prévu selon votre rôle.
      </p>
    </ModulePageShell>
  );
}
