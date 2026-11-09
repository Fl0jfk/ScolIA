export default function Avatar({
  name,
  imageUrl,
  size = 28,
}: {
  name: string;
  imageUrl?: string | null;
  size?: number;
}) {
  return (
    <span
      className="flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--dash-ink)] text-[11px] font-semibold text-white"
      style={{ width: size, height: size }}
      aria-hidden
    >
      {imageUrl ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={imageUrl} alt="" className="h-full w-full object-cover" />
      ) : (
        (name || "?").slice(0, 1).toUpperCase()
      )}
    </span>
  );
}
