import type { CSSProperties } from "react";

/** Fond verre : le flou est un frère, pas un parent — sinon le curseur pointer ne s’applique qu’à la bordure. */
export default function GlassLayer({
  className,
  style,
}: {
  className: string;
  style?: CSSProperties;
}) {
  return (
    <div
      aria-hidden
      className={`pointer-events-none absolute inset-0 z-0 rounded-[inherit] ${className}`}
      style={style}
    />
  );
}
