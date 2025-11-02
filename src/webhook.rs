use hex_color::HexColor;
use serenity::{all::{
    CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed, CreateEmbedFooter, ExecuteWebhook, 
    Http, LightMethod, Mentionable, Request, RoleId, Route, Timestamp, WebhookId
}, json};

pub type Webhook = (WebhookId, String);

/// Post a message to a webhook.
/// 
/// This helper function is used instead of ExecuteWebhook::execute()
/// as the latter does not expose any way of adding "with_components=true" to the parameters, 
/// which is needed to make non-interactive components work on a non-application webhook 
/// (interactive components don't work at all).
async fn execute_webhook(
    http: &Http,
    webhook: &Webhook,
    message: ExecuteWebhook,
) -> Result<(), Box<dyn std::error::Error>> {
    let params: Vec<(&'static str, String)> = vec![
        ("wait", "false".into()), 
        ("with_components", "true".into())
    ];

    let request = Request::new(
        Route::WebhookWithToken { webhook_id: webhook.0, token: &webhook.1 },
        LightMethod::Post,
    ).params(Some(params)).body(
        Some(json::to_vec(&message)?)
    );

    let response = http.request(request).await?;
    response.error_for_status()?;

    Ok(())
}

pub fn build_event_embed(
    color: &Option<HexColor>, description: &str, timestamp: u64, footer: Option<&str>
) -> Result<CreateEmbed, Box<dyn std::error::Error>> {
    let mut embed = CreateEmbed::new()
        .description(description)
        .color(color.unwrap_or(HexColor::GRAY).split_rgb())
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
    mentions: Vec<u64>,
    embed: CreateEmbed,
    buttons: Vec<CreateButton>,
) -> Result<(), Box<dyn std::error::Error>> {
    let roles: Vec<RoleId> = mentions.into_iter().map(RoleId::new).collect();

    let mut message = ExecuteWebhook::new().embed(embed).content(
        roles.iter().map(|id| id.mention().to_string()).collect::<Vec<String>>().join(" ")
    ).allowed_mentions(
        CreateAllowedMentions::new().roles(roles)
    );

    if !buttons.is_empty() {
        message = message.components(vec![
            CreateActionRow::Buttons(buttons)
        ]);
    }

    execute_webhook(http, &webhook, message).await
}