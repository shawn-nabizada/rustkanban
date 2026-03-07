#!/bin/sh
set -e

REPO="shawn-nabizada/rustkanban"
BINARY="rk"

get_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64) echo "rk-linux-x86_64" ;;
                aarch64) echo "rk-linux-aarch64" ;;
                *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                x86_64) echo "rk-macos-x86_64" ;;
                arm64)  echo "rk-macos-aarch64" ;;
                *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $OS (use Windows .exe from GitHub releases)" >&2
            exit 1
            ;;
    esac
}

PLATFORM="$(get_platform)"
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$PLATFORM"
CHECKSUMS_URL="https://github.com/$REPO/releases/latest/download/checksums.sha256"
INSTALL_DIR="/usr/local/bin"

echo "Downloading $BINARY for $(uname -s) $(uname -m)..."
curl -sL -o "$BINARY" "$DOWNLOAD_URL"
chmod +x "$BINARY"

echo "Verifying checksum..."
EXPECTED=$(curl -sL "$CHECKSUMS_URL" | grep "$PLATFORM" | awk '{print $1}')
if [ -n "$EXPECTED" ]; then
    ACTUAL=$(sha256sum "$BINARY" 2>/dev/null || shasum -a 256 "$BINARY" 2>/dev/null)
    ACTUAL=$(echo "$ACTUAL" | awk '{print $1}')
    if [ "$EXPECTED" != "$ACTUAL" ]; then
        echo "Checksum verification FAILED!" >&2
        echo "Expected: $EXPECTED" >&2
        echo "Actual:   $ACTUAL" >&2
        rm -f "$BINARY"
        exit 1
    fi
    echo "Checksum verified."
else
    echo "Warning: could not fetch checksums, skipping verification."
fi

if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY" "$INSTALL_DIR/$BINARY"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "$BINARY" "$INSTALL_DIR/$BINARY"
fi

echo "Installed $BINARY to $INSTALL_DIR/$BINARY"
echo "Run 'rk' to start."
