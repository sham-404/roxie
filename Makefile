EXE    := roxie
TARGET := $(shell rustc --print host-tuple)

# Explicit Architecture Instruction Sets
FLAG_NATIVE := -C target-cpu=native -C debuginfo=0 -C strip=symbols
FLAG_AVX512 := -C target-cpu=x86-64-v4 -C target-feature=+avx512f,+avx512bw,+avx512dq,+avx512vl,+avx512vnni,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols
FLAG_AVX2   := -C target-cpu=x86-64-v3 -C target-feature=+avx2,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols

.PHONY: all native avx512 avx2 pgo-avx512 clean help

all: native

native:
	RUSTFLAGS="$(FLAG_NATIVE)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)

avx512:
	RUSTFLAGS="$(FLAG_AVX512)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx512

avx2:
	RUSTFLAGS="$(FLAG_AVX2)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx2

pgo-avx512:
	@echo "Instrumenting for AVX-512..."
	RUSTFLAGS="$(FLAG_AVX512)" cargo pgo instrument
	@echo "Running engine benchmarks to gather PGO profiling data..."
	cargo pgo run -- bench
	@echo "Recompiling with PGO data and AVX-512 optimizations..."
	RUSTFLAGS="$(FLAG_AVX512)" cargo pgo optimize
	mv "target/$(TARGET)/release/$(EXE)" "$(EXE)-pgo-avx512"

clean:
	cargo clean
	rm -f $(EXE) $(EXE)-avx512 $(EXE)-avx2 $(EXE)-pgo-avx512

help:
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*?##/ { \
		printf "  %-15s %s\n", $$1, $$2 \
	}' $(MAKEFILE_LIST)
