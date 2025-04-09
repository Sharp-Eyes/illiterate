use anyhow::Result;
use bpe_thing::{generate_gibberish, load_tokens, tokenize, tokens_to_string};
use cmd_thing::{Command, Flag};
use rand::seq::IndexedRandom;
use std::{env, error::Error, sync::Arc};
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
    id::{
        marker::{ChannelMarker, MessageMarker},
        Id,
    },
};

const PREFIX: &'static str = "i!";

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

fn check_prefix(content: &str, prefix: &str) -> bool {
    // Assumes content comes without leading whitespace.
    content
        .chars()
        .zip(prefix.chars())
        .all(|(c, p)| c.eq_ignore_ascii_case(&p))
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => {
            if msg.author.id == Id::new(1169608948081492038)
                && msg.author.id == Id::new(1083860445275881493)
                || !check_prefix(&msg.content, PREFIX)
            {
                return Ok(());
            }

            let mut command = Command::parse(msg.content[PREFIX.len()..].to_string())?;
            let subcommand = command.get_next_argument()?;

            match subcommand.as_str() {
                "harry" | "h" => {
                    handle_bpe_gen(command, "./bpe/harry.bpe", http, msg.channel_id, msg.id).await?
                }
                "arknights" | "ak" => {
                    handle_bpe_gen(command, "./bpe/arknights.bpe", http, msg.channel_id, msg.id)
                        .await?
                }
                _ => todo!(),
            };

            Ok(())
        }
        _ => Ok(()),
    }
}

async fn handle_bpe_gen(
    mut command: Command,
    file: &str,
    http: Arc<HttpClient>,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
) -> Result<()> {
    let depth = Flag::new("depth")
        .alias("d")
        .default(20)
        .parse(&mut command)?;
    let freq_weight = Flag::new("freq-weight")
        .alias("f")
        .default(1.0)
        .parse(&mut command)?;
    let idx_weight = Flag::new("idx-weight")
        .alias("i")
        .default(1.0)
        .parse(&mut command)?;

    let token_grammar = load_tokens(file).map_err(|e| {
        log::error!("error loading bpe: {}", e);
        e
    })?;

    let seed_str = command.drain_arguments().make_contiguous().join(" ");
    let seed = if seed_str.is_empty() {
        let mut rng = rand::rng();
        vec![token_grammar.choose(&mut rng).unwrap().1.clone()]
    } else {
        tokenize(seed_str, &token_grammar)
    };

    let mut seed_token_iter = seed.into_iter();
    let last = seed_token_iter.next_back();

    let content = format!(
        "{}{}",
        tokens_to_string(&seed_token_iter.collect(), &token_grammar)?,
        generate_gibberish(
            &last.unwrap(),
            &token_grammar,
            depth,
            freq_weight,
            idx_weight
        )?
    );

    http.create_message(channel_id)
        .reply(message_id)
        .content(&content)
        .await?;

    Ok(())
}
