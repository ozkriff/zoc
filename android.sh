#!/bin/sh

set -e

ADB=$ANDROID_SDK_HOME/platform-tools/adb
APPNAME=com.example.native_activity/android.app.NativeActivity

cargo build --target arm-linux-androideabi -j 1
cp target/arm-linux-androideabi/libmarauder-*.a android/jni/librust.a
cd android
ndk-build
ant debug
$ADB install -r bin/RustyCardboard-debug.apk
$ADB shell am start -n $APPNAME
$ADB logcat | grep RustAndroidGlue

