//! Factory functions for constructing channels from config.
//!
//! Each `build_<name>()` function is extracted verbatim from the corresponding
//! block in `collect_configured_channels()`.  `build_channel_by_name()` maps a
//! config-key string to the appropriate builder so that `ChannelManager` can
//! start individual channels during hot-reload reconciliation.

use crate::channels::traits::Channel;
use crate::config::Config;
use std::sync::Arc;

// ── Telegram ─────────────────────────────────────────────────────────────────

pub fn build_telegram(config: &Config) -> Option<Arc<dyn Channel>> {
    let tg = config.channels_config.telegram.as_ref()?;
    let ack = tg
        .ack_reactions
        .unwrap_or(config.channels_config.ack_reactions);
    Some(Arc::new(
        crate::channels::TelegramChannel::new(
            tg.bot_token.clone(),
            tg.allowed_users.clone(),
            tg.mention_only,
        )
        .with_ack_reactions(ack)
        .with_streaming(tg.stream_mode, tg.draft_update_interval_ms)
        .with_transcription(config.transcription.clone())
        .with_tts(config.tts.clone())
        .with_workspace_dir(config.workspace_dir.clone())
        .with_proxy_url(tg.proxy_url.clone()),
    ))
}

// ── Discord ──────────────────────────────────────────────────────────────────

pub fn build_discord(config: &Config) -> Option<Arc<dyn Channel>> {
    let dc = config.channels_config.discord.as_ref()?;
    Some(Arc::new(
        crate::channels::DiscordChannel::new(
            dc.bot_token.clone(),
            dc.guild_id.clone(),
            dc.allowed_users.clone(),
            dc.listen_to_bots,
            dc.mention_only,
        )
        .with_proxy_url(dc.proxy_url.clone())
        .with_transcription(config.transcription.clone()),
    ))
}

// ── Discord History ──────────────────────────────────────────────────────────

pub fn build_discord_history(config: &Config) -> Option<Arc<dyn Channel>> {
    let dh = config.channels_config.discord_history.as_ref()?;
    match crate::memory::SqliteMemory::new_named(&config.workspace_dir, "discord") {
        Ok(discord_mem) => Some(Arc::new(
            crate::channels::DiscordHistoryChannel::new(
                dh.bot_token.clone(),
                dh.guild_id.clone(),
                dh.allowed_users.clone(),
                dh.channel_ids.clone(),
                Arc::new(discord_mem),
                dh.store_dms,
                dh.respond_to_dms,
            )
            .with_proxy_url(dh.proxy_url.clone()),
        )),
        Err(e) => {
            tracing::error!("discord_history: failed to open discord.db: {e}");
            None
        }
    }
}

// ── Slack ────────────────────────────────────────────────────────────────────

pub fn build_slack(config: &Config) -> Option<Arc<dyn Channel>> {
    let sl = config.channels_config.slack.as_ref()?;
    Some(Arc::new(
        crate::channels::SlackChannel::new(
            sl.bot_token.clone(),
            sl.app_token.clone(),
            sl.channel_id.clone(),
            Vec::new(),
            sl.allowed_users.clone(),
        )
        .with_thread_replies(sl.thread_replies.unwrap_or(true))
        .with_group_reply_policy(sl.mention_only, Vec::new())
        .with_workspace_dir(config.workspace_dir.clone())
        .with_proxy_url(sl.proxy_url.clone())
        .with_transcription(config.transcription.clone()),
    ))
}

// ── Mattermost ───────────────────────────────────────────────────────────────

pub fn build_mattermost(config: &Config) -> Option<Arc<dyn Channel>> {
    let mm = config.channels_config.mattermost.as_ref()?;
    Some(Arc::new(
        crate::channels::MattermostChannel::new(
            mm.url.clone(),
            mm.bot_token.clone(),
            mm.channel_id.clone(),
            mm.allowed_users.clone(),
            mm.thread_replies.unwrap_or(true),
            mm.mention_only.unwrap_or(false),
        )
        .with_proxy_url(mm.proxy_url.clone()),
    ))
}

// ── iMessage ─────────────────────────────────────────────────────────────────

pub fn build_imessage(config: &Config) -> Option<Arc<dyn Channel>> {
    let im = config.channels_config.imessage.as_ref()?;
    Some(Arc::new(crate::channels::IMessageChannel::new(
        im.allowed_contacts.clone(),
    )))
}

// ── Matrix ───────────────────────────────────────────────────────────────────

