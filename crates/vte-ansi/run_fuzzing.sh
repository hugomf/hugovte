#!/bin/bash
# Comprehensive fuzzing script for ANSI parser
# Usage: ./run_fuzzing.sh [quick|full|continuous]

set -e

MODE="${1:-quick}"
FUZZ_DIR="."
FUZZ_TARGETS=(
    "parser_basic:120"     # 2 hours
    "parser_utf8:60"       # 1 hour
    "parser_sgr:60"        # 1 hour
    "parser_state:60"      # 1 hour
    "parser_osc:30"        # 30 minutes
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo-fuzz is installed
check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check for nightly toolchain
    if ! rustup toolchain list | grep -q nightly; then
        log_warning "Nightly toolchain not found. Installing..."
        rustup toolchain install nightly
    fi
    
    # Check current toolchain
    local current_toolchain=$(rustup show active-toolchain | cut -d' ' -f1)
    if [[ ! "$current_toolchain" =~ "nightly" ]]; then
        log_warning "Current toolchain is $current_toolchain"
        log_info "Fuzzing requires nightly. Using +nightly for fuzz commands..."
        CARGO_FUZZ="cargo +nightly fuzz"
    else
        CARGO_FUZZ="cargo fuzz"
    fi
    
    if ! command -v cargo-fuzz &> /dev/null; then
        log_info "cargo-fuzz not found. Installing..."
        cargo +nightly install cargo-fuzz
    fi
    
    log_success "Dependencies OK (using: $CARGO_FUZZ)"
}

# Initialize fuzzing if needed
init_fuzzing() {
    if [ ! -d "$FUZZ_DIR/fuzz" ]; then
        log_error "Fuzz directory not found at $FUZZ_DIR/fuzz. Please ensure it has been moved."
        exit 1
    fi
    log_info "Fuzz directory found at $FUZZ_DIR/fuzz"
}

# Run a single fuzz target
run_target() {
    local target=$1
    local duration=$2
    local sanitizer=${3:-none}

    log_info "Running $target for ${duration}s with ${sanitizer} sanitizer..."

    local base_cmd="cd '$FUZZ_DIR/fuzz' && $CARGO_FUZZ run $target"
    local cmd="$base_cmd -- -max_total_time=$duration -timeout=1"

    if [ "$sanitizer" != "none" ]; then
        cmd="$base_cmd --sanitizer=$sanitizer -- -max_total_time=$duration -timeout=1"
    fi

    if eval "$cmd"; then
        log_success "$target completed successfully"
        return 0
    else
        log_error "$target found issues!"
        return 1
    fi
}

# Check for crashes
check_crashes() {
    local target=$1
    local crash_dir="$FUZZ_DIR/fuzz/artifacts/$target"

    if [ -d "$crash_dir" ] && [ "$(ls -A $crash_dir 2>/dev/null)" ]; then
        log_error "Crashes found in $crash_dir:"
        ls -lh "$crash_dir"
        return 1
    fi
    return 0
}

# Run quick fuzzing (5 minutes per target)
run_quick() {
    log_info "Running quick fuzzing (5 minutes per target)..."
    
    local failed=0
    
    for target_spec in "${FUZZ_TARGETS[@]}"; do
        IFS=':' read -r target _ <<< "$target_spec"
        
        if ! run_target "$target" 300; then
            ((failed++))
        fi
        
        if ! check_crashes "$target"; then
            ((failed++))
        fi
    done
    
    return $failed
}

# Run full fuzzing (recommended durations)
run_full() {
    log_info "Running full fuzzing (6+ hours total)..."
    
    local failed=0
    
    for target_spec in "${FUZZ_TARGETS[@]}"; do
        IFS=':' read -r target duration <<< "$target_spec"
        
        # Convert minutes to seconds
        local seconds=$((duration * 60))
        
        if ! run_target "$target" "$seconds"; then
            ((failed++))
        fi
        
        if ! check_crashes "$target"; then
            ((failed++))
        fi
    done
    
    return $failed
}

# Run continuous fuzzing (until interrupted)
run_continuous() {
    log_info "Running continuous fuzzing (Ctrl+C to stop)..."
    log_warning "Results will be saved. You can resume anytime."
    
    while true; do
        for target_spec in "${FUZZ_TARGETS[@]}"; do
            IFS=':' read -r target duration <<< "$target_spec"
            
            log_info "Fuzzing $target for ${duration} minutes..."
            run_target "$target" $((duration * 60)) || true
            
            # Check and report crashes after each run
            if ! check_crashes "$target"; then
                log_warning "Crashes detected, but continuing..."
            fi
            
            # Brief pause between targets
            sleep 5
        done
        
        log_info "Completed one full cycle. Starting over..."
        sleep 10
    done
}

# Run with sanitizers
run_sanitizers() {
    log_info "Running with sanitizers..."
    
    local failed=0
    local sanitizers=("address" "undefined")
    
    for sanitizer in "${sanitizers[@]}"; do
        log_info "Testing with $sanitizer sanitizer..."
        
        for target_spec in "${FUZZ_TARGETS[@]}"; do
            IFS=':' read -r target _ <<< "$target_spec"
            
            # Shorter runs with sanitizers (they're slower)
            if ! run_target "$target" 180 "$sanitizer"; then
                ((failed++))
            fi
        done
    done
    
    return $failed
}

