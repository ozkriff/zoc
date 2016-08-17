# CARGO_FLAGS += --release
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

APK = ./target/android-artifacts/build/bin/zoc-debug.apk

android: assets
	cargo apk

android_run: android
	adb install -r $(APK)
	adb logcat -c
	adb shell am start -n rust.zoc/rust.zoc.MainActivity
	adb logcat -v time | grep 'Rust\|DEBUG'

.PHONY: zoc run android android_run test
