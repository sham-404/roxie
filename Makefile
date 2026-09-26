EXE    := roxie
TARGET := $(shell rustc --print host-tuple)

# 1. Ask the Rust compiler what the host CPU actually supports
CFG_FEATURES := $(shell rustc --print cfg -C target-cpu=native)

IS_X86_64  := $(if $(findstring target_arch="x86_64",$(CFG_FEATURES)),1,0)
IS_AARCH64 := $(if $(findstring target_arch="aarch64",$(CFG_FEATURES)),1,0)

HAS_AVX512 := $(if $(findstring target_feature="avx512f",$(CFG_FEATURES)),1,0)
HAS_AVX2   := $(if $(findstring target_feature="avx2",$(CFG_FEATURES)),1,0)
HAS_NEON   := $(if $(findstring target_feature="neon",$(CFG_FEATURES)),1,0)

# Base stripping/debug flags to keep it DRY
BASE_FLAGS := -C debuginfo=0 -C strip=symbols

# Explicit Architecture Instruction Sets
FLAG_NATIVE      := -C target-cpu=native $(BASE_FLAGS)
FLAG_AVX512      := -C target-cpu=x86-64-v4 -C target-feature=+avx512f,+avx512bw,+avx512dq,+avx512vl,+avx512vnni,+bmi2,+popcnt $(BASE_FLAGS)
FLAG_AVX2        := -C target-cpu=x86-64-v3 -C target-feature=+avx2,+bmi2,+popcnt $(BASE_FLAGS)
FLAG_GENERIC_X86 := -C target-cpu=x86-64-v2 -C target-feature=+popcnt,+sse4.2 $(BASE_FLAGS)
FLAG_NEON        := -C target-cpu=native -C target-feature=+neon $(BASE_FLAGS)
FLAG_GENERIC_ARM := -C target-cpu=generic $(BASE_FLAGS)

# 2. Route to the most powerful available instruction set by Architecture
ifeq ($(IS_X86_64),1)
    ifeq ($(HAS_AVX512),1)
        BEST_TARGET := avx512
        BEST_FLAG   := $(FLAG_AVX512)
    else ifeq ($(HAS_AVX2),1)
        BEST_TARGET := avx2
        BEST_FLAG   := $(FLAG_AVX2)
    else
        BEST_TARGET := generic-x86
        BEST_FLAG   := $(FLAG_GENERIC_X86)
    endif
else ifeq ($(IS_AARCH64),1)
    ifeq ($(HAS_NEON),1)
        BEST_TARGET := neon
        BEST_FLAG   := $(FLAG_NEON)
    else
        BEST_TARGET := generic-arm
        BEST_FLAG   := $(FLAG_GENERIC_ARM)
    endif
else
    # Fallback for completely unknown/exotic architectures
    BEST_TARGET := native
    BEST_FLAG   := $(FLAG_NATIVE)
endif

.PHONY: all auto native avx512 avx2 neon generic-x86 generic-arm pgo clean help

all: auto

auto: ## Automatically detect host CPU and build the fastest version
	@echo "Host Architecture: $(if $(filter 1,$(IS_X86_64)),x86_64,$(if $(filter 1,$(IS_AARCH64)),aarch64,unknown))"
	@echo "Optimal instruction set selected: $(BEST_TARGET)"
	@$(MAKE) $(BEST_TARGET)

native: ## Build with generic native optimizations
	RUSTFLAGS="$(FLAG_NATIVE)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-native

avx512: ## Build explicitly for AVX-512 (x86-64-v4)
	RUSTFLAGS="$(FLAG_AVX512)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx512

avx2: ## Build explicitly for AVX2 (x86-64-v3)
	RUSTFLAGS="$(FLAG_AVX2)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-avx2

generic-x86: ## Build explicitly for x86-64-v2 (Includes POPCNT/SSE4.2)
	RUSTFLAGS="$(FLAG_GENERIC_X86)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-generic-x86

neon: ## Build explicitly for ARM NEON
	RUSTFLAGS="$(FLAG_NEON)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-neon

generic-arm: ## Build explicitly for generic ARM without NEON
	RUSTFLAGS="$(FLAG_GENERIC_ARM)" cargo rustc --release --bin $(EXE) -- --emit link=$(EXE)-generic-arm

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
	rm -f $(EXE)-native $(EXE)-avx512 $(EXE)-avx2 $(EXE)-neon $(EXE)-generic* $(EXE)-pgo-*

help: ## Show this help message
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*?##/ { \
		printf "  %-15s %s\n", $$1, $$2 \
	}' $(MAKEFILE_LIST)
