# Freshness Check Integration

## Overview
Integrated mirror freshness checking functionality into rate-mirrors, following the Go implementation from main.go. Freshness checks run during the re-test phase for top mirrors and reorder results by package freshness in addition to speed.

## Key Changes

### 1. Configuration (`src/config.rs`)
Added three new configuration fields:
- `freshness_check: bool` - Enable/disable freshness checking (default: true)
- `ref_local_dir: String` - Path to local reference database directory (default: /var/lib/pacman/sync)
- `freshness_timeout: u64` - Timeout for freshness downloads in milliseconds (default: 15000)

### 2. Target Configurations
All pacman-based target configs now use `base_path` instead of `path_to_test`:
- **Supported targets with freshness**: archlinux, archarm, archlinuxcn, artix, blackarch, cachyos, chaotic, endeavouros, manjaro, rebornos
- **Unsupported targets** (use `path_to_test`): stdin, openbsd, arcolinux

For supported targets:
- Speed test file: `{base_path}.files`
- Freshness DB file: `{base_path}.db`

Example: `extra/os/x86_64/extra` → `extra/os/x86_64/extra.files` and `extra/os/x86_64/extra.db`

### 3. Mirror Structure (`src/mirror.rs`)
Added `base_path: Option<String>` field to the `Mirror` struct. This field:
- Contains the base path for supported mirrors (e.g., "extra/os/x86_64/extra")
- Set to `Some(...)` for pacman-based mirrors
- Set to `None` for unsupported mirrors (stdin, openbsd, arcolinux)

### 4. Freshness Module (`src/freshness.rs`)
New module providing:
- `check_mirror()` - Async function to check mirror freshness by comparing package build dates
- `FreshnessCheckResult` - Result structure containing score, packages compared, and optional error
- `PackageBuildDates` - Structure for parsed package timestamps
- Database parsing for zstd, gzip, and tar formats
- Freshness score calculation: +2 for newer packages, +1 for equal timestamps, 0 for older

### 5. Speed Test Integration (`src/speed_test.rs`)
Extended `SpeedTestResult` with freshness fields:
- `freshness_score: Option<f64>`
- `freshness_packages_compared: Option<usize>`
- `freshness_error: Option<String>`

Freshness checking integrated in re-test phase:
1. After re-testing top mirrors for speed
2. Parallel freshness checks for mirrors with `base_path`
3. Calculate fallback score for failed checks: `avg - 10% * range`
4. Sort results by: freshness score (desc) → packages compared (desc) → speed (desc)

## Workflow

1. **Initial Speed Test**: Mirrors tested for speed across countries (unchanged)

2. **Re-test Phase**:
   - Top N mirrors (default: 42) re-tested for accurate speed
   - **NEW**: Freshness checks run in parallel if `freshness_check=true`
   - Each mirror's `.db` file downloaded and compared to local reference
   - Failed checks assigned fallback score based on successful checks

3. **Final Ordering**:
   - **WITH freshness**: sorted by freshness → packages → speed
   - **WITHOUT freshness**: sorted by speed only (original behavior)

## Usage

### Default (freshness enabled):
```bash
rate-mirrors arch
```

### Disable freshness checking:
```bash
rate-mirrors --freshness-check=false arch
```

### Custom reference directory:
```bash
rate-mirrors --ref-local-dir=/custom/path arch
```

### Custom timeout:
```bash
rate-mirrors --freshness-timeout=20000 arch  # 20 seconds
```

## Dependencies Added
- `tar = "0.4"` - TAR archive parsing
- `zstd = "0.13"` - ZSTD decompression
- `flate2 = "1.0"` - GZIP decompression

## Implementation Notes

1. **Always-on for supported targets**: Freshness checks run automatically when `freshness_check=true` (default) for all mirrors that have `base_path` set.

2. **Graceful degradation**: If freshness checks fail, mirrors receive a fallback score (average minus 10% of range), ensuring they're still included but ranked appropriately.

3. **Parallel execution**: All freshness checks run concurrently using tokio async tasks to minimize latency impact.

4. **Database format support**: Handles zstd-compressed, gzip-compressed, and raw tar archives, matching pacman's database formats.

5. **CachyOS variant support**: The `base_path` system preserves CachyOS's multi-architecture support (x86_64, x86_64-v3, x86_64-v4) through its existing wrapper logic.

6. **No changes to initial filtering**: Mirror selection and completion/delay filters remain in target fetch implementations (unchanged from original).

## Testing

Build succeeds with warnings only (unused fields in other modules):
```bash
cargo check --quiet  # ✓ Success
cargo build --release --quiet  # ✓ Success
```

To test freshness functionality:
```bash
# Test with limited mirrors and freshness enabled
./target/release/rate-mirrors arch --max-jumps=2 --country-test-mirrors-per-country=5
```

## Future Enhancements

1. Cache freshness results to avoid repeated downloads
2. Add freshness score display in output comments
3. Support additional database formats (e.g., sqlite)
4. Expose freshness metadata in machine-readable output formats
