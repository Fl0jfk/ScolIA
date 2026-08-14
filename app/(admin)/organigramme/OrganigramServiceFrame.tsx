"use client";

import type { ReactNode, ReactElement } from "react";
import { useId } from "react";

type OrganigramFrameVariant = | "direction" | "admin" | "accounting" | "poles" | "poleEcole" | "poleCollege" | "poleLycee" | "reception" | "health" | "maintenance" | "pastoral" | "ogec" | "tutelle";

type Theme = {
  badge: string;
  badgeColor: string;
  iconBg: string;
  border: string;
  outer: string;
  shadow: string;
  inner: string;
};

const THEMES: Record<OrganigramFrameVariant, Theme> = {
  direction: {
    badge: "Direction",
    badgeColor: "text-indigo-200",
    iconBg: "bg-indigo-950 shadow-lg ring-2 ring-indigo-400/30",
    border: "border-indigo-400/35",
    outer: "bg-gradient-to-br from-indigo-950 via-indigo-900 to-slate-950",
    shadow: "shadow-[0_1px_0_rgba(255,255,255,0.06)_inset,0_24px_50px_-28px_rgba(30,27,75,0.75)]",
    inner: "bg-white/[0.97] border border-indigo-100/80 shadow-md",
  },
  admin: {
    badge: "Administration",
    badgeColor: "text-sky-100",
    iconBg: "bg-sky-900 shadow-lg ring-2 ring-sky-400/25",
    border: "border-sky-500/40",
    outer: "bg-gradient-to-br from-sky-900 via-slate-800 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.08),0_20px_45px_-24px_rgba(12,74,110,0.5)]",
    inner: "bg-white/[0.97] border border-sky-100/90 shadow-md",
  },
  accounting: {
    badge: "Comptabilité",
    badgeColor: "text-emerald-100",
    iconBg: "bg-emerald-900 shadow-lg ring-2 ring-emerald-400/25",
    border: "border-emerald-500/40",
    outer: "bg-gradient-to-br from-emerald-900 via-teal-900 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_20px_45px_-24px_rgba(6,78,59,0.55)]",
    inner: "bg-white/[0.97] border border-emerald-100/90 shadow-md",
  },
  reception: {
    badge: "Accueil",
    badgeColor: "text-violet-100",
    iconBg: "bg-violet-900 shadow-lg ring-2 ring-violet-400/25",
    border: "border-violet-400/35",
    outer: "bg-gradient-to-br from-violet-900 via-slate-800 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.07),0_20px_45px_-24px_rgba(76,29,149,0.45)]",
    inner: "bg-white/[0.97] border border-violet-100/90 shadow-md",
  },
  health: {
    badge: "Santé",
    badgeColor: "text-teal-100",
    iconBg: "bg-teal-900 shadow-lg ring-2 ring-teal-300/30",
    border: "border-teal-400/40",
    outer: "bg-gradient-to-br from-teal-900 via-emerald-900 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_20px_45px_-24px_rgba(15,118,110,0.5)]",
    inner: "bg-white/[0.97] border border-teal-100/90 shadow-md",
  },
  poles: {
    badge: "Vie scolaire",
    badgeColor: "text-cyan-100",
    iconBg: "bg-cyan-950 shadow-lg ring-2 ring-cyan-400/25",
    border: "border-cyan-500/35",
    outer: "bg-gradient-to-br from-cyan-950 via-slate-800 to-slate-950",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_20px_45px_-24px_rgba(14,116,144,0.45)]",
    inner: "bg-transparent border-0 shadow-none p-0",
  },
  poleEcole: {
    badge: "École",
    badgeColor: "text-green-100",
    iconBg: "bg-green-800 shadow-md ring-2 ring-green-400/30",
    border: "border-green-500/40",
    outer: "bg-gradient-to-br from-green-900 via-emerald-900 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_16px_40px_-22px_rgba(20,83,45,0.45)]",
    inner: "bg-white/[0.97] border border-green-100/90 shadow-md",
  },
  poleCollege: {
    badge: "Collège",
    badgeColor: "text-blue-100",
    iconBg: "bg-blue-900 shadow-md ring-2 ring-blue-400/25",
    border: "border-blue-400/40",
    outer: "bg-gradient-to-br from-blue-900 via-indigo-900 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_16px_40px_-22px_rgba(30,58,138,0.45)]",
    inner: "bg-white/[0.97] border border-blue-100/90 shadow-md",
  },
  poleLycee: {
    badge: "Lycée",
    badgeColor: "text-fuchsia-100",
    iconBg: "bg-fuchsia-950 shadow-md ring-2 ring-fuchsia-400/25",
    border: "border-fuchsia-500/35",
    outer: "bg-gradient-to-br from-fuchsia-950 via-purple-900 to-slate-900",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_16px_40px_-22px_rgba(112,26,117,0.45)]",
    inner: "bg-white/[0.97] border border-fuchsia-100/80 shadow-md",
  },
  maintenance: {
    badge: "Maintenance",
    badgeColor: "text-amber-100",
    iconBg: "bg-amber-950 shadow-lg ring-2 ring-amber-400/35",
    border: "border-amber-600/45",
    outer: "bg-gradient-to-br from-amber-950 via-amber-900 to-stone-900",
    shadow: "shadow-[0_1px_0_rgba(255,255,255,0.08)_inset,0_24px_50px_-28px_rgba(120,53,15,0.55)]",
    inner: "bg-white/[0.97] border border-amber-100/90 shadow-md",
  },
  pastoral: {
    badge: "Pastorale",
    badgeColor: "text-rose-100",
    iconBg: "bg-rose-950 shadow-lg ring-2 ring-rose-400/25",
    border: "border-rose-500/35",
    outer: "bg-gradient-to-br from-rose-950 via-amber-900/90 to-stone-900",
    shadow: "shadow-[0_1px_0_rgba(255,255,255,0.06)_inset,0_24px_50px_-28px_rgba(136,19,55,0.45)]",
    inner: "bg-white/[0.97] border border-rose-100/90 shadow-md",
  },
  ogec: {
    badge: "OGEC",
    badgeColor: "text-slate-200",
    iconBg: "bg-slate-800 shadow-lg ring-2 ring-slate-400/35",
    border: "border-slate-500/40",
    outer: "bg-gradient-to-br from-slate-800 via-slate-900 to-zinc-950",
    shadow: "shadow-[inset_0_1px_0_rgba(255,255,255,0.05),0_22px_48px_-26px_rgba(0,0,0,0.65)]",
    inner: "bg-white/[0.97] border border-slate-200/90 shadow-md",
  },
  tutelle: {
    badge: "Tutelle",
    badgeColor: "text-amber-200",
    iconBg: "bg-amber-950 shadow-lg ring-2 ring-amber-500/40",
    border: "border-amber-800/40",
    outer: "bg-gradient-to-b from-amber-900/95 via-amber-950 to-stone-950",
    shadow: "shadow-[0_1px_0_rgba(255,255,255,0.12)_inset,0_24px_50px_-28px_rgba(69,26,3,0.55)]",
    inner: "bg-amber-50/95 border border-amber-200/60 shadow-inner",
  },
};

