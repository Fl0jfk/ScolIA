"use client";

import FaireUneDemandeForm, { type FaireUneDemandeSuccess } from "@/app/components/requests/FaireUneDemandeForm";

export default function CreateRequestModal({
  open,
  onClose,
  onCreated,
  title = "Faire une demande",
  subtitle = "Pas de catégorie à choisir — le système oriente vers le bon service.",
  subjectPrefix,
  fixedSubject,
  initialDescription,
  heading,
  intro,
}: {
  open: boolean;
  onClose: () => void;
  onCreated?: () => void;
  title?: string;
  subtitle?: string;
  subjectPrefix?: string;
  fixedSubject?: string;
  initialDescription?: string;
  heading?: string;
  intro?: string;
}) {
  if (!open) return null;

  const handleSuccess = (_result: FaireUneDemandeSuccess) => {
    onCreated?.();
  };

  return (
    <div className="fixed inset-0 z-[9999] flex items-start justify-center p-4 sm:p-8 overflow-y-auto isolate">
      <button
        type="button"
        className="absolute inset-0 bg-slate-900/50 backdrop-blur-[2px]"
        aria-label="Fermer"
        onClick={onClose}
      />
      <div className="relative w-full max-w-lg rounded-2xl bg-white shadow-2xl border border-slate-200 p-5 sm:p-6 mt-24 sm:mt-32 mb-8">
        <div className="flex items-start justify-between gap-3 mb-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-widest text-blue-600">Nouvelle demande</p>
            <h2 className="text-xl font-black text-slate-900 mt-0.5">{title}</h2>
            <p className="text-xs text-slate-500 mt-1">{subtitle}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="shrink-0 rounded-xl px-2.5 py-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-800 font-bold text-lg leading-none"
            aria-label="Fermer"
          >
            ×
          </button>
        </div>
        <FaireUneDemandeForm
          variant="modal"
          onSuccess={handleSuccess}
          mesDemandesHref="/requests#mes-demandes"
          subjectPrefix={subjectPrefix}
          fixedSubject={fixedSubject}
          initialDescription={initialDescription}
          heading={heading}
          intro={intro}
        />
      </div>
    </div>
  );
}
