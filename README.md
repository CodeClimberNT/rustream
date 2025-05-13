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

1. First clone the vcpkg 

```bash
git clone --depth=1 https://github.com/microsoft/vcpkg
```

2. install the ffmpeg libraries
   
```bash
vcpkg install ffmpeg[all-gpl]:x64-windows
```
> [!Note]
> Check this: https://vcpkg.link/ports/ffmpeg if other flag are required when the application evolve


3. Add to the path the following:
Inside `[...]/vcpkg/installed/x64-windows` 

There should be two folders:
  - bin
  - include 

Add both folder to the system path

(Not sure is needed) Additionally set two additional system variable with the previous added path
- `FFMPEG_DIR` variable should be set with the `[...]/vcpkg/installed/x64-windows` path from before
- `FFMPEG_LIB_DIR` variable should be set with the path of the `bin` path from before
- `FFMPEG_INCLUDE_DIR` variable should be set with the path of the `include` path from before

Also download the clang library to allow the linker to properly:

It can be downloaded alongside the llvm here: https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8

> [!Warning]
> Newer version may be available, this is the link for the one used to link this project and worked, newer release should not break this step.

Download the correct version for your os
> e.g. for my windows installation i downloaded: `clang+llvm-18.1.8-x86_64-pc-windows-msvc.tar.xz`

Extract the downloaded value and put the resulting folder in a known place (e.g. I renamed it and putted it in the root directory as `C:\clang`).

At this point, as before, add the `LIBCLANG_PATH` system variable pointing to the bin folder

> In my example it is `C:\clang\bin`