import { Row, Segmented } from "../core/platform-controls";

export type InputModeScheme = "quanpin" | "shuangpin" | "wubi" | "japanese" | "korean";
export type InputModeChineseScheme = Exclude<InputModeScheme, "japanese" | "korean">;

export interface InputModeSectionProps {
  scheme: InputModeScheme;
  lastChineseScheme?: InputModeChineseScheme | null;
  onChange: (
    patch: Partial<{
      scheme: InputModeScheme;
      last_chinese_scheme: InputModeChineseScheme | null;
    }>,
  ) => void;
  /** Hosts that choose among touch keyboard schemes instead keep this row out of sight. */
  hidden?: boolean;
}

const inputModeOptions = [
  { value: "chinese", label: "中文" },
  { value: "japanese", label: "日文" },
  { value: "korean", label: "韩文" },
] as const;

/** Whether the scheme is one of the Chinese schemes Japanese and Korean return to; switching between those two keeps the one already remembered. */
function isChineseScheme(scheme: InputModeScheme): scheme is InputModeChineseScheme {
  return scheme !== "japanese" && scheme !== "korean";
}

/** Chinese/Japanese/Korean input mode selector with remembered Chinese scheme: the first row of the 方案 group. */
export function InputModeSection({
  scheme,
  lastChineseScheme,
  onChange,
  hidden,
}: InputModeSectionProps) {
  return (
    <Row
      title="输入模式"
      description="切换中文、日文或韩文输入，并保留各模式上次选择的方案"
      hidden={hidden}
    >
      <Segmented
        options={inputModeOptions}
        value={isChineseScheme(scheme) ? "chinese" : scheme}
        onChange={(mode) =>
          onChange(
            mode === "chinese"
              ? { scheme: lastChineseScheme ?? "quanpin" }
              : {
                  last_chinese_scheme: isChineseScheme(scheme) ? scheme : lastChineseScheme,
                  scheme: mode,
                },
          )
        }
      />
    </Row>
  );
}
