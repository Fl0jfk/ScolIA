"use client";

export default function MoveDestBreadcrumb({
  rootLabel,
  path,
  onNavigate,
}: {
  rootLabel: string;
  path: string;
  onNavigate: (path: string) => void;
}) {
  const segments = path.replace(/\/$/, "").split("/").filter(Boolean);
  return (
    <div className="flex items-center gap-1 text-sm text-gray-500 mb-3 flex-wrap">
      <button type="button" onClick={() => onNavigate("")} className="hover:text-blue-600 font-medium">
        {rootLabel}
      </button>
      {segments.map((seg, i) => (
        <span key={`${seg}-${i}`} className="flex items-center gap-1">
          <span>/</span>
          <button
            type="button"
            onClick={() => onNavigate(`${segments.slice(0, i + 1).join("/")}/`)}
            className="hover:text-blue-600 font-medium"
          >
            {seg}
          </button>
        </span>
      ))}
    </div>
  );
}
