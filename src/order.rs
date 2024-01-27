use crate::{orderbook::Side, username::Username};
use serde::Serialize;

#[derive(Serialize)]
pub struct Order<'a> {
    pub username: &'a Username,
    pub password: &'a str,
    pub message: Message<'a>,
}

#[derive(Serialize)]
pub enum Message<'a> {
    Add(AddMessage<'a>),
    Delete(DeleteMessage<'a>),
    BulkDelete(BulkDeleteMessage<'a>),
}

#[derive(Serialize)]
pub struct AddMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
    pub price: f64,
    pub side: Side,
    pub volume: u32,
    pub order_type: OrderType,
}

#[derive(Serialize)]
pub struct DeleteMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
    pub id: &'a str,
}

#[derive(Serialize)]
pub struct BulkDeleteMessage<'a> {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub product: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Add,
    Delete,
    BulkDelete,
}

#[derive(Serialize)]
pub enum OrderType {
    Day,
    Ioc,
}