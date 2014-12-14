all:
	sed -i 's/^crate-type/# crate-type/' ../Cargo.toml; \
		cargo build; \
		STATUS=$?; \
		sed -i 's/^# crate-type/crate-type/' ../Cargo.toml; \
		exit $STATUS
