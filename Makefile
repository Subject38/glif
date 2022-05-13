COMMA:=,
SOURCES:=$(shell find src -iname '*.rs')

MFEK_MODULE := MFEKglif
export MFEK_ARGS := $(if $(MFEK_ARGS),$(MFEK_ARGS),'examples/Layered.ufo/glyphs/S_.rhigh.layered.glifjson')

UNAME_FIRST=$(word 1, $(shell uname -a))
DEFAULT_CARGO_FLAGS := $(shell if [[ "$(UNAME_FIRST)" =~ Linux ]]; then echo --features=sdl2-dynamic; else echo --features=sdl2-static; fi)
export CARGO_FLAGS := $(if $(CARGO_FLAGS),$(CARGO_FLAGS),$(DEFAULT_CARGO_FLAGS))
export CARGO_PROFILE := $(if $(CARGO_PROFILE),$(CARGO_PROFILE),debug)
ifneq ($(strip $(CARGO_PROFILE)),debug)
export CARGO_PROFILE_ARG := --$(CARGO_PROFILE)
endif

# Cargo flags
export RUST_LOG := $(if $(RUST_LOG),$(RUST_LOG),MFEKglif=debug$(COMMA)mfek_ipc=trace)
export RUST_BACKTRACE := $(if $(RUST_BACKTRACE),$(RUST_BACKTRACE),)

all: build

.PHONY: cargo
cargo:
	@env | grep -E 'MFEK|RUST|CARGO' &&\
	RUST_LOG="$(RUST_LOG)" RUST_BACKTRACE="$(RUST_BACKTRACE)" env cargo $(CARGO_CMD) $(CARGO_PROFILE_ARG) $(CARGO_FLAGS) $(MFEK_FLAGS)

.PHONY: clean
clean:
	cargo clean

target/$(CARGO_PROFILE)/$(MFEK_MODULE): $(SOURCES)
	$(MAKE) CARGO_CMD=build cargo

.PHONY .SILENT: build
build:
	$(MAKE) target/$(CARGO_PROFILE)/$(MFEK_MODULE)

.PHONY .SILENT: testrun
testrun:
	$(MAKE) build &&\
	target/$(CARGO_PROFILE)/$(MFEK_MODULE) $(MFEK_ARGS)

.PHONY .SILENT: echo-%
echo-%:
	@$(MAKE) -s --just-print $*

# --lzma due to upx/upx#224 (GitHub)
.PHONY: dist
dist:
	$(MAKE) CARGO_PROFILE=release build &&\
	which upx || (>&2 echo "Error: upx not installed." && exit 1) &&\
	mkdir -p target/release-upx &&\
	(upx --best --lzma -o target/release-upx/$(MFEK_MODULE) target/release/$(MFEK_MODULE) || (>&2 echo "Error: upx failed." && exit 1))

.PHONY: fmt
fmt:
	@FILES="`git ls-files | grep -E '.rs$$'`" &&\
	parallel --bar RUST_LOG=error rustfmt {} <<< "$$FILES" &&\
	cargo fmt --all -- --check &&\
	echo ✅

resources/fonts/icons.ttf:
	fontmake -u resources/fonts/$(MFEK_MODULE)IconFont.ufo -o ttf --output-path $@

.PHONY: iconfont
iconfont: resources/fonts/icons.ttf

## Macos stuff
TARGET = MFEKglif

ASSETS_DIR = resources
RELEASE_DIR = target/release

APP_NAME = MFEKglif.app
APP_TEMPLATE = $(ASSETS_DIR)/macos/$(APP_NAME)
APP_DIR = $(RELEASE_DIR)/macos
APP_BINARY = $(RELEASE_DIR)/$(TARGET)
APP_BINARY_DIR = $(APP_DIR)/$(APP_NAME)/Contents/MacOS
APP_EXTRAS_DIR = $(APP_DIR)/$(APP_NAME)/Contents/Resources

DMG_NAME = MFEKglif.dmg
DMG_DIR = $(RELEASE_DIR)/macos

vpath $(TARGET) $(RELEASE_DIR)
vpath $(APP_NAME) $(APP_DIR)
vpath $(DMG_NAME) $(APP_DIR)

all: help

help: ## Print this help message
	@grep -E '^[a-zA-Z._-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

binary: $(TARGET)-native ## Build a release binary
binary-universal: $(TARGET)-universal ## Build a universal release binary
$(TARGET)-native:
	MACOSX_DEPLOYMENT_TARGET="10.13" cargo build --profile release --features=sdl2-static
	@lipo target/release/$(TARGET) -create -output $(APP_BINARY)
$(TARGET)-universal:
	MACOSX_DEPLOYMENT_TARGET="10.13" cargo build --profile release --target=x86_64-apple-darwin --features=sdl2-static
	MACOSX_DEPLOYMENT_TARGET="10.13" cargo build --profile release --target=aarch64-apple-darwin --features=sdl2-static
	@lipo target/{x86_64,aarch64}-apple-darwin/release/$(TARGET) -create -output $(APP_BINARY)
	/usr/bin/codesign -vvv --deep --strict --options=runtime --force -s 8796B41A953882B21E3E57B7597E00B9CCCDAA38 $(APP_BINARY)

app: $(APP_NAME)-native ## Create a MFEKglif.app
app-universal: $(APP_NAME)-universal ## Create a universal MFEKglif.app
$(APP_NAME)-%: $(TARGET)-%
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_EXTRAS_DIR)
	@cp -fRp $(APP_TEMPLATE) $(APP_DIR)
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@touch -r "$(APP_BINARY)" "$(APP_DIR)/$(APP_NAME)"
	@echo "Created '$(APP_NAME)' in '$(APP_DIR)'"
	xattr -c $(APP_DIR)/$(APP_NAME)/Contents/Info.plist
	xattr -c $(APP_DIR)/$(APP_NAME)/Contents/Resources/MFEKglif.icns
	/usr/bin/codesign -vvv --deep --strict --options=runtime --force -s 8796B41A953882B21E3E57B7597E00B9CCCDAA38 $(APP_DIR)/$(APP_NAME)

dmg: $(DMG_NAME)-native ## Create a MFEKglif.dmg
dmg-universal: $(DMG_NAME)-universal ## Create a universal MFEKglif.dmg
$(DMG_NAME)-%: $(APP_NAME)-%
	@echo "Packing disk image..."
	@ln -sf /Applications $(DMG_DIR)/Applications
	@hdiutil create $(DMG_DIR)/$(DMG_NAME) \
		-volname "MFEKglif" \
		-fs HFS+ \
		-srcfolder $(APP_DIR) \
		-ov -format UDZO
	@echo "Packed '$(APP_NAME)' in '$(APP_DIR)'"
	/usr/bin/codesign -vvv --deep  --strict --options=runtime --force -s 8796B41A953882B21E3E57B7597E00B9CCCDAA38 $(DMG_DIR)/$(DMG_NAME)

install: $(INSTALL)-native ## Mount disk image
install-universal: $(INSTALL)-native ## Mount universal disk image
$(INSTALL)-%: $(DMG_NAME)-%
	@open $(DMG_DIR)/$(DMG_NAME)

.PHONY: app binary clean dmg install $(TARGET) $(TARGET)-universal
