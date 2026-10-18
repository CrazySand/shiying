use crate::models::{Variant, Video};
use reqwest::{header, Client};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

pub fn tweet_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    if !input.is_empty() && input.len() <= 25 && input.bytes().all(|c| c.is_ascii_digit()) {
        return Ok(input.into());
    }
    let url = Url::parse(input).map_err(|_| "请输入完整的 X / Twitter 链接或数字推文 ID")?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some("x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com")
        )
    {
        return Err("只支持 https://x.com 或 https://twitter.com 链接".into());
    }
    let segments: Vec<_> = url.path_segments().ok_or("链接格式不正确")?.collect();
    let index = segments
        .iter()
        .position(|part| *part == "status")
        .ok_or("链接中没有推文 ID")?;
    let id = segments.get(index + 1).ok_or("链接中没有推文 ID")?;
    if id.is_empty() || id.len() > 25 || !id.bytes().all(|c| c.is_ascii_digit()) {
        return Err("推文 ID 格式不正确".into());
    }
    Ok((*id).into())
}

pub fn valid_media_url(raw: &str) -> bool {
    Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("video.twimg.com")
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none_or(|port| port == 443)
    })
}

fn target_tweet<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    if value.get("rest_id").and_then(Value::as_str) == Some(id) && value.get("legacy").is_some() {
        return Some(value);
    }
    match value {
        Value::Object(map) => map.values().find_map(|v| target_tweet(v, id)),
        Value::Array(items) => items.iter().find_map(|v| target_tweet(v, id)),
        _ => None,
    }
}

