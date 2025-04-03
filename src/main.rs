use std::{env, error::Error, sync::Arc};
use bpe_thing::{generate_gibberish, load_tokens, tokenize, tokens_to_string};
use rand::seq::IndexedRandom;
use twilight_cache_inmemory::{DefaultInMemoryCache, ResourceType};
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};
use twilight_http::Client as HttpClient;
use twilight_model::{
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{ActivityType, MinimalActivity, Status},
    },
    id::Id,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("TOKEN")?;

    let shard_config = ConfigBuilder::new(
        token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    )
    .presence(UpdatePresencePayload {
        activities: vec![MinimalActivity {
            kind: ActivityType::Custom,
            name: "Biting your Pairs, Encoded.".to_string(),
            url: None,
        }
        .into()],
        afk: false,
        since: None,
        status: Status::DoNotDisturb,
    })
    .build();

    let mut shard = Shard::with_config(ShardId::ONE, shard_config);

    let http = Arc::new(HttpClient::new(token));

    let user = http.current_user().await?.model().await?;
    log::info!("Logged in as {}#{}", user.name, user.discriminator());

    let cache = DefaultInMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        cache.update(&event);

        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg)
            // if msg.channel_id == Id::new(1128297878855626873)
                if msg.author.id != Id::new(1169608948081492038)
                    && msg.author.id != Id::new(1083860445275881493) =>
        {
            if msg.content.starts_with("i!harry") {
                let token_grammar = load_tokens("./bpe/harry.bpe").map_err(|e| {
                    log::error!("error loading bpe: {}", e);
                    e
                })?;

                let seed = if let Some((_, input)) = msg.content.split_once(" ") {
                    tokenize(input.to_string(), &token_grammar)
                } else {
                    let mut rng = rand::rng();
                    vec![token_grammar.choose(&mut rng).unwrap().1.clone()]
                };

                let mut seed_token_iter = seed.into_iter();
                let last = seed_token_iter.next_back();
 
                let content = format!(
                    "{}{}",
                    tokens_to_string(&seed_token_iter.collect(), &token_grammar)?,
                    generate_gibberish(
                        &last.unwrap(),
                        &token_grammar,
                        20,
                        1.0,
                        1.0
                    )?
                );

                http.create_message(msg.channel_id)
                    .reply(msg.id)
                    .content(&content)
                    .await?;

            }
            
            Ok(())
        }
        _ => {Ok(())}
    }
}
