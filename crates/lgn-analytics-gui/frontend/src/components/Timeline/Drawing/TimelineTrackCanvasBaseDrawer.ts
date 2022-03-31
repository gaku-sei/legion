import binarySearch from "binary-search";
import { get } from "svelte/store";

import type { SpanTrack } from "@lgn/proto-telemetry/dist/span";

import { spanPixelHeight } from "@/components/Timeline/Values/TimelineValues";
import type { TimelineCaptionItem } from "@/lib/Timeline/TimelineSpanCaptionItem";
import type { TimelineStateStore } from "@/lib/Timeline/TimelineStateStore";
import { formatExecutionTime } from "@/lib/format";

import type { TimelineTrackContext } from "./TimelineTrackContext";

export abstract class TimelineTrackCanvasBaseDrawer {
  protected canvas: HTMLCanvasElement | undefined;
  protected ctx: CanvasRenderingContext2D | undefined;
  protected stateStore: TimelineStateStore;
  protected processOffsetMs: number;

  constructor(stateStore: TimelineStateStore, processOffsetMs: number) {
    this.stateStore = stateStore;
    this.processOffsetMs = processOffsetMs;
  }

  protected abstract canDraw(): boolean;

  protected abstract drawImpl(ctx: TimelineTrackContext): void;

  protected abstract getPixelRange(ctx: TimelineTrackContext): [number, number];

  initialize(canvas: HTMLCanvasElement, ctx: CanvasRenderingContext2D) {
    this.canvas = canvas;
    this.ctx = ctx;
  }

  draw(search: string) {
    if (!this.canvas || !this.ctx) {
      return;
    }

    const [begin, end] = get(this.stateStore).getViewRange();
    const invTimeSpan = 1.0 / (end - begin);
    const canvasWidth = this.canvas.clientWidth;
    const canvasHeight = this.canvas.clientHeight;
    const msToPixelsFactor = invTimeSpan * canvasWidth;

    this.ctx.fillStyle = "#F0F0F0";
    this.ctx.fillRect(0, 0, canvasWidth, canvasHeight);

    if (!this.canDraw()) {
      return;
    }

    const context = { begin, end, msToPixelsFactor, search };
    const pixelRange = this.getPixelRange(context);

    this.ctx.fillStyle = "#e8e8e8";
    this.ctx.fillRect(
      pixelRange[0],
      0,
      pixelRange[1] - pixelRange[0],
      canvasHeight
    );

    this.ctx.font = "15px arial";
    this.drawImpl(context);
  }

  protected drawSpanTrack(
    trackIndex: number,
    track: SpanTrack,
    timelineTrackContext: TimelineTrackContext
  ) {
    if (!this.ctx) {
      return;
    }

    const processOffsetMs = this.processOffsetMs;
    const beginViewRange = timelineTrackContext.begin;
    const endViewRange = timelineTrackContext.end;
    const msToPixelsFactor = timelineTrackContext.msToPixelsFactor;
    const search = timelineTrackContext.search;

    let firstSpan = binarySearch(
      track.spans,
      beginViewRange - processOffsetMs,
      function (span, needle) {
        if (span.endMs < needle) {
          return -1;
        }
        if (span.beginMs > needle) {
          return 1;
        }
        return 0;
      }
    );
    if (firstSpan < 0) {
      firstSpan = ~firstSpan;
    }

    let lastSpan = binarySearch(
      track.spans,
      endViewRange - processOffsetMs,
      function (span, needle) {
        if (span.beginMs < needle) {
          return -1;
        }
        if (span.endMs > needle) {
          return 1;
        }
        return 0;
      }
    );
    if (lastSpan < 0) {
      lastSpan = ~lastSpan;
    }

    const ctx = this.ctx;
    const testString = "<>_w";
    const testTextMetrics = ctx.measureText(testString);
    const characterWidth = testTextMetrics.width / testString.length;
    const characterHeight = testTextMetrics.actualBoundingBoxAscent;
    const offsetY = trackIndex * spanPixelHeight;
    const color = this.getIndexColor(trackIndex);

    for (let spanIndex = firstSpan; spanIndex < lastSpan; spanIndex += 1) {
      const span = track.spans[spanIndex];
      const beginSpan = span.beginMs + processOffsetMs;
      const endSpan = span.endMs + processOffsetMs;

      const beginPixels = (beginSpan - beginViewRange) * msToPixelsFactor;
      const endPixels = (endSpan - beginViewRange) * msToPixelsFactor;
      const callWidth = endPixels - beginPixels;
      if (callWidth < 0.1) {
        continue;
      }
      ctx.globalAlpha = span.alpha / 255;

      const scopes = get(this.stateStore).scopes;

      if (span.scopeHash !== 0) {
        const { name } = scopes[span.scopeHash];
        ctx.fillStyle =
          search && name.toLowerCase().includes(search.toLowerCase())
            ? "#ffee59"
            : color;
        ctx.fillRect(beginPixels, offsetY, callWidth, spanPixelHeight);
        this.drawSpanLeftMarker(ctx.fillStyle, callWidth, beginPixels, offsetY);
        if (callWidth > characterWidth * 5) {
          ctx.fillStyle = "#000000";
          const extraHeight = 0.5 * (spanPixelHeight - characterHeight);
          this.writeText(
            ctx,
            callWidth,
            characterWidth,
            Array.from(this.getCaptions(name, beginSpan, endSpan)),
            beginPixels + 5,
            offsetY + characterHeight + extraHeight
          );
        }
      } else {
        ctx.fillStyle = color;
        ctx.fillRect(beginPixels, offsetY, callWidth, spanPixelHeight);
      }
      ctx.globalAlpha = 1.0;
    }
  }

