CARGO_FLAGS += -j 1
CARGO_FLAGS += --release
# CARGO_FLAGS += --verbose

zoc:
	cargo build $(CARGO_FLAGS)

test:
	cargo test --package core $(CARGO_FLAGS)
	cargo test --package visualizer $(CARGO_FLAGS)

run:
	RUST_BACKTRACE=1 cargo run $(CARGO_FLAGS)

ANDROID_APP_NAME = com.example.native_activity/android.app.NativeActivity

android:
	cargo build --target arm-linux-androideabi -j 1 --release

android_run: android
	cp target/arm-linux-androideabi/release/zoc target/arm-linux-androideabi/release/zoc.apk
	adb install -r target/arm-linux-androideabi/release/zoc.apk
	adb logcat -c
	adb shell am start -n $(ANDROID_APP_NAME)
	adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

.PHONY: zoc run android android_run test
