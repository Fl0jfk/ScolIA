"use client";

import type { ToolboxToolId } from "@/app/lib/toolbox-types";
import type { ToolboxAdminLinkId } from "@/app/lib/toolbox-tools";

type IconProps = { className?: string };

export function QrToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="4" y="4" width="16" height="16" rx="2" fill="currentColor" opacity="0.9" />
      <rect x="28" y="4" width="16" height="16" rx="2" fill="currentColor" opacity="0.9" />
      <rect x="4" y="28" width="16" height="16" rx="2" fill="currentColor" opacity="0.9" />
      <rect x="30" y="30" width="6" height="6" fill="currentColor" />
      <rect x="38" y="30" width="6" height="6" fill="currentColor" />
      <rect x="30" y="38" width="6" height="6" fill="currentColor" />
      <rect x="38" y="38" width="6" height="6" fill="currentColor" />
    </svg>
  );
}

export function SantaToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <circle cx="24" cy="28" r="12" fill="currentColor" opacity="0.85" />
      <path d="M12 18 L24 8 L36 18 L32 22 H16 Z" fill="currentColor" />
      <circle cx="34" cy="14" r="4" fill="white" opacity="0.9" />
    </svg>
  );
}

export function RentreeToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="8" y="14" width="32" height="24" rx="3" fill="currentColor" opacity="0.85" />
      <path d="M8 18 L24 10 L40 18" stroke="white" strokeWidth="2" fill="none" />
      <rect x="20" y="26" width="8" height="12" fill="white" opacity="0.9" />
    </svg>
  );
}

export function TarifsToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="10" y="6" width="28" height="36" rx="4" fill="currentColor" opacity="0.85" />
      <rect x="16" y="14" width="16" height="3" rx="1" fill="white" opacity="0.9" />
      <rect x="16" y="22" width="12" height="3" rx="1" fill="white" opacity="0.7" />
      <rect x="16" y="30" width="14" height="3" rx="1" fill="white" opacity="0.7" />
      <text x="30" y="38" fontSize="10" fill="white" fontWeight="bold">
        €
      </text>
    </svg>
  );
}

export function FournituresToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="6" y="12" width="36" height="28" rx="4" fill="currentColor" opacity="0.85" />
      <rect x="14" y="8" width="4" height="12" fill="currentColor" />
      <rect x="22" y="6" width="4" height="14" fill="currentColor" />
      <rect x="30" y="9" width="4" height="11" fill="currentColor" />
    </svg>
  );
}

export function PortesOuvertesToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="8" y="10" width="32" height="30" rx="3" fill="currentColor" opacity="0.85" />
      <rect x="14" y="16" width="20" height="18" rx="2" fill="white" opacity="0.25" />
      <circle cx="30" cy="26" r="2" fill="white" />
      <path d="M12 6 H36" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}

export function RepartitionClassesToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <rect x="6" y="8" width="14" height="14" rx="3" fill="currentColor" opacity="0.9" />
      <rect x="28" y="8" width="14" height="14" rx="3" fill="currentColor" opacity="0.65" />
      <rect x="6" y="26" width="14" height="14" rx="3" fill="currentColor" opacity="0.65" />
      <rect x="28" y="26" width="14" height="14" rx="3" fill="currentColor" opacity="0.9" />
    </svg>
  );
}

export function ToolboxFolderIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <path
        d="M6 14 H20 L24 10 H42 C43.1 10 44 10.9 44 12 V38 C44 39.1 43.1 40 42 40 H6 C4.9 40 4 39.1 4 38 V16 C4 14.9 4.9 14 6 14 Z"
        fill="currentColor"
        opacity="0.9"
      />
    </svg>
  );
}

export function ParametresToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <circle cx="24" cy="24" r="8" fill="none" stroke="currentColor" strokeWidth="3" opacity="0.9" />
      <path
        d="M24 6v6M24 36v6M6 24h6M36 24h6M11.5 11.5l4.2 4.2M32.3 32.3l4.2 4.2M11.5 36.5l4.2-4.2M32.3 15.7l4.2-4.2"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function UtilisateursToolboxIcon({ className = "w-10 h-10" }: IconProps) {
  return (
    <svg viewBox="0 0 48 48" className={className} aria-hidden>
      <circle cx="18" cy="16" r="6" fill="currentColor" opacity="0.9" />
      <circle cx="32" cy="18" r="5" fill="currentColor" opacity="0.65" />
      <path
        d="M8 38c0-6 4.5-10 10-10s10 4 10 10M26 38c0-4.5 3-7.5 7-7.5"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
        fill="none"
      />
    </svg>
  );
}

export function renderToolboxIcon(id: ToolboxToolId, className?: string) {
  switch (id) {
    case "qrcreator":
      return <QrToolboxIcon className={className} />;
    case "secret-santa":
      return <SantaToolboxIcon className={className} />;
    case "rentree":
      return <RentreeToolboxIcon className={className} />;
    case "simulateur-tarifs":
      return <TarifsToolboxIcon className={className} />;
    case "simulateur-fournitures":
      return <FournituresToolboxIcon className={className} />;
    case "portes-ouvertes":
      return <PortesOuvertesToolboxIcon className={className} />;
    case "repartition-classes":
      return <RepartitionClassesToolboxIcon className={className} />;
    default:
      return <ToolboxFolderIcon className={className} />;
  }
}

export function renderToolboxAdminIcon(id: ToolboxAdminLinkId, className?: string) {
  switch (id) {
    case "parametres":
      return <ParametresToolboxIcon className={className} />;
    default:
      return <ToolboxFolderIcon className={className} />;
  }
}
