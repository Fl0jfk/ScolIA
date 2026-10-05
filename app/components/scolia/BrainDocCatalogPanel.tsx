"use client";

import { useMemo, useState, type MouseEvent } from "react";
import { DocumentFileIcon } from "@/app/components/documents/DocumentSystemIcons";
import type { ScoliaMemoryDocCatalog } from "@/app/lib/brain-ai/scolia-memory";

type Props = {
  catalog: ScoliaMemoryDocCatalog;
  onPreview: (href: string, label: string) => void;
  onOpenDossier: ((eleveId: string) => void) | null;
  onNavigate: (href: string) => void;
};

function ChevronIcon({ open }: { open: boolean }) {
  return (
    <svg
      viewBox="0 0 20 20"
      className={`h-4 w-4 shrink-0 text-neutral-400 transition ${open ? "" : "-rotate-90"}`}
      aria-hidden
    >
      <path
        d="M5.5 7.5 10 12l4.5-4.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function UserIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" aria-hidden>
      <path
        d="M12 12a4 4 0 1 0-4-4 4 4 0 0 0 4 4Zm0 2c-4.4 0-8 2.2-8 5v1h16v-1c0-2.8-3.6-5-8-5Z"
        fill="currentColor"
      />
    </svg>
  );
}

function dossierIdFromHref(href: string): string | null {
  const m = href.match(/^\/eleves\/dossier\/([^/?#]+)\/?$/);
  return m?.[1] ?? null;
}

function CatalogDocCard({
  title,
  subtitle,
  href,
  preview,
  dossierHref,
  ext,
  onPreview,
  onOpenDossier,
  onNavigate,
}: {
  title: string;
  subtitle?: string;
  href: string;
  preview?: boolean;
  dossierHref?: string;
  ext?: string;
  onPreview: (href: string, label: string) => void;
  onOpenDossier: ((eleveId: string) => void) | null;
  onNavigate: (href: string) => void;
}) {
  const isPreview = Boolean(preview);
  const open = () => {
    if (isPreview) {
      onPreview(href, title);
      return;
    }
    const dossierId = dossierIdFromHref(href);
    if (dossierId && onOpenDossier) {
      onOpenDossier(dossierId);
      return;
    }
    onNavigate(href);
  };

  const openFiche = (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    e.stopPropagation();
    if (!dossierHref) return;
    const dossierId = dossierIdFromHref(dossierHref);
    if (dossierId && onOpenDossier) {
      onOpenDossier(dossierId);
      return;
    }
    onNavigate(dossierHref);
  };

  return (
    <div className="group relative flex min-w-0 flex-col items-center rounded-2xl border-2 border-transparent bg-transparent p-2 transition-all hover:border-black/5 hover:bg-white hover:shadow-sm">
      <button
        type="button"
        onClick={open}
        title={title}
        className="flex w-full min-w-0 cursor-pointer flex-col items-center text-left"
      >
        <div className="mb-1.5 shrink-0">
          {isPreview ? (
            <DocumentFileIcon ext={ext || "pdf"} />
          ) : (
            <span className="flex h-14 w-12 items-center justify-center rounded-xl bg-[var(--dash-ink)] text-white shadow-sm">
              <UserIcon />
            </span>
          )}
        </div>
        <span className="line-clamp-2 w-full px-0.5 text-center text-[10px] font-medium leading-snug break-words text-gray-700">
          {title}
        </span>
        {subtitle ? (
          <span className="mt-0.5 text-center text-[9px] font-medium leading-tight text-gray-500">
            {subtitle}
          </span>
        ) : null}
      </button>
      {dossierHref && isPreview ? (
        <button
          type="button"
          onClick={openFiche}
          className="mt-1 text-[9px] font-semibold text-neutral-400 opacity-0 transition group-hover:opacity-100 hover:text-[var(--dash-ink)]"
        >
          Fiche
        </button>
      ) : null}
    </div>
  );
}

/**
 * Catalogue documents Brain AI — style cloud personnel, groupé par classe.
 */
export default function BrainDocCatalogPanel({
  catalog,
  onPreview,
  onOpenDossier,
  onNavigate,
}: Props) {
  const groupCount = catalog.groups.length;
  const defaultOpen = useMemo(() => {
    if (groupCount <= 6) return new Set(catalog.groups.map((g) => g.title));
    return new Set(catalog.groups.slice(0, 3).map((g) => g.title));
  }, [catalog.groups, groupCount]);

  const [openGroups, setOpenGroups] = useState<Set<string>>(defaultOpen);

  const toggle = (title: string) => {
    setOpenGroups((prev) => {
      const next = new Set(prev);
      if (next.has(title)) next.delete(title);
      else next.add(title);
      return next;
    });
  };

  const expandAll = () => setOpenGroups(new Set(catalog.groups.map((g) => g.title)));
  const collapseAll = () => setOpenGroups(new Set());

  return (
    <div className="mt-2.5 overflow-hidden rounded-2xl border border-black/8 bg-[#f4f5f3]">
      <div className="flex items-start justify-between gap-2 border-b border-black/5 px-3 py-2.5">
        <div className="min-w-0">
          <p className="text-[10px] font-semibold uppercase tracking-[0.08em] text-neutral-500">
            {catalog.kindLabel ? `Documents · ${catalog.kindLabel}` : "Documents"}
          </p>
          <p className="mt-0.5 truncate text-[13px] font-semibold text-[var(--dash-ink)]">
            {catalog.title}
          </p>
        </div>
        {groupCount > 1 ? (
          <div className="flex shrink-0 gap-1.5 pt-0.5">
            <button
              type="button"
              onClick={expandAll}
              className="text-[10px] font-semibold text-neutral-500 hover:text-[var(--dash-ink)]"
            >
              Tout ouvrir
            </button>
            <span className="text-neutral-300" aria-hidden>
              ·
            </span>
            <button
              type="button"
              onClick={collapseAll}
              className="text-[10px] font-semibold text-neutral-500 hover:text-[var(--dash-ink)]"
            >
              Réduire
            </button>
          </div>
        ) : null}
      </div>

      <div className="max-h-[min(62vh,520px)] space-y-1 overflow-y-auto p-2">
        {catalog.groups.map((group) => {
          const open = openGroups.has(group.title);
          return (
            <section
              key={group.title}
              className="overflow-hidden rounded-xl border border-black/5 bg-white/70"
            >
              <button
                type="button"
                onClick={() => toggle(group.title)}
                className="flex w-full items-center gap-2 px-3 py-2 text-left transition hover:bg-white"
                aria-expanded={open}
              >
                <ChevronIcon open={open} />
                <span className="min-w-0 flex-1 truncate text-[13px] font-semibold text-[var(--dash-ink)]">
                  {group.title}
                </span>
                <span className="shrink-0 rounded-full bg-[color:var(--dash-lime)]/50 px-2 py-0.5 text-[10px] font-bold text-[var(--dash-ink)]">
                  {group.count}
                </span>
              </button>
              {open ? (
                <div className="grid grid-cols-2 gap-1 border-t border-black/5 px-1.5 py-2 sm:grid-cols-3">
                  {group.items.map((item, idx) => (
                    <CatalogDocCard
                      key={`${item.href}_${idx}`}
                      title={item.title}
                      subtitle={item.subtitle}
                      href={item.href}
                      preview={item.preview}
                      dossierHref={item.dossierHref}
                      ext={item.ext}
                      onPreview={onPreview}
                      onOpenDossier={onOpenDossier}
                      onNavigate={onNavigate}
                    />
                  ))}
                </div>
              ) : null}
            </section>
          );
        })}
      </div>
    </div>
  );
}
