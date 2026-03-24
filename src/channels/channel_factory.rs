//! Factory functions for constructing hot-reloadable channels.
//!
//! Extracted from `start_channels()` so that `ChannelManager` can
//! start individual channels during hot-reload reconciliation.
//! Only the 4 channels in scope for hot-reload are included:
//! Feishu, DingTalk, WeCom, Mattermost.

use crate::channels::traits::Channel;
use crate::config::Config;
use std::sync::Arc;

/// Build a Feishu channel from config. Returns None if not configured or feature not enabled.
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

/// Build a DingTalk channel from config. Returns None if not configured.
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

/// Build a WeCom channel from config. Returns None if not configured.
pub fn build_wecom(config: &Config) -> Option<Arc<dyn Channel>> {
    let wc = config.channels_config.wecom.as_ref()?;
    Some(Arc::new(crate::channels::WeComChannel::new(
        wc.webhook_key.clone(),
        wc.allowed_users.clone(),
    )))
}

/// Build a Mattermost channel from config. Returns None if not configured.
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

/// Map channel name to factory function. Returns None if name unknown or channel not configured.
pub fn build_channel_by_name(name: &str, config: &Config) -> Option<Arc<dyn Channel>> {
    match name {
        "feishu" => build_feishu(config),
        "dingtalk" => build_dingtalk(config),
        "wecom" => build_wecom(config),
        "mattermost" => build_mattermost(config),
        _ => None,
    }
}
