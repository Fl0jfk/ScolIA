"use client";

import type { ReactNode } from "react";
import { useId } from "react";

const ICON_CLASS = "w-12 h-14 shrink-0 drop-shadow-sm";

/** Dossier style Finder (macOS) — forme générique, pas de logo Apple. */
export function DocumentFolderIcon({ variant }: { variant?: "shared-incoming" | "default" }) {
  const gid = useId();
  const shared = variant === "shared-incoming";
  const top = shared ? "#8B9CF6" : "#6CB4FF";
  const bottom = shared ? "#6366F1" : "#3B8BEB";
  const tab = shared ? "#A5B4FC" : "#89C4FF";

  return (
    <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
      <defs>
        <linearGradient id={`${gid}-body`} x1="24" y1="16" x2="24" y2="52" gradientUnits="userSpaceOnUse">
          <stop stopColor={top} />
          <stop offset="1" stopColor={bottom} />
        </linearGradient>
        <linearGradient id={`${gid}-tab`} x1="14" y1="10" x2="14" y2="22" gradientUnits="userSpaceOnUse">
          <stop stopColor={tab} />
          <stop offset="1" stopColor={top} />
        </linearGradient>
      </defs>
      <path
        fill={`url(#${gid}-tab)`}
        d="M6 18c0-1.1.9-2 2-2h9.2l2.4-3.6c.4-.6 1-.9 1.6-.9H38c2.2 0 4 1.8 4 4v3H8c-1.1 0-2 .9-2 2v-.5z"
      />
      <path
        fill={`url(#${gid}-body)`}
        d="M6 22h36c2.2 0 4 1.8 4 4v22c0 2.2-1.8 4-4 4H10c-2.2 0-4-1.8-4-4V22z"
      />
      <path
        fill="#000"
        fillOpacity="0.08"
        d="M6 22h36c2.2 0 4 1.8 4 4v1H6v-1c0-2.2 1.8-4 4-4z"
      />
      {shared ? (
        <g fill="#fff" fillOpacity="0.95">
          <circle cx="36" cy="40" r="7" fill="#4F46E5" fillOpacity="1" />
          <path
            d="M33.5 40c0-1.1.9-2 2-2s2 .9 2 2-.9 2-2 2m-3.5 2.5c.6-.9 1.6-1.5 2.7-1.5s2.1.6 2.7 1.5"
            stroke="#fff"
            strokeWidth="1.2"
            strokeLinecap="round"
            fill="none"
          />
        </g>
      ) : null}
    </svg>
  );
}

type FileKind = "pdf" | "word" | "excel" | "powerpoint" | "image" | "zip" | "text" | "generic";

function fileKindFromExt(ext?: string): FileKind {
  const k = (ext || "").toLowerCase();
  if (k === "pdf") return "pdf";
  if (k === "doc" || k === "docx" || k === "odt" || k === "rtf") return "word";
  if (k === "xls" || k === "xlsx" || k === "ods" || k === "csv") return "excel";
  if (k === "ppt" || k === "pptx" || k === "odp") return "powerpoint";
  if (["jpg", "jpeg", "png", "gif", "webp", "svg", "heic", "bmp"].includes(k)) return "image";
  if (["zip", "rar", "7z", "tar", "gz"].includes(k)) return "zip";
  if (["txt", "md", "log"].includes(k)) return "text";
  return "generic";
}

function DocSheet({
  gid,
  body,
  fold,
  children,
}: {
  gid: string;
  body: string;
  fold: string;
  children?: ReactNode;
}) {
  return (
    <>
      <path fill={body} d="M10 6h20l8 8v34c0 2.2-1.8 4-4 4H14c-2.2 0-4-1.8-4-4V10c0-2.2 1.8-4 4-4z" />
      <path fill={fold} d="M30 6v8h8l-8-8z" />
      <path fill="#000" fillOpacity="0.06" d="M10 6h20v8H14c-2.2 0-4 1.8-4 4v-2c0-2.2 1.8-4 4-4h-4z" />
      {children}
    </>
  );
}

