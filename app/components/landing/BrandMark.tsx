import { MARKETING } from "@/app/lib/marketing-site";

type Size = "sm" | "md" | "lg" | "xl";

const SIZE: Record<Size, string> = {
  sm: "text-sm",
  md: "text-xl",
  lg: "text-2xl md:text-3xl",
  xl: "text-4xl sm:text-5xl",
};

/**
 * Marque ScolIA — Scol + IA collés.
 * Dans I et A : bulles vertes type lava lamp (verre flottant), clipées au glyphe.
 */
export default function BrandMark({
  size = "md",
  className = "",
  invert = false,
}: {
  size?: Size;
  className?: string;
  invert?: boolean;
}) {
  const scol = invert ? "text-white" : "text-[#14231A]";

  return (
    <span
      className={`inline-flex items-baseline tracking-tight ${SIZE[size]} ${className}`}
      aria-label={MARKETING.productName}
    >
      <span className={`font-semibold ${scol}`}>Scol</span>
      <span className="inline-flex font-black tracking-tight" aria-hidden>
        <span className={`scolia-brand-letter ${invert ? "scolia-brand-letter--invert" : ""}`}>
          I
        </span>
        <span
          className={`scolia-brand-letter scolia-brand-letter--delay ${invert ? "scolia-brand-letter--invert" : ""}`}
        >
          A
        </span>
      </span>
    </span>
  );
}
