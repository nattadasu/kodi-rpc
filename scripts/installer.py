#!/usr/bin/env python3

# Thanks to https://github.com/pogmommy for making the original macOS installer
# Thanks to https://github.com/xenoncolt for making the original Windows installer
# Their contributions made this universal script a lot easier to produce.

import json
import os
import subprocess
import platform
from time import sleep
import sys


# customizable confirm prompt
def confirm(message: str = "Continue", default: bool | None = None, direct: bool | None = None) -> bool:
    prompts = {True: "(Y/n)", False: "(y/N)", None: "(y/n)"}
    full_message = f"{message} {prompts[default]}: "

    valid_inputs = {"y": True, "yes": True, "n": False, "no": False}
    if default is not None:
        valid_inputs[""] = default

    while (response := input(full_message).strip().lower()) not in valid_inputs:
        print("Invalid input, please type y or n")

    output = valid_inputs[response]
    if direct is not None and not output:
        return None
    return output


path = ""

if platform.system() != "Windows":
    if os.environ.get("XDG_CONFIG_HOME"):
        path = os.environ["XDG_CONFIG_HOME"].removesuffix("/") + "/kodi-rpc/"
    else:
        path = os.environ["HOME"].removesuffix("/") + "/.config/kodi-rpc/"

    subprocess.run(["mkdir", "-p", path])
else:
    path = os.environ["APPDATA"].removesuffix("\\") + "\\kodi-rpc\\"
    subprocess.run(
        ["powershell", "-NoProfile", "-Command", f'mkdir "{path}"'],
        stdout=subprocess.DEVNULL,
    )

print("""
Welcome to the Kodi-RPC installer
[https://github.com/nattadasu/kodi-rpc#Setup]
""")

config_path = path + "main.json"
use_existing = False

if os.path.isfile(config_path):
    print(f"Found existing config: {config_path}")

    if "--use-existing-config" in sys.argv:
        print("Using existing config")
        use_existing = True

    else:
        use_existing = confirm(message="Use existing config?", default=False)

