#!/bin/bash

# This script executes several automated tests on Burn-My-Windows. To do this, it is
# installed in a fedora-based container running GNOME Shell on xvfb. The used container is
# hosted on Github: https://github.com/Schneegans/gnome-shell-pod. This scripts installs
# Burn-My-Windows from the burn-my-windows@schneegans.github.com.zip file which is
# expected to be present in the repository root. Therefore you have to call "make" before
# this script.
#
# The scripts supports two arguments:
#
# -v fedora_version: This determines the version of GNOME Shell to test agains.
#                    -v 39: GNOME Shell 45
#                    -v rawhide: The current GNOME Shell version of Fedora Rawhide
# -s session:        This can either be "gnome-xsession" or "gnome-wayland-nested".


FEDORA_VERSION=41
SESSION="gnome-wayland-nested"



# This function is used below to execute any shell command inside the running container.
do_in_pod() {
  podman exec --user gnomeshell --workdir /home/gnomeshell "${POD}" set-env.sh "$@"
}

# This is called whenever a test fails. It prints an error message (given as second
# parameter), saves a current screenshot, the current cropped target and a log to
# "tests/output/".
fail() {
  echo "${2}"
  mkdir -p "tests/output"
  mv "${WORK_DIR}/screen.png" "tests/output/${1}"
  LOG=$(do_in_pod sudo journalctl)
  echo "${LOG}" > tests/output/fail.log
  exit 1
}

# This searches the virtual screen of the container for a given target image (first
# parameter). If it is not found, an error message (second paramter) is printed and the
# script exits via the fail() method above.
find_target() {
  echo "Searching for ${1} on the screen."

  podman cp "${POD}:/opt/Xvfb_screen0" - | tar xf - --to-command "convert xwd:- ${WORK_DIR}/screen.png"

  POS=$(./tests/find-target.sh "${WORK_DIR}/screen.png" "tests/references/${1}") || true

  if [[ -z "${POS}" ]]; then
    fail "${1}" "${2}"
  fi
}

  do_in_pod gsettings set org.gnome.shell welcome-dialog-last-shown-version "999" || true
fi

# Make sure that new windows are opened in the center.
do_in_pod gsettings set org.gnome.mutter center-new-windows true

echo "Starting $(do_in_pod gnome-shell --version)."
do_in_pod systemctl --user start "${SESSION}@:99"
sleep 10

# Enable the extension.
do_in_pod gnome-extensions enable "${EXTENSION}"

# Starting with GNOME 40, the overview is the default mode. We close this here by hitting
# the super key.
if [[ "${FEDORA_VERSION}" -gt 33 ]] || [[ "${FEDORA_VERSION}" == "rawhide" ]]; then
  echo "Closing Overview."
  send_keystroke "super"
fi

# Wait until the extension is enabled and the overview closed.
sleep 3

# ---------------------------------------------------------------------- perform the tests

# First we open the preferences and check whether the window is shown on screen by
# searching for a small snippet of the preferences dialog.
echo "Opening Preferences."
do_in_pod gnome-extensions prefs "${EXTENSION}"
sleep 10
find_target "preferences-${SESSION}-${FEDORA_VERSION}.png" "Failed to open preferences!"
send_keystroke "Alt+F4"

# The test mode ensures that the animations are "frozen" and do not change in time.
echo "Entering test mode."
set_setting "test-mode" true

test_effect "energize-a"
test_effect "energize-b"
test_effect "fire"
test_effect "focus"
test_effect "glide"
test_effect "glitch"
test_effect "hexagon"
test_effect "incinerate"
test_effect "pixelate"
test_effect "pixel-wheel"
test_effect "pixel-wipe"
test_effect "portal"
test_effect "tv"
test_effect "tv-glitch"
test_effect "wisps"

if [[ "${FEDORA_VERSION}" -gt 32 ]] || [[ "${FEDORA_VERSION}" == "rawhide" ]]; then
  test_effect "apparition"
  test_effect "doom"
fi

if [[ "${FEDORA_VERSION}" -gt 33 ]] || [[ "${FEDORA_VERSION}" == "rawhide" ]]; then
  test_effect "trex"
  test_effect "broken-glass"
  test_effect "matrix"
  test_effect "snap"
fi

echo "All tests executed successfully."
