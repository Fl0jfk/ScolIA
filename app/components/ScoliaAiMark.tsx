"use client";

type Props = {
  size?: "sm" | "md" | "lg";
  /** Fond sombre (header fenêtre / FAB) */
  inverted?: boolean;
  /** Pluie Matrix sur toute la surface du parent (bulle ronde) */
  fill?: boolean;
  className?: string;
};

const GLYPHS = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎ012345789:;=*+<>";

function columnText(seed: number, len: number) {
  let out = "";
  for (let i = 0; i < len; i++) {
    out += GLYPHS[(seed * 19 + i * 37) % GLYPHS.length]!;
  }
  return out;
}

function MatrixRain({ cols, rainClass }: { cols: number; rainClass: string }) {
  return (
    <span className="pointer-events-none absolute inset-0 flex justify-around overflow-hidden px-0.5 opacity-70" aria-hidden>
      {Array.from({ length: cols }, (_, i) => (
        <span
          key={i}
          className={`scolia-matrix-col font-mono font-bold ${rainClass} ${
            i % 2 === 0 ? "text-emerald-300/90" : "text-emerald-500/80"
          }`}
          style={{
            animationDuration: `${1.35 + (i % 4) * 0.45}s`,
            animationDelay: `-${(i % 6) * 0.22}s`,
          }}
        >
          {columnText(3 + i * 11, 22)
            .split("")
            .map((g, gi) => (
              <span key={gi} className="block text-center">
                {g}
              </span>
            ))}
          {columnText(9 + i * 17, 22)
            .split("")
            .map((g, gi) => (
              <span key={`b${gi}`} className="block text-center">
                {g}
              </span>
            ))}
        </span>
      ))}
    </span>
  );
}

/**
 * Marque « IA » — lettres nettes + pluie Matrix en fond (pas d'encadré par lettre).
 */
export default function ScoliaAiMark({
  size = "md",
  inverted = false,
  fill = false,
  className = "",
}: Props) {
  const letter =
    size === "lg" ? "text-3xl" : size === "sm" ? "text-base" : fill ? "text-xl" : "text-lg";
  const gap = size === "lg" ? "gap-1" : "gap-0.5";
  const rain =
    size === "lg" ? "text-[7px] leading-[8px]" : size === "sm" ? "text-[4px] leading-[5px]" : "text-[5px] leading-[6px]";
  const cols = fill ? 7 : size === "lg" ? 6 : 5;
  const ink = inverted || fill ? "text-white" : "text-emerald-950";

  return (
    <span
      className={`relative inline-flex items-center justify-center overflow-hidden ${
        fill ? "h-full w-full" : size === "lg" ? "h-10 px-1" : size === "sm" ? "h-6 px-0.5" : "h-8 px-0.5"
      } ${className}`}
      aria-label="IA"
    >
      <MatrixRain cols={cols} rainClass={rain} />
      <span
        className={`relative z-[1] inline-flex items-baseline font-black tracking-tight ${gap} ${letter} ${ink}`}
        style={{
          textShadow: inverted || fill
            ? "0 0 10px rgba(52,211,153,0.55), 0 1px 2px rgba(0,0,0,0.65)"
            : "0 0 6px rgba(16,185,129,0.25)",
        }}
      >
        <span>I</span>
        <span>A</span>
      </span>
    </span>
  );
}
