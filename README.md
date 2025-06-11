![Project CI Status](https://github.com/CodeClimberNT/rustream/actions/workflows/ci.yml/badge.svg?branch=main)
# Welcome to RUSTREAM

## Multi-platform screen-casting
PDS project of an application written in rust to host and watch a streaming in rust

### Description
Using the Rust programming language, create a screencasting application capable of continuously
grabbing the content of the screen (or a portion of it) and stream it to a set of peers.

The application should fulfill the following requirements:
1. [x] Platform Support: The utility should be compatible with multiple desktop operating systems,
including Windows, macOS, and Linux.
2. [x] User Interface (UI): The utility should have an intuitive and user-friendly interface that allows
users to easily navigate through the application's features.
3. [x] Operating mode: At startup, the user will choose whether the application should operate as a
caster or as a receiver. In the latter case, the user should be able to specify the address of the
caster it should connect to.
4. [x] Selection Options: When in casting mode, the utility should allow the user to restrict the
grabbed content to a custom area.
5. [x] Hotkey Support: The utility should support customizable keyboard shortcuts for
pausing/resuming the transmission, for blanking the screen and terminating the current session.


### As a bonus, the application may also provide the following features:

6. [x] Annotation Tools: When in casting mode, the utility can activate/deactivate a transparent
layer on top of the grabbed area where annotations like shapes, arrows, text, …, can be
superimposed to the original content.
7. [x] Save Options: When in receiving mode, the utility should allow users to record the received
content to a video file.
8. [x] Multi-monitor Support: The utility should be able to recognize and handle
multiple monitors independently, allowing users to cast content from any of the connected
displays. 


## Installation
To build rustream from source you will need a different setup based on the Operating System. a common dependency is, of course, having [rust installed](https://www.rust-lang.org/tools/install), but also ffmpeg invocable from the terminal as:  `ffmpeg`

If the following do not return an error
``` bash
ffmpeg -version
```

Then your ffmpeg installation is good to go for this project!

### Windows
If you already have the basics installed (rust and ffmpeg) you're good to go!

### Linux
Using Ubuntu (debian based) as an example you will need to have a gcc linker and gui dependency to allow the different dependency of this project to work, if you have access to `apt` then you can easily run the following command

``` bash
sudo apt update
sudo apt install -y \
   ubuntu-restricted-extras \
   build-essential \
   libssl-dev \
   pkg-config \
   libx11-dev \
   libxcb1-dev \
   libxcb-render0-dev \
   libxcb-shape0-dev \
   libxcb-xfixes0-dev \
   libxcb-randr0-dev \
   libxcb-shm0-dev \
   libgtk-3-dev \
   libglib2.0-dev \
   libpango1.0-dev \
   libcairo2-dev \
   libxkbcommon-x11-0 \
   libxkbcommon-dev \
   libxkbcommon-x11-dev \
   libavcodec-extra\
   xrandr\
   ffmpeg \
   libavcodec-dev \
   libavformat-dev \
   libavutil-dev \
   libswscale-dev \
   libx264-dev \
   libx265-dev \
   libvpx-dev\
   vainfo \
   libva-dev \
   libvdpau-dev\
```

>[!Note]
> some of those library may not be required, i had to use them for wsl

>[!Warning]
> Remember to check what people online make you install before modifying you system!

If you are running a non-debian derived distro, probabily you know what you are doing and can easily compensate for the (possibly) missing dependency

## MacOS
**¯\\_(ツ)_/¯**