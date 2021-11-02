#  Coolero - monitor and control your cooling and other devices
#  Copyright (c) 2021  Guy Boldon
#  All credit for basis of the user interface (GUI) goes to: Wanderson M.Pimenta
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
from typing import Dict

from settings import app_path

_LOG = logging.getLogger(__name__)


class Settings:
    json_path = app_path.joinpath('resources/settings.json')
    items: Dict = {}
    if not json_path.is_file():
        _LOG.warning(f"WARNING: \"settings.json\" not found! check in the folder {json_path}")

    def __init__(self) -> None:
        super(Settings, self).__init__()
        self.deserialize()

    def serialize(self) -> None:
        with open(self.json_path, "w", encoding='utf-8') as write:
            json.dump(self.items, write, indent=4)

    def deserialize(self) -> None:
        with open(self.json_path, "r", encoding='utf-8') as reader:
            settings = json.loads(reader.read())
            self.items = settings
