sed -i 's/^crate-type/# crate-type/' ../Cargo.toml
RUST_BACKTRACE=1 cargo run --verbose -j 1; STATUS=$?; sed -i 's/^# crate-type/crate-type/' ../Cargo.toml; exit ${STATUS}
