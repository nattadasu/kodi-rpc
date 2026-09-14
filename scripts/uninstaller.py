import platform
import os
import subprocess
from time import sleep
import shutil

# Thanks to https://github.com/pogmommy for making the original macOS uninstaller
# Thanks to https://github.com/xenoncolt for making the original Windows uninstaller

print("Welcome to the Kodi-RPC uninstaller")
input("Hit enter to continue...")

if platform.system() != "Windows":
    if os.environ.get("XDG_CONFIG_HOME"):
        path = os.environ["XDG_CONFIG_HOME"].removesuffix("/") + "/kodi-rpc/"
    else:
        path = os.environ["HOME"].removesuffix("/") + "/.config/kodi-rpc/"
else:
    path = os.environ["APPDATA"].removesuffix("\\") + "\\kodi-rpc\\"

if platform.system() == "Windows":
    if os.path.isfile(path + "winsw.exe"):
        print("The script will ask for admin rights to remove the autostart service")
        print("waiting 5 seconds")
        sleep(5)
        subprocess.run([path + "winsw.exe", "uninstall"])

    shutil.rmtree(path)
elif platform.system() == "Darwin":
    if subprocess.run(["pgrep", "-xq", "--", "'kodi-rpc'"]).returncode == 0:
        subprocess.run(["killall", "kodi-rpc"])

    if "Kodi-RPC" in subprocess.Popen("launchctl list", shell=True, stdout=subprocess.PIPE).stdout.read().decode():
        subprocess.run(["launchctl", "remove", "Kodi-RPC"])

    servicepath = os.environ["HOME"].removesuffix("/") + "/Library/LaunchAgents/kodirpc.local.plist"
    if os.path.isfile(servicepath):
        os.remove(servicepath)
    shutil.rmtree(path)
    os.remove("/usr/local/bin/kodi-rpc")
else:
    if "kodi-rpc.service" in subprocess.Popen("systemctl --user list-units", shell=True, stdout=subprocess.PIPE).stdout.read().decode():
        subprocess.run(["systemctl", "--user", "disable", "--now", "kodi-rpc.service"])

    if subprocess.run(["pgrep", "-xq", "--", "'kodi-rpc'"]).returncode == 0:
        subprocess.run(["killall", "kodi-rpc"])

    servicepath = path.removesuffix("kodi-rpc/") + "systemd/user/kodi-rpc.service"
    if os.path.isfile(servicepath):
        subprocess.run(["rm", servicepath])
    shutil.rmtree(path)
    os.remove(os.environ["HOME"].removesuffix("/") + "/.local/bin/kodi-rpc")

print("Uninstall complete!")
sleep(5)
