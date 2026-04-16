#!/bin/sh
# format-usb.sh - Format Lexar 32GB USB drive and transfer rmpca-rust folder
#
# WARNING: This will format the USB drive, erasing all data!
# Please verify you're selecting the correct drive before proceeding.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="rmpca-rust"
SOURCE_DIR="/home/drone/rmpca-rust"
MOUNT_POINT="/mnt/usb-drive"

# Functions
log_info() {
    echo -e "${BLUE}INFO:${NC} $1"
}

log_success() {
    echo -e "${GREEN}SUCCESS:${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1"
}

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  Lexar USB Drive Format & Transfer${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

print_section() {
    echo ""
    echo -e "${YELLOW}>>> $1${NC}"
    echo ""
}

# Check if running as root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        log_error "This script requires root privileges to format the USB drive."
        log_error "Please run with sudo."
        exit 1
    fi
}

# List available drives
list_drives() {
    print_section "Available Drives"

    log_info "Scanning for block devices..."

    echo -e "${YELLOW}Detected drives:${NC}"
    echo ""

    # Get list of USB/Removable drives
    lsblk -o NAME,SIZE,TYPE,MOUNTPOINT -n -l | grep -v "^loop" | while read line; do
        echo "  $line"
    done

    echo ""
    log_info "USB drives typically start with 'sd' (e.g., sdb, sdc, sdd)"
    log_info "External drives are usually NOT sda (main system drive)"
}

# Confirm drive selection
confirm_drive() {
    while true; do
        echo -e "${YELLOW}Enter the device name of your Lexar USB drive:${NC}"
        echo -e "${YELLOW}(e.g., sdb, sdc, sdd - NOT sda!)${NC}"
        echo ""
        read -p "Device [/dev/sdX]: " DRIVE_NAME

        if [ -z "$DRIVE_NAME" ]; then
            log_error "Device name cannot be empty."
            continue
        fi

        DRIVE_DEVICE="/dev/$DRIVE_NAME"

        # Verify device exists
        if [ ! -b "$DRIVE_DEVICE" ]; then
            log_error "Device $DRIVE_DEVICE not found!"
            echo ""
            continue
        fi

        # Check if device is mounted
        if lsblk -n -l -o MOUNTPOINT "$DRIVE_DEVICE" | grep -q "^/"; then
            log_warning "Device $DRIVE_DEVICE is currently mounted."
            log_info "Attempting to unmount..."

            # Find all mount points and unmount
            lsblk -n -l -o MOUNTPOINT "$DRIVE_DEVICE" | grep "^/" | while read mount_point; do
                umount "$mount_point" 2>/dev/null && log_success "Unmounted: $mount_point"
            done
        fi

        # Get drive info for confirmation
        DRIVE_SIZE=$(lsblk -b -n -o SIZE "$DRIVE_DEVICE" | head -1)
        DRIVE_SIZE_GB=$((DRIVE_SIZE / 1024 / 1024 / 1024))

        echo ""
        echo -e "${RED}═══════════════════════════════════════${NC}"
        echo -e "${RED}WARNING: You are about to format:${NC}"
        echo -e "${RED}  Device: $DRIVE_DEVICE${NC}"
        echo -e "${RED}  Size: ~${DRIVE_SIZE_GB} GB${NC}"
        echo -e "${RED}  ALL DATA ON THIS DRIVE WILL BE ERASED!${NC}"
        echo -e "${RED}═══════════════════════════════════════${NC}"
        echo ""

        read -p "Type 'YES' to confirm format: " CONFIRMATION

        if [ "$CONFIRMATION" = "YES" ]; then
            log_success "Drive selection confirmed."
            echo "$DRIVE_DEVICE"
            return 0
        else
            log_warning "Confirmation cancelled. Please try again."
            echo ""
        fi
    done
}

# Format the drive
format_drive() {
    local drive_device=$1

    print_section "Formatting USB Drive"

    log_info "Creating partition table on $drive_device..."
    parted -s "$drive_device" mklabel gpt || {
        log_error "Failed to create partition table."
        exit 1
    }

    log_info "Creating partition (32GB)..."
    parted -s "$drive_device" mkpart primary fat32 1MiB 100% || {
        log_error "Failed to create partition."
        exit 1
    }

    # Wait for device to settle
    sleep 2

    log_info "Formatting partition as exFAT (for maximum compatibility)..."
    # Use exFAT for better large file support on 32GB drive
    mkfs.exfat -F "${drive_device}1" || {
        log_error "Failed to format with exFAT. Trying FAT32..."
        mkfs.vfat -F 32 "${drive_device}1" || {
            log_error "Failed to format with FAT32."
            exit 1
        }
    }

    log_success "Drive formatted successfully"
}

