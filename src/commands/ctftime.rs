use std::{
    ops::{Add, Sub},
    time::Duration,
};

use poise::serenity_prelude::{CreateEmbed, Error};
use serde::Deserialize;
use tracing::info;

use crate::Context;
use chrono::Utc;

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
#[derive(Deserialize, Debug)]
struct Organizers {
    id: usize,
    name: String,
}

#[derive(Deserialize, Debug)]
struct CtfDuration {
    hours: usize,
    days: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Ctf {
    organizers: Vec<Organizers>,
    onsite: bool,
    finish: chrono::DateTime<Utc>,
    description: String,
    weight: f32,
    title: String,
    url: String,
    is_votable_now: bool,
    restrictions: String,
    format: String,
    start: chrono::DateTime<Utc>,
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

#[poise::command(slash_command, help_text_fn = "generate_help")]
pub async fn get_upcoming_ctf(
    ctx: Context<'_>,
    #[description = "Requested time frame"] timeframe: TimeFrame,
) -> Result<(), Error> {
    let url = format!(
        "https://ctftime.org/api/v1/events/?limit=100&start={}&finish={}",
        now().as_secs(),
        now().add(timeframe.to_duration()).as_secs()
    );
    let resp = reqwest::get(url).await?;

    let ctfs: Vec<Ctf> = resp.json().await.unwrap();

    info!("logged {:?}", &ctfs);
    if ctfs.is_empty() {
        ctx.say("No Upcoming CTFs in that time period").await?;
        return Ok(());
    }
    for ctf in &ctfs {
        ctx.send(|builder| {
            builder.content("").embed(|eb| embed_generator(ctf, eb))
            // // Uncomment to add buttons
            // .components(|b| {
            //     b.create_action_row(|b| {
            //         b.create_button(|b| {
            //             b.label("Visit ctftime");
            //             b
            //         })
            //     })
            // })
        })
        .await?;
    }
    Ok(())
}

fn generate_help() -> String {
    "Get all upcoming ctfs for the requested time frame".to_string()
}

fn now() -> Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}

fn embed_generator<'a>(ctf: &Ctf, eb: &'a mut CreateEmbed) -> &'a mut CreateEmbed {
    eb.title(&ctf.title)
        .description(&ctf.description)
        .thumbnail(&ctf.logo)
        .field(
            "Dates",
            format!(
                "Starts: {}.\n Ends: {}",
                ctf.finish
                    .sub(chrono::Duration::hours(ctf.duration.hours as i64))
                    .sub(chrono::Duration::days(ctf.duration.days as i64)),
                ctf.finish
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
