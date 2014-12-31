all: linux

linux:
	cd linux && ./make.sh

run: linux
	RUST_BACKTRACE=1 ./linux/target/zoc

android:
	cargo build --target arm-linux-androideabi -j 1 -v --release
	cp target/arm-linux-androideabi/release/libzoc-*.a android/jni/librust.a
	cd android && ./make.sh

.PHONY: all linux run android
