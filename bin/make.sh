sed -i 's/^crate-type/# crate-type/' ../Cargo.toml
cargo build --verbose -j 1; STATUS=$?; sed -i 's/^# crate-type/crate-type/' ../Cargo.toml; exit ${STATUS}
