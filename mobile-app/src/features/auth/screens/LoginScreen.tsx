import { useState } from "react";
import { KeyboardAvoidingView, Platform, ScrollView, StyleSheet, Text, TextInput, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import Svg, { Path } from "react-native-svg";
import { api } from "../../../api/client";
import { Button } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { useSessionState } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { loginErrorMessage } from "../loginError";

/**
 * Login screen: email and password, with plain-words errors.
 *
 * @returns The screen.
 */
export function LoginScreen() {
  const { endReason } = useSessionState();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(endReason === "expired" ? vi.login.sessionEnded : null);

  const submit = async () => {
    if (busy) return;
    if (!email.trim() || !password) {
      setError(vi.login.missingFields);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await api.login({ email: email.trim(), password });
    } catch (err) {
      setError(loginErrorMessage(err));
      setBusy(false);
    }
  };

  return (
    <SafeAreaView style={styles.safe}>
      <KeyboardAvoidingView behavior={Platform.OS === "android" ? "height" : "padding"} style={styles.flex}>
        <ScrollView contentContainerStyle={styles.content} keyboardShouldPersistTaps="handled">
          <View style={styles.logo}>
            <Svg width={34} height={34} viewBox="0 0 24 24">
              <Path
                d="M12 20.5s-7.5-4.6-7.5-10.2A4.3 4.3 0 0 1 12 7.6a4.3 4.3 0 0 1 7.5 2.7c0 5.6-7.5 10.2-7.5 10.2z"
                fill={colors.paper}
              />
              <Path d="M5.5 12.5h3l1.5-2.5 2 4.5 1.5-2h5" fill="none" stroke={colors.accent} strokeWidth={1.6} strokeLinejoin="round" />
            </Svg>
          </View>
          <Text style={styles.app}>{vi.appName}</Text>
          <Text style={styles.subtitle}>{vi.login.subtitle}</Text>
          <View style={styles.form}>
            <Text style={styles.label}>{vi.login.email}</Text>
            <TextInput
              style={styles.input}
              value={email}
              onChangeText={setEmail}
              autoCapitalize="none"
              autoComplete="email"
              keyboardType="email-address"
              textContentType="username"
              inputMode="email"
              testID="email"
              accessibilityLabel={vi.login.email}
            />
            <Text style={styles.label}>{vi.login.password}</Text>
            <TextInput
              style={styles.input}
              value={password}
              onChangeText={setPassword}
              secureTextEntry
              autoComplete="password"
              textContentType="password"
              onSubmitEditing={submit}
              testID="password"
              accessibilityLabel={vi.login.password}
            />
            {error ? (
              <Text style={styles.error} accessibilityLiveRegion="polite" testID="login-error">
                {error}
              </Text>
            ) : null}
            <View style={styles.submit}>
              <Button label={busy ? vi.login.submitting : vi.login.submit} onPress={submit} disabled={busy} testID="login" />
            </View>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  safe: { flex: 1, backgroundColor: colors.bg },
  flex: { flex: 1 },
  content: { flexGrow: 1, justifyContent: "center", padding: 24 },
  logo: {
    width: 64,
    height: 64,
    borderRadius: 20,
    backgroundColor: colors.accent,
    alignItems: "center",
    justifyContent: "center",
    alignSelf: "center",
    marginBottom: 14,
  },
  app: { fontFamily: fonts.display, fontSize: 26, color: colors.ink, textAlign: "center" },
  subtitle: { fontFamily: fonts.body, fontSize: 16, color: colors.muted, textAlign: "center", marginTop: 4 },
  form: {
    marginTop: 28,
    backgroundColor: colors.paper,
    borderWidth: 1,
    borderColor: colors.line,
    borderRadius: 18,
    padding: 18,
  },
  label: { fontFamily: fonts.bodySemi, fontSize: 15, color: colors.ink, marginTop: 8, marginBottom: 6 },
  input: {
    minHeight: 50,
    borderWidth: 1,
    borderColor: colors.line,
    borderRadius: 12,
    paddingHorizontal: 14,
    fontFamily: fonts.body,
    fontSize: 17,
    color: colors.ink,
    backgroundColor: colors.bg,
  },
  error: { fontFamily: fonts.bodySemi, fontSize: 15, color: colors.alert, marginTop: 12, lineHeight: 21 },
  submit: { marginTop: 18 },
});
