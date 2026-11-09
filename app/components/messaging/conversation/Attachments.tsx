import type { MessagingAttachmentDto } from "@/app/lib/messaging/types";
import { IconDownload } from "./ExtraIcons";
import { FILE_KIND_META, fileKindOf, formatBytes } from "./fileKinds";

/** Vignette d'image : clic = visionneuse plein écran. */
export function ImageThumb({
  attachment,
  onOpen,
}: {
  attachment: MessagingAttachmentDto;
  onOpen: (attachmentId: string) => void;
}) {
  if (!attachment.url) return null;
  return (
    <button
      type="button"
      onClick={() => onOpen(attachment.id)}
      aria-label={`Agrandir l’image ${attachment.fileName}`}
      className="block max-w-full overflow-hidden rounded-xl bg-black/5 outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/60"
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={attachment.url}
        alt={attachment.fileName}
        loading="lazy"
        className="max-h-60 min-h-[3rem] min-w-[6rem] max-w-full cursor-zoom-in object-cover transition hover:brightness-95"
        style={{ maxWidth: "min(100%, 18rem)" }}
      />
    </button>
  );
}

/** Carte de pièce jointe : icône selon le type, nom tronqué, taille, téléchargement. */
export function FileCard({
  attachment,
  isMine,
}: {
  attachment: MessagingAttachmentDto;
  isMine: boolean;
}) {
  const meta = FILE_KIND_META[fileKindOf(attachment.mime, attachment.fileName)];
  const Icon = meta.Icon;
  const href = attachment.url ?? undefined;
  return (
    <div
      className={`mb-1.5 flex w-64 max-w-full items-center gap-2.5 rounded-xl p-2 ${
        isMine ? "bg-white/12" : "bg-black/[0.04]"
      }`}
    >
      <span className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${meta.tone}`}>
        <Icon className="h-5 w-5" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13px] font-medium" title={attachment.fileName}>
          {attachment.fileName}
        </span>
        <span className={`block text-[11px] ${isMine ? "text-white/70" : "text-neutral-500"}`}>
          {meta.label} · {formatBytes(attachment.size)}
        </span>
      </span>
      {href ? (
        <a
          href={href}
          download={attachment.fileName}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Télécharger ${attachment.fileName}`}
          title="Télécharger"
          className={`shrink-0 rounded-full p-1.5 outline-none transition focus-visible:ring-2 ${
            isMine
              ? "hover:bg-white/15 focus-visible:ring-white/70"
              : "hover:bg-black/[0.07] focus-visible:ring-[var(--dash-ink)]/50"
          }`}
        >
          <IconDownload className="h-4 w-4" />
        </a>
      ) : null}
    </div>
  );
}