# Mount the drive
mount_drive() {
    local drive_device=$1

    print_section "Mounting USB Drive"

    # Create mount point
    mkdir -p "$MOUNT_POINT"

    log_info "Mounting ${drive_device}1 to $MOUNT_POINT..."

    mount -t exfat "${drive_device}1" "$MOUNT_POINT" 2>/dev/null || {
        mount -t vfat "${drive_device}1" "$MOUNT_POINT" || {
            log_error "Failed to mount drive."
            log_error "Trying without filesystem type..."
            mount "${drive_device}1" "$MOUNT_POINT" || {
                log_error "Failed to mount drive. Please check dmesg for errors."
                exit 1
            }
        }
    }

    log_success "Drive mounted at $MOUNT_POINT"
}

# Transfer files
transfer_files() {
    print_section "Transferring Files"

    if [ ! -d "$SOURCE_DIR" ]; then
        log_error "Source directory not found: $SOURCE_DIR"
        exit 1
    fi

    # Calculate source size
    SOURCE_SIZE=$(du -sb "$SOURCE_DIR" | cut -f1)
    SOURCE_SIZE_MB=$((SOURCE_SIZE / 1024 / 1024))

    log_info "Source: $SOURCE_DIR"
    log_info "Size: ~${SOURCE_SIZE_MB} MB"
    log_info "Destination: $MOUNT_POINT"
    echo ""

    log_info "Starting file transfer (this may take several minutes)..."

    # Use rsync for efficient transfer with progress
    rsync -avh --progress "$SOURCE_DIR/" "$MOUNT_POINT/" || {
        log_error "File transfer failed. Trying with cp..."
        cp -r "$SOURCE_DIR/" "$MOUNT_POINT/" || {
            log_error "Failed to transfer files."
            exit 1
        }
    }

    log_success "File transfer completed"
}

# Verify transfer
verify_transfer() {
    print_section "Verifying Transfer"

    DEST_DIR="$MOUNT_POINT/$PROJECT_NAME"

    if [ ! -d "$DEST_DIR" ]; then
        log_error "Destination directory not found: $DEST_DIR"
        exit 1
    fi

    # Count files
    SOURCE_COUNT=$(find "$SOURCE_DIR" -type f | wc -l)
    DEST_COUNT=$(find "$DEST_DIR" -type f | wc -l)

    log_info "Source files: $SOURCE_COUNT"
    log_info "Destination files: $DEST_COUNT"

    if [ "$SOURCE_COUNT" -eq "$DEST_COUNT" ]; then
        log_success "All files transferred successfully!"
    else
        log_warning "File count mismatch: $SOURCE_COUNT source vs $DEST_COUNT destination"
    fi

    # Show disk usage
    echo ""
    log_info "Disk usage:"
    df -h "$MOUNT_POINT"
}

# Cleanup
cleanup() {
    print_section "Cleanup"

    log_info "Unmounting drive from $MOUNT_POINT..."

    umount "$MOUNT_POINT" 2>/dev/null || {
        log_warning "Could not unmount drive. It may still be in use."
    }

    log_success "Drive unmounted"
    log_info "You can now safely remove the USB drive."
}

# Main function
main() {
    print_header

    # Step 1: Check root privileges
    check_root

    # Step 2: List available drives
    list_drives

    # Step 3: Confirm drive selection
    DRIVE_DEVICE=$(confirm_drive)

    # Step 4: Format the drive
    format_drive "$DRIVE_DEVICE"

    # Step 5: Mount the drive
    mount_drive "$DRIVE_DEVICE"

    # Step 6: Transfer files
    transfer_files

    # Step 7: Verify transfer
    verify_transfer

    # Step 8: Cleanup
    read -p "Press Enter to unmount drive: "
    cleanup
}

# Error handler
handle_error() {
    echo ""
    log_error "Script interrupted or encountered an error."
    log_error "The USB drive may be in an inconsistent state."
    log_info "Please check manually with: lsblk"
    exit 1
}

# Set error handler
trap handle_error INT TERM

# Run main function
main
