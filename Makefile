all: zoc

zoc:
	cd client && cargo build --verbose -j 1

run: zoc
	RUST_BACKTRACE=1 ./client/target/zoc

android:
	cargo build --target arm-linux-androideabi -j 1 -v --release
	cp target/arm-linux-androideabi/release/libzoc-*.a android/jni/librust.a
	cd android && ./make.sh

.PHONY: all zoc run android
