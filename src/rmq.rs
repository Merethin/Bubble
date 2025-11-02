use lapin::{options::{BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions}, types::FieldTable};
use log::info;

use crate::config::Config;

pub async fn open_rmq_connection(config: &Config) -> Result<lapin::Connection, Box<dyn std::error::Error>> {
    let conn = lapin::Connection::connect(
        &config.input.url,
        lapin::ConnectionProperties::default(),
    ).await?;

    info!("Connected to RabbitMQ");

    Ok(conn)
}

pub async fn create_akari_consumer(config: &Config, channel: &lapin::Channel) -> Result<lapin::Consumer, Box<dyn std::error::Error>> {
    channel.exchange_declare(
        &config.input.exchange_name,
        lapin::ExchangeKind::Topic,
        ExchangeDeclareOptions::default(),
        FieldTable::default()
    ).await?;

    let queue = channel.queue_declare(
        "", QueueDeclareOptions {
                exclusive: true,
                auto_delete: true,
                ..Default::default()
            }, FieldTable::default()
    ).await?;

    channel.queue_bind(
        queue.name().as_str(), &config.input.exchange_name, "*", QueueBindOptions::default(), FieldTable::default()
    ).await?;

    Ok(channel.basic_consume(
        queue.name().as_str(), "consumer", BasicConsumeOptions::default(), FieldTable::default()
    ).await?)
}