use crate::{
    types::{Price, Side, Volume},
    username::Username,
};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: String,
    pub price: Price,
    pub side: Side,
    pub volume: Volume,
    pub order_type: OrderType,
}

#[derive(Debug, Deserialize)]
pub struct OrderAddedMessage {
    pub id: String,
    #[allow(dead_code)]
    side: Side,
    #[allow(dead_code)]
    price: Price,
    pub filled: Volume,
    pub resting: Volume,
    #[allow(dead_code)]
    owner: Username,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: String,
    pub id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeleteMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: String,
}

#[derive(PartialEq, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Add,
    #[allow(dead_code)]
    Delete,
    #[allow(dead_code)]
    BulkDelete,
}

#[derive(PartialEq, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    #[allow(dead_code)]
    Day,
    Ioc,
}
