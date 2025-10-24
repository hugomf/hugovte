#!/bin/bash
# Benchmark helper script

set -e

BASELINE_DIR="target/criterion"
REPORT_DIR="benchmark-reports"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_usage() {
    cat << EOF
Usage: $0 [command]

Commands:
    run             Run all benchmarks
    baseline        Save current performance as baseline
    compare NAME    Compare current performance against baseline NAME
    report          Open HTML report in browser
    clean           Clean benchmark data
    quick           Run quick benchmarks (fewer samples)
    help            Show this help message

Examples:
    $0 run                    # Run all benchmarks
    $0 baseline main          # Save baseline as 'main'
    $0 compare main           # Compare against 'main' baseline
    $0 quick                  # Quick benchmark run

EOF
}

run_benchmarks() {
    echo -e "${GREEN}Running benchmarks...${NC}"
    cargo bench "$@"
    echo -e "${GREEN}Benchmarks complete!${NC}"
    echo -e "View HTML reports: ${YELLOW}target/criterion/report/index.html${NC}"
}

save_baseline() {
    local name="${1:-baseline}"
    echo -e "${GREEN}Saving baseline as '${name}'...${NC}"
    cargo bench -- --save-baseline "$name"
    echo -e "${GREEN}Baseline '${name}' saved!${NC}"
}

compare_baseline() {
    local name="${1:-baseline}"
    if [ ! -d "${BASELINE_DIR}/${name}" ]; then
        echo -e "${RED}Error: Baseline '${name}' not found${NC}"
        echo "Available baselines:"
        ls -1 "${BASELINE_DIR}" 2>/dev/null | grep -v "^report$" || echo "  (none)"
        exit 1
    fi
    
    echo -e "${GREEN}Comparing against baseline '${name}'...${NC}"
    cargo bench -- --baseline "$name"
    echo -e "${GREEN}Comparison complete!${NC}"
}

open_report() {
    local report="${BASELINE_DIR}/report/index.html"
    if [ ! -f "$report" ]; then
        echo -e "${RED}Error: Report not found. Run benchmarks first.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Opening benchmark report...${NC}"
    
    # Try to open in browser (cross-platform)
    if command -v xdg-open > /dev/null; then
        xdg-open "$report"
    elif command -v open > /dev/null; then
        open "$report"
    elif command -v start > /dev/null; then
        start "$report"
    else
        echo -e "${YELLOW}Please open manually: $report${NC}"
    fi
}

clean_benchmarks() {
    echo -e "${YELLOW}Cleaning benchmark data...${NC}"
    rm -rf "${BASELINE_DIR}"
    echo -e "${GREEN}Benchmark data cleaned!${NC}"
}

quick_bench() {
    echo -e "${GREEN}Running quick benchmarks (10 samples, 1s warm-up)...${NC}"
    cargo bench -- --quick
    echo -e "${GREEN}Quick benchmarks complete!${NC}"
}

# Main command handling
case "${1:-run}" in
    run)
        shift
        run_benchmarks "$@"
        ;;
    baseline)
        save_baseline "$2"
        ;;
    compare)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: Please specify baseline name${NC}"
            print_usage
            exit 1
        fi
        compare_baseline "$2"
        ;;
    report)
        open_report
        ;;
    clean)
        clean_benchmarks
        ;;
    quick)
        quick_bench
        ;;
    help|--help|-h)
        print_usage
        ;;
    *)
        echo -e "${RED}Error: Unknown command '$1'${NC}"
        print_usage
        exit 1
        ;;
esac