/*
 * CoolerControl - monitor and control your cooling and other devices
 * Copyright (c) 2023  Guy Boldon
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
 */

import {Type} from "class-transformer"
import type {UID} from "@/models/Device"
// @ts-ignore
import {v4 as uuidV4} from 'uuid'

/**
 * This is currently an internal model that will be also used externally by daemon at some point. The existing external
 * model will be transformed into this one until then.
 */
export class Profile {
  readonly uid: UID
  orderId: number
  type: ProfileType
  speed_duty?: number

  @Type(() => TempSource)
  temp_source?: TempSource

  /**
   * The profile temp/duty speeds to set. eg: `[(20, 50), (25, 80)]`
   */
  speed_profile: Array<[number, number]>
  name: string

  constructor(
      orderId: number,
      type: ProfileType,
      name: string = "",
      speed_profile: Array<[number, number]> = [],
      speed_duty?: number,
      temp_source?: TempSource,
  ) {
    this.uid = uuidV4()
    this.orderId = orderId
    this.type = type
    this.speed_duty = speed_duty
    this.temp_source = temp_source
    this.speed_profile = speed_profile
    this.name = name
  }

  static createDefault(): Profile {
    return new Profile(0, ProfileType.DEFAULT, 'Default Profile', [])
  }
}

export enum ProfileType {
  DEFAULT = 'DEFAULT',
  FIXED = 'FIXED',
  GRAPH = 'GRAPH',
}

export class TempSource {
  // todo: TempSourceTypes to enable Custom Temp Sources
  constructor(
      /**
       * The internal name for this Temperature Source. Not the frontend_name or external_name
       */
      readonly temp_name: string,
      /**
       * The associated device uid containing current temp values
       */
      readonly device_uid: UID,
  ) {
  }
}