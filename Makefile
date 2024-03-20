PRECOMMIT_VERSION="3.6.2"

.PHONY: pre-commit

# Assumes python3
pre-commit:
	wget -O pre-commit.pyz https://github.com/pre-commit/pre-commit/releases/download/v${PRECOMMIT_VERSION}/pre-commit-${PRECOMMIT_VERSION}.pyz
	python pre-commit.pyz install
	python pre-commit.pyz install --hook-type commit-msg
	rm pre-commit.pyz

# Assumes rustup
init-dev: pre-commit
	rustup component add clippy
	rustup component add rustfmt

build:
	cargo build --release

test:
	cargo test
