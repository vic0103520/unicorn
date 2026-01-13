# Configuration (Debug or Release)
CONFIG ?= Debug

ifeq ($(CONFIG),Release)
	RUST_FLAGS = --release
	RUST_TARGET_DIR = target/release
else
	RUST_FLAGS =
	RUST_TARGET_DIR = target/debug
endif

# Paths
APP_NAME = unicorn
MACOS_APP_DIR = apps/macos
MACOS_ARTIFACTS_DIR = $(MACOS_APP_DIR)/UnicornCore
MACOS_BUILD_DIR = $(MACOS_APP_DIR)/build
MACOS_INSTALL_DIR = $(HOME)/Library/Input Methods
MACOS_BUNDLE = $(MACOS_INSTALL_DIR)/$(APP_NAME).app

# Absolute path for symlinking
MACOS_XCODE_BUILD_PATH = $(shell pwd)/$(MACOS_APP_DIR)/Library/Input Methods/$(CONFIG)/$(APP_NAME).app

ARTIFACTS_DIR = artifacts
TARGET_DIR = $(RUST_TARGET_DIR)
LS_REGISTER = /System/Library/Frameworks/CoreServices.framework/Versions/Current/Frameworks/LaunchServices.framework/Versions/Current/Support/lsregister

.PHONY: all build-adapter-uniffi update-artifacts-for-macos build-macos install-macos clean-macos clean logs logs-live test lint format fix

# Default
all: build-macos

# Build UniFFI adapter (Cargo handles core dependency automatically)
build-adapter-uniffi:
	@echo "🔗 Building UniFFI Adapter ($(CONFIG))..."
	cargo build $(RUST_FLAGS) -p unicorn-adapter-uniffi
	@echo "🛠️ Generating Bindings..."
	mkdir -p $(ARTIFACTS_DIR)
	cargo run $(RUST_FLAGS) -p unicorn-adapter-uniffi --bin uniffi-bindgen \
		 generate --library $(TARGET_DIR)/libunicorn_uniffi.dylib \
		 --language swift --out-dir $(ARTIFACTS_DIR) --no-format

# Update the macOS app project with latest artifacts
update-artifacts-for-macos: build-adapter-uniffi
	@echo "📦 Updating macOS App artifacts..."
	mkdir -p $(MACOS_ARTIFACTS_DIR)
	cp $(TARGET_DIR)/libunicorn_uniffi.a \
	   $(ARTIFACTS_DIR)/unicorn_uniffi.swift \
	   $(ARTIFACTS_DIR)/unicorn_uniffiFFI.h \
	   $(ARTIFACTS_DIR)/unicorn_uniffiFFI.modulemap \
	   $(MACOS_ARTIFACTS_DIR)/
	@echo "📂 Copying keymap.json..."
	cp keymap.json $(MACOS_APP_DIR)/unicorn/keymap.json

# Build the macOS App using xcodebuild
build-macos: update-artifacts-for-macos
	@echo "🍏 Building macOS App ($(CONFIG))..."
	cd $(MACOS_APP_DIR) && xcodebuild -project unicorn.xcodeproj -scheme unicorn -configuration $(CONFIG) ENABLE_APP_INTENTS_METADATA_EXTRACTION=NO $(XCODE_FLAGS) build

# Install the built app via symlink
install-macos: build-macos
	@echo "🚀 Installing to $(MACOS_INSTALL_DIR)..."
	mkdir -p "$(MACOS_INSTALL_DIR)"
	# Copy the app bundle instead of symlinking
	rm -rf "$(MACOS_BUNDLE)"
	cp -R "$(MACOS_XCODE_BUILD_PATH)" "$(MACOS_BUNDLE)"
	@echo "🔄 Registering..."
	$(LS_REGISTER) -f "$(MACOS_BUNDLE)"
	@echo "🔄 Restarting..."
	pkill -9 $(APP_NAME) || true
	@echo "✨ Done."

# Cleanup
clean-macos:
	@echo "🧹 Uninstalling..."
	pkill -9 $(APP_NAME) || true
	$(LS_REGISTER) -u "$(MACOS_BUNDLE)" || true
	rm -rf "$(MACOS_BUNDLE)"

clean:
	cargo clean
	rm -rf $(ARTIFACTS_DIR)
	rm -rf $(MACOS_ARTIFACTS_DIR)
	rm -rf $(MACOS_BUILD_DIR)
	rm -rf "$(MACOS_APP_DIR)/Library"

# Dev Tools
test:
	cargo test --workspace

lint:
	cargo fmt -- --check
	cargo clippy --workspace -- -D warnings

format:
	@echo "🎨 Formatting Rust code..."
	cargo fmt

fix: format
	cargo clippy --workspace --fix --allow-dirty

logs:
	log show --predicate 'process == "$(APP_NAME)"' --last 5m --info --debug

logs-live:
	log stream --predicate 'process == "$(APP_NAME)"' --info --debug