zoc:
	cargo build -j 1

test:
	cargo test -j 1 --package core

run: zoc
	RUST_BACKTRACE=1 cargo run

ANDROID_APP_NAME = com.example.native_activity/android.app.NativeActivity

android:
	cargo build --target arm-linux-androideabi -j 1 -v --release

android_run: android
	adb install -r target/arm-linux-androideabi/release/zoc
	adb logcat -c
	adb shell am start -n $(ANDROID_APP_NAME)
	adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

.PHONY: zoc run android android_run test
