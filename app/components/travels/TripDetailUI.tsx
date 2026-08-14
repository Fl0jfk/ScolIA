"use client";

import type { ReactNode } from "react";

/* ─── Statut ─── */

const STATUS_MAP: Record<string, { label: string; tone: string }> = {
  VALIDE: { label: "Finalisé", tone: "bg-emerald-100 text-emerald-800 ring-emerald-200" },
  REJETE: { label: "Refusé", tone: "bg-rose-100 text-rose-800 ring-rose-200" },
  SEANCE_ANNULEE: { label: "Séance annulée", tone: "bg-slate-200 text-slate-700 ring-slate-300" },
  ANNULE: { label: "Sortie annulée", tone: "bg-red-100 text-red-800 ring-red-200" },
  BESOIN_MODIFICATION: { label: "Modifications demandées", tone: "bg-orange-100 text-orange-800 ring-orange-200" },
  EN_ATTENTE_DIR_INITIAL: { label: "Validation pédagogique", tone: "bg-blue-100 text-blue-800 ring-blue-200" },
  PROF_LOGISTICS: { label: "Logistique transport", tone: "bg-violet-100 text-violet-800 ring-violet-200" },
  EN_ATTENTE_BUS_SIGNATURE: { label: "Signature devis bus", tone: "bg-amber-100 text-amber-900 ring-amber-200" },
  EN_ATTENTE_COMPTA: { label: "Validation finances", tone: "bg-cyan-100 text-cyan-900 ring-cyan-200" },
  EN_ATTENTE_DIR_FINAL: { label: "Validation finale", tone: "bg-indigo-100 text-indigo-800 ring-indigo-200" },
};

function tripStatusDisplay(status: string) {
  return STATUS_MAP[status] || {
    label: status.replace(/EN_ATTENTE_/g, "").replace(/_/g, " "),
    tone: "bg-slate-100 text-slate-700 ring-slate-200",
  };
}

