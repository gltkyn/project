# from ~/Projects/Rust/project/

# 1. Remove stale binaries you don't use
rm -f src/bin/activity.rs src/bin/panel.rs src/bin/sync.rs
rmdir src/bin 2>/dev/null || true

# 2. Make sure these files exist
ls src/main.rs src/panel.rs src/panel/page.html src/panel/style.css src/panel/app.js

# 3. Compile
cargo check --all-targets

# 4. Run
cargo run
