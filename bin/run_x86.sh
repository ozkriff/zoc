sed -i 's/^crate-type/# crate-type/' ../Cargo.toml
cargo run; STATUS=$?; sed -i 's/^# crate-type/crate-type/' ../Cargo.toml; exit ${STATUS}
