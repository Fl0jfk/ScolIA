import { MARKETING } from "@/app/lib/marketing-site";

type Size = "sm" | "md" | "lg" | "xl";

const SIZE: Record<Size, string> = {
  sm: "text-sm",
  md: "text-xl",
  lg: "text-2xl md:text-3xl",
  xl: "text-4xl sm:text-5xl",
};

/**
 * Marque ScolIA — Scol + IA collés, « IA » en verre animé (bounce).
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
  const ia = invert ? "text-[#4ADE80]" : "text-[#2F6B4A]";

  return (
    <span
      className={`inline-flex items-baseline tracking-tight ${SIZE[size]} ${className}`}
      aria-label={MARKETING.productName}
    >
      <span className={`font-semibold ${scol}`}>Scol</span>
      <span className={`scolia-brand-ia relative inline-block font-black tracking-tight ${ia}`}>
        <span
          className={`scolia-brand-ia-glass ${invert ? "scolia-brand-ia-glass--invert" : ""}`}
          aria-hidden
        />
        <span className="relative z-[1]">IA</span>
      </span>
    </span>
  );
}
