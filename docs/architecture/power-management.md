# Power Management

## Goal

Pause indexing when the system is in Low Power Mode to conserve battery. Resume automatically when power is restored.

## Detection Strategy

### macOS Low Power Mode

On macOS, Low Power Mode can be detected via:

1. **NSProcessInfo** (preferred): `ProcessInfo.processInfo.isLowPowerModeEnabled`
2. **IOKit**: Query power management properties

The Rust implementation will use `objc` crate bindings to call `NSProcessInfo` methods.

### Notification

macOS posts `NSProcessInfoPowerStateDidChange` when Low Power Mode toggles. The power module will:

1. Register for the notification on startup
2. Update an `AtomicBool` flag when the state changes
3. Expose `is_low_power_mode() -> bool` for the queue consumer to check

## Queue Pause/Resume Behavior

The indexing consumer thread checks the power state before processing each item:

```
loop {
    if is_low_power_mode() {
        // Sleep briefly and check again
        thread::sleep(Duration::from_secs(5));
        continue;
    }

    match queue.receiver().recv_timeout(Duration::from_secs(1)) {
        Ok(snapshot) => { /* index it */ }
        Err(Timeout) => { /* idle */ }
        Err(Disconnected) => break,
    }
}
```

Key behaviors:
- **No data loss**: Items remain in the bounded channel while paused
- **Backpressure**: If the queue fills up during a pause, new snapshots are dropped with a warning
- **Responsive resume**: 5-second polling means indexing resumes within 5 seconds of exiting Low Power Mode

## Polling vs. Notification

Initial implementation uses polling (`is_low_power_mode()` checked each iteration). Future improvement: use NSNotificationCenter observer to set an AtomicBool, avoiding repeated FFI calls.
