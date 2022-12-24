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

from typing import List

from coolercontrol.models.device import Device


class DevicesRepository:

    def __init__(self) -> None:
        self._initialize_devices()

    @property
    def statuses(self) -> List[Device]:
        raise NotImplementedError("This method should be implemented in the child class")

    def update_statuses(self) -> None:
        raise NotImplementedError("This method should be implemented in the child class")

    def shutdown(self) -> None:
        raise NotImplementedError("This method should be implemented in the child class")

    def _initialize_devices(self) -> None:
        raise NotImplementedError("This method should be implemented in the child class")