function IconDirection({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M2 12h4m12 0h4M4.93 19.07l2.83-2.83m8.48-8.48l2.83-2.83"
      />
      <circle cx="12" cy="12" r="3" strokeWidth="1.75" />
    </svg>
  );
}

function IconBuilding({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M6 22V10l6-4 6 4v12M6 22h12M9 22v-4h6v4M10 14h1M13 14h1M10 18h1M13 18h1"
      />
    </svg>
  );
}

function IconClipboard({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 4h6l1 3h3v14H5V7h3l1-3zm0 8h6M9 16h6"
      />
    </svg>
  );
}

function IconChart({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path strokeWidth="1.75" strokeLinecap="round" d="M4 19V5M8 17V9m4 8V7m4 12v-6m4 10v-9" />
    </svg>
  );
}

function IconPhone({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M14 2h-4l-2 5v11a2 2 0 002 2h6a2 2 0 002-2V7l-2-5zM10 18h4"
      />
    </svg>
  );
}

function IconHealth({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <circle cx="12" cy="12" r="10" strokeWidth="1.75" />
      <path strokeWidth="1.75" strokeLinecap="round" d="M12 8v8M8 12h8" />
    </svg>
  );
}

function IconWrench({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z"
      />
    </svg>
  );
}

function IconUsers({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" aria-hidden>
      <path
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zM23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75"
      />
    </svg>
  );
}

function IconPastoral({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <path strokeLinecap="round" d="M12 4v16M7 10h10" />
    </svg>
  );
}

function IconShield({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 28" fill="currentColor" aria-hidden>
      <path
        fillRule="evenodd"
        d="M12 2l8 3.2v8.2c0 4.3-3 8.1-8 12.6-5-4.5-8-8.3-8-12.6V5.2L12 2zm0 3.2L6 7.4v6.8c0 3 2.1 6 6 9.8 3.9-3.8 6-6.8 6-9.8V7.4l-6-2.2z"
        opacity="0.95"
      />
    </svg>
  );
}

