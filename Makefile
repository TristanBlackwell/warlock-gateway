dev:
	cargo run

start:
	cargo run

test:
	cargo test

# Prepare sqlx offline query cache for CI
prepare-sqlx:
	DATABASE_URL=sqlite::memory: cargo sqlx prepare

# Check if sqlx offline cache is up to date
check-sqlx:
	DATABASE_URL=sqlite::memory: cargo sqlx prepare --check
