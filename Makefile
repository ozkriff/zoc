all: zoc

zoc:
	cd client && cargo build --verbose -j 1

run: zoc
	RUST_BACKTRACE=1 ./client/target/zoc

ANDROID_APP_NAME = com.example.native_activity/android.app.NativeActivity

android: android_build

android_build:
	rm -rf android/assets/*
	cp -r data android/assets/data
	cargo build --target arm-linux-androideabi -j 1 -v --release
	cp target/arm-linux-androideabi/release/libzoc-*.a android/jni/librust.a
	cd android && ndk-build && ant debug

android_run: android_build
	adb install -r android/bin/RustyCardboard-debug.apk
	adb logcat -c
	adb shell am start -n $(ANDROID_APP_NAME)
	adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

.PHONY: all zoc run android android_build android_run
