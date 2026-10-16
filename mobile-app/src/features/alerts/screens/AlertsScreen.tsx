import { useRouter } from "expo-router";
import { StyleSheet, Text, View } from "react-native";
import { Caption, ErrorNote, Loading, Screen, Tile } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { usePrimaryDevice, useUserId } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { useNow } from "../../../utils/useNow";
import { sortAlertsOpenFirst } from "../alertText";
import { AlertRow } from "../components/AlertRow";
import { useAlerts } from "../queries";

/**
 * Alerts tab: open alerts first, each one opens the full-screen alert.
 *
 * @returns The screen.
 */
export function AlertsScreen() {
  const router = useRouter();
  const alerts = useAlerts();
  const { device } = usePrimaryDevice();
  const myUserId = useUserId();
  const now = useNow(30_000);
  const sorted = sortAlertsOpenFirst(alerts.data ?? []);
  const open = sorted.filter((a) => a.status === "open");
  const past = sorted.filter((a) => a.status !== "open");

  const body = () => {
    if (alerts.isLoading) return <Loading />;
    if (alerts.error) return <ErrorNote message={vi.common.loadError} onRetry={() => alerts.refetch()} />;
    if (sorted.length === 0) {
      return (
        <Tile>
          <Text style={styles.empty}>{device ? vi.alerts.empty : vi.alerts.noDevice}</Text>
        </Tile>
      );
    }
    return (
      <>
        {open.length > 0 ? <Caption style={styles.section}>{vi.alerts.openSection}</Caption> : null}
        {open.map((a) => (
          <AlertRow
            key={a.id}
            alert={a}
            myUserId={myUserId}
            nowMs={now}
            onOpen={() => router.push({ pathname: "/emergency/[alertId]", params: { alertId: a.id } })}
          />
        ))}
        {past.length > 0 ? <Caption style={styles.section}>{vi.alerts.pastSection}</Caption> : null}
        {past.map((a) => (
          <AlertRow key={a.id} alert={a} myUserId={myUserId} nowMs={now} />
        ))}
      </>
    );
  };

  return (
    <Screen title={vi.alerts.title} onRefresh={() => alerts.refetch()} refreshing={alerts.isRefetching}>
      <View style={styles.list}>{body()}</View>
    </Screen>
  );
}

const styles = StyleSheet.create({
  list: { gap: 8 },
  section: { marginTop: 6, marginBottom: 2 },
  empty: { fontFamily: fonts.body, fontSize: 16, color: colors.muted, textAlign: "center", paddingVertical: 12 },
});
