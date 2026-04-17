#!/usr/local/bin/bash
# Launch GNOME Maps with rmpca integration
ROOT="$HOME/Documents/rmpca-rust"
MAPS_DIR="$ROOT/gnome-maps-main"
BUILD_DIR="$MAPS_DIR/build_v2"

# Point GSettings to local schema
export GSETTINGS_SCHEMA_DIR="$MAPS_DIR/data"
# Point to local Typelib (GnomeMaps-1.0)
export GI_TYPELIB_PATH="$BUILD_DIR/lib:/usr/local/lib/girepository-1.0"
# Ensure blueprint-compiler is in path for runtime if needed (though usually only for build)
export PATH="$HOME/.local/bin:$PATH"

# Setup environment variables for rmpca-rust
export RMPCA_PATH="$HOME/rmpca-target/release/rmpca"
export RMPCA_OFFLINE_MAP="$ROOT/montreal.osm.pbf"

# Force memory backend for GSettings to allow setting values in jail
export GSETTINGS_BACKEND=memory
gsettings set org.gnome.Maps rmpca-path "$RMPCA_PATH"
gsettings set org.gnome.Maps cpp-offline-map-file "$RMPCA_OFFLINE_MAP"

echo "Launching GNOME Maps from: $BUILD_DIR/src/org.gnome.Maps"
exec "$BUILD_DIR/src/org.gnome.Maps" "$@"
