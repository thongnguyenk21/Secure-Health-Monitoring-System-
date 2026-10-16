import { useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import Svg, { Circle, Line } from "react-native-svg";
import { Caption, ErrorNote, Loading, NoDeviceCard, Screen, Tile } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { usePrimaryDevice, useUserId } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { useNow } from "../../../utils/useNow";
import { AlertRow } from "../../alerts/components/AlertRow";
import { useAlerts } from "../../alerts/queries";
import { HeartRateChart } from "../components/HeartRateChart";
import { useHistory } from "../queries";
import { RANGES, type HistoryRange } from "../ranges";

const ORDER: readonly HistoryRange[] = ["day", "week", "month"];

/**
 * History tab: heart-rate chart by day, week or month, and the alert timeline.
 *
 * @returns The screen.
 */
export function HistoryScreen() {
  const [range, setRange] = useState<HistoryRange>("week");
  const { device, isLoading: devicesLoading } = usePrimaryDevice();
  const deviceId = device?.device_id ?? null;
  const history = useHistory(deviceId, range);
  const alerts = useAlerts();
  const myUserId = useUserId();
  const now = useNow(60_000);
  const spec = RANGES[range];
  const right = <Text style={styles.metric}>{vi.history.metric}</Text>;

  if (devicesLoading) {
    return (
      <Screen title={vi.history.title} right={right}>
        <Loading />
      </Screen>
    );
  }
  if (!deviceId) {
    return (
      <Screen title={vi.history.title} right={right}>
        <NoDeviceCard />
      </Screen>
    );
  }

  const fromMs = history.data?.fromMs ?? now - spec.spanMs;
  const toMs = history.data?.toMs ?? now;
  const timeline = (alerts.data ?? [])
    .filter((a) => a.raised_at_unix_ms >= fromMs)
    .sort((a, b) => b.raised_at_unix_ms - a.raised_at_unix_ms);

  return (
    <Screen
      title={vi.history.title}
      right={right}
      onRefresh={() => {
        history.refetch();
        alerts.refetch();
      }}
      refreshing={history.isRefetching}
    >
      <View style={styles.segments} accessibilityRole="tablist">
        {ORDER.map((id) => {
          const active = id === range;
          return (
            <Pressable
              key={id}
              accessibilityRole="tab"
              accessibilityState={{ selected: active }}
              onPress={() => setRange(id)}
              style={[styles.pill, active && styles.pillActive]}
              testID={`range-${id}`}
            >
              <Text style={[styles.pillText, active && styles.pillTextActive]}>{RANGES[id].label}</Text>
            </Pressable>
          );
        })}
      </View>
      <Tile style={styles.chartTile}>
        {history.isLoading ? (
          <Loading />
        ) : history.error ? (
          <ErrorNote message={vi.common.loadError} onRetry={() => history.refetch()} />
        ) : (
          <>
            <HeartRateChart
              points={history.data?.points ?? []}
              alerts={alerts.data ?? []}
              fromMs={fromMs}
              toMs={toMs}
              spec={spec}
              label={vi.history.chartLabel(spec.label)}
            />
            {(history.data?.points.length ?? 0) === 0 ? <Text style={styles.noData}>{vi.history.noData}</Text> : null}
            <View style={styles.legend}>
              <Svg width={18} height={10}>
                <Line x1={0} y1={5} x2={18} y2={5} stroke={colors.accent} strokeWidth={2} />
              </Svg>
              <Text style={styles.legendText}>{vi.history.legendAvg}</Text>
              <Svg width={10} height={10}>
                <Circle cx={5} cy={5} r={4.5} fill={colors.alert} />
              </Svg>
              <Text style={styles.legendText}>{vi.history.legendAlert}</Text>
            </View>
          </>
        )}
      </Tile>
      <Caption style={styles.section}>{vi.history.timeline}</Caption>
      {timeline.length === 0 ? (
        <Tile>
          <Text style={styles.noData}>{vi.history.timelineEmpty}</Text>
        </Tile>
      ) : (
        timeline.map((a) => <AlertRow key={a.id} alert={a} myUserId={myUserId} nowMs={now} />)
      )}
    </Screen>
  );
}

const styles = StyleSheet.create({
  metric: { fontFamily: fonts.body, fontSize: 15, color: colors.muted },
  segments: { flexDirection: "row", gap: 8 },
  pill: {
    minHeight: 40,
    paddingHorizontal: 18,
    borderRadius: 999,
    borderWidth: 1,
    borderColor: colors.line,
    backgroundColor: colors.paper,
    justifyContent: "center",
  },
  pillActive: { backgroundColor: colors.accent, borderColor: colors.accent },
  pillText: { fontFamily: fonts.mono, fontSize: 14, color: colors.ink },
  pillTextActive: { color: colors.paper },
  chartTile: { paddingHorizontal: 8, paddingVertical: 8 },
  noData: { fontFamily: fonts.body, fontSize: 15, color: colors.muted, textAlign: "center", paddingVertical: 6 },
  legend: { flexDirection: "row", alignItems: "center", gap: 6, paddingHorizontal: 6, paddingTop: 2 },
  legendText: { fontFamily: fonts.body, fontSize: 13, color: colors.muted, marginRight: 10 },
  section: { marginTop: 8 },
});
