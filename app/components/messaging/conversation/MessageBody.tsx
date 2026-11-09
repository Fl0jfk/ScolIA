import { Fragment, type ReactNode } from "react";
import { findMatches, splitLinks } from "./textMatch";
import { splitMentions } from "./utils";

type Props = {
  body: string;
  mentionNames: string[];
  isMine: boolean;
  /** Recherche dans la conversation : surligne les occurrences. */
  query?: string;
};

function highlight(text: string, query: string | undefined, keyBase: string): ReactNode {
  if (!query) return text;
  const ranges = findMatches(text, query);
  if (ranges.length === 0) return text;
  const out: ReactNode[] = [];
  let last = 0;
  ranges.forEach(([s, e], i) => {
    if (s > last) out.push(text.slice(last, s));
    out.push(
      <mark key={`${keyBase}-${i}`} className="rounded bg-amber-200 px-0.5 text-[var(--dash-ink)]">
        {text.slice(s, e)}
      </mark>,
    );
    last = e;
  });
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** Texte d'un message : @mentions, liens cliquables (http/https), surlignage de recherche. */
export default function MessageBody({ body, mentionNames, isMine, query }: Props) {
  const segments = splitMentions(body, mentionNames);
  return (
    <p className="whitespace-pre-wrap break-words leading-relaxed">
      {segments.map((seg, i) => {
        if (seg.mention) {
          return (
            <span
              key={i}
              className={
                isMine
                  ? "font-semibold text-[var(--dash-lime)]"
                  : "rounded bg-[var(--dash-lime)]/50 px-0.5 font-semibold"
              }
            >
              {highlight(seg.text, query, `m${i}`)}
            </span>
          );
        }
        return (
          <Fragment key={i}>
            {splitLinks(seg.text).map((piece, j) =>
              piece.kind === "link" ? (
                <a
                  key={j}
                  href={piece.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={`break-all underline decoration-1 underline-offset-2 outline-none focus-visible:ring-2 ${
                    isMine
                      ? "text-white decoration-white/60 hover:decoration-white focus-visible:ring-white/70"
                      : "text-sky-700 decoration-sky-700/40 hover:decoration-sky-700 focus-visible:ring-sky-500/60"
                  }`}
                >
                  {highlight(piece.text, query, `l${i}-${j}`)}
                </a>
              ) : (
                <Fragment key={j}>{highlight(piece.text, query, `t${i}-${j}`)}</Fragment>
              ),
            )}
          </Fragment>
        );
      })}
    </p>
  );
}
