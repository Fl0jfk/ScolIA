"use client";

import { useCallback, useMemo, useState } from "react";

const FONT_FAMILY = '"Inter", sans-serif';

const DEFAULT_TEXT = "Votre texte ici";
const DEFAULT_SIZE = 48;
const PADDING = 24;

type WordSegment = { kind: "word"; text: string; color: string; wordIndex: number };
type SpaceSegment = { kind: "space"; text: string };
type Segment = WordSegment | SpaceSegment;

type ParsedLine = { segments: Segment[]; nextWordIndex: number };

async function waitForFont(weight: number, sizePx: number) {
  const spec = `${weight} ${sizePx}px Inter`;
  await document.fonts.load(spec);
  await document.fonts.ready;
}

function countWords(text: string): number {
  return text.match(/\S+/g)?.length ?? 0;
}

function syncWordColors(text: string, prev: string[], defaultColor: string): string[] {
  const n = countWords(text);
  const next = [...prev];
  while (next.length < n) next.push(defaultColor);
  return next.slice(0, n);
}

function parseLine(
  line: string,
  wordColors: string[],
  defaultColor: string,
  startWordIndex: number,
): ParsedLine {
  const parts = line.split(/(\s+)/);
  let wordIndex = startWordIndex;
  const segments: Segment[] = [];

  for (const part of parts) {
    if (!part) continue;
    if (/^\s+$/.test(part)) {
      segments.push({ kind: "space", text: part });
    } else {
      segments.push({
        kind: "word",
        text: part,
        color: wordColors[wordIndex] ?? defaultColor,
        wordIndex,
      });
      wordIndex += 1;
    }
  }

  return { segments, nextWordIndex: wordIndex };
}

function parseText(text: string, wordColors: string[], defaultColor: string): Segment[][] {
  let wordIndex = 0;
  return text.split("\n").map((line) => {
    const parsed = parseLine(line, wordColors, defaultColor, wordIndex);
    wordIndex = parsed.nextWordIndex;
    return parsed.segments;
  });
}

function measureLineWidth(ctx: CanvasRenderingContext2D, segments: Segment[]): number {
  return segments.reduce((w, seg) => w + ctx.measureText(seg.text).width, 0);
}

function extractWords(text: string): string[] {
  return text.match(/\S+/g) ?? [];
}

