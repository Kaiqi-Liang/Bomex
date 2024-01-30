use crate::types::{Price, Side, Volume};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
    pub price: Price,
    pub side: Side,
    pub volume: Volume,
    pub order_type: OrderType,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
    pub id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeleteMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Add,
    #[allow(dead_code)]
    Delete,
    #[allow(dead_code)]
    BulkDelete,
}

#[allow(dead_code)]
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Day,
    Ioc,
}
