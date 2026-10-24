export type ChevronGlyphVariant = "profile";

export interface ChevronGlyphProps {
  /** 固定尺寸；调用方应按所在控件传入原有尺寸。 */
  size?: number;
  /** 线宽，默认使用设置控件的线宽。 */
  strokeWidth?: number;
  /** `profile` 保留个人资料行的窄高几何。 */
  variant?: ChevronGlyphVariant;
  className?: string;
}

/** 装饰性的右向 chevron 图标。 */
export function ChevronGlyph({ size, strokeWidth, variant, className }: ChevronGlyphProps) {
  const isProfile = variant === "profile";
  const width = isProfile ? (size ?? 7) : size;
  const height = isProfile ? (size ?? 12) : size;
  return (
    <svg
      {...(width === undefined ? {} : { width, height })}
      viewBox={isProfile ? "0 0 8 14" : "0 0 24 24"}
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth ?? (isProfile ? 2 : 2.4)}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      <path d={isProfile ? "m1 1 6 6-6 6" : "m9 6 6 6-6 6"} />
    </svg>
  );
}
