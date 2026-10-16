import { Pressable, StyleSheet, Text, View } from "react-native";
import type { Alert } from "../../../api/types";
import { vi } from "../../../i18n/vi";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { formatWhen } from "../../../utils/format";
import { alertStatusText, alertTitle, alertValueText, severityColor } from "../alertText";

/** Props of {@link AlertRow}. */
export interface AlertRowProps {
  /** The alert. */
  alert: Alert;
  /** Id of the signed-in patient. */
  myUserId: string | null;
  /** Current time, UTC epoch ms, for relative dates. */
  nowMs: number;
  /** Press handler; when set the row shows a "Xem" action. */
  onOpen?: () => void;
}

/**
 * One alert in the alert list or the history timeline.
 *
 * @param props - See {@link AlertRowProps}.
 * @returns The row with a left border coloured by severity.
 */
export function AlertRow({ alert, myUserId, nowMs, onOpen }: AlertRowProps) {
  const value = alertValueText(alert);
  const sub = [formatWhen(alert.raised_at_unix_ms, nowMs), value, alertStatusText(alert, myUserId)]
    .filter((p): p is string => !!p)
    .join(" · ");
  const content = (
    <View style={[styles.row, { borderLeftColor: severityColor(alert.severity) }]}>
      <View style={styles.texts}>
        <Text style={styles.title}>{alertTitle(alert)}</Text>
        <Text style={styles.sub}>{sub}</Text>
      </View>
      {onOpen ? <Text style={styles.action}>{vi.alerts.open}</Text> : null}
    </View>
  );
  if (!onOpen) return content;
  return (
    <Pressable accessibilityRole="button" onPress={onOpen} style={({ pressed }) => pressed && styles.pressed}>
      {content}
    </Pressable>
  );
}

const styles = StyleSheet.create({
  row: {
    flexDirection: "row",
    alignItems: "center",
    backgroundColor: colors.paper,
    borderWidth: 1,
    borderColor: colors.line,
    borderLeftWidth: 5,
    borderRadius: 14,
    paddingVertical: 10,
    paddingHorizontal: 14,
    gap: 10,
  },
  texts: { flex: 1, gap: 2 },
  title: { fontFamily: fonts.bodyBold, fontSize: 17, color: colors.ink },
  sub: { fontFamily: fonts.body, fontSize: 14.5, color: colors.muted, lineHeight: 20 },
  action: { fontFamily: fonts.bodyBold, fontSize: 16, color: colors.accent },
  pressed: { opacity: 0.7 },
});
