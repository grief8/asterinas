# RFC: CI Workflow Restructuring for Multi-Architecture Support

## Problem Statement

Current CI workflows (`test_asterinas.yml`, `test_osdk.yml`) contain architecture-specific logic and general testing steps tightly coupled, making it difficult to:

1.  Add new architecture targets (ARM/RISC-V)
2.  Maintain consistent testing patterns across platforms
3.  Reuse common validation logic
4.  Isolate hardware-specific configurations

## Proposed Solution

The CI workflow has been restructured into the following components:

1.  **Backend Integration Layer (`backend_integration.yml`)**: This workflow is a reusable workflow that handles all architecture-specific setup and test execution. It takes various inputs to parameterize the test environment, including:

    *   `test_id`: A unique identifier for the test being run.
    *   `auto_test`: The type of test to run (boot, syscall, or test).
    *   `release`: Whether to build in release mode.
    *   `enable_kvm`: Whether to enable KVM.
    *   `intel_tdx`: Whether to enable Intel TDX.
    *   `smp`: The number of CPUs to use.
    *   `netdev`: The network device to use.
    *   `scheme`: The memory scheme (none, microvm, or iommu).
    *   `extra_blocklists`: Additional blocklists for specific tests.
    *   `syscall_test_dir`: The directory for syscall tests.
    *   `boot_protocol`: The boot protocol to use.
    *   `runs_on`: The runner to use.
    *   `timeout_minutes`: The timeout for the test.
    *   `integration_image`: The container image for integration tests.
    *   `osdk_images`: A JSON array of container images for OSDK testing.
    *   `run_general_tests`: A boolean flag to indicate whether to run general tests (lint, compilation, unit tests).

    It defines three jobs: `general-tests` (if `run_general_tests` is true), `integration-test`, and `osdk-test`. The `integration-test` and `osdk-test` jobs depend on `general-tests` if it's enabled. The `integration-test` job runs the core tests using the provided parameters. The `osdk-test` job performs linting and unit testing for the OSDK, using a matrix of container images.

2.  **Pre-Integration Checks (`pre_integration_checks.yml`)**: This workflow contains steps that are common across all architectures and are intended to be run before the more resource-intensive integration tests. It includes linting and compilation checks. It is triggered on pull requests and pushes to the main branch.

3.  **Architecture Frontends**:

    *   `test_x86.yml`: This workflow is the entry point for standard x86\_64 testing. It uses a matrix strategy to run a variety of tests (boot, syscall, general) with different configurations, calling the `backend_integration.yml` workflow with specific parameters for each test case.
    *   `test_x86_tdx.yml`: This workflow is the entry point for Intel TDX-specific testing. It also uses a matrix strategy and calls `backend_integration.yml` with TDX-specific parameters.

    Example of how the frontends call the backend:

    ```yaml
    # test_x86.yml
    jobs:
      x86-test:
        uses: ./.github/workflows/backend_integration.yml
        strategy:
          matrix:
            test_id:
              - 'boot_mb'
              - 'syscall_debug'
              # ... other tests ...
          fail-fast: false
        with:
          test_id: ${{ matrix.test_id }}
          auto_test: ${{ startsWith(matrix.test_id, 'boot') && 'boot' || ... }}
          # ... other parameters ...
          run_general_tests: true # Example of enabling general tests
    ```

### Key Changes

-   Abstract hardware provisioning through parameterized jobs in `backend_integration.yml`.
-   General tests (lint, compilation, unit tests) are integrated into `backend_integration.yml` and controlled by the `run_general_tests` input.
-   `test_general.yml` has been renamed to `pre_integration_checks.yml` and now only includes linting and compilation.
-   Frontends (`test_x86.yml`, `test_x86_tdx.yml`) define test matrices and call the backend with appropriate parameters, including `run_general_tests`.

## Implementation Plan

1.  Phase 1: Workflow Decomposition (Completed)
    *   Extracted common logic to `pre_integration_checks.yml` (formerly `test_general.yml`)
    *   Migrated TDX-specific logic to `test_x86_tdx.yml`
    *   Created `backend_integration.yml` to handle parameterized test execution, including general tests.
    *   Created `test_x86.yml` for standard x86 testing.

2.  Phase 2: Artifact Management (Next Steps)
    *   Implement architecture-specific artifact tagging
    *   Set up cross-workflow dependency chain
    *   Add matrix strategy for multi-arch builds

3.  Phase 3: Validation & Transition (Future)
    *   Parallel run of old/new workflows
    *   Metrics comparison for regression checking
    *   Final cutover and legacy workflow removal

## Additional Considerations

1.  Environment Variables Management
    *   Centralized environment configuration
    *   Architecture-specific secret namespacing

2.  Error Handling Improvements
    *   Unified artifact collection
    *   Architecture-aware failure triage
    *   Automated bisection tooling

3.  Documentation
    *   Workflow relationship diagram
    *   Architecture porting guide
    *   Debugging checklist per platform

## Open Questions

1.  Should we maintain separate job queues for different architectures?
2.  How to handle shared resource constraints (e.g., TDX-enabled hosts)?
3.  Versioning strategy for backend integration layer?
