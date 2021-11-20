#  Coolero - monitor and control your cooling and other devices
#  Copyright (c) 2021  Guy Boldon
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

import json
import logging
from enum import Enum
from pathlib import Path
from typing import Dict

from PySide6 import QtCore
from PySide6.QtCore import QSettings

_LOG = logging.getLogger(__name__)


def serialize(path: Path, settings: Dict) -> None:
    with open(path, "w", encoding='utf-8') as write:
        json.dump(settings, write, indent=2)


def deserialize(path: Path) -> Dict:
    with open(path, "r", encoding='utf-8') as reader:
        return dict(json.loads(reader.read()))


class UserSettings(str, Enum):
    SAVE_WINDOW_SIZE = 'save_window_size'
    WINDOW_GEOMETRY = 'window_geometry'

    def __str__(self) -> str:
        return str.__str__(self)


class Settings:
    """This class provides static Settings access to all files in the application"""
    application_path: Path = Path(__file__).resolve().parent
    user: QSettings = QtCore.QSettings('coolero', 'Coolero')
    app: Dict = {}
    theme: Dict = {}

    _app_json_path = application_path.joinpath('resources/settings.json')
    if not _app_json_path.is_file():
        _LOG.fatal(f'FATAL: "settings.json" not found! check in the folder {_app_json_path}')
    app = deserialize(_app_json_path)

    _theme_json_path = application_path.joinpath(f'resources/themes/{app["theme_name"]}.json')
    if not _theme_json_path.is_file():
        _LOG.warning(f' "gui/themes/{app["theme_name"]}.json" not found! check in the folder {_theme_json_path}')
    theme = deserialize(_theme_json_path)

    @staticmethod
    def save_app_settings() -> None:
        """
        This is just a helper function for doing things like updating the version per script.
        This should not be called during the normal run of the application
        """
        if not Settings.app:
            return
        serialize(Settings._app_json_path, Settings.app)


class FeatureToggle:
    speed_cpu: bool = False
    speed_gpu: bool = False
    lighting: bool = False