#[cfg(feature = "channel-matrix")]
pub fn build_matrix(config: &Config) -> Option<Arc<dyn Channel>> {
    let mx = config.channels_config.matrix.as_ref()?;
    Some(Arc::new(crate::channels::MatrixChannel::new_full(
        mx.homeserver.clone(),
        mx.access_token.clone(),
        mx.room_id.clone(),
        mx.allowed_users.clone(),
        mx.allowed_rooms.clone(),
        mx.user_id.clone(),
        mx.device_id.clone(),
        config.config_path.parent().map(|path| path.to_path_buf()),
    )))
}

#[cfg(not(feature = "channel-matrix"))]
pub fn build_matrix(_config: &Config) -> Option<Arc<dyn Channel>> {
    None
}

// ── Signal ───────────────────────────────────────────────────────────────────

pub fn build_signal(config: &Config) -> Option<Arc<dyn Channel>> {
    let sig = config.channels_config.signal.as_ref()?;
    Some(Arc::new(
        crate::channels::SignalChannel::new(
            sig.http_url.clone(),
            sig.account.clone(),
            sig.group_id.clone(),
            sig.allowed_from.clone(),
            sig.ignore_attachments,
            sig.ignore_stories,
        )
        .with_proxy_url(sig.proxy_url.clone()),
    ))
}

// ── WhatsApp ─────────────────────────────────────────────────────────────────

pub fn build_whatsapp(config: &Config) -> Option<Arc<dyn Channel>> {
    let wa = config.channels_config.whatsapp.as_ref()?;
    if wa.is_ambiguous_config() {
        tracing::warn!(
            "WhatsApp config has both phone_number_id and session_path set; preferring Cloud API mode. Remove one selector to avoid ambiguity."
        );
    }
    // Runtime negotiation: detect backend type from config
    match wa.backend_type() {
        "cloud" => {
            // Cloud API mode: requires phone_number_id, access_token, verify_token
            if wa.is_cloud_config() {
                Some(Arc::new(
                    crate::channels::WhatsAppChannel::new(
                        wa.access_token.clone().unwrap_or_default(),
                        wa.phone_number_id.clone().unwrap_or_default(),
                        wa.verify_token.clone().unwrap_or_default(),
                        wa.allowed_numbers.clone(),
                    )
                    .with_proxy_url(wa.proxy_url.clone()),
                ))
            } else {
                tracing::warn!("WhatsApp Cloud API configured but missing required fields (phone_number_id, access_token, verify_token)");
                None
            }
        }
        "web" => {
            // Web mode: requires session_path
            #[cfg(feature = "whatsapp-web")]
            if wa.is_web_config() {
                return Some(Arc::new(
                    crate::channels::WhatsAppWebChannel::new(
                        wa.session_path.clone().unwrap_or_default(),
                        wa.pair_phone.clone(),
                        wa.pair_code.clone(),
                        wa.allowed_numbers.clone(),
                        wa.mode.clone(),
                        wa.dm_policy.clone(),
                        wa.group_policy.clone(),
                        wa.self_chat_mode,
                    )
                    .with_transcription(config.transcription.clone())
                    .with_tts(config.tts.clone()),
                ));
            } else {
                tracing::warn!("WhatsApp Web configured but session_path not set");
            }
            #[cfg(not(feature = "whatsapp-web"))]
            {
                tracing::warn!("WhatsApp Web backend requires 'whatsapp-web' feature. Enable with: cargo build --features whatsapp-web");
                eprintln!("  \u{26a0} WhatsApp Web is configured but the 'whatsapp-web' feature is not compiled in.");
                eprintln!("    Rebuild with: cargo build --features whatsapp-web");
            }
            None
        }
        _ => {
            tracing::warn!("WhatsApp config invalid: neither phone_number_id (Cloud API) nor session_path (Web) is set");
            None
        }
    }
}

// ── Linq ─────────────────────────────────────────────────────────────────────

pub fn build_linq(config: &Config) -> Option<Arc<dyn Channel>> {
    let lq = config.channels_config.linq.as_ref()?;
    Some(Arc::new(crate::channels::LinqChannel::new(
        lq.api_token.clone(),
        lq.from_phone.clone(),
        lq.allowed_senders.clone(),
    )))
}

// ── WATI ─────────────────────────────────────────────────────────────────────

pub fn build_wati(config: &Config) -> Option<Arc<dyn Channel>> {
    let wati_cfg = config.channels_config.wati.as_ref()?;
    Some(Arc::new(crate::channels::WatiChannel::new_with_proxy(
        wati_cfg.api_token.clone(),
        wati_cfg.api_url.clone(),
        wati_cfg.tenant_id.clone(),
        wati_cfg.allowed_numbers.clone(),
        wati_cfg.proxy_url.clone(),
    )))
}

