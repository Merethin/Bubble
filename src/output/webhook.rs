use hex_color::HexColor;
use serenity::all::{
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed, CreateEmbedFooter, ExecuteWebhook, Http, Mentionable, RoleId, Timestamp, UserId
};

use caramel::webhook::{Webhook, execute_webhook};

pub fn build_event_embed(
    color: HexColor, description: &str, timestamp: u64, footer: Option<&str>
) -> Result<CreateEmbed, Box<dyn std::error::Error>> {
    let mut embed = CreateEmbed::new()
        .description(description)
        .color(color.split_rgb())
        .timestamp(Timestamp::from_unix_timestamp(
            timestamp.try_into().expect("Timestamp is too far in the future")
        )?);

    if let Some(text) = footer {
        embed = embed.footer(CreateEmbedFooter::new(text));
    }

    Ok(embed)
}

pub async fn send_embed_to_webhook(
    http: &Http,
    webhook: &Webhook,
    role_mentions: Vec<u64>,
    user_mentions: Vec<u64>,
    embed: CreateEmbed,
    buttons: Vec<CreateButton>,
) -> Result<(), Box<dyn std::error::Error>> {
    let roles: Vec<RoleId> = role_mentions.into_iter().map(RoleId::new).collect();
    let users: Vec<UserId> = user_mentions.into_iter().map(UserId::new).collect();

    let mut mentions = roles.iter().map(|id| id.mention().to_string()).collect::<Vec<String>>();
    mentions.append(&mut users.iter().map(|id| id.mention().to_string()).collect::<Vec<String>>());

    let mut message = ExecuteWebhook::new().embed(embed).content(mentions.join(" ")).allowed_mentions(
        CreateAllowedMentions::new().roles(roles).users(users)
    );

    if !buttons.is_empty() {
        message = message.components(vec![
            CreateActionRow::Buttons(buttons)
        ]);
    }

    execute_webhook(http, &webhook, message).await
}