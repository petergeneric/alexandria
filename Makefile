.PHONY: all app rust swift bundle run clean extension extension-icons bindings

RUST_SOURCES = $(shell find crates -name '*.rs' 2>/dev/null)
SWIFT_SOURCES = $(shell find macos/Sources -name '*.swift' 2>/dev/null)

APP_BUNDLE  = target/Alexandria.app
CONTENTS    = $(APP_BUNDLE)/Contents
MACOS_DIR   = $(CONTENTS)/MacOS
RESOURCES   = $(CONTENTS)/Resources

RUST_LIB    = target/release/libalexandria_core.a
RUST_DYLIB  = target/debug/libalexandria_core.dylib
NATIVE_HOST = target/release/alexandria-native-host
SWIFT_BIN   = macos/.build/release/Alexandria
INFO_PLIST  = macos/Sources/Alexandria/Info.plist
ICON_SVG    = docs/icon.svg
ICON_ICNS   = $(RESOURCES)/AppIcon.icns
HELPERS     = $(CONTENTS)/Helpers

# Generated UniFFI binding outputs (used as Make targets)
BINDINGS_SWIFT = macos/Sources/Alexandria/alexandria_core.swift
BINDINGS_HEADER = macos/Sources/alexandria_coreFFI/alexandria_coreFFI.h

all: app

# Build the full .app bundle
app: bundle

# Step 1: Build the Rust static library and native host
rust: $(RUST_LIB) $(NATIVE_HOST)

$(RUST_LIB): $(RUST_SOURCES)
	cargo build -p alexandria-core --release

$(NATIVE_HOST): $(RUST_SOURCES)
	cargo build -p alexandria-native-host --release

# Step 1b: Build debug dylib for uniffi-bindgen (fast, shares build cache)
$(RUST_DYLIB): $(RUST_SOURCES)
	cargo build -p alexandria-core

# Step 2: Generate Swift bindings from the Rust FFI interface
bindings: $(BINDINGS_SWIFT)

$(BINDINGS_SWIFT) $(BINDINGS_HEADER): $(RUST_DYLIB)
	cargo run --bin uniffi-bindgen generate \
		--library $(RUST_DYLIB) \
		--language swift \
		--out-dir target/uniffi-swift
	cp target/uniffi-swift/alexandria_core.swift \
		macos/Sources/Alexandria/
	cp target/uniffi-swift/alexandria_coreFFI.h \
		macos/Sources/alexandria_coreFFI/
	cp target/uniffi-swift/alexandria_coreFFI.modulemap \
		macos/Sources/alexandria_coreFFI/module.modulemap

# Step 3: Build the Swift binary (depends on Rust lib + bindings)
swift: $(SWIFT_BIN)

$(SWIFT_BIN): $(RUST_LIB) $(NATIVE_HOST) $(BINDINGS_SWIFT) $(SWIFT_SOURCES)
	cd macos && swift build -c release

# Step 3: Assemble the .app bundle
bundle: $(SWIFT_BIN)
	@mkdir -p $(MACOS_DIR) $(RESOURCES) $(HELPERS)
	@# Copy binary
	cp $(SWIFT_BIN) $(MACOS_DIR)/Alexandria
	@# Copy native messaging host into Helpers
	cp $(NATIVE_HOST) $(HELPERS)/alexandria-native-host
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

EXTENSION_XPI = target/alexandria-extension.xpi
EXTENSION_SOURCES = $(wildcard extension/*.js extension/*.html extension/*.json)
EXTENSION_ICONS = extension/icon16.png extension/icon32.png extension/icon48.png extension/icon128.png
BLOCKLIST_JSON = shared/blocklist.json

# Step 4: Generate extension icons from SVG (requires rsvg-convert)
extension-icons: $(EXTENSION_ICONS)

extension/icon%.png: $(ICON_SVG)
	@if command -v rsvg-convert >/dev/null 2>&1; then \
		size=$$(echo $@ | sed 's/.*icon\([0-9]*\)\.png/\1/'); \
		rsvg-convert -w $$size -h $$size $< -o $@; \
	else \
		echo "Warning: rsvg-convert not found, creating placeholder icon"; \
		printf '\x89PNG\r\n\x1a\n' > $@; \
	fi

# Build the Firefox extension .xpi
extension: $(EXTENSION_XPI)

# Regenerate rules.js from shared blocklist when the JSON changes
extension/rules.js: $(BLOCKLIST_JSON) shared/generate_rules.py
	python3 shared/generate_rules.py

$(EXTENSION_XPI): $(EXTENSION_SOURCES) $(EXTENSION_ICONS) extension/rules.js
	@mkdir -p target
	cd extension && zip -r -FS ../$(EXTENSION_XPI) manifest.json *.js *.html *.png 2>/dev/null; true
	@echo "Built $(EXTENSION_XPI)"
	@echo ""
	@echo "To test in Firefox:"
	@echo "  1. Open about:debugging#/runtime/this-firefox"
	@echo "  2. Click 'Load Temporary Add-on'"
	@echo "  3. Select extension/manifest.json (or $(EXTENSION_XPI))"
	@echo ""
	@echo "Make sure the native host manifest is installed first:"
	@echo "  make app && open target/Alexandria.app  (installs manifest on launch)"
	@echo "  OR: ./scripts/install-native-host.sh --firefox"

# Build, kill any running instance (and native host), and launch
run: app
	@pkill -x alexandria-native-host 2>/dev/null || true
	@pkill -x Alexandria 2>/dev/null || true
	@echo "Launching Alexandria..."
	@open $(APP_BUNDLE) 2>/dev/null || { sleep 1 && open $(APP_BUNDLE); }

clean:
	cargo clean
	cd macos && swift package clean
	rm -rf $(APP_BUNDLE)
