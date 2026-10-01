import { Children, useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import * as settings from "./settings-style";

/** The most dots the nav shows. With more themes than this - every installed skin is a slide - the dots are a window that follows the current slide, so the nav row keeps its width however many skins there are. */
export const MAX_CAROUSEL_DOTS = 9;

/** The first slide of the window of dots around `index`, keeping the window inside the slides. */
export function carouselDotWindowStart(
  index: number,
  count: number,
  size = MAX_CAROUSEL_DOTS,
): number {
  if (count <= size) return 0;
  return Math.min(Math.max(index - Math.floor(size / 2), 0), count - size);
}

export interface ThemeCarouselProps {
  /** One name per slide, in order; the dots are labelled with them. */
  labels: string[];
  /** The slide to bring into view when the page opens and whenever the selected theme changes. */
  selectedIndex: number;
  children: ReactNode;
}

/** Shows the theme cards one at a time, switched with the arrows, the dots, the arrow keys, or a horizontal swipe on the track. */
export function ThemeCarousel({ labels, selectedIndex, children }: ThemeCarouselProps) {
  const track = useRef<HTMLDivElement>(null);
  const [index, setIndex] = useState(selectedIndex);
  // The resize handler re-snaps to this without re-subscribing on every slide change.
  const current = useRef(index);
  const count = Children.count(children);
  const windowStart = carouselDotWindowStart(index, labels.length);

  const show = useCallback(
    (next: number, smooth: boolean) => {
      const clamped = Math.min(Math.max(next, 0), Math.max(count - 1, 0));
      current.current = clamped;
      setIndex(clamped);
      const element = track.current;
      element?.scrollTo?.({
        left: clamped * element.clientWidth,
        behavior: smooth ? "smooth" : "auto",
      });
    },
    [count],
  );

  useEffect(() => show(selectedIndex, false), [selectedIndex, show]);

  // The page mounts inside a hidden fieldset, where the track has no width; snapping again once it is laid out (and on every later resize) keeps the chosen card in view.
  useEffect(() => {
    const element = track.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => show(current.current, false));
    observer.observe(element);
    return () => observer.disconnect();
  }, [show]);

  function onScroll() {
    const element = track.current;
    if (!element || element.clientWidth === 0) return;
    const next = Math.round(element.scrollLeft / element.clientWidth);
    current.current = next;
    setIndex(next);
  }

  function onKeyDown(event: import("react").KeyboardEvent<HTMLElement>) {
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      show(index + (event.key === "ArrowLeft" ? -1 : 1), true);
    }
  }

  return (
    <section aria-roledescription="carousel" aria-label="主题列表" onKeyDown={onKeyDown}>
      <div ref={track} className={settings.themeCarouselTrack} onScroll={onScroll}>
        {Children.map(children, (child, slide) => (
          <div
            className={settings.themeCarouselSlide}
            aria-roledescription="slide"
            aria-label={`${slide + 1} / ${count}`}
          >
            {child}
          </div>
        ))}
      </div>
      <div className={settings.themeCarouselNav}>
        <button
          type="button"
          className={settings.themeCarouselArrow}
          aria-label="上一个主题"
          disabled={index <= 0}
          onClick={() => show(index - 1, true)}
        >
          ‹
        </button>
        <div className={settings.themeCarouselDots}>
          {labels.slice(windowStart, windowStart + MAX_CAROUSEL_DOTS).map((label, offset) => {
            const slide = windowStart + offset;
            // The end dots of a window that does not reach the first or last slide are drawn smaller, to say there are more themes past them.
            const edge =
              (offset === 0 && windowStart > 0) ||
              (offset === MAX_CAROUSEL_DOTS - 1 && windowStart + MAX_CAROUSEL_DOTS < labels.length);
            return (
              <button
                type="button"
                key={slide}
                className={settings.themeCarouselDot(slide === index, edge)}
                aria-label={`查看${label}`}
                aria-current={slide === index ? "true" : undefined}
                onClick={() => show(slide, true)}
              />
            );
          })}
        </div>
        <span className={settings.themeCarouselCount}>
          {index + 1} / {count}
        </span>
        <button
          type="button"
          className={settings.themeCarouselArrow}
          aria-label="下一个主题"
          disabled={index >= count - 1}
          onClick={() => show(index + 1, true)}
        >
          ›
        </button>
      </div>
    </section>
  );
}
