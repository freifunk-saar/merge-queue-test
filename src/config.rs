//  ff-node-monitor -- Monitoring for Freifunk nodes
//  Copyright (C) 2018  Ralf Jung <post AT ralfj DOT de>
//
//  This program is free software: you can redistribute it and/or modify
//  it under the terms of the GNU Affero General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU Affero General Public License for more details.
//
//  You should have received a copy of the GNU Affero General Public License
//  along with this program.  If not, see <https://www.gnu.org/licenses/>.

use rocket::fairing::{AdHoc, Fairing};
use rocket::http::uri;

use anyhow::{bail, Result};
use lettre::address::Address;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use url::Url;

use crate::util;

#[derive(Serialize, Deserialize)]
pub struct Ui {
    pub instance_name: String,
    pub instance_article_dative: String,
    pub email_from: Address,
    pub min_online_nodes: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Urls {
    pub root: Url,
    pub nodes: Url,
    pub sources: Url,
    pub stylesheet: Option<String>,
}

impl Urls {
    pub fn absolute(&self, origin: uri::Origin) -> String {
        format!("{}{}", self.root.as_str().trim_end_matches('/'), origin)
    }
}

#[derive(Deserialize)]
pub struct Secrets {
    pub smtp_host: Option<String>,
    #[serde(with = "util::hex_signing_key")]
    pub action_signing_key: hmac::Key,
}

impl Secrets {
    /// Getters for default values
    pub fn get_smtp_host(&self) -> &str {
        self.smtp_host.as_deref().unwrap_or("localhost")
    }
}

#[derive(Deserialize)]
pub struct Config {
    pub ui: Ui,
    pub secrets: Secrets,
    pub urls: Urls,
}

pub fn fairing(section: &'static str) -> impl Fairing {
    AdHoc::on_ignite(
        "Parse application configuration",
        move |rocket| async move {
            let config: Config = rocket.figment().extract_inner(section).unwrap_or_else(|_| {
                panic!("[{}] table in Rocket.toml missing or not a table", section)
            });
            rocket.manage(config)
        },
    )
}

impl Config {
    pub fn template_vals(&self, mut vals: serde_json::Value) -> Result<serde_json::Value> {
        if let Some(obj) = vals.as_object_mut() {
            let old = obj.insert(
                "config".to_string(),
                json!({
                    "ui": self.ui,
                    "urls": self.urls,
                }),
            );
            if old.is_some() {
                bail!("Someone else already put a config here")
            }
        } else {
            bail!("The context must be a JSON object")
        }
        Ok(vals)
    }
}