// ── Nextcloud Talk ───────────────────────────────────────────────────────────

pub fn build_nextcloud_talk(config: &Config) -> Option<Arc<dyn Channel>> {
    let nc = config.channels_config.nextcloud_talk.as_ref()?;
    Some(Arc::new(
        crate::channels::NextcloudTalkChannel::new_with_proxy(
            nc.base_url.clone(),
            nc.app_token.clone(),
            nc.allowed_users.clone(),
            nc.proxy_url.clone(),
        ),
    ))
}

// ── Email ────────────────────────────────────────────────────────────────────

pub fn build_email(config: &Config) -> Option<Arc<dyn Channel>> {
    let email_cfg = config.channels_config.email.as_ref()?;
    Some(Arc::new(crate::channels::EmailChannel::new(
        email_cfg.clone(),
    )))
}

// ── Gmail Push ───────────────────────────────────────────────────────────────

pub fn build_gmail_push(config: &Config) -> Option<Arc<dyn Channel>> {
    let gp_cfg = config.channels_config.gmail_push.as_ref()?;
    if gp_cfg.enabled {
        Some(Arc::new(crate::channels::GmailPushChannel::new(
            gp_cfg.clone(),
        )))
    } else {
        None
    }
}

// ── IRC ──────────────────────────────────────────────────────────────────────

pub fn build_irc(config: &Config) -> Option<Arc<dyn Channel>> {
    let irc = config.channels_config.irc.as_ref()?;
    Some(Arc::new(crate::channels::IrcChannel::new(
        crate::channels::irc::IrcChannelConfig {
            server: irc.server.clone(),
            port: irc.port,
            nickname: irc.nickname.clone(),
            username: irc.username.clone(),
            channels: irc.channels.clone(),
            allowed_users: irc.allowed_users.clone(),
            server_password: irc.server_password.clone(),
            nickserv_password: irc.nickserv_password.clone(),
            sasl_password: irc.sasl_password.clone(),
            verify_tls: irc.verify_tls.unwrap_or(true),
        },
    )))
}

// ── Lark ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "channel-lark")]
pub fn build_lark(config: &Config) -> Option<Arc<dyn Channel>> {
    let lk = config.channels_config.lark.as_ref()?;
    if lk.use_feishu {
        if config.channels_config.feishu.is_some() {
            tracing::warn!(
                "Both [channels_config.feishu] and legacy [channels_config.lark].use_feishu=true are configured; ignoring legacy Feishu fallback in lark."
            );
            None
        } else {
            tracing::warn!(
                "Using legacy [channels_config.lark].use_feishu=true compatibility path; prefer [channels_config.feishu]."
            );
            Some(Arc::new(crate::channels::LarkChannel::from_config(lk)))
        }
    } else {
        Some(Arc::new(crate::channels::LarkChannel::from_lark_config(lk)))
    }
}

#[cfg(not(feature = "channel-lark"))]
pub fn build_lark(_config: &Config) -> Option<Arc<dyn Channel>> {
    None
}

// ── Feishu ───────────────────────────────────────────────────────────────────

#[cfg(feature = "channel-lark")]
pub fn build_feishu(config: &Config) -> Option<Arc<dyn Channel>> {
    let fs = config.channels_config.feishu.as_ref()?;
    Some(Arc::new(crate::channels::LarkChannel::from_feishu_config(
        fs,
    )))
}

#[cfg(not(feature = "channel-lark"))]
pub fn build_feishu(_config: &Config) -> Option<Arc<dyn Channel>> {
    None
}

// ── DingTalk ─────────────────────────────────────────────────────────────────

pub fn build_dingtalk(config: &Config) -> Option<Arc<dyn Channel>> {
    let dt = config.channels_config.dingtalk.as_ref()?;
    Some(Arc::new(
        crate::channels::DingTalkChannel::new(
            dt.client_id.clone(),
            dt.client_secret.clone(),
            dt.allowed_users.clone(),
        )
        .with_proxy_url(dt.proxy_url.clone()),
    ))
}

// ── QQ ───────────────────────────────────────────────────────────────────────

pub fn build_qq(config: &Config) -> Option<Arc<dyn Channel>> {
    let qq = config.channels_config.qq.as_ref()?;
    Some(Arc::new(
        crate::channels::QQChannel::new(
            qq.app_id.clone(),
            qq.app_secret.clone(),
            qq.allowed_users.clone(),
        )
        .with_workspace_dir(config.workspace_dir.clone())
        .with_proxy_url(qq.proxy_url.clone()),
    ))
}

