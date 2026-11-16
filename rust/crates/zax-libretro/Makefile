# Builds the Zaxnes libretro core as zaxnes_libretro.<so|dll|dylib> in this folder, the name the
# libretro frontends and buildbot expect. Cargo does the work (`--profile libretro`, see the
# workspace's Cargo.toml); this file only picks the platform's file names.
#
#   make                  build for the host
#   make platform=win     the Windows names on any host (with a matching CARGO_BUILD_TARGET)
#   make clean

TARGET_NAME := zaxnes
CARGO ?= cargo
WORKSPACE := ../..

ifeq ($(platform),)
  ifeq ($(OS),Windows_NT)
    platform = win
  else
    UNAME := $(shell uname -s)
    ifeq ($(UNAME),Darwin)
      platform = osx
    else
      platform = unix
    endif
  endif
endif

ifeq ($(platform),win)
  BUILT := zax_libretro.dll
  TARGET := $(TARGET_NAME)_libretro.dll
else ifeq ($(platform),osx)
  BUILT := libzax_libretro.dylib
  TARGET := $(TARGET_NAME)_libretro.dylib
else
  BUILT := libzax_libretro.so
  TARGET := $(TARGET_NAME)_libretro.so
endif

TARGET_DIR := $(or $(CARGO_TARGET_DIR),$(WORKSPACE)/target)
ifneq ($(CARGO_BUILD_TARGET),)
  OUT := $(TARGET_DIR)/$(CARGO_BUILD_TARGET)/libretro
else
  OUT := $(TARGET_DIR)/libretro
endif

all: $(TARGET)

$(TARGET): FORCE
	cd $(WORKSPACE) && $(CARGO) build --profile libretro -p zax-libretro
	cp $(OUT)/$(BUILT) $(TARGET)

clean:
	rm -f $(TARGET)
	cd $(WORKSPACE) && $(CARGO) clean --profile libretro -p zax-libretro

FORCE:

.PHONY: all clean FORCE
