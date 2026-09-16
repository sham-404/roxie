EXE    := roxie
TARGET := $(shell rustc --print host-tuple)

# 1. Ask the Rust compiler what the host CPU actually supports
CFG_FEATURES := $(shell rustc --print cfg -C target-cpu=native)
HAS_AVX512   := $(if $(findstring target_feature="avx512f",$(CFG_FEATURES)),1,0)
HAS_AVX2     := $(if $(findstring target_feature="avx2",$(CFG_FEATURES)),1,0)
HAS_NEON     := $(if $(findstring target_feature="neon",$(CFG_FEATURES)),1,0)

# 2. Route to the most powerful available instruction set
ifeq ($(HAS_AVX512),1)
    BEST_TARGET := avx512
    BEST_FLAG   := -C target-cpu=x86-64-v4 -C target-feature=+avx512f,+avx512bw,+avx512dq,+avx512vl,+avx512vnni,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols
else ifeq ($(HAS_AVX2),1)
    BEST_TARGET := avx2
    BEST_FLAG   := -C target-cpu=x86-64-v3 -C target-feature=+avx2,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols
else ifeq ($(HAS_NEON),1)
    BEST_TARGET := neon
    BEST_FLAG   := -C target-cpu=native -C target-feature=+neon -C debuginfo=0 -C strip=symbols
else
    BEST_TARGET := native
    BEST_FLAG   := -C target-cpu=native -C debuginfo=0 -C strip=symbols
endif

# Explicit Architecture Instruction Sets (for manual overrides/cross-compilation)
FLAG_NATIVE := -C target-cpu=native -C debuginfo=0 -C strip=symbols
FLAG_AVX512 := -C target-cpu=x86-64-v4 -C target-feature=+avx512f,+avx512bw,+avx512dq,+avx512vl,+avx512vnni,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols
FLAG_AVX2   := -C target-cpu=x86-64-v3 -C target-feature=+avx2,+bmi2,+popcnt -C debuginfo=0 -C strip=symbols
FLAG_NEON   := -C target-cpu=native -C target-feature=+neon -C debuginfo=0 -C strip=symbols

.PHONY: all auto native avx512 avx2 neon pgo clean help

all: auto

auto: ## Automatically detect host CPU and build the fastest version
	@echo "Host CPU features detected."
	@echo "Optimal instruction set selected: $(BEST_TARGET)"
	@$(MAKE) $(BEST_TARGET)

native: ## Build with generic native optimizations
	RUSTFLAGS="$(FLAG_NATIVE)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-native

avx512: ## Build explicitly for AVX-512 (x86-64-v4)
	RUSTFLAGS="$(FLAG_AVX512)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx512

avx2: ## Build explicitly for AVX2 (x86-64-v3)
	RUSTFLAGS="$(FLAG_AVX2)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx2

neon: ## Build explicitly for ARM NEON
	RUSTFLAGS="$(FLAG_NEON)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-neon

pgo: ## Auto-detect CPU and run a full Profile-Guided Optimization build
	@echo "Instrumenting for $(BEST_TARGET)..."
	RUSTFLAGS="$(BEST_FLAG)" cargo pgo instrument
	@echo "Running engine benchmarks to gather PGO profiling data..."
	cargo pgo run -- bench
	@echo "Recompiling with PGO data and $(BEST_TARGET) optimizations..."
	RUSTFLAGS="$(BEST_FLAG)" cargo pgo optimize
	mv "target/$(TARGET)/release/$(EXE)" "$(EXE)-pgo-$(BEST_TARGET)"

clean: ## Clean build artifacts
	cargo clean
	rm -f $(EXE)-native $(EXE)-avx512 $(EXE)-avx2 $(EXE)-neon $(EXE)-pgo-*

help: ## Show this help message
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*?##/ { \
		printf "  %-15s %s\n", $$1, $$2 \
	}' $(MAKEFILE_LIST)
