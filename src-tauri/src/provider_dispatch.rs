//! Executable provider adapters shared by global and single-card refresh.

use crate::{codebuddy_login, provider_catalog, providers};
use provider_catalog::ProviderDefinition;

pub(crate) type SnapshotFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = providers::Snapshot> + Send>>;

macro_rules! native_snapshots {
    ($( $family:literal => $snapshot:expr ),* $(,)?) => {
        const NATIVE_FAMILIES: &[&str] = &[$($family),*];
        pub(crate) fn native_snapshot(family: &str) -> Option<SnapshotFuture> {
            match family {
                $($family => Some(Box::pin($snapshot)),)*
                _ => codebuddy_login::family(family).map(|definition|
                    Box::pin(providers::codebuddy::snapshot(definition)) as SnapshotFuture),
            }
        }
    };
}

native_snapshots! {
    "claude" => providers::claude::snapshot(),
    "codex" => providers::codex::snapshot(),
    "cursor" => providers::cursor::snapshot(),
    "opencode" => providers::opencode::snapshot(),
    "copilot" => providers::copilot::snapshot(),
    "windsurf" => providers::windsurf::snapshot(),
    "grok" => providers::grok::snapshot(),
    "devin" => providers::devin::snapshot(),
    "minimax" => providers::minimax::snapshot(),
    "openrouter" => providers::openrouter::snapshot(),
    "zai" => providers::zai::snapshot(),
    "antigravity" => providers::antigravity::snapshot(),
    "deepseek" => providers::deepseek::snapshot(),
    "moonshot" => providers::moonshot::snapshot(),
    "elevenlabs" => providers::elevenlabs::snapshot(),
    "ollama" => providers::ollama::snapshot(),
    "codebuff" => providers::codebuff::snapshot(),
    "kilo" => providers::kilo::snapshot(),
    "aihubmix" => providers::aihubmix::snapshot(),
    "qwen" => providers::qwen::snapshot(),
    "hermes" => providers::hermes::snapshot(),
    "kimi" => providers::kimi::snapshot(),
    "stepfun" => providers::stepfun::snapshot(),
    "stepfun-plan" => providers::stepfun_plan::snapshot(),
    "siliconflow" => providers::siliconflow::snapshot(),
    "novita" => providers::novita::snapshot(),
    "relaybalance" => providers::relaybalance::snapshot(),
    "qodercn" => providers::qodercn::snapshot(),
    "traecn" => providers::traecn::snapshot(),
    "commandcode" => providers::commandcode::snapshot(),
    "doubao" => providers::doubao::snapshot(),
    "clawsgo" => providers::clawsgo::snapshot(),
    "shandianshuo" => providers::shandianshuo::snapshot(),
    "bocha" => providers::searchquota::bocha_snapshot(),
    "tavily" => providers::searchquota::tavily_snapshot(),
    "firecrawl" => providers::searchquota::firecrawl_snapshot(),
    "brave" => providers::searchquota::brave_snapshot(),
    "giphy" => providers::giphy::snapshot(),
    "clinepass" => providers::clinepass::snapshot(),
    "sensenova" => providers::sensenova::snapshot(),
    "apigoto" => providers::apigoto::snapshot(),
    "amp" => providers::amp::snapshot(),
    "bedrock" => providers::bedrock::snapshot(),
    "chutes" => providers::chutes::snapshot(),
    "deepgram" => providers::deepgram::snapshot(),
    "kiro" => providers::kiro::snapshot(),
    "openai-api" => providers::openai_api::snapshot(),
    "poe" => providers::poe::snapshot(),
    "tokenrhythm" => providers::tokenrhythm::snapshot(),
    "venice" => providers::venice::snapshot(),
    "vertexai" => providers::vertexai::snapshot(),
    "warp" => providers::warp::snapshot(),
    "mimo" => providers::mimo::snapshot(),
    "trae" => providers::trae::snapshot(),
    "qoder" => providers::qoder::snapshot(),
    "zed" => providers::zed::snapshot(),
    "factory" => providers::factory::snapshot(),
    "jetbrains" => providers::jetbrains::snapshot(),
    "groq" => providers::groq::snapshot(),
    "huggingface" => providers::huggingface::snapshot(),
    "longcat" => providers::longcat::snapshot(),
    "sub2api" => providers::sub2api::snapshot(),
    "mistral" => providers::mistral::snapshot(),
    "perplexity" => providers::perplexity::snapshot(),
    "volcengine" => providers::volcengine::snapshot(),
}

pub(crate) fn registered_families() -> impl Iterator<Item = &'static str> {
    NATIVE_FAMILIES.iter().copied().chain(
        codebuddy_login::FAMILIES
            .iter()
            .map(|family| family.family_id),
    )
}

pub(crate) fn registrations() -> impl Iterator<Item = &'static ProviderDefinition> {
    registered_families().map(|family| {
        provider_catalog::provider_definition(family)
            .expect("every executable family is in the capability catalog")
    })
}

