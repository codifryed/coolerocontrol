[![GPLv3 License](https://img.shields.io/badge/License-GPL%20v3-blue.svg?logo=gnu)](https://opensource.org/licenses/)
[![Gitlab pipeline status](https://img.shields.io/gitlab/pipeline-status/codifryed/coolero?branch=main&label=pipeline&logo=gitlab)](https://gitlab.com/codifryed/coolero/pipelines)
[![GitLab Release (latest by SemVer)](https://img.shields.io/gitlab/v/release/30707566?sort=semver&logo=gitlab)](https://gitlab.com/codifryed/coolero/pipelines)
[![Discord](https://img.shields.io/discord/908873022105079848?&logo=discord)](https://discord.gg/MbcgUFAfhV)

# Coolero

is a program to monitor and control your cooling devices.  
It uses libraries like
[liquidctl](https://github.com/liquidctl/liquidctl) and others with a focus on cooling control under Linux.  
Written in [Python](https://www.python.org/) it uses [PySide](https://wiki.qt.io/Qt_for_Python) for the UI
and [Poetry](https://python-poetry.org/) for dependency management.

This project is currently in active development and is slowly working it's way towards an stable release.  
Testers welcome!

## Screenshots

![Open Overview](screenshots/open-overview.png)
![Speed Channel](screenshots/speed-channel.png)
![Overview Customer Profile](screenshots/overview-custom-profile.png)

## Supported Devices:

*more comming!*

| Name | Cooling | Lighting | Notes |
|------|---------|----------|-------|
| NZXT Kraken X53, X63, X73 | X |  | |

## Installation

### Binary:

Goto the [Releases](https://gitlab.com/codifryed/coolero/-/releases) page and download the latest binary.  
The binary is a self-packaged application. Just make it executable and run it.

```bash
chmod +x coolero.bin
./coolero.bin
```

### Flatpak: (WIP)

Goto the [Releases](https://gitlab.com/codifryed/coolero/-/releases) page and download the lastest flatpak.  
Install with: ```flatpak install coolero.flatpak```  
**no official release on the flathub repo, in progress*

### Snap: (WIP)

Goto the [Releases](https://gitlab.com/codifryed/coolero/-/releases) page and download the latest snap.  
Install with: ```snap install coolero_*_amd64.snap --classic --dangerous```  
**--dangerous needed until officially added to the snap store*

### From Source:

#### Requirements:

* Linux
* [Python 3.9](https://www.python.org/)
    * including the python3.9-dev package (may already be installed)
* System packages:

  Ubuntu: ```sudo apt install libusb-1.0-0 curl python3.9-virtualenv python3.9-venv build-essential libgl1-mesa-dev```
    * Specifically:
        * LibUSB 1.0 (libusb-1.0, libusb-1.0-0, or libusbx from your system package manager)
        * curl
        * python3-virtualenv  (or python3.9-virtualenv)
        * python3-venv  (or python3.9-venv)
        * Packages needed to build Qt applications:
            * build-essential
            * libgl1-mesa-dev
* [Poetry](https://python-poetry.org/) -
    * run `curl -sSL https://raw.githubusercontent.com/python-poetry/poetry/master/install-poetry.py | python3 -`
    * run `poetry --version` to make sure poetry works
    * if needed, add `$HOME/.local/bin` to your PATH to execute poetry easily - `export PATH=$HOME/.local/bin:$PATH`
    * if Python 3.9 is not your default python installation, then run `poetry env use python3.9` to give poetry access

#### How:

* Clone the Repo `git clone git@gitlab.com:codifryed/coolero.git`
* Install the dependencies:
    ```bash
    cd coolero
    poetry install
    ```
* run it: `poetry run coolero`

## Debugging

`poetry run coolero --debug`
*this will produce a lot of debug output

## Credits

* A major inspiration and where this projects stems from is [GKraken](https://gitlab.com/leinardi/gkraken) written by
  Roberto Leinardi.  
  This project started from the need for a more extensible application for liquidctl and other libraries.
* UI based on [PyOneDark](https://github.com/Wanderson-Magalhaes/PyOneDark_Qt_Widgets_Modern_GUI) by Wanderson M.Pimenta

## License

This program is licensed under [GPLv3](COPYING.txt)  
also see [the copyright notice](COPYRIGHT.md)