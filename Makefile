.PHONY: all app rust swift bundle run clean

RUST_SOURCES = $(shell find crates -name '*.rs' 2>/dev/null)
SWIFT_SOURCES = $(shell find alexandria-app/Sources -name '*.swift' 2>/dev/null)

APP_BUNDLE  = target/Alexandria.app
CONTENTS    = $(APP_BUNDLE)/Contents
MACOS_DIR   = $(CONTENTS)/MacOS
RESOURCES   = $(CONTENTS)/Resources

RUST_LIB    = target/release/libalexandria_core.a
SWIFT_BIN   = alexandria-app/.build/release/Alexandria
INFO_PLIST  = alexandria-app/Sources/Alexandria/Info.plist
ICON_SVG    = docs/icon.svg
ICON_ICNS   = $(RESOURCES)/AppIcon.icns

all: app

# Build the full .app bundle
app: bundle

# Step 1: Build the Rust static library
rust: $(RUST_LIB)

$(RUST_LIB): $(RUST_SOURCES)
	cargo build -p alexandria-core --release

# Step 2: Build the Swift binary (depends on Rust lib)
swift: $(SWIFT_BIN)

$(SWIFT_BIN): $(RUST_LIB) $(SWIFT_SOURCES)
	cd alexandria-app && swift build -c release

# Step 3: Assemble the .app bundle
bundle: $(SWIFT_BIN)
	@mkdir -p $(MACOS_DIR) $(RESOURCES)
	@# Copy binary
	cp $(SWIFT_BIN) $(MACOS_DIR)/Alexandria
	@# Copy Info.plist
	cp $(INFO_PLIST) $(CONTENTS)/Info.plist
	@# Generate .icns from SVG if rsvg-convert and iconutil are available
	@if command -v rsvg-convert >/dev/null 2>&1; then \
		set -e; \
		ICONSET=$$(mktemp -d)/Alexandria.iconset; \
		mkdir -p "$$ICONSET"; \
		for size in 16 32 128 256 512; do \
			rsvg-convert -w $$size -h $$size $(ICON_SVG) -o "$$ICONSET/icon_$${size}x$${size}.png"; \
			double=$$((size * 2)); \
			rsvg-convert -w $$double -h $$double $(ICON_SVG) -o "$$ICONSET/icon_$${size}x$${size}@2x.png"; \
		done; \
		iconutil -c icns "$$ICONSET" -o $(ICON_ICNS); \
		rm -rf "$$(dirname "$$ICONSET")"; \
		echo "App icon generated"; \
	else \
		echo "Note: install rsvg-convert (librsvg) to generate the app icon from SVG"; \
	fi
	@echo "Built $(APP_BUNDLE)"

# Build, kill any running instance, and launch
run: app
	@pkill -x Alexandria 2>/dev/null || true
	@echo "Launching Alexandria..."
	@open $(APP_BUNDLE)

clean:
	cargo clean
	cd alexandria-app && swift package clean
	rm -rf $(APP_BUNDLE)