pub(crate) async fn key_snapshot_for(
    provider: &str,
    key: &str,
    base_url: Option<String>,
) -> Result<providers::Snapshot, String> {
    Ok(match provider {
        "bocha" | "tavily" | "firecrawl" | "brave" => {
            providers::searchquota::snapshot_with_key(provider, key).await
        }
        "openrouter" => providers::openrouter::snapshot_with_key(key).await,
        "zai" => providers::zai::snapshot_with_key(key).await,
        "commandcode" => providers::commandcode::snapshot_with_key(key).await,
        "clawsgo" => providers::clawsgo::snapshot_with_key(key).await,
        "minimax" => providers::minimax::snapshot_with_key(key).await,
        "deepseek" => providers::deepseek::snapshot_with_key(key).await,
        "moonshot" => providers::moonshot::snapshot_with_key(key).await,
        "elevenlabs" => providers::elevenlabs::snapshot_with_key(key).await,
        "codebuff" => providers::codebuff::snapshot_with_key(key).await,
        "kilo" => providers::kilo::snapshot_with_key(key).await,
        "aihubmix" => providers::aihubmix::snapshot_with_key(key).await,
        "qwen" => providers::qwen::snapshot_with_key(key).await,
        "kimi" => providers::kimi::snapshot_with_key(key).await,
        "opencode" => providers::opencode::snapshot_with_key(key).await,
        "stepfun" => providers::stepfun::snapshot_with_key(key).await,
        "stepfun-plan" => providers::stepfun_plan::snapshot_with_creds(key, "", "").await,
        "clinepass" => providers::clinepass::snapshot_with_key(key).await,
        "sensenova" => providers::sensenova::snapshot_with_key(key).await,
        "apigoto" => providers::apigoto::snapshot_with_key(key).await,
        "amp" => providers::amp::snapshot_with_key(key).await,
        "chutes" => providers::chutes::snapshot_with_key(key).await,
        "deepgram" => providers::deepgram::snapshot_with_key(key).await,
        "openai-api" => providers::openai_api::snapshot_with_key(key).await,
        "poe" => providers::poe::snapshot_with_key(key).await,
        "venice" => providers::venice::snapshot_with_key(key).await,
        "warp" => providers::warp::snapshot_with_key(key).await,
        "mimo" => providers::mimo::snapshot_with_key(key).await,
        "factory" => providers::factory::snapshot_with_key(key).await,
        "groq" => providers::groq::snapshot_with_key(key).await,
        "huggingface" => providers::huggingface::snapshot_with_key(key).await,
        "longcat" => providers::longcat::snapshot_with_key(key).await,
        "sub2api" => {
            let url = base_url
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .ok_or_else(|| "a base URL is required for sub2api".to_string())?;
            providers::sub2api::snapshot_with_key_and_url(key, url).await
        }
        "mistral" => providers::mistral::snapshot_with_key(key).await,
        "perplexity" => providers::perplexity::snapshot_with_key(key).await,
        "volcengine" => providers::volcengine::snapshot_with_key(key).await,

        "siliconflow" => providers::siliconflow::snapshot_with_key(key).await,
        "novita" => providers::novita::snapshot_with_key(key).await,
        "relaybalance" => {
            let url = base_url
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .ok_or_else(|| "a base URL is required for Custom Relay".to_string())?;
            providers::relaybalance::snapshot_with_key(key, url).await
        }
        _ => return Err(format!("unknown provider: {provider}")),
    })
}

/// One extra account's snapshot, fetched through the same snapshot_with_key
/// flow a pasted key uses (what test_api_key probes), with the account card's
/// own id and display name passed into the provider adapter. Native adapters
/// that use card identity while reading credentials retain that scoped path;
/// the others share the key probe and only re-brand its result.
pub(crate) async fn account_snapshot(
    family: String,
    id: String,
    name: String,
    key: String,
    base_url: Option<String>,
) -> providers::Snapshot {
    match family.as_str() {
        "deepseek" => providers::deepseek::snapshot_with_key_as(&key, &id, &name).await,
        "commandcode" => providers::commandcode::snapshot_with_key_as(&key, &id, &name).await,
        "kimi" => providers::kimi::snapshot_with_key_as(&key, &id, &name).await,
        "stepfun" => providers::stepfun::snapshot_with_key_as(&key, &id, &name).await,
        "siliconflow" => providers::siliconflow::snapshot_with_key_as(&key, &id, &name).await,
        "novita" => providers::novita::snapshot_with_key_as(&key, &id, &name).await,
        "clinepass" => providers::clinepass::snapshot_with_key_as(&key, &id, &name).await,
        "relaybalance" => match base_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(url) => providers::relaybalance::snapshot_with_key_at(&key, url, &id, &name).await,
            None => providers::Snapshot::error(
                &id,
                &name,
                "this account has no base URL — remove and re-add it".into(),
            ),
        },
        // Every other key family fetches through the shared probe and is
        // re-branded with this account's card id + display name.
        _ => match key_snapshot_for(&family, &key, base_url).await {
            Ok(mut snap) => {
                snap.id = id.clone();
                snap.name = name.clone();
                snap
            }
            Err(e) => providers::Snapshot::error(&id, &name, e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn relay_probe_rejects_missing_base_url_without_querying_a_vendor() {
        let error = key_snapshot_for("relaybalance", "synthetic", None)
            .await
            .err()
            .unwrap();
        assert_eq!(error, "a base URL is required for Custom Relay");
    }

    #[tokio::test]
    async fn an_account_error_keeps_the_saved_card_identity() {
        let snapshot = account_snapshot(
            "relaybalance".into(),
            "relaybalance@work".into(),
            "Work".into(),
            "synthetic".into(),
            None,
        )
        .await;
        assert_eq!(snapshot.id, "relaybalance@work");
        assert_eq!(snapshot.name, "Work");
        assert_eq!(snapshot.status, "error");
    }

    #[tokio::test]
    async fn an_unknown_key_family_is_rejected_without_a_network_request() {
        assert_eq!(
            key_snapshot_for("unknown", "synthetic", None)
                .await
                .err()
                .unwrap(),
            "unknown provider: unknown",
        );
        assert!(native_snapshot("unknown").is_none());
        assert!(
            native_snapshot("onenewapi").is_none(),
            "relay sites use their own composite fetch"
        );
    }
}
