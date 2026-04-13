use raijin_client::RAIJIN_URL_SCHEME;
use inazuma::{AsyncApp, actions};

actions!(
    cli,
    [
        /// Registers the raijin:// URL scheme handler.
        RegisterRaijinScheme
    ]
);

pub async fn register_raijin_scheme(cx: &AsyncApp) -> anyhow::Result<()> {
    cx.update(|cx| cx.register_url_scheme(RAIJIN_URL_SCHEME)).await
}
