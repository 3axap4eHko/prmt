#!/usr/bin/env bash
set -euo pipefail

AUR_REPO_DIR="${AUR_REPO_DIR:-../aur-prmt}"
REPO="3axap4eHko/prmt"

# Get latest release version from GitHub
LATEST_VER=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v?([^\"]+)".*/\1/')

if [ -z "$LATEST_VER" ]; then
    echo "Error: unable to detect latest version"
    exit 1
fi

echo "Updating to version: $LATEST_VER"

# Update PKGBUILD version and release
sed -i "s/^pkgver=.*/pkgver=$LATEST_VER/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD

# Update checksums
updpkgsums

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Clone AUR repository if it does not exist locally
if [ ! -d "$AUR_REPO_DIR" ]; then
    echo "Cloning AUR repository..."
    git clone ssh://aur@aur.archlinux.org/prmt.git "$AUR_REPO_DIR"
fi

# Copy package files into AUR repo
echo "Copying PKGBUILD and .SRCINFO to AUR repo"
cp PKGBUILD .SRCINFO "$AUR_REPO_DIR/"

# Commit and push to AUR
cd "$AUR_REPO_DIR"
git add PKGBUILD .SRCINFO

if git diff --staged --quiet; then
    echo "No changes to commit"
    exit 0
fi

git commit -m "upgpkg: $LATEST_VER-1"
git push

echo "Push to AUR completed: $LATEST_VER"