const ICONS: Record<OrganigramFrameVariant, (p: { className?: string }) => ReactElement> = {
  direction: IconDirection,
  admin: IconClipboard,
  accounting: IconChart,
  reception: IconPhone,
  health: IconHealth,
  poles: IconUsers,
  poleEcole: IconUsers,
  poleCollege: IconUsers,
  poleLycee: IconUsers,
  maintenance: IconWrench,
  pastoral: IconPastoral,
  ogec: IconBuilding,
  tutelle: IconShield,
};

function DecoColumns({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 200 120" fill="none" aria-hidden>
      {[32, 72, 112, 152].map((x) => (
        <g key={x} opacity="0.12">
          <rect x={x} y="20" width="16" height="80" rx="2" fill="white" />
          <rect x={x + 2} y="24" width="12" height="6" rx="1" fill="white" fillOpacity="0.5" />
        </g>
      ))}
    </svg>
  );
}

function DecoCompass({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 120 120" fill="none" aria-hidden>
      <circle cx="60" cy="60" r="48" stroke="white" strokeOpacity="0.06" strokeWidth="1" />
      <circle cx="60" cy="60" r="32" stroke="white" strokeOpacity="0.08" strokeWidth="1" />
      <path d="M60 12 L68 52 L60 60 L52 52 Z" fill="white" fillOpacity="0.04" />
    </svg>
  );
}

