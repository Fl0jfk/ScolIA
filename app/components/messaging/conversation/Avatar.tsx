import type { CSSProperties } from "react";

/** Teinte stable (0–359) dérivée du nom. */
function hueOf(name: string): number {
  let h = 0;
  const s = (name || "?").trim().toLowerCase();
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
  return h;
}

/** Couleurs pastel stables (fond clair, texte foncé : contraste ≥ 4.5). */
export function avatarTone(name: string): CSSProperties {
  const h = hueOf(name);
  return { backgroundColor: `hsl(${h} 55% 86%)`, color: `hsl(${h} 45% 24%)` };
}

/** Initiales : première lettre du prénom + du dernier mot (max 2). */
export function initialsOf(name: string): string {
  const parts = (name || "").trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  const first = Array.from(parts[0])[0] ?? "?";
  const last = parts.length > 1 ? (Array.from(parts[parts.length - 1])[0] ?? "") : "";
  return (first + last).toUpperCase();
}

/** Avatar rond : photo, sinon initiales sur pastel. Même composant partout. */
export default function Avatar({
  name,
  imageUrl,
  size = 28,
  square = false,
  className = "",
}: {
  name: string;
  imageUrl?: string | null;
  size?: number;
  /** Coins arrondis (liste, tiroir) plutôt que rond. */
  square?: boolean;
  className?: string;
}) {
  return (
    <span
      className={`flex shrink-0 select-none items-center justify-center overflow-hidden font-semibold ${
        square ? "rounded-2xl" : "rounded-full"
      } ${className}`}
      style={{
        width: size,
        height: size,
        fontSize: Math.max(9, Math.round(size * 0.38)),
        ...(imageUrl ? { backgroundColor: "#d4d4d8" } : avatarTone(name)),
      }}
      aria-hidden
    >
      {imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={imageUrl} alt="" className="h-full w-full object-cover" />
      ) : (
        initialsOf(name)
      )}
    </span>
  );
}