# Generate coverage report
generate_coverage() {
    log_info "Generating coverage report..."

    for target_spec in "${FUZZ_TARGETS[@]}"; do
        IFS=':' read -r target _ <<< "$target_spec"

        log_info "Coverage for $target..."
        if cd "$FUZZ_DIR/fuzz" && $CARGO_FUZZ coverage "$target"; then
            log_success "Coverage generated for $target"
        else
            log_warning "Coverage generation failed for $target"
        fi
    done
}

# Minimize crash cases
minimize_crashes() {
    log_info "Minimizing crash cases..."

    local found_crashes=0

    for target_spec in "${FUZZ_TARGETS[@]}"; do
        IFS=':' read -r target _ <<< "$target_spec"
        local crash_dir="$FUZZ_DIR/fuzz/artifacts/$target"

        if [ -d "$crash_dir" ] && [ "$(ls -A $crash_dir 2>/dev/null)" ]; then
            log_info "Minimizing crashes for $target..."

            for crash in "$crash_dir"/*; do
                if [ -f "$crash" ]; then
                    log_info "Minimizing $(basename $crash)..."
                    cd "$FUZZ_DIR/fuzz" && $CARGO_FUZZ cmin "$target" "$(basename "$crash")" || true
                    ((found_crashes++))
                fi
            done
        fi
    done

    if [ $found_crashes -eq 0 ]; then
        log_success "No crashes to minimize"
    else
        log_warning "Minimized $found_crashes crash case(s)"
    fi
}

# Clean up artifacts
clean_artifacts() {
    log_info "Cleaning up fuzz artifacts..."

    if [ -d "$FUZZ_DIR/fuzz/artifacts" ]; then
        rm -rf "$FUZZ_DIR/fuzz/artifacts"/*
        log_success "Artifacts cleaned"
    fi

    if [ -d "$FUZZ_DIR/fuzz/corpus" ]; then
        log_info "Corpus directory preserved (contains useful test cases)"
    fi
}

# Summary report
generate_report() {
    log_info "Generating fuzzing summary..."
    
    echo ""
    echo "======================================"
    echo "        FUZZING SUMMARY REPORT        "
    echo "======================================"
    echo ""
    
    local total_crashes=0
    
    for target_spec in "${FUZZ_TARGETS[@]}"; do
        IFS=':' read -r target _ <<< "$target_spec"
        local crash_dir="$FUZZ_DIR/fuzz/artifacts/$target"

        echo "Target: $target"

        if [ -d "$crash_dir" ] && [ "$(ls -A $crash_dir 2>/dev/null)" ]; then
            local count=$(ls -1 "$crash_dir" | wc -l)
            echo "  Status: ❌ FAILED ($count crashes)"
            total_crashes=$((total_crashes + count))

            echo "  Crashes:"
            ls -1 "$crash_dir" | head -5 | sed 's/^/    - /'

            if [ "$count" -gt 5 ]; then
                echo "    ... and $((count - 5)) more"
            fi
        else
            echo "  Status: ✅ PASSED"
        fi

        # Corpus size
        local corpus_dir="$FUZZ_DIR/fuzz/corpus/$target"
        if [ -d "$corpus_dir" ]; then
            local corpus_count=$(ls -1 "$corpus_dir" 2>/dev/null | wc -l)
            echo "  Corpus: $corpus_count test cases"
        fi
        
        echo ""
    done
    
    echo "======================================"
    echo "Total crashes found: $total_crashes"
    echo "======================================"
    echo ""
    
    if [ $total_crashes -eq 0 ]; then
        log_success "All fuzzing tests passed! 🎉"
        return 0
    else
        log_error "Fuzzing found $total_crashes issue(s)"
        log_info "Review crashes in $FUZZ_DIR/fuzz/artifacts/"
        log_info "Minimize with: ./run_fuzzing.sh minimize"
        return 1
    fi
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [command]

Commands:
    quick       Run quick fuzzing (5 min per target) - Default
    full        Run full fuzzing (recommended durations)
    continuous  Run continuous fuzzing until interrupted
    sanitizers  Run with address and undefined sanitizers
    coverage    Generate coverage reports
    minimize    Minimize existing crash cases
    clean       Clean up artifacts
    report      Generate summary report
    help        Show this help message

Examples:
    $0 quick              # Quick 25-minute test
    $0 full               # Full 6+ hour test
    $0 continuous         # Run indefinitely
    $0 sanitizers         # Test with sanitizers

Recommended workflow:
    1. Run 'quick' to verify setup
    2. Run 'full' for comprehensive testing
    3. Run 'minimize' if crashes found
    4. Run 'sanitizers' for memory issues
    5. Run 'coverage' to check test coverage
EOF
}

# Parse arguments and run
main() {
    case "$MODE" in
        quick)
            check_dependencies
            init_fuzzing
            run_quick
            local exit_code=$?
            generate_report
            exit $exit_code
            ;;
        full)
            check_dependencies
            init_fuzzing
            run_full
            local exit_code=$?
            generate_report
            exit $exit_code
            ;;
        continuous)
            check_dependencies
            init_fuzzing
            run_continuous
            ;;
        sanitizers)
            check_dependencies
            init_fuzzing
            run_sanitizers
            local exit_code=$?
            generate_report
            exit $exit_code
            ;;
        coverage)
            generate_coverage
            ;;
        minimize)
            minimize_crashes
            ;;
        clean)
            clean_artifacts
            ;;
        report)
            generate_report
            ;;
        help|--help|-h)
            show_usage
            exit 0
            ;;
        *)
            log_error "Unknown command: $MODE"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Run main function
main
