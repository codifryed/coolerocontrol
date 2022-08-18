/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2022  Guy Boldon
 * |
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * |
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * |
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 ******************************************************************************/

use strum::{Display, EnumString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Display, EnumString, Serialize, Deserialize)]
pub enum BaseDriver {
    CommanderPro,
    Kraken2,
    KrakenX3,
    KrakenZ3,
    SmartDevice,
    SmartDevice2,
    H1V2,
    HydroPlatinum,
    CorsairHidPsu,
    RgbFusion2,
    AuraLed,
    CommanderCore,
    NzxtEPsu,
    Modern690Lc,
    Hydro690Lc,
    Legacy690Lc,
    HydroPro,
    EvgaPascal,
    RogTuring,
    Ddr4Temperature,
    VengeanceRgb,
}