if not use_existing:
    print("----------Kodi----------")
    url = input("Kodi URL (include http/https, e.g. http://localhost:8080): ")
    print("Kodi HTTP auth (Settings -> Services -> Control). Leave blank for none.")
    username = input("username [Default: kodi]: ")
    if username == "":
        username = "kodi"
    import getpass
    password = getpass.getpass("password (blank for none): ")

    self_signed_cert = None
    if url.startswith("https://"):
        self_signed_cert = confirm(
            message="Are you using a self signed certificate?", default=False, direct=True
        )

    print(
        "If you dont want anything else you can just press enter through all of these"
    )

    music = confirm(message="Do you want to customize music display?", default=False)
    if music:
        print("Enter what you would like to be shown in a comma seperated list")
        print("Remember that it will show in the order you type it in")
        print("Valid options are year, album and/or genres")
        display = input("[Default: genres]: ").split(",")

        print("Choose the separator between the artist name and the info")
        separator = input("[Default: -]: ")

        if display == "":
            display = None
        if separator == "":
            separator = None

        music = {"display": display, "separator": separator}
    else:
        music = None

    movies = confirm(message="Do you want to customize movie display?", default=False)
    if movies:
        print("Enter what you would like to be shown in a comma seperated list")
        print("Remember that it will show in the order you type it in")
        print("Valid options are year, critic-score, community-score and/or genres")
        display = input("[Default: genres]: ").split(",")

        print("Choose the separator between the artist name and the info")
        separator = input("[Default: -]: ")

        if display == "":
            display = None
        if separator == "":
            separator = None

        movies = {"display": display, "separator": separator}
    else:
        movies = None

    blacklist = confirm(
        message="Do you want to blacklist media types or libraries?", default=False
    )
    if blacklist:
        print(
            "You will first type in what media types to blacklist, this should be a comma separated list WITHOUT SPACES"
        )
        print(
            "then after that you can choose what libraries to blacklist, this should ALSO be a comma separated list,"
        )
        print(
            "there should be no spaces before or after the commas but there can be spaces in the names of libraries"
        )
        sleep(2)

        print("Media types 1/2")
        media_types = input(
            "Valid types are music, movie, episode, livetv and/or unknown [Default: ]: "
        ).split(",")

        print("Libraries 2/2 (matched as substrings against the Kodi file path, e.g. plugin.video.foo)")
        libraries = input("Enter path substrings to blacklist [Default: ]: ").split(",")

        blacklist = {"media_types": media_types, "libraries": libraries}
    else:
        blacklist = None

    show_simple = not confirm(
        message="Do you want to see episode names in the activity?", default=True, direct=None
    )

    append_prefix = confirm(
        "Do you want to add a leading 0 to season and episode numbers?", default=False, direct=True
    )

    add_divider = confirm(
        "Do you want to add a divider between numbers, ex. S01 - E01?", default=False, direct=True
    )

    kodi = {
        "url": url,
        "username": username,
        "password": password,
        "music": music,
        "movies": movies,
        "blacklist": blacklist,
        "self_signed_cert": self_signed_cert,
        "show_simple": show_simple,
        "append_prefix": append_prefix,
        "add_divider": add_divider,
    }

    print("----------Discord----------")

    appid = input("Enter your discord application ID [Default: 1053747938519679018]: ")
    if appid == "":
        appid = None

    show_paused = confirm(message="Do you want to show paused videos?", default=True, direct=True)

    print("----------Buttons----------")

    buttons = confirm(message="Do you want custom buttons?", default=False)
    if buttons:
        buttons = []

        print(
            "If you want one button to continue being dynamic then you have to specifically enter dynamic into both name and url fields"
        )
        print(
            "If you dont want any buttons to appear then you can leave everything blank here and it wont show anything."
        )

        print("Button 1/2")
        name = input("Choose what the button will show [Default: dynamic]: ")
        url = input("Choose where the button will direct to [Default: dynamic]: ")

        if name != "" and url != "":
            buttons.append({"name": name, "url": url})

        print("Button 2/2")
        name = input("Choose what the button will show [Default: dynamic]: ")
        url = input("Choose where the button will direct to [Default: dynamic]: ")

        if name != "" and url != "":
            buttons.append({"name": name, "url": url})
    else:
        buttons = None

    print("----------Images----------")
    images = confirm(message="Do you want images?", default=False)
    if images:
        imgur_images = confirm("Do you want to use imgur for images?", default=False, direct=True)
        litterbox_images = False
        if imgur_images:
            client_id = input("Enter your imgur client id: ")
            imgur = {"client_id": client_id}
        else:
            litterbox_images = confirm("Do you want to use litterbox for images?", default=False, direct=True)
            imgur = None

        images = {
            "enable_images": True,
            "imgur_images": imgur_images,
            "litterbox_images": litterbox_images,
        }
    else:
        imgur = None
        images = None

    discord = {"application_id": appid, "buttons": buttons, "show_paused": show_paused}

    config = {
        "kodi": kodi,
        "discord": discord,
        "imgur": imgur,
        "images": images,
    }

    print(f"\nPlacing config in '{path}'")

    file = open(config_path, "w")
    file.write(json.dumps(config, indent=2))
    file.close()

if "--no-install" in sys.argv:
    print("Skipping installation")
    exit(0)

continue_setup = confirm(message="Do you want to download Kodi-RPC?", default=True)
if not continue_setup:
    print("Exiting...")
    exit(0)

print("\nDownloading Kodi-RPC")

if platform.system() == "Windows":
    subprocess.run(
        [
            "curl",
            "-o",
            path + "kodi-rpc.exe",
            "-L",
            "https://github.com/nattadasu/kodi-rpc/releases/latest/download/kodi-rpc.exe",
        ]
    )

    autostart = confirm(
        message="Do you want to autostart Kodi-RPC at login?", default=False
    )
    if autostart:
        if os.path.isfile(path + "winsw.exe"):
            print(
                "The script will prompt for administrator to remove the already installed service"
            )
            sleep(1)
            subprocess.run([path + "winsw.exe", "uninstall"])

        subprocess.run(
            [
                "curl",
                "-o",
                path + "winsw.exe",
                "-L",
                "https://github.com/winsw/winsw/releases/latest/download/WinSW-x64.exe",
            ]
        )

        content = f"""<service>
    <id>kodi-rpc</id>
    <name>Kodi-RPC</name>
    <description>This service is running Kodi-RPC for rich presence support</description>
    <executable>{path}kodi-rpc.exe</executable>
    <arguments>-c {path}main.json -i {path}urls.json</arguments>
</service>"""

        file = open(path + "winsw.xml", "w")
        file.write(content)
        file.close()

        print(
            "The program will now ask you for administrator rights twice, this is so the service can be installed!"
        )
        print("waiting 5 seconds")
        sleep(5)

        subprocess.run([path + "winsw.exe", "install"])
        subprocess.run([path + "winsw.exe", "start"])

        print(
            "Autostart has been set up, kodi-rpc should now launch at login\nas long as there are no issues with the configuration"
        )