// ── Twitter ──────────────────────────────────────────────────────────────────

pub fn build_twitter(config: &Config) -> Option<Arc<dyn Channel>> {
    let tw = config.channels_config.twitter.as_ref()?;
    Some(Arc::new(crate::channels::TwitterChannel::new(
        tw.bearer_token.clone(),
        tw.allowed_users.clone(),
    )))
}

// ── Mochat ───────────────────────────────────────────────────────────────────

pub fn build_mochat(config: &Config) -> Option<Arc<dyn Channel>> {
    let mc = config.channels_config.mochat.as_ref()?;
    Some(Arc::new(crate::channels::MochatChannel::new(
        mc.api_url.clone(),
        mc.api_token.clone(),
        mc.allowed_users.clone(),
        mc.poll_interval_secs,
    )))
}

// ── WeCom ────────────────────────────────────────────────────────────────────

pub fn build_wecom(config: &Config) -> Option<Arc<dyn Channel>> {
    let wc = config.channels_config.wecom.as_ref()?;
    Some(Arc::new(crate::channels::WeComChannel::new(
        wc.webhook_key.clone(),
        wc.allowed_users.clone(),
    )))
}

// ── ClawdTalk ────────────────────────────────────────────────────────────────

pub fn build_clawdtalk(config: &Config) -> Option<Arc<dyn Channel>> {
    let ct = config.channels_config.clawdtalk.as_ref()?;
    Some(Arc::new(crate::channels::ClawdTalkChannel::new(ct.clone())))
}

// ── Reddit ───────────────────────────────────────────────────────────────────

pub fn build_reddit(config: &Config) -> Option<Arc<dyn Channel>> {
    let rd = config.channels_config.reddit.as_ref()?;
    Some(Arc::new(crate::channels::RedditChannel::new(
        rd.client_id.clone(),
        rd.client_secret.clone(),
        rd.refresh_token.clone(),
        rd.username.clone(),
        rd.subreddit.clone(),
    )))
}

// ── Bluesky ──────────────────────────────────────────────────────────────────

pub fn build_bluesky(config: &Config) -> Option<Arc<dyn Channel>> {
    let bs = config.channels_config.bluesky.as_ref()?;
    Some(Arc::new(crate::channels::BlueskyChannel::new(
        bs.handle.clone(),
        bs.app_password.clone(),
    )))
}

// ── Voice Wake ───────────────────────────────────────────────────────────────

#[cfg(feature = "voice-wake")]
pub fn build_voice_wake(config: &Config) -> Option<Arc<dyn Channel>> {
    let vw = config.channels_config.voice_wake.as_ref()?;
    Some(Arc::new(crate::channels::VoiceWakeChannel::new(
        vw.clone(),
        config.transcription.clone(),
    )))
}

#[cfg(not(feature = "voice-wake"))]
pub fn build_voice_wake(_config: &Config) -> Option<Arc<dyn Channel>> {
    None
}

// ── Universal factory ────────────────────────────────────────────────────────

macro_rules! build_channel_match {
    ($name:expr, $config:expr, $($key:literal => $builder:expr),+ $(,)?) => {
        match $name {
            $($key => $builder($config),)+
            _ => None,
        }
    };
}

/// Map channel name to factory function. Returns None if name unknown or
/// channel not configured.
pub fn build_channel_by_name(name: &str, config: &Config) -> Option<Arc<dyn Channel>> {
    let result = build_channel_match!(name, config,
        "telegram" => build_telegram,
        "discord" => build_discord,
        "discord_history" => build_discord_history,
        "slack" => build_slack,
        "mattermost" => build_mattermost,
        "feishu" => build_feishu,
        "dingtalk" => build_dingtalk,
        "wecom" => build_wecom,
        "irc" => build_irc,
        "nextcloud_talk" => build_nextcloud_talk,
        "qq" => build_qq,
        "email" => build_email,
        "gmail_push" => build_gmail_push,
        "reddit" => build_reddit,
        "bluesky" => build_bluesky,
        "twitter" => build_twitter,
        "mochat" => build_mochat,
        "wati" => build_wati,
        "linq" => build_linq,
        "clawdtalk" => build_clawdtalk,
        "imessage" => build_imessage,
        "matrix" => build_matrix,
        "signal" => build_signal,
        "whatsapp" => build_whatsapp,
        "lark" => build_lark,
    );

    // Feature-gated channels that need special handling
    if result.is_some() {
        return result;
    }

    #[cfg(feature = "voice-wake")]
    if name == "voice_wake" {
        return build_voice_wake(config);
    }

    None
}