function TutelleShieldBackdrop({ className }: { className?: string }) {
  const gid = useId().replace(/:/g, "");
  const gradId = `og-shield-grad-${gid}`;
  return (
    <svg className={className} viewBox="0 0 140 168" aria-hidden xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#fbbf24" stopOpacity="0.08" />
          <stop offset="100%" stopColor="#78350f" stopOpacity="0.06" />
        </linearGradient>
      </defs>
      <path
        d="M70 6 L124 28 v46 c0 28-18 52-54 84 C18 126 16 102 16 74 V28 Z"
        fill={`url(#${gradId})`}
        stroke="rgb(250 204 21 / 0.15)"
        strokeWidth="2"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function DecoGears({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 100 100" fill="none" aria-hidden>
      <circle cx="72" cy="28" r="14" stroke="white" strokeOpacity="0.07" strokeWidth="2" strokeDasharray="4 3" />
      <circle cx="24" cy="70" r="20" stroke="white" strokeOpacity="0.05" strokeWidth="1.5" />
    </svg>
  );
}

function ExploreRails({ slotIndex }: { slotIndex: number }) {
  const isEven = slotIndex % 2 === 0;
  const delaySec = slotIndex * 0.14;
  const sideEven = "-right-px lg:-right-1";
  const sideOdd = "-left-px lg:-left-1";
  return (
    <>
      <div
        aria-hidden
        className={`pointer-events-none absolute z-[1] top-[10%] bottom-[10%] w-1 rounded-full opacity-75 hidden md:block md:motion-safe:animate-pulse ${
          isEven ? sideEven : sideOdd
        }`}
        style={{
          background:
            "linear-gradient(180deg, transparent 0%, rgb(56 189 248 / 0.38) 42%, rgb(167 139 250 / 0.35) 58%, transparent 100%)",
          animationDuration: `${3.2 + (slotIndex % 4) * 0.45}s`,
          animationDelay: `${delaySec}s`,
        }}
      />
      <div
        aria-hidden
        className={`pointer-events-none absolute top-[26%] h-0.5 rounded-full opacity-60 hidden md:block md:motion-safe:animate-pulse w-12 lg:w-20 ${
          isEven ? "right-2 bg-gradient-to-l from-cyan-400/70 via-sky-400/40 to-transparent" : "left-2 bg-gradient-to-r from-cyan-400/70 via-sky-400/40 to-transparent"
        }`}
        style={{
          animationDuration: "2.6s",
          animationDelay: `${delaySec + 0.18}s`,
        }}
      />
    </>
  );
}

type ServiceFrameProps = {
  variant: OrganigramFrameVariant;
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
  narrow?: boolean;
  bareContent?: boolean;
  slotIndex?: number;
};

export function OrganigramServiceFrame({
  variant,
  title,
  description,
  children,
  className = "",
  narrow = false,
  bareContent = false,
  slotIndex,
}: ServiceFrameProps) {
  const t = THEMES[variant];
  const Icon = ICONS[variant];
  const titleClass = variant === "tutelle" ? "text-amber-950" : "text-slate-900";
  const descClass = variant === "tutelle" ? "text-amber-950/85" : "text-slate-600";

  const exploreStagger =
    narrow || slotIndex == null
      ? ""
      : slotIndex % 2 === 0
        ? "md:-translate-x-1 lg:-translate-x-2 xl:-translate-x-3 md:pr-1"
        : "md:translate-x-1 lg:translate-x-2 xl:translate-x-3 md:pl-1";

  return (
    <section
      className={`relative w-full scroll-mt-24 ${exploreStagger} ${narrow ? "max-w-md mx-auto" : ""} ${className}`.trim()}
    >
      {slotIndex != null && !narrow ? <ExploreRails slotIndex={slotIndex} /> : null}
      <div
        className={`relative z-[2] overflow-hidden rounded-[1.75rem] border-[3px] p-5 sm:p-7 transition-[transform,box-shadow] duration-300 ease-out md:duration-500 md:hover:-translate-y-1 md:hover:shadow-2xl motion-reduce:transition-none motion-reduce:md:hover:translate-y-0 ${t.border} ${t.outer} ${t.shadow}`}
      >
        {variant === "direction" && (
          <DecoCompass className="absolute -right-6 top-1/2 -translate-y-1/2 w-40 h-40 pointer-events-none text-white" />
        )}
        {variant === "ogec" && (
          <DecoColumns className="absolute right-0 bottom-0 w-48 h-28 pointer-events-none opacity-90" />
        )}
        {variant === "maintenance" && (
          <DecoGears className="absolute right-4 top-4 w-24 h-24 pointer-events-none" />
        )}
        {variant === "tutelle" && (
          <>
            <TutelleShieldBackdrop className="absolute -right-2 top-1/2 -translate-y-1/2 w-[min(40vw,200px)] h-auto opacity-90 pointer-events-none select-none" />
            <TutelleShieldBackdrop className="absolute -left-6 top-10 w-28 h-auto opacity-35 pointer-events-none select-none -rotate-6" />
          </>
        )}

        <div className="relative z-10 flex items-center gap-3 mb-4">
          <span
            className={`inline-flex items-center justify-center w-10 h-10 rounded-xl ${
              variant === "tutelle" ? "text-amber-300" : "text-white"
            } ${t.iconBg}`}
            aria-hidden
          >
            <Icon className="w-5 h-5" />
          </span>
          <p className={`text-[10px] font-bold uppercase tracking-[0.22em] ${t.badgeColor}`}>{t.badge}</p>
        </div>

        {bareContent ? (
          <>
            <h2 className={`text-lg sm:text-xl font-bold tracking-tight text-white mb-1 ${titleClass === "text-amber-950" ? "!text-white" : ""}`}>
              {title}
            </h2>
            {description ? (
              <p className="text-sm text-cyan-100/90 max-w-3xl leading-relaxed mb-6">{description}</p>
            ) : null}
            {children}
          </>
        ) : (
          <div className={`relative z-10 rounded-2xl p-5 sm:p-6 ${t.inner}`}>
            <h2 className={`text-lg sm:text-xl font-bold tracking-tight mb-1 ${titleClass}`}>{title}</h2>
            {description ? (
              <p className={`text-sm max-w-3xl leading-relaxed mb-6 ${descClass}`}>{description}</p>
            ) : null}
            {children}
          </div>
        )}
      </div>
    </section>
  );
}

export function OrganigramPoleColumn({
  poleVariant,
  label,
  children,
  className = "",
}: {
  poleVariant: "poleEcole" | "poleCollege" | "poleLycee";
  label: string;
  children: ReactNode;
  className?: string;
}) {
  const t = THEMES[poleVariant];
  const Icon = ICONS[poleVariant];
  return (
    <div
      className={`relative overflow-hidden rounded-2xl border-[3px] p-4 ${t.border} ${t.outer} ${t.shadow} ${className}`}
    >
      <div className="flex items-center gap-2 mb-3">
        <span
          className={`inline-flex items-center justify-center w-9 h-9 rounded-lg text-white ${t.iconBg}`}
          aria-hidden
        >
          <Icon className="w-4 h-4" />
        </span>
        <p className={`text-[10px] font-bold uppercase tracking-[0.18em] ${t.badgeColor}`}>{t.badge}</p>
      </div>
      <h3 className="text-sm font-black uppercase tracking-wide text-white border-b border-white/15 pb-2 mb-3">{label}</h3>
      <div className={`rounded-xl p-3 ${t.inner}`}>{children}</div>
    </div>
  );
}
