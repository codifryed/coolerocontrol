/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2021-2024  Guy Boldon and contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

import { UID } from '@/models/Device.ts'

export class ChannelSource {
    // The associated device uid containing current values
    device_uid: UID

    /// The internal name for this channel source. NOT the Label.
    channel_name: string

    channel_metric: ChannelMetric

    constructor(device_uid: UID, channel_name: string, channel_metric: ChannelMetric) {
        this.device_uid = device_uid
        this.channel_name = channel_name
        this.channel_metric = channel_metric
    }
}

export enum ChannelMetric {
    Temp = 'Temp',
    Duty = 'Duty',
    Load = 'Load',
    RPM = 'RPM',
    Freq = 'Freq',
}
