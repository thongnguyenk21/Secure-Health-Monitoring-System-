import { useLocalSearchParams, useRouter } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { useCallback, useEffect, useRef, useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { useNotice } from "../../../components/Notice";
import { Button, Loading } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { usePrimaryDevice } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { FRESH_MS } from "../../home/healthStatus";
import { useLatest } from "../../home/queries";
import { alertContextLine, alertTitle } from "../alertText";
import { CountdownRing } from "../components/CountdownRing";
import { EMERGENCY_COUNTDOWN_S, markEmergencyShown } from "../emergency";
import { useAckAlert, useAlerts } from "../queries";

/**
 * Full-screen alert with a 25 s countdown ("Cảnh báo khẩn" in the wireframe).
 *
 * @returns The screen.
 * @remarks
 * "Tôi ổn" acknowledges the alert, which stops the server-side escalation.
 * "Cần giúp ngay" and the end of the countdown only close the screen with a
 * confirmation: the backend escalates every unacknowledged alert on its own,
 * so the app needs no extra call (M1).
 */
export function EmergencyScreen() {
  const { alertId } = useLocalSearchParams<{ alertId: string }>();
  const router = useRouter();
  const notify = useNotice();
  const alerts = useAlerts();
  const { device } = usePrimaryDevice();
  const latest = useLatest(device?.device_id ?? null);
  const ack = useAckAlert();
  const [seconds, setSeconds] = useState(EMERGENCY_COUNTDOWN_S);
  const closed = useRef(false);

  const alert = alerts.data?.find((a) => a.id === alertId) ?? null;
  const isOpen = alert?.status === "open";

  const close = useCallback(() => {
    if (closed.current) return;
    closed.current = true;
    if (router.canGoBack()) router.back();
    else router.replace("/");
  }, [router]);

  const escalate = useCallback(() => {
    if (closed.current) return;
    notify(vi.emergency.doctorNotified, "ok");
    close();
  }, [close, notify]);

  useEffect(() => {
    if (alertId) markEmergencyShown(alertId);
  }, [alertId]);

  useEffect(() => {
    if (!isOpen || ack.isPending) return undefined;
    const timer = setInterval(() => setSeconds((s) => Math.max(0, s - 1)), 1_000);
    return () => clearInterval(timer);
  }, [isOpen, ack.isPending]);

  useEffect(() => {
    if (seconds === 0 && isOpen) escalate();
  }, [seconds, isOpen, escalate]);

  const onImOk = () => {
    if (!alert) return;
    ack.mutate(alert.id, {
      onSuccess: () => {
        notify(vi.emergency.cancelled, "ok");
        close();
      },
    });
  };

  if (alerts.isLoading) {
    return (
      <SafeAreaView style={[styles.safe, { backgroundColor: colors.bg }]}>
        <Loading />
      </SafeAreaView>
    );
  }

  // After "Tôi ổn" the refetched alert is no longer open; keep drawing it while the screen closes.
  if (!alert || (!isOpen && !closed.current)) {
    return (
      <SafeAreaView style={[styles.safe, styles.doneWrap]}>
        <Text style={styles.doneText}>{vi.emergency.notFound}</Text>
        <Button label={vi.emergency.back} onPress={close} variant="secondary" />
      </SafeAreaView>
    );
  }

  const reading = latest.data ?? null;
  const activity = reading && Math.abs(reading.ts_unix_ms - alert.raised_at_unix_ms) < FRESH_MS ? reading.activity : null;
  const critical = alert.severity === "critical";
  const background = critical ? colors.alert : colors.warn;

  return (
    <SafeAreaView style={[styles.safe, { backgroundColor: background }]} testID="emergency-screen">
      <StatusBar style="light" />
      <View style={styles.body}>
        <Text style={styles.kicker}>{critical ? vi.emergency.kickerCritical : vi.emergency.kickerWarning}</Text>
        <Text style={styles.title} accessibilityRole="header">
          {alertTitle(alert, activity)}
        </Text>
        <Text style={styles.context}>{alertContextLine(alert, activity)}</Text>
        <View style={styles.ring}>
          <CountdownRing seconds={seconds} total={EMERGENCY_COUNTDOWN_S} />
        </View>
        <Text style={styles.notice}>{vi.emergency.autoNotice}</Text>
        {ack.isError ? <Text style={styles.error}>{vi.emergency.cancelFailed}</Text> : null}
        <View style={styles.actions}>
          <Pressable
            accessibilityRole="button"
            onPress={onImOk}
            disabled={ack.isPending}
            style={({ pressed }) => [styles.okButton, (pressed || ack.isPending) && styles.pressed]}
            testID="im-ok"
          >
            <Text style={[styles.okText, { color: background }]}>
              {ack.isPending ? vi.emergency.cancelling : vi.emergency.imOk}
            </Text>
          </Pressable>
          <Pressable
            accessibilityRole="button"
            onPress={escalate}
            style={({ pressed }) => [styles.helpButton, pressed && styles.pressed]}
            testID="need-help"
          >
            <Text style={styles.helpText}>{vi.emergency.needHelp}</Text>
          </Pressable>
        </View>
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1 },
  body: { flex: 1, paddingHorizontal: 26, paddingTop: 28, paddingBottom: 24, alignItems: "center" },
  kicker: { fontFamily: fonts.bodySemi, fontSize: 15, letterSpacing: 1, color: colors.paper, opacity: 0.85 },
  title: { fontFamily: fonts.display, fontSize: 32, color: colors.paper, marginTop: 6, textAlign: "center" },
  context: { fontFamily: fonts.body, fontSize: 16.5, color: colors.paper, opacity: 0.92, marginTop: 6, textAlign: "center" },
  ring: { marginTop: 30, marginBottom: 18 },
  notice: { fontFamily: fonts.body, fontSize: 16.5, lineHeight: 23, color: colors.paper, textAlign: "center" },
  error: { fontFamily: fonts.bodySemi, fontSize: 15, color: colors.paper, marginTop: 10, textAlign: "center" },
  actions: { marginTop: "auto", alignSelf: "stretch", gap: 12, paddingTop: 24 },
  okButton: { backgroundColor: colors.paper, borderRadius: 18, minHeight: 58, alignItems: "center", justifyContent: "center" },
  okText: { fontFamily: fonts.bodyBold, fontSize: 19 },
  helpButton: { backgroundColor: colors.alertDeep, borderRadius: 18, minHeight: 58, alignItems: "center", justifyContent: "center" },
  helpText: { fontFamily: fonts.bodyBold, fontSize: 19, color: colors.paper },
  pressed: { opacity: 0.75 },
  doneWrap: { backgroundColor: colors.bg, padding: 24, justifyContent: "center", gap: 16 },
  doneText: { fontFamily: fonts.display, fontSize: 19, color: colors.ink, textAlign: "center" },
});
