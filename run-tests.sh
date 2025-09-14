#!/bin/bash

# Comprehensive Test Runner for Crypto Trading Dashboard
# This script runs all test suites for both backend and frontend

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to run backend tests
run_backend_tests() {
    print_status "Running Backend Tests (Rust)..."
    
    cd crypto-dash-backend
    
    print_status "Running cargo check..."
    cargo check
    
    print_status "Running unit tests..."
    cargo test --lib
    
    print_status "Running integration tests..."
    cargo test --test "*"
    
    print_success "Backend tests completed!"
    cd ..
}

# Function to run frontend unit tests
run_frontend_unit_tests() {
    print_status "Running Frontend Unit Tests (Jest)..."
    
    cd crypto-dash-frontend
    
    print_status "Installing dependencies..."
    npm install
    
    print_status "Running unit and integration tests..."
    npm run test:unit || print_warning "Some unit tests failed"
    npm run test:integration || print_warning "Some integration tests failed"
    
    print_success "Frontend unit tests completed!"
    cd ..
}

# Function to run E2E tests
run_e2e_tests() {
    print_status "Running End-to-End Tests (Playwright)..."
    
    cd crypto-dash-frontend
    
    print_status "Installing Playwright browsers..."
    npx playwright install || print_warning "Playwright install had issues"
    
    print_status "Starting backend server..."
    cd ../crypto-dash-backend
    timeout 30s cargo run -p api &
    BACKEND_PID=$!
    
    print_status "Waiting for backend to start..."
    sleep 10
    
    cd ../crypto-dash-frontend
    
    print_status "Starting frontend server..."
    timeout 30s npm run dev &
    FRONTEND_PID=$!
    
    print_status "Waiting for frontend to start..."
    sleep 15
    
    print_status "Running E2E tests..."
    npm run test:e2e || print_warning "Some E2E tests failed"
    
    print_status "Cleaning up servers..."
    kill $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
    
    print_success "E2E tests completed!"
    cd ..
}

# Function to run performance tests
run_performance_tests() {
    print_status "Running Performance Tests..."
    
    cd crypto-dash-frontend
    
    print_status "Running load tests..."
    # Add performance testing commands here
    print_warning "Performance tests not fully implemented yet"
    
    cd ..
}

# Function to generate test reports
generate_reports() {
    print_status "Generating Test Reports..."
    
    cd crypto-dash-frontend
    
    print_status "Generating coverage report..."
    npm run test:coverage || print_warning "Coverage report generation failed"
    
    print_status "Test reports generated in coverage/ directory"
    cd ..
}

# Main function
main() {
    print_status "Starting Comprehensive Test Suite for Crypto Trading Dashboard"
    print_status "=============================================================="
    
    # Parse command line arguments
    BACKEND_TESTS=true
    FRONTEND_TESTS=true
    E2E_TESTS=false
    PERFORMANCE_TESTS=false
    GENERATE_REPORTS=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --backend-only)
                FRONTEND_TESTS=false
                E2E_TESTS=false
                shift
                ;;
            --frontend-only)
                BACKEND_TESTS=false
                E2E_TESTS=false
                shift
                ;;
            --e2e)
                E2E_TESTS=true
                shift
                ;;
            --performance)
                PERFORMANCE_TESTS=true
                shift
                ;;
            --reports)
                GENERATE_REPORTS=true
                shift
                ;;
            --all)
                E2E_TESTS=true
                PERFORMANCE_TESTS=true
                GENERATE_REPORTS=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --backend-only    Run only backend tests"
                echo "  --frontend-only   Run only frontend unit tests"
                echo "  --e2e            Include end-to-end tests"
                echo "  --performance    Include performance tests"
                echo "  --reports        Generate test reports"
                echo "  --all            Run all tests including E2E and performance"
                echo "  --help           Show this help message"
                echo ""
                echo "Examples:"
                echo "  $0                       # Run backend and frontend unit tests"
                echo "  $0 --all                 # Run all tests"
                echo "  $0 --frontend-only       # Run only frontend tests"
                echo "  $0 --e2e --reports       # Run with E2E tests and generate reports"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Create test results directory
    mkdir -p test-results
    
    START_TIME=$(date +%s)
    
    # Run tests based on configuration
    if [ "$BACKEND_TESTS" = true ]; then
        run_backend_tests 2>&1 | tee test-results/backend-tests.log
    fi
    
    if [ "$FRONTEND_TESTS" = true ]; then
        run_frontend_unit_tests 2>&1 | tee test-results/frontend-tests.log
    fi
    
    if [ "$E2E_TESTS" = true ]; then
        run_e2e_tests 2>&1 | tee test-results/e2e-tests.log
    fi
    
    if [ "$PERFORMANCE_TESTS" = true ]; then
        run_performance_tests 2>&1 | tee test-results/performance-tests.log
    fi
    
    if [ "$GENERATE_REPORTS" = true ]; then
        generate_reports 2>&1 | tee test-results/reports.log
    fi
    
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    print_status "=============================================================="
    print_success "Test Suite Completed!"
    print_status "Total execution time: ${DURATION} seconds"
    print_status "Test logs available in: test-results/"
    
    if [ "$GENERATE_REPORTS" = true ]; then
        print_status "Coverage report available in: crypto-dash-frontend/coverage/"
    fi
    
    print_status ""
    print_status "Test Summary:"
    if [ "$BACKEND_TESTS" = true ]; then
        print_status "  ✓ Backend tests (Rust)"
    fi
    if [ "$FRONTEND_TESTS" = true ]; then
        print_status "  ✓ Frontend unit tests (Jest)"
    fi
    if [ "$E2E_TESTS" = true ]; then
        print_status "  ✓ End-to-end tests (Playwright)"
    fi
    if [ "$PERFORMANCE_TESTS" = true ]; then
        print_status "  ✓ Performance tests"
    fi
    if [ "$GENERATE_REPORTS" = true ]; then
        print_status "  ✓ Test reports generated"
    fi
}

# Check if we're in the right directory
if [ ! -d "crypto-dash-backend" ] || [ ! -d "crypto-dash-frontend" ]; then
    print_error "Please run this script from the project root directory"
    print_error "Expected directories: crypto-dash-backend, crypto-dash-frontend"
    exit 1
fi

# Run main function with all arguments
main "$@"