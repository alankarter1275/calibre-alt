FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libgtk-4-dev \
        libadwaita-1-dev \
        libglib2.0-dev \
        libpango1.0-dev \
        libgdk-pixbuf-2.0-dev \
        libgraphene-1.0-dev \
        libwebkitgtk-6.0-dev \
        libsoup-3.0-dev \
        xvfb \
        openbox \
        x11vnc \
        novnc \
        websockify \
        dbus-x11 \
        xauth \
        x11-apps \
        adwaita-icon-theme \
        fonts-dejavu-core \
        ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
