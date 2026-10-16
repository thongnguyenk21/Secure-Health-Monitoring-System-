import { View } from "react-native";
import Svg, { Polyline } from "react-native-svg";
import { colors } from "../../../theme/colors";

/** Props of {@link Sparkline}. */
export interface SparklineProps {
  /** Values in time order; nulls are skipped. */
  values: readonly (number | null)[];
  /** Drawing height in dp. */
  height?: number;
}

const VIEW_W = 200;
const VIEW_H = 40;
const PAD = 4;

/**
 * Map values to SVG polyline points in a 200 × 40 box.
 *
 * @param values - Values in time order; nulls are skipped.
 * @returns The `points` attribute, or an empty string with fewer than two values.
 */
export function sparklinePoints(values: readonly (number | null)[]): string {
  const present = values.map((v, i) => ({ v, i })).filter((p): p is { v: number; i: number } => p.v !== null);
  if (present.length < 2) return "";
  const vs = present.map((p) => p.v);
  const lo = Math.min(...vs);
  const hi = Math.max(...vs);
  const span = Math.max(hi - lo, 10);
  const mid = (hi + lo) / 2;
  const last = Math.max(values.length - 1, 1);
  return present
    .map((p) => {
      const x = (p.i / last) * VIEW_W;
      const y = VIEW_H / 2 - ((p.v - mid) / span) * (VIEW_H - 2 * PAD);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

/**
 * Small line chart of the last minute of heart rate.
 *
 * @param props - See {@link SparklineProps}.
 * @returns The chart, or an empty box when there are too few values.
 */
export function Sparkline({ values, height = 46 }: SparklineProps) {
  const points = sparklinePoints(values);
  return (
    <View aria-hidden>
      <Svg width="100%" height={height} viewBox={`0 0 ${VIEW_W} ${VIEW_H}`} preserveAspectRatio="none">
        {points ? (
          <Polyline points={points} fill="none" stroke={colors.accent} strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" />
        ) : null}
      </Svg>
    </View>
  );
}
