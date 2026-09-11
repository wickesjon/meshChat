package org.meshchat.securityprobe;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import java.io.File;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.security.KeyStore;
import java.util.Arrays;

/** Three external invocations establish process-restart persistence on an emulator. */
public final class SecurityInstrumentation extends Instrumentation {
    private String phase;

    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        phase = arguments.getString("phase", "");
        start();
    }

    private void invoke(Activity activity, String operation) throws Exception {
        Method method = SecurityActivity.class.getDeclaredMethod(operation);
        method.setAccessible(true);
        try { method.invoke(activity); }
        catch (InvocationTargetException error) {
            if (error.getCause() instanceof Exception) throw (Exception) error.getCause();
            throw error;
        }
    }

    private void refused(Activity activity, String operation) throws Exception {
        try { invoke(activity, operation); }
        catch (IllegalStateException expected) { return; }
        throw new AssertionError("expected explicit refusal");
    }

    @Override public void onStart() {
        Bundle report = new Bundle();
        Activity activity = null;
        int result = Activity.RESULT_CANCELED;
        try {
            Intent intent = new Intent(getTargetContext(), SecurityActivity.class);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            activity = startActivitySync(intent);
            waitForIdleSync();
            switch (phase) {
                case "create":
                    invoke(activity, "reset");
                    invoke(activity, "create");
                    invoke(activity, "verify");
                    refused(activity, "create");
                    break;
                case "reopen":
                    invoke(activity, "verify");
                    break;
                case "key-loss":
                    File directory = new File(getTargetContext().getNoBackupFilesDir(), "mc005");
                    File envelope = new File(directory, "material.enc");
                    File database = new File(directory, "fixture.db");
                    byte[] before = Files.readAllBytes(envelope.toPath());
                    byte[] databaseBefore = Files.readAllBytes(database.toPath());
                    KeyStore store = KeyStore.getInstance("AndroidKeyStore");
                    store.load(null);
                    store.deleteEntry("mc005-wrapping-v1");
                    refused(activity, "verify");
                    refused(activity, "create");
                    if (store.containsAlias("mc005-wrapping-v1") ||
                        !Arrays.equals(before, Files.readAllBytes(envelope.toPath())) ||
                        !Arrays.equals(databaseBefore, Files.readAllBytes(database.toPath()))) {
                        throw new AssertionError("key loss changed retained fixture");
                    }
                    invoke(activity, "reset");
                    invoke(activity, "create");
                    invoke(activity, "verify");
                    invoke(activity, "reset");
                    break;
                default: throw new AssertionError("unknown test phase");
            }
            report.putString("mc005", "PASS " + phase + " synthetic_functional_only");
            result = Activity.RESULT_OK;
        } catch (Throwable error) {
            report.putString("mc005", "FAIL " + phase + " type=" + error.getClass().getSimpleName());
        } finally {
            if (activity != null) {
                Activity finished = activity;
                runOnMainSync(finished::finish);
            }
        }
        finish(result, report);
    }
}
