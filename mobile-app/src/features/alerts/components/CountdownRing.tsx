import { StyleSheet, Text, View } from "react-native";
import Svg, { Circle } from "react-native-svg";
import { vi } from "../../../i18n/vi";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";

/** Props of {@link CountdownRing}. */
export interface CountdownRingProps {
  /** Seconds left. */
  seconds: number;
  /** Seconds at the start, for the ring progress. */
  total: number;
  /** Diameter in dp. */
  size?: number;
}

const STROKE = 8;

/**
 * Round countdown with the seconds left in the middle.
 *
 * @param props - See {@link CountdownRingProps}.
 * @returns The ring.
 */
export function CountdownRing({ seconds, total, size = 176 }: CountdownRingProps) {
  const r = (size - STROKE) / 2;
  const circumference = 2 * Math.PI * r;
  const progress = total > 0 ? Math.max(0, Math.min(1, seconds / total)) : 0;
  return (
    <View
      style={{ width: size, height: size }}
      accessibilityRole="timer"
      accessibilityLabel={vi.emergency.countdownLabel(seconds)}
      testID="countdown"
    >
      <Svg width={size} height={size} style={StyleSheet.absoluteFill}>
        <Circle cx={size / 2} cy={size / 2} r={r} stroke="rgba(255,255,255,0.35)" strokeWidth={STROKE} fill="none" />
        <Circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          stroke={colors.paper}
          strokeWidth={STROKE}
          fill="none"
          strokeLinecap="round"
          strokeDasharray={`${circumference} ${circumference}`}
          strokeDashoffset={circumference * (1 - progress)}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </Svg>
      <View style={styles.center}>
        <Text style={styles.num}>{String(seconds)}</Text>
        <Text style={styles.unit}>{vi.emergency.seconds}</Text>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  center: { position: "absolute", top: 0, right: 0, bottom: 0, left: 0, alignItems: "center", justifyContent: "center" },
  num: { fontFamily: fonts.display, fontSize: 56, lineHeight: 62, color: colors.paper, fontVariant: ["tabular-nums"] },
  unit: { fontFamily: fonts.body, fontSize: 16, color: colors.paper },
});