export default function BrouillonClient() {
  const [text, setText] = useState(DEFAULT_TEXT);
  const [fontSize, setFontSize] = useState(DEFAULT_SIZE);
  const [fontWeight, setFontWeight] = useState<400 | 600 | 700>(600);
  const [defaultColor, setDefaultColor] = useState("#1c1917");
  const [perWordColor, setPerWordColor] = useState(false);
  const [wordColors, setWordColors] = useState<string[]>([defaultColor]);
  const [exporting, setExporting] = useState(false);

  const words = useMemo(() => extractWords(text), [text]);
  const lines = useMemo(
    () => parseText(text, perWordColor ? wordColors : [], defaultColor),
    [text, wordColors, defaultColor, perWordColor],
  );

  const handleTextChange = (value: string) => {
    setText(value);
    setWordColors((prev) => syncWordColors(value, prev, defaultColor));
  };

  const setWordColor = (index: number, color: string) => {
    setWordColors((prev) => {
      const next = [...prev];
      next[index] = color;
      return next;
    });
  };

  const exportPng = useCallback(async () => {
    const trimmed = text.trim();
    if (!trimmed) return;

    setExporting(true);
    try {
      await waitForFont(fontWeight, fontSize);

      const parsedLines = parseText(
        text,
        perWordColor ? wordColors : [],
        defaultColor,
      );
      const lineHeight = fontSize * 1.25;
      const font = `${fontWeight} ${fontSize}px ${FONT_FAMILY}`;

      const measureCanvas = document.createElement("canvas");
      const measureCtx = measureCanvas.getContext("2d");
      if (!measureCtx) return;
      measureCtx.font = font;

      const lineWidths = parsedLines.map((segs) => measureLineWidth(measureCtx, segs));
      const textWidth = Math.max(...lineWidths, 0);
      const textHeight = parsedLines.length * lineHeight;

      const canvas = document.createElement("canvas");
      canvas.width = Math.ceil(textWidth + PADDING * 2);
      canvas.height = Math.ceil(textHeight + PADDING * 2);

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.font = font;
      ctx.textBaseline = "top";

      parsedLines.forEach((segments, lineIndex) => {
        const lineWidth = lineWidths[lineIndex] ?? 0;
        let x = PADDING + (textWidth - lineWidth) / 2;
        const y = PADDING + lineIndex * lineHeight;

        for (const seg of segments) {
          ctx.fillStyle = seg.kind === "word" ? seg.color : defaultColor;
          ctx.fillText(seg.text, x, y);
          x += ctx.measureText(seg.text).width;
        }
      });

      const link = document.createElement("a");
      link.download = "texte-parkinsans.png";
      link.href = canvas.toDataURL("image/png");
      link.click();
    } finally {
      setExporting(false);
    }
  }, [text, fontSize, fontWeight, defaultColor, perWordColor, wordColors]);

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-8 bg-stone-100 px-6 py-12">
      <div className="w-full max-w-xl space-y-5 rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-lg font-semibold text-stone-800">Texte → image PNG</h1>
        <p className="text-xs text-stone-500">Police : Inter</p>

        <label className="block space-y-1.5">
          <span className="text-sm font-medium text-stone-600">Texte</span>
          <textarea
            value={text}
            onChange={(e) => handleTextChange(e.target.value)}
            rows={3}
            className="w-full resize-y rounded-lg border border-stone-300 px-3 py-2 text-base text-stone-800 outline-none focus:border-stone-500"
            placeholder="Saisissez votre texte…"
          />
        </label>

        <div className="flex flex-wrap gap-4">
          <label className="space-y-1.5">
            <span className="text-sm font-medium text-stone-600">Taille (px)</span>
            <input
              type="number"
              min={12}
              max={200}
              value={fontSize}
              onChange={(e) => setFontSize(Number(e.target.value) || DEFAULT_SIZE)}
              className="w-24 rounded-lg border border-stone-300 px-3 py-2 text-sm outline-none focus:border-stone-500"
            />
          </label>

          <label className="space-y-1.5">
            <span className="text-sm font-medium text-stone-600">Graisse</span>
            <select
              value={fontWeight}
              onChange={(e) => setFontWeight(Number(e.target.value) as 400 | 600 | 700)}
              className="rounded-lg border border-stone-300 px-3 py-2 text-sm outline-none focus:border-stone-500"
            >
              <option value={400}>Regular (400)</option>
              <option value={600}>Semi-bold (600)</option>
              <option value={700}>Bold (700)</option>
            </select>
          </label>

          <label className="space-y-1.5">
            <span className="text-sm font-medium text-stone-600">
              {perWordColor ? "Couleur par défaut" : "Couleur du texte"}
            </span>
            <input
              type="color"
              value={defaultColor}
              onChange={(e) => setDefaultColor(e.target.value)}
              className="block h-10 w-14 cursor-pointer rounded border border-stone-300"
            />
          </label>
        </div>

        <label className="flex cursor-pointer items-center gap-2 text-sm text-stone-700">
          <input
            type="checkbox"
            checked={perWordColor}
            onChange={(e) => setPerWordColor(e.target.checked)}
            className="h-4 w-4 rounded border-stone-300"
          />
          Une couleur différente par mot
        </label>

        {perWordColor && words.length > 0 ? (
          <div className="space-y-2 rounded-lg border border-stone-200 bg-stone-50 p-3">
            <p className="text-xs font-medium text-stone-500">Couleur de chaque mot</p>
            <div className="flex flex-wrap gap-2">
              {words.map((word, i) => (
                <label
                  key={`${i}-${word}`}
                  className="flex items-center gap-1.5 rounded-full border border-stone-200 bg-white py-1 pl-2.5 pr-1.5 text-sm shadow-sm"
                >
                  <span className="max-w-[8rem] truncate font-medium text-stone-700">{word}</span>
                  <input
                    type="color"
                    value={wordColors[i] ?? defaultColor}
                    onChange={(e) => setWordColor(i, e.target.value)}
                    className="h-7 w-9 cursor-pointer rounded border-0 bg-transparent p-0"
                    title={`Couleur du mot « ${word} »`}
                  />
                </label>
              ))}
            </div>
          </div>
        ) : null}

        <button
          type="button"
          onClick={() => void exportPng()}
          disabled={exporting || !text.trim()}
          className="w-full rounded-lg bg-stone-800 px-4 py-2.5 text-sm font-semibold text-white transition hover:bg-stone-700 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {exporting ? "Génération…" : "Télécharger en PNG (fond transparent)"}
        </button>
      </div>

      <div className="space-y-2 text-center">
        <p className="text-xs font-medium uppercase tracking-wide text-stone-500">Aperçu</p>
        <div
          className="inline-block rounded-lg border border-stone-300 p-6"
          style={{
            backgroundImage:
              "linear-gradient(45deg, #e7e5e4 25%, transparent 25%), linear-gradient(-45deg, #e7e5e4 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #e7e5e4 75%), linear-gradient(-45deg, transparent 75%, #e7e5e4 75%)",
            backgroundSize: "16px 16px",
            backgroundPosition: "0 0, 0 8px, 8px -8px, -8px 0",
            backgroundColor: "#fafaf9",
          }}
        >
          <div
            className="text-center"
            style={{
              fontFamily: FONT_FAMILY,
              fontSize: `${fontSize}px`,
              fontWeight,
              lineHeight: 1.25,
            }}
          >
            {lines.length === 0 || (lines.length === 1 && lines[0].length === 0) ? (
              <span className="text-stone-400">…</span>
            ) : (
              lines.map((segments, lineIndex) => (
                <div key={lineIndex} className="whitespace-pre-wrap">
                  {segments.length === 0 ? (
                    <br />
                  ) : (
                    segments.map((seg, segIndex) =>
                      seg.kind === "word" ? (
                        <span key={segIndex} style={{ color: seg.color }}>
                          {seg.text}
                        </span>
                      ) : (
                        <span key={segIndex}>{seg.text}</span>
                      ),
                    )
                  )}
                </div>
              ))
            )}
          </div>
        </div>
        <p className="text-xs text-stone-400">Damier = transparence à l&apos;export</p>
      </div>
    </main>
  );
}
