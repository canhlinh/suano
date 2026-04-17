#!/usr/bin/env bash
set -e

APP_ID="dev.lingcloud.suano"
BINDING_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/suano/"

# Stop running instance
pkill -f suano 2>/dev/null || true

# Remove binary
rm -f "$HOME/.local/bin/suano"

# Remove desktop entries
rm -f "$HOME/.local/share/applications/$APP_ID.desktop"
rm -f "$HOME/.local/share/applications/suano.desktop"
rm -f "$HOME/.config/autostart/suano.desktop"
update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true

# Remove GNOME custom keybinding
SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
CURRENT=$(gsettings get "$SCHEMA" custom-keybindings 2>/dev/null || echo "[]")
NEW=$(echo "$CURRENT" | sed "s|, '$BINDING_PATH'||g; s|'$BINDING_PATH', ||g; s|'$BINDING_PATH'||g")
gsettings set "$SCHEMA" custom-keybindings "$NEW" 2>/dev/null || true

# Remove icons
for size in 16x16 32x32 128x128 256x256 512x512; do
    rm -f "$HOME/.local/share/icons/hicolor/$size/apps/suano.png"
done
gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# Remove config
rm -rf "$HOME/.config/suano"

echo "✅ Suano uninstalled."
