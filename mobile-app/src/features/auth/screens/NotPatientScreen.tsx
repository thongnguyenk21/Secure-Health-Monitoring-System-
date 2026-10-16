import { useQueryClient } from "@tanstack/react-query";
import { StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { api } from "../../../api/client";
import { Button } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";

/**
 * Shown when a clinician or admin logs in: the app is for patients only (D28).
 *
 * @returns The screen.
 */
export function NotPatientScreen() {
  const client = useQueryClient();
  return (
    <SafeAreaView style={styles.safe}>
      <View style={styles.card}>
        <Text style={styles.title}>{vi.notPatient.title}</Text>
        <Text style={styles.body}>{vi.notPatient.body}</Text>
        <Button
          label={vi.notPatient.logout}
          onPress={async () => {
            await api.logout();
            client.clear();
          }}
          testID="logout"
        />
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: colors.bg, justifyContent: "center", padding: 24 },
  card: { backgroundColor: colors.paper, borderWidth: 1, borderColor: colors.line, borderRadius: 18, padding: 20, gap: 14 },
  title: { fontFamily: fonts.display, fontSize: 21, color: colors.ink },
  body: { fontFamily: fonts.body, fontSize: 16.5, color: colors.muted, lineHeight: 23 },
});
