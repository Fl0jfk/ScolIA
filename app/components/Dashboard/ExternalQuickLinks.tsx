"use client";

import Link from "next/link";
import type { DashboardQuickLink } from "@/app/lib/dashboard-quick-links";
import { dash } from "@/app/lib/dashboard-brand";

export function QuickLinkIcon({ src, name }: { src: string; name: string }) {
  if (!src) {
    return (
      <div
        className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-[10px] font-bold ${dash.bgSoftMuted} ${dash.textMid}`}
      >
        {name.slice(0, 1).toUpperCase() || "?"}
      </div>
    );
  }
  return (
    <div className={`relative h-7 w-7 shrink-0 overflow-hidden rounded-lg ${dash.bgSoftMuted}`}>
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src={src} alt="" className="h-full w-full object-contain p-0.5" />
    </div>
  );
}

type Props = {
  links: DashboardQuickLink[];
  /** Lien vers la gestion des accès rapides (affiché au hover sur le label). */
  manageHref?: string | null;
  /** Variante compacte pour le header site. */
  compact?: boolean;
  className?: string;
};

export function ExternalQuickLinksBar({
  links,
  manageHref = null,
  compact = false,
  className = "",
}: Props) {
  if (links.length === 0 && !manageHref) return null;

  return (
    <div
      className={`group/quick flex flex-wrap items-center gap-1.5 ${
        compact ? "justify-center" : "justify-start sm:justify-end"
      } ${className}`}
    >
      <span
        className={`relative inline-flex items-center gap-1 text-[10px] font-bold uppercase tracking-widest ${dash.label} ${
          compact ? "" : "w-full sm:mr-1 sm:w-auto"
        }`}
      >
        Accès rapides
        {manageHref ? (
          <Link
            href={manageHref}
            title="Ajouter ou modifier les accès rapides"
            aria-label="Gérer les accès rapides"
            className="inline-flex h-4 w-4 translate-y-px cursor-pointer items-center justify-center rounded-full bg-[var(--dash-primary)] text-[11px] font-black leading-none text-white opacity-0 shadow-sm transition-all duration-200 group-hover/quick:opacity-100 hover:scale-110 hover:brightness-110"
          >
            +
          </Link>
        ) : null}
      </span>
      {links.map((link) => (
        <a
          key={link.id}
          href={link.link}
          target="_blank"
          rel="noopener noreferrer"
          title={link.name}
          className={`flex cursor-pointer items-center gap-2 rounded-xl border bg-white/90 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md ${dash.borderSoft} ${dash.hoverBorder} ${
            compact ? "px-2 py-1" : "px-2.5 py-1.5"
          }`}
        >
          <QuickLinkIcon src={link.img} name={link.name} />
          <span
            className={`truncate font-bold text-stone-600 ${dash.hoverPrimary} ${
              compact ? "max-w-[6.5rem] text-[11px]" : "max-w-[8rem] text-xs sm:max-w-[10rem]"
            }`}
          >
            {link.name}
          </span>
        </a>
      ))}
    </div>
  );
}
