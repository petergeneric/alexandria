![LLM Generated](../llm-generated.svg)

# Power Management

## Goal

Pause indexing when the system is in Low Power Mode to conserve battery. Resume automatically when power is restored.

## Implementation

Power management is handled in the macOS Swift app layer, not in the Rust core library. The app monitors system power state and controls when `ingest_from_store` is called.

### macOS Low Power Mode Detection

The Swift app uses `ProcessInfo.processInfo.isLowPowerModeEnabled` and observes `NSProcessInfoPowerStateDidChange` notifications to detect power state changes.

### Ingestion Control

When Low Power Mode is active, the app skips scheduled ingestion cycles. When power is restored, ingestion resumes on the next cycle.

Key behaviors:
- **No data loss**: Pending pages remain in SQLite until ingestion resumes
- **Responsive resume**: Ingestion resumes on the next scheduled cycle after exiting Low Power Mode