  private drawSpanLeftMarker(
    color: string,
    callWidth: number,
    beginPixels: number,
    offsetY: number
  ) {
    if (callWidth >= 8 && this.ctx) {
      const ctx = this.ctx;
      ctx.save();
      ctx.fillStyle = this.shadeColor(color, 1.04);
      ctx.fillRect(beginPixels, offsetY, 1, spanPixelHeight);
      ctx.restore();
    }
  }

  private writeText(
    ctx: CanvasRenderingContext2D,
    width: number,
    characterWidth: number,
    items: TimelineCaptionItem[],
    x: number,
    y: number
  ) {
    const defaultFillStyle = ctx.fillStyle;
    const defaultFont = ctx.font;
    ctx.save();
    for (const { value, font, color, skippable } of items) {
      ctx.fillStyle = color || defaultFillStyle;
      ctx.font = font || defaultFont;
      const budget = Math.floor(width / characterWidth);
      if (!budget) {
        break;
      }
      if (value.length > budget && skippable) {
        continue;
      }
      const textSlice = value.slice(0, budget);
      ctx.fillText(textSlice, x, y);
      const size = ctx.measureText(textSlice).width;
      x += size;
      width -= size;
    }
    ctx.restore();
  }

  private *getCaptions(
    caption: string,
    beginSpan: number,
    endSpan: number
  ): Generator<TimelineCaptionItem> {
    const mainColor = "#000000";
    const subColor = "#4d4d4d";
    const defaultFont = "15px arial";
    const split = caption.split("::");
    if (split.length > 1) {
      const first = split.shift();
      yield { value: first ?? "", font: defaultFont, color: subColor };
      let current = null;
      while ((current = split.shift())) {
        yield {
          value: `::${current}`,
          font: defaultFont,
          color: split.length > 0 ? subColor : mainColor,
        };
      }
    } else {
      yield { value: caption, color: mainColor };
    }
    yield {
      value: `  (${formatExecutionTime(endSpan - beginSpan)})`,
      color: subColor,
      font: "12px arial",
      skippable: true,
    };
  }

  private getIndexColor(trackIndex: number) {
    return trackIndex % 2 === 0 ? "#fea446" : "#fede99";
  }

  private shadeColor(color: string, decimal: number): string {
    const base = color.startsWith("#") ? 1 : 0;

    let r = parseInt(color.substring(base, 3), 16);
    let g = parseInt(color.substring(base + 2, 5), 16);
    let b = parseInt(color.substring(base + 4, 7), 16);

    r = Math.round(r / decimal);
    g = Math.round(g / decimal);
    b = Math.round(b / decimal);

    r = r < 255 ? r : 255;
    g = g < 255 ? g : 255;
    b = b < 255 ? b : 255;

    const rr =
      r.toString(16).length === 1 ? `0${r.toString(16)}` : r.toString(16);
    const gg =
      g.toString(16).length === 1 ? `0${g.toString(16)}` : g.toString(16);
    const bb =
      b.toString(16).length === 1 ? `0${b.toString(16)}` : b.toString(16);

    return `#${rr}${gg}${bb}`;
  }
}