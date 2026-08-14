import type { ReactNode } from "react";
import type { ModuleTourStep } from "@/app/lib/module-tours";

type Variant = NonNullable<ModuleTourStep["travelsPreview"]>;

const WORKFLOW_STEPS = [
  { label: "Validation pédagogique", icon: "📋", note: "Direction valide le projet" },
  { label: "Choix du devis transport", icon: "🚌", note: "Si bus : devis transporteurs" },
  { label: "Signature devis bus", icon: "✍️", note: "Direction signe le devis retenu" },
  { label: "Validation finances", icon: "💶", note: "Comptabilité / budget" },
  { label: "Validation finale", icon: "✅", note: "Direction confirme" },
  { label: "Finalisé", icon: "🎉", note: "Documents parents, sortie prête" },
];

function PreviewCard({
  title,
  subtitle,
  children,
  accent = "indigo",
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
  accent?: "indigo" | "amber" | "emerald" | "slate";
}) {
  const border =
    accent === "amber"
      ? "border-amber-200 bg-amber-50/80"
      : accent === "emerald"
        ? "border-emerald-200 bg-emerald-50/80"
        : accent === "slate"
          ? "border-slate-200 bg-slate-50"
          : "border-indigo-200 bg-indigo-50/80";
  return (
    <div className={`rounded-xl border px-3 py-2.5 ${border}`}>
      <p className="text-xs font-bold text-slate-900">{title}</p>
      {subtitle && <p className="text-[10px] text-slate-500 mt-0.5">{subtitle}</p>}
      <div className="mt-2">{children}</div>
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-white/90 border border-slate-200 px-2 py-1">
      <p className="text-[9px] font-bold uppercase text-slate-400">{label}</p>
      <p className="text-[11px] font-medium text-slate-800">{value}</p>
    </div>
  );
}

