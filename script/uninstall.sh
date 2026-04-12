#!/usr/bin/env sh
set -eu

# Uninstalls Raijin that was installed using the install.sh script

check_remaining_installations() {
    platform="$(uname -s)"
    if [ "$platform" = "Darwin" ]; then
        # Check for any Raijin variants in /Applications
        remaining=$(ls -d /Applications/Raijin*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    else
        # Check for any Raijin variants in ~/.local
        remaining=$(ls -d "$HOME/.local/raijin"*.app 2>/dev/null | wc -l)
        [ "$remaining" -eq 0 ]
    fi
}

prompt_remove_preferences() {
    printf "Do you want to keep your Raijin preferences? [Y/n] "
    read -r response
    case "$response" in
        [nN]|[nN][oO])
            rm -rf "$HOME/.config/raijin"
            echo "Preferences removed."
            ;;
        *)
            echo "Preferences kept."
            ;;
    esac
}

main() {
    platform="$(uname -s)"
    channel="${RAIJIN_CHANNEL:-stable}"

    if [ "$platform" = "Darwin" ]; then
        platform="macos"
    elif [ "$platform" = "Linux" ]; then
        platform="linux"
    else
        echo "Unsupported platform $platform"
        exit 1
    fi

    "$platform"

    echo "Raijin has been uninstalled"
}

linux() {
    suffix=""
    if [ "$channel" != "stable" ]; then
        suffix="-$channel"
    fi

    appid=""
    db_suffix="stable"
    case "$channel" in
      stable)
        appid="dev.nyxb.Raijin"
        db_suffix="stable"
        ;;
      nightly)
        appid="dev.nyxb.Raijin-Nightly"
        db_suffix="nightly"
        ;;
      preview)
        appid="dev.nyxb.Raijin-Preview"
        db_suffix="preview"
        ;;
      dev)
        appid="dev.nyxb.Raijin-Dev"
        db_suffix="dev"
        ;;
      *)
        echo "Unknown release channel: ${channel}. Using stable app ID."
        appid="dev.nyxb.Raijin"
        db_suffix="stable"
        ;;
    esac

    # Remove the app directory
    rm -rf "$HOME/.local/raijin$suffix.app"

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/raijin"

    # Remove the .desktop file
    rm -f "$HOME/.local/share/applications/${appid}.desktop"

    # Remove the database directory for this channel
    rm -rf "$HOME/.local/share/raijin/db/0-$db_suffix"

    # Remove socket file
    rm -f "$HOME/.local/share/raijin/raijin-$db_suffix.sock"

    # Remove the entire Raijin directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/.local/share/raijin"
        prompt_remove_preferences
    fi

    rm -rf $HOME/.raijin_server
}

macos() {
    app="Raijin.app"
    db_suffix="stable"
    app_id="dev.nyxb.Raijin"
    case "$channel" in
      nightly)
        app="Raijin Nightly.app"
        db_suffix="nightly"
        app_id="dev.nyxb.Raijin-Nightly"
        ;;
      preview)
        app="Raijin Preview.app"
        db_suffix="preview"
        app_id="dev.nyxb.Raijin-Preview"
        ;;
      dev)
        app="Raijin Dev.app"
        db_suffix="dev"
        app_id="dev.nyxb.Raijin-Dev"
        ;;
    esac

    # Remove the app bundle
    if [ -d "/Applications/$app" ]; then
        rm -rf "/Applications/$app"
    fi

    # Remove the binary symlink
    rm -f "$HOME/.local/bin/raijin"

    # Remove the database directory for this channel
    rm -rf "$HOME/Library/Application Support/Raijin/db/0-$db_suffix"

    # Remove app-specific files and directories
    rm -rf "$HOME/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.ApplicationRecentDocuments/$app_id.sfl"*
    rm -rf "$HOME/Library/Caches/$app_id"
    rm -rf "$HOME/Library/HTTPStorages/$app_id"
    rm -rf "$HOME/Library/Preferences/$app_id.plist"
    rm -rf "$HOME/Library/Saved Application State/$app_id.savedState"

    # Remove the entire Raijin directory if no installations remain
    if check_remaining_installations; then
        rm -rf "$HOME/Library/Application Support/Raijin"
        rm -rf "$HOME/Library/Logs/Raijin"

        prompt_remove_preferences
    fi

    rm -rf $HOME/.raijin_server
}

main "$@"