function TripStatusBadge({ status, pulse }: { status: string; pulse?: boolean }) {
  const { label, tone } = tripStatusDisplay(status);
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-xs font-bold uppercase tracking-wide ring-1 ${tone} ${pulse ? "animate-pulse" : ""}`}
    >
      <span className="w-1.5 h-1.5 rounded-full bg-current opacity-60" />
      {label}
    </span>
  );
}

/* ─── Layout ─── */

export function TripPageShell({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-50 via-white to-slate-50/80">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 py-6 sm:py-8 space-y-6">{children}</div>
    </div>
  );
}

export function TripSection({
  title,
  subtitle,
  icon,
  accent = "indigo",
  action,
  children,
  className = "",
}: {
  title: string;
  subtitle?: string;
  icon?: string;
  accent?: "indigo" | "amber" | "slate" | "emerald";
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  const accents = {
    indigo: "border-indigo-100",
    amber: "border-amber-200 bg-gradient-to-br from-amber-50/80 to-orange-50/40",
    slate: "border-slate-200",
    emerald: "border-emerald-100",
  };
  return (
    <section className={`rounded-2xl border bg-white shadow-sm overflow-hidden ${accents[accent]} ${className}`}>
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 px-6 py-4 border-b border-slate-100/80 bg-white/60">
        <div className="flex items-start gap-3">
          {icon && (
            <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-slate-100 text-lg">
              {icon}
            </span>
          )}
          <div>
            <h2 className="text-base font-bold text-slate-900">{title}</h2>
            {subtitle && <p className="text-xs text-slate-500 mt-0.5">{subtitle}</p>}
          </div>
        </div>
        {action}
      </div>
      <div className="p-6">{children}</div>
    </section>
  );
}

/* ─── En-tête ─── */

export function TripHeroHeader({
  title,
  typeLabel,
  ownerName,
  seriesLabel,
  etablissement,
  status,
  statusPulse,
}: {
  title: string;
  typeLabel: string;
  ownerName: string;
  seriesLabel?: string | null;
  etablissement?: string;
  status: string;
  statusPulse?: boolean;
}) {
  const etabEmoji =
    etablissement === "École" ? "🏫" : etablissement === "Collège" ? "📚" : etablissement === "Lycée" ? "🎓" : "🏛";

  return (
    <div className="relative overflow-hidden rounded-2xl bg-gradient-to-br from-slate-900 via-slate-800 to-indigo-950 text-white shadow-xl">
      <div className="absolute inset-0 opacity-20 bg-[radial-gradient(ellipse_at_top_right,white,transparent_55%)]" />
      <div className="relative px-6 sm:px-8 py-6 sm:py-8">
        <div className="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4">
          <div className="space-y-3 min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-[10px] font-bold uppercase tracking-widest text-indigo-300">{typeLabel}</span>
              {etablissement && (
                <span className="text-[10px] font-semibold px-2.5 py-0.5 rounded-full bg-white/10 text-white/90">
                  {etabEmoji} {etablissement}
                </span>
              )}
              {seriesLabel && (
                <span className="text-[10px] font-semibold px-2.5 py-0.5 rounded-full bg-indigo-500/30 text-indigo-100">
                  {seriesLabel}
                </span>
              )}
            </div>
            <h1 className="text-2xl sm:text-3xl font-black tracking-tight leading-tight">{title}</h1>
            <p className="text-sm text-slate-300">
              Dossier créé par <span className="font-semibold text-white">{ownerName}</span>
            </p>
          </div>
          <div className="shrink-0">
            <TripStatusBadge status={status} pulse={statusPulse} />
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── KPI rapides ─── */

export function TripQuickStats({
  items,
}: {
  items: { label: string; value: string; icon?: string; action?: React.ReactNode }[];
}) {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
      {items.map((item) => (
        <div
          key={item.label}
          className="rounded-xl border border-slate-200/80 bg-white px-4 py-3 shadow-sm"
        >
          <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">{item.label}</p>
          <p className="mt-1 text-sm font-bold text-slate-800 truncate" title={item.value}>
            {item.icon && <span className="mr-1">{item.icon}</span>}
            {item.value}
          </p>
          {item.action}
        </div>
      ))}
    </div>
  );
}

/* ─── Stepper ─── */

export function TripWorkflowStepper({
  steps,
  currentStatus,
  busSignatureOnLogistics,
}: {
  steps: { n: string; label: string; key: string }[];
  currentStatus: string;
  busSignatureOnLogistics?: boolean;
}) {
  const isFullyComplete = currentStatus === "VALIDE";
  const activeIndex = steps.findIndex(
    (s) =>
      currentStatus === s.key ||
      (busSignatureOnLogistics && currentStatus === "EN_ATTENTE_BUS_SIGNATURE" && s.key === "PROF_LOGISTICS"),
  );
  const resolvedActive = isFullyComplete
    ? steps.length
    : activeIndex >= 0
      ? activeIndex
      : 0;

  return (
    <div className="rounded-2xl border border-slate-200 bg-white p-4 sm:p-6 shadow-sm overflow-x-auto">
      <div className="flex min-w-max sm:min-w-0 sm:w-full items-center gap-0">
        {steps.map((s, i) => {
          const isActive = !isFullyComplete && i === resolvedActive;
          const isDone = i < resolvedActive;
          return (
            <div key={s.key} className="flex items-center flex-1 last:flex-none">
              <div className="flex flex-col items-center gap-2 min-w-[4.5rem] sm:min-w-0 sm:flex-1">
                <div
                  className={`flex h-9 w-9 items-center justify-center rounded-full text-xs font-black transition-all ${
                    isActive
                      ? "bg-indigo-600 text-white ring-4 ring-indigo-100 scale-110"
                      : isDone
                        ? "bg-emerald-500 text-white"
                        : "bg-slate-100 text-slate-400"
                  }`}
                >
                  {isDone && !isActive ? "✓" : s.n}
                </div>
                <p
                  className={`text-[10px] sm:text-xs font-bold text-center leading-tight max-w-[5rem] sm:max-w-none ${
                    isActive ? "text-indigo-700" : isDone ? "text-emerald-700" : "text-slate-400"
                  }`}
                >
                  {s.label}
                </p>
              </div>
              {i < steps.length - 1 && (
                <div
                  className={`h-0.5 flex-1 min-w-[1rem] sm:min-w-[2rem] mx-1 rounded ${
                    i < resolvedActive ? "bg-emerald-400" : "bg-slate-200"
                  }`}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ─── Champs ─── */

export function TripField({
  label,
  children,
  span = 1,
}: {
  label: string;
  children: ReactNode;
  span?: 1 | 2;
}) {
  return (
    <div className={span === 2 ? "sm:col-span-2" : ""}>
      <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-1.5">{label}</p>
      <div className="text-sm text-slate-800">{children}</div>
    </div>
  );
}

export function TripFieldValue({ value, multiline }: { value?: string | null; multiline?: boolean }) {
  if (!value) return <span className="text-slate-400 italic">—</span>;
  return (
    <span className={`font-medium ${multiline ? "whitespace-pre-wrap leading-relaxed block" : ""}`}>{value}</span>
  );
}

/** Actions sous une valeur de champ (boutons « Modifier… ») avec espacement léger. */
export function TripFieldActions({ children }: { children: ReactNode }) {
  return <div className="mt-3 flex flex-col items-start gap-2">{children}</div>;
}

export function TripInput({
  className = "",
  ...props
}: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`w-full rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm font-medium text-slate-800 outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100 ${className}`}
      {...props}
    />
  );
}

export function TripTextarea({
  className = "",
  ...props
}: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={`w-full rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm font-medium text-slate-800 outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100 min-h-[88px] ${className}`}
      {...props}
    />
  );
}

/* ─── Boutons ─── */

type BtnVariant = "primary" | "secondary" | "ghost" | "success" | "warning" | "danger" | "dark";

const BTN_STYLES: Record<BtnVariant, string> = {
  primary: "bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm shadow-indigo-200",
  secondary: "bg-white text-slate-700 border border-slate-200 hover:bg-slate-50",
  ghost: "bg-transparent text-slate-600 hover:bg-slate-100",
  success: "bg-emerald-600 text-white hover:bg-emerald-700 shadow-sm shadow-emerald-200",
  warning: "bg-amber-600 text-white hover:bg-amber-700",
  danger: "bg-rose-600 text-white hover:bg-rose-700",
  dark: "bg-slate-800 text-white hover:bg-slate-700 border border-slate-700",
};

export function TripButton({
  variant = "primary",
  size = "md",
  className = "",
  children,
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: BtnVariant; size?: "sm" | "md" | "lg" }) {
  const sizes = { sm: "px-3 py-1.5 text-xs rounded-lg", md: "px-4 py-2.5 text-sm rounded-xl", lg: "px-6 py-3 text-sm rounded-xl" };
  return (
    <button
      className={`inline-flex items-center justify-center gap-2 font-bold transition-all active:scale-[0.98] disabled:opacity-50 disabled:pointer-events-none ${BTN_STYLES[variant]} ${sizes[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}

/* ─── Alertes ─── */

export function TripAlert({
  tone,
  icon,
  title,
  children,
  action,
}: {
  tone: "warning" | "info" | "muted" | "success";
  icon?: string;
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}) {
  const tones = {
    warning: "bg-orange-50 border-orange-200 text-orange-900",
    info: "bg-blue-50 border-blue-200 text-blue-900",
    muted: "bg-slate-100 border-slate-200 text-slate-800",
    success: "bg-emerald-50 border-emerald-200 text-emerald-900",
  };
  return (
    <div className={`flex flex-col sm:flex-row sm:items-center justify-between gap-4 rounded-2xl border p-5 ${tones[tone]}`}>
      <div className="flex items-start gap-3 text-left">
        {icon && <span className="text-2xl shrink-0">{icon}</span>}
        <div>
          <p className="font-bold">{title}</p>
          {children && <div className="text-sm mt-1 opacity-90">{children}</div>}
        </div>
      </div>
      {action}
    </div>
  );
}

/* ─── Documents ─── */

export function TripDocumentChip({
  name,
  onOpen,
  onZeendoc,
  onRemove,
  zeendocBusy,
  showZeendoc,
  canRemove,
}: {
  name: string;
  onOpen: () => void;
  onZeendoc?: () => void;
  onRemove?: () => void;
  zeendocBusy?: boolean;
  showZeendoc?: boolean;
  canRemove?: boolean;
}) {
  return (
    <div className="group flex items-center gap-1 rounded-xl border border-slate-200 bg-slate-50 pl-3 pr-1 py-1.5 text-sm shadow-sm hover:border-indigo-200 hover:bg-indigo-50/30 transition-colors">
      <button type="button" onClick={onOpen} className="font-semibold text-indigo-700 hover:underline truncate max-w-[200px]">
        📄 {name}
      </button>
      {showZeendoc && onZeendoc && (
        <button
          type="button"
          onClick={onZeendoc}
          disabled={zeendocBusy}
          className="shrink-0 px-2 py-0.5 rounded-lg text-[10px] font-bold text-slate-600 bg-white border border-slate-200 hover:bg-slate-100 disabled:opacity-50"
        >
          {zeendocBusy ? "…" : "Zeendoc"}
        </button>
      )}
      {canRemove && onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="shrink-0 w-6 h-6 rounded-lg text-rose-500 hover:bg-rose-50 font-bold text-xs"
          aria-label="Supprimer"
        >
          ✕
        </button>
      )}
    </div>
  );
}

/* ─── Devis bus card ─── */

export function TripBusQuoteCard({
  selected,
  review,
  children,
  actions,
}: {
  selected?: boolean;
  review?: boolean;
  children: ReactNode;
  actions: ReactNode;
}) {
  return (
    <div
      className={`rounded-xl border-2 p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 transition-all ${
        selected
          ? "border-emerald-400 bg-emerald-50/50 shadow-md"
          : review
            ? "border-orange-300 bg-orange-50/60"
            : "border-slate-200 bg-white hover:border-slate-300"
      }`}
    >
      <div className="min-w-0 flex-1 text-left">{children}</div>
      <div className="flex flex-col items-stretch sm:items-end gap-2 shrink-0">{actions}</div>
    </div>
  );
}

/* ─── Panneau décision ─── */

export function TripDecisionPanel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div id="trip-decision-panel" className="rounded-2xl bg-gradient-to-br from-slate-900 to-slate-800 text-white shadow-xl overflow-hidden">
      <div className="px-6 py-4 border-b border-white/10">
        <p className="text-xs font-bold uppercase tracking-widest text-indigo-300">Circuit de validation</p>
        <p className="text-lg font-bold mt-0.5">{title}</p>
      </div>
      <div className="p-6 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">{children}</div>
    </div>
  );
}

