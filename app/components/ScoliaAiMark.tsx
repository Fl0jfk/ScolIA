"use client";

type Props = {
  size?: "sm" | "md" | "lg";
  /** Fond sombre (header fenêtre / FAB) */
  inverted?: boolean;
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

/**
 * Marque « IA » — contour lisible + pluie Matrix dans les lettres.
 */
export default function ScoliaAiMark({ size = "md", inverted = false, className = "" }: Props) {
  const h = size === "lg" ? "h-9" : size === "sm" ? "h-5" : "h-7";
  const gap = size === "lg" ? "gap-1.5" : "gap-1";
  const iW = size === "lg" ? "w-5" : size === "sm" ? "w-2.5" : "w-3.5";
  const aW = size === "lg" ? "w-8" : size === "sm" ? "w-4" : "w-[1.35rem]";
  const label =
    size === "lg" ? "text-xl" : size === "sm" ? "text-[11px]" : "text-sm";
  const rain =
    size === "lg" ? "text-[8px] leading-[9px]" : size === "sm" ? "text-[4px] leading-[5px]" : "text-[6px] leading-[7px]";

  return (
    <span className={`inline-flex items-end ${h} ${gap} ${className}`} aria-label="IA">
      <Letter
        char="I"
        widthClass={iW}
        labelClass={label}
        rainClass={rain}
        inverted={inverted}
        cols={2}
        seed={2}
      />
      <Letter
        char="A"
        widthClass={aW}
        labelClass={label}
        rainClass={rain}
        inverted={inverted}
        cols={4}
        seed={9}
        clipA
      />
    </span>
  );
}

function Letter({
  char,
  widthClass,
  labelClass,
  rainClass,
  inverted,
  cols,
  seed,
  clipA,
}: {
  char: "I" | "A";
  widthClass: string;
  labelClass: string;
  rainClass: string;
  inverted: boolean;
  cols: number;
  seed: number;
  clipA?: boolean;
}) {
  return (
    <span
      className={`relative inline-flex h-full overflow-hidden rounded-[2px] ${widthClass} ${
        inverted ? "bg-black/55 ring-1 ring-emerald-400/40" : "bg-slate-950 ring-1 ring-emerald-600/30"
      }`}
      style={
        clipA
          ? { clipPath: "polygon(50% 0%, 100% 100%, 76% 100%, 66% 70%, 34% 70%, 24% 100%, 0% 100%)" }
          : undefined
      }
      aria-hidden
    >
      <span className="absolute inset-0 flex justify-around px-[1px] overflow-hidden">
        {Array.from({ length: cols }, (_, i) => (
          <span
            key={i}
            className={`scolia-matrix-col font-mono font-bold ${rainClass} ${
              i % 2 === 0 ? "text-emerald-300" : "text-emerald-500"
            }`}
            style={{
              animationDuration: `${1.4 + (i % 3) * 0.55}s`,
              animationDelay: `-${((seed + i) % 5) * 0.2}s`,
            }}
          >
            {columnText(seed + i * 11, 18)
              .split("")
              .map((g, gi) => (
                <span key={gi} className="block text-center opacity-90">
                  {g}
                </span>
              ))}
            {columnText(seed + i * 17 + 3, 18)
              .split("")
              .map((g, gi) => (
                <span key={`b${gi}`} className="block text-center opacity-90">
                  {g}
                </span>
              ))}
          </span>
        ))}
      </span>
      <span
        className={`relative z-[1] flex h-full w-full items-center justify-center font-black tracking-tighter ${labelClass} text-white`}
        style={{ textShadow: "0 0 8px rgba(52,211,153,0.75), 0 1px 2px rgba(0,0,0,0.8)" }}
      >
        {char}
      </span>
    </span>
  );
}
