CARGO_FLAGS += --release
# CARGO_FLAGS += --verbose

zoc: assets
	cargo build $(CARGO_FLAGS)

test:
	cargo test --package core $(CARGO_FLAGS)
	cargo test --package visualizer $(CARGO_FLAGS)

run: assets
	RUST_BACKTRACE=1 cargo run $(CARGO_FLAGS)

assets:
	git clone --depth=1 https://github.com/ozkriff/zoc_assets assets

ANDROID_APP_NAME = com.example.native_activity/android.app.NativeActivity

android: assets
	cargo build --target arm-linux-androideabi --release

android_run: android
	cp target/arm-linux-androideabi/release/zoc target/arm-linux-androideabi/release/zoc.apk
	adb install -r target/arm-linux-androideabi/release/zoc.apk
	adb logcat -c
	adb shell am start -n $(ANDROID_APP_NAME)
	adb logcat -v time | grep 'RustAndroidGlue\|native-activity'

.PHONY: zoc run android android_run test