/* ─── Loading ─── */

export function TripLoadingOverlay({
  mode,
}: {
  mode: "circular" | "signing" | "default";
}) {
  return (
    <div className="fixed inset-0 z-[999] bg-slate-900/70 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl p-8 max-w-sm w-full shadow-2xl text-center space-y-5">
        <div className="relative flex justify-center">
          <div className="w-14 h-14 border-4 border-indigo-100 border-t-indigo-600 rounded-full animate-spin" />
          <span className="absolute inset-0 flex items-center justify-center text-lg">
            {mode === "circular" ? "📄" : mode === "signing" ? "✍️" : "⏳"}
          </span>
        </div>
        {mode === "circular" ? (
          <>
            <div>
              <h3 className="text-lg font-bold text-slate-900">Génération de la circulaire</h3>
              <p className="text-slate-500 text-sm mt-2 leading-relaxed">
                Analyse en cours — comptez une dizaine de secondes. Ne quittez pas cette page.
              </p>
            </div>
            <p className="text-xs text-indigo-700 bg-indigo-50 rounded-xl p-3">
              Le document sera ajouté sous le nom <strong>Circulaire Parents</strong>.
            </p>
          </>
        ) : (
          <div>
            <h3 className="text-lg font-bold text-slate-900">
              {mode === "signing" ? "Signature en cours…" : "Traitement en cours…"}
            </h3>
            <p className="text-slate-500 text-sm mt-1">Merci de patienter.</p>
          </div>
        )}
      </div>
    </div>
  );
}
