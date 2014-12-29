all: linux

linux:
	cd bin && ./make.sh

run: linux
	./bin/target/zoc

android:
	cargo build --target arm-linux-androideabi -j 1 -v --release
	cp target/arm-linux-androideabi/release/libzoc-*.a android/jni/librust.a
	cd android && ./make.sh

.PHONY: all linux run android
