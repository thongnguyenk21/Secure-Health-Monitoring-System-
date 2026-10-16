import { Pressable, StyleSheet, Text, View } from "react-native";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";

/** Props of {@link ToggleRow}. */
export interface ToggleRowProps {
  /** Label on the left. */
  label: string;
  /** Current position. */
  value: boolean;
  /** Called with the new position. */
  onChange: (next: boolean) => void;
  /** Disable while a change is being saved. */
  disabled?: boolean;
  /** Test id for screen tests. */
  testID?: string;
}

/**
 * Labelled switch drawn like the wireframe `.tg` toggle, with a full-row touch target.
 *
 * @param props - See {@link ToggleRowProps}.
 * @returns The row.
 */
export function ToggleRow({ label, value, onChange, disabled = false, testID }: ToggleRowProps) {
  return (
    <Pressable
      accessibilityRole="switch"
      accessibilityLabel={label}
      accessibilityState={{ checked: value, disabled }}
      disabled={disabled}
      onPress={() => onChange(!value)}
      style={({ pressed }) => [styles.row, (pressed || disabled) && styles.dim]}
      testID={testID}
    >
      <Text style={styles.label}>{label}</Text>
      <View style={[styles.track, value && styles.trackOn]}>
        <View style={[styles.knob, value && styles.knobOn]} />
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", minHeight: 48, gap: 12 },
  dim: { opacity: 0.6 },
  label: { flex: 1, fontFamily: fonts.body, fontSize: 16, color: colors.ink },
  track: { width: 46, height: 26, borderRadius: 13, backgroundColor: colors.line, justifyContent: "center" },
  trackOn: { backgroundColor: colors.ok },
  knob: { width: 20, height: 20, borderRadius: 10, backgroundColor: colors.paper, marginLeft: 3 },
  knobOn: { marginLeft: 23 },
});
