sed -i 's/^crate-type/# crate-type/' ../Cargo.toml
RUST_BACKTRACE=1 cargo build --verbose -j 1; STATUS=$?; sed -i 's/^# crate-type/crate-type/' ../Cargo.toml; exit ${STATUS}
