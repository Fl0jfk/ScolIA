"use client";

export default function SettingsToolboxPanel() {
  return (
    <div className="bg-white rounded-2xl border p-6 space-y-4">
      <h2 className="text-lg font-black text-slate-900">Boîte à outils saisonnière</h2>
      <p className="text-sm text-slate-600 max-w-xl">
        QR code et répartition des classes. Rentrée + fournitures →{" "}
        <a href="/etablissement/evenements" className="font-bold text-slate-900 underline">
          Événements
        </a>
        . Simulateur de tarifs →{" "}
        <a href="/etablissement/communication" className="font-bold text-slate-900 underline">
          Communication
        </a>
        .
      </p>
      <div className="flex flex-wrap gap-2">
        <a
          href="/toolbox"
          className="inline-flex rounded-xl bg-slate-900 px-5 py-2.5 text-sm font-bold text-white hover:bg-slate-800"
        >
          Ouvrir la boîte à outils →
        </a>
        <a
          href="/etablissement/evenements"
          className="inline-flex rounded-xl border border-slate-200 px-5 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
        >
          Événements →
        </a>
        <a
          href="/etablissement/communication"
          className="inline-flex rounded-xl border border-slate-200 px-5 py-2.5 text-sm font-bold text-slate-800 hover:bg-slate-50"
        >
          Communication →
        </a>
      </div>
    </div>
  );
}