/** Icônes fichiers inspirées des vues système (couleurs familières, sans logos déposés). */
export function DocumentFileIcon({ ext }: { ext?: string }) {
  const gid = useId();
  const kind = fileKindFromExt(ext);

  if (kind === "pdf") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
          <rect x="10" y="6" width="28" height="10" fill="#E53935" rx="1" />
          <text x="24" y="13.5" textAnchor="middle" fill="#fff" fontSize="7" fontWeight="800" fontFamily="Inter,sans-serif">
            PDF
          </text>
          <rect x="14" y="22" width="16" height="1.5" rx=".75" fill="#CBD5E1" />
          <rect x="14" y="26" width="20" height="1.5" rx=".75" fill="#E2E8F0" />
          <rect x="14" y="30" width="18" height="1.5" rx=".75" fill="#E2E8F0" />
          <rect x="14" y="34" width="20" height="1.5" rx=".75" fill="#E2E8F0" />
        </DocSheet>
      </svg>
    );
  }

  if (kind === "word") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
          <rect x="10" y="6" width="28" height="14" fill="#2B579A" rx="1" />
          <text x="24" y="16" textAnchor="middle" fill="#fff" fontSize="11" fontWeight="800" fontFamily="Inter,sans-serif">
            W
          </text>
          <rect x="14" y="24" width="18" height="1.5" rx=".75" fill="#CBD5E1" />
          <rect x="14" y="28" width="20" height="1.5" rx=".75" fill="#E2E8F0" />
          <rect x="14" y="32" width="16" height="1.5" rx=".75" fill="#E2E8F0" />
        </DocSheet>
      </svg>
    );
  }

  if (kind === "excel") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
          <rect x="10" y="6" width="28" height="14" fill="#217346" rx="1" />
          <text x="24" y="16" textAnchor="middle" fill="#fff" fontSize="10" fontWeight="800" fontFamily="Inter,sans-serif">
            X
          </text>
          <g stroke="#94A3B8" strokeWidth="0.8">
            <rect x="14" y="23" width="20" height="14" fill="#F0FDF4" rx="1" />
            <path d="M14 27h20M14 31h20M20 23v14M26 23v14" />
          </g>
        </DocSheet>
      </svg>
    );
  }

  if (kind === "powerpoint") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
          <rect x="10" y="6" width="28" height="14" fill="#D24726" rx="1" />
          <text x="24" y="16" textAnchor="middle" fill="#fff" fontSize="10" fontWeight="800" fontFamily="Inter,sans-serif">
            P
          </text>
          <circle cx="24" cy="32" r="6" fill="none" stroke="#FDBA74" strokeWidth="2" />
          <path d="M24 26v6l4 2" stroke="#FDBA74" strokeWidth="1.5" strokeLinecap="round" fill="none" />
        </DocSheet>
      </svg>
    );
  }

  if (kind === "image") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
          <rect x="14" y="22" width="20" height="14" rx="1.5" fill="#EDE9FE" />
          <circle cx="19" cy="27" r="2" fill="#FDE68A" />
          <path d="M14 36l5-5 4 4 3-3 8 8H14v-4z" fill="#8B5CF6" fillOpacity="0.55" />
        </DocSheet>
      </svg>
    );
  }

  if (kind === "zip") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <defs>
          <linearGradient id={`${gid}-zip`} x1="24" y1="8" x2="24" y2="48" gradientUnits="userSpaceOnUse">
            <stop stopColor="#FDE68A" />
            <stop offset="1" stopColor="#D97706" />
          </linearGradient>
        </defs>
        <path
          fill={`url(#${gid}-zip)`}
          d="M12 14h24c2.2 0 4 1.8 4 4v24c0 2.2-1.8 4-4 4H12c-2.2 0-4-1.8-4-4V18c0-2.2 1.8-4 4-4z"
        />
        <rect x="22" y="10" width="4" height="3" rx="1" fill="#92400E" fillOpacity="0.5" />
        <rect x="22" y="15" width="4" height="2" rx="1" fill="#92400E" fillOpacity="0.4" />
        <rect x="22" y="19" width="4" height="2" rx="1" fill="#92400E" fillOpacity="0.4" />
        <path d="M18 28h12M18 32h12M18 36h8" stroke="#78350F" strokeWidth="1.5" strokeLinecap="round" opacity="0.5" />
      </svg>
    );
  }

  if (kind === "text") {
    return (
      <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
        <DocSheet gid={gid} body="#FFFFFF" fold="#E2E8F0">
          {[22, 26, 30, 34, 38].map((y) => (
            <rect key={y} x="14" y={y} width={y === 38 ? 12 : 20} height="1.5" rx=".75" fill="#CBD5E1" />
          ))}
        </DocSheet>
      </svg>
    );
  }

  const label = (ext || "FILE").slice(0, 4).toUpperCase();
  return (
    <svg viewBox="0 0 48 56" className={ICON_CLASS} aria-hidden>
      <DocSheet gid={gid} body="#F8FAFC" fold="#E2E8F0">
        <rect x="10" y="6" width="28" height="10" fill="#64748B" rx="1" />
        <text x="24" y="13.5" textAnchor="middle" fill="#fff" fontSize="6.5" fontWeight="800" fontFamily="Inter,sans-serif">
          {label}
        </text>
        <rect x="14" y="22" width="18" height="1.5" rx=".75" fill="#CBD5E1" />
        <rect x="14" y="26" width="20" height="1.5" rx=".75" fill="#E2E8F0" />
        <rect x="14" y="30" width="16" height="1.5" rx=".75" fill="#E2E8F0" />
      </DocSheet>
    </svg>
  );
}
