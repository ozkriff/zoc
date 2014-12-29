#!/bin/sh

set -e

APPNAME=com.example.native_activity/android.app.NativeActivity

ndk-build
ant debug
adb install -r bin/RustyCardboard-debug.apk
adb logcat -c
adb shell am start -n $APPNAME
adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