elif platform.system() == "Darwin":
    file = f"https://github.com/nattadasu/kodi-rpc/releases/latest/download/kodi-rpc-{platform.machine()}-darwin"
    subprocess.run(["curl", "-o", "/usr/local/bin/kodi-rpc", "-L", file])
    subprocess.run(["chmod", "+x", "/usr/local/bin/kodi-rpc"])

    autostart = confirm(
        message="Do you want to autostart Kodi-RPC at login?", default=False
    )
    if autostart:
        if subprocess.run(["pgrep", "-xq", "--", "'kodi-rpc'"]).returncode == 0:
            subprocess.run(["killall", "kodi-rpc"])

        if (
            "Kodi-RPC"
            in subprocess.Popen("launchctl list", shell=True, stdout=subprocess.PIPE)
            .stdout.read()
            .decode()
        ):
            subprocess.run(["launchctl", "remove", "Kodi-RPC"])

        content = """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>Kodi-RPC</string>
        <key>Program</key>
        <string>/usr/local/bin/kodi-rpc</string>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardErrorPath</key>
        <string>/tmp/kodirpc.local.stderr.txt</string>
        <key>StandardOutPath</key>
        <string>/tmp/kodirpc.local.stdout.txt</string>
    </dict>
</plist>"""

        path = os.environ["HOME"] + "/Library/LaunchAgents/kodirpc.local.plist"

        file = open(path, "w")
        file.write(content)
        file.close()

        subprocess.run(["chmod", "644", path])
        subprocess.run(["launchctl", "load", path])

        print("Kodi RPC is now set up to start at login.")
        print(
            "If needed, you can run Kodi RPC at any time by running 'kodi-rpc' in a terminal."
        )

else:
    # If ARM64
    if "aarch64" in platform.machine().lower() or "armv8" in platform.machine().lower():
        linux_binary = "kodi-rpc-arm64-linux"
    # Else If ARM32
    elif "aarch" in platform.machine().lower() or "arm" in platform.machine().lower():
        linux_binary = "kodi-rpc-arm32-linux"
    else:
        linux_binary = "kodi-rpc-x86_64-linux"

    subprocess.run(
        ["mkdir", "-p", os.environ["HOME"].removesuffix("/") + "/.local/bin"]
    )
    subprocess.run(
        [
            "curl",
            "-o",
            os.environ["HOME"].removesuffix("/") + "/.local/bin/kodi-rpc",
            "-L",
            f"https://github.com/nattadasu/kodi-rpc/releases/latest/download/{linux_binary}",
        ]
    )
    subprocess.run(
        [
            "chmod",
            "+x",
            os.environ["HOME"].removesuffix("/") + "/.local/bin/kodi-rpc",
        ]
    )

    if os.environ.get("XDG_CONFIG_HOME"):
        path = (
            os.environ["XDG_CONFIG_HOME"].removesuffix("/")
            + "/systemd/user/kodi-rpc.service"
        )
    else:
        path = (
            os.environ["HOME"].removesuffix("/")
            + "/.config/systemd/user/kodi-rpc.service"
        )

    autostart = confirm(
        message="Do you want to autostart Kodi-RPC at login using systemd?", default=False
    )
    if autostart:
        print(f"\nSetting up service file in {path}")

        subprocess.run(["mkdir", "-p", path.removesuffix("kodi-rpc.service")])

        content = f"""[Unit]
Description=Kodi-RPC Service
Documentation=https://github.com/nattadasu/kodi-rpc
After=network.target

[Service]
Type=simple
ExecStart={os.environ["HOME"].removesuffix("/") + "/.local/bin/kodi-rpc"}
Restart=on-failure

[Install]
WantedBy=default.target"""

        file = open(path, "w")
        file.write(content)
        file.close()

        subprocess.run(["systemctl", "--user", "daemon-reload"])
        subprocess.run(
            ["systemctl", "--user", "enable", "--now", "kodi-rpc.service"]
        )

        print("Kodi-RPC is now set up to start at login.")

print("Installation complete!")
