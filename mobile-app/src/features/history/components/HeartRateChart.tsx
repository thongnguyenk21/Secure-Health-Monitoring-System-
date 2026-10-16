import { View } from "react-native";
import Svg, { Circle, G, Line, Polyline, Text as SvgText } from "react-native-svg";
import type { Alert } from "../../../api/types";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { downsample, hrAxis, splitOnGaps, type HrPoint, type RangeSpec } from "../ranges";

/** Props of {@link HeartRateChart}. */
export interface HeartRateChartProps {
  /** Average heart rate per bucket, oldest first. */
  points: readonly HrPoint[];
  /** Alerts to mark with red dots; those outside the window are skipped. */
  alerts: readonly Alert[];
  /** Window start, UTC epoch ms. */
  fromMs: number;
  /** Window end, UTC epoch ms. */
  toMs: number;
  /** Range being shown, for tick labels and gap size. */
  spec: RangeSpec;
  /** Accessible description of the chart. */
  label: string;
}

const W = 330;
const H = 176;
const LEFT = 34;
const RIGHT = 6;
const TOP = 10;
const BOTTOM = 22;
const TICKS = 4;

const HR_TYPES = new Set(["hr_high", "hr_low", "anomaly"]);

/**
 * Heart-rate history chart: average line plus a red dot at each alert.
 *
 * @param props - See {@link HeartRateChartProps}.
 * @returns The SVG chart.
 */
export function HeartRateChart({ points: rawPoints, alerts, fromMs, toMs, spec, label }: HeartRateChartProps) {
  const points = downsample(rawPoints, spec.displayBinMs);
  const inWindow = alerts.filter((a) => a.raised_at_unix_ms >= fromMs && a.raised_at_unix_ms <= toMs);
  const alertValues = inWindow
    .filter((a) => HR_TYPES.has(a.alert_type) && typeof a.value === "number")
    .map((a) => a.value as number);
  const axis = hrAxis([...points.map((p) => p.hr), ...alertValues]);
  const span = Math.max(toMs - fromMs, 1);
  const x = (t: number) => LEFT + ((t - fromMs) / span) * (W - LEFT - RIGHT);
  const y = (hr: number) => {
    const clamped = Math.max(axis.min, Math.min(axis.max, hr));
    return TOP + (1 - (clamped - axis.min) / (axis.max - axis.min)) * (H - TOP - BOTTOM);
  };
  const runs = splitOnGaps(points, spec.displayBinMs * 3);
  const grid = [axis.max, (axis.max + axis.min) / 2, axis.min];

  const nearestHr = (t: number): number | null => {
    let best: HrPoint | null = null;
    for (const p of points) if (!best || Math.abs(p.t - t) < Math.abs(best.t - t)) best = p;
    return best ? best.hr : null;
  };

  return (
    <View accessible accessibilityRole="image" accessibilityLabel={label} testID="hr-chart">
      <Svg width="100%" height={H} viewBox={`0 0 ${W} ${H}`}>
        <G>
          {grid.map((v) => (
            <G key={`g${v}`}>
              <Line x1={LEFT} y1={y(v)} x2={W - RIGHT} y2={y(v)} stroke={colors.line} strokeWidth={1} />
              <SvgText x={LEFT - 6} y={y(v) + 4} fontSize={11} fill={colors.muted} textAnchor="end" fontFamily={fonts.body}>
                {String(Math.round(v))}
              </SvgText>
            </G>
          ))}
        </G>
        {runs.map((run) =>
          run.length === 1 && run[0] ? (
            <Circle key={`r${run[0].t}`} cx={x(run[0].t)} cy={y(run[0].hr)} r={2} fill={colors.accent} />
          ) : (
            <Polyline
              key={`r${run[0]?.t ?? 0}`}
              points={run.map((p) => `${x(p.t).toFixed(1)},${y(p.hr).toFixed(1)}`).join(" ")}
              fill="none"
              stroke={colors.accent}
              strokeWidth={2}
              strokeLinejoin="round"
            />
          ),
        )}
        {inWindow.map((a) => {
          const hr = HR_TYPES.has(a.alert_type) && typeof a.value === "number" ? a.value : nearestHr(a.raised_at_unix_ms);
          if (hr === null) return null;
          return <Circle key={a.id} cx={x(a.raised_at_unix_ms)} cy={y(hr)} r={5} fill={colors.alert} />;
        })}
        {Array.from({ length: TICKS }, (_, i) => {
          const t = fromMs + (span * i) / (TICKS - 1);
          const anchor = i === 0 ? "start" : i === TICKS - 1 ? "end" : "middle";
          return (
            <SvgText key={`t${i}`} x={x(t)} y={H - 6} fontSize={11} fill={colors.muted} textAnchor={anchor} fontFamily={fonts.body}>
              {spec.tickLabel(t)}
            </SvgText>
          );
        })}
      </Svg>
    </View>
  );
}
