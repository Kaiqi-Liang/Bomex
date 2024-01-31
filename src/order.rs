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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderAddedMessage {
    pub order_id: String,
    side: Side,
    price: Price,
    pub filled_volume: Volume,
    pub resting_volume: Volume,
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
    #[allow(unused)]
    Delete,
    #[allow(unused)]
    BulkDelete,
}

#[allow(unused)]
#[derive(PartialEq, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Day,
    Ioc,
}
