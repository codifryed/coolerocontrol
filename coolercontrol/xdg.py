#  CoolerControl - monitor and control your cooling and other devices
#  Copyright (c) 2022  Guy Boldon
#  |
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#  |
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#  |
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.
# ----------------------------------------------------------------------------------------------------------------------

#
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL
# WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED
# WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE
# AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL
# DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR
# PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
# TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
# PERFORMANCE OF THIS SOFTWARE.

"""XDG Base Directory Specification variables.

xdg_cache_home(), xdg_config_home(), xdg_data_home(), and xdg_state_home()
return pathlib.Path objects containing the value of the environment variable
named XDG_CACHE_HOME, XDG_CONFIG_HOME, XDG_DATA_HOME, and XDG_STATE_HOME
respectively, or the default defined in the specification if the environment
variable is unset, empty, or contains a relative path rather than absolute
path.

xdg_config_dirs() and xdg_data_dirs() return a list of pathlib.Path
objects containing the value, split on colons, of the environment
variable named XDG_CONFIG_DIRS and XDG_DATA_DIRS respectively, or the
default defined in the specification if the environment variable is
unset or empty. Relative paths are ignored, as per the specification.

xdg_runtime_dir() returns a pathlib.Path object containing the value of
the XDG_RUNTIME_DIR environment variable, or None if the environment
variable is not set, or contains a relative path rather than absolute path.

"""

# pylint: disable=fixme

import os
from pathlib import Path
from typing import List, Optional


class XDG:

    @staticmethod
    def _path_from_env(variable: str, default: Path) -> Path:
        """Read an environment variable as a path.

        The environment variable with the specified name is read, and its
        value returned as a path. If the environment variable is not set, is
        set to the empty string, or is set to a relative rather than
        absolute path, the default value is returned.

        Parameters
        ----------
        variable : str
            Name of the environment variable.
        default : Path
            Default value.

        Returns
        -------
        Path
            Value from environment or default.

        """
        value = os.environ.get(variable)
        if value and os.path.isabs(value):
            return Path(value)
        return default

    @staticmethod
    def _paths_from_env(variable: str, default: List[Path]) -> List[Path]:
        """Read an environment variable as a list of paths.

        The environment variable with the specified name is read, and its
        value split on colons and returned as a list of paths. If the
        environment variable is not set, or set to the empty string, the
        default value is returned. Relative paths are ignored, as per the
        specification.

        Parameters
        ----------
        variable : str
            Name of the environment variable.
        default : List[Path]
            Default value.

        Returns
        -------
        List[Path]
            Value from environment or default.

        """
        if value := os.environ.get(variable):
            if paths := [Path(path) for path in value.split(":") if os.path.isabs(path)]:
                return paths
        return default

    @staticmethod
    def xdg_cache_home() -> Path:
        """Return a Path corresponding to XDG_CACHE_HOME."""
        return XDG._path_from_env("XDG_CACHE_HOME", Path.home() / ".cache")

    @staticmethod
    def xdg_config_dirs() -> List[Path]:
        """Return a list of Paths corresponding to XDG_CONFIG_DIRS."""
        return XDG._paths_from_env("XDG_CONFIG_DIRS", [Path("/etc/xdg")])

    @staticmethod
    def xdg_config_home() -> Path:
        """Return a Path corresponding to XDG_CONFIG_HOME."""
        return XDG._path_from_env("XDG_CONFIG_HOME", Path.home() / ".config")

    @staticmethod
    def xdg_data_dirs() -> List[Path]:
        """Return a list of Paths corresponding to XDG_DATA_DIRS."""
        return XDG._paths_from_env(
            "XDG_DATA_DIRS",
            [Path(path) for path in "/usr/local/share/:/usr/share/".split(":")],
        )

    @staticmethod
    def xdg_data_home() -> Path:
        """Return a Path corresponding to XDG_DATA_HOME."""
        return XDG._path_from_env("XDG_DATA_HOME", Path.home() / ".local" / "share")

    @staticmethod
    def xdg_runtime_dir() -> Optional[Path]:
        """Return a Path corresponding to XDG_RUNTIME_DIR.

        If the XDG_RUNTIME_DIR environment variable is not set, None will be
        returned as per the specification.

        """
        value = os.getenv("XDG_RUNTIME_DIR")
        if value and os.path.isabs(value):
            return Path(value)
        return None

    @staticmethod
    def xdg_state_home() -> Path:
        """Return a Path corresponding to XDG_STATE_HOME."""
        return XDG._path_from_env("XDG_STATE_HOME", Path.home() / ".local" / "state")

    @staticmethod
    def xdg_current_desktop() -> str:
        """Returns the current desktop, eg GNOME, KDE, etc."""
        desktop = os.environ.get("XDG_CURRENT_DESKTOP")
        return desktop.strip() if desktop is not None else ""
