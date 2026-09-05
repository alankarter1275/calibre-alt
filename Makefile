PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
APPID = app.kalam.Kalam

.PHONY: all build dev test check clean install uninstall

all: build

build:
	cargo build --release

dev:
	cargo build

test:
	cargo test

check:
	cargo check

clean:
	cargo clean

install: build
	install -d "$(DESTDIR)$(BINDIR)"
	install -m 755 target/release/kalam "$(DESTDIR)$(BINDIR)/kalam"
	install -d "$(DESTDIR)$(DATADIR)/applications"
	install -m 644 resources/$(APPID).desktop "$(DESTDIR)$(DATADIR)/applications/$(APPID).desktop"
	install -d "$(DESTDIR)$(DATADIR)/icons/hicolor/512x512/apps"
	install -m 644 assets/logo.png "$(DESTDIR)$(DATADIR)/icons/hicolor/512x512/apps/$(APPID).png"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/kalam"
	rm -f "$(DESTDIR)$(DATADIR)/applications/$(APPID).desktop"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/512x512/apps/$(APPID).png"
