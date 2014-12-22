#!/bin/sh

set -e

APPNAME=com.example.native_activity/android.app.NativeActivity

cargo build --target arm-linux-androideabi -j 1 -v --release
cp target/arm-linux-androideabi/release/libmarauder-*.a android/jni/librust.a
cd android
ndk-build
ant debug
adb install -r bin/RustyCardboard-debug.apk
adb shell am start -n $APPNAME
adb logcat -c
adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