export default function ModuleTourTravelsPreview({ variant }: { variant: Variant }) {
  if (variant === "type-choice") {
    return (
      <div className="mt-3 grid gap-2">
        <PreviewCard title="🍦 Sortie de proximité" subtitle="Sans transport spécifique">
          <p className="text-[10px] text-slate-600">Cinéma, parc, musée à pied ou en métro…</p>
        </PreviewCard>
        <PreviewCard title="🚌 Voyage / sortie bus" subtitle="Transport, budget, nuitées" accent="amber">
          <p className="text-[10px] text-slate-600">Bus optionnel dans le formulaire suivant.</p>
        </PreviewCard>
      </div>
    );
  }

  if (variant === "simple-form") {
    return (
      <div className="mt-3 space-y-2">
        <p className="text-[10px] font-bold text-indigo-800 uppercase tracking-wide">Formulaire sortie simple</p>
        <div className="grid grid-cols-2 gap-1.5">
          <Field label="Titre" value="Visite musée des Beaux-Arts" />
          <Field label="Établissement" value="Collège" />
          <Field label="Destination" value="Rouen centre" />
          <Field label="Date" value="15 mars 2026" />
          <Field label="Effectif" value="28 élèves · 3 accompagnateurs" />
          <Field label="Coût total" value="420 €" />
        </div>
        <PreviewCard title="Pique-nique / cuisine" subtitle="Optionnel" accent="emerald">
          <p className="text-[10px] text-slate-600">Commande repas ou paniers — bon envoyé à la cuisine.</p>
        </PreviewCard>
        <p className="text-[10px] text-slate-500 italic">Pas de case « bus » sur ce parcours.</p>
      </div>
    );
  }

  if (variant === "complex-no-bus") {
    return (
      <div className="mt-3 space-y-2">
        <p className="text-[10px] font-bold text-violet-800 uppercase tracking-wide">Voyage — sans bus</p>
        <div className="grid grid-cols-2 gap-1.5">
          <Field label="Titre" value="Séjour linguistique" />
          <Field label="Dates" value="3 → 7 juin 2026" />
          <Field label="Budget famille" value="180 € / élève" />
          <Field label="Budget établissement" value="1 200 €" />
        </div>
        <div className="flex items-center gap-2 rounded-lg border border-dashed border-slate-300 bg-white px-2 py-1.5">
          <span className="text-lg opacity-40">☐</span>
          <span className="text-[11px] font-medium text-slate-600">Besoin d&apos;un bus — décoché</span>
        </div>
        <p className="text-[10px] text-slate-500">→ Pas d&apos;e-mail transporteur · pas d&apos;onglet Transport.</p>
      </div>
    );
  }

  if (variant === "complex-with-bus") {
    return (
      <div className="mt-3 space-y-2">
        <p className="text-[10px] font-bold text-amber-800 uppercase tracking-wide">Voyage — avec bus</p>
        <div className="flex items-center gap-2 rounded-lg border border-amber-300 bg-amber-100/80 px-2 py-1.5">
          <span className="text-lg">☑</span>
          <span className="text-[11px] font-bold text-amber-900">Besoin d&apos;un bus</span>
        </div>
        <div className="grid grid-cols-2 gap-1.5">
          <Field label="Prise en charge" value="Parking collège — 8h00" />
          <Field label="Bus sur place" value="Oui (visites)" />
          <Field label="Programme" value="programme-bus.pdf" />
          <Field label="Effectif" value="52 él. · 5 acc." />
        </div>
        <PreviewCard title="Récap avant envoi" accent="amber">
          <p className="text-[10px] text-slate-700">
            Modale de confirmation → puis e-mail automatique aux transporteurs pour devis.
          </p>
        </PreviewCard>
        <PreviewCard title="Dans le dossier" subtitle="Onglet Transport">
          <p className="text-[10px] text-slate-600">Devis reçus → choix direction → signature → compta.</p>
        </PreviewCard>
      </div>
    );
  }

  if (variant === "workflow") {
    return (
      <div className="mt-3 space-y-1">
        {WORKFLOW_STEPS.map((s, i) => (
          <div key={s.label} className="flex items-start gap-2">
            <div className="flex flex-col items-center">
              <span className="w-6 h-6 rounded-full bg-indigo-100 text-indigo-800 text-xs flex items-center justify-center font-bold">
                {s.icon}
              </span>
              {i < WORKFLOW_STEPS.length - 1 && <span className="w-px h-3 bg-indigo-200 my-0.5" />}
            </div>
            <div className="pb-1">
              <p className="text-[11px] font-bold text-slate-800">{s.label}</p>
              <p className="text-[10px] text-slate-500">{s.note}</p>
            </div>
          </div>
        ))}
        <p className="text-[10px] text-amber-800 bg-amber-50 border border-amber-100 rounded-lg px-2 py-1 mt-2">
          Étapes bus et cuisine ignorées si non concernées. « Modifications demandées » renvoie au professeur.
        </p>
      </div>
    );
  }

  if (variant === "dossier-hub") {
    const tabs = [
      { icon: "🏠", label: "Vue d'ensemble", on: true },
      { icon: "🚌", label: "Transport", on: true },
      { icon: "🍽️", label: "Cuisine", on: false },
      { icon: "📁", label: "Documents", on: true },
      { icon: "📜", label: "Journal", on: true },
      { icon: "💬", label: "Messagerie", on: true },
      { icon: "⚡", label: "Actions", on: true },
    ];
    return (
      <div className="mt-3 space-y-2">
        <div className="flex flex-wrap gap-1">
          {tabs.map((t) => (
            <span
              key={t.label}
              className={`text-[10px] font-bold px-2 py-1 rounded-lg border ${
                t.on
                  ? "bg-indigo-600 text-white border-indigo-600"
                  : "bg-white text-slate-400 border-slate-200 line-through opacity-60"
              }`}
            >
              {t.icon} {t.label}
            </span>
          ))}
        </div>
        <p className="text-[10px] text-slate-500">
          Les onglets grisés n&apos;apparaissent pas (ex. Cuisine sans commande repas).
        </p>
      </div>
    );
  }

  return null;
}
