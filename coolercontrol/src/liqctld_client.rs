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

use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use zmq::{Message, Socket};

const TMP_SOCKET_DIR: &str = "/tmp/coolercontrol.sock";

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    command: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    success: String,
    error: String,
}

pub struct Client {
    context: zmq::Context,
    socket: Socket,
}

impl Client {
    pub fn new() -> Client {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::REQ).unwrap();
        // todo: check is running as systemd daemon and use FD... appears different in the rust impl
        socket.connect(format!("ipc://{}", TMP_SOCKET_DIR).as_str())
            .with_context(|| format!("Could not open socket: {}", TMP_SOCKET_DIR)).unwrap();
        info!("connected to socket: {}", TMP_SOCKET_DIR);

        Client {
            context,
            socket,
        }
    }

    pub async fn handshake(&self) -> Result<()> {
        let request = Request { command: "handshake".to_string() };
        let handshake_json: String = serde_json::to_string(&request)
            .with_context(|| format!("Object serialization failed: {:?}", request))?;

        self.socket.send(handshake_json.as_str(), 0)
            .with_context(|| format!("Sending of message failed: {:?}", request))?;

        debug!("Handshake sent: {:?}", handshake_json);

        let mut response_msg = Message::new();
        self.socket.recv(&mut response_msg, 0)
            .with_context(|| "Error waiting for response from handshake")?;

        let response: Response = serde_json::from_str(response_msg.as_str().unwrap())
            .with_context(|| format!("Could not deserialize response: {:?}", response_msg.as_str()))?;
        debug!("Handshake response received: {:?}", response);

        if response.success == request.command {
            Ok(())
        } else { Err(anyhow!("Unexpected handshake response: {:?}", response)) }
    }

    pub async fn quit(&self) -> Result<()> {
        let request = Request { command: "quit".to_string() };
        let quit_json: String = serde_json::to_string(&request)
            .with_context(|| format!("Object serialization failed: {:?}", request))?;

        self.socket.send(quit_json.as_str(), 0)
            .with_context(|| format!("Sending of message failed: {:?}", request))?;
        debug!("Quit signal sent");

        let mut response_msg = Message::new();
        self.socket.recv(&mut response_msg, 0)
            .with_context(|| "Error waiting for response from handshake")?;

        let response: Response = serde_json::from_str(response_msg.as_str().unwrap())
            .with_context(|| format!("Could not deserialize response: {:?}", response_msg.as_str()))?;
        debug!("Quit response received: {:?}", response);

        if response.success == request.command {
            Ok(())
        } else { Err(anyhow!("Unexpected quit response: {:?}", response)) }
    }
}
