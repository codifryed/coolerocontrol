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

use std::sync::Arc;

use actix_web::{delete, get, HttpResponse, post, put, Responder};
use actix_web::web::{Data, Json, Path};
use serde::{Deserialize, Serialize};

use crate::api::{CCError, handle_error, handle_simple_result};
use crate::config::Config;
use crate::setting::Function;
use crate::processors::SettingsProcessor;

/// Retrieves the persisted Function list
#[get("/functions")]
async fn get_functions(
    config: Data<Arc<Config>>
) -> Result<impl Responder, CCError> {
    config.get_functions().await
        .map(|functions| HttpResponse::Ok().json(Json(FunctionsDto { functions })))
        .map_err(handle_error)
}

/// Set the function order in the array of functions
#[post("/functions/order")]
async fn save_functions_order(
    functions_dto: Json<FunctionsDto>,
    config: Data<Arc<Config>>,
) -> Result<impl Responder, CCError> {
    config.set_functions_order(&functions_dto.functions).await.map_err(handle_error)?;
    handle_simple_result(config.save_config_file().await)
}

#[post("/functions")]
async fn save_function(
    function: Json<Function>,
    config: Data<Arc<Config>>,
) -> Result<impl Responder, CCError> {
    config.set_function(function.into_inner()).await.map_err(handle_error)?;
    handle_simple_result(config.save_config_file().await)
}

#[put("/functions")]
async fn update_function(
    function: Json<Function>,
    settings_processor: Data<Arc<SettingsProcessor>>,
    config: Data<Arc<Config>>,
) -> Result<impl Responder, CCError> {
    let function_uid = function.uid.clone();
    config.update_function(function.into_inner()).await.map_err(handle_error)?;
    config.save_config_file().await.map_err(handle_error)?;
    settings_processor.function_updated(&function_uid).await;
    Ok(HttpResponse::Ok().finish())
}

#[delete("/functions/{function_uid}")]
async fn delete_function(
    function_uid: Path<String>,
    settings_processor: Data<Arc<SettingsProcessor>>,
    config: Data<Arc<Config>>,
) -> Result<impl Responder, CCError> {
    config.delete_function(&function_uid).await.map_err(handle_error)?;
    settings_processor.function_deleted(&function_uid).await;
    config.save_config_file().await.map_err(handle_error)?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionsDto {
    functions: Vec<Function>,
}
