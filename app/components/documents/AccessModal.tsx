"use client";

import type { AccessPerson } from "@/app/lib/documents-page-model";
import DocumentModal from "./DocumentModal";

export default function AccessModal({
  title,
  people,
  onClose,
}: {
  title: string;
  people: AccessPerson[];
  onClose: () => void;
}) {
  return (
    <DocumentModal title={`Qui a accès — ${title}`} onClose={onClose} wide>
      <p className="text-sm text-gray-500 mb-3">
        Personnel autorisé à consulter ce partage.
      </p>
      <ul className="divide-y divide-gray-100 border border-gray-200 rounded-xl overflow-hidden max-h-72 overflow-y-auto">
        {people.map((person) => (
          <li key={person.userId} className="px-4 py-3 bg-white">
            <p className="text-sm font-semibold text-gray-900">
              {person.isOwner ? "👑 " : ""}
              {person.name}
              {person.isYou ? <span className="text-gray-500 font-normal"> (vous)</span> : null}
              {person.isOwner ? (
                <span className="ml-2 text-[10px] font-bold uppercase tracking-wide text-amber-700 bg-amber-50 px-1.5 py-0.5 rounded">
                  Propriétaire
                </span>
              ) : null}
            </p>
            <p className="text-xs text-gray-500 mt-0.5">{person.detail}</p>
          </li>
        ))}
      </ul>
      <div className="flex justify-end mt-4">
        <button
          type="button"
          onClick={onClose}
          className="px-4 py-2 text-sm rounded-xl bg-slate-800 text-white font-semibold"
        >
          Fermer
        </button>
      </div>
    </DocumentModal>
  );
}