pub fn extract(value: &Value, id: &str) -> Result<Vec<Video>, String> {
    let tweet =
        target_tweet(value, id).ok_or("未找到目标推文。可能已删除、不可见，或 X 接口结构已变化")?;
    let legacy = &tweet["legacy"];
    let media = legacy
        .pointer("/extended_entities/media")
        .or_else(|| legacy.pointer("/entities/media"))
        .and_then(Value::as_array)
        .ok_or("此推文没有媒体")?;
    let title = legacy["full_text"].as_str().unwrap_or("X 视频");
    let mut videos = Vec::new();
    for (index, item) in media.iter().enumerate() {
        if !matches!(item["type"].as_str(), Some("video" | "animated_gif")) {
            continue;
        }
        let mut variants: Vec<Variant> = item
            .pointer("/video_info/variants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|v| v["content_type"] == "video/mp4")
            .filter_map(|v| {
                let url = v["url"].as_str()?;
                if !valid_media_url(url) {
                    return None;
                }
                let bitrate = v["bitrate"].as_u64().unwrap_or(0);
                let resolution = Url::parse(url).ok().and_then(|u| {
                    u.path_segments()?
                        .find(|s| {
                            s.split_once('x').is_some_and(|(w, h)| {
                                !w.is_empty()
                                    && !h.is_empty()
                                    && w.bytes().all(|b| b.is_ascii_digit())
                                    && h.bytes().all(|b| b.is_ascii_digit())
                            })
                        })
                        .map(str::to_owned)
                });
                let label = resolution
                    .map(|s| s.replace('x', " × "))
                    .unwrap_or_else(|| {
                        if bitrate > 0 {
                            format!("{} kbps", bitrate / 1000)
                        } else {
                            "原始画质".into()
                        }
                    });
                Some(Variant {
                    url: url.into(),
                    bitrate,
                    label,
                })
            })
            .collect();
        variants.sort_by_key(|variant| std::cmp::Reverse(variant.bitrate));
        variants.dedup_by(|a, b| a.url == b.url);
        if variants.is_empty() {
            continue;
        }
        variants[0].label.push_str(" · 推荐");
        let thumbnail = item["media_url_https"]
            .as_str()
            .filter(|raw| {
                Url::parse(raw)
                    .is_ok_and(|u| u.scheme() == "https" && u.host_str() == Some("pbs.twimg.com"))
            })
            .unwrap_or("");
        videos.push(Video {
            id: format!("{id}_{}", index + 1),
            tweet_id: id.into(),
            title: title.chars().take(180).collect(),
            thumbnail: thumbnail.into(),
            variants,
        });
    }
    if videos.is_empty() {
        return Err("此推文没有可下载的 MP4 视频".into());
    }
    Ok(videos)
}

pub async fn parse(
    input: String,
    cookie: String,
    authorization: String,
    query_id: String,
) -> Result<Vec<Video>, String> {
    let id = tweet_id(&input)?;
    let authorization = crate::settings::normalize_authorization(&authorization)?;
    if authorization.is_empty() {
        return Err("请先在设置中填写 Authorization".into());
    }
    let csrf = cookie
        .split(';')
        .find_map(|part| part.trim().strip_prefix("ct0="))
        .filter(|v| !v.is_empty())
        .ok_or("请先在设置中填写包含 ct0 的 X 登录 Cookie")?;
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::COOKIE,
        header::HeaderValue::from_str(&cookie).map_err(|_| "Cookie 格式不正确")?,
    );
    headers.insert(
        "x-csrf-token",
        header::HeaderValue::from_str(csrf).map_err(|_| "ct0 格式不正确")?,
    );
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&authorization).map_err(|_| "Authorization 格式不正确")?,
    );
    headers.insert(
        "x-twitter-active-user",
        header::HeaderValue::from_static("yes"),
    );
    headers.insert(
        "x-twitter-auth-type",
        header::HeaderValue::from_static("OAuth2Session"),
    );
    let client = Client::builder().timeout(Duration::from_secs(30)).connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none()).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130.0.0.0 Safari/537.36")
        .build().map_err(|_| "无法初始化网络客户端")?;
    let variables = json!({"focalTweetId":id,"with_rux_injections":false,"rankingMode":"Relevance","includePromotedContent":false,"withCommunity":true,"withQuickPromoteEligibilityTweetFields":true,"withBirdwatchNotes":true,"withVoice":true});
    let features = json!({
        "rweb_video_screen_enabled":false,"profile_label_improvements_pcf_label_in_post_enabled":true,
        "responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,
        "creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_timeline_navigation_enabled":true,
        "responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"premium_content_api_read_enabled":false,
        "communities_web_enable_tweet_community_results_fetch":true,"c9s_tweet_anatomy_moderator_badge_enabled":true,
        "responsive_web_grok_analyze_button_fetch_trends_enabled":false,"responsive_web_grok_analyze_post_followups_enabled":true,
        "responsive_web_jetfuel_frame":false,"responsive_web_grok_share_attachment_enabled":true,
        "articles_preview_enabled":true,"responsive_web_edit_tweet_api_enabled":true,"graphql_is_translatable_rweb_tweet_is_translatable_enabled":true,
        "view_counts_everywhere_api_enabled":true,"longform_notetweets_consumption_enabled":true,
        "responsive_web_twitter_article_tweet_consumption_enabled":true,"tweet_awards_web_tipping_enabled":false,
        "creator_subscriptions_quote_tweet_preview_enabled":false,"freedom_of_speech_not_reach_fetch_enabled":true,
        "standardized_nudges_misinfo":true,"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled":true,
        "longform_notetweets_rich_text_read_enabled":true,"longform_notetweets_inline_media_enabled":true,
        "responsive_web_enhance_cards_enabled":false
    });
    let response = client
        .get(format!(
            "https://x.com/i/api/graphql/{query_id}/TweetDetail"
        ))
        .headers(headers)
        .query(&[
            ("variables", variables.to_string()),
            ("features", features.to_string()),
        ])
        .send()
        .await
        .map_err(|_| "连接 X 失败或超时，请检查网络连接")?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => {
                "X 拒绝访问，请检查 Cookie 和 Authorization；也可能需要更新接口适配".into()
            }
            429 => "请求过于频繁，请稍后重试".into(),
            400 | 404 => "X 查询接口不可用，请更新应用的接口适配".into(),
            _ => format!("X 接口返回 HTTP {status}"),
        });
    }
    let value: Value = response
        .json()
        .await
        .map_err(|_| "X 返回了无法识别的数据")?;
    if value.get("errors").is_some() && target_tweet(&value, &id).is_none() {
        return Err("X 返回接口错误，请确认登录状态、推文可见性和接口适配".into());
    }
    extract(&value, &id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_tweet_input() {
        assert_eq!(
            tweet_id("https://x.com/user/status/12345?s=20").unwrap(),
            "12345"
        );
        assert_eq!(tweet_id("12345").unwrap(), "12345");
        assert!(tweet_id("https://evil.test/status/12345").is_err());
        assert!(tweet_id("https://x.com/user/status/../settings").is_err());
        assert!(tweet_id("").is_err());
    }
    #[test]
    fn extracts_only_target_video_and_skips_images() {
        let data = json!({"items":[
            {"rest_id":"999","legacy":{"extended_entities":{"media":[{"type":"video","video_info":{"variants":[{"content_type":"video/mp4","url":"https://video.twimg.com/other.mp4"}]}}]}}},
            {"rest_id":"123","legacy":{"full_text":"hello","extended_entities":{"media":[
                {"type":"photo"},
                {"type":"video","video_info":{"variants":[
                    {"content_type":"application/x-mpegURL","url":"https://video.twimg.com/a.m3u8"},
                    {"content_type":"video/mp4","url":"https://evil.test/file.mp4","bitrate":9999},
                    {"content_type":"video/mp4","url":"https://video.twimg.com/vid/640x360/low.mp4","bitrate":256000},
                    {"content_type":"video/mp4","url":"https://video.twimg.com/vid/1280x720/high.mp4","bitrate":832000}
                ]}}
            ]}}}
        ]});
        let videos = extract(&data, "123").unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].variants.len(), 2);
        assert_eq!(videos[0].variants[0].bitrate, 832000);
        assert_eq!(videos[0].id, "123_2");
        assert!(extract(&data, "456").is_err());
    }
    #[test]
    fn rejects_media_with_untrusted_origin() {
        assert!(valid_media_url("https://video.twimg.com/a.mp4?tag=12"));
        assert!(!valid_media_url("http://video.twimg.com/a.mp4"));
        assert!(!valid_media_url("https://video.twimg.com.evil.test/a.mp4"));
        assert!(!valid_media_url("https://user:pass@video.twimg.com/a.mp4"));
    }
}
