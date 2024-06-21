use std::{
    convert::Into,
    ops::{Add, Sub},
    time::Duration,
};

use chrono::{DateTime, Utc};
use poise::{serenity_prelude::{CreateEmbed, Error, GuildId, RoleId}, CreateReply};
use serde::Deserialize;
use tracing::info;

use crate::{CTFLog, Context};

#[derive(poise::ChoiceParameter)]
pub enum TimeFrame {
    #[name = "Today"]
    Today,
    #[name = "Tomorrow"]
    Tomorrow,
    #[name = "Coming week"]
    Week,
}

impl TimeFrame {
    pub const fn to_duration(&self) -> Duration {
        match self {
            Self::Today => std::time::Duration::from_secs(86_400),
            Self::Tomorrow => std::time::Duration::from_secs(172_800),
            Self::Week => std::time::Duration::from_secs(604_800),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug)]
struct Organizers {
    id: usize,
    name: String,
}

#[derive(Clone, Deserialize, Debug)]
struct CtfDuration {
    hours: usize,
    days: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Ctf {
    organizers: Vec<Organizers>,
    onsite: bool,
    finish: DateTime<Utc>,
    description: String,
    weight: f32,
    title: String,
    url: String,
    is_votable_now: bool,
    restrictions: String,
    format: String,
    start: DateTime<Utc>,
    participants: usize,
    ctftime_url: String,
    location: String,
    live_feed: String,
    public_votable: bool,
    duration: CtfDuration,
    logo: String,
    format_id: usize,
    id: usize,
    ctf_id: usize,
}
impl Ctf {
    pub async fn get_ctfs(duration: Duration) -> Result<Vec<Self>, Error> {
        let url = format!(
            "https://ctftime.org/api/v1/events/?limit=100&start={}&finish={}",
            now().as_secs(),
            now().add(duration).as_secs()
        );
        reqwest::get(url).await?.json().await.map_err(Into::into)
    }

    pub const fn finish(&self) -> DateTime<Utc> {
        self.finish
    }
}

impl From<Ctf> for CTFLog {
    fn from(ctf: Ctf) -> Self {
        Self {
            ctf_id: ctf.ctf_id,
            finish: ctf.finish,
        }
    }
}

#[poise::command(slash_command, help_text_fn = "generate_help_get_ctf")]
pub async fn get_upcoming_ctf(
    ctx: Context<'_>,
    #[description = "Requested time frame"] timeframe: TimeFrame,
) -> Result<(), Error> {
    let ctfs = Ctf::get_ctfs(timeframe.to_duration()).await?;
    info!("logged {:?}", &ctfs);
    if ctfs.is_empty() {
        ctx.say("No Upcoming CTFs in that time period").await?;
        return Ok(());
    }

    for ctf in &ctfs {
        ctx.send(CreateReply::default().embed(generate_embed(ctf))).await?;
    }
    Ok(())
}

fn generate_help_get_ctf() -> String {
    "Get all upcoming ctfs for the requested time frame".to_string()
}

fn now() -> Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}

pub fn generate_embed<'a>(ctf: &Ctf) -> CreateEmbed {
    CreateEmbed::new().title(&ctf.title)
        .description(&ctf.description)
        .thumbnail(&ctf.logo)
        .field(
            "Dates",
            format!(
                "Starts: <t:{}:f>.\n Ends: <t:{}:f>",
                ctf.finish
                    .sub(chrono::Duration::hours(ctf.duration.hours as i64))
                    .sub(chrono::Duration::days(ctf.duration.days as i64))
                    .timestamp(),
                ctf.finish.timestamp()
            ),
            true,
        )
        .field("CTF Page", &ctf.url, true)
        .url(&ctf.ctftime_url)
        .fields([
            ("Weight", &ctf.weight.to_string(), true),
            ("Participants", &ctf.participants.to_string(), true),
            ("Format", &ctf.format.to_string(), true),
        ])
}

#[poise::command(
    slash_command,
    help_text_fn = "generate_help_assign_ctf_announcement_role"
)]
pub async fn assign_ctf_announcement_role(ctx: Context<'_>) -> Result<(), Error> {
    let config = &ctx.data().config;
    let author = ctx.author();
    let guild = GuildId::new(config.guild_id);
    let role = RoleId::new(config.notification_role_id);
    let cache_http = ctx.http();
    let has_role = author.has_role(cache_http, guild, RoleId::new(config.notification_role_id)).await?;

    if has_role {
        info!("{} just removed the announcement role", author.name);
        guild.member(cache_http, author.id).await.unwrap().remove_role(cache_http, role).await?;
    } else {
        info!("{} just gave themselves the announcement role", author.name);
        guild.member(cache_http, author.id).await.unwrap().add_role(cache_http, role).await?;
    }

    ctx.say("Success").await?;

    Ok(())
}

fn generate_help_assign_ctf_announcement_role() -> String {
    "Get the ctf announcement role to get pinged for all upcoming ctftime ctfs".to_string()
}
