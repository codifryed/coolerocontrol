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

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from PySide6.QtCore import Qt
from PySide6.QtGui import QColor
from PySide6.QtWidgets import QMessageBox, QGraphicsDropShadowEffect

from dialogs.dialog_style import DIALOG_STYLE
from settings import Settings

if TYPE_CHECKING:
    from coolero import MainWindow  # type: ignore[attr-defined]

_LOG = logging.getLogger(__name__)


class QuitDialog(QMessageBox):

    def __init__(self, parent: MainWindow) -> None:
        super().__init__()
        self.main_window = parent
        self._dialog_style = DIALOG_STYLE.format(
            _text_size=Settings.app["font"]["text_size"],
            _font_family=Settings.app["font"]["family"],
            _text_color=Settings.theme["app_color"]["text_foreground"],
            _bg_color=Settings.theme["app_color"]["bg_one"]
        )
        shadow = QGraphicsDropShadowEffect()
        shadow.setBlurRadius(20)
        shadow.setXOffset(0)
        shadow.setYOffset(0)
        shadow.setColor(QColor(0, 0, 0, 160))
        self.setGraphicsEffect(shadow)
        self.setTextFormat(Qt.TextFormat.RichText)
        self.setWindowTitle('Exit Coolero')
        self.setText(
            '''
            <h2><center>&nbsp;&nbsp;Are you sure you want to quit?&nbsp;&nbsp;</center></h2>
            '''
        )
        self.setInformativeText(
            '''
            <h4><center><font style="color:orange">Warning!</font></center></h4>
            All manually scheduled profiles, ie. CPU based profiles, will be fixed at their current value.
            '''
        )
        self.setStandardButtons(QMessageBox.No | QMessageBox.Yes)
        self.setDefaultButton(QMessageBox.No)
        self.setStyleSheet(self._dialog_style)

    def run(self) -> int:
        return self.exec()
