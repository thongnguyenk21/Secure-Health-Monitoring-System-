import { StyleSheet, Text, View } from "react-native";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { toneStyle, type HealthStatus } from "../healthStatus";

/**
 * Context-aware health status card at the top of the home screen.
 *
 * @param props - The status to show.
 * @returns The card.
 */
export function StatusCard({ status }: { status: HealthStatus }) {
  const tone = toneStyle(status.tone);
  return (
    <View
      testID="status-card"
      accessibilityRole="summary"
      style={[styles.card, { backgroundColor: tone.background, borderColor: tone.color }]}
    >
      <View style={[styles.icon, { backgroundColor: tone.color }]}>
        <Text style={styles.glyph}>{tone.glyph}</Text>
      </View>
      <View style={styles.texts}>
        <Text style={styles.title}>{status.title}</Text>
        <Text style={styles.context}>{status.context}</Text>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    flexDirection: "row",
    alignItems: "center",
    gap: 14,
    borderWidth: 1,
    borderRadius: 16,
    paddingVertical: 14,
    paddingHorizontal: 14,
  },
  icon: { width: 46, height: 46, borderRadius: 23, alignItems: "center", justifyContent: "center" },
  glyph: { fontFamily: fonts.display, fontSize: 22, color: colors.paper },
  texts: { flex: 1, gap: 2 },
  title: { fontFamily: fonts.display, fontSize: 19, color: colors.ink },
  context: { fontFamily: fonts.body, fontSize: 15.5, color: colors.muted, lineHeight: 21 },
});
