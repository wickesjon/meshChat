#!/usr/bin/env bash
# Source from the repository root on the isolated Linux CI runner.
# No host permission changes: software emulation is allowed when KVM is absent.
start_security_emulator() {
  export ANDROID_HOME="$PWD/.work/emulator-sdk"
  export ANDROID_SDK_ROOT="$ANDROID_HOME"
  export ANDROID_AVD_HOME="$PWD/.work/android/avd"
  export ANDROID_USER_HOME="$PWD/.work/android"
  export TMPDIR="$PWD/.work/tmp"
  security_acceleration=off
  if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then security_acceleration=on; fi
  "$ANDROID_HOME/emulator/emulator" -avd mc005 -port 5554 -wipe-data -no-window -no-audio \
    -no-boot-anim -no-snapshot -gpu swiftshader -accel "$security_acceleration" \
    > .work/emulator.log 2>&1 &
  security_emulator_pid=$!
  trap 'tail -n 80 .work/emulator.log; kill "$security_emulator_pid" 2>/dev/null || true' EXIT
  "$ANDROID_HOME/platform-tools/adb" start-server
  security_boot_deadline=$((SECONDS + 600))
  security_booted=false
  security_ready_samples=0
  while [ "$SECONDS" -lt "$security_boot_deadline" ]; do
    if ! kill -0 "$security_emulator_pid" 2>/dev/null; then exit 1; fi
    security_boot_state=$(timeout 5 "$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
    security_package_state=$(timeout 5 "$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell service check package 2>/dev/null || true)
    security_activity_state=$(timeout 5 "$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell service check activity 2>/dev/null || true)
    if [ "$security_boot_state" = 1 ] && [[ "$security_package_state" == *": found"* ]] && [[ "$security_activity_state" == *": found"* ]] &&
      timeout 15 "$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell input keyevent 224 &&
      timeout 15 "$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell input keyevent 82; then
      security_ready_samples=$((security_ready_samples + 1))
      if [ "$security_ready_samples" -ge 2 ]; then
        security_booted=true
        break
      fi
    else
      security_ready_samples=0
    fi
    sleep 2
  done
  if [ "$security_booted" != true ]; then exit 1; fi
  echo "Security emulator ready after two service/wake checks; hardware acceptance remains separate"
  trap - EXIT
}

stop_security_emulator() {
  if [ -x .work/emulator-sdk/platform-tools/adb ]; then
    timeout 15 .work/emulator-sdk/platform-tools/adb -s emulator-5554 emu kill || true
  fi
}
