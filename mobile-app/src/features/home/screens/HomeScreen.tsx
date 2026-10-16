import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import { Caption, ErrorNote, Loading, NoDeviceCard, Screen, StatusChip, Tile } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { useMe, usePrimaryDevice } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { formatInteger, formatTime } from "../../../utils/format";
import { useNow } from "../../../utils/useNow";
import { topOpenAlert } from "../../alerts/alertText";
import { useAlerts } from "../../alerts/queries";
import { Sparkline } from "../components/Sparkline";
import { StatusCard } from "../components/StatusCard";
import { VitalTile } from "../components/VitalTile";
import { healthStatus, isFresh } from "../healthStatus";
import { useLatest, useRecentReadings, useStepsToday } from "../queries";

/**
 * Home tab: greeting, connection, context-aware status and live vitals.
 *
 * @returns The screen.
 */
export function HomeScreen() {
  const me = useMe();
  const { device, isLoading: devicesLoading, error: devicesError } = usePrimaryDevice();
  const deviceId = device?.device_id ?? null;
  const latest = useLatest(deviceId);
  const recent = useRecentReadings(deviceId);
  const steps = useStepsToday(deviceId);
  const alerts = useAlerts();
  const now = useNow(5_000);
  const client = useQueryClient();
  const [refreshing, setRefreshing] = useState(false);

  const onRefresh = useCallback(() => {
    setRefreshing(true);
    client.invalidateQueries().finally(() => setRefreshing(false));
  }, [client]);

  const name = me.data?.display_name;
  const title = name ? vi.home.greeting(name) : vi.home.greetingNoName;
  const reading = latest.data ?? null;
  const fresh = isFresh(reading, now);
  const right = deviceId ? (
    <StatusChip
      color={fresh ? colors.ok : colors.alert}
      label={fresh ? vi.home.connected : vi.home.disconnected}
      labelColor={fresh ? colors.ink : colors.alert}
    />
  ) : null;

  if (devicesLoading) {
    return (
      <Screen title={title}>
        <Loading />
      </Screen>
    );
  }
  if (devicesError) {
    return (
      <Screen title={title} onRefresh={onRefresh} refreshing={refreshing}>
        <ErrorNote message={vi.common.loadError} onRetry={onRefresh} />
      </Screen>
    );
  }
  if (!deviceId) {
    return (
      <Screen title={title} onRefresh={onRefresh} refreshing={refreshing}>
        <NoDeviceCard />
      </Screen>
    );
  }

  const status = healthStatus({ openAlert: topOpenAlert(alerts.data ?? []), latest: reading, nowMs: now });
  const hr = reading?.hr_bpm ?? null;
  const spo2 = reading?.spo2_pct ?? null;
  const sparkValues = (recent.data ?? []).map((m) => m.hr_bpm);

  return (
    <Screen title={title} right={right} onRefresh={onRefresh} refreshing={refreshing}>
      <StatusCard status={status} />
      {latest.error ? <ErrorNote message={vi.common.offlineError} onRetry={onRefresh} /> : null}
      <Tile style={styles.hrTile}>
        <View style={styles.hrHead}>
          <Caption>{vi.home.heartRate}</Caption>
          <Text style={styles.small}>{vi.home.last60s}</Text>
        </View>
        <View style={styles.hrRow}>
          <Text style={styles.hrNum} testID="hr-value">
            {hr === null ? vi.common.dash : String(hr)}
          </Text>
          <Text style={styles.hrUnit}>{` ${vi.home.bpm}`}</Text>
        </View>
        <Sparkline values={sparkValues} />
      </Tile>
      <View style={styles.grid}>
        <VitalTile caption={vi.home.spo2} value={spo2 === null ? vi.common.dash : String(spo2)} unit={vi.home.percent} testID="spo2-value" />
        <VitalTile caption={vi.home.activity} value={reading ? vi.activity[reading.activity] : vi.common.dash} word />
      </View>
      <View style={styles.grid}>
        <VitalTile caption={vi.home.speed} value={vi.common.dash} unit={vi.home.kmh} />
        <VitalTile caption={vi.home.steps} value={steps.data === undefined ? vi.common.dash : formatInteger(steps.data)} testID="steps-value" />
      </View>
      <Text style={styles.footer}>
        {reading
          ? fresh
            ? vi.home.doctorHasLatest
            : vi.home.lastReadingAt(formatTime(reading.ts_unix_ms))
          : vi.home.noReadingYet}
      </Text>
    </Screen>
  );
}

const styles = StyleSheet.create({
  hrTile: { paddingBottom: 6 },
  hrHead: { flexDirection: "row", justifyContent: "space-between", alignItems: "baseline" },
  small: { fontFamily: fonts.body, fontSize: 13.5, color: colors.muted },
  hrRow: { flexDirection: "row", alignItems: "baseline", marginTop: 2 },
  hrNum: { fontFamily: fonts.display, fontSize: 54, lineHeight: 64, color: colors.ink, fontVariant: ["tabular-nums"] },
  hrUnit: { fontFamily: fonts.body, fontSize: 17, color: colors.muted },
  grid: { flexDirection: "row", gap: 10 },
  footer: { fontFamily: fonts.body, fontSize: 14.5, color: colors.muted, textAlign: "center", marginTop: 4 },
});
